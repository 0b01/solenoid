use inkwell::context::Context;
use libsolenoid::evm::{Disassembly, Instruction};
use libsolenoid::compiler::Compiler;
use std::process::Command;
use uint::rustc_hex::FromHex;
use structopt::StructOpt;
use std::path::PathBuf;
use serde_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::{info, debug};

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

impl Contract {
    pub fn parse(&self, runtime: bool) -> (Vec<(usize, Instruction)>, Vec<u8>) {
        let code = if runtime {
            &self.bin_runtime
        } else {
            &self.bin
        };
        let bytes: Vec<u8> = (code).from_hex().expect("Invalid Hex String");
        let opcodes =  Disassembly::from_bytes(&bytes).unwrap().instructions;
        (opcodes, bytes)
    }
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

    let context = Context::create();
    let module = context.create_module("contracts");
    for (name, contract) in &contracts.contracts {
        let name = name.split(":").last().unwrap();
        let builder = context.create_builder();
        let mut compiler = Compiler::new(&context, &module);

        let (instrs, payload) = contract.parse(false);
        let (runtime_instrs, runtime_payload) = contract.parse(true);

        debug!("Constructor instrs: {:#?}", instrs);
        debug!("Runtime instrs: {:#?}", instrs);

        info!("Compiling {} constructor", name);
        compiler.compile(&builder, &instrs, &payload, name, false);

        info!("Compiling {} runtime", name);
        compiler.compile(&builder, &runtime_instrs, &runtime_payload, name, true);

        compiler.compile_abi(&builder, &contract.abi);
    }
    module.print_to_file("out.ll").unwrap();
}
