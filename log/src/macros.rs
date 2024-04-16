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

use bmw_derive::*;

#[document]
/// The [`crate::trace`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Trace`] level _or_ to log at the [`crate::LogLevel::Trace`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Trace`] for the
/// global logger. If arguments are supplied, the global logger will be called at the trace level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Trace`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::trace_all")]
#[add_doc(see: "crate::trace_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied, the global logger will
/// be called at the trace level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Trace`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::trace_all")]
#[add_doc(see: "crate::trace")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Trace`] for the global logger. If arguments are supplied, the global logger will
/// be called at the trace level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Trace`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::trace_plain")]
#[add_doc(see: "crate::trace")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
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
///    // this will not be logged because the threshold is higher
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
        ($($values:tt)*) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, &format!($($values)*)[..], BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
}

#[document]
/// The [`crate::debug`] macro is used to set the global logging level at the current scope to the
/// [`crate::LogLevel::Debug`] level _or_ to log at the [`crate::LogLevel::Debug`] level depending on
/// which arguments are passed to the macro. If no arguments are supplied, it is the equivalent to
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Debug`] for the
/// global logger. If arguments are supplied, the global logger will be called at the debug level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Debug`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::debug_all")]
#[add_doc(see: "crate::debug_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Debug`] for the global logger. If arguments are supplied, the global logger will
/// be called at the debug level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Debug`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::debug_all")]
#[add_doc(see: "crate::debug")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Debug`] for the global logger. If arguments are supplied, the global logger will
/// be called at the debug level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Debug`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::debug_plain")]
#[add_doc(see: "crate::debgu")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
///         DisplayStdout(false)
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
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Info`] for the
/// global logger. If arguments are supplied, the global logger will be called at the info level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Info`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::info_all")]
#[add_doc(see: "crate::info_plain")]
#[add_doc(see: "crate::debug")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Info`] for the global logger. If arguments are supplied, the global logger will
/// be called at the info level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Info`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::info_all")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::debug")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Info`] for the global logger. If arguments are supplied, the global logger will
/// be called at the info level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Info`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::info_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::debug")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'info'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'info' threshold.
/// info_all!();
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
///     // log at the info level. Since the threshold is info, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if info were called with this configuration, the line
///     // would not be printed to stdout.
///     info_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     info_all!("1 + 1 = {}", 2)?;
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
///    info_all!("will not show up")?;
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
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Warn`] for the
/// global logger. If arguments are supplied, the global logger will be called at the warn level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Warn`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::warn_all")]
#[add_doc(see: "crate::warn_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Warn`] for the global logger. If arguments are supplied, the global logger will
/// be called at the warn level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Warn`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::warn_all")]
#[add_doc(see: "crate::warn")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Warn`] for the global logger. If arguments are supplied, the global logger will
/// be called at the warn level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Warn`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::warn_plain")]
#[add_doc(see: "crate::warn")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'warn'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'warn' threshold.
/// warn_all!();
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
///     // log at the warn level. Since the threshold is warn, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if warn were called with this configuration, the line
///     // would not be printed to stdout.
///     warn_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     warn_all!("1 + 1 = {}", 2)?;
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
///    warn_all!("will not show up")?;
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
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Error`] for the
/// global logger. If arguments are supplied, the global logger will be called at the error level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Error`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::error_all")]
#[add_doc(see: "crate::error_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Error`] for the global logger. If arguments are supplied, the global logger will
/// be called at the error level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Error`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::error_all")]
#[add_doc(see: "crate::error")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Error`] for the global logger. If arguments are supplied, the global logger will
/// be called at the error level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Error`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::error_plain")]
#[add_doc(see: "crate::error")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// // set the global logger's logging level to 'error'. Since it's outside of the
/// // function block, any logging that occurs for the rest of this file will use
/// // the 'error' threshold.
/// error_all!();
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
///     // log at the error level. Since the threshold is error, this will be
///     // logged. Additionally this will be logged to both the file and stdout
///     // whereas if error were called with this configuration, the line
///     // would not be printed to stdout.
///     error_all!("this is a test")?;
///
///     // formatting can be used just like println! and format!
///     error_all!("1 + 1 = {}", 2)?;
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
///    error_all!("will not show up")?;
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
/// calling [`crate::Log::set_log_level`] with an argument of [`crate::LogLevel::Fatal`] for the
/// global logger. If arguments are supplied, the global logger will be called at the fatal level
/// and the formatted output will be logged if the threshold of the global logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Fatal`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::fatal_all")]
#[add_doc(see: "crate::fatal_plain")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Fatal`] for the global logger. If arguments are supplied, the global logger will
/// be called at the fatal level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Fatal`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::fatal_all")]
#[add_doc(see: "crate::fatal")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
/// are supplied, it is the equivalent to calling [`crate::Log::set_log_level`] with an argument of
/// [`crate::LogLevel::Fatal`] for the global logger. If arguments are supplied, the global logger will
/// be called at the fatal level and the formatted output will be logged if the threshold of the global
/// logger permits it.
#[add_doc(input: values - "(optional) - if specified, the values are logged as if they were", "($($values:tt)*)")]
#[add_doc(input: values - "parameters sent to [`std::format`]. If none are specified, the log")]
#[add_doc(input: values - "level for this scope is set to [`crate::LogLevel::Fatal`].")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if an i/o error occurs")]
#[add_doc(see: "crate::fatal_plain")]
#[add_doc(see: "crate::fatal")]
#[add_doc(see: "crate::info")]
#[add_doc(see: "crate::Log")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
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
///         DisplayStdout(false)
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
/// use bmw_err::*;
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
///     assert!(log_init!(DisplayColors(false)).is_err());
///     
///     Ok(())
/// }
///```
/// It is usually a good idea to initialize the global logger very early in the program to avoid
/// issues like these where log_init is called multiple times.
/// # Input Parameters
/// The input parameters to this macro may be any value in the [`crate::LogConfigOptions`]
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
/// * `DisplayStdout(bool)` - If set to true, data are logged to stdout. The defaule value is true.
/// * `DisplayTimestamp(bool)` - If set to true, timestamps are displayed on each log line. The
/// default value is true.
/// * `DisplayLogLevel(bool)` - If set to true, log level is displayed on each log line. The default
/// value is true.
/// * `DisplayLineNum(bool)` - If set to true, line number data are displayed on each log line. The
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
#[add_doc(error: "bmw_err::ErrKind::Configuration", "if a configuration option is specified more than once")]
#[add_doc(error: "bmw_err::ErrKind::IO", "if the file cannot be accessed")]
#[add_doc(error: "bmw_err::ErrKind::Log", "if the MaxAgeMillis option is set to less than 1,000")]
#[add_doc(error: "bmw_err::ErrKind::Log", "if the MaxSizeBytes option is set to less than 50")]
#[add_doc(error: "bmw_err::ErrKind::Log", "if the LineNumDataMaxLen option is set to less than 10")]
#[add_doc(return: "On success, a [`crate::Log`] is returned", " Result < Box < dyn Log + Send + Sync , Error >")]
#[add_doc(see: "crate::Log")]
#[add_doc(see: "crate::logger")]
#[add_doc(see: "crate::info")]
#[add_doc(doc_point)]
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let test_info = test_info!()?;
///     let mut path = PathBuf::from(test_info.directory());
///     path.push("test.log");
///     let path = path.display().to_string();
///
///     // first call log_init
///     log_init!(DisplayColors(false), DisplayLineNum(false), LogFilePath(&path))?;
///
///     // then log with the global logger macros.
///     info!("Logger initialized!")?;
///
///     spawn(move || -> Result<(), Error> {
///         info!("global logger can be used in other threads")?;
///         Ok(())
///     });
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! log_init {
        ($($config:tt)*) => {{
                use bmw_log::GlobalLogFunctions;
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                GlobalLogFunctions::init(v)
        }};
}

#[document]
/// The [`crate::set_log_option`] macro calls the [`crate::Log::set_config_option`] function on the
/// global logger.
#[add_doc(input: option - "The LogConfigOptions to set. After initialization, most of the", " crate::LogConfigOptions ")]
#[add_doc(input: option - "configuration settings may be changed. The only exception is")]
#[add_doc(input: option - "[`crate::LogConfigOptions::LogFilePath`]. Attempting to set this option will result in an error")]
#[add_doc(error: "bmw_err::ErrKind::Log" - "if the log is not initialized.")]
#[add_doc(error: "bmw_err::ErrKind::Log" - "if `option` is a [`crate::LogConfigOptions::LogFilePath`].")]
#[add_doc(see: "crate::logger")]
#[add_doc(see: "crate::LogConfigOptions")]
#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		use bmw_log::GlobalLogFunctions;
		use bmw_log::LogConfigOptions::*;
		GlobalLogFunctions::set_log_option($option)
	}};
}

#[document]
/// The [`crate::log_rotate`] macro calls the [`crate::Log::rotate`] function on the global logger.
/// This results in the global logger's log to be rotated if needed.
#[add_doc(see: "crate::need_rotate")]
#[add_doc(see: "crate::logger")]
#[add_doc(see: "crate::log_init")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let test_info = test_info!()?;
///     let mut path = PathBuf::from(test_info.directory());
///     path.push("test.log");
///     let path = path.display().to_string();
///
///     // init the logger with a 100 byte size limit
///     log_init!(MaxSizeBytes(100), LogFilePath(&path), AutoRotate(false))?;
///
///     // log  few lines
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///
///     // a rotation should be needed at this point
///     assert!(need_rotate!()?);
///
///     // do the rotation
///     log_rotate!()?;
///
///     // now a rotation is not needed
///     assert!(!need_rotate!()?);
///
///     Ok(())
/// }
///```
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
#[add_doc(return: "[`true`] if the global logger needs to be rotated. Otherwise, returns [`false`].", " bool ")]
#[add_doc(see: "crate::log_rotate")]
#[add_doc(see: "crate::logger")]
#[add_doc(see: "crate::log_init")]
#[add_doc(doc_point)]
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use std::path::PathBuf;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let test_info = test_info!()?;
///     let mut path = PathBuf::from(test_info.directory());
///     path.push("test.log");
///     let path = path.display().to_string();
///
///     // init the logger with a 100 byte size limit
///     log_init!(MaxSizeBytes(100), LogFilePath(&path), AutoRotate(false))?;
///
///     // log  few lines
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///     info!("01234567890")?;
///
///     // a rotation should be needed at this point
///     assert!(need_rotate!()?);
///
///     // do the rotation
///     log_rotate!()?;
///
///     // now a rotation is not needed
///     assert!(!need_rotate!()?);
///
///     Ok(())
/// }
///```
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
/// * `DisplayStdout(bool)` - If set to true, data are logged to stdout. The defaule value is true.
/// * `DisplayTimestamp(bool)` - If set to true, timestamps are displayed on each log line. The
/// default value is true.
/// * `DisplayLogLevel(bool)` - If set to true, log level is displayed on each log line. The default
/// value is true.
/// * `DisplayLineNum(bool)` - If set to true, line number data are displayed on each log line. The
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
///     let mut logger1 = logger!(
///         LogFilePath(&path1),
///         DisplayMillis(false),
///         DisplayTimestamp(true),
///         DisplayLineNum(true),
///         DisplayLogLevel(true)
///     )?;
///     let mut logger2 = logger!(
///         LogFilePath(&path2),
///         DisplayTimestamp(true),
///         DisplayLineNum(true),
///         DisplayLogLevel(false)
///     )?;
///     let mut logger3 = logger!(
///         LogFilePath(&path3),
///         DisplayTimestamp(true),
///         DisplayLineNum(false),
///         DisplayLogLevel(true)
///     )?;
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
///     logger1.log_plain(LogLevel::Warn, "test")?;
///     logger2.log_plain(LogLevel::Warn, "test")?;
///     logger3.log_plain(LogLevel::Warn, "test")?;
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
/// The output of log1.log may look something like this:
///```text
/// [2024-04-14 23:36:28] (INFO)  [..bitcoinmw/log/src/test.rs:1084]: test
/// test plain
/// [2024-04-14 23:36:28] (ERROR) [..bitcoinmw/log/src/test.rs:1092]: test
/// [2024-04-14 23:36:28] (FATAL) [..bitcoinmw/log/src/test.rs:1096]: test
///```
#[macro_export]
macro_rules! logger {
        ($($config:tt)*) => {{
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                LogBuilder::build_log(v)
        }};
}
