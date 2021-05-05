use inkwell::context::Context;
use libsolenoid::evm::{self, Instruction};
use libsolenoid::compiler::Compiler;

fn build_instrs() -> Vec<Instruction> {
    let n = 220;
    let mut ret = vec![];
    ret.push(Instruction::Push(vec![0])); // C.push("0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001");  // q
    ret.push(Instruction::Push(vec![0x44])); // C.push("0x44");
    
    ret.push(Instruction::MLoad);// C.mload();          // k q
    ret.push(Instruction::Push(vec![0x04]));// C.push("0x04");
    ret.push(Instruction::MLoad);// C.mload();          // xL k q
    ret.push(Instruction::Dup(2));// C.dup(2);           // q xL k q
    ret.push(Instruction::Push(vec![0x24]));// C.push("0x24");
    ret.push(Instruction::MLoad);// C.mload();          // xR q xL k q
    ret.push(Instruction::Dup(1));// C.dup(1);           // q xR q xL k q
    ret.push(Instruction::Dup(0));// C.dup(0);           // q q xR q xL k q
    ret.push(Instruction::Dup(4));// C.dup(4);           // xL q q xR q xL k q
    ret.push(Instruction::Dup(6));// C.dup(6);           // k xL q q xR q xL k q
    ret.push(Instruction::AddMod);// C.addmod();         // t=k+xL q xR q xL k q
    ret.push(Instruction::Dup(1));// C.dup(1);           // q t q xR q xL k q
    ret.push(Instruction::Dup(0));// C.dup(0);           // q q t q xR q xL k q
    ret.push(Instruction::Dup(2));// C.dup(2);           // t q q t q xR q xL k q
    ret.push(Instruction::Dup(0));// C.dup(0);           // t t q q t q xR q xL k q
    ret.push(Instruction::MulMod);// C.mulmod();         // b=t^2 q t q xR q xL k q
    ret.push(Instruction::Dup(0));// C.dup(0);           // b b q t q xR q xL k q
    ret.push(Instruction::MulMod);// C.mulmod();         // c=t^4 t q xR q xL k q
    ret.push(Instruction::MulMod);// C.mulmod();         // d=t^5 xR q xL k q
    ret.push(Instruction::AddMod);// C.addmod();         // e=t^5+xR xL k q (for next round: xL xR k q)

    for i in 0..(n-1) {
        //     if (i < n-2) {
        //       ci = Web3Utils.keccak256(ci);
        //     } else {
        //       ci = "0x00";
        //     }
        ret.push(Instruction::Swap(1));//     C.swap(1);      // xR xL k q
        ret.push(Instruction::Dup(3));//     C.dup(3);       // q xR xL k q
        ret.push(Instruction::Dup(3));//     C.dup(3);       // k q xR xL k q
        ret.push(Instruction::Dup(1));//     C.dup(1);       // q k q xR xL k q
        ret.push(Instruction::Dup(4));//     C.dup(4);       // xL q k q xR xL k q
        ret.push(Instruction::Push(vec![0]));//     C.push(ci);     // ci xL q k q xR xL k q
        ret.push(Instruction::AddMod);//     C.addmod();     // a=ci+xL k q xR xL k q
        ret.push(Instruction::AddMod);//     C.addmod();     // t=a+k xR xL k q
        ret.push(Instruction::Dup(4));//     C.dup(4);       // q t xR xL k q
        ret.push(Instruction::Swap(1));//     C.swap(1);      // t q xR xL k q
        ret.push(Instruction::Dup(1));//     C.dup(1);       // q t q xR xL k q
        ret.push(Instruction::Dup(0));//     C.dup(0);       // q q t q xR xL k q
        ret.push(Instruction::Dup(2));//     C.dup(2);       // t q q t q xR xL k q
        ret.push(Instruction::Dup(0));//     C.dup(0);       // t t q q t q xR xL k q
        ret.push(Instruction::MulMod);//     C.mulmod();     // b=t^2 q t q xR xL k q
        ret.push(Instruction::Dup(0));//     C.dup(0);       // b b q t q xR xL k q
        ret.push(Instruction::MulMod);//     C.mulmod();     // c=t^4 t q xR xL k q
        ret.push(Instruction::MulMod);//     C.mulmod();     // d=t^5 xR xL k q
        ret.push(Instruction::Dup(4));//     C.dup(4);       // q d xR xL k q
        ret.push(Instruction::Swap(2));//     C.swap(2);      // xR d q xL k q
        ret.push(Instruction::AddMod);//     C.addmod();     // e=t^5+xR xL k q (for next round: xL xR k q)
    }

    ret
}

fn main() {
    let context = Context::create();
    let module = context.create_module("contract");
    let builder = context.create_builder();

    let instrs = build_instrs();
    let bytes = evm::assemble_instructions(&instrs);
    let instrs = evm::Disassembly::from_bytes(&bytes).unwrap().instructions;

    let mut compiler = Compiler::new(&context, &module, false);
    compiler.compile(&builder, &instrs, &bytes, "test", false);
    // compiler.dbg();
    module.print_to_file("out.ll").unwrap();
}
