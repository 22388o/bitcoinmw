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

/// Build the specified [`crate::ErrorKind`] and convert it into an [`crate::Error`].
/// # Input Parameters
/// * `$kind` - [`crate::ErrorKind`] - The kind of this error. Note that [`crate::ErrorKind`] is a trait
/// and any crate can implement its own kinds of errors. This crate implements the
/// [`crate::BaseErrorKind`] enum which includes many errors that are automatically converted.
/// * `$msg` - [`std::str`] - The message to display with this error.
/// * `$($param)*` - The formatting parameters as in with [`std::format`].
/// # Return
/// Err ( [`crate::Error`] ) of the coresponding [`crate::ErrorKind`] and message.
/// # Also See
/// * [`crate::Error`]
/// * [`crate::map_err`]
/// # Examples
///
///```
/// use bmw_base::{Error, err, BaseErrorKind, ErrorKind, kind};
///
/// fn main() -> Result<(), Error> {
///     let err1: Result<(), Error> = err!(
///         BaseErrorKind::Parse,
///         "unexpected token: '{}'",
///         "test"
///     );
///     let err1 = err1.unwrap_err();
///     assert_eq!(err1.kind(), &kind!(BaseErrorKind::Parse, "unexpected token: 'test'"));
///
///     Ok(())
/// }
///
///```
#[macro_export]
macro_rules! err {
	($kind:expr, $msg:expr, $($param:tt)*) => {{
                let msg = &format!($msg, $($param)*)[..];
                let error: Error = $kind(msg.to_string()).into();
                Err(error)
        }};
	($kind:expr, $msg:expr) => {{
                let error: Error = $kind($msg.to_string()).into();
                Err(error)
	}};
}

/// This macro is the same as [`crate::err`] except that it only returns the [`crate::Error`] and
/// doesn't wrap it in the [`std::result::Result::Err`] like [`crate::err`] does.
/// # Input Parameters
/// * `$kind` - [`crate::ErrorKind`] - Kind of this error. Note that [`crate::ErrorKind`] is a trait
/// and any crate can implement its own kinds of errors. This crate implements the
/// [`crate::BaseErrorKind`] enum which includes many errors that are automatically converted.
/// * `$msg` - [`std::str`] - The message to display with this error.
/// * `$($param)*` - The formatting parameters as in with [`std::format`].
/// # Return
/// [`crate::Error`] of the coresponding [`crate::ErrorKind`] and message.
/// # Also See
/// * [`crate::Error`]
/// * [`crate::map_err`]
/// # Examples
///
///```
/// use bmw_base::{Error, err_only, BaseErrorKind, ErrorKind, kind};
///
/// fn main() -> Result<(), Error> {
///     let err1: Error = err_only!(
///         BaseErrorKind::Parse,
///         "unexpected token: '{}'",
///         "test"
///     );
///     assert_eq!(err1.kind(), &kind!(BaseErrorKind::Parse, "unexpected token: 'test'"));
///
///     Ok(())
/// }
///
///```
#[macro_export]
macro_rules! err_only {
        ($kind:expr, $msg:expr, $($param:tt)*) => {{
                let msg = &format!($msg, $($param)*)[..];
                let error: Error = $kind(msg.to_string()).into();
                error
        }};
        ($kind:expr, $m:expr) => {{
                let error: Error = $kind($m.to_string()).into();
                error
        }};
}

/// Map the error into the specified [`crate::ErrorKind`].
/// Optionally specify an additional message to be included in the error.
/// # Input Parameters
/// * `$in_err` - The error that is being mapped.
/// * `$kind` - [`crate::ErrorKind`] - The kind of error to map to.
/// * `$msg` - [`std::str`] - (optional) The message to display with this error.
/// # Return
/// [`crate::Error`] with the specified $kind and $msg.
/// # Also see
/// * [`crate::err`]
/// * [`crate::err_only`]
/// # Examples
///```
/// use bmw_base::{Error, ErrorKind, map_err, BaseErrorKind, kind};
/// use std::fs::File;
/// use std::io::Write;
///
/// fn main() -> Result<(), Error> {
///     let err = map_err!("".parse::<usize>(), BaseErrorKind::Parse, "custom message: 1");
///     assert_eq!(
///         err.unwrap_err().kind(),
///         &kind!(
///             BaseErrorKind::Parse,
///             "custom message: 1: cannot parse integer from empty string"
///         )
///     );
///     Ok(())
/// }
///
///```
#[macro_export]
macro_rules! map_err {
	($in_err:expr, $kind:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}", e)).into() })
	}};
	($in_err:expr, $kind:expr, $msg:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}: {}", $msg, e)).into() })
	}};
}

/// Convenience macro to return an error kind as a `Box<dyn ErrorKind>`. This is mostly useful for
/// tests.
/// # Input Parameters
/// * `$kind` - [`crate::BaseErrorKind`] (or any other concrete type that implements the
/// [`crate::ErrorKind`] trait.
/// * `$msg` - [`std::str`] - The message to display with this error.
/// # Return
/// [`Box`] < dyn [`crate::ErrorKind`] > - The boxed version of this error kind.
/// # Also see
/// * [`crate::ErrorKind`]
/// * [`crate::err`]
#[macro_export]
macro_rules! kind {
	($kind:expr, $msg:expr) => {{
		let r: Box<dyn ErrorKind> = Box::new($kind($msg.to_string()));
		r
	}};
}

/// Macro for implementing a conditional break.
/// # Input Parameters
/// * `cond` - [`bool`] - if [`true`], execute break statement. Otherwise continue in the loop's
/// execution.
/// # Return
/// [`unit`] - nothing is returned.
/// # Examples
///```
/// use bmw_base::*;
///
/// fn main() -> Result<(), Error> {
///     let mut counter = 0;
///     loop {
///         counter += 1;
///         cbreak!(counter == 10);
///     }
///     assert_eq!(counter, 10);
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! cbreak {
	($cond:expr) => {{
		if $cond {
			break;
		}
	}};
}

/// Macro for invoking `try_into` and mapping any errors to the corresponding [`crate::ErrorKind`].
/// # Input parameters
/// * `value` - [`TryInto`] - The value from which to attempt conversion.
/// # Return
/// [`Result`] < [`TryInto`], [`crate::Error`] >
/// # Examples
///```
/// use bmw_base::*;
///
/// fn main() -> Result<(), Error> {
///     let x: u32 = try_into!(100u64)?;
///     assert_eq!(x, 100u32);
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! try_into {
	($value:expr) => {{
		use std::convert::TryInto;
		map_err!($value.try_into(), BaseErrorKind::TryInto)
	}};
}

/// Build a configuration based on the specified input parameters. Checks for duplicates and
/// returns and error if any are specified.
/// # Input Parameters
/// * `$configurable` - The name of the [`crate::Configurable`] struct to configure.
/// * `$enum_name` - The name of the enum that coresponds to this struct.
/// * `$vec` - A vector of variants from the $enum_name to configure this instance with.
/// # Return
/// An instance of the [`crate::Configurable`] struct with the specified values. Anything not
/// specified will take on the default value.
/// # Errors
/// [`crate::BaseErrorKind::Configuration`] - if a duplicate value is specified.
/// # Also see
/// [`crate::Configurable`]
/// # Examples
///```
/// use bmw_base::*;
/// use std::collections::HashSet;
///
/// // build a struct, normally the Configurable trait would be derived via proc macro
/// // usize, bool, u8, u16, u32, u64, u128, String and (String, String) are supported types
/// // Vec of any of those types are also supported
/// struct MyStruct {
///     threads: usize,
///     timeout: usize,
/// }
///
/// // implement the Default trait
/// impl Default for MyStruct {
///     fn default() -> Self {
///         Self {
///             threads: 1,
///             timeout: 0,
///         }
///     }
/// }
///
/// // auto generated by macro
/// enum MyStruct_Options {
///     Threads(usize),
///     Timeout(usize),
/// }
///
/// // auto generated by macro
/// impl MyStruct {
///     fn new() -> Self {
///         Self::default()
///     }
///     fn required() -> Vec<String> { vec![] }
/// }
///
/// // auto generated by macro
/// impl Configurable for MyStruct {
///     fn set_usize(&mut self, name: &str, value: usize) {
///         if name == "Threads" {
///             self.threads = value;
///         }
///         if name == "Timeout" {
///             self.timeout = value;
///         }
///     }
///
///     fn set_bool(&mut self, name: &str, value: bool) {}
///     fn set_u8(&mut self, name: &str, value: u8) {}
///     fn set_u16(&mut self, name: &str, value: u16) {}
///     fn set_u32(&mut self, name: &str, value: u32) {}
///     fn set_u64(&mut self, name: &str, value: u64) {}
///     fn set_u128(&mut self, name: &str, value: u128) {}
///     fn set_string(&mut self, name: &str, value: String) {}
///     fn set_string_tuple(&mut self, name: &str, value: (String, String)) {}
///     fn allow_dupes(&self) -> HashSet<String> { HashSet::new() }
/// }
///
/// // auto generated by macro
/// impl MyStruct_Options {
///     fn name(&self) -> &str {
///         match self {
///              MyStruct_Options::Threads(v) => "Threads",
///              MyStruct_Options::Timeout(v) => "Timeout"
///         }
///     }
///     fn value_usize(&self) -> Option<usize> {
///         match self {
///             MyStruct_Options::Threads(v) => Some(*v),
///             MyStruct_Options::Timeout(v) => Some(*v),
///             _ => None,
///         }
///     }
///     fn value_bool(&self) -> Option<bool> { None }
///     fn value_u8(&self) -> Option<u8> { None }
///     fn value_u16(&self) -> Option<u16> { None }
///     fn value_u32(&self) -> Option<u32> { None }
///     fn value_u64(&self) -> Option<u64> { None }
///     fn value_u128(&self) -> Option<u128> { None }
///     fn value_string(&self) -> Option<String> { None }
///     fn value_string_tuple(&self) -> Option<(String, String)> {
///         None
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     // call configure with the specifed values. The values are changed from snake case
///     // to `Pascal` format.
///     let x = configure!(MyStruct, MyStruct_Options, vec![Threads(4), Timeout(100)])?;
///
///     // if a value is not specified, the default is used. In this case both values
///     // were set in the macro call.
///     assert_eq!(x.threads, 4);
///     assert_eq!(x.timeout, 100);
///
///     let x = configure!(MyStruct, MyStruct_Options, vec![Threads(10)])?;
///
///     assert_eq!(x.threads, 10);
///     assert_eq!(x.timeout, 0);
///
///     // Threads is specified twice so it's an error
///     assert!(configure!(MyStruct, MyStruct_Options, vec![Threads(10), Threads(20)]).is_err());
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! configure {
	( $configurable:ident, $enum_name:ident, $vec:expr ) => {{
		use bmw_base::*;
		use std::collections::HashSet;
		use $enum_name::*;

		let mut ret = $configurable::new();

		let mut name_set: HashSet<String> = HashSet::new();
		let mut err = None;
		let options: Vec<$enum_name> = $vec;

		for cfg in options {
			let name = cfg.name();
			if name_set.contains(name.clone()) && !ret.allow_dupes().contains(name.clone()) {
				let text = format!("config option ({}) was specified more than once", name);
				err = Some(err!(BaseErrorKind::Configuration, text));
			}
			name_set.insert(name.to_string());
			match cfg.value_u8() {
				Some(value) => ret.set_u8(name, value),
				None => {}
			}
			match cfg.value_u16() {
				Some(value) => ret.set_u16(name, value),
				None => {}
			}
			match cfg.value_u32() {
				Some(value) => ret.set_u32(name, value),
				None => {}
			}
			match cfg.value_u64() {
				Some(value) => ret.set_u64(name, value),
				None => {}
			}
			match cfg.value_u128() {
				Some(value) => ret.set_u128(name, value),
				None => {}
			}
			match cfg.value_usize() {
				Some(value) => ret.set_usize(name, value),
				None => {}
			}
			match cfg.value_string() {
				Some(value) => ret.set_string(name, value),
				None => {}
			}
			match cfg.value_bool() {
				Some(value) => ret.set_bool(name, value),
				None => {}
			}
			match cfg.value_string_tuple() {
				Some(value) => ret.set_string_tuple(name, value),
				None => {}
			}
		}

		for r in $configurable::required() {
			if !name_set.contains(&r) {
				let text = format!("required option ({}) was not specified", r);
				err = Some(err!(BaseErrorKind::Configuration, text));
			}
		}

		match err {
			Some(e) => e,
			None => Ok(ret),
		}
	}};
}
