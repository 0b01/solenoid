export LLVM_DIR=/mnt/c/Users/ricky/Desktop/llvm/build/bin
export LLVM_SYS_80_PREFIX=/mnt/c/Users/ricky/Desktop/llvm
export LLVM_SYS_80_STRICT_VERSIONING=true
export RUST_LOG=warn

run:
	cargo run -- --input tests/contracts/set.sol -o test_contract

test:
	RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test -- --nocapture

clean:
	rm *.o a.out *.ll *.s