use crate::evm::Instruction;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::string::String;

use inkwell::AddressSpace;
use inkwell::values::{FunctionValue, GlobalValue, BasicValueEnum, IntValue, PointerValue};
use inkwell::types::{IntType, BasicTypeEnum};
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

use crate::ethabi::{Function, Constructor, Contract, param_type::ParamType::*, ParamType};

use log::{info, warn, error, debug};

fn param_type_size(kind: &ParamType) -> u64 {
    match kind {
        Bool | Int(_) | Uint(_) | Address => 32,
        crate::ethabi::ParamType::String | Array(_) | Bytes => {
            error!("solenoid does not support dynamic sized input yet");
            0
        }
        FixedArray(ty, n) => (*n as u64) * param_type_size(ty),
        Tuple(_) | FixedBytes(_) => {
            error!("unimpl {:?}", kind);
            0
        }
    }
}


fn nibble2i256(vals: &[u8]) -> Vec<u64> {
    let mut ret = vec![];
    let mut vals = vals.to_vec();
    vals.reverse();
    for values in vals.chunks(32) {
        for nibbles in values.chunks(8) {
            let mut out = 0;
            for &i in nibbles.iter().rev() {
                out = out << 8 | i as u64;
            }
            ret.push(out);
        }
    }
    ret
}

pub struct Compiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    label_stack: Rc<RefCell<Vec<&'static str>>>,

    i256_ty: IntType<'ctx>,
    sp: Option<GlobalValue<'ctx>>,
    pc: Option<GlobalValue<'ctx>>,
    stack: Option<GlobalValue<'ctx>>,
    mem: Option<GlobalValue<'ctx>>,
    code: Option<GlobalValue<'ctx>>,
    code_ptr: Option<GlobalValue<'ctx>>,
    code_size: u64,
    fun: Option<FunctionValue<'ctx>>,
    jumpbb: Option<BasicBlock<'ctx>>,
    errbb: Option<BasicBlock<'ctx>>,

    jumpdests: BTreeMap<usize, BasicBlock<'ctx>>,
    debug: bool,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn compile_abi(&self, builder: &'a Builder<'ctx>, contract: &Contract, contract_name: &str) {
        let inputs = contract.constructor.as_ref().map(|i|i.inputs.to_owned()).unwrap_or(vec![]);
        let fun = Function {
            name: "constructor".to_owned(),
            inputs,
            outputs: vec![],
            constant: false,
        };
        self.compile_abi_function(builder, contract_name, &fun, 0, true);
        for (_name, funs) in &contract.functions {
            for (idx, fun) in funs.iter().enumerate() {
                self.compile_abi_function(builder, contract_name, fun, idx, false);
            }
        }
    }
    
    /// void get(char* out_buf, int* buf_length, params..)
    fn compile_abi_function(&self, builder: &'a Builder<'ctx>, contract_name: &str, fun: &Function, idx: usize, is_ctor: bool) {
        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let buf_len_ty = self.context.i32_type().ptr_type(AddressSpace::Generic).into();
        let mut param_types: Vec<BasicTypeEnum<'ctx>> = vec![char_ptr_ty, buf_len_ty];

        for param in &fun.inputs {

            let ty = match param.kind {
                Address => char_ptr_ty,
                Bytes => char_ptr_ty,

                Int(8) => self.context.i8_type().into(),
                Int(16) => self.context.i16_type().into(),
                Int(32) => self.context.i32_type().into(),
                Int(64) => self.context.i64_type().into(),
                Int(_) => char_ptr_ty,

                Uint(8) => self.context.i8_type().into(),
                Uint(16) => self.context.i16_type().into(),
                Uint(32) => self.context.i32_type().into(),
                Uint(64) => self.context.i64_type().into(),
                Uint(_) => char_ptr_ty,

                Bool => self.context.i32_type().into(),
                String => char_ptr_ty,
                Array(_) => char_ptr_ty,
                FixedBytes(_) => char_ptr_ty,
                FixedArray(_, _) => char_ptr_ty,
                Tuple(_) => char_ptr_ty,
            };
            param_types.push(ty);
        }

        let fun_name = Self::format_abi_fn_name(contract_name, fun, idx);
        let fn_ty = self.context.void_type().fn_type(param_types.as_slice(),false);
        let llvm_fun = self.module.add_function(&fun_name, fn_ty, None);
        let basic_block = self.context.append_basic_block(llvm_fun, "entry");

        builder.position_at_end(basic_block);
        self.build_abi_conversion(builder, contract_name, fun, llvm_fun, is_ctor);
        builder.build_return(None);
    }

    fn build_abi_conversion(&self, builder: &'a Builder<'ctx>, contract_name: &str, fun: &Function, llvm_fun: FunctionValue<'ctx>, is_ctor: bool) {
        let buf = llvm_fun.get_nth_param(0).unwrap().into_pointer_value();
        let len_ptr = llvm_fun.get_nth_param(1).unwrap().into_pointer_value();
        builder.build_store(len_ptr, self.i32(0));
        let mut len  = builder.build_load(len_ptr, "len").into_int_value();

        if is_ctor {
            let code = self.module.get_global(&format!("{}_code", contract_name)).unwrap().as_pointer_value();
            builder.build_memcpy(buf, 1, code, 1, self.i32(self.code_size)).unwrap();
            builder.build_store(len_ptr, self.i32(self.code_size));
            len = builder.build_int_add(len, self.i32(self.code_size), "len");
        } else {
            // encode abi signature
            let sig = fun.short_signature();
            let sig_glb = self.module.add_global(
                self.context.i8_type().array_type(sig.len() as u32),
                Some(AddressSpace::Generic),
                &format!("{}_sig", fun.name));
            let sig_buf = self.context.const_string(&sig, false);
            sig_glb.set_initializer(&sig_buf);
            builder.build_memcpy(buf, 1, sig_glb.as_pointer_value(), 1, self.i32(sig.len() as u64)).unwrap();
            len = builder.build_int_add(len, self.i32(4), "len");
        }

        for (idx, param) in fun.inputs.iter().enumerate() {
            let x = llvm_fun.get_nth_param(idx as u32 + 2).unwrap();
            match &param.kind {
                Uint(8) | Uint(16) | Uint(32) | Uint(64) |
                Int(8) | Int(16) | Int(32) | Int(64) | Bool => {
                    let x = x.into_int_value();
                    let value = builder.build_int_z_extend(x, self.i256_ty, &param.name);
                    let ptr = unsafe { builder.build_gep(buf, &[len], "ptr") };
                    let ptr = builder.build_pointer_cast(ptr, self.i256_ty.ptr_type(AddressSpace::Generic), "ptr");
                    builder.build_store(ptr, value);
                    len = builder.build_int_add(len, self.i32(32), "len");
                },
                Uint(bits) | Int(bits) => {
                    let x = x.into_pointer_value();
                    let val_ptr = builder.build_pointer_cast(x, self.context.custom_width_int_type(*bits as u32).ptr_type(AddressSpace::Generic), "ptr");
                    let value = builder.build_load(val_ptr, "value").into_int_value();
                    let ptr = unsafe { builder.build_gep(buf, &[len], "ptr") };
                    let value = builder.build_int_z_extend(value, self.i256_ty, &param.name);
                    let ptr = builder.build_pointer_cast(ptr, self.i256_ty.ptr_type(AddressSpace::Generic), "ptr");
                    builder.build_store(ptr, value);
                    len = builder.build_int_add(len, self.i32(32), "len");
                }
                Address => {
                    let x = x.into_pointer_value();
                    len = builder.build_int_add(len, self.i32(12), "len");
                    let ptr = unsafe { builder.build_gep(buf, &[len], "ptr") };
                    builder.build_memcpy(ptr, 1, x, 1, self.i32(20));
                    len = builder.build_int_add(len, self.i32(20), "len");
                }
                _ => {
                    error!("unimpl {:?}", &param.kind);
                }
            }
        }

        builder.build_store(len_ptr, len);
    }

    pub fn format_abi_fn_name(contract_name: &str, fun: &Function, idx: usize) -> String {
        if idx == 0 {
            format!("abi_{}_{}", contract_name, fun.name)
        } else {
            format!("abi_{}_{}_{}", contract_name, fun.name, idx)
        }
    }
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Compile instructions
    pub fn new(
        context: &'ctx Context,
        module: &'a Module<'ctx>,
        debug: bool,
    ) -> Self {
        let compiler = Self {
            context,
            i256_ty: context.custom_width_int_type(256),
            module,
            sp: None,
            pc: None,
            stack: None,
            mem: None,
            code: None,
            code_ptr: None,
            code_size: 0,
            fun: None,
            jumpdests: BTreeMap::new(),
            jumpbb: None,
            errbb: None,
            label_stack: Rc::new(RefCell::new(Vec::new())),
            debug,
        };
        compiler
    }

    fn calc_ctor_params_size(ctor: &Constructor) -> u64 {
        let mut ret = 0;
        for param in &ctor.inputs {
            ret += param_type_size(&param.kind);
        } 
        ret
    }

    pub fn compile(&mut self,
        builder: &'a Builder<'ctx>,
        instrs: &[(usize, Instruction)],
        payload: &[u8],
        name: &str,
        is_runtime: bool,
    ) {
        if !is_runtime {
            self.build_globals(payload, name, is_runtime);
        }

        self.build_function(name, is_runtime);

        // entry
        let entrybb = self.context.append_basic_block(self.fun.unwrap(), "entry");
        builder.position_at_end(entrybb);

        // err
        self.errbb = Some(self.context.append_basic_block(self.fun.unwrap(), "err"));


        // jump table
        self.jumpbb = Some(self.context.append_basic_block(self.fun.unwrap(), "jumpbb"));
        let mainbb = self.context.append_basic_block(self.fun.unwrap(), "main");
        let jumpdests = instrs.iter()
            // .take_while(|(_, i)| *i != Instruction::Invalid)
            .filter(|(_,i)|*i==Instruction::JumpDest);
        self.jumpdests.clear();
        for (offset, _i) in jumpdests {
            let jumpdestbb = self.context.append_basic_block(self.fun.unwrap(), "jumpdest");
            self.jumpdests.insert(*offset, jumpdestbb);
        }
        self.build_jumpbb(builder);


        // entry br to main
        builder.position_at_end(entrybb);
        builder.build_unconditional_branch(mainbb);

        // err ret
        builder.position_at_end(self.errbb.unwrap());
        self.build_errbb(builder);

        // position to main
        builder.position_at_end(mainbb);
        // set sp = 0, TODO: is it valid in cross contract calls?
        // builder.build_store(self.sp.unwrap().as_pointer_value(), self.i64(0));

        // set code_ptr for runtime
        if !is_runtime {
            let code_ptr = self.code_ptr.unwrap().as_pointer_value();
            let value = self.fun.unwrap().get_nth_param(0).unwrap().into_pointer_value();
            builder.build_store(code_ptr, value);
        }

        for (offset, instr) in instrs {
            if Option::None == self.build_instr(*offset, instr, builder, is_runtime) {
                info!("Stopping compilation early.");
                break;
            }
        }
        builder.build_return(None);
    }

    fn build_errbb(&self, builder: &'a Builder<'ctx>) {
        builder.build_call(self.revert(), &[], "revert");
        builder.build_return(None);
    }

    fn build_jumpbb(&self, builder: &'a Builder<'ctx>) {
        builder.position_at_end(self.jumpbb.unwrap());
        let sp = self.build_sp(builder);
        let (dest, _sp) = self.build_pop(builder, sp);
        let cases = self.jumpdests.iter()
            .map(|(offset, bb)|
                (self.i256(*offset), *bb)).collect::<Vec<_>>();
        builder.build_switch(dest, self.errbb.unwrap(), &cases);
    }

    fn sdiv256(&self) -> FunctionValue<'ctx> {
        let name = "sdiv256";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[ty, ty, ty], false);
        let sdiv256 = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        sdiv256
    }

    fn udiv256(&self) -> FunctionValue<'ctx> {
        let name = "udiv256";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[ty, ty, ty], false);
        let udiv256 = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        udiv256
    }

    fn powmod(&self) -> FunctionValue<'ctx> {
        let name = "powmod";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[ty, ty, ty], false);
        let powmod = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        powmod
    }

    fn sha3(&self) -> FunctionValue<'ctx> {
        let name = "keccak256";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let fn_ty = self.context.void_type().fn_type(
            &[
                    char_ptr_ty.into(),
                    self.context.i16_type().into(),
                    char_ptr_ty.into(),
                ],
                false);
        let sha3 = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));

        sha3
    }

    fn revert(&self) -> FunctionValue<'ctx> {
        let name = "revert";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let fn_ty = self.context.void_type().fn_type(&[],false);
        let revert = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        revert
    }


    fn sstore(&self) -> FunctionValue<'ctx> {
        let name = "sstore";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[char_ptr_ty, char_ptr_ty, char_ptr_ty],false);
        let sstore = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        sstore
    }

    fn sload(&self) -> FunctionValue<'ctx> {
        let name = "sload";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[char_ptr_ty, char_ptr_ty],false);
        let sload = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        sload
    }

    fn swap_endianness(&self) -> FunctionValue<'ctx> {
        let name = "swap_endianness";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_ty = self.context.void_type().fn_type(&[char_ptr_ty],false);
        let swap_endianness = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));
        swap_endianness
    }

    fn storage_ptr(&self) -> PointerValue<'ctx> {
        self.fun.unwrap().get_nth_param(4).unwrap().into_pointer_value()
    }

    fn dump_stack(&self) -> FunctionValue<'ctx> {
        let name = "dump_stack";
        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let char_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let int_ty = self.context.i64_type().into();
        let fn_ty = self.context.void_type().fn_type(&[char_ptr_ty, int_ty, int_ty, char_ptr_ty, char_ptr_ty],false);
        let dump_stack = self.module.add_function(name, fn_ty, Some(inkwell::module::Linkage::External));

        // TODO:
        // let readonly = self.context.create_string_attribute("readonly", "true");
        // dump_stack.add_attribute(inkwell::attributes::AttributeLoc::Function, readonly);
        dump_stack
    }

    fn code(&self, builder: &'a Builder<'ctx>, is_runtime: bool) -> PointerValue<'ctx> {
        if !is_runtime {
            self.fun.unwrap().get_nth_param(0).unwrap().into_pointer_value()
        } else {
            self.code_ptr.unwrap().as_pointer_value()
        }
    }

    /// Build stack related global variables
    fn build_globals(&mut self, payload: &[u8], contract_name: &str, is_runtime: bool) {
        let i64_ty = self.context.i64_type();
        let i256_arr_ty = self.i256_ty.array_type(1024); // .zero (256 / 8 * size)

        // stack
        let stack = self.module.add_global(i256_arr_ty, Some(AddressSpace::Generic), &format!("{}_stack", contract_name));
        stack.set_initializer(&i256_arr_ty.const_zero());
        self.stack = Some(stack);

        // sp
        let sp = self.module.add_global(i64_ty, Some(AddressSpace::Generic), &format!("{}_sp", contract_name));
        sp.set_initializer(&i64_ty.const_zero());
        self.sp = Some(sp);

        // pc
        let pc = self.module.add_global(i64_ty, Some(AddressSpace::Generic), &format!("{}_pc", contract_name));
        pc.set_initializer(&i64_ty.const_zero());
        self.pc = Some(pc);

        // mem
        let i8_array_ty = self.context.i8_type().array_type(1024 * 32);
        let mem = self.module.add_global(i8_array_ty, Some(AddressSpace::Generic), &format!("{}_mem", contract_name));
        mem.set_initializer(&i8_array_ty.const_zero());
        self.mem = Some(mem);

        // code
        self.code_size = payload.len() as u64;
        let code = self.module.add_global(
            self.context.i8_type().array_type(payload.len() as u32),
            Some(AddressSpace::Generic),
            &format!("{}_code", contract_name));
        let payload = self.context.const_string(payload, false);
        code.set_initializer(&payload);
        self.code = Some(code);

        // code_ptr
        let code_ptr = self.module.add_global(
            self.context.i8_type().ptr_type(AddressSpace::Generic),
            Some(AddressSpace::Generic),
            &format!("{}_code_ptr", contract_name));
        code_ptr.set_initializer(&self.context.i8_type().ptr_type(AddressSpace::Generic).const_null());
        self.code_ptr = Some(code_ptr);
    }

    pub fn build_function(&mut self, name: &str, is_runtime: bool) {
        let msg_len = self.context.i64_type().into();
        let ret_offset = self.context.i64_type().ptr_type(AddressSpace::Generic).into();
        let ret_len = self.context.i64_type().ptr_type(AddressSpace::Generic).into();
        let msg = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let storage = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let caller = self.context.i8_type().ptr_type(AddressSpace::Generic).into();
        let fn_type = self.context.void_type()
            .fn_type(
                &[msg, msg_len, ret_offset, ret_len, storage, caller],
                false
            );
        let fn_name = Self::format_fn_name(name, is_runtime);
        let function = self.module.add_function(&fn_name, fn_type, None);
        self.fun = Some(function);
    }

    pub fn format_fn_name(name: &str, is_runtime: bool) -> String {
        // let name = "contract";
        let fn_name = format!("{}_{}", name, if is_runtime {"runtime"} else {"constructor"});
        fn_name
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
        if !self.debug {
            return;
        }
        let s = self.label_stack.borrow_mut();
        let lbl_name = s.join("_");
        let s = unsafe {
            builder.build_global_string(&lbl_name, "str")
                .as_pointer_value()
                .const_cast(
                    self.context.i8_type().ptr_type(AddressSpace::Generic)) };
        let sp = builder.build_load(self.sp.unwrap().as_pointer_value(), "sp");
        let pc = builder.build_load(self.pc.unwrap().as_pointer_value(), "pc");
        let stack = unsafe { builder.build_gep(self.stack.unwrap().as_pointer_value(),&[self.i32(0), self.i32(0)], "stack") };
        let stack = builder.build_pointer_cast(stack, self.context.i8_type().ptr_type(AddressSpace::Generic), "stack");
        let mem = unsafe { builder.build_gep(self.mem.unwrap().as_pointer_value(),&[self.i32(0), self.i32(0)], "mem") };
        let mem = builder.build_pointer_cast(mem, self.context.i8_type().ptr_type(AddressSpace::Generic), "mem");
        builder.build_call(self.dump_stack(), &[s.into(), sp, pc, stack.into(), mem.into()], "dump");
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
            self.i64(n),
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
            self.i64(n),
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
            self.i64(n),
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
            self.i64(1),
            "sp");
        let ret = self.build_peek(builder, sp, 0, "ret");
        builder.build_store(self.sp.unwrap().as_pointer_value(), sp);
        self.pop_label();
        (ret, sp)
    }

    /// Push a value onto stack
    fn build_push(&self, builder: &'a Builder<'ctx>, value: BasicValueEnum<'ctx>, sp: IntValue<'ctx>) -> IntValue<'ctx> {
        self.push_label("push", builder);

        let stack = self.stack.unwrap().as_pointer_value();
        let addr = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
        builder.build_store(addr, value);
        let sp = self.build_incr(builder, sp, 1);
        self.pop_label();
        sp
    }

    /// return char pointer to tos
    fn build_tos_ptr(&self, builder: &'a Builder<'ctx>, idx: u64) -> PointerValue<'ctx> {
        let sp = self.build_sp(builder);
        let stack = self.stack.unwrap().as_pointer_value();
        let tos = builder.build_int_sub(sp, self.i64(idx), "sp_p_1");
        let key_ptr = unsafe { builder.build_in_bounds_gep(stack, &[self.i64(0), tos], "stack") };
        let key_ptr_i8 = builder.build_pointer_cast(key_ptr, self.context.i8_type().ptr_type(AddressSpace::Generic), "key");
        key_ptr_i8
    }

    fn i256(&self, i: usize) -> IntValue<'ctx> {
        self.i256_ty.const_int(i as u64, false)
    }

    fn i32(&self, i: u64) -> IntValue<'ctx> {
        self.context.i32_type().const_int(i as u64, false)
    }

    fn i64(&self, i: u64) -> IntValue<'ctx> {
        self.context.i64_type().const_int(i as u64, false)
    }

    /// Build instruction
    fn build_instr(&self, offset: usize, instr: &Instruction, builder: &'a Builder<'ctx>, is_runtime: bool) -> Option<()> {
        debug!("{:?}", (offset, instr));

        builder.build_store(self.pc.unwrap().as_pointer_value(), self.i64(offset as u64));
        match instr {
            Instruction::Addr |
            Instruction::Balance |
            Instruction::CallDataCopy |
            Instruction::GasPrice |
            Instruction::ChainId |
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
            Instruction::PC |
            Instruction::MSize |
            Instruction::Gas |
            Instruction::GasLimit |
            Instruction::Create |
            Instruction::Call |
            Instruction::CallCode |
            Instruction::DelegateCall |
            Instruction::Create2 |
            Instruction::StaticCall => {
                error!("unimpl: {:?}", instr);
            }
            Instruction::Origin |
            Instruction::Caller => {
                let name = "caller";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let tos = self.build_tos_ptr(builder, 0);
                let x = self.fun.unwrap().get_nth_param(5).unwrap().into_pointer_value();
                let ptr = unsafe { builder.build_gep(tos, &[self.i32(12)], "ptr") };
                builder.build_memcpy(ptr, 1, x, 1, self.i32(20));
                builder.build_call(self.swap_endianness(), &[tos.into()], "pos");
                self.build_incr(builder, sp, 1);
            }
            Instruction::CodeSize => {
                let name = "codesize";
                warn!("{} is unaudited", name);
                if is_runtime {
                    unimplemented!()
                } else {
                    self.push_label(name, builder);
                    let sp = self.build_sp(builder);
                    let int_value = self.fun.unwrap().get_nth_param(1).unwrap().into_int_value();
                    let value = builder.build_int_z_extend(int_value, self.i256_ty, "value").into();
                    self.build_push(builder, value, sp);
                }
            }
            Instruction::SignExtend => {
                let name = "signextend";
                warn!("{} is unaudited", name);
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let x = self.build_peek(builder, sp, 2, "x");
                let b = self.build_peek(builder, sp, 1, "b");
                let sp = self.build_decr(builder, sp, 2);

                let end = self.context.insert_basic_block_after(builder.get_insert_block().unwrap(), &format!("sext_end"));

                let mut cases = vec![];
                for i in 1..32 {
                    let bb = self.context.insert_basic_block_after(builder.get_insert_block().unwrap(), &format!("sext_{}", i));
                    cases.push((self.i256(i), bb));
                }
                builder.build_switch(b, self.errbb.unwrap(), &cases);
                for (i, (_, bb)) in cases.iter().enumerate() {
                    builder.position_at_end(*bb);
                    let x = builder.build_int_truncate(x, self.context.custom_width_int_type((i+1) as u32*8), "val");
                    let value = builder.build_int_s_extend(x, self.i256_ty, "sext");
                    self.build_push(builder, value.into(), sp);
                    builder.build_unconditional_branch(end);
                }
                builder.position_at_end(end);
                // TODO:
                // build a switch then use sext .. to ..
            }
            Instruction::SLoad =>  {
                let name = "sload";
                self.push_label(name, builder);
                let tos = self.build_tos_ptr(builder, 1);
                builder.build_call(self.sload(), &[self.storage_ptr().into(), tos.into()], "sload");
                // no increment because value overwrites key
            }
            Instruction::SStore => {
                let name = "sstore";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);

                let key_ptr_i8 = self.build_tos_ptr(builder, 1);
                let val_ptr_i8 = self.build_tos_ptr(builder, 2);

                builder.build_call(self.sstore(), &[self.storage_ptr().into(), key_ptr_i8.into(), val_ptr_i8.into()], "sstore");
                self.build_decr(builder, sp, 2);
            }
            Instruction::Sha3 => {
                let name = "sha3";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let length = self.build_peek(builder, sp, 2, "length");
                let offset = self.build_peek(builder, sp, 1, "offset");
                let sp = self.build_decr(builder, sp, 2);

                let length = builder.build_int_truncate_or_bit_cast(length, self.context.i16_type(), "length");

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.context.i64_type().const_zero(), offset], "mem") };
                let addr = builder.build_pointer_cast(addr, self.context.i8_type().ptr_type(AddressSpace::Generic), "addr");

                let stack = self.stack.unwrap().as_pointer_value();
                let tos = unsafe { builder.build_in_bounds_gep(stack, &[self.context.i64_type().const_zero(), sp], "stack") };
                let tos = builder.build_pointer_cast(tos, self.context.i8_type().ptr_type(AddressSpace::Generic), "tos");

                let _func = builder.build_call(
                    self.sha3(),
                    &[
                        addr.into(),
                        length.into(),
                        tos.into(),
                    ],
                    "hash");
                self.build_incr(builder, sp, 1);
            }
            Instruction::Byte => {
                let name = "byte";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let x = self.build_peek(builder, sp, 2, "x");
                let i = self.build_peek(builder, sp, 1, "x");
                let sp = self.build_decr(builder, sp, 2);
                // y = (x >> (248 - i * 8)) & 0xFF
                let i = builder.build_left_shift(i, self.i256(3), "i");
                let sub = builder.build_int_sub(self.i256(248), i, "sub");
                let rr = builder.build_right_shift(x, sub, false, "rr");
                let value = builder.build_and(rr, self.i256(0xFF), "ret").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Log(_) => {
                error!("Event emission is unimpl: {:?}", instr);
            }
            Instruction::Stop => {
                let name = "stop";
                self.push_label(name, builder);
                builder.build_return(None);
            }
            Instruction::SelfDestruct => {
                error!("{:#?}", instr);
                return None;
            }
            Instruction::CallDataLoad => {
                let name = "calldataload";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (idx, sp) = self.build_pop(builder, sp);
                let calldata = self.fun.unwrap().get_nth_param(0).unwrap().into_pointer_value();
                let ptr = unsafe { builder.build_gep(calldata, &[idx], name)};
                let ptr = builder.build_pointer_cast(ptr, self.i256_ty.ptr_type(AddressSpace::Generic), "ptr");
                let value = builder.build_load(ptr, "value");
                self.build_push(builder, value, sp);
                let ptr_i8 = self.build_tos_ptr(builder, 1);
                builder.build_call(self.swap_endianness(), &[ptr_i8.into()], "swap_endian");
            }
            Instruction::CallDataSize => {
                let name = "calldatasize";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let calldatasize = self.fun.unwrap().get_nth_param(1).unwrap().into_int_value();
                let calldatasize = builder.build_int_z_extend(calldatasize, self.i256_ty, "calldatasize").into();
                self.build_push(builder, calldatasize, sp);
            }
            Instruction::Invalid => {
                let name = "invalid";
                self.push_label(name, builder);
                builder.build_unconditional_branch(self.errbb.unwrap());
                self.pop_label();

                warn!("Invalid instruction encountered. Continuing compilation!");
                return Some(());
            }
            Instruction::Return => {
                let name = "return";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let length = self.build_peek(builder, sp, 2, "length");
                let offset = self.build_peek(builder, sp, 1, "offset");
                let _sp = self.build_decr(builder, sp, 2);

                let length = builder.build_int_truncate_or_bit_cast(length, self.context.i64_type(), "length");
                let offset = builder.build_int_truncate_or_bit_cast(offset, self.context.i64_type(), "offset");

                let offset_ptr = self.fun.unwrap().get_nth_param(2).unwrap().into_pointer_value();
                let len_ptr = self.fun.unwrap().get_nth_param(3).unwrap().into_pointer_value();
                builder.build_store(offset_ptr, offset);
                builder.build_store(len_ptr, length);

                builder.build_return(None);
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
                        self.code(builder, is_runtime),
                        &[offset],
                        "src") };
                let dest = unsafe {
                    builder.build_in_bounds_gep(
                        self.mem.unwrap().as_pointer_value(),
                        &[self.i256(0), dest_offset],
                        "dest") };

                // memory[destOffset:destOffset+length] = code[offset:offset+length];
                // let length = builder.build_int_z_extend_or_bit_cast(length, self.context.i64_type(), "length");
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
            Instruction::Jump => {
                let name = "jump";
                self.push_label(name, builder);
                // noop, jumpbb pops off the new pc
                builder.build_unconditional_branch(self.jumpbb.unwrap());
            }
            Instruction::JumpIf => {
                let name = "jumpi";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                // we pop everything and then push jump addr on to the stack
                let (new_pc, sp) = self.build_pop(builder, sp);
                let (cond, sp) = self.build_pop(builder, sp);
                let sp = self.build_push(builder, new_pc.into(), sp);
                let cond = builder.build_int_compare(IntPredicate::EQ, cond, self.i256(1), "cond");

                let else_block = self.context.insert_basic_block_after(builder.get_insert_block().unwrap(), "else");
                builder.build_conditional_branch(cond, self.jumpbb.unwrap(), else_block);
                builder.position_at_end(else_block);
                self.build_decr(builder, sp, 1); // if else branch, sp didn't get decr at jumpbb
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
                    self.i64(1),
                    "sp");

                let sp_r = builder.build_int_sub(
                    sp,
                    self.i64(*n as u64 + 1),
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
                let name = "callvalue";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let value = self.i256(0).into();
                self.build_push(builder, value, sp);
            }
            Instruction::MLoad => {
                let name = "mload";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let offset = self.build_peek(builder, sp, 1, "offset");
                let sp = self.build_decr(builder, sp, 1);

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.i64(0), offset], "off") };
                let addr = builder.build_pointer_cast(addr, self.i256_ty.ptr_type(AddressSpace::Generic), "addr");
                let value = builder.build_load(addr, "value");
                self.build_push(builder, value, sp);
            }
            Instruction::MStore8 => {
                let name = "mstore8";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let offset = self.build_peek(builder, sp, 1, "offset");
                let value = self.build_peek(builder, sp, 2, "value");
                let value = builder.build_int_truncate(value, self.context.i8_type(), "trunc");
                let _sp = self.build_decr(builder, sp, 2);
                let offset = builder.build_int_truncate_or_bit_cast(offset, self.context.i64_type(), "idx");

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.context.i64_type().const_zero(), offset], "stack") };
                builder.build_store(addr, value);
            }
            Instruction::MStore => {
                let name = "mstore";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let offset = self.build_peek(builder, sp, 1, "offset");
                let value = self.build_peek(builder, sp, 2, "value");
                let _sp = self.build_decr(builder, sp, 2);

                let mem = self.mem.unwrap().as_pointer_value();
                let addr = unsafe { builder.build_in_bounds_gep(mem, &[self.i64(0), offset], "off") };
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
            Instruction::AddMod => {
                self.build_instr(offset, &Instruction::Add, builder, is_runtime)?;
                self.build_instr(offset, &Instruction::Mod, builder, is_runtime)?;
            }
            Instruction::MulMod => {
                self.build_instr(offset, &Instruction::Mul, builder, is_runtime)?;
                self.build_instr(offset, &Instruction::Mod, builder, is_runtime)?;
            }
            Instruction::Exp => {
                let name = "exp";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let base_ptr = self.build_tos_ptr(builder, 1);
                let exp_ptr = self.build_tos_ptr(builder, 2);
                let ret_ptr = self.build_tos_ptr(builder, 0);
                builder.build_call(self.powmod(), &[base_ptr.into(), exp_ptr.into(), ret_ptr.into()], "powmod");

                let ret = builder.build_load(ret_ptr, "ret");
                builder.build_store(exp_ptr, ret);
                self.build_decr(builder, sp, 1);
            }
            Instruction::SDiv => {
                let name = "sdiv";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let d_ptr = self.build_tos_ptr(builder, 2);
                let n_ptr = self.build_tos_ptr(builder, 1);
                let q_ptr = self.build_tos_ptr(builder, 0);
                builder.build_call(self.sdiv256(), &[n_ptr.into(), d_ptr.into(), q_ptr.into()], "sdiv");

                let ret = builder.build_load(q_ptr, "ret");
                builder.build_store(d_ptr, ret);
                self.build_decr(builder, sp, 1);
            }
            Instruction::Div => {
                let name = "div";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let d_ptr = self.build_tos_ptr(builder, 2);
                let n_ptr = self.build_tos_ptr(builder, 1);
                let q_ptr = self.build_tos_ptr(builder, 0);
                builder.build_call(self.udiv256(), &[n_ptr.into(), d_ptr.into(), q_ptr.into()], "div");
                let sp = self.build_incr(builder, sp, 1);
                let (value, sp) = self.build_pop(builder, sp);
                let (_, sp) = self.build_pop(builder, sp);
                let (_, sp) = self.build_pop(builder, sp);
                self.build_push(builder, value.into(), sp);
            }
            Instruction::Mod => {
                let name = "mod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let d_ptr = self.build_tos_ptr(builder, 2);
                let n_ptr = self.build_tos_ptr(builder, 1);
                let q_ptr = self.build_tos_ptr(builder, 0);
                builder.build_call(self.udiv256(), &[n_ptr.into(), d_ptr.into(), q_ptr.into()], "mod");

                let ret = builder.build_load(n_ptr, "ret");
                builder.build_store(d_ptr, ret);
                self.build_decr(builder, sp, 1);
            }
            Instruction::SMod => {
                let name = "smod";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let d_ptr = self.build_tos_ptr(builder, 2);
                let n_ptr = self.build_tos_ptr(builder, 1);
                let q_ptr = self.build_tos_ptr(builder, 0);
                builder.build_call(self.sdiv256(), &[n_ptr.into(), d_ptr.into(), q_ptr.into()], "smod");

                let ret = builder.build_load(n_ptr, "ret");
                builder.build_store(d_ptr, ret);
                self.build_decr(builder, sp, 1);
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
                let value = builder.build_int_compare(IntPredicate::ULT, lhs, rhs, "lt");
                let value = builder.build_int_z_extend(value, self.i256_ty, "value").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Gt => {
                let name = "gt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::UGT, lhs, rhs, "lt");
                let value = builder.build_int_z_extend(value, self.i256_ty, "value").into();
                self.build_push(builder, value, sp);
            }
            Instruction::SLt => {
                let name = "slt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::SLT, lhs, rhs, "lt");
                let value = builder.build_int_z_extend(value, self.i256_ty, "value").into();
                self.build_push(builder, value, sp);
            }
            Instruction::SGt => {
                let name = "sgt";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::SGT, lhs, rhs, "lt");
                let value = builder.build_int_z_extend(value, self.i256_ty, "value").into();
                self.build_push(builder, value, sp);
            }
            Instruction::EQ => {
                let name = "eq";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let lhs = self.build_peek(builder, sp, 1, "a");
                let rhs = self.build_peek(builder, sp, 2, "b");
                let sp = self.build_decr(builder, sp, 2);
                let value = builder.build_int_compare(IntPredicate::EQ, lhs, rhs, "lt");
                let value = builder.build_int_z_extend_or_bit_cast(value, self.i256_ty, "eq").into();
                self.build_push(builder, value, sp);
            }
            Instruction::Pop => {
                let name = "pop";
                self.push_label(name, builder);
                let sp = self.build_sp(builder);
                let (_ret, _sp) = self.build_pop(builder, sp);
            }
            Instruction::Push(vals) => {
                assert!(vals.len() <= 32);
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
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_nibbles2i256() {
        let nibbles = vec![0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41];
        let ret = nibble2i256(&nibbles);
        assert_eq!(vec![
            0x4141414141414141,
            0x4141414141414141,
            0x4141414141414141,
            0x4141414141414141,
        ], ret);
    }
}
