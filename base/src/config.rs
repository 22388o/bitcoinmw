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

use bmw_deps::downcast::{downcast, Any};
use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use std::collections::HashSet;

pub trait PassthroughValue: DynClone + Any {}
clone_trait_object!(PassthroughValue);
downcast!(dyn PassthroughValue);
impl<T: Clone + Any> PassthroughValue for T {}

#[derive(Clone)]
pub struct Passthrough {
	pub name: String,
	pub value: Box<dyn PassthroughValue>,
}

/// The [`Configurable`] trait is used as a generic way to configure data structures. This trait is
/// used by the `derive` crate and should generally through the derive proc_macro.
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

	fn set_passthrough(&mut self, passthrough: Passthrough);
	/// returns a [`std::collections::HashSet`] with the configurations that allow duplicates.
	/// This is used by the `config` macro when [`std::vec::Vec`] configuration
	/// options are used.
	fn allow_dupes(&self) -> HashSet<String>;
	/// returns a list of required parameters, if any of these are not specified,
	/// the [`crate::configure`] macro will return an error
	fn required(&self) -> Vec<String>;

	/// get a [`std::vec::Vec`] of all names and values of the [`usize`] parameters within this
	/// [`Configurable`].
	fn get_usize_params(&self) -> Vec<(String, usize)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u8`] parameters within this
	/// [`Configurable`].
	fn get_u8_params(&self) -> Vec<(String, u8)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u16`] parameters within this
	/// [`Configurable`].
	fn get_u16_params(&self) -> Vec<(String, u16)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u32`] parameters within this
	/// [`Configurable`].
	fn get_u32_params(&self) -> Vec<(String, u32)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u64`] parameters within this
	/// [`Configurable`].
	fn get_u64_params(&self) -> Vec<(String, u64)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u128`] parameters within this
	/// [`Configurable`].
	fn get_u128_params(&self) -> Vec<(String, u128)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`bool`] parameters within this
	/// [`Configurable`].
	fn get_bool_params(&self) -> Vec<(String, bool)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`String`] parameters within this
	/// [`Configurable`].
	fn get_string_params(&self) -> Vec<(String, String)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`Configurable`] parameters within this
	/// [`Configurable`].
	fn get_configurable_params(&self) -> Vec<(String, Box<dyn Configurable>)>;

	/// get a [`std::vec::Vec`] of all names and values of the [`usize`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_usize_params(&self) -> Vec<(String, Vec<usize>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u8`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_u8_params(&self) -> Vec<(String, Vec<u8>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u16`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_u16_params(&self) -> Vec<(String, Vec<u16>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u32`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_u32_params(&self) -> Vec<(String, Vec<u32>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u64`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_u64_params(&self) -> Vec<(String, Vec<u64>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`u128`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_u128_params(&self) -> Vec<(String, Vec<u128>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`bool`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_bool_params(&self) -> Vec<(String, Vec<bool>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`String`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_string_params(&self) -> Vec<(String, Vec<String>)>;
	/// get a [`std::vec::Vec`] of all names and values of the [`Configurable`] parameters that allow
	/// multiple entries within this [`Configurable`].
	fn get_vec_configurable_params(&self) -> Vec<(String, Vec<Box<dyn Configurable>>)>;
}

clone_trait_object!(Configurable);

/// The [`ConfigurableOptions`] trait is used as a generic way to configure data structures. This trait is
/// used by the `derive` crate and should generally through the derive proc_macro.
pub trait ConfigurableOptions {
	/// return the name of this [`ConfigurableOptions`] structure.
	fn name(&self) -> &str;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`usize`]. Otherwise,
	/// return [`None`].
	fn value_usize(&self) -> Option<usize>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`bool`]. Otherwise,
	/// return [`None`].
	fn value_bool(&self) -> Option<bool>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`u8`]. Otherwise,
	/// return [`None`].
	fn value_u8(&self) -> Option<u8>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`u16`]. Otherwise,
	/// return [`None`].
	fn value_u16(&self) -> Option<u16>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`u32`]. Otherwise,
	/// return [`None`].
	fn value_u32(&self) -> Option<u32>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`u64`]. Otherwise,
	/// return [`None`].
	fn value_u64(&self) -> Option<u64>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`u128`]. Otherwise,
	/// return [`None`].
	fn value_u128(&self) -> Option<u128>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`String`]. Otherwise,
	/// return [`None`].
	fn value_string(&self) -> Option<String>;
	/// return the value of this [`ConfigurableOptions`] structure if it is a [`Configurable`]. Otherwise,
	/// return [`None`].
	fn value_configurable(&self) -> Option<Box<dyn Configurable>>;

	fn value_passthrough(&self) -> Option<Passthrough>;
}
