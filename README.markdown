# Solenoid compiler

Solenoid compiler uses LLVM to translate Ethereum contracts to Solana's BPF backend. It uses solc to compile contracts into EVM, gets the constructor and runtime payload and their associated ABI, and then compile into LLVM IR. It outputs a module containing the following functions:

1. contract constructor
2. contract runtime
2. abi conversion functions

## How to run

```
cargo install
./test.sh
```