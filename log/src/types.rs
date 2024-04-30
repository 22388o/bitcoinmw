// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_core::*;

/// Standard 6 log levels.
#[derive(PartialEq, Copy, Clone)]
pub enum LogLevel {
	/// Very fine grained logging information that should not generally be visible except for
	/// debugging purposes
	Trace,
	/// Debugging information
	Debug,
	/// Standard information that is usually displayed to the user under most circumstances
	Info,
	/// Warning of something that the user should be aware of, although it may not be an error
	Warn,
	/// Error that the user must be aware of
	Error,
	/// Fatal error that usually causes the application to be unusable
	Fatal,
}

/// Types of errors that occur during logging
#[ErrorKind]
pub enum LogErrorKind {
	Log,
	Configuration,
	IllegalState,
	AlreadyInitialized,
	/// failed to retreive metadata
	MetaData,
	NotInitialized,
	/// simulated test error
	Test,
}

// used by macros
#[doc(hidden)]
#[derive(PartialEq)]
pub enum LoggingType {
	Standard,
	Plain,
	All,
}
