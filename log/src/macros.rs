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

/// a usize conversion macro
#[macro_export]
macro_rules! usize {
	($v:expr) => {{
		use std::convert::TryInto;
		let v: usize = $v.try_into()?;
		v
	}};
}

/// a u64 conversion macro
#[macro_export]
macro_rules! u64 {
	($v:expr) => {{
		use std::convert::TryInto;
		let v: u64 = $v.try_into()?;
		v
	}};
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Trace`] or log at the [`crate::LogLevel::Trace`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// trace!();
///
/// fn main() -> Result<(), Error> {
///     trace!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! trace {
	() => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
	};
	($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
	}};
	($line:expr,$($values:tt)*) => {
                trace!(&format!($line, $($values)*)[..])
	};
}

/// Same as [`trace`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! trace_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`trace`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! trace_all {
        () => {
                 #[doc(hidden)]
                 const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_all!(&format!($line, $($values)*)[..])
        };
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Debug`] or log at the [`crate::LogLevel::Debug`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     debug!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! debug {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                debug!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`debug`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! debug_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`debug`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! debug_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_all!(&format!($line, $($values)*)[..])
        };
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Info`] or log at the [`crate::LogLevel::Info`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     info!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! info {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                info!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`info`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! info_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                info_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`info`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! info_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                info_all!(&format!($line, $($values)*)[..])
        };
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Warn`] or log at the [`crate::LogLevel::Warn`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// warn!();
///
/// fn main() -> Result<(), Error> {
///     warn!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! warn {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                warn!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`warn`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! warn_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`warn`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! warn_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_all!(&format!($line, $($values)*)[..])
        };
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Error`] or log at the [`crate::LogLevel::Error`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// error!();
///
/// fn main() -> Result<(), Error> {
///     error!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! error {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                error!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`error`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! error_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                error_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`error`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! error_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                error_all!(&format!($line, $($values)*)[..])
        };
}

/// Set [`crate::LogLevel`] to [`crate::LogLevel::Fatal`] or log at the [`crate::LogLevel::Fatal`] log level.
/// If no parameters are specified the log level will be set. If a single parameter is specified,
/// that string will be logged. If two or more parameters are specified, the first parameter is a format
/// string, the additional parameters will be formatted based on the format string. Logging
/// is done by the global logger which can be configured using the [`crate::log_init`]
/// macro. If [`crate::log_init`] is not called, the default values are used.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// fatal!();
///
/// fn main() -> Result<(), Error> {
///     fatal!("v1={},v2={}", 123, "def")?;
///
///     Ok(())
/// }
///
///```
///
/// Note: log level must be set before using the logger or a compilation error will occur. Log
/// level can be changed at any time and the inner most scope is used. The suggested method of use
/// is to set the level at the top of each file and adjust as development is completed. You may
/// start with debug or trace and eventually end up at info or warn.
#[macro_export]
macro_rules! fatal {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`fatal`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! fatal_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_plain!(&format!($line, $($values)*)[..])
        };
}

/// Same as [`fatal`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! fatal_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_all!(&format!($line, $($values)*)[..])
        };
}

/// Initialize the global log. This macro takes a list of ConfigOption, If none are
/// specified, the default values are used. Note that if this macro
/// is not called before logging occurs, the default configuration is used. After
/// either this macro is called or the default is set via another logging macro,
/// calling this macro again will result in an error. It usually makes sense to
/// initialize this macro very early in the startup of an application so that no
/// unanticipated logging occurs before this macro is called by mistake.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     // get test_info to assign a test directory for the log
///     let test_info = test_info!()?;
///     let mut buf = PathBuf::new();
///     buf.push(test_info.directory());
///     buf.push("./main.log");
///     let buf = buf.display().to_string();
///
///     // init the global logger
///     log_init!(
///         DisplayBacktrace(false),
///         DisplayMillis(false),
///         LogFilePath(&buf)
///     )?;
///
///     info!("Startup complete!")?;
///
///     Ok(())
/// }
///```
///
/// Or without calling [`crate::log_init`]...
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     // the default configuration is used
///     info!("Startup complete!")?;
///
///     Ok(())
/// }
///```
///
/// Note that in the last example, the default values as described in [`crate::logger`] will be used.
/// Also, see that macro for an exhaustive list of configuration options that are valid for this
/// macro as well.
#[macro_export]
macro_rules! log_init {
	($($config:tt)*) => {{
		use bmw_log::GlobalLogContainer;
                use bmw_log::LogConfig2_Options::*;
                let v: Vec<LogConfig2_Options> = vec![$($config)*];
                GlobalLogContainer::init(v)
	}};
}

/// Configure the global log with the specified ConfigOption. This macro takes
/// a single argument. The macro returns () on success or Error on failure.
/// See [`crate::Log::set_config_option`] which is the underlying function call for
/// full details.
#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		use bmw_log::GlobalLogContainer;
		use bmw_log::LogConfig2_Options::*;
		GlobalLogContainer::set_log_option($option)
	}};
}

/// Rotate the global log. See [`crate::Log::rotate`] for full details on
/// the underlying rotate function and log rotation in general.
#[macro_export]
macro_rules! log_rotate {
	() => {{
		use bmw_log::GlobalLogContainer;
		GlobalLogContainer::rotate()
	}};
}

/// See if the global log needs to be rotated. See [`crate::Log::need_rotate`] for full details
/// on the underlying need_rotate function.
#[macro_export]
macro_rules! need_rotate {
	() => {{
		use bmw_log::GlobalLogContainer;
		GlobalLogContainer::need_rotate()
	}};
}

/// This macro builds a [`crate::Log`] implementation and returns it. Specifically, it return a
/// Box<dyn Log + Send + Sync>. This example below shows all of the allowed configurations that may
/// be specified. All of these are optional.
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
///     buf.push("test.log");
///     let buf = buf.display().to_string();
///
///     // these are all the legal configurations to use.
///     // most of these are the defaults. The only exceptions are:
///     // MaxSizeBytes - u64::MAX (i.e. no rotation)
///     // MaxAgeMillis - u128::MAX (i.e. no rotation)
///     // LogFilePath - None (by default only stdout logging occurs)
///     // FileHeader - "" (no header)
///     let mut logger = logger!(
///         MaxSizeBytes(1_024 * 1_024), // set rotation at 1mb
///         MaxAgeMillis(60 * 60 * 1_000), // set rotation at 1hr
///         DisplayColors(true), // whether or not to display colors on stdout
///         DisplayStdout(true), // whether or not to display on stdout
///         DisplayTimestamp(true), // whether or not to display the timestamp
///         DisplayLogLevel(true), // whether or not to display the log level
///         DisplayLineNum(true), // whether or not to display the code line number
///         DisplayMillis(true), // whether or not to display millisecond precision
///         LogFilePath(&buf), // path to the log file or None if no file logging
///         AutoRotate(true), // whether or not to automatically rotate the log file
///         DisplayBacktrace(false), // whether or not to display a backtrace on error/fatal
///         LineNumDataMaxLen(30), // maximum length of line num data
///         DeleteRotation(false), // whether or not to delete the rotated log file (test only)
///         FileHeader("my_header"), // header to place at the top of each file
///     )?;
///
///     logger.init()?;
///     logger.set_log_level(LogLevel::Debug);
///
///     logger.log(LogLevel::Debug, "This is a test!")?;
///     logger.log(LogLevel::Trace, "This will not show up!")?;
///
///     Ok(())
/// }
/// ```
///
/// # Invalid values
///
/// * The value for MaxAgeMillis must be at least 1_000 (1 second).
/// * The value for MaxSizeBytes must be at least 50 bytes.
/// * The value for LineNumDataMaxLen must be at least 10 bytes.
/// * The parent directory of LogFilePath must exist.
#[macro_export]
macro_rules! logger {
        ($($config:tt)*) => {{
                use bmw_log::LogConfig2_Options::*;
                let v: Vec<LogConfig2_Options> = vec![$($config)*];
                LogBuilder::build_log(v)
        }};
}
