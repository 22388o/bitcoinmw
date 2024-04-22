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
