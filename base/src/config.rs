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

use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use std::collections::HashSet;

pub trait Configurable: DynClone {
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
	/// sets the configuration with the specified `name` to the specified `Configurable` value
	fn set_configurable(&mut self, name: &str, value: &dyn Configurable);
	/// returns a [`std::collections::HashSet`] with the configurations that allow duplicates.
	/// This is used by the `config` macro when [`std::vec::Vec`] configuration
	/// options are used.
	fn allow_dupes(&self) -> HashSet<String>;
	/// returns a list of required parameters, if any of these are not specified,
	/// [`crate::configurable`] will return an error
	fn required(&self) -> Vec<String>;

	fn get_u8_params(&self) -> Vec<(String, u8)>;
	fn get_u16_params(&self) -> Vec<(String, u16)>;
	fn get_u32_params(&self) -> Vec<(String, u32)>;
	fn get_u64_params(&self) -> Vec<(String, u64)>;
	fn get_u128_params(&self) -> Vec<(String, u128)>;
	fn get_usize_params(&self) -> Vec<(String, usize)>;
	fn get_vec_usize_params(&self) -> Vec<(String, Vec<usize>)>;
	fn get_bool_params(&self) -> Vec<(String, bool)>;
	fn get_string_params(&self) -> Vec<(String, String)>;
	fn get_configurable_params(&self) -> Vec<(String, Box<dyn Configurable>)>;
}

clone_trait_object!(Configurable);

pub trait ConfigurableOptions {
	fn name(&self) -> &str;
	fn value_usize(&self) -> Option<usize>;
	fn value_bool(&self) -> Option<bool>;
	fn value_u8(&self) -> Option<u8>;
	fn value_u16(&self) -> Option<u16>;
	fn value_u32(&self) -> Option<u32>;
	fn value_u64(&self) -> Option<u64>;
	fn value_u128(&self) -> Option<u128>;
	fn value_string(&self) -> Option<String>;
	fn value_configurable(&self) -> Option<Box<dyn Configurable>>;
}
