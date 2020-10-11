#! /usr/bin/bash

cargo run
llc out.ll -march=bpf -o out.bpf.s -O3
llc out.ll -o out.x64.s -O3
llc out.ll -filetype=obj -o out.o -relocation-model=pic -O3
gcc rt.c out.o -fPIC -o a.out
./a.out