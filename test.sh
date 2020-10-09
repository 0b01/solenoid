#! /usr/bin/bash

cargo test
llc out.ll -march=bpf -o out.bpf.s -O3
llc out.ll -filetype=obj -o out.o -relocation-model=pic -O3
gcc rt.c out.o -fPIC -o a.out
./a.out