// Copyright (c) 2023-2024, The BitcoinMW Developers // Some code and concepts from: // * Grin: https://github.com/mimblewimble/grin // * Arti: https://gitlab.torproject.org/tpo/core/arti // * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw //
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

use bmw_deps::ring::rand::{SecureRandom, SystemRandom};
use std::cell::RefCell;

thread_local!(
	pub static RAND_CONTEXT: RefCell<
		SystemRandom
	> = RefCell::new(SystemRandom::new())

);

pub fn random_u32() -> u32 {
	let mut buffer = [0u8; 4];
	fill(&mut buffer);
	u32::from_be_bytes(buffer)
}

pub fn random_u64() -> u64 {
	let mut buffer = [0u8; 8];
	fill(&mut buffer);
	u64::from_be_bytes(buffer)
}

pub fn random_u128() -> u128 {
	let mut buffer = [0u8; 16];
	fill(&mut buffer);
	u128::from_be_bytes(buffer)
}

pub fn random_bytes(mut buffer: &mut [u8]) {
	fill(&mut buffer);
}

fn fill(mut buffer: &mut [u8]) {
	// we use unwrap because we'd rather panic than have a bad random number
	RAND_CONTEXT.with(|f| f.borrow().fill(&mut buffer).unwrap());
}

#[cfg(test)]
mod test {
	use crate::rand::*;
	use bmw_err::*;
	use bmw_log::*;

	debug!();

	#[test]
	fn test_random_u32() -> Result<(), Error> {
		let r1 = random_u32();
		let r2 = random_u32();
		let r3 = random_u32();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_u64() -> Result<(), Error> {
		let r1 = random_u64();
		let r2 = random_u64();
		let r3 = random_u64();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_u128() -> Result<(), Error> {
		let r1 = random_u128();
		let r2 = random_u128();
		let r3 = random_u128();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_bytes() -> Result<(), Error> {
		let mut buffer1 = [0u8; 10];
		let mut buffer2 = [0u8; 10];
		let mut buffer3 = [0u8; 10];

		random_bytes(&mut buffer1);
		random_bytes(&mut buffer2);
		random_bytes(&mut buffer3);
		debug!("r1={:?},r2={:?},r3={:?}", buffer1, buffer2, buffer3)?;
		assert!(buffer1 != buffer2 || buffer2 != buffer3);
		Ok(())
	}
}
