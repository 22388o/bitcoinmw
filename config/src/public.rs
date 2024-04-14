// Copyright (c) 2023-2024, The BitcoinMW Developers
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

use std::collections::HashSet;

/// The configurable trait, when implemented allows structs to be configured.
/// Currently, u8, u16, u32, u64, u128, usize, string and a string tuple (String, String) are
/// supported. Also Vec of any of these types are supported. This should generally be used with the
/// proc-macro Configurable, but that is done at a higher level crate so see it's documentation
/// there in the `derive` crate.
pub trait Configurable {
	fn set_u8(&mut self, name: &str, value: u8);
	fn set_u16(&mut self, name: &str, value: u16);
	fn set_u32(&mut self, name: &str, value: u32);
	fn set_u64(&mut self, name: &str, value: u64);
	fn set_u128(&mut self, name: &str, value: u128);
	fn set_usize(&mut self, name: &str, value: usize);
	fn set_string(&mut self, name: &str, value: String);
	fn set_bool(&mut self, name: &str, value: bool);
	fn set_string_tuple(&mut self, name: &str, value: (String, String));
	fn allow_dupes(&self) -> HashSet<String>;
}
