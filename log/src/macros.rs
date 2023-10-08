// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::Log;
use bmw_deps::lazy_static::lazy_static;
use std::cell::UnsafeCell;
use std::sync::{Arc, RwLock};

/// Internal struct used for global logger
#[doc(hidden)]
pub struct LogHolder {
	pub log: Option<Box<dyn Log + Send + Sync>>,
}

lazy_static! {
	#[doc(hidden)]
	pub static ref STATIC_LOG: Arc<RwLock<Option<Box<dyn Log + Send + Sync>>>> = Arc::new(RwLock::new(None));
}

thread_local! {
	#[doc(hidden)]
	pub static LOG_REF: UnsafeCell<LogHolder> =  UnsafeCell::new(LogHolder {log: None});
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
		bmw_log::log!(bmw_log::LogLevel::Fatal);
	};
	($a:expr) => {
		{
                        bmw_log::log!(bmw_log::LogLevel::Fatal, $a)
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        bmw_log::log!(bmw_log::LogLevel::Fatal, $a, $($b)*)
		}
	};
}

/// Same as [`fatal`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! fatal_plain {
	($a:expr) => {
		{
			bmw_log::log_plain!(bmw_log::LogLevel::Fatal, $a)
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
			bmw_log::log_plain!(bmw_log::LogLevel::Fatal, $a, $($b)*)
		}
	};
}

/// Same as [`fatal`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! fatal_all {
	($a:expr) => {
		{
			bmw_log::log_all!(bmw_log::LogLevel::Fatal, $a)
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        bmw_log::log_all!(bmw_log::LogLevel::Fatal, $a, $($b)*)
		}
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
		bmw_log::log!(bmw_log::LogLevel::Error);
	};
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log!(bmw_log::LogLevel::Error, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log!(bmw_log::LogLevel::Error, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`error`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! error_plain {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log_plain!(bmw_log::LogLevel::Error, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log_plain!(bmw_log::LogLevel::Error, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`error`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! error_all {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log_all!(bmw_log::LogLevel::Error, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal {
				bmw_log::log_all!(bmw_log::LogLevel::Error, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
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
		bmw_log::log!(bmw_log::LogLevel::Warn);
	};
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log!(bmw_log::LogLevel::Warn, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log!(bmw_log::LogLevel::Warn, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`warn`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! warn_plain {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log_plain!(bmw_log::LogLevel::Warn, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log_plain!(bmw_log::LogLevel::Warn, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`warn`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! warn_all {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log_all!(bmw_log::LogLevel::Warn, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error {
				bmw_log::log_all!(bmw_log::LogLevel::Warn, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
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
		bmw_log::log!(bmw_log::LogLevel::Info);
	};
	($a:expr) => {
		{

                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log!(bmw_log::LogLevel::Info, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log!(bmw_log::LogLevel::Info, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`info`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! info_plain {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log_plain!(bmw_log::LogLevel::Info, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log_plain!(bmw_log::LogLevel::Info, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`info`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! info_all {
	($a:expr) => {
                {
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log_all!(bmw_log::LogLevel::Info, $a)
                        } else {
                                Ok(())
                        }
                }
        };
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn {
				bmw_log::log_all!(bmw_log::LogLevel::Info, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
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
		bmw_log::log!(bmw_log::LogLevel::Debug);
	};
	($a:expr) => {
		{
                    if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
				bmw_log::log!(bmw_log::LogLevel::Debug, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                    if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
				bmw_log::log!(bmw_log::LogLevel::Debug, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`debug`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! debug_plain {
	($a:expr) => {
		{
                    if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
				bmw_log::log_plain!(bmw_log::LogLevel::Debug, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
				bmw_log::log_plain!(bmw_log::LogLevel::Debug, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`debug`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! debug_all {
	($a:expr) => {
		{
                    if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
				bmw_log::log_all!(bmw_log::LogLevel::Debug, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info {
                                bmw_log::log_all!(bmw_log::LogLevel::Debug, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
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
		    bmw_log::log!(bmw_log::LogLevel::Trace);
	};
	($a:expr) => {
		{
			if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
                                bmw_log::log!(bmw_log::LogLevel::Trace, $a)
			} else {
				Ok(())
			}
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
			        bmw_log::log!(bmw_log::LogLevel::Trace, $a, $($b)*)
			} else {
		                Ok(())
                        }
		}
	};
}

/// Same as [`trace`] except that the [`crate::Log::log_plain`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! trace_plain {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
				bmw_log::log_plain!(bmw_log::LogLevel::Trace, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
				bmw_log::log_plain!(bmw_log::LogLevel::Trace, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// Same as [`trace`] except that the [`crate::Log::log_all`] function of the underlying logger
/// is called instead of the the [`crate::Log::log`] function. See the [`crate::Log`] trait for
/// details on each.
#[macro_export]
macro_rules! trace_all {
	($a:expr) => {
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
				bmw_log::log_all!(bmw_log::LogLevel::Trace, $a)
                        } else {
                                Ok(())
                        }
		}
	};
	($a:expr,$($b:tt)*)=>{
		{
                        if LOG_LEVEL != bmw_log::LogLevel::Fatal && LOG_LEVEL != bmw_log::LogLevel::Error && LOG_LEVEL != bmw_log::LogLevel::Warn && LOG_LEVEL != bmw_log::LogLevel::Info && LOG_LEVEL != bmw_log::LogLevel::Debug {
				bmw_log::log_all!(bmw_log::LogLevel::Trace, $a, $($b)*)
                        } else {
                                Ok(())
                        }
		}
	};
}

/// This macro is called by the [`fatal`], [`error`], [`warn`], [`info`], [`debug`] and [`trace`] macros to process logging.
/// Generally those macros should be used in favor of calling this one directly.
#[macro_export]
macro_rules! log {
        ($level:expr) => {
                const LOG_LEVEL: bmw_log::LogLevel = $level;
        };
        ($level:expr, $a:expr) => {{
                bmw_log::do_log!(0, $level, $a)
        }};
        ($level:expr, $a:expr, $($b:tt)*) => {
                bmw_log::do_log!(0, $level, $a, $($b)*)
        }
}

/// This macro is called by the [`fatal_plain`], [`error_plain`], [`warn_plain`], [`info_plain`], [`debug_plain`] and [`trace_plain`]
/// macros to process logging. Generally those macros should be used in favor of calling this one directly.
#[macro_export]
macro_rules! log_plain {
        ($level:expr) => {
                const LOG_LEVEL: bmw_log::LogLevel = $level;
        };
        ($level:expr, $a:expr) => {{
                bmw_log::do_log!(1, $level, $a)
        }};
        ($level:expr, $a:expr, $($b:tt)*) => {
                bmw_log::do_log!(1, $level, $a, $($b)*)
        }
}

/// This macro is called by the [`fatal_all`], [`error_all`], [`warn_all`], [`info_all`], [`debug_all`] and [`trace_all`] macros
/// to process logging. Generally those macros should be used in favor of calling this one directly.
#[macro_export]
macro_rules! log_all {
        ($level:expr) => {
                const LOG_LEVEL: bmw_log::LogLevel = $level;
        };
        ($level:expr, $a:expr) => {{
                bmw_log::do_log!(2, $level, $a)
        }};
        ($level:expr, $a:expr, $($b:tt)*) => {
                bmw_log::do_log!(2, $level, $a, $($b)*)
        }
}

/// This macro is called by the [`fatal`], [`error`], [`warn`], [`info`], [`debug`] and [`trace`] macros and the 'all' / 'plain'
/// macros to process logging. Generally those macros should be used in favor of calling this one directly.
#[macro_export]
macro_rules! do_log {
	($flavor:expr, $level:expr, $a:expr) => {{
                let ret = bmw_log::LOG_REF.with(|f| -> Option<Result<(), bmw_err::Error>> {
                        let mut r = unsafe { f.get().as_mut() };
                        if r.as_mut().unwrap().log.is_some() {
                                let ret = if $flavor == 0 {
                                        r.as_mut().unwrap().log.as_mut().unwrap().log($level, $a, None)
                                } else if $flavor == 1 {
                                        r.as_mut().unwrap().log.as_mut().unwrap().log_plain($level, $a, None)
                                } else {
                                        r.as_mut().unwrap().log.as_mut().unwrap().log_all($level, $a, None)
                                };
                                Some(ret)
                        } else {
                                None
                        }
                });

                if ret.is_some() {
                        ret.unwrap()
                } else {
                match bmw_log::lockw!(bmw_log::STATIC_LOG) {
                    Ok(mut static_log) => {
                        bmw_log::LOG_REF.with(|f| {
                                let mut r = unsafe { f.get().as_mut() };
                                match (*static_log).as_mut() {
                                    Some(log) => {
                                        r.as_mut().unwrap().log = Some(log.clone());
                                    }
                                    None => {}
                                };
                        });

                        let (ret, log) = match (*static_log).as_mut() {
                            Some(log) => {
                                if $flavor == 0 {
                                    (log.log($level, $a, None), None)
                                } else if $flavor == 1 {
                                    (log.log_plain($level, $a, None), None)
                                } else {
                                    (log.log_all($level, $a, None), None)
                                }
                            },
                            None => {
                                match bmw_log::LogBuilder::build_send_sync(bmw_log::LogConfig::default()) {
                                    Ok(mut log) => {
                                        match log.init() {
                                            Ok(_) => {

                                                bmw_log::LOG_REF.with(|f| {
                                                        let mut r = unsafe { f.get().as_mut() };
                                                        r.as_mut().unwrap().log = Some(log.clone());
                                                });


                                                if $flavor == 0 {
                                                    (log.log($level, $a, None), Some(log))
                                                } else if $flavor == 1 {
                                                    (log.log_plain($level, $a, None), Some(log))
                                                } else {
                                                    (log.log_all($level, $a, None), Some(log))
                                                }
                                            }
                                            Err(e) => {
                                                (
                                                    Err(
                                                        bmw_err::err!(
                                                            bmw_err::ErrKind::Log,
                                                            format!(
                                                                "error initializing log: {}",
                                                                e
                                                            )
                                                        )
                                                    ),
                                                    None
                                                )
                                            },
                                        }
                                    },
                                    Err(e) => {
                                        (Err(e), None)
                                    },
                                }
                            }
                        };

                        if log.is_some() {
                            (*static_log) = log;
                        }

                        bmw_log::LOG_REF.with(|f| {
                                let mut r = unsafe { f.get().as_mut() };
                                r.as_mut().unwrap().log = None;
                        });
                    ret
                },
                Err(e) => Err(e),
            }
                }
        }};
        ($flavor:expr, $level:expr, $a:expr, $($b:tt)*) => {
                do_log!($flavor, $level, &format!($a, $($b)*)[..])
        };
}

/// Get the current value of the specified log option. The single parameter must be of the
/// type [`crate::LogConfigOptionName`]. The macro returns a [`crate::LogConfigOption`] on
/// success and [`bmw_err::Error`] on error. See [`crate::Log::get_config_option`] which
/// is the underlying function call for full details.
///
/// # Examples
///
///```
/// use bmw_err::Error;
/// use bmw_log::*;
///
/// debug!();
///
/// fn test() -> Result<(), Error> {
///
///     // first init the logger
///     log_init!(LogConfig::default());
///     // get the configured value for FilePath
///     let v = get_log_option!(LogConfigOptionName::FilePath)?;
///     // It should be the default value which is None
///     assert_eq!(v, LogConfigOption::FilePath(None));
///     info!("file_path={:?}", v)?;
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! get_log_option {
	($option:expr) => {{
		match bmw_log::lockr!(bmw_log::STATIC_LOG) {
			Ok(static_log) => match (*static_log).as_ref() {
				Some(log) => match log.get_config_option($option) {
					Ok(x) => Ok(x.clone()),
					Err(e) => Err(bmw_err::err!(
						bmw_err::ErrKind::Log,
						format!("log get_config_option returned error: {}", e)
					)),
				},
				None => Err(bmw_err::err!(bmw_err::ErrKind::Log, "log not initialized")),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::Poison,
				format!("could not obtain static_log lock: {}", e)
			)),
		}
	}};
}

/// Configure the log with the specified [`crate::LogConfigOption`]. This macro takes
/// a single argument. The macro returns () on success or [`bmw_err::Error`] on failure.
/// See [`crate::Log::set_config_option`] which is the underlying function call for
/// full details.
#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		match bmw_log::lockw!(bmw_log::STATIC_LOG) {
			Ok(mut static_log) => match (*static_log).as_mut() {
				Some(log) => log.set_config_option($option),
				None => Err(bmw_err::err!(bmw_err::ErrKind::Log, "log not initialized")),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::Poison,
				format!("could not obtain static_log lock: {}", e)
			)),
		}
	}};
}

/// Initialize the global log. This macro takes a single parameter, if none are
/// specified, the default [`crate::LogConfig`] is used. Note that if this macro
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
/// use bmw_log::LogConfigOption::*;
/// use std::path::PathBuf;
///
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     let main_log_path = "./main.log";
///     log_init!(LogConfig {
///         show_bt: ShowBt(false),
///         show_millis: ShowMillis(false),
///         // comment this line out to avoid tests actually creating this file
///         // file_path: FilePath(Some(PathBuf::from(main_log_path))),
///         ..Default::default()
///     });
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
/// use bmw_log::LogConfigOption::*;
///
/// debug!();
///
/// fn main() -> Result<(), Error> {
///     info!("Startup complete!")?;
///     
///     Ok(())
/// }
///```
///
/// Note that in the last example, the default [`crate::LogConfig`] will be used.
#[macro_export]
macro_rules! log_init {
	() => {{
		let config = bmw_log::LogConfig::default();
		log_init!(config)
	}};
	($config:expr) => {
		match bmw_log::lockw!(bmw_log::STATIC_LOG) {
			Ok(mut static_log) => {
				if (*static_log).is_some() {
					Err(bmw_err::err!(bmw_err::ErrKind::Log, "already initialized"))
				} else {
					match bmw_log::LogBuilder::build_send_sync($config) {
						Ok(mut log) => {
							log.init().unwrap();
							*static_log = Some(log);
							Ok(())
						}
						Err(e) => Err(bmw_err::err!(
							bmw_err::ErrKind::Log,
							format!("error building log: {}", e)
						)),
					}
				}
			}
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::Poison,
				format!("could not obtain static logger lock: {}", e)
			)),
		}
	};
}

/// Rotate the global log. See [`crate::Log::rotate`] for full details on
/// the underlying rotate function and log rotation in general.
#[macro_export]
macro_rules! log_rotate {
	() => {{
		match bmw_log::lockw!(bmw_log::STATIC_LOG) {
			Ok(mut static_log) => match (*static_log).as_mut() {
				Some(log) => log.rotate(),
				None => Err(bmw_err::err!(bmw_err::ErrKind::Log, "log not initialized")),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::Poison,
				format!("couldn't obtain lock for global log: {}", e)
			)),
		}
	}};
}

/// See if the global log needs to be rotated. See [`crate::Log::need_rotate`] for full details
/// on the underlying need_rotate function.
#[macro_export]
macro_rules! need_rotate {
	() => {{
		match bmw_log::lockr!(bmw_log::STATIC_LOG) {
			Ok(static_log) => match (*static_log).as_ref() {
				Some(log) => log.need_rotate(None),
				None => Err(bmw_err::err!(bmw_err::ErrKind::Log, "log not initialized")),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::Poison,
				format!("couldn't obtain lock for global log: {}", e)
			)),
		}
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! lockw {
	($a:expr) => {{
		let res = $a.write().map_err(|e| {
			let error: bmw_err::Error =
				bmw_err::ErrorKind::Poison(format!("Poison Error: {}", e.to_string())).into();
			error
		});
		res
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! lockr {
	($a:expr) => {{
		let res = $a.read().map_err(|e| {
			let error: bmw_err::Error =
				bmw_err::ErrorKind::Poison(format!("Poison Error: {}", e.to_string())).into();
			error
		});

		res
	}};
}

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

#[cfg(test)]
mod test {
	use crate as bmw_log;
	use crate::{LogConfigOption, LogConfigOptionName};
	use bmw_err::Error;
	use bmw_log::LogConfig;
	use bmw_test::testdir::{setup_test_dir, tear_down_test_dir};
	use std::fmt::{Debug, Formatter};
	use std::fs::read_to_string;
	use std::path::PathBuf;
	use std::sync::{Arc, RwLock};
	use std::thread::spawn;

	trace!();

	#[test]
	fn test_lock() -> Result<(), Error> {
		let v: u32 = 0;
		let x = Arc::new(RwLock::new(v));
		let x_clone = x.clone();

		let jh = spawn(move || {
			let p: Option<u32> = None;
			let _x = lockw!(x_clone).unwrap();
			p.unwrap(); // cause thread panic and poison this lock
		});

		// this will be an error but we ignore for the purposes of this test
		let _ = jh.join();

		// try to get the lock here and it's a poison error
		let res = lockw!(x);
		assert!(res.is_err());
		let res = lockr!(x);
		assert!(res.is_err());

		Ok(())
	}

	#[test]
	fn test_log_macros_werror() -> Result<(), Error> {
		let test_dir = ".test_macros.bmw";
		setup_test_dir(test_dir)?;

		let log_file = format!("{}/test.log", test_dir);
		log_init!(LogConfig {
			file_path: LogConfigOption::FilePath(Some(PathBuf::from(log_file.clone()))),
			max_size_bytes: LogConfigOption::MaxSizeBytes(100),
			auto_rotate: LogConfigOption::AutoRotate(false),
			show_bt: LogConfigOption::ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		})?;

		trace!("1")?;
		debug!("2")?;
		info!("3")?;
		warn!("4")?;
		error!("5")?;
		fatal!("6")?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.len(), 406);

		set_log_option!(LogConfigOption::ShowMillis(false))?;
		let show_millis = get_log_option!(LogConfigOptionName::ShowMillis)?;
		info!("show_millis={:?}", show_millis)?;
		assert_eq!(show_millis, LogConfigOption::ShowMillis(false));

		trace!("1")?;
		debug!("2")?;
		info!("3")?;
		warn!("4")?;
		error!("5")?;
		fatal!("6")?;

		trace_plain!("1")?;
		debug_plain!("2")?;
		info_plain!("3")?;
		warn_plain!("4")?;
		error_plain!("5")?;
		fatal_plain!("6")?;

		trace_all!("1")?;
		debug_all!("2")?;
		info_all!("3")?;
		warn_all!("4")?;
		error_all!("5")?;
		fatal_all!("6")?;

		let need_rotate = need_rotate!()?;
		info!("need_rotate={}", need_rotate)?;
		assert_eq!(need_rotate, true);
		log_rotate!()?;
		let need_rotate = need_rotate!()?;
		assert_eq!(need_rotate, false);

		let recursive = Recursive {};
		info_all!("{:?}", recursive)?;
		info_all!("another {:?}", recursive)?;

		test_log_macros_expect();

		tear_down_test_dir(test_dir)?;
		Ok(())
	}

	fn test_log_macros_expect() {
		trace!("1").expect("");
		debug!("2").expect("");
		info!("3").expect("");
		warn!("4").expect("");
		error!("5").expect("");
		fatal!("6").expect("");

		set_log_option!(LogConfigOption::ShowMillis(false)).expect("");
		let show_millis = get_log_option!(LogConfigOptionName::ShowMillis).expect("");
		assert_eq!(show_millis, LogConfigOption::ShowMillis(false));

		trace!("1").expect("");
		debug!("2").expect("");
		info!("3").expect("");
		warn!("4").expect("");
		error!("5").expect("");
		fatal!("6").expect("");

		let need_rotate = need_rotate!().expect("");
		info!("need_rotate={}", need_rotate).expect("");
		log_rotate!().expect("");
		need_rotate!().expect("");
	}
	struct Recursive {}

	impl Debug for Recursive {
		fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
			let _ = info!("test {}", "ok");
			let _ = info_all!("test all {}", "ok");
			let _ = info_plain!("test plain {}", "ok");
			write!(f, "done")?;
			Ok(())
		}
	}
}
