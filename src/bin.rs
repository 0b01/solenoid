use libsolenoid::evm_opcode::{Disassembly, assemble_instructions};
use libsolenoid::compiler::Compiler;
use std::process::Command;
use hex::FromHex;
use structopt::StructOpt;
use std::path::PathBuf;
use serde_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::{info, warn};

#[derive(Debug, StructOpt)]
#[structopt(name = "solenoid", about = "solenoid compiler toolchain")]
struct Opt {
    /// debug
    #[structopt(short, long)]
    debug: bool,

    /// Input contract
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct Contract {
    abi: String,
    bin: String,
    #[serde(rename="bin-runtime")]
    bin_runtime: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Contracts {
    contracts: HashMap<String, Contract>,
}


fn main() {
    env_logger::init();


    let opt = Opt::from_args();

    let cmd = Command::new("solc")
            .arg(opt.input)
            .arg("--combined-json")
            .arg("bin,bin-runtime,abi")
            .output()
            .expect("solc command failed to start");
    let json = String::from_utf8_lossy(&cmd.stdout);

    let contracts: Contracts = serde_json::from_str(&json).unwrap();

    for (name, contract) in &contracts.contracts {
        info!("Compiling {} constructor", name);
        let code = &contract.bin;
        let bytes: Vec<u8> = Vec::from_hex(code).expect("Invalid Hex String");
        let opcodes =  Disassembly::from_bytes(&bytes).unwrap().instructions;
        Compiler::codegen(&opcodes, &bytes);

        info!("Compiling {} runtime", name);
        let code = &contract.bin_runtime;
        let bytes: Vec<u8> = Vec::from_hex(code).expect("Invalid Hex String");
        let opcodes =  Disassembly::from_bytes(&bytes).unwrap().instructions;
        Compiler::codegen(&opcodes, &bytes);
    }

}