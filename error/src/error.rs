// Copyright (c) 2023, The BitcoinMW Developers
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

use bmw_deps::failure::{Backtrace, Context, Fail};
use bmw_deps::rustls::client::InvalidDnsNameError;
use bmw_deps::rustls::sign::SignError;
use std::alloc::LayoutError;
use std::array::TryFromSliceError;
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

/// Base Error struct which is used throughout bmw.
#[derive(Debug, Fail)]
pub struct Error {
	inner: Context<ErrorKind>,
}

impl PartialEq for Error {
	fn eq(&self, r: &Error) -> bool {
		r.kind() == self.kind()
	}
}

/// Kinds of errors that can occur.
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
	/// IO Error
	#[fail(display = "IO Error: {}", _0)]
	IO(String),
	/// Log Error
	#[fail(display = "Log Error: {}", _0)]
	Log(String),
	/// UTF8 Error
	#[fail(display = "UTF8 Error: {}", _0)]
	Utf8(String),
	/// ArrayIndexOutOfBounds
	#[fail(display = "ArrayIndexOutofBounds: {}", _0)]
	ArrayIndexOutOfBounds(String),
	/// Configuration Error
	#[fail(display = "Configuration Error: {}", _0)]
	Configuration(String),
	/// Poison error multiple locks
	#[fail(display = "Poison Error: {}", _0)]
	Poison(String),
	/// CorruptedData
	#[fail(display = "Corrupted Data Error: {}", _0)]
	CorruptedData(String),
	/// Timeout
	#[fail(display = "Timeout: {}", _0)]
	Timeout(String),
	/// Capacity Exceeded
	#[fail(display = "Capacity Exceeded: {}", _0)]
	CapacityExceeded(String),
	/// UnexpectedEof Error
	#[fail(display = "UnexpectedEOF: {}", _0)]
	UnexpectedEof(String),
	/// IllegalArgument
	#[fail(display = "IllegalArgument: {}", _0)]
	IllegalArgument(String),
	/// Miscellaneous Error
	#[fail(display = "Miscellaneous Error: {}", _0)]
	Misc(String),
	/// Illegal State
	#[fail(display = "Illegal State Error: {}", _0)]
	IllegalState(String),
	/// Simulated Error used in testing
	#[fail(display = "simulated test error: {}", _0)]
	Test(String),
	/// Overflow error
	#[fail(display = "overflow error: {}", _0)]
	Overflow(String),
	/// Thread Panic
	#[fail(display = "thread panic: {}", _0)]
	ThreadPanic(String),
	/// Memmory Allocation Error
	#[fail(display = "memory allocation error: {}", _0)]
	Alloc(String),
	/// Operation not supported
	#[fail(display = "operation not supported error: {}", _0)]
	OperationNotSupported(String),
	/// system time error
	#[fail(display = "system time error: {}", _0)]
	SystemTime(String),
	/// Errno system error
	#[fail(display = "errno error: {}", _0)]
	Errno(String),
	/// Rustls Error
	#[fail(display = "rustls error: {}", _0)]
	Rustls(String),
	/// BMW Crypt Error
	#[fail(display = "bmw_crypt error: {}", _0)]
	Crypt(String),
	#[fail(display = "http_error: {}", _0)]
	Http(String),
}

/// The names of ErrorKinds in this crate. This enum is used to map to error
/// names using the [`crate::err`] and [`crate::map_err`] macros.
pub enum ErrKind {
	/// IO Error
	IO,
	/// Log Error
	Log,
	/// A conversion to the utf8 format resulted in an error
	Utf8,
	/// An array index was out of bounds
	ArrayIndexOutOfBounds,
	/// Configuration error
	Configuration,
	/// Attempt to obtain a lock resulted in a poison error. See [`std::sync::PoisonError`]
	/// for further details
	Poison,
	/// Data is corrupted
	CorruptedData,
	/// A timeout has occurred
	Timeout,
	/// The capacity is exceeded
	CapacityExceeded,
	/// Unexpected end of file
	UnexpectedEof,
	/// Illegal argument was specified
	IllegalArgument,
	/// A Miscellaneous Error occurred
	Misc,
	/// Application is in an illegal state
	IllegalState,
	/// Overflow error
	Overflow,
	/// A simulated error used in tests
	Test,
	/// Thread panic
	ThreadPanic,
	/// Memory allocation error
	Alloc,
	/// Operation not supported
	OperationNotSupported,
	/// System time error
	SystemTime,
	/// Errno system error
	Errno,
	/// Rustls error
	Rustls,
	/// Crypt error
	Crypt,
	/// Http error
	Http,
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

// infallible cannot happen
#[cfg(not(tarpaulin_include))]
impl From<Infallible> for Error {
	fn from(e: Infallible) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("Infallible: {}", e))),
		}
	}
}

#[cfg(unix)]
impl From<bmw_deps::nix::errno::Errno> for Error {
	fn from(e: bmw_deps::nix::errno::Errno) -> Error {
		Error {
			inner: Context::new(ErrorKind::Errno(format!("Errno system error: {}", e))),
		}
	}
}

impl From<bmw_deps::rustls::Error> for Error {
	fn from(e: bmw_deps::rustls::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::Rustls(format!("Rustls error: {}", e))),
		}
	}
}

impl From<SignError> for Error {
	fn from(e: SignError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Rustls(format!("Rustls Signing error: {}", e))),
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

impl From<TryFromSliceError> for Error {
	fn from(e: TryFromSliceError) -> Error {
		Error {
			inner: Context::new(ErrorKind::Misc(format!("tryfromsliceerror: {}", e))),
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

#[cfg(test)]
mod test {
	#![allow(invalid_from_utf8)]
	use crate as bmw_err;
	use crate::{err, ErrKind, Error, ErrorKind};
	use bmw_deps::rustls::client::InvalidDnsNameError;
	use bmw_deps::rustls::sign::{any_supported_type, SignError, SigningKey};
	use bmw_deps::rustls::{PrivateKey, ServerConfig, ServerName, ALL_CIPHER_SUITES};
	use bmw_deps::rustls_pemfile::{read_one, Item};
	use bmw_deps::substring::Substring;
	use std::alloc::Layout;
	use std::convert::TryInto;
	use std::ffi::OsString;
	use std::fs::File;
	use std::io::BufReader;
	use std::sync::mpsc::channel;
	use std::sync::{Arc, Mutex, RwLock};
	use std::time::{Duration, SystemTime, SystemTimeError};

	#[cfg(target_os = "linux")]
	use bmw_deps::nix::errno::Errno;
	#[cfg(target_os = "linux")]
	use bmw_deps::nix::sys::epoll::{epoll_ctl, EpollEvent, EpollFlags, EpollOp};

	fn get_os_string() -> Result<(), Error> {
		Err(OsString::new().into())
	}

	fn check_error<T: Sized, Q>(r: Result<T, Q>, ematch: Error) -> Result<(), Error>
	where
		crate::Error: From<Q>,
	{
		if let Err(r) = r {
			let e: Error = r.into();

			// Some errors are slightly different on different platforms. So, we check
			// the first 10 characters which is specified in the ErrorKind generally.
			assert_eq!(
				e.to_string().substring(0, 10),
				ematch.to_string().substring(0, 10)
			);
			assert_eq!(
				e.kind().to_string().substring(0, 10),
				ematch.to_string().substring(0, 10)
			);
			assert!(e.cause().is_none());
			assert!(e.backtrace().is_some());
			assert_eq!(
				e.inner().substring(0, 10),
				ematch.to_string().substring(0, 10),
			);
			println!("e.backtrace()={:?}", e.backtrace());
			println!("e={}", e);
		}
		Ok(())
	}

	fn get_utf8() -> Result<String, Error> {
		Ok(std::str::from_utf8(&[0xC0])?.to_string())
	}

	#[test]
	fn test_errors() -> Result<(), Error> {
		check_error(
			std::fs::File::open("/no/path/here"),
			ErrorKind::IO("No such file or directory (os error 2)".to_string()).into(),
		)?;

		check_error(get_os_string(), ErrorKind::Misc("".to_string()).into())?;

		let x: Result<u32, _> = u64::MAX.try_into();
		check_error(x, ErrorKind::Misc(format!("TryFromIntError..")).into())?;

		let x: Result<u32, _> = "abc".parse();
		check_error(x, ErrorKind::Misc(format!("ParseIntError..")).into())?;
		check_error(get_utf8(), ErrorKind::Utf8(format!("Utf8 Error..")).into())?;

		Ok(())
	}

	fn load_private_key(filename: &str) -> Result<PrivateKey, Error> {
		let keyfile = File::open(filename)?;
		let mut reader = BufReader::new(keyfile);

		let x = read_one(&mut reader)?.unwrap();
		match x {
			Item::PKCS8Key(key) => Ok(PrivateKey(key)),
			_ => todo!(),
		}
	}

	#[test]
	fn test_other_errors() -> Result<(), Error> {
		let mutex = Arc::new(Mutex::new(0));
		let mutex_clone = mutex.clone();
		let lock = Arc::new(RwLock::new(0));
		let lock_clone = lock.clone();
		let _ = std::thread::spawn(move || -> Result<u32, Error> {
			let _mutex = mutex_clone.lock();
			let _x = lock.write();
			let y: Option<u32> = None;
			Ok(y.unwrap())
		})
		.join();

		check_error(
			lock_clone.write(),
			ErrorKind::Poison(format!("Poison..")).into(),
		)?;

		check_error(
			lock_clone.read(),
			ErrorKind::Poison(format!("Poison..")).into(),
		)?;

		check_error(mutex.lock(), ErrorKind::Poison(format!("Poison..")).into())?;

		let x = err!(ErrKind::Poison, "");
		let y = err!(ErrKind::IllegalArgument, "");
		let z = err!(ErrKind::Poison, "");

		assert_ne!(x, y);
		assert_eq!(x, z);

		let (tx, rx) = channel();

		std::thread::spawn(move || -> Result<(), Error> {
			tx.send(1)?;
			Ok(())
		});

		assert!(rx.recv().is_ok());
		let err = rx.recv();
		assert!(err.is_err());
		check_error(
			err,
			ErrorKind::IllegalState(format!("IllegalState..")).into(),
		)?;
		let tx = {
			let (tx, _rx) = channel();
			tx
		};

		let err = tx.send(1);
		check_error(
			err,
			ErrorKind::IllegalState(format!("IllegalState..")).into(),
		)?;

		let err = Layout::from_size_align(7, 7);
		check_error(err, ErrorKind::Alloc(format!("LayoutError..")).into())?;

		let now = SystemTime::now();
		let err: Result<Duration, SystemTimeError> = now
			.checked_add(Duration::from_millis(1_000_000))
			.unwrap()
			.duration_since(now.checked_add(Duration::from_millis(2_000_000)).unwrap());
		check_error(
			err,
			ErrorKind::SystemTime("System time error".into()).into(),
		)?;

		#[cfg(target_os = "linux")]
		{
			let mut event = EpollEvent::new(EpollFlags::empty(), u64::MAX);
			let err: Result<(), Errno> =
				epoll_ctl(i32::MAX, EpollOp::EpollCtlAdd, i32::MAX, &mut event);
			check_error(err, ErrorKind::Errno("Errno error: ".to_string()).into())?;
		}

		let err = ServerConfig::builder()
			.with_cipher_suites(&ALL_CIPHER_SUITES.to_vec())
			.with_safe_default_kx_groups()
			.with_protocol_versions(&vec![]);

		check_error(err, ErrorKind::Rustls("rustls error: ".to_string()).into())?;

		let err: Result<ServerName, InvalidDnsNameError> = "a*$&@@!aa".try_into();
		assert!(err.is_err());
		check_error(err, ErrorKind::Rustls("rustls error: ".to_string()).into())?;

		let err: Result<Arc<dyn SigningKey>, SignError> =
			any_supported_type(&load_private_key("./resources/badkey.pem")?);
		assert!(err.is_err());
		check_error(err, ErrorKind::Rustls("rustls error: ".to_string()).into())?;
		Ok(())
	}
}
