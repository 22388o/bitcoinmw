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
use bmw_derive::{add_doc, document};
use bmw_err::*;
use std::sync::{Arc, RwLock};

/// Log2 trait description
#[document]
#[add_doc(see: crate::LogLevel)]
pub trait Log2 {
	/// Some doc in regular style
	#[add_doc(input: level - logging level to log at)]
	#[add_doc(input: level - more about the level here)]
	#[add_doc(input: line - line to log)]
	#[add_doc(return: this is a very interesting return)]
	#[add_doc(error: bmw_err::ErrKind::Log - if the stream closes)]
	#[add_doc(error: bmw_err::ErrKind::IO - if an i/o error occurs)]
	#[add_doc(error: more about the i/o error)]
	#[add_doc(see: crate::Log::log_all)]
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;

	#[add_doc(see: crate::Log::log)]
	#[add_doc(return: cool bool value returned)]
	fn ok(&self) -> Result<bool, Error>;
}

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

/// The [`crate::Log`] trait is the interface for all logging in BitcoinMW.
/// Logging can be done either with an individual logger created by a call to the
/// [`crate::logger`] macro...
///```
/// use bmw_err::*;
/// use bmw_log::*;
///
/// fn main() -> Result<(), Error> {
///     // create a logger with defaults
///     let mut logger = logger!()?;
///
///     // init the logger
///     logger.init()?;
///
///     // set log level to Info
///     logger.set_log_level(LogLevel::Info);
///
///     // log at the info level
///     logger.log(LogLevel::Info, "this will show up")?;
///
///     // log at the debug level
///     logger.log(LogLevel::Debug, "this will not")?;
///     Ok(())
/// }   
///```
/// ...or globally using the global logging macros...
///```
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!(); // set the global log level for this scope
///
/// fn main() -> Result<(), Error> {
///     info!("this shows up")?;
///     debug!("this doesn't")?;
///
///     info_plain!("log plain")?;
///     debug_plain!("this doesn't show up")?;
///     
///     Ok(())
/// }
///```
pub trait Log: DynClone {
	/// Log data to this logger.
	/// # Input Parameters
	/// * `level` - The [`crate::LogLevel`] to log at. If the level is equal to or above the level
	/// set by [`crate::Log::set_log_level`], the data will be logged. Otherwise, the function
	/// call will be ignored.
	/// * `line` - line to log.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if the logger has not been initialized.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
	/// # Also see
	/// [`crate::Log::log_all`]
	///
	/// [`crate::Log::log_plain`]
	/// # Examples
	///```
	/// use bmw_err::*;
	/// use bmw_log::*;
	///
	/// fn main() -> Result<(), Error> {
	///     // create a logger with defaults
	///     let mut logger = logger!()?;
	///
	///     // init the logger
	///     logger.init()?;
	///
	///     // set log level to Info
	///     logger.set_log_level(LogLevel::Info);
	///
	///     // log at the info level
	///     logger.log(LogLevel::Info, "this will show up")?;
	///
	///     // log at the debug level
	///     logger.log(LogLevel::Debug, "this will not")?;
	///
	///     Ok(())
	/// }
	///```
	/// # Output
	/// The output of the above example would look something like this:
	///
	///```text
	/// [2024-04-14 17:45:46.899]:  (INFO) [..src/bin/http.rs:100]: this will show up
	///```
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// The same as [`Log::log`], but this function will always log to standard output even if
	/// standard output logging is currently disabled by the underlying logger.
	/// # Input Parameters
	/// * `level` - The [`crate::LogLevel`] to log at. If the level is equal to or above the level
	/// set by [`crate::Log::set_log_level`], the data will be logged. Otherwise, the function
	/// call will be ignored.
	///
	/// * `line` - line to log.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if the logger has not been initialized.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
	/// # Also see
	/// [`crate::Log::log`]
	///
	/// [`crate::Log::log_plain`]
	/// # Examples
	///```
	/// use bmw_err::*;
	/// use bmw_log::*;
	/// use bmw_test::*;
	/// use std::path::PathBuf;
	///
	/// fn main() -> Result<(), Error> {
	///     // get test info so we can get a temporary directory name
	///     let test_info = test_info!()?;
	///
	///     // create a logger that logs to a log file only
	///     let mut path_buf = PathBuf::from(test_info.directory());
	///     path_buf.push("test.log");
	///     let path = path_buf.display().to_string();
	///     let mut logger = logger!(LogFilePath(&path), DisplayStdout(false))?;
	///
	///     // init the logger
	///     logger.init()?;
	///
	///     // set log level to Info
	///     logger.set_log_level(LogLevel::Info);
	///
	///     // log at the info level (this will log to disk only)
	///     logger.log(LogLevel::Info, "this will show up on disk")?;
	///
	///     // log_all at the info level (this will log both to disk and stdout)
	///     logger.log_all(LogLevel::Info, "this will show up on disk and stdout")?;
	///
	///     Ok(())
	/// }
	///```
	fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// The same as [`Log::log`], but Log without any of the header details. No timestamp,
	/// logging level, or line numbers data are logged.
	/// # Input Parameters
	/// * `level` - The [`crate::LogLevel`] to log at. If the level is equal to or above the level
	/// set by [`crate::Log::set_log_level`], the data will be logged. Otherwise, the function
	/// call will be ignored.
	///
	/// * `line` - line to log.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if the logger has not been initialized.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
	/// # Also see
	/// [`crate::Log::log`]
	///
	/// [`crate::Log::log_all`]
	/// # Examples
	///```
	/// use bmw_err::*;
	/// use bmw_log::*;
	///
	/// fn main() -> Result<(), Error> {
	///     // create a logger with default values
	///     let mut logger = logger!()?;
	///
	///     // init the logger
	///     logger.init()?;
	///
	///     // set log level to Info
	///     logger.set_log_level(LogLevel::Info);
	///
	///     // call 'log'
	///     logger.log(LogLevel::Info, "regular log line")?;
	///
	///     // call 'log_plain'
	///     logger.log_plain(LogLevel::Info, "only line is logged")?;
	///
	///     Ok(())
	/// }
	///```
	/// # Output
	/// The output of the above example would look something like this:
	///
	///```text
	/// [2024-04-14 17:45:46.899]:  (INFO) [..src/bin/http.rs:100]: regular log line
	/// only line is logged
	///```
	fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	/// Do a log rotation. The name of the file rotated is automatically generated and stored
	/// in the same directory as the original log file. Logging then proceeds with the original
	/// log file name.
	/// # Rotated log file name
	/// The name of the rotated log file will be of the form:
	/// <log_name_without_extension>.r_%M_%D_%Y_%H-%M-%S_<random_value>.log
	/// where
	/// %M is month
	/// %D is day
	/// %Y is year
	/// %H is hour (0-23)
	/// %M is minute
	/// %S is second
	/// # Timezone
	/// These values are based on the local time
	/// An example log file rotation name might look like:
	/// test.r_08_09_2022_15-54-58_11545678356999821787.log
	///
	/// # Auto-rotation information
	/// If auto rotation is enabled, then this function does not need to be called, however it
	/// still may be called manually. Note that auto-rotation only occurs when the logger is
	/// called so it might take some time to happen unless called manually.
	/// # Input Parameters
	/// n/a
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if the log is not initialized.
	///
	/// [`bmw_err::ErrKind::Log`] - if the log is not configured to log to a file.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs while rotating the log file.
	/// # Also see
	/// [`crate::Log::need_rotate`]
	fn rotate(&mut self) -> Result<(), Error>;
	/// This function checks if a log rotation is needed. It returns true if it is needed and
	/// false otherwise.
	/// # Input Parameters
	/// n/a
	/// # Return
	/// [`true`] if a log rotation is needed. Otherwise [`false`].
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if the log is not initialized.
	/// # Also see
	/// [`crate::Log::rotate`]
	fn need_rotate(&self) -> Result<bool, Error>;
	/// Sets the log level threshold. Logging only occurs if the logged line is logged at at
	/// least this level.
	/// # Input Parameters
	/// * `level` - the threshold level to set for this logger. Any level set during a call to
	/// [`crate::Log::log`] that is equal to or greater than this level will be logged.
	/// Anything lower than this level will be ignored.
	/// # Return
	/// n/a
	/// # Errors
	/// n/a
	/// # Also see
	/// [`crate::Log::log`]
	///
	/// [`crate::Log::log_all`]
	///
	/// [`crate::Log::log_plain`]
	fn set_log_level(&mut self, level: LogLevel);
	/// Initialize the log. The function does any needed i/o operations to secure the file
	/// handle. It may only be called once and must be called before any logging or rotations
	/// occur.
	/// # Input Parameters
	/// n/a
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if [`crate::Log::init`] has already been called.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
	/// # Also see
	/// [`crate::logger`]
	///
	/// [`crate::log_init`]
	fn init(&mut self) -> Result<(), Error>;
	/// Close the log file.
	/// # Input Parameters
	/// n/a
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if [`crate::Log::init`] has not been called.
	///
	/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
	/// # Also see
	/// [`crate::Log::init`]
	fn close(&mut self) -> Result<(), Error>;
	/// Set the specified LogConfigOption. Attempting to set LogFilePath will result in an error.
	/// Note that this function must be called after [`crate::Log::init`] has been called.
	/// # Input ParametersA
	/// * `value` - The [`crate::LogConfigOptions`] to set. After initialization, most of the
	/// configuration settings may be changed. The only exception is
	/// [`crate::LogConfigOptions::LogFilePath`]. Attempting to set this option will result in
	/// an error.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// [`bmw_err::ErrKind::Log`] - if value is a [`crate::LogConfigOptions::LogFilePath`].
	///
	/// [`bmw_err::ErrKind::Log`] - if [`crate::Log::init`] has not been called.
	/// # Also see
	/// [`crate::logger`]
	///
	/// [`crate::LogConfigOptions`]
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
