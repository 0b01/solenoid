export LLVM_DIR=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_100_PREFIX=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_100_STRICT_VERSIONING=true
export RUST_LOG=bindgen::*=error,libsolenoid=debug

OUTDIR := example_contract
CONTRACT := tests/contracts/set.sol

run:
	cargo run -- --input $(CONTRACT) -o $(OUTDIR)

debug:
	cargo run -- --debug --input $(CONTRACT) -o $(OUTDIR)

test:
	RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test -- --nocapture

clean:
	rm -R *.o a.out *.ll *.s $(OUTDIR) test_*
