use inkwell::context::Context;
use libsolenoid::compiler::Compiler;
use libsolenoid::solc;
use libsolenoid::cffigen::CFFIGenerator;
use structopt::StructOpt;
use std::path::PathBuf;
use uint::rustc_hex::FromHex;
use libsolenoid::evm::Disassembly;
use log::{info, debug};

#[derive(Debug, StructOpt)]
#[structopt(name = "solenoid", about = "solenoid compiler toolchain")]
struct Opt {
    /// print opcodes then exit
    #[structopt(short, long)]
    print_opcodes: bool,

    /// debug
    #[structopt(short, long)]
    debug: bool,

    /// Input contract
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    /// Output LLVM IR file 
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,

    /// Compile raw hex-encoded opcodes
    opcodes: Option<String>,
}

fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    let ll_output = opt.output.unwrap_or(PathBuf::from("out.ll"));
    let context = Context::create();
    let module = context.create_module("contracts");
    let builder = context.create_builder();
    let mut cffigen = CFFIGenerator::new();

    if let Some(hex_opcodes) = opt.opcodes {
        let bytes: Vec<u8> = hex_opcodes.from_hex().expect("Invalid Hex String");
        let instrs =  Disassembly::from_bytes(&bytes).unwrap().instructions;

        let mut compiler = Compiler::new(&context, &module, false);
        compiler.compile(&builder, &instrs, &bytes, "test", false);
        // compiler.dbg();
        module.print_to_file(ll_output).unwrap();
    } else if let Some(input) = &opt.input {
        let contracts = solc::solc_compile(input);
        for (name, contract) in &contracts {
            let name = name.split(":").last().unwrap();
            let mut compiler = Compiler::new(&context, &module, opt.debug);
            let (ctor_bytes, rt_bytes, ctor_opcodes, rt_opcodes) = contract.parse();

            debug!("Constructor instrs: {:#?}", ctor_opcodes);
            debug!("Runtime instrs: {:#?}", rt_opcodes);

            if opt.print_opcodes {
                continue;
            }

            info!("Compiling {} constructor", name);
            compiler.compile(&builder, &ctor_opcodes, &ctor_bytes, name, false);

            info!("Compiling {} runtime", name);
            compiler.compile(&builder, &rt_opcodes, &rt_bytes, name, true);

            let contract = libsolenoid::ethabi::Contract::load(contract.abi.as_bytes()).unwrap();
            compiler.compile_abi(&builder, &contract);

            cffigen.add_contract(name, contract);
        }

        if opt.print_opcodes {
            return;
        }

        module.print_to_file(ll_output).unwrap();
        cffigen.generate("out/");
    }
}
