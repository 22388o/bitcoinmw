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

#[document]
/// The [`crate::trace`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Trace`] level _or_ to log at the [`crate::LogLevel::Trace`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Trace`] for the
/// global logger. If arguments are supplied, the global logger will be called at the trace level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::trace_all
/// @see crate::trace_plain
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
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
///    // this will not be logged because the threshold is higher
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
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::trace_plain`] macro is identical to the [`crate::trace`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::trace`] macro, the [`crate::trace_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Trace`] level _or_ to log at the
/// [`crate::LogLevel::Trace`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied, the global logger will
/// be called at the trace level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::trace_all
/// @see crate::trace
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
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
///    // this will not be logged because the threshold is higher
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
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::trace_all`] macro is identical to the [`crate::trace`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::trace`],  the [`crate::trace_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Trace`] level _or_ to log at the
/// [`crate::LogLevel::Trace`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied, the global logger will
/// be called at the trace level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::trace_plain
/// @see crate::trace
/// @see crate::info
/// @see crate::Logger
#[macro_export]
macro_rules! trace_all {
        () => {
                 #[doc(hidden)]
                 const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::debug`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Debug`] level _or_ to log at the [`crate::LogLevel::Debug`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Debug`] for the
/// global logger. If arguments are supplied, the global logger will be called at the debug level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::debug_all
/// @see crate::debug_plain
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'debug'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'debug'
/// // threshold.
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     // log at the debug level. Since the threshold is debug, this will be logged.
///     debug!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     debug!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    info!();
///
///    // this will not be logged because the threshold is higher
///    debug!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (DEBUG) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (DEBUG) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! debug {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::debug_plain`] macro is identical to the [`crate::debug`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::debug`] macro, the [`crate::debug_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Debug`] level _or_ to log at the
/// [`crate::LogLevel::Debug`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Debug`] for the global logger. If arguments are supplied, the global logger will
/// be called at the debug level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::debug_all
/// @see crate::debug
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'debug'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'debug'
/// // threshold.
/// debug_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the debug level. Since the threshold is debug, this will be logged.
///     debug_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     debug_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    info!();
///
///    // this will not be logged because the threshold is higher
///    debug_plain!("will not show up")?;
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
macro_rules! debug_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::debug_all`] macro is identical to the [`crate::debug`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::debug`],  the [`crate::debug_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Debug`] level _or_ to log at the
/// [`crate::LogLevel::Debug`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Debug`] for the global logger. If arguments are supplied, the global logger will
/// be called at the debug level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::debug_plain
/// @see crate::debug
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'debug'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'debug' threshold.
/// debug_all!();
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
///         Stdout(false)
///     )?;
///
///     // log at the debug level. Since the threshold is debug, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if debug were called with this configuration, the line
///     // would not be printed to stdout.
///     debug_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     debug_all!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    info!();
///
///    // this will not be logged because the threshold is higher
///    debug_all!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (DEBUG) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (DEBUG) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! debug_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::info`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Info`] level _or_ to log at the [`crate::LogLevel::Info`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Info`] for the
/// global logger. If arguments are supplied, the global logger will be called at the info level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::info_all
/// @see crate::info_plain
/// @see crate::debug
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'info'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'info'
/// // threshold.
/// info!();
///
/// fn main() -> Result<(), Error> {
///     // log at the info level. Since the threshold is info, this will be logged.
///     info!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     info!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    warn!();
///
///    // this will not be logged because the threshold is higher
///    info!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]:  (INFO) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]:  (INFO) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! info {
	() => {
		#[doc(hidden)]
		const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
	};
	($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::info_plain`] macro is identical to the [`crate::info`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::info`] macro, the [`crate::info_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Info`] level _or_ to log at the
/// [`crate::LogLevel::Info`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Info`] for the global logger. If arguments are supplied, the global logger will
/// be called at the info level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::info_all
/// @see crate::info
/// @see crate::debug
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'info'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'info'
/// // threshold.
/// info_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the info level. Since the threshold is info, this will be logged.
///     info_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     info_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    warn!();
///
///    // this will not be logged because the threshold is higher
///    info_plain!("will not show up")?;
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
macro_rules! info_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::info_all`] macro is identical to the [`crate::info`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::info`],  the [`crate::info_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Info`] level _or_ to log at the
/// [`crate::LogLevel::Info`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Info`] for the global logger. If arguments are supplied, the global logger will
/// be called at the info level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::info_plain
/// @see crate::info
/// @see crate::debug
/// @see crate::Logger
#[macro_export]
macro_rules! info_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::warn`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Warn`] level _or_ to log at the [`crate::LogLevel::Warn`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Warn`] for the
/// global logger. If arguments are supplied, the global logger will be called at the warn level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::warn_all
/// @see crate::warn_plain
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'warn'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'warn'
/// // threshold.
/// warn!();
///
/// fn main() -> Result<(), Error> {
///     // log at the warn level. Since the threshold is warn, this will be logged.
///     warn!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     warn!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    error!();
///
///    // this will not be logged because the threshold is higher
///    warn!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]:  (WARN) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]:  (WARN) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! warn {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::warn_plain`] macro is identical to the [`crate::warn`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::warn`] macro, the [`crate::warn_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Warn`] level _or_ to log at the
/// [`crate::LogLevel::Warn`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Warn`] for the global logger. If arguments are supplied, the global logger will
/// be called at the warn level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::warn_all
/// @see crate::warn
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'warn'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'warn'
/// // threshold.
/// warn_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the warn level. Since the threshold is warn, this will be logged.
///     warn_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     warn_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    error!();
///
///    // this will not be logged because the threshold is higher
///    warn_plain!("will not show up")?;
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
macro_rules! warn_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::warn_all`] macro is identical to the [`crate::warn`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::warn`],  the [`crate::warn_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Warn`] level _or_ to log at the
/// [`crate::LogLevel::Warn`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Warn`] for the global logger. If arguments are supplied, the global logger will
/// be called at the warn level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::warn_plain
/// @see crate::warn
/// @see crate::info
/// @see crate::Logger
#[macro_export]
macro_rules! warn_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::error`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Error`] level _or_ to log at the [`crate::LogLevel::Error`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Error`] for the
/// global logger. If arguments are supplied, the global logger will be called at the error level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::error_all
/// @see crate::error_plain
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'error'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'error'
/// // threshold.
/// error!();
///
/// fn main() -> Result<(), Error> {
///     // log at the error level. Since the threshold is error, this will be logged.
///     error!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     error!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    fatal!();
///
///    // this will not be logged because the threshold is higher
///    error!("will not show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (ERROR) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (ERROR) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! error {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::error_plain`] macro is identical to the [`crate::error`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::error`] macro, the [`crate::error_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Error`] level _or_ to log at the
/// [`crate::LogLevel::Error`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Error`] for the global logger. If arguments are supplied, the global logger will
/// be called at the error level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::error_all
/// @see crate::error
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'error'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'error'
/// // threshold.
/// error_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the error level. Since the threshold is error, this will be logged.
///     error_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     error_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    fatal!();
///
///    // this will not be logged because the threshold is higher
///    error_plain!("will not show up")?;
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
macro_rules! error_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::error_all`] macro is identical to the [`crate::error`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::error`],  the [`crate::error_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Error`] level _or_ to log at the
/// [`crate::LogLevel::Error`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Error`] for the global logger. If arguments are supplied, the global logger will
/// be called at the error level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::error_plain
/// @see crate::error
/// @see crate::info
/// @see crate::Logger
#[macro_export]
macro_rules! error_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::fatal`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Fatal`] level _or_ to log at the [`crate::LogLevel::Fatal`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Logger::set_log_level`] with an argument of [`crate::LogLevel::Fatal`] for the
/// global logger. If arguments are supplied, the global logger will be called at the fatal level
/// and the formatted output will be logged if the threshold of the global logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::fatal_all
/// @see crate::fatal_plain
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'fatal'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'fatal'
/// // threshold.
/// fatal!();
///
/// fn main() -> Result<(), Error> {
///     // log at the fatal level. Since the threshold is fatal, this will be logged.
///     fatal!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     fatal!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///
///    // fatal will always be logged
///    fatal!("will still show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (FATAL) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (FATAL) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! fatal {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
}

#[document]
/// The [`crate::fatal_plain`] macro is identical to the [`crate::fatal`] macro except that just
/// the formatted log line is logged with no timestamp, log level, or line number. Just as with
/// the [`crate::fatal`] macro, the [`crate::fatal_plain`] macro can be used to set the global logging
/// level at the current scope to the [`crate::LogLevel::Fatal`] level _or_ to log at the
/// [`crate::LogLevel::Fatal`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Fatal`] for the global logger. If arguments are supplied, the global logger will
/// be called at the fatal level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::fatal_all
/// @see crate::fatal
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
///
/// // set the global logger's logging level to 'fatal'. Since it's outside of the function
/// // block, any logging that occurs for the rest of this file will use the 'fatal'
/// // threshold.
/// fatal_plain!();
///
/// fn main() -> Result<(), Error> {
///     // log at the fatal level. Since the threshold is fatal, this will be logged.
///     fatal_plain!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     fatal_plain!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///
///    // fatal is always logged
///    fatal_plain!("will show up")?;
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
macro_rules! fatal_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
}

#[document]
/// The [`crate::fatal_all`] macro is identical to the [`crate::fatal`] macro except that data
/// are logged both to stdout and a file (if configured) regardless of whether or not stdout logging
/// is enabled. Just as with [`crate::fatal`],  the [`crate::fatal_all`] macro can also be used
/// to set the global logging level at the current scope to the [`crate::LogLevel::Fatal`] level _or_ to log at the
/// [`crate::LogLevel::Fatal`] level depending on which arguments are passed to the macro. If no arguments
/// are supplied, it is the equivalent to calling [`crate::Logger::set_log_level`] with an argument of
/// [`crate::LogLevel::Fatal`] for the global logger. If arguments are supplied, the global logger will
/// be called at the fatal level and the formatted output will be logged if the threshold of the global
/// logger permits it.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::fatal_plain
/// @see crate::fatal
/// @see crate::info
/// @see crate::Logger
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'fatal'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'fatal' threshold.
/// fatal_all!();
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
///         Stdout(false)
///     )?;
///
///     // log at the fatal level. Since the threshold is fatal, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if fatal were called with this configuration, the line
///     // would not be printed to stdout.
///     fatal_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     fatal_all!("1 + 1 = {}", 2)?;
///
///     Ok(())
/// }
///
/// fn another_fn() -> Result<(), Error> {
///    // if you set the log level to a different threshold at a different scope, that
///    // value will overwrite the one at the outer scope.
///    debug!();
///
///    // fatal is always logged
///    fatal_all!("will show up")?;
///
///    Ok(())
/// }
///```
/// # Output of the above example
/// The output of the above example may look something like this:
///```text
/// [2024-04-14 17:45:46.899]: (FATAL) [..src/bin/http.rs:100]: this is a test
/// [2024-04-14 17:45:46.900]: (FATAL) [..src/bin/http.rs:103]: 1 + 1 = 2
///```
#[macro_export]
macro_rules! fatal_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::log_init`] macro initializes the global logger. It may only be called once and
/// must be called before any logging occurs. If the other logging macros are called before the
/// [`crate::log_init`] macro is called, a global logger will be initialized with the default
/// values as described in the [`crate::logger`] macro will be initialized. Calling log_init after
/// that point, will result in an error as seen below.
/// # Bad initialization Example
///```
/// use bmw_core::*;
/// use bmw_log::*;
/// use bmw_test::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///
///     // this call will automatically cal log_init with the default parameters
///     info!("starting logger up")?;
///
///     // so, now this will result in an error because the global logger has already been
///     // initialized.
///     assert!(log_init!(Colors(false)).is_err());
///     
///     Ok(())
/// }
///```
/// It is usually a good idea to initialize the global logger very early in the program to avoid
/// issues like these where log_init is called multiple times.
/// # Input Parameters
/// The input parameters to this macro may be any value in the [`crate::LogConstOptions`]
/// enumeration. These are:
/// * `MaxSizeBytes(u64)` - The maximum size, in bytes, at which this log's log file will be rotated.
/// The default value is u64::MAX.
/// * `MaxAgeMillis(u64)` - The maximum age, in milliseconds, at which this log's log file will be
/// rotated. The default value is u64::MAX.
/// * `LineNumDataMaxLen(u64)` - The maximum length, in bytes, of the line number data which, if
/// enabled, is included in the log line. The default value is 30.
/// * `LogFilePath(&str)` - The path of log file. If this value is set to "", file logging is
/// disabled. The default value is "".
/// * `FileHeader(&str)` - The header to include at the top of all log files. The default value is
/// "".
/// * `DisplayColors(bool)` - If set to true, colors are displayed to make the log lines easier to
/// read. Not that colors are only enabled on stdout.
/// * `Stdout(bool)` - If set to true, data are logged to stdout. The defaule value is true.
/// * `DisplayTimestamp(bool)` - If set to true, timestamps are displayed on each log line. The
/// default value is true.
/// * `DisplayLogLevel(bool)` - If set to true, log level is displayed on each log line. The default
/// value is true.
/// * `LineNum(bool)` - If set to true, line number data are displayed on each log line. The
/// default value is true.
/// * `DisplayMillis(bool)` - If set to true, millisecond data are displayed on each log line. The
/// default value is true.
/// * `DisplayBacktrace(bool)` - If set to true, a backtrace will be logged when data are logged at
/// the [`crate::LogLevel::Error`] or [`crate::LogLevel::Fatal`] level. The default value is false.
/// * `AutoRotate(bool)` - If set to true, log files are automatically rotated, but the rotation is
/// only checked at the time that data are logged. See [`crate::Log::rotate`] for further details.
/// The default value is false.
/// * `DeleteRotation(bool)` - If set to true, log files are immidately deleted upon log rotation.
/// This option is useful for long running tests where logging to disk may result in running out of
/// disk space. This value _MUST_ be set to false in a production environment. The default value is
/// false.
/// * `DebugResolveFrameError(bool)` - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// * `DebugInvalidMetadata(bool)` - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// * `DebugLinenoIsNone(bool)` - this is an option used in testing. This option _MUST_ be set to
/// false in a production environment. The default value is false.
/// @see crate::Logger
/// @see crate::logger
/// @see crate::info
/// @error crate::LogErrorKind::AlreadyInitialized if the log is already initialized.
/// @error crate::LogErrorKind::IllegalState if the log file cannot be read.
/// @error crate::LogErrorKind::MetaData if an error accessing the log file's metadata occurs.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
#[macro_export]
macro_rules! log_init {
        ($($config:tt)*) => {{
                use bmw_log::GlobalLogFunctions;
                #[allow(unused_imports)]
                use bmw_log::LogConstOptions::*;
                let v: Vec<LogConstOptions> = vec![$($config)*];
                GlobalLogFunctions::init(v)
        }};
}

#[document]
/// The [`crate::set_log_option`] macro calls the [`crate::Log::set_config_option`] function on the
/// global logger.
/// @error crate::LogErrorKind::NotInitialized if the log is not initialized.
/// @error crate::LogErrorKind::IllegalArgument if the value is invalid.
/// @error bmw_core::CoreErrorKind::IO if an i/o error occurs.
/// @see crate::logger
/// @see crate::Logger
#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		use bmw_log::GlobalLogFunctions;
		use bmw_log::LogConstOptions::*;
		GlobalLogFunctions::set_log_option($option)
	}};
}

#[document]
/// The [`crate::log_rotate`] macro calls the [`crate::Log::rotate`] function on the global logger.
/// This results in the global logger's log to be rotated if needed.
/// @see crate::need_rotate
/// @see crate::logger
/// @see crate::log_init
#[macro_export]
macro_rules! log_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::rotate()
	}};
}

#[document]
/// The [`crate::need_rotate`] macro calls the [`crate::Log::need_rotate`] function on the
/// underlying global logger and returns the [`bool`] value returned by that function.
/// @return [`true`] if the global logger needs to be rotated. Otherwise, returns [`false`].
/// @see crate::log_rotate
/// @see crate::logger
/// @see crate::log_init
#[macro_export]
macro_rules! need_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::need_rotate()
	}};
}
