use crate::evm_opcode::Instruction;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::BTreeMap;

use inkwell::AddressSpace;
use inkwell::values::{FunctionValue, GlobalValue, BasicValueEnum, IntValue};
use inkwell::types::{IntType};
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

use log::{info, warn, error};

const DEBUG: bool = true;

fn nibble2i256(vals: &[u8]) -> Vec<u64> {
    let mut ret = vec![];
    let mut vals = vals.to_vec();
    vals.reverse();
    for values in vals.chunks(32) {
        let mut out = 0;
        for &i in values.iter().rev() {
            out = out << 8 | i as u64;
        }
        ret.push(out);
    }
    ret
}

pub struct Compiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    label_stack: Rc<RefCell<Vec<&'static str>>>,

    i256_ty: IntType<'ctx>,
    sp: Option<GlobalValue<'ctx>>,
    stack: Option<GlobalValue<'ctx>>,
    mem: Option<GlobalValue<'ctx>>,
    code: Option<GlobalValue<'ctx>>,
    fun: Option<FunctionValue<'ctx>>,
    jumpbb: Option<BasicBlock<'ctx>>,
    errbb: Option<BasicBlock<'ctx>>,

    jumpdests: BTreeMap<usize, BasicBlock<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Compile instructions
    pub fn new(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a Module<'ctx>,
    ) -> Self {
        let compiler = Self {
            context,
            i256_ty: context.custom_width_int_type(256),
            module,
            sp: None,
            stack: None,
            mem: None,
            code: None,
            fun: None,
            jumpdests: BTreeMap::new(),
            jumpbb: None,
            errbb: None,
            label_stack: Rc::new(RefCell::new(Vec::new())),
        };
        compiler
    }

    pub fn compile(&mut self,
        builder: &'a Builder<'ctx>,
        instrs: &[(usize, Instruction)],
        payload: &[u8],
        name: &str,
        is_runtime: bool,
    ) {
        if !is_runtime {
            self.build_globals(builder, payload, name, is_runtime);
        }

        self.build_function(builder, name, is_runtime);

        // entry
        let entrybb = self.context.append_basic_block(self.fun.unwrap(), "entry");
        builder.position_at_end(entrybb);

        // err
        self.errbb = Some(self.context.append_basic_block(self.fun.unwrap(), "err"));


        // jump table
        self.jumpbb = Some(self.context.append_basic_block(self.fun.unwrap(), "jumpbb"));
        let mainbb = self.context.append_basic_block(self.fun.unwrap(), "main");
        for (offset, _dest) in instrs.iter()
            .take_while(|(_, i)| *i != Instruction::Invalid)
            .filter(|(_,i)|*i==Instruction::JumpDest)
        {
            let jumpdestbb = self.context.append_basic_block(self.fun.unwrap(), "jumpdest");
            self.jumpdests.insert(*offset, jumpdestbb);
        }
        self.build_jumpbb(builder);


        // entry br to main
        builder.position_at_end(entrybb);
        builder.build_unconditional_branch(mainbb);

        // err ret
        builder.position_at_end(self.errbb.unwrap());
        builder.build_return(None);

        // position to main
        builder.position_at_end(mainbb);

        for (offset, instr) in instrs {
            if Option::None == self.build_instr(*offset, instr, builder) {
                break;
            }
        }
        builder.build_return(None);
    }

    fn build_jumpbb(&self, builder: &'a Builder<'ctx>) {
        builder.position_at_end(self.jumpbb.unwrap());
        let sp = self.build_sp(builder);
        let dest = self.build_peek(builder, sp, 1, "dest");
        let _sp = self.build_decr(builder, sp, 2);
        let cases = self.jumpdests.iter()
            .map(|(offset, bb)|
                (self.i256_ty.const_int(*offset as u64, false), *bb)).collect::<Vec<_>>();
        builder.build_switch(dest, self.errbb.unwrap(), &cases);
    }

    fn dump_stack(&self, builder: &'a Builder<'ctx>) -> FunctionValue<'ctx> {
        let name = "dump_stack";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        // dump_stack
        let str_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[str_ty],false);
        let dump_stack = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));

        let readonly = self.context.create_string_attribute("readonly", "true");
        dump_stack.add_attribute(inkwell::attributes::AttributeLoc::Function, readonly);
        dump_stack
    }

    /// Build stack related global variables
    fn build_globals(&mut self, builder: &'a Builder<'ctx>, payload: &[u8], name: &str, is_runtime: bool) {
        let i64_ty = self.context.i64_type();
        let i256_arr_ty = self.i256_ty.array_type(1024); // .zero (256 / 8 * size)

        // stack
        let stack = self.module.add_global(i256_arr_ty, Some(AddressSpace::Generic), "stack");
        stack.set_initializer(&i256_arr_ty.const_zero());

        // sp
        let sp = self.module.add_global(i64_ty, Some(AddressSpace::Generic), "sp");
        sp.set_initializer(&i64_ty.const_int(0, false));

        // mem
        let i8_array_ty = self.context.i8_type().array_type(1024 * 32);
        let mem = self.module.add_global(i8_array_ty, Some(AddressSpace::Generic), "mem");
        mem.set_initializer(&i8_array_ty.const_zero());

        // code
        let code = self.module.add_global(
            self.context.i8_type().array_type(payload.len() as u32),
            Some(AddressSpace::Generic),
            if is_runtime { "code_runtime" } else{ "code" });
        let payload = self.context.const_string(payload, false);
        code.set_initializer(&payload);

        self.stack = Some(stack);
        self.sp = Some(sp);
        self.mem = Some(mem);
        self.code = Some(code);
    }

    pub fn build_function(&mut self, builder: &'a Builder<'ctx>, name: &str, is_runtime: bool) {
        let msg_len = self.context.i64_type().into();
        let ret_offset = self.context.i64_type().ptr_type(AddressSpace::Generic).into();
        let ret_len = self.context.i64_type().ptr_type(AddressSpace::Generic).into();
        let msg = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_type = self.context.void_type()
            .fn_type(
                &[msg, msg_len, ret_offset, ret_len],
                false
            );
        let name = "contract";
        let fn_name = format!("{}_{}", name, if is_runtime {"runtime"} else {"constructor"});
        let function = self.module.add_function(&fn_name, fn_type, None);
        self.fun = Some(function);
    }

    /// Print IR
    pub fn dbg(&self) {
        self.module.print_to_stderr();
    }

    fn push_label(&self, name: &'static str, builder: &'a Builder<'ctx>) -> BasicBlock<'ctx> {
        let mut s = self.label_stack.borrow_mut();
        s.push(name);
        let lbl_name = s.join("_");
        let _function = self.fun.unwrap();
        let basic_block = self.context.insert_basic_block_after(builder.get_insert_block().unwrap(), &lbl_name);
        builder.build_unconditional_branch(basic_block);
        builder.position_at_end(basic_block);
        basic_block
    }

    fn pop_label(&self) {
        let mut s = self.label_stack.borrow_mut();
        s.pop();
    }

    /// Function call to dump_stack
    fn build_dump_stack(&self, builder: &'a Builder<'ctx>) {
        if DEBUG {
            let s = self.label_stack.borrow_mut();
            let lbl_name = s.join("_");
            let s = unsafe {
                builder.build_global_string(&lbl_name, "str")
                    .as_pointer_value()
                    .const_cast(
                        self.context.i8_type().ptr_type(AddressSpace::Generic)) };
            builder.build_call(self.dump_stack(builder), &[s.into()], "dump");
        }
    }

    fn build_sp(&self, builder: &'a Builder<'ctx>) -> IntValue<'ctx> {
        let sp_ptr = self.sp.unwrap().as_pointer_value();
        let sp = builder.build_load(sp_ptr, "sp").into_int_value();
        sp
    }

    /// Increment sp
    fn build_incr(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>, n: u64) -> IntValue<'ctx> {
        self.push_label("incr", builder);
        let sp = builder.build_int_add(
            sp,
            self.context.i64_type().const_int(n, false),
            "sp");
        builder.build_store(self.sp.unwrap().as_pointer_value(), sp);
        self.pop_label();
        sp
    }

    /// Decrement sp
    fn build_decr(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>, n: u64) -> IntValue<'ctx> {
        self.push_label("decr", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(n, false),
            "sp");
        builder.build_store(self.sp.unwrap().as_pointer_value(), sp);
        self.pop_label();
        sp
    }

    /// Peek a value off stack with offset
    fn build_peek(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>, n: u64, name: &str) -> IntValue<'ctx> {
        self.push_label("peek", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(n, false),
            "sp");

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        let ret = builder.build_load(addr, name).into_int_value();

        self.pop_label();
        ret
    }

    /// Pop a value off stack
    fn build_pop(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>) -> (IntValue<'ctx>, IntValue<'ctx>) {
        self.push_label("pop", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(1, false),
            "sp");
        let ret = self.build_peek(builder, sp, 0, "ret");
        builder.build_store(self.sp.unwrap().as_pointer_value(), sp);
        self.pop_label();
        (ret, sp)
    }

    /// Push a value onto stack
    fn build_push(&self, builder: &'a Builder<'ctx>, value: BasicValueEnum<'ctx>, sp: IntValue<'ctx>) -> BasicValueEnum<'ctx> {
        self.push_label("push", builder);

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        builder.build_store(addr, value);
        self.build_incr(builder, sp, 1);
        self.pop_label();
        value
    }

    /// Build instruction
    fn build_instr(&self, offset: usize, instr: &Instruction, builder: &'a Builder<'ctx>) -> Option<()> {
        // dbg!((offset, instr));
        match instr {
            Instruction::Stop |
            Instruction::SignExtend |
            Instruction::Byte |
            Instruction::Sha3 |
            Instruction::Addr |
            Instruction::Balance |
            Instruction::Origin |
            Instruction::Caller |
            Instruction::CallDataLoad |
            Instruction::CallDataCopy |
            Instruction::CodeSize |
            Instruction::GasPrice |
            Instruction::ExtCodeSize |
            Instruction::ExtCodeCopy |
            Instruction::ReturnDataSize |
            Instruction::ReturnDataCopy |
            Instruction::ExtCodeHash |
            Instruction::Blockhash |
            Instruction::Coinbase |
            Instruction::Timestamp |
            Instruction::Number |
            Instruction::Difficulty |
            Instruction::MStore8 |
            Instruction::SLoad |
            Instruction::Jump |
            Instruction::PC |
            Instruction::MSize |
            Instruction::Gas |
            Instruction::GasLimit |
            Instruction::Log(_) |
            Instruction::Create |
            Instruction::Call |
            Instruction::CallCode |
            Instruction::DelegateCall |
            Instruction::Create2 |
            Instruction::StaticCall |
            Instruction::SStore |
            Instruction::SelfDestruct => {
                error!("{:#?}", instr);
                return None;
            }
            Instruction::CallDataSize => {
                let name = "calldatasize";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let calldatasize = self.fun.unwrap().get_nth_param(1).unwrap().into_int_value();
                let calldatasize = builder.build_int_cast(calldatasize, self.i256_ty, "calldatasize").into();
                self.build_push(builder, calldatasize, sp);
            }
            Instruction::Invalid => {
                warn!("Invalid instruction encountered. Halting compilation!");
                return None;
            }
            Instruction::Return => {
                let name = "return";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let length = self.build_peek(builder, sp, 2, "length");
                let offset = self.build_peek(builder, sp, 1, "offset");
                let _sp = self.build_decr(builder, sp, 2);

                let length = builder.build_int_cast(length, self.context.i64_type(), "length");
                let offset = builder.build_int_cast(offset, self.context.i64_type(), "offset");

                let offset_ptr = self.fun.unwrap().get_nth_param(2).unwrap().into_pointer_value();
                let len_ptr = self.fun.unwrap().get_nth_param(3).unwrap().into_pointer_value();
                builder.build_store(offset_ptr, offset);
                builder.build_store(len_ptr, length);
            }
            Instruction::CodeCopy => {
                let name = "codecopy";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let length = self.build_peek(builder, sp, 3, "length");
                let offset = self.build_peek(builder, sp, 2, "offset");
                let dest_offset = self.build_peek(builder, sp, 1, "dest_offset");
                let _sp = self.build_decr(builder, sp, 3);

                let src = unsafe {
                    builder.build_in_bounds_gep(
                        self.code.unwrap().as_pointer_value(),
                        &[self.context.i8_type().const_int(0, false), offset],
                        "src") };
                let dest = unsafe {
                    builder.build_in_bounds_gep(
                        self.mem.unwrap().as_pointer_value(),
                        &[self.context.i8_type().const_int(0, false), dest_offset],
                        "dest") };

                // memory[destOffset:destOffset+length] = code[offset:offset+length];
                // let length = builder.build_int_cast(length, self.context.i64_type(), "length");
                builder.build_memcpy(dest, 1, src, 1, length).unwrap();
            }
            Instruction::JumpDest => {
                let bb = self.jumpdests.get(&offset).unwrap();
                builder.build_unconditional_branch(*bb);
                builder.position_at_end(*bb);
                self.push_label("jumpdest", builder);
            }
            Instruction::Revert => {
                let name = "revert";
                self.push_label(name, builder);
                builder.build_unconditional_branch(self.errbb.unwrap());
            }
            Instruction::JumpIf => {
                let name = "jumpi";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let cond = self.build_peek(builder, sp, 2, "cond");
                let cond = builder.build_int_compare(IntPredicate::EQ, cond, self.i256_ty.const_int(1, false), "cond");

                let else_block = self.context.insert_basic_block_after(builder.get_insert_block().unwrap(), "else");
                builder.build_conditional_branch(cond, self.jumpbb.unwrap(), else_block);
                builder.position_at_end(else_block);
            }
            Instruction::IsZero => {
                let name = "iszero";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (value, sp) = self.build_pop(builder, sp);
                let cmp = builder.build_int_compare(
                    IntPredicate::EQ,
                    value,
                    self.i256_ty.const_zero(),
                    name);
                let cmp = builder.build_int_z_extend_or_bit_cast(
                    cmp,
                    self.i256_ty,
                    name).into();
                self.build_push(builder, cmp, sp);
            }
            Instruction::Dup(n) => {
                let name = "dup";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let value = self.build_peek(builder, sp, *n as u64 + 1, "val").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Swap(n) => {
                let name = "swap";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let sp_l = builder.build_int_sub(
                    sp,
                    self.context.i64_type().const_int(1, false),
                    "sp");

                let sp_r = builder.build_int_sub(
                    sp,
                    self.context.i64_type().const_int(*n as u64 +1, false),
                    "sp");

                let stack = self.stack.unwrap().as_pointer_value();
                let addr_l = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp_l], "stack") };
                let addr_r = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp_r], "stack") };
                let value_l = builder.build_load(addr_l, "arr").into_int_value();
                let value_r = builder.build_load(addr_r, "arr").into_int_value();
                builder.build_store(addr_l, value_r);
                builder.build_store(addr_r, value_l);
            }
            Instruction::CallValue => {
                // TODO:
                let name = "callvalue";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let value = self.i256_ty.const_int(0, false).into();
                self.build_push(builder, value, sp);
            }
            Instruction::MLoad => {
                let name = "mstore";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (offset, _sp) = self.build_pop(builder, sp);
                let offset = builder.build_int_truncate_or_bit_cast(offset, self.context.i64_type(), "idx");

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.context.i64_type().const_zero(), offset], "stack") };
                let _value = builder.build_load(addr, "value");
            }
            Instruction::MStore => {
                let name = "mstore";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let offset = self.build_peek(builder, sp, 1, "offset");
                let value = self.build_peek(builder, sp, 2, "value");
                let _sp = self.build_decr(builder, sp, 2);
                let offset = builder.build_int_truncate_or_bit_cast(offset, self.context.i64_type(), "idx");

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.context.i64_type().const_zero(), offset], "stack") };
                let addr = builder.build_pointer_cast(addr, self.i256_ty.ptr_type(AddressSpace::Generic), "addr");
                builder.build_store(addr, value);
            }
            Instruction::Sub => {
                let name = "sub";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_sub(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::SDiv => {
                let name = "sdiv";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_signed_div(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::AddMod => {
                let name = "addmod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let n = self.build_peek(builder, sp, 3, "N");
                let sp = self.build_decr(builder, sp, 3);
                let add = builder.build_int_add(lhs, rhs, "add");
                let value = builder.build_int_unsigned_rem(add, n, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::MulMod => {
                let name = "mulmod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let n = self.build_peek(builder, sp, 3, "N");
                let sp = self.build_decr(builder, sp, 3);
                let mul = builder.build_int_mul(lhs, rhs, "add");
                let value = builder.build_int_unsigned_rem(mul, n, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Exp => {
                let name = "exp";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let base = self.build_peek(builder, sp, 1, "base").into();
                let exp = self.build_peek(builder, sp, 2, "exp").into();
                let sp = self.build_decr(builder, sp, 2);
                let upow = self.upow(builder, 256);
                let value = builder.build_call(upow, &[base, exp], "upow").try_as_basic_value().unwrap_left();
                self.build_push(builder, value, sp);
            }
            Instruction::Mod => {
                let name = "mod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_unsigned_rem(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::SMod => {
                let name = "mod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_signed_rem(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Div => {
                let name = "div";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_unsigned_div(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Mul => {
                let name = "mul";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_mul(lhs, rhs, name).into(); // TODO: verify
                self.build_push(builder, value, sp);
            }
            Instruction::Add => {
                let name = "add";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_add(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::And => {
                let name = "and";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_and(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Or => {
                let name = "or";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_or(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Shl => {
                let name = "shl";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let shift = self.build_peek(builder, sp, 1, "shift");
                let value = self.build_peek(builder, sp, 2, "value");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_left_shift(value, shift, "shl").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Sar => {
                let name = "sar";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let shift = self.build_peek(builder, sp, 1, "shift");
                let value = self.build_peek(builder, sp, 2, "value");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_right_shift(value, shift, true, "shr").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Shr => {
                let name = "shr";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let shift = self.build_peek(builder, sp, 1, "shift");
                let value = self.build_peek(builder, sp, 2, "value");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_right_shift(value, shift, false, "shr").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Xor => {
                let name = "xor";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_xor(lhs, rhs, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Not => {
                let name = "not";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (value, sp) = self.build_pop(builder, sp);
                let value = builder.build_not(value, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Lt => {
                let name = "lt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::ULT, lhs, rhs, "lt").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Gt => {
                let name = "gt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::UGT, lhs, rhs, "lt").into();
                self.build_push(builder, value, sp);
            }
            Instruction::SLt => {
                let name = "slt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::SLT, lhs, rhs, "lt").into();
                self.build_push(builder, value, sp);
            }
            Instruction::SGt => {
                let name = "sgt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::SGT, lhs, rhs, "lt").into();
                self.build_push(builder, value, sp);
            }
            Instruction::EQ => {
                let name = "eq";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::EQ, lhs, rhs, "lt").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Pop => {
                let name = "pop";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (_ret, _sp) = self.build_pop(builder, sp);
            }
            Instruction::Push(vals) => {
                self.push_label("push", builder);
                let sp = self.build_sp(builder);
                let value = self.i256_ty.const_int_arbitrary_precision(&nibble2i256(vals)).into();
                self.build_push(builder, value, sp);
            }
        };
        self.build_dump_stack(builder);
        self.pop_label();
        Some(())
    }

    /// From
    /// https://github.com/hyperledger-labs/solang/blob/2dfd46dfc3b709c2c8e233ee4e7f27380fd58964/src/emit/mod.rs#L4709
    pub fn upow(&self, builder: &'a Builder<'ctx>, bit: u32) -> FunctionValue<'ctx> {
        /*
            int ipow(int base, int exp)
            {
                int result = 1;
                for (;;)
                {
                    if (exp & 1)
                        result *= base;
                    exp >>= 1;
                    if (!exp)
                        break;
                    base *= base;
                }
                return result;
            }
        */
        let name = format!("__upower{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let pos = builder.get_insert_block().unwrap();

        // __upower(base, exp)
        let function =
            self.module
                .add_function(&name, ty.fn_type(&[ty.into(), ty.into()], false), None);

        let entry = self.context.append_basic_block(function, "entry");
        let loop_block = self.context.append_basic_block(function, "loop");
        let multiply = self.context.append_basic_block(function, "multiply");
        let nomultiply = self.context.append_basic_block(function, "nomultiply");
        let done = self.context.append_basic_block(function, "done");
        let notdone = self.context.append_basic_block(function, "notdone");

        builder.position_at_end(entry);

        let l = builder.build_alloca(ty, "");
        let r = builder.build_alloca(ty, "");
        let o = builder.build_alloca(ty, "");

        builder.build_unconditional_branch(loop_block);

        builder.position_at_end(loop_block);
        let base = builder.build_phi(ty, "base");
        base.add_incoming(&[(&function.get_nth_param(0).unwrap(), entry)]);

        let exp = builder.build_phi(ty, "exp");
        exp.add_incoming(&[(&function.get_nth_param(1).unwrap(), entry)]);

        let result = builder.build_phi(ty, "result");
        result.add_incoming(&[(&ty.const_int(1, false), entry)]);

        let lowbit = builder.build_int_truncate(
            exp.as_basic_value().into_int_value(),
            self.context.bool_type(),
            "bit",
        );

        builder
            .build_conditional_branch(lowbit, multiply, nomultiply);

        builder.position_at_end(multiply);

        let result2 = if bit > 64 {
            builder
                .build_store(l, result.as_basic_value().into_int_value());
            builder
                .build_store(r, base.as_basic_value().into_int_value());

            builder.build_call(
                self.module.get_function("__mul32").unwrap(),
                &[
                    builder
                        .build_pointer_cast(
                            l,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    builder
                        .build_pointer_cast(
                            r,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    builder
                        .build_pointer_cast(
                            o,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    self.context
                        .i32_type()
                        .const_int(bit as u64 / 32, false)
                        .into(),
                ],
                "",
            );

            builder.build_load(o, "result").into_int_value()
        } else {
            builder.build_int_mul(
                result.as_basic_value().into_int_value(),
                base.as_basic_value().into_int_value(),
                "result",
            )
        };

        builder.build_unconditional_branch(nomultiply);
        builder.position_at_end(nomultiply);

        let result3 = builder.build_phi(ty, "result");
        result3.add_incoming(&[(&result.as_basic_value(), loop_block), (&result2, multiply)]);

        let exp2 = builder.build_right_shift(
            exp.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "exp",
        );
        let zero = builder.build_int_compare(IntPredicate::EQ, exp2, ty.const_zero(), "zero");

        builder.build_conditional_branch(zero, done, notdone);

        builder.position_at_end(done);

        builder.build_return(Some(&result3.as_basic_value()));

        builder.position_at_end(notdone);

        let base2 = if bit > 64 {
            builder
                .build_store(l, base.as_basic_value().into_int_value());
            builder
                .build_store(r, base.as_basic_value().into_int_value());

            builder.build_call(
                self.module.get_function("__mul32").unwrap(),
                &[
                    builder
                        .build_pointer_cast(
                            l,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    builder
                        .build_pointer_cast(
                            r,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    builder
                        .build_pointer_cast(
                            o,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    self.context
                        .i32_type()
                        .const_int(bit as u64 / 32, false)
                        .into(),
                ],
                "",
            );

            builder.build_load(o, "base").into_int_value()
        } else {
            builder.build_int_mul(
                base.as_basic_value().into_int_value(),
                base.as_basic_value().into_int_value(),
                "base",
            )
        };

        base.add_incoming(&[(&base2, notdone)]);
        result.add_incoming(&[(&result3.as_basic_value(), notdone)]);
        exp.add_incoming(&[(&exp2, notdone)]);

        builder.build_unconditional_branch(loop_block);

        builder.position_at_end(pos);

        function
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codegen() {
        let context = Context::create();
        let module = context.create_module("contract");
        let builder = context.create_builder();

         // (5 - 2) * 3
        let instrs = vec![
            // Instruction::Push(vec![0x80]),
            // Instruction::Push(vec![0x40]),
            // Instruction::MStore,
            // Instruction::CallValue,
            // Instruction::Dup(1),
            // Instruction::IsZero,
            // Instruction::Push(vec![0x00, 0x10]),
            // Instruction::JumpIf,
            // Instruction::Push(vec![0]),
            // Instruction::Dup(1),
            // Instruction::Revert,
            // Instruction::JumpDest,
            // Instruction::Pop,
            Instruction::Push(vec![0xA]),
            Instruction::Push(vec![0xB]),
            Instruction::Push(vec![0xC]),
            Instruction::Swap(2),
        ];
        let bytes = crate::evm_opcode::assemble_instructions(instrs);
        let instrs = crate::evm_opcode::Disassembly::from_bytes(&bytes).unwrap().instructions;

        let mut compiler = Compiler::new(&context, &builder, &module);
        compiler.compile(&builder, &instrs, &bytes, "test", false);
        // compiler.dbg();
        module.print_to_file("out.ll").unwrap();
    }
}
