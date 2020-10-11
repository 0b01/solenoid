use libsolenoid::evm_opcode::{Disassembly, assemble_instructions};
use libsolenoid::compiler::Compiler;
use hex::FromHex;

fn main() {
    let code = "608060405234801561001057600080fd5b5060c78061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c806360fe47b11460375780636d4ce63c146062575b600080fd5b606060048036036020811015604b57600080fd5b8101908080359060200190929190505050607e565b005b60686088565b6040518082815260200191505060405180910390f35b8060008190555050565b6000805490509056fea2646970667358221220a9fae844c36e17167b8eb3c2a937fae45ccababd9dedf0238ef8597e021ba56964736f6c63430006060033";
    let bytes: Vec<u8> = Vec::from_hex(code).expect("Invalid Hex String");
    let opcodes =  Disassembly::from_bytes(&bytes).unwrap().instructions;

    let instrs: Vec<_> = opcodes.iter().map(|(_,i)|i.clone()).collect();
    let bytes = &assemble_instructions(instrs)[..100];
    let opcodes =  Disassembly::from_bytes(&bytes).unwrap().instructions;
    Compiler::codegen(&opcodes, &bytes);
}