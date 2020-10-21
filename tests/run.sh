#! /bin/bash
export LLVM_DIR=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_80_PREFIX=/mnt/c/Users/ricky/Desktop/llvm
export LLVM_SYS_80_STRICT_VERSIONING=true
export RUST_LOG=warn

DIR=/mnt/c/Users/ricky/Desktop/solenoid/out/
OUTDIR=$DIR/bin/
mkdir $OUTDIR

$LLVM_DIR/llc $DIR/src/arith.ll -filetype=obj -o $OUTDIR/arith.o -relocation-model=pic -O3
$LLVM_DIR/llc $DIR/src/contracts.ll -filetype=obj -o $OUTDIR/contracts.o -relocation-model=pic -O3
clang $DIR/src/rt.c $DIR/src/main.c $OUTDIR/arith.o $OUTDIR/contracts.o -fPIC -o $OUTDIR/a.out
./a.out
