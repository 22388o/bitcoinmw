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

//! # The BitcoinMW derive crate
//! The bmw_derive crate implements the low level derive macros which are used in BitcoinMW.

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

/// The [`crate::Serializable`] proc_macro_derive derives a the Serializable trait for
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

/// The [`crate::Configurable`] macro creates data structures that turns a regular struct into a
/// configurable struct in which defaults can be overwritten with the [`bmw_base::configure`]
/// macro. The required and options helper attributes allow the user to
/// indicate required fields and to overight the default name of the options enum, which is used to
/// configure the struct, respectively.
/// The [`crate::Configurable`] macro generates data structures that transform a standard struct
/// into a configurable one. This configurable struct allows users to override defaults
/// using the configure macro. Additionally, the required and
/// options helper attributes enable users to specify required fields and customize the default name
/// of the options enum used for struct configuration, respectively.
/// # Supported types
/// The currently supported types are:
/// * [`u8`]
/// * [`u16`]
/// * [`u32`]
/// * [`u64`]
/// * [`u128`]
/// * [`usize`]
/// * [`bool`]
/// * [`std::string::String`]
/// * ([`std::string::String`], [`std::string::String`])
/// * [`std::vec::Vec`] of any of the above types.
/// # Vectors
/// Vectors are used to allow multiple entries for one type. So, for instance, let's say you want a
/// server to listen on multiple ports. You could include a port field as a `Vec<u16>`. Then, any
/// number of ports may be configured. See the example below for further details.
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_derive2::Configurable;
/// use crate::MyOptions::Headers;
///
/// // define a struct and derive 'Configurable'
/// #[derive(Configurable)]
/// #[options="MyOptions"] // use MyOptions instead of the default MyStructOptions
/// struct MyStruct {
///     #[required] // require 'name'
///     name: String,
///     size: usize,
///     headers: Vec<(String, String)>,
/// }
///
/// // The Default trait must be implemented
/// impl Default for MyStruct {
///     fn default() -> Self {
///         Self {
///             name: "test".to_string(),
///             size: 100,
///             headers: vec![],
///         }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     // configure the struct with required field "Name" set to "sam"
///     let x = configure!(MyStruct, MyOptions, vec![Name("sam")])?;
///
///     // assert that name is set and size is the default (100)
///     assert_eq!(x.name, "sam".to_string());
///     assert_eq!(x.size, 100);
///
///     // now set both name and size
///     let x = configure!(MyStruct, MyOptions, vec![Name("joe"), Size(101)])?;
///
///     // assert both values are correct
///     assert_eq!(x.name, "joe".to_string());
///     assert_eq!(x.size, 101);
///
///     let x = configure!(MyStruct, MyOptions, vec![
///         Name("chris"),
///         Headers(("Content-Type", "text/html")),
///         Headers(("Content-Length", "1234"))
///     ])?;
///
///     assert_eq!(x.name, "chris".to_string());
///
///     assert_eq!(x.headers, vec![
///         ("Content-Type".to_string(), "text/html".to_string()),
///         ("Content-Length".to_string(), "1234".to_string())
///     ]);
///
///     Ok(())
/// }
///
///```
/// Attempting to configure duplicates will result in an error as well as omitting a `required`
/// configuration.
///```
/// use bmw_base::*;
/// use bmw_derive2::Configurable;
///
/// #[derive(Configurable)]
/// struct MyStruct {
///     #[required]
///     name: String,
///     count: usize,
///     id: u128,
/// }
///
/// impl Default for MyStruct {
///     fn default() -> Self {
///         Self {
///             name: "sam".to_string(),
///             count: 10,
///             id: 1234,
///         }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     // error because required field "name" is not configured
///     assert!(configure!(MyStruct, MyStructOptions, vec![Id(0)]).is_err());
///
///     // error because duplicate counts are specified
///     assert!(
///         configure!(
///             MyStruct,
///             MyStructOptions,
///             vec![
///                 Count(1),
///                 Count(2),
///                 Name("joe")
///             ]
///         ).is_err()
///     );
///     Ok(())
/// }
///```
/// Duplicates are allowed for [`std::vec::Vec`]...
///```
/// use bmw_base::*;
/// use bmw_derive2::Configurable;
///
/// #[derive(Configurable)]
/// struct MyStruct {
///     val1: Vec<usize>,
///     val2: Vec<String>,
///     val3: u64,
/// }
///
///
/// impl Default for MyStruct {
///     fn default() -> Self {
///         Self {
///             val1: vec![],
///             val2: vec![],
///             val3: 0,
///         }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     let x = configure!(
///         MyStruct,
///         MyStructOptions,
///         vec![Val1(100), Val1(200), Val1(300), Val2("str1"), Val2("str2"), Val2("str3")]
///     )?;
///
///     // 3 entries that were specified
///     assert_eq!(x.val1, vec![100, 200, 300]);
///     // 3 entries that were specified
///     assert_eq!(x.val2, vec!["str1".to_string(), "str2".to_string(), "str3".to_string()]);
///     // default
///     assert_eq!(x.val3, 0);
///     Ok(())
/// }
///
///```
#[proc_macro_derive(Configurable, attributes(required, options))]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm)
}

// document section
mod document;
use crate::document::do_derive_document;

/// The [`crate::document()`] attribute documents functions and macros. To use it, you must add the
/// document tag to the function or macro and then you can specify any number of items within the
/// rust comment tag. The syntax mirrors that of Javadoc.
///```
/// use bmw_base::*;
/// use bmw_derive2::*;
///
/// #[document]
/// /// This appears at the top of the documentation
/// /// # Example
/// /// Anything after the example title (or anything that starts with "# Example" like
/// /// "# Examples" will end up below the parameter/return/error/and see titles
/// /// @param v1 a usize parameter
/// /// multi line ok, this goes to v1
/// /// @param v2 an option of a String
/// /// @param v3 bool value. If true exit, otherwise continue...
/// /// @return [`unit`] or error
/// /// multi line ok, goes to return tag
/// /// @error bmw_base::BaseErrorKind::IllegalState if the state becomes illegal
/// /// multi line ok, goes to the error tag
/// /// @error bmw_base::BaseErrorKind::Parse if the parser crashes
/// /// @see bmw_derive2::ErrorKind
/// /// @see bmw_derive2::Serializable
/// /// @see bmw_base::Error
/// /// @deprecated
/// fn my_function(v1: usize, v2: Option<String>, v3: bool) -> Result<(), Error> {
///     Ok(())
/// }
///```
/// These tags will result in a nicely documented function/macro. Note that macros do not support
/// the `param` tag since they do not have a known or strongly typed input parameter list.
/// # Tag types
/// * param - the parameter name with a comment
///```text
/// /// @param <param_name> <param comment>
///```
/// note: that newlines are ok and the next line following a param line, if no other tag is
/// specified will automatically be appended to the param tag on the previous line.
///
/// note: self param can be used to describe the self param in the function as well
///```text
/// /// @param timeout the amount of time, in milliseconds before function returns an error.
///```
/// * see - a link to additional items
///```text
/// /// @see <link>
///```
/// note: only the path is needed. The macro will automatically put it in mark down.
///```text
/// /// @see crate::MyTrait
///```
/// * error - an error that may occur for this function/macro
///```text
/// /// @error <link to error type> <comment>
///```
/// note: the comment is optional
///
/// note: multiple values are allowed
///```text
/// /// @error bmw_base::BaseErrorKind::IllegalState if the state becomes illegal
/// /// @error bmw_base::BaseErrorKind::Parse if a parse error occurs
///```
/// * return - a message to inlcude about the return parameter
///```text
/// /// @return <return_comment>
///```
///```text
/// /// @return the returned value is the sum of all numbers in the sequence
///```
/// * deprecated - display a warning indicating this function/macro is deprecated
///```text
/// /// @deprecated
///```
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[allow(non_snake_case)]
pub fn document(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_document(attr, item)
}
