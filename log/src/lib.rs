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

//! # The BMW Logging Crate
//! <style>
//!     .box img {
//!         display: inline-block;
//!         vertical-align: middle;
//!         float: left;
//!         width: 171px;
//!         margin-right: 15px;
//!     }
//!     .box .text {
//!         display: inline-block;
//!         vertical-align: top;
//!         float: right;
//!         width: calc(100% - 171px);    
//!     }
//! </style>
//! <span class="box">
//! <img style="width: 171px; background: white;"
//! src="https://raw.githubusercontent.com/cgilliard/bitcoinmw/main/.github/images/butterfly-7402310_1280.png">
//! The BMW Logging crate handles logging for all crates within BMW. There is a global static logger which is used
//! as a main log and for debugging tests and independant loggers which may be used for things like request and
//! statistical logging. The interface should be fairly straightforward to understand and it is largely compatible
//! with the Rust <a href="https://docs.rs/log/latest/log/">log</a> crate with some minor adjustments. Most
//! notably, the logging macros return errors in the case of i/o and other configuration related errors. The global
//! logger, controlled through macros, is ideal for things like test logging and a main application log. When more
//! controlled and/or performant logging is required, the standalone logger is ideal.
//! </span>
//!
//! # Macros
//! In addition to the [`trace`], [`debug`], [`info`], [`warn`], [`error`]
//! and [`fatal`] macros, this crate provides an 'all' version and 'plain'
//! version of each macro. For example: [`info_all`] and [`info_plain`].
//! These macros allow for logging to standard out no matter how the log is
//! configured and for logging without the timestamp respectively.
//!
//! # Examples
//!
//!```
//! // example of using the global static logger
//! use bmw_err::*;
//! use bmw_log::*;
//! use bmw_test::*;
//! use std::path::PathBuf;
//!
//! info!(); // set the log level of the global logger to 'info'.
//!
//! fn global_logger() -> Result<(), Error> {
//!     // get test_info for a uniqe test directory
//!     let test_info = test_info!()?;
//!
//!     // create a path_buf
//!     let mut buf = PathBuf::new();
//!     buf.push(test_info.directory());
//!     buf.push("mylog.log");
//!     let buf = buf.display().to_string();
//!
//!     // init the log. Important to do this before any logging takes place or a default log
//!     // config will be applied
//!     log_init!(
//!         AutoRotate(true), // turn on autorotation
//!         LogFilePath(&buf), // log to our log file
//!         MaxSizeBytes(1024 * 1024), // do a rotation when the log file reaches 1mb
//!         MaxAgeMillis(60 * 60 * 1000) // do a rotation when the log file is over 1 hour old
//!     )?;
//!
//!     // log at the info level
//!     info!("Starting up the logger")?;
//!
//!     // log at the debug level
//!     debug!("This will not show up because 'debug' is below 'info'")?;
//!     Ok(())
//! }
//!
//! // example of an independent logger
//! fn independent_logger() -> Result<(), Error> {
//!     // get a test_info to get a unique test directory
//!     let test_info = test_info!()?;
//!
//!     // create the path buffer with our log name
//!     let mut buf = PathBuf::new();
//!     buf.push(test_info.directory());
//!     buf.push("some_log.log");
//!     let buf = buf.display().to_string();
//!
//!     // create the logger with the logger macro.
//!     let mut logger = logger!(
//!         LogFilePath(&buf), // our path
//!         MaxAgeMillis(1000 * 30 * 60), // log max age before rotation
//!         DisplayColors(false), // don't display colors
//!         DisplayBacktrace(false) // don't show the backtrace on error/fatal log lines
//!     )?;
//!
//!     logger.init()?;
//!     logger.set_log_level(LogLevel::Debug);
//!     logger.log(LogLevel::Debug, "this is a test")?;
//!
//!     Ok(())
//! }
//!
//! fn main() -> Result<(), Error> {
//!     global_logger()?;
//!     independent_logger()?;
//!     Ok(())
//! }
//!```
//!
//! # Sample output
//!
//! The default output will look something like this:
//!
//! ```text
//! [2022-02-24 13:52:24.123]: (FATAL) [..ibconcord/src/main.rs:116]: fatal
//! [2022-02-24 13:52:24.123]: (ERROR) [..ibconcord/src/main.rs:120]: error
//! [2022-02-24 13:52:24.123]: (WARN) [..ibconcord/src/main.rs:124]: warn
//! [2022-02-24 13:52:24.123]: (INFO) [..ibconcord/src/main.rs:128]: info
//! [2022-02-24 13:52:24.123]: (DEBUG) [..ibconcord/src/main.rs:132]: debug
//! [2022-02-24 13:52:24.123]: (TRACE) [..ibconcord/src/main.rs:136]: trace
//! ```
//!
//! If enabled, color coding is included as well.
//!
//! Logging may be configured in many ways. The [`crate::log_init`] macro
//! allows for convenient configuration of logging.
//!
//! # Post initialization configuration
//!
//! Most log configuration options may be set after the log has been initialized. See the example
//! below. For all configuration options, see [`crate::log_init`]. Only the
//! [`bmw_conf::ConfigOption::LogFilePath`] may NOT be changed after [`crate::Log::init`] is called.
//!
//!```
//! use bmw_err::*;
//! use bmw_log::*;
//!
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     // Init log first
//!     log_init!(
//!         DisplayColors(false),
//!         DisplayStdout(true),
//!     )?;
//!
//!     info!("show this!")?;
//!
//!     set_log_option!(DisplayColors(true))?;
//!
//!     info!("show this with colors!")?;
//!
//!     Ok(())
//! }
//!
//!```

mod builder;
mod constants;
mod log;
mod macros;
mod public;
mod test;
mod types;

pub use crate::public::*;
pub use crate::types::LogConfig2_Options;
