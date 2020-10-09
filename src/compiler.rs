use crate::evm_opcode::Instruction;

use std::rc::Rc;
use std::cell::RefCell;

use inkwell::AddressSpace;
use inkwell::values::{FunctionValue, GlobalValue, IntMathValue, BasicValueEnum, VectorValue, IntValue};
use inkwell::types::{IntType, VectorType};
use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;

const DEBUG: bool = true;

fn nibble_to_u64(vals: &[u8]) -> Vec<u64> {
    vals.iter().map(|i| *i as u64).collect()
}

pub struct Compiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    label_stack: Rc<RefCell<Vec<&'static str>>>,

    i256_ty: IntType<'ctx>,
    sp: Option<GlobalValue<'ctx>>,
    stack: Option<GlobalValue<'ctx>>,
    fun: Option<FunctionValue<'ctx>>,
    dump_stack: Option<FunctionValue<'ctx>>,
}


impl<'a, 'ctx> Compiler<'a, 'ctx> {

    /// write module to ir file
    pub fn write_ir(&self, filename: &str) {
        self.module.print_to_file(filename);
    }

    /// Compile instructions
    pub fn compile(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a Module<'ctx>,
        instrs: &[Instruction],
    ) -> Self {
        let mut compiler = Self {
            context,
            i256_ty: context.custom_width_int_type(256),
            module,
            sp: None,
            stack: None,
            fun: None,
            dump_stack: None,
            label_stack: Rc::new(RefCell::new(Vec::new())),
        };

        compiler.build_stack(builder);
        for instr in instrs {
            compiler.build_instr(instr, builder);
        }
        compiler.build_ret(builder);
        compiler
    }

    /// Build ret void
    fn build_ret(&self, builder: &'a Builder<'ctx>) {
        builder.build_return(None);
    }

    /// Build stack related global variables
    fn build_stack(&mut self, builder: &'a Builder<'ctx>) {
        let i64_ty = self.context.i64_type();
        let i256_arr_ty = self.i256_ty.array_type(1024); // .zero (256 / 8 * size)

        // dump_stack
        let str_ty = inkwell::types::BasicTypeEnum::PointerType(self.context.i8_type().ptr_type(AddressSpace::Generic));
        let fn_ty = self.context.void_type().fn_type(&[str_ty],false);
        let dump_stack = self.module.add_function("dump_stack", fn_ty, Some(inkwell::module::Linkage::External));
        self.dump_stack = Some(dump_stack);

        let stack = self.module.add_global(i256_arr_ty, Some(AddressSpace::Generic), "stack");
        let sp = self.module.add_global(i64_ty, Some(AddressSpace::Generic), "sp");
        sp.set_initializer(&i64_ty.const_int(0, false));
        stack.set_initializer(&i256_arr_ty.const_zero());

        self.sp = Some(sp);
        self.stack = Some(stack);

        let fn_type = self.context.void_type().fn_type(&[], false);
        let function = self.module.add_function("contract", fn_type, None);


        let basic_block = self.context.append_basic_block(function, "entry");
        builder.position_at_end(basic_block);

        self.fun = Some(function);
    }

    /// Print IR
    pub fn dbg(&self) {
        self.module.print_to_stderr();
    }

    fn push_label(&self, name: &'static str, builder: &'a Builder<'ctx>) {
        let mut s = self.label_stack.borrow_mut();
        s.push(name);
        let lbl_name = s.join("_");
        let function = self.fun.unwrap();
        let basic_block = self.context.append_basic_block(function, &lbl_name);
        builder.build_unconditional_branch(basic_block);
        builder.position_at_end(basic_block);
    }

    fn pop_label(&self) {
        let mut s = self.label_stack.borrow_mut();
        s.pop();
    }

    /// Function call to dump_stack
    pub fn dump_stack(&self, builder: &'a Builder<'ctx>) {
        if DEBUG {
            let mut s = self.label_stack.borrow_mut();
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
        self.push_label("sp", builder);
        let sp_ptr = self.sp.unwrap().as_pointer_value();
        let sp = builder.build_load(sp_ptr, "sp").into_int_value();
        self.pop_label();
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
    fn build_peek(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>, n: u64) -> BasicValueEnum<'ctx> {
        self.push_label("peek", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(n, false),
            "sp");

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        let ret = builder.build_load(addr, "arr");

        self.pop_label();
        ret
    }

    /// Pop a value off stack
    fn build_pop(&self, builder: &'a Builder<'ctx>, sp: IntValue<'ctx>) -> BasicValueEnum<'ctx> {
        self.push_label("pop", builder);
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(1, false),
            "sp");
        let ret = self.build_peek(builder, sp, 0);
        builder.build_store(self.sp.unwrap().as_pointer_value(), sp); 
        self.pop_label();
        ret
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
    fn build_instr(&self, instr: &Instruction, builder: &'a Builder<'ctx>) -> BasicValueEnum<'ctx> {
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
            Instruction::IsZero |
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
            Instruction::CallValue |
            Instruction::CallDataLoad |
            Instruction::CallDataSize |
            Instruction::CallDataCopy |
            Instruction::CodeSize |
            Instruction::CodeCopy |
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
            Instruction::MLoad |
            Instruction::MStore |
            Instruction::MStore8 |
            Instruction::SLoad |
            Instruction::SStore |
            Instruction::Jump |
            Instruction::JumpIf |
            Instruction::PC |
            Instruction::MSize |
            Instruction::Gas |
            Instruction::JumpDest |
            Instruction::GasLimit |
            Instruction::Dup(_) |
            Instruction::Swap(_) |
            Instruction::Log(_) |
            Instruction::Create |
            Instruction::Call |
            Instruction::CallCode |
            Instruction::Return |
            Instruction::DelegateCall |
            Instruction::Create2 |
            Instruction::Revert |
            Instruction::StaticCall |
            Instruction::Invalid |
            Instruction::SelfDestruct => {
                BasicValueEnum::IntValue(self.context.i8_type().const_zero())
            }
            Instruction::Sub => {
                let name = "sub";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1).into_int_value();
                let b = self.build_peek(builder, sp, 2).into_int_value();
                let sp = self.build_decr(builder, sp, 2);

                self.pop_label();
                self.push_label("sub_actual", builder);
                let ret = builder.build_int_sub(a, b, name);
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, sp);
                self.dump_stack(builder);
                self.pop_label();
                value
            }
            Instruction::Div => {
                let name = "div";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1).into_int_value();
                let b = self.build_peek(builder, sp, 2).into_int_value();
                let sp = self.build_decr(builder, sp, 2);
                self.pop_label();

                self.push_label("div_actual", builder);
                let ret = builder.build_int_unsigned_div(a, b, name); // TODO: verify
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, sp);
                self.dump_stack(builder);
                self.pop_label();
                value
            }
            Instruction::Mul => {
                let name = "mul";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1).into_int_value();
                let b = self.build_peek(builder, sp, 2).into_int_value();
                let sp = self.build_decr(builder, sp, 2);
                self.pop_label();

                self.push_label("mul_actual", builder);
                let ret = builder.build_int_mul(a, b, name); // TODO: verify
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, sp);
                self.dump_stack(builder);
                self.pop_label();
                value
            }
            Instruction::Add => {
                let name = "add";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let a = self.build_peek(builder, sp, 1).into_int_value();
                let b = self.build_peek(builder, sp, 2).into_int_value();
                let sp = self.build_decr(builder, sp, 2);
                self.pop_label();

                self.push_label("add_actual", builder);
                let ret = builder.build_int_add(a, b, name);
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, sp);
                self.dump_stack(builder);
                self.pop_label();
                value
            }
            Instruction::Pop => {
                let name = "pop";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let ret = self.build_pop(builder, sp);
                self.dump_stack(builder);
                self.pop_label();
                ret
            }
            Instruction::Push(vals) => {
                self.push_label("push", builder);

                let sp = self.build_sp(builder);
                let value = self.i256_ty.const_int_arbitrary_precision(&nibble_to_u64(vals));
                self.build_push(builder, BasicValueEnum::IntValue(value), sp);
                self.dump_stack(builder);
                self.pop_label();
                return BasicValueEnum::IntValue(value);
            }
        }
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
        let function = context.create_builder();

         // (5 - 2) * 3
        let instrs = vec![
            Instruction::Push(vec![2]),
            Instruction::Push(vec![5]),
            Instruction::Sub,
        ];

        let compiler = Compiler::compile(&context, &builder, &module, &instrs);
        // compiler.dbg();
        compiler.write_ir("out.ll");
    }
}
