use serde_json;
use serde::{Deserialize, Serialize};
use crate::evm::{Disassembly, Instruction};
use std::collections::HashMap;
use std::process::Command;
use std::path::PathBuf;
use uint::rustc_hex::FromHex;

#[derive(Serialize, Deserialize, Debug)]
pub struct Contract {
    pub abi: String,
    pub bin: String,
    #[serde(rename="bin-runtime")]
    pub bin_runtime: String,
}

impl Contract {
    pub fn parse(&self) -> (Vec<u8>, Vec<u8>, Vec<(usize, Instruction)>, Vec<(usize, Instruction)>) {
        let ctor_bytes: Vec<u8> = (self.bin).from_hex().expect("Invalid Hex String");
        let ctor_opcodes =  Disassembly::from_bytes(&ctor_bytes).unwrap().instructions;

        let rt_bytes: Vec<u8> = (self.bin_runtime).from_hex().expect("Invalid Hex String");
        let rt_opcodes =  Disassembly::from_bytes(&rt_bytes).unwrap().instructions;
        (ctor_bytes, rt_bytes, ctor_opcodes, rt_opcodes)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Contracts {
    contracts: HashMap<String, Contract>,
}

pub fn solc_compile(path: &PathBuf) -> HashMap<String, Contract> {
    let cmd = Command::new("solc")
            .arg(path)
            .arg("--combined-json")
            .arg("bin,bin-runtime,abi")
            .arg("--allow-paths=/")
            .output()
            .expect("solc command failed to start");
    let json = String::from_utf8_lossy(&cmd.stdout);

    let contracts = serde_json::from_str::<Contracts>(&json).unwrap().contracts;
    contracts
}