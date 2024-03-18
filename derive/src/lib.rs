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

//! # The BMW Derive crate
//! This crate is a proc_macro crate and it includes the Serializable macro.
//! This macro implements the [`bmw_ser::Serializable`] trait for any struct or enum.
//!
//! # Examples
//!
//!```
//! use bmw_derive::Serializable;
//! use bmw_ser::Serializable;
//! use bmw_err::Error;
//!
//! #[derive(Serializable)]
//! struct MyStruct {
//!     id: u64,
//!     is_member: bool,
//!     name: String,
//! }
//!
//! #[derive(Serializable)]
//! enum MyEnum {
//!     Type1(String),
//!     Type2(u64),
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let _s1 = MyStruct {
//!         id: 1234,
//!         is_member: true,
//!         name: "Hagrid".to_string(),
//!     };
//!
//!     let _s2 = MyEnum::Type1("something".to_string());
//!
//!     Ok(())
//! }
//!
//!```
//!
//! This macro is used in the bmw_util and other crates within BMW. For additional examples, see the bmw_util documentation.

extern crate proc_macro;
use crate::derive::do_derive_serialize;
use proc_macro::TokenStream;

/// This is a proc macro for implementing the [`bmw_ser::Serializable`] trait. See the [`crate`]
/// documentation for examples.
#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serialize(strm: TokenStream) -> TokenStream {
	do_derive_serialize(strm)
}

mod derive;
mod types;
