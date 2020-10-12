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

const DEBUG: bool = true;

fn nibble_to_u64(vals: &[u8]) -> Vec<u64> {
    let mut ret = vec![];
    for values in vals.chunks(32) {
        let mut out = 0;
        for &i in values {
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
    dump_stack: Option<FunctionValue<'ctx>>,
    jumpbb: Option<BasicBlock<'ctx>>,
    errbb: Option<BasicBlock<'ctx>>,

    jumpdests: BTreeMap<usize, BasicBlock<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {

    /// write module to ir file
    pub fn write_ir(&self, filename: &str) {
        self.module.print_to_file(filename).unwrap();
    }

    /// Compile instructions
    pub fn compile(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a Module<'ctx>,
        instrs: &[(usize, Instruction)],
        payload: &[u8],
    ) -> Self {
        let mut compiler = Self {
            context,
            i256_ty: context.custom_width_int_type(256),
            module,
            sp: None,
            stack: None,
            mem: None,
            code: None,
            fun: None,
            dump_stack: None,
            jumpdests: BTreeMap::new(),
            jumpbb: None,
            errbb: None,
            label_stack: Rc::new(RefCell::new(Vec::new())),
        };

        compiler.build_globals(builder, payload);

        // entry
        let entrybb = compiler.context.append_basic_block(compiler.fun.unwrap(), "entry");
        builder.position_at_end(entrybb);

        // err
        compiler.errbb = Some(compiler.context.append_basic_block(compiler.fun.unwrap(), "err"));


        // jump table
        compiler.jumpbb = Some(compiler.context.append_basic_block(compiler.fun.unwrap(), "jumpbb"));
        let mainbb = compiler.context.append_basic_block(compiler.fun.unwrap(), "main");
        for (offset, _dest) in instrs.iter()
            .filter(|(_,i)|*i==Instruction::JumpDest)
        {
            let jumpdestbb = compiler.context.append_basic_block(compiler.fun.unwrap(), "jumpdest");
            compiler.jumpdests.insert(*offset, jumpdestbb);
        }
        compiler.build_jumpbb(builder);


        // entry br to main
        builder.position_at_end(entrybb);
        builder.build_unconditional_branch(mainbb);

        // err ret
        builder.position_at_end(compiler.errbb.unwrap());
        compiler.build_ret(builder);

        // position to main
        builder.position_at_end(mainbb);

        for (offset, instr) in instrs {
            compiler.build_instr(*offset, instr, builder);
        }
        compiler.build_ret(builder);
        compiler
    }

    fn build_jumpbb(&self, builder: &'a Builder<'ctx>) {
        builder.position_at_end(self.jumpbb.unwrap());
        let sp = self.build_sp(builder);
        let dest = self.build_peek(builder, sp, 1);
        let _sp = self.build_decr(builder, sp, 2);
        let cases = self.jumpdests.iter()
            .map(|(offset, bb)|
                (self.i256_ty.const_int(*offset as u64, false), *bb)).collect::<Vec<_>>();
        builder.build_switch(dest, self.errbb.unwrap(), &cases);
    }

    /// Build ret void
    fn build_ret(&self, builder: &'a Builder<'ctx>) {
        builder.build_return(None);
    }

    /// Build stack related global variables
    fn build_globals(&mut self, _builder: &'a Builder<'ctx>, payload: &[u8]) {
        let i64_ty = self.context.i64_type();
        let i256_arr_ty = self.i256_ty.array_type(1024); // .zero (256 / 8 * size)

        // dump_stack
        let str_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[str_ty],false);
        let dump_stack = self.module.add_function("dump_stack", fn_ty, Some(inkwell::module::Linkage::External));
        self.dump_stack = Some(dump_stack);

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
            "code");
        let payload = self.context.const_string(payload, false);
        code.set_initializer(&payload);


        self.stack = Some(stack);
        self.sp = Some(sp);
        self.mem = Some(mem);
        self.code = Some(code);

        let fn_type = self.context.void_type().fn_type(&[], false);
        let function = self.module.add_function("contract", fn_type, None);

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
            builder.build_call(self.dump_stack.unwrap(), &[s.into()], "dump");
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
    fn build_peek(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>, n: u64) -> IntValue<'ctx> {
        self.push_label("peek", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(n, false),
            "sp");

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        let ret = builder.build_load(addr, "arr").into_int_value();

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
        let ret = self.build_peek(builder, sp, 0);
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
    fn build_instr(&self, offset: usize, instr: &Instruction, builder: &'a Builder<'ctx>) {
        dbg!((offset, instr));
        match instr {
            Instruction::Stop |
            Instruction::SDiv |
            Instruction::Mod |
            Instruction::SMod |
            Instruction::AddMod |
            Instruction::MulMod |
            Instruction::Exp |
            Instruction::SignExtend |
            Instruction::Lt |
            Instruction::Gt |
            Instruction::SLt |
            Instruction::SGt |
            Instruction::EQ |
            Instruction::And |
            Instruction::Or |
            Instruction::Xor |
            Instruction::Not |
            Instruction::Byte |
            Instruction::Shl |
            Instruction::Shr |
            Instruction::Sar |
            Instruction::Sha3 |
            Instruction::Addr |
            Instruction::Balance |
            Instruction::Origin |
            Instruction::Caller |
            Instruction::CallDataLoad |
            Instruction::CallDataSize |
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
            Instruction::SStore |
            Instruction::Jump |
            Instruction::PC |
            Instruction::MSize |
            Instruction::Gas |
            Instruction::GasLimit |
            Instruction::Swap(_) |
            Instruction::Log(_) |
            Instruction::Create |
            Instruction::Call |
            Instruction::CallCode |
            Instruction::DelegateCall |
            Instruction::Create2 |
            Instruction::StaticCall |
            Instruction::Invalid |
            Instruction::SelfDestruct => {
                unimplemented!("{:#?}", instr);
            }
            Instruction::Return => {
                let name = "return";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let _length = self.build_peek(builder, sp, 2);
                let _offset = self.build_peek(builder, sp, 1);
                let _sp = self.build_decr(builder, sp, 2);
            }
            Instruction::CodeCopy => {
                let name = "codecopy";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let length = self.build_peek(builder, sp, 3);
                let offset = self.build_peek(builder, sp, 2);
                let dest_offset = self.build_peek(builder, sp, 1);
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
                let cond = self.build_peek(builder, sp, 2);
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
                let value = self.build_peek(builder, sp, *n as u64 + 1).into();
                self.build_push(builder, value, sp);
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
                let offset = self.build_peek(builder, sp, 1);
                let value = self.build_peek(builder, sp, 2);
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
                let a = self.build_peek(builder, sp, 1);
                let b = self.build_peek(builder, sp, 2);
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_sub(a, b, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Div => {
                let name = "div";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1);
                let b = self.build_peek(builder, sp, 2);
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_unsigned_div(a, b, name).into();
                self.build_push(builder, value, sp);
            }
            Instruction::Mul => {
                let name = "mul";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1);
                let b = self.build_peek(builder, sp, 2);
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_mul(a, b, name).into(); // TODO: verify
                self.build_push(builder, value, sp);
            }
            Instruction::Add => {
                let name = "add";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1);
                let b = self.build_peek(builder, sp, 2);
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_add(a, b, name).into();
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
                let value = self.i256_ty.const_int_arbitrary_precision(&nibble_to_u64(vals)).into();
                self.build_push(builder, value, sp);
            }
        };
        self.build_dump_stack(builder);
        self.pop_label();
    }

    pub fn codegen(instrs: &[(usize, Instruction)], payload: &[u8]) {
        let context = Context::create();
        let module = context.create_module("contract");
        let builder = context.create_builder();

        let compiler = Compiler::compile(&context, &builder, &module, &instrs, payload);
        // compiler.dbg();
        compiler.write_ir("out.ll");
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
            Instruction::Push(vec![0x80]),
            Instruction::Push(vec![0x40]),
            Instruction::MStore,
            Instruction::CallValue,
            Instruction::Dup(1),
            Instruction::IsZero,
            Instruction::Push(vec![0x00, 0x10]),
            Instruction::JumpIf,
            Instruction::Push(vec![0]),
            Instruction::Dup(1),
            Instruction::Revert,
            Instruction::JumpDest,
            Instruction::Pop,
        ];
        let bytes = crate::evm_opcode::assemble_instructions(instrs);
        let instrs = crate::evm_opcode::Disassembly::from_bytes(&bytes).unwrap().instructions;

        let compiler = Compiler::compile(&context, &builder, &module, &instrs);
        // compiler.dbg();
        compiler.write_ir("out.ll");
    }
}
