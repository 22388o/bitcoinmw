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

use crate::LogConfig2_Options;
use bmw_deps::dyn_clone::DynClone;
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use std::sync::{Arc, RwLock};

/// Internal enum used by the global logging macros like [`crate::info`], [`crate::info_plain`],
/// and [`crate::info_all`] to configure which option is being used. This should not generally be
/// used in favor of using the macros to control output.
#[derive(PartialEq)]
pub enum LoggingType {
	Standard,
	Plain,
	All,
}

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

/// The main trait implemented by the bmw_log crate. Some features include: color coding, timestamps,
/// stdout/file, rotation by size and time, log levels, file/line number to help with debugging,
/// millisecond precision, auto-rotation capabilities, backtraces, file headers and ability to
/// delete log rotations. Most implementations can use the log macros in this library instead
/// of using the logger directly.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///     
/// fn main() -> Result<(), Error> {
///     let test_info = test_info!()?;
///     let mut buf = PathBuf::new();
///     buf.push(test_info.directory());
///     buf.push("mylog.log");
///     let buf = buf.display().to_string();
///     
///     let mut log = logger!(
///         DisplayBacktrace(false),
///         LogFilePath(&buf),
///         AutoRotate(false)
///     )?;
///     log.init()?;
///     
///     log.log(LogLevel::Info, "test1")?;
///     log.log_all(LogLevel::Debug, "test2")?;
///     log.log_plain(LogLevel::Warn, "test3")?;
///                     
///     Ok(())          
/// }                   
///```                  
///             
/// The output of the above code will look something like this:
///
///```text
/// [2022-08-09 15:41:55.633]: (INFO) [../ops/function.rs:248]: test1
/// [2022-08-09 15:41:55.633]: (DEBUG) [../ops/function.rs:248]: test2
/// test3
///```
pub trait Log: DynClone {
	/// Log data to disk/stdout. Note that even though a log level is specified,
	/// the line is always logged for display purposes. If you wish to use log levels to
	/// filter, use the macros: [`crate::fatal`], [`crate::error`], [`crate::warn`], [`crate::info`],
	/// [`crate::debug`], [`crate::trace`]. This function returns () or a Error.
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// The same as [`Log::log`], but this function will always log to standard output even if
	/// standard output logging is currently disabled by the underlying logger.
	/// This function returns () or a Error.
	fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// Log without any of the header details. As seen in the example, only 'test3' was logged.
	/// no timestamp, log level, or line num info is logged. This function returns () or a
	/// Error.
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
	/// least this level
	fn set_log_level(&mut self, level: LogLevel);
	/// Initialize the log. The function does any needed i/o operations to secure the file
	/// handle. It may only be called once and must be called before any logging or rotations
	/// occur.
	fn init(&mut self) -> Result<(), Error>;
	/// Close the log file
	fn close(&mut self) -> Result<(), Error>;
	/// Set the specified ConfigOption. It may seem a little non-intuitive to see a set
	/// function with a single parameter, however part of the design of the logger is such that
	/// there is only a single function to set these values which have multiple types. It is
	/// possible to do that with enums and that is how it is implemented. The examples should
	/// make it clear how to set these options which can be set in the initial config or after
	/// logging has began with the exception of LogFilePath. This function returns () or a
	/// Error.
	fn set_config_option(&mut self, value: LogConfig2_Options) -> Result<(), Error>;

	#[cfg(test)]
	fn debug_process_resolve_frame_error(&mut self);

	#[cfg(test)]
	fn debug_invalid_metadata(&mut self);

	#[cfg(test)]
	fn debug_lineno_is_none(&mut self);
}

/// Builder struct used to build [`crate::Log`] implementations.
pub struct LogBuilder {}

#[doc(hidden)]
pub struct GlobalLogContainer {}

lazy_static! {
	#[doc(hidden)]
	pub static ref BMW_GLOBAL_LOG: Arc<RwLock<Option<Box<dyn Log + Send + Sync>>>> = Arc::new(RwLock::new(None));
}
