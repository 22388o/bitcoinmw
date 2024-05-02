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

#[cfg(unix)]
use bmw_deps::nix::errno::Errno as NixErrno;

use crate::{err_only, CoreErrorKind, Error, ErrorKind};
use bmw_deps::failure::{Backtrace, Context, Fail};
use bmw_deps::url::ParseError;
use std::alloc::LayoutError;
use std::collections::TryReserveError;
use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt::{Display, Formatter, Result};
use std::net::AddrParseError;
use std::num::{ParseIntError, TryFromIntError};
use std::str::{ParseBoolError, Utf8Error};
use std::string::FromUtf8Error;
use std::sync::mpsc::{RecvError, SendError};
use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTimeError;

// commpare errors by "kind" only
impl PartialEq for Error {
	fn eq(&self, r: &Error) -> bool {
		r.kind() == self.kind()
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let output = format!("{} \n Backtrace: {:?}", self.inner, self.backtrace());
		Display::fmt(&output, f)
	}
}

impl Error {
	/// create an error from the specified error kind. This should be called through the
	/// [`crate::err`] macro.
	/// # Input Parameters
	/// * `kind` - [`Box`] <dyn [`crate::ErrorKind`] > - creates an error with the specified
	/// error kind.
	/// # Return
	/// An instance of [`crate::Error`] is returned.
	/// # Also see
	/// * [`crate::Error::kind`]
	/// * [`crate::err`]
	pub fn new(kind: Box<dyn ErrorKind>) -> Self {
		Self {
			inner: Context::new(kind),
		}
	}

	/// get the kind of error that occurred.
	/// # Input Parameters
	/// * `self` - `&self`
	/// # Return
	/// An immutable reference to this error's [`crate::ErrorKind`].
	pub fn kind(&self) -> &Box<dyn ErrorKind> {
		self.inner.get_context()
	}

	/// get the cause (if available) of this error.
	/// # Input Parameters
	/// * `self` - `&self`
	/// # Return
	/// [`std::option::Option`] < & dyn [`Fail`] > - The cause of the error, if it can be
	/// returned.
	/// # Also see
	/// * [`crate::ErrorKind`]
	/// * [`crate::Error::backtrace`]
	pub fn cause(&self) -> Option<&dyn Fail> {
		self.inner.cause()
	}

	/// get the backtrace (if available) of this error.
	/// # Input Parameters
	/// * `self` - `&self`
	/// # Return
	/// [`std::option::Option`] < [`Backtrace`] > - The backtrace for this error, if it can be
	/// returned.
	/// * [`crate::ErrorKind`]
	/// * [`crate::Error::cause`]
	pub fn backtrace(&self) -> Option<&Backtrace> {
		self.inner.backtrace()
	}

	/// get the inner error as a string.
	/// # Input Parameters
	/// `self` - `&self`
	/// # Return
	/// [`std::string::String`] - The error as a string.
	/// # Also see
	/// * [`crate::ErrorKind`]
	/// * [`crate::Error::backtrace`]
	pub fn inner(&self) -> String {
		self.inner.to_string()
	}
}

// do conversions of some common errors

impl From<Box<dyn ErrorKind>> for Error {
	fn from(kind: Box<dyn ErrorKind>) -> Error {
		Error::new(kind)
	}
}

impl PartialEq for Box<dyn ErrorKind> {
	fn eq(&self, cmp: &Box<(dyn ErrorKind + 'static)>) -> bool {
		cmp.to_string() == self.to_string()
	}
}

impl ErrorKind for CoreErrorKind {}

impl From<CoreErrorKind> for Error {
	fn from(kind: CoreErrorKind) -> Error {
		Error::new(Box::new(kind))
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Error {
		err_only!(CoreErrorKind::IO, e)
	}
}

impl From<ParseError> for Error {
	fn from(e: ParseError) -> Error {
		err_only!(CoreErrorKind::Parse, e)
	}
}

impl From<OsString> for Error {
	fn from(e: OsString) -> Error {
		err_only!(CoreErrorKind::OsString, format!("{:?}", e))
	}
}

impl From<TryFromIntError> for Error {
	fn from(e: TryFromIntError) -> Error {
		err_only!(CoreErrorKind::TryFrom, e)
	}
}

impl From<ParseIntError> for Error {
	fn from(e: ParseIntError) -> Error {
		err_only!(CoreErrorKind::Parse, e)
	}
}

impl From<Utf8Error> for Error {
	fn from(e: Utf8Error) -> Error {
		err_only!(CoreErrorKind::Utf8, e)
	}
}

impl<T> From<PoisonError<RwLockWriteGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockWriteGuard<'_, T>>) -> Error {
		err_only!(CoreErrorKind::Poison, e)
	}
}

impl<T> From<PoisonError<RwLockReadGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockReadGuard<'_, T>>) -> Error {
		err_only!(CoreErrorKind::Poison, e)
	}
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for Error {
	fn from(e: PoisonError<MutexGuard<'_, T>>) -> Error {
		err_only!(CoreErrorKind::Poison, e)
	}
}

impl From<RecvError> for Error {
	fn from(e: RecvError) -> Error {
		err_only!(CoreErrorKind::IllegalState, e)
	}
}

impl<T> From<SendError<T>> for Error {
	fn from(e: SendError<T>) -> Error {
		err_only!(CoreErrorKind::IllegalState, e)
	}
}

impl From<LayoutError> for Error {
	fn from(e: LayoutError) -> Error {
		err_only!(CoreErrorKind::Alloc, format!("layout error: {}", e))
	}
}

impl From<SystemTimeError> for Error {
	fn from(e: SystemTimeError) -> Error {
		err_only!(CoreErrorKind::SystemTime, e)
	}
}

#[cfg(not(tarpaulin_include))] // can't happen
impl From<Infallible> for Error {
	fn from(e: Infallible) -> Error {
		err_only!(CoreErrorKind::Misc, e)
	}
}

#[cfg(unix)]
impl From<NixErrno> for Error {
	fn from(e: NixErrno) -> Error {
		err_only!(CoreErrorKind::Errno, e)
	}
}

impl From<FromUtf8Error> for Error {
	fn from(e: FromUtf8Error) -> Error {
		err_only!(CoreErrorKind::Utf8, e)
	}
}

impl From<AddrParseError> for Error {
	fn from(e: AddrParseError) -> Error {
		err_only!(CoreErrorKind::Parse, e)
	}
}

impl From<ParseBoolError> for Error {
	fn from(e: ParseBoolError) -> Error {
		err_only!(CoreErrorKind::Parse, e)
	}
}

impl From<TryReserveError> for Error {
	fn from(e: TryReserveError) -> Error {
		err_only!(CoreErrorKind::OOM, e)
	}
}
