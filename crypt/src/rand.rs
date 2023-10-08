// Copyright (c) 2023, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::types::RngWrapper;
use crate::RngCompatExt;
use bmw_deps::old_rand_core::{
	CryptoRng as OldCryptoRng, Error as OldError, RngCore as OldRngCore,
};
use bmw_deps::rand_core::{CryptoRng, RngCore};

impl<T: RngCore + Sized> RngCompatExt for T {
	type Wrapper = RngWrapper<T>;
	fn rng_compat(self) -> RngWrapper<Self> {
		self.into()
	}
}

impl<T: RngCore> From<T> for RngWrapper<T> {
	fn from(rng: T) -> RngWrapper<T> {
		RngWrapper(rng)
	}
}

impl<T: RngCore> OldRngCore for RngWrapper<T> {
	fn next_u32(&mut self) -> u32 {
		self.0.next_u32()
	}
	fn next_u64(&mut self) -> u64 {
		self.0.next_u64()
	}
	fn fill_bytes(&mut self, dest: &mut [u8]) {
		self.0.fill_bytes(dest);
	}
	fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), OldError> {
		self.0.try_fill_bytes(dest).map_err(|e| err_to_old(&e))
	}
}

impl<T: RngCore> RngCore for RngWrapper<T> {
	fn next_u32(&mut self) -> u32 {
		self.0.next_u32()
	}
	fn next_u64(&mut self) -> u64 {
		self.0.next_u64()
	}
	fn fill_bytes(&mut self, dest: &mut [u8]) {
		self.0.fill_bytes(dest);
	}
	fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), bmw_deps::rand::Error> {
		self.0.try_fill_bytes(dest)
	}
}

impl<T: CryptoRng> OldCryptoRng for RngWrapper<T> {}
impl<T: CryptoRng> CryptoRng for RngWrapper<T> {}

fn err_to_old(e: &bmw_deps::rand::Error) -> OldError {
	use std::num::NonZeroU32;
	if let Some(code) = e.code() {
		code.into()
	} else {
		// CUSTOM_START is defined to be a nonzero value in rand_core,
		// so this conversion will succeed, so this unwrap can't panic.
		#[allow(clippy::unwrap_used)]
		let nz: NonZeroU32 = OldError::CUSTOM_START.try_into().unwrap();
		nz.into()
	}
}
