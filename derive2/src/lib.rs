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
use crate::ser::do_derive_serialize;

#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serialize(strm: TokenStream) -> TokenStream {
	do_derive_serialize(strm)
}
