#! /bin/bash

# RUST_LOG=debug cargo run tests/contracts/set.sol
cargo test
llc out.ll -march=bpf -o out.bpf.s -O3
llc out.ll -o out.x64.s -O3
llc out.ll -filetype=obj -o out.o -relocation-model=pic -O3
llc runtime/upow.ll -filetype=obj -o upow.o -relocation-model=pic -O3
clang runtime/rt.c runtime/sha3.c out.o upow.o -fPIC -o a.out
./a.out
