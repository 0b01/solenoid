// Copyright 2015-2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::ethabi::errors::Error;
use crate::ethabi::token::Tokenizer;
use hex::FromHex;

/// Tries to parse string as a token. Require string to clearly represent the value.
pub struct StrictTokenizer;

impl Tokenizer for StrictTokenizer {
	fn tokenize_address(value: &str) -> Result<[u8; 20], Error> {
		let hex: Vec<u8> = value.from_hex()?;
		match hex.len() == 20 {
			false => Err(Error::InvalidData),
			true => {
				let mut address = [0u8; 20];
				address.copy_from_slice(&hex);
				Ok(address)
			}
		}
	}

	fn tokenize_string(value: &str) -> Result<String, Error> {
		Ok(value.to_owned())
	}

	fn tokenize_bool(value: &str) -> Result<bool, Error> {
		match value {
			"true" | "1" => Ok(true),
			"false" | "0" => Ok(false),
			_ => Err(Error::InvalidData),
		}
	}

	fn tokenize_bytes(value: &str) -> Result<Vec<u8>, Error> {
		value.from_hex().map_err(Into::into)
	}

	fn tokenize_fixed_bytes(value: &str, len: usize) -> Result<Vec<u8>, Error> {
		let hex: Vec<u8> = value.from_hex()?;
		match hex.len() == len {
			true => Ok(hex),
			false => Err(Error::InvalidData),
		}
	}

	fn tokenize_uint(value: &str) -> Result<[u8; 32], Error> {
		let hex: Vec<u8> = value.from_hex()?;
		match hex.len() == 32 {
			true => {
				let mut uint = [0u8; 32];
				uint.copy_from_slice(&hex);
				Ok(uint)
			}
			false => Err(Error::InvalidData),
		}
	}

	fn tokenize_int(value: &str) -> Result<[u8; 32], Error> {
		Self::tokenize_uint(value)
	}
}
