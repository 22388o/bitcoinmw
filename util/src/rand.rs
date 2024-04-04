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

use bmw_deps::ring::rand::{SecureRandom, SystemRandom};
use std::cell::RefCell;

thread_local!(
	#[doc(hidden)]
	pub static RAND_CONTEXT: RefCell<SystemRandom> = RefCell::new(SystemRandom::new())

);

/// Return a secure random u32 value using ring::rand::SystemRandom.
pub fn random_u32() -> u32 {
	let mut buffer = [0u8; 4];
	fill(&mut buffer);
	u32::from_be_bytes(buffer)
}

/// Return a secure random u64 value using ring::rand::SystemRandom.
pub fn random_u64() -> u64 {
	let mut buffer = [0u8; 8];
	fill(&mut buffer);
	u64::from_be_bytes(buffer)
}

/// Return a secure random u128 value using ring::rand::SystemRandom.
pub fn random_u128() -> u128 {
	let mut buffer = [0u8; 16];
	fill(&mut buffer);
	u128::from_be_bytes(buffer)
}

/// Return a secure random byte value using ring::rand::SystemRandom.
pub fn random_bytes(mut buffer: &mut [u8]) {
	fill(&mut buffer);
}

fn fill(mut buffer: &mut [u8]) {
	// we use unwrap because we'd rather panic than have a bad random number
	RAND_CONTEXT.with(|f| f.borrow().fill(&mut buffer).unwrap());
}
