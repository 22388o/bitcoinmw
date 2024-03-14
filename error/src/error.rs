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

use crate::types::{Error, ErrorKind};
use bmw_deps::failure::{Backtrace, Context, Fail};
use bmw_deps::rustls::pki_types::InvalidDnsNameError;
use bmw_deps::rustls::Error as RustlsError;
use bmw_deps::url::ParseError;
use bmw_deps::webpki::Error as WebpkiError;
use std::alloc::LayoutError;
use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt::{Display, Formatter, Result};
use std::net::AddrParseError;
use std::num::{ParseIntError, TryFromIntError};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::mpsc::{RecvError, SendError};
use std::sync::MutexGuard;
use std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTimeError;

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
	/// get the kind of error that occurred.
	pub fn kind(&self) -> ErrorKind {
		self.inner.get_context().clone()
	}

	/// get the cause (if available) of this error.
	pub fn cause(&self) -> Option<&dyn Fail> {
		self.inner.cause()
	}

	/// get the backtrace (if available) of this error.
	pub fn backtrace(&self) -> Option<&Backtrace> {
		self.inner.backtrace()
	}

	/// get the inner error as a string.
	pub fn inner(&self) -> String {
		self.inner.to_string()
	}
}

impl From<ErrorKind> for Error {
	fn from(kind: ErrorKind) -> Error {
		Error {
			inner: Context::new(kind),
		}
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::IO(format!("{}", e))),
		}
	}
}

impl From<InvalidDnsNameError> for Error {
	fn from(e: InvalidDnsNameError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Rustls(format!(
				"Rustls Invalid DnsNameError: {}",
				e
			))),
		}
	}
}

impl From<ParseError> for Error {
	fn from(e: ParseError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("url::ParseError: {}", e))),
		}
	}
}

impl From<OsString> for Error {
	fn from(e: OsString) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("{:?}", e))),
		}
	}
}

impl From<TryFromIntError> for Error {
	fn from(e: TryFromIntError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("TryFromIntError: {}", e))),
		}
	}
}

impl From<WebpkiError> for Error {
	fn from(e: WebpkiError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("webpkiError: {}", e))),
		}
	}
}

impl From<ParseIntError> for Error {
	fn from(e: ParseIntError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("ParseIntError: {}", e))),
		}
	}
}

impl From<Utf8Error> for Error {
	fn from(e: Utf8Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::Utf8(format!("Utf8 error: {}", e))),
		}
	}
}

impl<T> From<PoisonError<RwLockWriteGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockWriteGuard<'_, T>>) -> Error {
		Error {
			inner: Context::new(ErrorKind::Poison(format!("Poison error: {}", e))),
		}
	}
}

impl<T> From<PoisonError<RwLockReadGuard<'_, T>>> for Error {
	fn from(e: PoisonError<RwLockReadGuard<'_, T>>) -> Error {
		Error {
			inner: Context::new(ErrorKind::Poison(format!("Poison error: {}", e))),
		}
	}
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for Error {
	fn from(e: PoisonError<MutexGuard<'_, T>>) -> Error {
		Error {
			inner: Context::new(ErrorKind::Poison(format!("Poison error: {}", e))),
		}
	}
}

impl From<RecvError> for Error {
	fn from(e: RecvError) -> Error {
		Error {
			inner: Context::new(ErrorKind::IllegalState(format!("Recv error: {}", e))),
		}
	}
}

impl<T> From<SendError<T>> for Error {
	fn from(e: SendError<T>) -> Error {
		Error {
			inner: Context::new(ErrorKind::IllegalState(format!("Send error: {}", e))),
		}
	}
}

impl From<LayoutError> for Error {
	fn from(e: LayoutError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Alloc(format!("Layout error: {}", e))),
		}
	}
}

impl From<SystemTimeError> for Error {
	fn from(e: SystemTimeError) -> Error {
		Error {
			inner: Context::new(ErrorKind::SystemTime(format!("System Time error: {}", e))),
		}
	}
}

#[cfg(not(tarpaulin_include))] // can't happen
impl From<Infallible> for Error {
	fn from(e: Infallible) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("Infallible: {}", e))),
		}
	}
}

#[cfg(unix)]
impl From<NixErrno> for Error {
	fn from(e: NixErrno) -> Error {
		Error {
			inner: Context::new(ErrorKind::Errno(format!("Errno system error: {}", e))),
		}
	}
}

impl From<RustlsError> for Error {
	fn from(e: RustlsError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Rustls(format!("Rustls error: {}", e))),
		}
	}
}

impl From<FromUtf8Error> for Error {
	fn from(e: FromUtf8Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("utf8 error: {}", e))),
		}
	}
}

impl From<AddrParseError> for Error {
	fn from(e: AddrParseError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("addr parse error: {}", e))),
		}
	}
}
