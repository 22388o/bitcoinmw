// Copyright (c) 2022, 37 Miners, LLC
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

use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::time::Instant;

/// Standard 6 log levels.
#[derive(PartialEq)]
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

impl Display for LogLevel {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self {
			LogLevel::Trace => write!(w, "TRACE"),
			LogLevel::Debug => write!(w, "DEBUG"),
			LogLevel::Info => write!(w, "INFO"),
			LogLevel::Warn => write!(w, "WARN"),
			LogLevel::Error => write!(w, "ERROR"),
			LogLevel::Fatal => write!(w, "FATAL"),
		}
	}
}

/// This enum contains the names of the configuration options. It is used in the
/// [`Log::get_config_option`] function. See [`Log::get_config_option`] for further details.
pub enum LogConfigOptionName {
	/// View the Colors setting. See [`LogConfigOption::Colors`].
	Colors,
	/// View the Stdout logging setting. See [`LogConfigOption::Stdout`].
	Stdout,
	/// View the MaxSizeBytes setting. See [`LogConfigOption::MaxSizeBytes`].
	MaxSizeBytes,
	/// View the MaxAgeMillis setting. See [`LogConfigOption::MaxAgeMillis`].
	MaxAgeMillis,
	/// View the Timestamp setting. See [`LogConfigOption::Timestamp`].
	Timestamp,
	/// View the Level setting. See [`LogConfigOption::Level`].
	Level,
	/// View the LinNum setting. See [`LogConfigOption::LineNum`].
	LineNum,
	/// View the ShowMillis setting. See [`LogConfigOption::ShowMillis`].
	ShowMillis,
	/// View the AutoRotate setting. See [`LogConfigOption::AutoRotate`].
	AutoRotate,
	/// View the FilePath setting. See [`LogConfigOption::FilePath`].
	FilePath,
	/// View the ShowBt setting. See [`LogConfigOption::ShowBt`].
	ShowBt,
	/// View the LineNumDataMaxLen setting. See [`LogConfigOption::LineNumDataMaxLen`].
	LineNumDataMaxLen,
	/// View the DeleteRotation setting. See [`LogConfigOption::DeleteRotation`].
	DeleteRotation,
	/// View the FileHeader setting. See [`LogConfigOption::FileHeader`].
	FileHeader,
}

/// This enum is used to get/set log settings after [`Log::init`] is called. The
/// only setting that cannot be set after initialization is the [`LogConfigOption::FilePath`]
/// setting. It is read only. Trying to write to it will result in an error. The function used
/// to get these values is [`Log::get_config_option`] and the function used to set these values
/// is [`Log::set_config_option`].
#[derive(PartialEq, Debug, Clone)]
pub enum LogConfigOption {
	/// Whether or not to display colors for this log. The default value is true.
	Colors(bool),
	/// Whether or not to log to standard output for this log. The default value is true.
	Stdout(bool),
	/// The maximum size in bytes before this log needs to be rotated. The default value is
	/// 1_048_576 bytes or 1 mb.
	MaxSizeBytes(u64),
	/// The maximum time in milliseconds before this log needs to be rotated. The default value
	/// is 3_600_000 ms or 1 hour.
	MaxAgeMillis(u128),
	/// Whether or not to display the timestamp with this log. The default value is true.
	Timestamp(bool),
	/// Whether or not to display the log level with this log. The default value is true.
	Level(bool),
	/// Whether or not to display the line number information with this log. The default value
	/// is true for all platforms except windows. On windows there are some potential
	/// problems with the resolve_frame function as noted in the backtrace crate documentation.
	LineNum(bool),
	/// Whether or not to show milliseconds with this log. The default value is true.
	ShowMillis(bool),
	/// Whether or not to auto-rotate this log. The default value is true.
	AutoRotate(bool),
	/// The optional file path that this log writes to. The default value is None.
	FilePath(Option<PathBuf>),
	/// Whether or not to show backtraces with this log. Backtraces are only displayed with the
	/// [`LogLevel::Error`] and [`LogLevel::Fatal`] when this configuration is enabled. The default
	/// value is true.
	ShowBt(bool),
	/// The maximum length of the line number data that is logged. Since the path of the
	/// filename may be long, it must be limited. The default value is 25 characters.
	LineNumDataMaxLen(usize),
	/// Whether or not to delete the log rotation with this log. This is usually only used for
	/// testing purposes when many logs would be generated and must be deleted to save space
	/// on the test system. The default value is false.
	DeleteRotation(bool),
	/// A header line to be displayed at the top of each file produced by this logger. The
	/// default value is an empty string which is not displayed.
	FileHeader(String),
}

/// The log configuration struct. Logs can only be built through the [`crate::LogBuilder::build`]
/// function. This is the only parameter to that function. An example configuration with all
/// parameters explicitly specified might look like this:
///
///```
/// use bmw_log::LogConfigOption::*;
/// use bmw_log::LogConfig;
///
/// let config = LogConfig {
///     colors: Colors(true),
///     stdout: Stdout(true),
///     max_size_bytes: MaxSizeBytes(1024 * 1024 * 5),
///     max_age_millis: MaxAgeMillis(1000 * 30 * 60),
///     timestamp: Timestamp(true),
///     level: Level(true),
///     line_num: LineNum(false),
///     show_millis: ShowMillis(false),
///     auto_rotate: AutoRotate(true),
///     file_path: FilePath(None),
///     show_bt: ShowBt(true),
///     line_num_data_max_len: LineNumDataMaxLen(20),
///     delete_rotation: DeleteRotation(false),
///     file_header: FileHeader("BMW Log V1.1".to_string()),
///     debug_invalid_metadata: false,
///     debug_invalid_os_str: false,
///     debug_lineno_none: false,
///     debug_process_resolve_frame_error: false,
/// };
///```
///
/// Generally speaking the configurations are specified using the  [`core::default::Default`] trait
/// which is implemented for [`LogConfig`]. An example might look like this:
///```
///
/// use bmw_log::LogConfigOption::*;
/// use bmw_log::LogConfig;
/// use std::path::PathBuf;
///
/// let config = LogConfig {
///     colors: Colors(false),
///     stdout: Stdout(false),
///     line_num: LineNum(false),
///     file_path: FilePath(Some(PathBuf::from("/path/to/my/log.log".to_string()))),
///     ..Default::default()
/// };
/// ```
///
#[derive(Debug, Clone)]
pub struct LogConfig {
	/// See [`LogConfigOption::Colors`]. The default value is Colors(true).
	pub colors: LogConfigOption,
	/// See [`LogConfigOption::Stdout`]. The default value is Stdout(true).
	pub stdout: LogConfigOption,
	/// See [`LogConfigOption::MaxSizeBytes`]. The default value is MaxSizeBytes(1024 * 1024) or 1 mb.
	pub max_size_bytes: LogConfigOption,
	/// See [`LogConfigOption::MaxAgeMillis`]. The default value is MaxAgeMillis(60 * 60 * 1000) or 1 hour.
	pub max_age_millis: LogConfigOption,
	/// See [`LogConfigOption::Timestamp`]. The default value is Timestamp(true).
	pub timestamp: LogConfigOption,
	/// See [`LogConfigOption::Level`]. The default value is Level(true).
	pub level: LogConfigOption,
	/// See [`LogConfigOption::LineNum`]. The default value is LineNum(true).
	pub line_num: LogConfigOption,
	/// See [`LogConfigOption::ShowMillis`]. The default value is ShowMillis(true).
	pub show_millis: LogConfigOption,
	/// See [`LogConfigOption::AutoRotate`]. The default value is AutoRotate(true).
	pub auto_rotate: LogConfigOption,
	/// See [`LogConfigOption::FilePath`]. The default value is FilePath(None).
	pub file_path: LogConfigOption,
	/// See [`LogConfigOption::ShowBt`]. The default value is ShowBt(true).
	pub show_bt: LogConfigOption,
	/// See [`LogConfigOption::LineNumDataMaxLen`]. The default value is LinNumDataMaxLen(25)
	/// or 25 bytes.
	pub line_num_data_max_len: LogConfigOption,
	/// See [`LogConfigOption::DeleteRotation`]. The default value is DeleteRotation(false).
	pub delete_rotation: LogConfigOption,
	/// See [`LogConfigOption::FileHeader`]. The default value is FileHeader("".to_string()) or
	/// no file header.
	pub file_header: LogConfigOption,
	/// Parameter used in testing. Must always be set false in normal use.
	pub debug_invalid_metadata: bool,
	/// Parameter used in testing. Must always be set false in normal use.
	pub debug_invalid_os_str: bool,
	/// Parameter used in testing. Must always be set false in normal use.
	pub debug_lineno_none: bool,
	/// Parameter used in testing. Must always be set false in normal use.
	pub debug_process_resolve_frame_error: bool,
}

impl Default for LogConfig {
	fn default() -> Self {
		Self {
			colors: LogConfigOption::Colors(true),
			stdout: LogConfigOption::Stdout(true),
			max_size_bytes: LogConfigOption::MaxSizeBytes(1024 * 1024),
			max_age_millis: LogConfigOption::MaxAgeMillis(1000 * 60 * 60),
			timestamp: LogConfigOption::Timestamp(true),
			level: LogConfigOption::Level(true),
			#[cfg(windows)]
			line_num: LogConfigOption::LineNum(false),
			#[cfg(not(windows))]
			line_num: LogConfigOption::LineNum(true),
			show_millis: LogConfigOption::ShowMillis(true),
			auto_rotate: LogConfigOption::AutoRotate(true),
			file_path: LogConfigOption::FilePath(None),
			show_bt: LogConfigOption::ShowBt(true),
			line_num_data_max_len: LogConfigOption::LineNumDataMaxLen(25),
			delete_rotation: LogConfigOption::DeleteRotation(false),
			file_header: LogConfigOption::FileHeader("".to_string()),
			debug_invalid_metadata: false,
			debug_invalid_os_str: false,
			debug_lineno_none: false,
			debug_process_resolve_frame_error: false,
		}
	}
}

/// The main trait implemented by the bmw logger. Some features include: color coding, timestamps,
/// stdout/file, rotation by size and time, log levels, file/line number to help with debugging,
/// millisecond precision, auto-rotation capabilities, backtraces, file headers and ability to
/// delete log rotations. Most implementations can use the log macros in this library instead
/// of using the logger directly.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::LogBuilder;
/// use bmw_log::LogConfigOption::*;
/// use bmw_log::LogConfig;
/// use bmw_log::LogLevel;
/// use std::path::PathBuf;
///
/// fn my_log() -> Result<(), Error> {
///
///     let config = LogConfig {
///         show_bt: ShowBt(false),
///         file_path: FilePath(Some(PathBuf::from("/path/to/my.log"))),
///         auto_rotate: AutoRotate(false),
///         ..Default::default()
///     };
///
///     let mut log = LogBuilder::build(config)?;
///     log.init()?;
///
///     log.log(LogLevel::Info, "test1", None)?;
///     log.log_all(LogLevel::Debug, "test2", None)?;
///     log.log_plain(LogLevel::Warn, "test3", None)?;
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
	/// [`crate::debug`], [`crate::trace`]. Optionally an Instant may be specified to avoid
	/// having the logger create its own instant. This function returns () or a
	/// [`bmw_err::Error`].
	fn log(&mut self, level: LogLevel, line: &str, now: Option<Instant>) -> Result<(), Error>;

	/// The same as [`Log::log`], but this function will always log to standard output even if
	/// standard output logging is currently disabled by the underlying logger. Optionally an
	/// Instant may be specified to avoid having the logger create its own instant.
	/// This function returns () or a [`bmw_err::Error`].
	fn log_all(&mut self, level: LogLevel, line: &str, now: Option<Instant>) -> Result<(), Error>;
	/// Log without any of the header details. As seen in the example, only 'test3' was logged.
	/// no timestamp, log level, or line num info is logged. Optionally an Instant may be specified to avoid
	/// having the logger create its own instant. This function returns () or a
	/// [`bmw_err::Error`].
	fn log_plain(&mut self, level: LogLevel, line: &str, now: Option<Instant>)
		-> Result<(), Error>;
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
	/// no parameters and returns () or a [`bmw_err::Error`].
	fn rotate(&mut self) -> Result<(), Error>;

	/// This function checks if a log rotation is needed. It returns true if it is needed and
	/// false otherwise. Optionally an Instant may be specified to avoid
	/// having the logger create its own instant. This function returns () or a
	/// [`bmw_err::Error`].
	fn need_rotate(&self, now: Option<Instant>) -> Result<bool, Error>;

	/// Initialize the log. This function must be called before any logging or rotation
	/// functions can be called. This function has no parameters and returns () or a
	/// [`bmw_err::Error`].
	fn init(&mut self) -> Result<(), Error>;

	/// Set the specified LogConfigOption. It may seem a little non-intuitive to see a set
	/// function with a single parameter, however part of the design of the logger is such that
	/// there is only a single function to set these values which have multiple types. It is
	/// possible to do that with enums and that is how it was implemented. The examples should
	/// make it clear how to set these options which can be set in the initial config or after
	/// logging has began with the exception of FilePath. This function returns () or a
	/// [`bmw_err::Error`].
	fn set_config_option(&mut self, value: LogConfigOption) -> Result<(), Error>;

	/// Get the specified [`crate::LogConfigOptionName`]. These names correspond to the
	/// [`crate::LogConfigOption`]. The function returns either a reference to a
	/// [`crate::LogConfigOption`] or a [`bmw_err::Error`].
	fn get_config_option(&self, option: LogConfigOptionName) -> Result<&LogConfigOption, Error>;
}

clone_trait_object!(Log);

#[cfg(test)]
mod test {
	use crate::types::{LogConfig, LogConfigOption, LogLevel};
	use bmw_err::Error;

	#[test]
	fn test_log_config() -> Result<(), Error> {
		let d = LogConfig::default();
		assert_eq!(d.colors, LogConfigOption::Colors(true));
		Ok(())
	}

	#[test]
	fn test_display_levels() -> Result<(), Error> {
		assert_eq!(format!("{}", LogLevel::Trace), "TRACE".to_string());
		assert_eq!(format!("{}", LogLevel::Debug), "DEBUG".to_string());

		assert_eq!(format!("{}", LogLevel::Info), "INFO".to_string());
		assert_eq!(format!("{}", LogLevel::Warn), "WARN".to_string());
		assert_eq!(format!("{}", LogLevel::Error), "ERROR".to_string());
		assert_eq!(format!("{}", LogLevel::Fatal), "FATAL".to_string());

		Ok(())
	}
}
