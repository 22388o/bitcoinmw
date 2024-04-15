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

/// The [`crate::Configurable`] trait, when implemented, allows structs to be configured.
/// Currently, [`u8`], [`u16`], [`u32`], [`u64`], [`u128`], [`usize`], [`std::string::String`] and a string tuple `(String, String)` are
/// supported. Also, a [`std::vec::Vec`] of any of these types are supported. This should generally be used with the
/// proc-macro Configurable, but that is done at a higher level crate so see its documentation
/// there in the `derive` crate.
pub trait Configurable {
	/// sets the configuration with the specified `name` to the specified [`prim@u8`] value
	fn set_u8(&mut self, name: &str, value: u8);
	/// sets the configuration with the specified `name` to the specified [`prim@u16`] value
	fn set_u16(&mut self, name: &str, value: u16);
	/// sets the configuration with the specified `name` to the specified [`prim@u32`] value
	fn set_u32(&mut self, name: &str, value: u32);
	/// sets the configuration with the specified `name` to the specified [`prim@u64`] value
	fn set_u64(&mut self, name: &str, value: u64);
	/// sets the configuration with the specified `name` to the specified [`prim@u128`] value
	fn set_u128(&mut self, name: &str, value: u128);
	/// sets the configuration with the specified `name` to the specified [`prim@usize`] value
	fn set_usize(&mut self, name: &str, value: usize);
	/// sets the configuration with the specified `name` to the specified [`std::string::String`] value
	fn set_string(&mut self, name: &str, value: String);
	/// sets the configuration with the specified `name` to the specified [`prim@bool`] value
	fn set_bool(&mut self, name: &str, value: bool);
	/// sets the configuration with the specified `name` to the specified `(String, String)` value
	fn set_string_tuple(&mut self, name: &str, value: (String, String));
	/// returns a [`std::collections::HashSet`] with the configurations that allow duplicates.
	/// This is used by the [`crate::config`] macro when [`std::vec::Vec`] configuration
	/// options are used.
	fn allow_dupes(&self) -> HashSet<String>;
}
