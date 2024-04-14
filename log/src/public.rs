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

use bmw_deps::dyn_clone::DynClone;
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use std::sync::{Arc, RwLock};

pub use crate::types::LogConfigOptions;

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

/// Builder struct used to build [`crate::Log`] implementations.
pub struct LogBuilder {}

pub trait Log: DynClone {
	/// Log data to disk/stdout.
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// The same as [`Log::log`], but this function will always log to standard output even if
	/// standard output logging is currently disabled by the underlying logger.
	/// This function returns () or a Error.
	fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// Log without any of the header details. No timestamp, logging level, or line numbers
	/// data are logged.
	fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// Do a log rotation. The name of the file rotated is automatically generated and stored
	/// in the same directory as the original log file. Logging then proceeds with the original
	/// log file. The name of the rotated log file will be of the form:
	/// <log_name_without_extension>.r_%M_%D_%Y_%H-%M-%S_<random_value>.log
	/// where
	/// %M is month
	/// %D is day
	/// %Y is year
	/// %H is hour (0-23)
	/// %M is minute
	/// %S is second
	/// These values are based on the local time
	/// An example log file rotation name might look like:
	/// test.r_08_09_2022_15-54-58_11545678356999821787.log
	///
	/// If auto rotation is enabled, then this function does not need to be called, however it
	/// still may be called manually. Note that auto-rotation only occurs when the logger is
	/// called so it might take some time to happen unless called manually. This function has
	/// no parameters and returns () or a Error.
	fn rotate(&mut self) -> Result<(), Error>;
	/// This function checks if a log rotation is needed. It returns true if it is needed and
	/// false otherwise. This function returns () or a Error.
	fn need_rotate(&self) -> Result<bool, Error>;
	/// Sets the log level threshold. Logging only occurs if the logged line is logged at at
	/// least this level.
	fn set_log_level(&mut self, level: LogLevel);
	/// Initialize the log. The function does any needed i/o operations to secure the file
	/// handle. It may only be called once and must be called before any logging or rotations
	/// occur.
	fn init(&mut self) -> Result<(), Error>;
	/// Close the log file.
	fn close(&mut self) -> Result<(), Error>;
	/// Set the specified LogConfigOption. Attempting to set LogFilePath will result in an error.
	fn set_config_option(&mut self, value: LogConfigOptions) -> Result<(), Error>;
}

// used by macros
#[doc(hidden)]
#[derive(PartialEq)]
pub enum LoggingType {
	Standard,
	Plain,
	All,
}

// Holder for the global logger
#[doc(hidden)]
pub struct GlobalLogFunctions {}

//  global logger
lazy_static! {
	#[doc(hidden)]
	pub static ref BMW_GLOBAL_LOG: Arc<RwLock<Option<Box<dyn Log + Send + Sync>>>> = Arc::new(RwLock::new(None));
}
