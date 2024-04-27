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

use proc_macro::TokenStream;

use bmw_deps::failure;
mod impls;
mod types;

// Note about tarpaulin. Tarpaulin doesn't cover proc_macros so we disable it throughout this
// crate.

// errorkind section
mod errorkind;
use crate::errorkind::do_derive_errorkind;

/// The [`crate::ErrorKind()`] proc_macro_attribute automatically generates Failure messages and
/// required trait implementations for the enum that it is attached to. All that is needed is to
/// use three libraries as shown in the example below. This enum is designed to be used with
/// [`bmw_base::Error`] and can be returned using the [`bmw_base::err`] or [`bmw_base::map_err`]
/// macros.
///```
/// use bmw_derive2::*;
/// use bmw_base::*;
/// use bmw_deps::failure;
///
/// #[ErrorKind]
/// enum MyErrorKind {
///     InternalError,
///     /// the system crashed
///     SystemCrashError,
/// }
///
/// fn ret_err() -> Result<(), Error> {
///     err!(MyErrorKind::InternalError, "custom message")
/// }
///
/// fn ret_err2() -> Result<(), Error> {
///     err!(MyErrorKind::SystemCrashError, "another message")
/// }
///
/// fn main() -> Result<(), Error> {
///     // generate an internal error
///     let err = ret_err().unwrap_err();
///     // since this error did not have any comments, it will be prefixed by "InternalError: "
///     assert_eq!(err.kind().to_string(), "InternalError: custom message");
///
///     // generate a system crash error
///     let err = ret_err2().unwrap_err();
///     // this error will be prefixed by the specified comment i.e. "the system crashed: "
///     assert_eq!(err.kind().to_string(), "the system crashed: another message");
///     
///     Ok(())
/// }
///```
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[allow(non_snake_case)]
pub fn ErrorKind(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_errorkind(attr, item)
}

// ser section
mod ser;
use crate::ser::do_derive_serializable;

/// The [`crate::derive_serializable()`] proc_macro_derive derives a the Serializable trait for
/// structs and enums. It's important to note that the elements within the data structure must
/// implement [`bmw_base::Serializable`] as well. Since the [`bmw_base`] crate already implements
/// [`bmw_base::Serializable`] for most of the primative types, Tuples, [`std::vec::Vec`] and
/// [`std::option::Option`] it's possible to derive many data types. It's important to note that
/// generics are not currently supported so if you need to implement Serilizable for something with
/// a generic, you will need to do your own implementation.
/// # Example
///```
/// use bmw_base::*;
/// use bmw_deps::rand;
/// use bmw_derive2::Serializable;
/// use std::fmt::Debug;
///
/// // create a serializable to include in our other serializable
/// #[derive(Serializable, PartialEq, Debug)]
/// struct OtherSer {
///     a: usize,
///     b: String,
/// }
///
/// // create a serializable with all supported types
/// #[derive(Serializable, PartialEq, Debug)]
/// struct SerAll {
///     a: u8,
///     b: i8,
///     c: u16,
///     d: i16,
///     e: u32,
///     f: i32,
///     g: u64,
///     h: i64,
///     i: u128,
///     j: i128,
///     k: usize,
///     l: bool,
///     m: f64,
///     n: char,
///     v: Vec<u8>,
///     o: Option<u8>,
///     s: String,
///     x: Vec<String>,
///     y: Vec<Option<(String, ())>>,
///     z: Option<Vec<OtherSer>>,
/// }
///
/// // helper function that serializes and deserializes a Serializable and tests them for
/// // equality
/// fn ser_helper<S: Serializable + Debug + PartialEq>(ser_in: S) -> Result<(), Error> {
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_in)?;
///     let ser_out: S = deserialize(&mut &v[..])?;
///     assert_eq!(ser_out, ser_in);
///     Ok(())
/// }
///
/// fn main() -> Result<(), Error> {
///     // create a SerAll with random values
///     let rand_u8: u8 = rand::random();
///     let rand_ch: char = rand_u8 as char;
///     let ser_out = SerAll {
///         a: rand::random(),
///         b: rand::random(),
///         c: rand::random(),
///         d: rand::random(),
///         e: rand::random(),
///         f: rand::random(),
///         g: rand::random(),
///         h: rand::random(),
///         i: rand::random(),
///         j: rand::random(),
///         k: rand::random(),
///         l: false,
///         m: rand::random(),
///         n: rand_ch,
///         v: vec![rand::random(), rand::random(), rand::random()],
///         o: Some(rand::random()),
///         s: "abcdef".to_string(),
///         x: vec!["123".to_string(), "456".to_string()],
///         y: vec![
///             None,
///             None,
///             None,
///             Some(("hi".to_string(), ())),
///             Some(("hi2".to_string(), ())),
///         ],
///         z: None,
///     };
///
///     // test it
///     ser_helper(ser_out)?;
///
///     Ok(())
/// }
///```
#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serializable(strm: TokenStream) -> TokenStream {
	do_derive_serializable(strm)
}

// config section
mod config;
use crate::config::do_derive_configurable;

#[proc_macro_derive(Configurable, attributes(required, options))]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm)
}
