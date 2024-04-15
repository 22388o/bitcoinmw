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

/// The [`crate::trace`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Trace`] level _or_ to log at the [`crate::LogLevel::Trace`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Trace`] for the
/// global logger. If arguments are supplied, the global logger will be called at the trace level.
/// # Input parameters
/// Either none or a the same parameters as the [`std::format`] macro.
/// # Return
/// [`unit`]
/// # Errors
/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
/// # Also see
/// [`crate::trace_all`]
///
/// [`crate::trace_plain`]
///
/// [`crate::info`]
///
/// [`crate::Log`]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'trace'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'trace'
/// // threshold.
/// trace!();
///
/// fn main() -> Result<(), Error> {
///     // log at the trace level. Since the threshold is trace, this will be logged.
///     trace!("this is a test")?;
///     
///     // formatting can be used just like println! and format!
///     trace!("1 + 1 = {}", 2)?;
///     
///     Ok(())
/// }   
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///    
///    // this will not be logged because the threshold is debug in this function
///    trace!("will not show up")?;
///    
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (TRACE) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (TRACE) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! trace {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                trace!(&format!($line, $($values)*)[..])
        };
}

/// The [`crate::trace_plain`] macro is identical to the [`crate::trace`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// [`crate::trace`], the [`crate::trace_plain`] macro can also be used to set the global logging level
/// at the current scope to the [`crate::LogLevel::Trace`] level _or_ to log at the
/// [`crate::LogLevel::Trace`] level depending on which arguments are passed to the macro.
/// If no arguments are supplied, it is the equivalent to calling [`crate::Log::set_log_level`]
/// with an argument of [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied,
/// the global logger will be called at the trace level.
/// # Input parameters
/// Either none or a the same parameters as the [`std::format`] macro.
/// # Return
/// [`unit`]
/// # Errors
/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
/// # Also see
/// [`crate::trace_all`]
///
/// [`crate::trace`]
///
/// [`crate::info`]
///
/// [`crate::Log`]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'trace'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'trace'
/// // threshold.
/// trace_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the trace level. Since the threshold is trace, this will be logged.
///     trace_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     trace_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///
///    // this will not be logged because the threshold is debug in this function
///    trace_plain!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// this is a test
/// 1 + 1 = 2
///```
#[macro_export]
macro_rules! trace_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_plain!(&format!($line, $($values)*)[..])
        };
}

/// The [`crate::trace_all`] macro is identical to the [`crate::trace`] macro except that data are
/// logged to both stdout and a file (if configured) regardless of whether or not stdout logging is
/// enabled. Just as with [`crate::trace`], the [`crate::trace_all`] macro can also be used to set the global logging level
/// at the current scope to the [`crate::LogLevel::Trace`] level _or_ to log at the
/// [`crate::LogLevel::Trace`] level depending on which arguments are passed to the macro.
/// If no arguments are supplied, it is the equivalent to calling [`crate::Log::set_log_level`]
/// with an argument of [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied,
/// the global logger will be called at the trace level.
/// # Input parameters
/// Either none or a the same parameters as the [`std::format`] macro.
/// # Return
/// [`unit`]
/// # Errors
/// [`bmw_err::ErrKind::IO`] - if an i/o error occurs.
/// # Also see
/// [`crate::trace_plain`]
///
/// [`crate::trace`]
///
/// [`crate::info`]
///
/// [`crate::Log`]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'trace'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'trace' threshold.
/// trace_all!();
///
/// fn main() -> Result<(), Error> {
///     // get test info so we can get a temporary directory name
///     let test_info = test_info!()?;
///
///     // create a logger that logs to a log file only
///     let mut path_buf = PathBuf::from(test_info.directory());
///     path_buf.push("test.log");
///     let path = path_buf.display().to_string();
///     // call log_init to initialize the global logger
///     log_init!(
///         LogFilePath(&path),
///         DisplayStdout(false)
///     )?;
///
///     // log at the trace level. Since the threshold is trace, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if trace were called with this configuration, the line
///     // would not be printed to stdout.
///     trace_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     trace_all!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///
///    // this will not be logged because the threshold is debug in this function
///    trace_all!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (TRACE) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (TRACE) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! trace_all {
        () => {
                 #[doc(hidden)]
                 const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                debug!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                info!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                info_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                info_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                warn!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                error!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                error_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                error_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! log_init {
        ($($config:tt)*) => {{
                use bmw_log::GlobalLogFunctions;
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                GlobalLogFunctions::init(v)
        }};
}

#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		use bmw_log::GlobalLogFunctions;
		use bmw_log::LogConfigOptions::*;
		GlobalLogFunctions::set_log_option($option)
	}};
}

#[macro_export]
macro_rules! log_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::rotate()
	}};
}

#[macro_export]
macro_rules! need_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::need_rotate()
	}};
}

/// The [`crate::logger`] macro is used to create a logger. This logger is distinct from the global
/// logger which can be invoked through the various macros provided in this library. This macro
/// returns an independent instance that is not affected by the other macro calls.
/// # Input Parameters
/// The input parameters to this macro may be any value in the [`crate::LogConfigOptions`]
/// enumeration. These are:
/// * MaxSizeBytes(u64) - The maximum size, in bytes, at which this log's log file will be rotated.
/// The default value is u64::MAX.
/// * MaxAgeMillis(u64) - The maximum age, in milliseconds, at which this log's log file will be
/// rotated. The default value is u64::MAX.
/// * LineNumDataMaxLen(u64) - The maximum length, in bytes, of the line number data which, if
/// enabled, is included in the log line. The default value is 30.
/// * LogFilePath(&str) - The path of log file. If this value is set to "", file logging is
/// disabled. The default value is "".
/// * FileHeader(&str) - The header to include at the top of all log files. The default value is
/// "".
/// * DisplayColors(bool) - If set to true, colors are displayed to make the log lines easier to
/// read. Not that colors are only enabled on stdout.
/// * DisplayStdout(bool) - If set to true, data are logged to stdout. The defaule value is true.
/// * DisplayTimestamp(bool) - If set to tru, timestamps are displayed on each log line. The
/// default value is true.
/// * DisplayLogLevel(bool) - If set to true, log level is displayed on each log line. The default
/// value is true.
/// * DisplayLineNum(bool) - If set to true, line number data are displayed on each log line. The
/// default value is true.
/// * DisplayMillis(bool) - If set to true, millisecond data are displayed on each log line. The
/// default value is true.
/// * DisplayBacktrace(bool) - If set to true, a backtrace will be logged when data are logged at
/// the [`crate::LogLevel::Error`] or [`crate::LogLevel::Fatal`] level. The default value is false.
/// * DeleteRotation(bool) - If set to true, log files are immidately deleted upon log rotation.
/// This option is useful for long running tests where logging to disk may result in running out of
/// disk space. This value _MUST_ be set to false in a production environment. The default value is
/// false.
/// * AutoRotate(bool) - If set to true, log files are automatically rotated, but the rotation is
/// only checked at the time that data are logged. See [`crate::Log::rotate`] for further details.
/// The default value is false.
/// * DebugResolveFrameError(bool) - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// * DebugInvalidMetadata(bool) - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// * DebugLinenoIsNone(bool) - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// # Return
/// The macro calls [`crate::LogBuilder::build_log`]. So, the returned value is a `Box<dyn Log +
/// Send + Sync>`.
/// # Errors
/// [`bmw_err::ErrKind::Configuration`] - if a configuration option is specified more than once.
///
/// [`bmw_err::ErrKind::IO`] - if the file cannot be accessed.
///
/// [`bmw_err::ErrKind::Log`] - if the MaxAgeMillis option is set to less than 1,000.
///
/// [`bmw_err::ErrKind::Log`] - if the MaxSizeBytes option is set to less than 50.
///
/// [`bmw_err::ErrKind::Log`] - if the LineNumDataMaxLen option is set to less than 10.
/// # Also see
/// [`crate::Log`]
///
/// [`crate::info`]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// fn main() -> Result<(), Error> {
///     let test_info = test_info!(true)?;
///     let directory = test_info.directory();
///
///     let mut path1 = PathBuf::from(directory);
///     let mut path2 = path1.clone();
///     let mut path3 = path1.clone();
///
///     path1.push("log1.log");
///     path2.push("log2.log");
///     path3.push("log3.log");
///
///     let path1 = path1.display().to_string();
///     let path2 = path2.display().to_string();
///     let path3 = path3.display().to_string();
///
///     let mut logger1 = logger!(LogFilePath(&path1))?;
///     let mut logger2 = logger!(LogFilePath(&path2))?;
///     let mut logger3 = logger!(LogFilePath(&path3))?;
///
///     logger1.init()?;
///     logger2.init()?;
///     logger3.init()?;
///
///     logger1.set_log_level(LogLevel::Info);
///     logger2.set_log_level(LogLevel::Debug);
///     logger3.set_log_level(LogLevel::Trace);
///
///     logger1.log(LogLevel::Trace, "test")?;
///     logger2.log(LogLevel::Trace, "test")?;
///     logger3.log(LogLevel::Trace, "test")?;
///
///     logger1.log(LogLevel::Debug, "test")?;
///     logger2.log(LogLevel::Debug, "test")?;
///     logger3.log(LogLevel::Debug, "test")?;
///
///     logger1.log(LogLevel::Info, "test")?;
///     logger2.log(LogLevel::Info, "test")?;
///     logger3.log(LogLevel::Info, "test")?;
///
///     logger1.log(LogLevel::Warn, "test")?;
///     logger2.log(LogLevel::Warn, "test")?;
///     logger3.log(LogLevel::Warn, "test")?;
///
///     logger1.log(LogLevel::Error, "test")?;
///     logger2.log(LogLevel::Error, "test")?;
///     logger3.log(LogLevel::Error, "test")?;
///
///     logger1.log(LogLevel::Fatal, "test")?;
///     logger2.log(LogLevel::Fatal, "test")?;
///     logger3.log(LogLevel::Fatal, "test")?;
///
///     Ok(())
/// }
///```
/// # Output of the above example
#[macro_export]
macro_rules! logger {
        ($($config:tt)*) => {{
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                LogBuilder::build_log(v)
        }};
}
