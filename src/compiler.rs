use crate::evm_opcode::Instruction;

use inkwell::AddressSpace;
use inkwell::values::{FunctionValue, GlobalValue, IntMathValue, BasicValueEnum};
use inkwell::types::{IntType};
use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;

fn nibble_to_u64(vals: &[u8]) -> Vec<u64> {
    vals.iter().map(|i| *i as u64).collect()
}

pub struct Compiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,

    i256_ty: IntType<'ctx>,
    sp: Option<GlobalValue<'ctx>>,
    stack: Option<GlobalValue<'ctx>>,
    fun: Option<FunctionValue<'ctx>>,
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
        };

        compiler.build_stack(builder);
        for instr in instrs {
            compiler.build_instr(instr, builder);
        }
        compiler.build_ret(builder);
        compiler.dbg();
        compiler
    }

    /// Build ret void
    pub fn build_ret(&self, builder: &'a Builder<'ctx>) {
        builder.build_return(None);
    }

    /// Build stack related global variables
    pub fn build_stack(&mut self, builder: &'a Builder<'ctx>) {
        let i64_ty = self.context.i64_type();
        let i256_arr_ty = self.i256_ty.array_type(1024); // .zero (256 / 8 * size)

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

    /// Insert label for section
    pub fn label(&self, name: &str, builder: &'a Builder<'ctx>) {
        let function = self.fun.unwrap();
        let basic_block = self.context.append_basic_block(function, name);
        builder.build_unconditional_branch(basic_block);
        builder.position_at_end(basic_block);
    }

    /// Pop a value off stack
    pub fn build_pop(&self, builder: &'a Builder<'ctx>, label: &str) -> BasicValueEnum<'ctx> {
        self.label(&format!("{}_pop", label), builder);
        let sp_ptr = self.sp.unwrap().as_pointer_value();
        let sp = builder.build_load(sp_ptr, "sp").into_int_value();
        let sp = builder.build_int_sub(
            sp,
            self.context.i64_type().const_int(1, false),
            "sp");
        builder.build_store(sp_ptr, sp); 

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        let ret = builder.build_load(addr, "arr");
        ret
    }

    /// Push a value onto stack
    pub fn build_push(&self, builder: &'a Builder<'ctx>, value: BasicValueEnum<'ctx>, label: &str) -> BasicValueEnum<'ctx> {
        self.label(&format!("{}_push", label), builder);
        let sp_ptr = self.sp.unwrap().as_pointer_value();
        let sp = builder.build_load(sp_ptr, "sp").into_int_value();

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        builder.build_store(addr, value);

        let sp = builder.build_int_add(
            sp,
            self.context.i64_type().const_int(1, false),
            "sp");
        builder.build_store(sp_ptr, sp); 
        value
    }

    /// Build instruction
    pub fn build_instr(&self, instr: &Instruction, builder: &'a Builder<'ctx>) -> BasicValueEnum<'ctx> {
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
                self.label("sub", builder);
                let a = self.build_pop(builder, "sub").into_int_value();
                let b = self.build_pop(builder, "sub").into_int_value();

                self.label("sub_actual", builder);
                let ret = builder.build_int_sub(a, b, "sub");
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, "sub");
                value
            }
            Instruction::Div => {
                self.label("div", builder);
                let a = self.build_pop(builder, "div").into_int_value();
                let b = self.build_pop(builder, "div").into_int_value();

                self.label("div_actual", builder);
                let ret = builder.build_int_unsigned_div(a, b, "div"); // TODO: verify
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, "div");
                value
            }
            Instruction::Mul => {
                self.label("mul", builder);
                let a = self.build_pop(builder, "mul").into_int_value();
                let b = self.build_pop(builder, "mul").into_int_value();

                self.label("mul_actual", builder);
                let ret = builder.build_int_mul(a, b, "mul"); // TODO: verify
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, "mul");
                value
            }
            Instruction::Add => {
                self.label("add", builder);
                let a = self.build_pop(builder, "add").into_int_value();
                let b = self.build_pop(builder, "add").into_int_value();

                self.label("add_actual", builder);
                let ret = builder.build_int_add(a, b, "add");
                let value = BasicValueEnum::IntValue(ret);
                self.build_push(builder, value, "add");
                value
            }
            Instruction::Pop => {
                self.label("pop", builder);
                self.build_pop(builder, "")
            }
            Instruction::Push(vals) => {
                self.label("push", builder);

                let value = self.i256_ty.const_int_arbitrary_precision(&nibble_to_u64(vals));
                self.build_push(builder, BasicValueEnum::IntValue(value), "");
                BasicValueEnum::IntValue(value)
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

        let instrs = vec![
            Instruction::Push(vec![2]), // (5 - 2) * 3
            Instruction::Push(vec![5]),
            Instruction::Sub,
        ];

        let ret = Compiler::compile(&context, &builder, &module, &instrs);
        ret.write_ir("out.ll");
    }
}
