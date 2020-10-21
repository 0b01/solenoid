#! /usr/bin/bash

mkdir bin
/mnt/c/Users/ricky/Desktop/llvm/build/bin/llc ./test.ll -filetype=obj -relocation-model=pic -O3 -o bin/contracts.o
/mnt/c/Users/ricky/Desktop/llvm/build/bin/llc ./runtime/arith.ll -filetype=obj -relocation-model=pic -O3 -o bin/arith.o
clang ./runtime/rt.c -fPIC -O3 -c -o bin/rt.o
clang ./tests/main.c bin/contracts.o  bin/arith.o  bin/rt.o -o bin/contracts.exe