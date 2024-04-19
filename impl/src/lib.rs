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

//! # The BitcoinMW impl crate
//! This crate is a proc_macro crate and it includes the Serializable macro and the Configurable
//! macro.
//! The serializable macro implements the bmw_ser::Serializable trait for any struct or enum.
//! The Configurable macro makes a struct configurable via the [`bmw_core::config`] macro.
//!
//! # Serializable example
//!
//!```
//! use bmw_impl::Serializable;
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
//! This macro is used in the bmw_util and other crates within BitcoinMW. For additional examples, see the bmw_util documentation.
//!
//! # Generics
//! It's important to note that generics are not currently supported and will result in an error. If you need
//! generics, currenly you must build your own bmw_ser::Serializable implementation.
//!
//! # Configurable
//! The configurable derive macro is used to allow for fast and easy configuration of specified structs.
//! Configurations can be of the following types: [`u8`], [`u16`], [`u32`], [`u64`], [`u128`],
//! [`usize`], [`bool`], [`std::string::String`], `(String, String)`. [`std::vec::Vec`] of any of
//! these types are also configurable. See the detailed example below for further details on how to
//! use this derive macro.
//!
//! # Configurable example
//!```
//! use bmw_impl::Configurable; // derive proc-macro
//! use bmw_core::{Configurable, config}; // Configurable trait and the config macro
//!
//! // define your struct and derive 'Configurable'.
//! // the supported types are bool, u8, u16, u32, u64, u128, usize, bool,
//! // String, and (String, String). Vec of each of these types also are supported.
//! // (e.g. 'headers' below.)
//! #[derive(Configurable)]
//! #[options = "ConfigOptions"] // The name of the Options enum for this struct. (default
//!                              // {StructName}Options. Note that this will be a 'pub' enum
//!                              // so it can be exported and used in other crates, etc.
//! struct MyStruct {
//!     // the 'required' helper attribute indicates this field must always be specified in the
//!     // config macro.
//!     #[required] threads: usize,
//!     timeout: u128,
//!     stats_frequency: u64,
//!     log_file_location: String,
//!     headers: Vec<(String, String)>,
//! }
//!
//! // implement the Default trait for the struct and the proc-macro does the rest.
//! // note that if Vecs have elements in them, configuring data will not delete them, the new
//! // elements will just be appended to the vec.
//! impl Default for MyStruct {
//!     fn default() -> Self {
//!         Self {
//!             threads: 1,
//!             timeout: 10_000,
//!             stats_frequency: 5_000,
//!             log_file_location: "~/.bmw/mylog.log".to_string(),
//!             headers: vec![],
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // call the config macro to build your struct with the specified values that
//!     // overwrite the default values. Note that the names of the configurations are
//!     // the names in the struct converted to 'Pascal' case. See the 'convert_case'
//!     // crate for details on this.
//!     let my_config = config!(
//!         MyStruct, // name of your struct
//!         ConfigOptions, // name of the options enum. This would be called
//!         // {struct_name}Options if the #[options = "ConfigOptions"] attribute was not
//!         // specified above.
//!         vec![
//!             Threads(6), // for non-vec, a single value is specified
//!             Headers(("Content-Type", "text/html")), // vec's may have multiple entries
//!             Headers(("Connection", "keep-alive")),
//!         ]
//!     );
//!
//!     // check for errors. Duplicates for non-vecs are considered errors
//!     // also if a 'required' field is missing an error will be returned.
//!     let my_config = match my_config {
//!         Ok(my_config) => my_config,
//!         Err(e) => {
//!             panic!("config returned err: {}", e);
//!         }
//!     };
//!
//!     assert_eq!(my_config.threads, 6); // specified
//!     assert_eq!(my_config.timeout, 10_000); // default
//!     assert_eq!(my_config.stats_frequency, 5_000); // default
//!     assert_eq!(my_config.log_file_location, "~/.bmw/mylog.log".to_string()); // default
//!     assert_eq!(
//!         my_config.headers,
//!         vec![
//!             ("Content-Type".to_string(), "text/html".to_string()),
//!             ("Connection".to_string(), "keep-alive".to_string())
//!         ]
//!     ); // specified with two values
//!
//!     // some errors
//!
//!     // error because required field (Threads) is not specified.
//!     assert!(config!(MyStruct, ConfigOptions, vec![Timeout(5_000)]).is_err());
//!
//!     // error because Threads is specified twice.
//!     assert!(config!(MyStruct, ConfigOptions, vec![Threads(10), Threads(20)]).is_err());
//! }
//!
//!```

mod config;
mod constants;
mod document;
mod ser;
mod types;

extern crate proc_macro;
use crate::config::do_derive_configurable;
use crate::document::do_derive_document;
use crate::ser::do_derive_serialize;
use proc_macro::TokenStream;

/// This is a proc macro for implementing the bmw_ser::Serializable trait. See the [`crate`]
/// documentation for examples.
#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serialize(strm: TokenStream) -> TokenStream {
	do_derive_serialize(strm)
}

/// This is a proc macro for implementing the bmw_conf::Configurable trait. See the [`crate`]
/// documentation for examples.
#[proc_macro_derive(Configurable, attributes(required, options))]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm)
}

/// The [`crate::document()`] proc_macro_attribute is used to document traits and macros. While the
/// work is done by this macro, all that needs to be done to use it is to place it before a macro
/// or a trait. All of what it does depends on the [`crate::add_doc()`] attribute. See that
/// attribute for details on how to use the document attribute. Below are two simple examples
/// showing where to place the document attribute.
///
///```
/// // trait example
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait mytrait {
///     /// This is the write function.
///     /// more comments.
///     /// ...
///     #[add_doc(doc_point)]
///     /// # Example
///     ///```
///     /// myvalue.write();
///     ///```
///     #[add_doc(see: "bmw_log::Log::log")]
///     #[add_doc(error: "bmw_err::ErrKind::IO" - "if an i/o error occurs while writing")]
///     #[add_doc(input: data - "data to write")]
///     #[add_doc(return: "this function returns the [`unit`] primitive")]
///     fn write(&mut self, data: &[u8]) -> Result<(), Error>;
/// }   
///```
///
///```
/// // macro example
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// #[add_doc(input: values - "value to add to 1", " usize ")]
/// #[macro_export]
/// macro_rules! add_to_1 {
///        ($name:expr) => {
///                 let value = $name as usize + 1;
///                 println!("value: {}", value);
///        };
/// }
///```
///
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn document(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_document(attr, item)
}

/// The [`crate::add_doc()`] proc_macro_attribute is used as a marker to let the [`crate::document()`]
/// proc_macro_attribute know what to document. It is important to note that currently, these
/// proc_macro_attributes only support documenting traits and macros. That is because in BitcoinMW,
/// the public interfaces are typically exposed through traits and macros. Using these
/// proc_macro_attributes on anything else will result in undefined behaviour.
///
/// The [`crate::add_doc()`] proc_macro_attribute takes a parameter that has a specific syntax
/// needed to document code. The basic form of this attribute is as follows:
///```text
/// #[add_doc(<command>: <arguments>]
///```
///
/// # Documenting Input Parameters
/// To document input parameters, the following format is used:
///```text
/// #[add_doc(input: <input_name> - <comment> (, <optional type string>))]
///```
/// where <input_name> \<comment\> and (optional type string) are all string literals. See example
/// below.
///
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait TestDoc {
///     #[add_doc(input: "myvalue1" - "this is a comment about `myvalue1`.")]
///     fn abc(&self, myvalue1: usize, myvalue2: bool) -> Result<String, Error>;
///
/// }
///```
/// The optional type string is used for macros where the proc_macro cannot determine anything about the
/// type. This allows the user to specify a type. See example below.
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// #[add_doc(input: values - "value to add to 1", " usize ")]
/// #[macro_export]
/// macro_rules! add_to_1 {
///        ($name:expr) => {
///                 let value = $name as usize + 1;
///                 println!("value: {}", value);
///        };
/// }
///```
/// # Documenting Errors
/// To document errors, the following format is used:
///```text
/// #[add_doc(error: <path_to_error_type> - <error comment>)]
///```
/// Both parameters are string literals. See example below.
///
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait mytrait {
///     #[add_doc(error: "bmw_err::ErrKind::IO" - "if an i/o error occurs while writing")]
///     fn write(&mut self, data: &[u8]) -> Result<(), Error>;
/// }
///```
/// # Documenting Also See
///
/// To point users to additional documentation, `see` can be used. The format is as follows:
///```text
/// #[add_doc(see: <path_to_additional_documentation>)]
///```
/// The parameter is expected to be a string literal. See example below.
///
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait mytrait {
///     #[add_doc(see: "bmw_log::Log::log")]
///     fn write(&mut self, data: &[u8]) -> Result<(), Error>;
/// }
///```
/// # Documenting Return
/// To document the returned value, `return` can be used. The format is as follows:
///```text
/// #[add_doc(return: <comment>)]
///```
/// \<comment\> is expected to be a string literal. See example below.
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait mytrait {
///     #[add_doc(return: "this function returns the [`unit`] primitive")]
///     fn write(&mut self, data: &[u8]) -> Result<(), Error>;
/// }
///```
/// # Using the add_doc with other documentation
/// The [`crate::add_doc()`] and [`crate::document()`] proc_macro_attributes work fine with other
/// documentation. When generating documentation, by default add_doc will add it's documentation
/// just before the function so all other documentation will appear above it, however a `doc_point`
/// may be set to place certain documentation below the documentation that add_doc produces. See
/// example below.
///
///```
/// use bmw_err::*;
/// use bmw_impl::*;
///
/// #[document]
/// pub trait mytrait {
///     /// This is the write function.
///     /// more comments.
///     /// ...
///     #[add_doc(doc_point)]
///     /// # Example
///     ///```
///     /// myvalue.write();
///     ///```
///     #[add_doc(see: "bmw_log::Log::log")]
///     #[add_doc(error: "bmw_err::ErrKind::IO" - "if an i/o error occurs while writing")]
///     #[add_doc(input: data - "data to write")]
///     #[add_doc(return: "this function returns the [`unit`] primitive")]
///     fn write(&mut self, data: &[u8]) -> Result<(), Error>;
/// }
///```
/// In the case above, the documentation is put after the initial comments, but before the example.
/// If the `doc_point` attribute were not placed there, the example would be above the generated
/// documentation.
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn add_doc(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// add doc doesn't actually change anything, it's just a marker used by the document attribute
	// which modifies the TokenStream. So, we just return the input token stream unchanged.
	item
}
