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

use bmw_deps::failure::{Context, Fail};

/// Base Error struct which is used throughout BMW.
#[derive(Debug, Fail)]
pub struct Error {
	pub(crate) inner: Context<ErrorKind>,
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
	/// Http Error
	#[fail(display = "http_error: {}", _0)]
	Http(String),
	/// Http 404 Error
	#[fail(display = "http404_error: {}", _0)]
	Http404(String),
	/// Http 403 Error
	#[fail(display = "http403_error: {}", _0)]
	Http403(String),
	/// Http 400 Error
	#[fail(display = "http400_error: {}", _0)]
	Http400(String),
	/// Rustlet Error
	#[fail(display = "rustlet_error: {}", _0)]
	Rustlet(String),
	/// Parse Error
	#[fail(display = "parse_error: {}", _0)]
	Parse(String),
}

/// The kinds of errors in this crate. This enum is used to map to error
/// names using the [`crate::err`] and [`crate::map_err`] macros.
pub enum ErrKind {
	/// IO Error
	IO,
	/// Log Error
	Log,
	/// A conversion to the UTF-8 format resulted in an error
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
	/// Http 404 error
	Http404,
	/// Http 403 error
	Http403,
	/// Http 400 error
	Http400,
	/// Rustlet error
	Rustlet,
	/// Parse error
	Parse,
}
