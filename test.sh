#! /bin/bash
export LLVM_DIR=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_80_PREFIX=/mnt/c/Users/ricky/Desktop/llvm
export LLVM_SYS_80_STRICT_VERSIONING=true
export RUST_LOG=debug

# FILE=/home/g/Desktop/chainlink/evm-contracts/src/v0.7/dev/Owned.sol
# FILE=/home/g/Desktop/chainlink/evm-contracts/src/v0.7/dev/Operator.sol
FILE=tests/contracts/set.sol
cargo run $FILE

# cargo run --example codegen

$LLVM_DIR/opt out.ll --O3 -S -o opt.ll
$LLVM_DIR/llc out.ll -march=bpf -o out.bpf.s -O3

$LLVM_DIR/llc out.ll -o out.x64.s -O3
$LLVM_DIR/llc runtime/arith.ll -filetype=obj -o arith.o -relocation-model=pic -O3
$LLVM_DIR/llc out.ll -filetype=obj -o out.o -relocation-model=pic -O3
clang runtime/utils.c runtime/sha3.c runtime/rt.c runtime/main.c arith.o out.o -fPIC -o a.out
./a.out
