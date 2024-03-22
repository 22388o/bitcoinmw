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

/// Macro to map the try_from error into an appropriate error.
#[macro_export]
macro_rules! try_into {
	($v:expr) => {{
		use bmw_err::{map_err, ErrKind};
		use std::convert::TryInto;
		map_err!($v.try_into(), ErrKind::Misc, "TryInto Error")
	}};
}

/// Build the specified [`crate::ErrorKind`] and convert it into an [`crate::Error`]. The desired
/// [`crate::ErrorKind`] is specified using the [`crate::ErrKind`] name enum.
///
/// Example:
///
///```
/// use bmw_err::{Error, ErrorKind, ErrKind, err};
///
/// fn show_err_kind(do_error: bool) -> Result<(), Error> {
///     let e = err!(ErrKind::Configuration, "invalid parameter name");
///
///     if do_error {
///         return Err(e);
///     }
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! err {
	($kind:expr, $msg:expr, $($param:tt)*) => {{
                use bmw_err::err;
                let msg = &format!($msg, $($param)*)[..];
                err!($kind, msg)
        }};
	($kind:expr, $msg:expr) => {{
            use bmw_err::{ErrKind, ErrorKind, Error};
		match $kind {
			ErrKind::Configuration => {
				let error: Error = ErrorKind::Configuration($msg.to_string()).into();
				error
			}
			ErrKind::IO => {
				let error: Error = ErrorKind::IO($msg.to_string()).into();
				error
			}
			ErrKind::Log => {
				let error: Error = ErrorKind::Log($msg.to_string()).into();
				error
			}
			ErrKind::Utf8 => {
				let error: Error = ErrorKind::Utf8($msg.to_string()).into();
				error
			}
			ErrKind::ArrayIndexOutOfBounds => {
				let error: Error = ErrorKind::ArrayIndexOutOfBounds($msg.to_string()).into();
				error
			}
			ErrKind::Poison => {
				let error: Error = ErrorKind::Poison($msg.to_string()).into();
				error
			}
			ErrKind::CorruptedData => {
				let error: Error =
					ErrorKind::CorruptedData($msg.to_string()).into();
				error
			}
			ErrKind::Timeout => {
				let error: Error = ErrorKind::Timeout($msg.to_string()).into();
				error
			}
			ErrKind::CapacityExceeded => {
				let error: Error =
					ErrorKind::CapacityExceeded($msg.to_string()).into();
				error
			}
			ErrKind::UnexpectedEof => {
				let error: Error =
					ErrorKind::UnexpectedEof($msg.to_string()).into();
				error
			}
			ErrKind::IllegalArgument => {
				let error: Error = ErrorKind::IllegalArgument($msg.to_string()).into();
                                error
			}
			ErrKind::Misc => {
				let error: Error = ErrorKind::Misc($msg.to_string()).into();
				error
			}
			ErrKind::IllegalState => {
				let error: Error = ErrorKind::IllegalState($msg.to_string()).into();
				error
			}
			ErrKind::Test => {
				let error: Error = ErrorKind::Test($msg.to_string()).into();
				error
			}
			ErrKind::Overflow => {
				let error: Error = ErrorKind::Overflow($msg.to_string()).into();
				error
			}
			ErrKind::ThreadPanic => {
				let error: Error = ErrorKind::ThreadPanic($msg.to_string()).into();
				error
			}
			ErrKind::Alloc => {
				let error: Error = ErrorKind::Alloc($msg.to_string()).into();
				error
			}
			ErrKind::OperationNotSupported => {
				let error: Error = ErrorKind::OperationNotSupported($msg.to_string()).into();
				error
			}
			ErrKind::SystemTime => {
				let error: Error = ErrorKind::SystemTime($msg.to_string()).into();
				error
			}
			ErrKind::Errno => {
				let error: Error = ErrorKind::Errno($msg.to_string()).into();
				error
			}
			ErrKind::Rustls => {
				let error: Error = ErrorKind::Rustls($msg.to_string()).into();
				error
			}
			ErrKind::Crypt => {
				let error: Error = ErrorKind::Crypt($msg.to_string()).into();
				error
			}
			ErrKind::Http => {
				let error: Error = ErrorKind::Http($msg.to_string()).into();
				error
			}
			ErrKind::Rustlet => {
				let error: Error = ErrorKind::Rustlet($msg.to_string()).into();
				error
			}
		}
	}};
}

/// Map the specified error into the [`crate::ErrKind`] enum name from this crate.
/// Optionally specify an additional message to be included in the error.
///
/// Example:
///
///```
/// use bmw_err::{Error, ErrorKind, ErrKind, map_err};
/// use std::fs::File;
/// use std::io::Write;
///
/// fn show_map_err(do_error: bool) -> Result<(), Error> {
///     let file = map_err!(File::open("/path/to/something"), ErrKind::IO, "file open failed")?;
///     println!("file_type={:?}", file.metadata()?.file_type());
///
///     let mut x = map_err!(File::open("/invalid/log/path.log"), ErrKind::Log)?;
///     x.write(b"test")?;
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! map_err {
	($in_err:expr, $kind:expr) => {{
		use bmw_err::map_err;
		map_err!($in_err, $kind, "")
	}};
	($in_err:expr, $kind:expr, $msg:expr) => {{
		use bmw_err::{ErrKind, Error, ErrorKind};
		$in_err.map_err(|e| -> Error {
			match $kind {
				ErrKind::Configuration => {
					ErrorKind::Configuration(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::IO => ErrorKind::IO(format!("{}: {}", $msg, e)).into(),
				ErrKind::Log => ErrorKind::Log(format!("{}: {}", $msg, e)).into(),
				ErrKind::UnexpectedEof => {
					ErrorKind::UnexpectedEof(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::Utf8 => ErrorKind::Utf8(format!("{}: {}", $msg, e)).into(),
				ErrKind::ArrayIndexOutOfBounds => {
					ErrorKind::ArrayIndexOutOfBounds(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::Timeout => ErrorKind::Timeout(format!("{}: {}", $msg, e)).into(),
				ErrKind::CapacityExceeded => {
					ErrorKind::CapacityExceeded(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::IllegalArgument => {
					ErrorKind::IllegalArgument(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::Poison => ErrorKind::Poison(format!("{}: {}", $msg, e)).into(),
				ErrKind::Misc => ErrorKind::Misc(format!("{}: {}", $msg, e)).into(),
				ErrKind::CorruptedData => {
					ErrorKind::CorruptedData(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::IllegalState => ErrorKind::IllegalState(format!("{}: {}", $msg, e)).into(),
				ErrKind::Test => ErrorKind::Test(format!("{}: {}", $msg, e)).into(),
				ErrKind::Overflow => ErrorKind::Overflow(format!("{}: {}", $msg, e)).into(),
				ErrKind::ThreadPanic => ErrorKind::ThreadPanic(format!("{}: {}", $msg, e)).into(),
				ErrKind::Alloc => ErrorKind::Alloc(format!("{}: {}", $msg, e)).into(),
				ErrKind::OperationNotSupported => {
					ErrorKind::OperationNotSupported(format!("{}: {}", $msg, e)).into()
				}
				ErrKind::SystemTime => ErrorKind::SystemTime(format!("{}: {}", $msg, e)).into(),
				ErrKind::Errno => ErrorKind::Errno(format!("{}: {}", $msg, e)).into(),
				ErrKind::Rustls => ErrorKind::Rustls(format!("{}: {}", $msg, e)).into(),
				ErrKind::Crypt => ErrorKind::Crypt(format!("{}: {}", $msg, e)).into(),
				ErrKind::Http => ErrorKind::Http(format!("{}: {}", $msg, e)).into(),
				ErrKind::Rustlet => ErrorKind::Rustlet(format!("{}: {}", $msg, e)).into(),
			}
		})
	}};
}

/// Macro to do a conditional break
#[macro_export]
macro_rules! cbreak {
	($cond:expr) => {{
		if $cond {
			break;
		}
	}};
}
