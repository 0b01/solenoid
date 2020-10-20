// Copyright 2015-2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ABI decoder.

use crate::ethabi::util::slice_data;
use crate::ethabi::{Error, ParamType, Token, Word};

struct DecodeResult {
	token: Token,
	new_offset: usize,
}

struct BytesTaken {
	bytes: Vec<u8>,
	new_offset: usize,
}

fn as_u32(slice: &Word) -> Result<u32, Error> {
	if !slice[..28].iter().all(|x| *x == 0) {
		return Err(Error::InvalidData);
	}

	let result =
		((slice[28] as u32) << 24) + ((slice[29] as u32) << 16) + ((slice[30] as u32) << 8) + (slice[31] as u32);

	Ok(result)
}

fn as_bool(slice: &Word) -> Result<bool, Error> {
	if !slice[..31].iter().all(|x| *x == 0) {
		return Err(Error::InvalidData);
	}

	Ok(slice[31] == 1)
}

/// Decodes ABI compliant vector of bytes into vector of tokens described by types param.
pub fn decode(types: &[ParamType], data: &[u8]) -> Result<Vec<Token>, Error> {
	let is_empty_bytes_valid_encoding = types.iter().all(|t| t.is_empty_bytes_valid_encoding());
	if !is_empty_bytes_valid_encoding && data.is_empty() {
		return Err(Error::InvalidName(
			"please ensure the contract and method you're calling exist! \
			 failed to decode empty bytes. if you're using jsonrpc this is \
			 likely due to jsonrpc returning `0x` in case contract or method \
			 don't exist"
				.into(),
		));
	}
	let slices = slice_data(data)?;
	let mut tokens = Vec::with_capacity(types.len());
	let mut offset = 0;
	for param in types {
		let res = decode_param(param, &slices, offset)?;
		offset = res.new_offset;
		tokens.push(res.token);
	}
	Ok(tokens)
}

fn peek(slices: &[Word], position: usize) -> Result<&Word, Error> {
	slices.get(position).ok_or_else(|| Error::InvalidData)
}

fn take_bytes(slices: &[Word], position: usize, len: usize) -> Result<BytesTaken, Error> {
	let slices_len = (len + 31) / 32;

	let mut bytes_slices = Vec::with_capacity(slices_len);
	for i in 0..slices_len {
		let slice = peek(slices, position + i)?;
		bytes_slices.push(slice);
	}

	let bytes = bytes_slices.into_iter().flat_map(|slice| slice.to_vec()).take(len).collect();

	let taken = BytesTaken { bytes, new_offset: position + slices_len };

	Ok(taken)
}

fn decode_param(param: &ParamType, slices: &[Word], offset: usize) -> Result<DecodeResult, Error> {
	match *param {
		ParamType::Address => {
			let slice = peek(slices, offset)?;
			let mut address = [0u8; 20];
			address.copy_from_slice(&slice[12..]);

			let result = DecodeResult { token: Token::Address(address.into()), new_offset: offset + 1 };

			Ok(result)
		}
		ParamType::Int(_) => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult { token: Token::Int(slice.clone().into()), new_offset: offset + 1 };

			Ok(result)
		}
		ParamType::Uint(_) => {
			let slice = peek(slices, offset)?;

			let result = DecodeResult { token: Token::Uint(slice.clone().into()), new_offset: offset + 1 };

			Ok(result)
		}
		ParamType::Bool => {
			let slice = peek(slices, offset)?;

			let b = as_bool(slice)?;

			let result = DecodeResult { token: Token::Bool(b), new_offset: offset + 1 };
			Ok(result)
		}
		ParamType::FixedBytes(len) => {
			// FixedBytes is anything from bytes1 to bytes32. These values
			// are padded with trailing zeros to fill 32 bytes.
			let taken = take_bytes(slices, offset, len)?;
			let result = DecodeResult { token: Token::FixedBytes(taken.bytes), new_offset: taken.new_offset };
			Ok(result)
		}
		ParamType::Bytes => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;

			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let taken = take_bytes(slices, len_offset + 1, len)?;

			let result = DecodeResult { token: Token::Bytes(taken.bytes), new_offset: offset + 1 };
			Ok(result)
		}
		ParamType::String => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;

			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let taken = take_bytes(slices, len_offset + 1, len)?;

			let result = DecodeResult { token: Token::String(String::from_utf8(taken.bytes)?), new_offset: offset + 1 };
			Ok(result)
		}
		ParamType::Array(ref t) => {
			let offset_slice = peek(slices, offset)?;
			let len_offset = (as_u32(offset_slice)? / 32) as usize;
			let len_slice = peek(slices, len_offset)?;
			let len = as_u32(len_slice)? as usize;

			let tail = &slices[len_offset + 1..];
			let mut tokens = Vec::with_capacity(len);
			let mut new_offset = 0;

			for _ in 0..len {
				let res = decode_param(t, &tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			let result = DecodeResult { token: Token::Array(tokens), new_offset: offset + 1 };

			Ok(result)
		}
		ParamType::FixedArray(ref t, len) => {
			let mut tokens = Vec::with_capacity(len);
			let is_dynamic = param.is_dynamic();

			let (tail, mut new_offset) = if is_dynamic {
				(&slices[(as_u32(peek(slices, offset)?)? as usize / 32)..], 0)
			} else {
				(slices, offset)
			};

			for _ in 0..len {
				let res = decode_param(t, &tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			let result = DecodeResult {
				token: Token::FixedArray(tokens),
				new_offset: if is_dynamic { offset + 1 } else { new_offset },
			};

			Ok(result)
		}
		ParamType::Tuple(ref t) => {
			let is_dynamic = param.is_dynamic();

			// The first element in a dynamic Tuple is an offset to the Tuple's data
			// For a static Tuple the data begins right away
			let (tail, mut new_offset) = if is_dynamic {
				(&slices[(as_u32(peek(slices, offset)?)? as usize / 32)..], 0)
			} else {
				(slices, offset)
			};

			let len = t.len();
			let mut tokens = Vec::with_capacity(len);
			for i in 0..len {
				let res = decode_param(&t[i], &tail, new_offset)?;
				new_offset = res.new_offset;
				tokens.push(res.token);
			}

			// The returned new_offset depends on whether the Tuple is dynamic
			// dynamic Tuple -> follows the prefixed Tuple data offset element
			// static Tuple  -> follows the last data element
			let result = DecodeResult {
				token: Token::Tuple(tokens),
				new_offset: if is_dynamic { offset + 1 } else { new_offset },
			};

			Ok(result)
		}
	}
}