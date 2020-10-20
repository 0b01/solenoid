use crate::compiler::Compiler;
use crate::ethabi::{Function, Contract, param_type::ParamType::*};
use std::fs;
use std::string::String;

/// Generate C header file containing the functions in compiled contracts
pub struct CFFIGenerator {
    fn_names: Vec<String>,
    fn_stubs: Vec<String>,
}

impl CFFIGenerator {
    pub fn new() -> Self {
        let fn_stubs = Vec::new();
        let fn_names = Vec::new();
        Self {
            fn_stubs,
            fn_names,
        }
    }

    pub fn add_stub(&mut self, fn_name: &str, params: &[String]) {
        let fn_stub = format!("extern void {}({});", fn_name, params.join(", "));
        self.fn_stubs.push(fn_stub.to_string());
        self.fn_names.push(fn_name.to_owned());
    }

    pub fn add(&mut self, name: &str, contract: Contract) {
        // abi formatters
        for (_name, funs) in &contract.functions {
            for (idx, fun) in funs.iter().enumerate() {
                self.add_abi_function(fun, idx);
            }
        }

        // constructor
        let fn_name = Compiler::format_fn_name(name, false);
        let params = vec![
            "i8* msg".to_owned(),
            "long msg_len".to_owned(),
            "long* ret_offset".to_owned(),
            "long* ret_len".to_owned(),
            "i8* storage".to_owned()
        ];
        self.add_stub(&fn_name, &params);


        // runtime, same params
        let fn_name = Compiler::format_fn_name(name, true);
        self.add_stub(&fn_name, &params);
    }

    pub fn add_abi_function(&mut self, fun: &Function, idx: usize) {
        let fn_name = Compiler::format_abi_fn_name(fun, idx);
        let mut params = Vec::new();
        params.push("i8* tx".to_owned());
        params.push("int* tx_size".to_owned());
        for param in &fun.inputs {
            let ty = match param.kind {
                Address => "i8*",
                Bytes => "i8*",

                Int(8) => "byte",
                Int(16) => "short",
                Int(32) => "int",
                Int(64) => "long",
                Int(_) => "i8*",

                Uint(8) => "byte",
                Uint(16) => "short",
                Uint(32) => "int",
                Uint(64) => "long",
                Uint(_) => "i8*",

                Bool => "int",
                String => "i8*",
                Array(_) => "i8*",
                FixedBytes(_) => "i8*",
                FixedArray(_, _) => "i8*",
                Tuple(_) => "i8*",
            };
            params.push(format!("{} {}", ty, param.name));
        }

        self.add_stub(&fn_name, &params);
    }

    pub fn generate(&self, out_path: &str) {
        std::fs::create_dir_all(out_path).expect("unable to create output directory");
        let mut contents = String::new();
        contents += "/* This header file is automatically generated by solana-labs/solenoid. Do not modify it by hand */\n";
        contents += "\n";
        contents += "#include \"rt.h\"\n";
        contents += "\n";
        contents += &self.fn_stubs.join("\n\n");
        contents += "\n";
        let header_path = format!("{}/contracts.h", out_path);
        let bindings_path = format!("{}/bindings.rs", out_path);

        fs::write(&header_path, contents).expect("unable to write contracts.h header");
        Self::write_deps(&out_path);

        let mut builder = bindgen::builder().header(&header_path);
        for f in &self.fn_names {
            builder = builder.whitelist_function(f);
        }

        let bindings = builder
            .generate()
            .expect("unable to generate bindings");
        bindings.write_to_file(bindings_path).expect("unable to write bindings");
    }

    pub fn write_deps(out_path: &str) {
        macro_rules! include {
            ($($x:expr,)*) => {
                $(
                    fs::write(&format!("{}/{}", out_path, $x), include_str!(concat!("../runtime/", $x))).unwrap();
                )*
            }
        }
        include!( "rt.c", "rt.h", "sha3.h", "sha3.c", "utils.c", "utils.h", );
    }
}