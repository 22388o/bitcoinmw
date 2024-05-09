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

use crate::err_only;
use bmw_deps::backtrace::Backtrace;
use std::env::VarError;
use std::ffi::OsString;
use std::fmt::{Debug, Display, Formatter};
use std::str::Utf8Error;
use std::sync::mpsc::{RecvError, SendError};
use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use CoreErrorKind::*;

pub trait ErrorKind: Send + Sync + Display + Debug {}

pub struct Error {
	kind: Box<dyn ErrorKind>,
}

pub enum CoreErrorKind {
	/// illegal argument
	IllegalArgument(String),
	/// illegal state
	IllegalState(String),
	/// array index out of bounds
	ArrayIndexOutOfBounds(String),
	/// poison
	Poison(String),
	/// parse
	Parse(String),
	/// configuration
	Configuration(String),
	/// class builder
	Builder(String),
	/// var error
	Var(String),
	/// IO error
	IO(String),
	/// TryInto error
	TryInto(String),
	/// OsString error
	OsString(String),
	/// Utf8 error
	Utf8(String),
}

impl Error {
	pub fn new(kind: Box<dyn ErrorKind>) -> Self {
		Self { kind }
	}
}

macro_rules! impl_debug {
	($self:expr, $f:expr, $variant_name:ident, $type_str:expr) => {
		match $self {
			$variant_name(s) => {
				write!($f, "{}: {}", $type_str, s)?;
			}
			_ => {}
		}
	};
}

impl Debug for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{}\n", self.kind)?;
		match std::env::var("RUST_BACKTRACE") {
			Ok(_) => {
				write!(f, "backtrace: {:?}", Backtrace::new())?;
			}
			Err(_e) => {
				write!(f, "Backtrace disabled. For backtrace set RUST_BACKTRACE enviornment variable to 1.")?;
			}
		}

		Ok(())
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{:?}", self)
	}
}

impl ErrorKind for CoreErrorKind {}

impl Display for CoreErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{:?}", self)
	}
}

impl Debug for CoreErrorKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		impl_debug!(self, f, IllegalArgument, "illegal argument");
		impl_debug!(self, f, IllegalState, "illegal state err");
		impl_debug!(self, f, ArrayIndexOutOfBounds, "array index out of bounds");
		impl_debug!(self, f, Poison, "poison");
		impl_debug!(self, f, Parse, "parse");
		impl_debug!(self, f, Configuration, "configuration");
		Ok(())
	}
}

impl From<Box<dyn ErrorKind>> for Error {
	fn from(kind: Box<dyn ErrorKind>) -> Error {
		Error::new(kind)
	}
}

impl From<CoreErrorKind> for Error {
	fn from(kind: CoreErrorKind) -> Error {
		Error::new(Box::new(kind))
	}
}

impl Error {
	pub fn kind(&self) -> &Box<dyn ErrorKind> {
		&self.kind
	}
}

impl PartialEq for Error {
	fn eq(&self, other_err: &Error) -> bool {
		self.kind().to_string() == other_err.kind().to_string()
	}
}

impl PartialEq for Box<dyn ErrorKind> {
	fn eq(&self, other_error: &Box<(dyn ErrorKind)>) -> bool {
		let self_string = self.to_string();
		let other_string = other_error.to_string();
		match self_string.find(":") {
			Some(pos1) => match other_string.find(":") {
				Some(pos2) => &self_string.as_bytes()[0..pos1] == &other_string.as_bytes()[0..pos2],
				None => false,
			},
			None => false,
		}
	}
}

impl From<VarError> for Error {
	fn from(e: VarError) -> Error {
		err_only!(Var, e)
	}
}

impl<T> From<PoisonError<RwLockWriteGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockWriteGuard<'_, T>>) -> Error {
		err_only!(Poison, e)
	}
}

impl<T> From<PoisonError<RwLockReadGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockReadGuard<'_, T>>) -> Error {
		err_only!(Poison, e)
	}
}

impl From<Utf8Error> for Error {
	fn from(e: Utf8Error) -> Error {
		err_only!(Utf8, e)
	}
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for Error {
	fn from(e: PoisonError<MutexGuard<'_, T>>) -> Error {
		err_only!(Poison, e)
	}
}

impl From<RecvError> for Error {
	fn from(e: RecvError) -> Error {
		err_only!(IllegalState, e)
	}
}

impl From<OsString> for Error {
	fn from(e: OsString) -> Error {
		err_only!(OsString, format!("{:?}", e))
	}
}

impl<T> From<SendError<T>> for Error {
	fn from(e: SendError<T>) -> Error {
		err_only!(IllegalState, e)
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Error {
		err_only!(IO, e)
	}
}

impl PartialEq<Box<dyn ErrorKind>> for &Box<dyn ErrorKind> {
	fn eq(&self, other: &Box<dyn ErrorKind>) -> bool {
		*self == other
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::err;

	fn ret_err() -> Result<(), Error> {
		err!(IllegalArgument, "the argument was not legal")
	}

	fn ret_err2() -> Result<(), Error> {
		err!(IllegalArgument, "the argument was not legal2")
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		let err1 = ret_err().unwrap_err();
		println!("err={}", err1);
		let err2 = ret_err().unwrap_err();
		assert_eq!(err1, err2);

		let err2 = ret_err2().unwrap_err();
		assert_ne!(err1, err2);

		assert_eq!(err1.kind(), err2.kind());

		Ok(())
	}
}
