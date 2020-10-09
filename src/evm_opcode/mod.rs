// Copyright 2019 Joel Frank
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod error;
pub mod instructions;

use hex::FromHex;
use std::io::Cursor;

use instructions::{assemble_instruction, disassemble_next_byte};

pub use error::DisassemblyError;
pub use instructions::Instruction;

#[derive(Clone, Debug)]
pub struct Disassembly {
    pub instructions: Vec<Instruction>,
}

impl Disassembly {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DisassemblyError> {
        let instructions = disassemble_bytes(bytes)?;
        Ok(Self { instructions })
    }

    pub fn from_hex_str(input: &str) -> Result<Self, DisassemblyError> {
        let instructions = disassemble_hex_str(input)?;
        Ok(Self { instructions })
    }
}

pub fn assemble_instructions(disassembly: Vec<Instruction>) -> Vec<u8> {
    let mut result = Vec::new();
    for disas in disassembly {
        result.extend(assemble_instruction(disas));
    }
    result
}

fn disassemble_hex_str(input: &str) -> Result<Vec<Instruction>, DisassemblyError> {
    let input = if input[0..2] == *"0x" {
        &input[2..]
    } else {
        input
    };
    let bytes = Vec::from_hex(input)?;
    disassemble_bytes(&bytes)
}

fn disassemble_bytes(bytes: &[u8]) -> Result<Vec<Instruction>, DisassemblyError> {
    let mut instructions = Vec::new();
    let mut cursor = Cursor::new(bytes);
    loop {
        let result = disassemble_next_byte(&mut cursor);
        match result {
            Err(DisassemblyError::IOError(..)) => break,
            Ok((offset, instruction)) => {
                instructions.push(instruction);
            }
            Err(err) => {
                if let DisassemblyError::TooFewBytesForPush = err {
                    // the solidity compiler sometimes puts push instructions at the end, however,
                    // this is considered normal behaviour
                    break;
                }
                return Err(err);
            }
        }
    }

    Ok(instructions)
}