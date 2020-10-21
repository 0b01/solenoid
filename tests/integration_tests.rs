use std::process::Command;
use primitive_types::U256;
use uint::rustc_hex::FromHex;

use inkwell::context::Context;
use libsolenoid::evm::{self, Instruction};
use libsolenoid::compiler::Compiler;

#[cfg(test)]
#[track_caller]
fn assert_stack(actual_stack: &str, expected_stack:&[Vec<u8>]) {
    let actual = actual_stack.split("\n");
    let actual = actual_stack.split("\n").take(actual.count() - 1)
        .map(|i|i.from_hex().unwrap())
        .map(|i: Vec<u8> | U256::from_little_endian(&i)).collect::<Vec<_>>();
    let expected = expected_stack.iter().map(|i| U256::from_big_endian(i)).collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[cfg(test)]
fn compile_and_run(instrs: &[Instruction]) -> String {
    let context = Context::create();
    let module = context.create_module("contract");
    let builder = context.create_builder();

    let bytes = evm::assemble_instructions(instrs);
    let instrs = evm::Disassembly::from_bytes(&bytes).unwrap().instructions;
    dbg!(&instrs);

    let mut compiler = Compiler::new(&context, &module, false);
    compiler.compile(&builder, &instrs, &bytes, "test", false);
    // compiler.dbg();
    module.print_to_file("test.ll").unwrap();

    Command::new("./tests/build.sh").output().unwrap();
    let output = Command::new("./bin/contracts.exe").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.to_string()
}

#[test]
fn test_comparisons() {
    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::Lt,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![20]),
        Instruction::Push(vec![30]),
        Instruction::Lt,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![20]),
        Instruction::Push(vec![30]),
        Instruction::Gt,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::Gt,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::SLt,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![20]),
        Instruction::Push(vec![30]),
        Instruction::SLt,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![20]),
        Instruction::Push(vec![30]),
        Instruction::SGt,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::SGt,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::EQ,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![30]),
        Instruction::EQ,
    ]), &[vec![1]]);
}

#[test]
fn test_stack() {
    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::Push(vec![10]),
        Instruction::Swap(2),
    ]), &[vec![10], vec![20], vec![30]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
        Instruction::Swap(1),
    ]), &[vec![20], vec![30]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![20]),
    ]), &[vec![30], vec![20]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![10]),
        Instruction::Pop,
    ]), &[vec![30]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![10]),
        Instruction::Dup(1),
    ]), &[vec![30], vec![10], vec![30]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![10]),
        Instruction::Dup(0),
    ]), &[vec![30], vec![10], vec![10]]);
}

#[test]
fn test_mem() {
    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0xAA, 0xBB, 0xCC, 0xDD]),
        Instruction::Push(vec![31]),
        Instruction::Byte,
        Instruction::Push(vec![0xAA, 0xBB, 0xCC, 0xDD]),
        Instruction::Push(vec![30]),
        Instruction::Byte,
        Instruction::Push(vec![0xAA, 0xBB, 0xCC, 0xDD]),
        Instruction::Push(vec![29]),
        Instruction::Byte,
        Instruction::Push(vec![0xAA, 0xBB, 0xCC, 0xDD]),
        Instruction::Push(vec![28]),
        Instruction::Byte,
    ]), &[vec![0xDD], vec![0xCC], vec![0xBB], vec![0xAA]]);
}

#[test]
fn test_arith() {
    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0]),
        Instruction::IsZero,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![10]),
        Instruction::IsZero,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![7]),
        Instruction::Push(vec![3]),
        Instruction::Push(vec![10]),
        Instruction::MulMod,
    ]), &[vec![2]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![5]),
        Instruction::Push(vec![3]),
        Instruction::Push(vec![10]),
        Instruction::AddMod,
    ]), &[vec![3]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![3]),
        Instruction::Push(vec![2]),
        Instruction::Exp,
    ]), &[vec![8]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![3]),
        Instruction::Push(vec![3]),
        Instruction::Exp,
    ]), &[vec![27]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![3]),
        Instruction::Push(vec![10]),
        Instruction::SMod,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![3]),
        Instruction::Push(vec![10]),
        Instruction::Mod,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0x0f, 0xf0]),
        Instruction::Push(vec![0x0f, 0xf0]),
        Instruction::And,
    ]), &[vec![0x0f, 0xf0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0xff, 0x00]),
        Instruction::Push(vec![0x00, 0xff]),
        Instruction::And,
    ]), &[vec![0x00, 0x00]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0xff, 0x00]),
        Instruction::Push(vec![0x00, 0xff]),
        Instruction::Or,
    ]), &[vec![0xff, 0xff]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![1]),
        Instruction::Push(vec![1]),
        Instruction::Shl,
    ]), &[vec![2]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![2]),
        Instruction::Push(vec![2]),
        Instruction::Shl,
    ]), &[vec![8]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![8]),
        Instruction::Push(vec![2]),
        Instruction::Shr,
    ]), &[vec![2]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![8]),
        Instruction::Push(vec![1]),
        Instruction::Shr,
    ]), &[vec![4]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![2]),
        Instruction::Push(vec![1]),
        Instruction::Shr,
    ]), &[vec![1]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![1]),
        Instruction::Push(vec![1]),
        Instruction::Shr,
    ]), &[vec![0]]);

    let mut out = vec![0xff;31]; out.push(0xfe);
    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![1]),
        Instruction::Not,
    ]), &[out]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![10]),
        Instruction::Push(vec![30]),
        Instruction::Sub,
    ]), &[vec![20]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![30]),
        Instruction::Push(vec![10]),
        Instruction::Add,
    ]), &[vec![40]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![10]),
        Instruction::Push(vec![30]),
        Instruction::Div,
    ]), &[vec![3]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![10]),
        Instruction::Push(vec![30]),
        Instruction::SDiv,
    ]), &[vec![3]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![100]),
        Instruction::Push(vec![2]),
        Instruction::Mul,
        Instruction::Push(vec![55]),
        Instruction::Add,
    ]), &[vec![0xff]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![0]),
        Instruction::Push(vec![10]),
        Instruction::Mul,
    ]), &[vec![0]]);

    assert_stack(&compile_and_run(&[
        Instruction::Push(vec![2]),
        Instruction::Push(vec![10]),
        Instruction::Mul,
    ]), &[vec![20]]);
}