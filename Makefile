export LLVM_DIR=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_80_PREFIX=/mnt/c/Users/ricky/Desktop/llvm
export LLVM_SYS_80_STRICT_VERSIONING=true
export RUST_LOG=bindgen::*=error,libsolenoid=info

OUTDIR := example_contract
CONTRACT := tests/contracts/flipper.sol

run:
	cargo run -- --input $(CONTRACT) -o $(OUTDIR)

debug:
	cargo run -- --debug --input $(CONTRACT) -o $(OUTDIR)

test:
	RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test -- --nocapture

clean:
	rm *.o a.out *.ll *.s $(OUTDIR)