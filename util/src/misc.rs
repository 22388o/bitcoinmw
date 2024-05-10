// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::misc::MiscErrorKind::*;
use bmw_core::*;
use bmw_log::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::str::from_utf8;

info!();

thread_local! {
	pub(crate) static DEBUG_INVALID_PATH: RefCell<bool> = RefCell::new(false);
}

#[ErrorKind]
enum MiscErrorKind {
	IllegalArgument,
	IO,
}

/// Utility to canonicalize a path relative to a base directory (used in http and rustlet)
pub fn canonicalize_base_path(base_dir: &String, path: &String) -> Result<String, Error> {
	if path.len() < 1 {
		return err!(IO, "invalid path");
	}
	let base_dir = PathBuf::from(base_dir);
	let base_dir = base_dir.canonicalize()?;
	let mut ret = base_dir.clone();
	let path = from_utf8(&path.as_bytes()[1..])?.to_string();
	ret.push(path);
	let ret = ret.canonicalize()?;

	if !ret.starts_with(base_dir) {
		err!(IO, "canonicalized version is above the base_dir")
	} else {
		let ret_str = ret.to_str();

		let debug_invalid_str =
			DEBUG_INVALID_PATH.with(|f| -> Result<bool, Error> { Ok(*f.borrow()) })?;

		if ret_str.is_some() && !debug_invalid_str {
			Ok(ret_str.unwrap().to_string())
		} else {
			err!(IO, "could not generate string from the path specfied")
		}
	}
}

/// Utility to convert a u128 to an arbitrary length slice (up to 16 bytes).
pub fn u128_to_slice(mut n: u128, slice: &mut [u8]) -> Result<(), Error> {
	let len = slice.len();
	if len > 16 {
		let fmt = format!("slice must be equal to or less than 16 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}

	for i in (0..len).rev() {
		slice[i] = (n & 0xFF) as u8;
		n >>= 8;
	}

	if n != 0 {
		// this is an overflow, but for our purposes we return "MAX".
		for i in 0..len {
			slice[i] = 0xFF;
		}
	}

	Ok(())
}

/// Utility to convert an arbitrary length slice (up to 16 bytes) to a u128.
pub fn slice_to_u128(slice: &[u8]) -> Result<u128, Error> {
	let len = slice.len();
	if len > 16 {
		let fmt = format!("slice must be equal to or less than 16 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}
	let mut ret = 0;
	for i in 0..len {
		ret <<= 8;
		ret |= (slice[i] & 0xFF) as u128;
	}

	Ok(ret)
}

/// Utility to convert a usize to an arbitrary length slice (up to 8 bytes).
pub fn usize_to_slice(mut n: usize, slice: &mut [u8]) -> Result<(), Error> {
	let len = slice.len();
	if len > 8 {
		let fmt = format!("slice must be equal to or less than 8 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}

	for i in (0..len).rev() {
		slice[i] = (n & 0xFF) as u8;
		n >>= 8;
	}

	if n != 0 {
		// this is an overflow, but for our purposes we return "MAX".
		for i in 0..len {
			slice[i] = 0xFF;
		}
	}

	Ok(())
}

/// Utility to convert an arbitrary length slice (up to 8 bytes) to a usize.
pub fn slice_to_usize(slice: &[u8]) -> Result<usize, Error> {
	let len = slice.len();
	if len > 8 {
		let fmt = format!("slice must be equal to or less than 8 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}
	let mut ret = 0;
	for i in 0..len {
		ret <<= 8;
		ret |= (slice[i] & 0xFF) as usize;
	}

	Ok(ret)
}

/// Utility to convert a u64 to an arbitrary length slice (up to 8 bytes).
pub fn u64_to_slice(n: u64, slice: &mut [u8]) -> Result<(), Error> {
	usize_to_slice(try_into!(n)?, slice)
}

/// Utility to convert an arbitrary length slice (up to 8 bytes) to a u64.
pub fn slice_to_u64(slice: &[u8]) -> Result<u64, Error> {
	try_into!(slice_to_usize(slice)?)
}

/// Utility to convert a u32 to an arbitrary length slice (up to 4 bytes).
pub fn u32_to_slice(mut n: u32, slice: &mut [u8]) -> Result<(), Error> {
	let len = slice.len();
	if len > 4 {
		let fmt = format!("slice must be equal to or less than 4 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}

	for i in (0..len).rev() {
		slice[i] = (n & 0xFF) as u8;
		n >>= 8;
	}

	if n != 0 {
		// this is an overflow, but for our purposes we return "MAX".
		for i in 0..len {
			slice[i] = 0xFF;
		}
	}

	Ok(())
}

/// Utility to convert an arbitrary length slice (up to 8 bytes) to a u32.
pub fn slice_to_u32(slice: &[u8]) -> Result<u32, Error> {
	let len = slice.len();
	if len > 4 {
		let fmt = format!("slice must be equal to or less than 4 bytes ({})", len);
		return err!(IllegalArgument, fmt);
	}
	let mut ret = 0;
	for i in 0..len {
		ret <<= 8;
		ret |= (slice[i] & 0xFF) as u32;
	}

	Ok(ret)
}

/// Set the maximum possible value in this slice
pub(crate) fn set_max(slice: &mut [u8]) {
	for i in 0..slice.len() {
		slice[i] = 0xFF;
	}
}
