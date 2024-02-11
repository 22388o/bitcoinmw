// Copyright (c) 2023, The BitcoinMW Developers
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

//! Logging crate used by other crates in bmw. The crate has a macro
//! library that allows for logging at the standard 6 levels and also
//! allows for specifying a log file and various options. All options can
//! be seen in the [`crate::LogConfig`] struct. This crate is largely compatible
//! with the [log](https://docs.rs/log/latest/log/) crate. So any code
//! that was written to work with that crate will work with this crate with minor
//! adjustments. In addition to the [`trace`], [`debug`], [`info`], [`warn`], [`error`]
//! and [`fatal`] macros, this crate provides an 'all' version and 'plain'
//! version of each macro. For example: [`info_all`] and [`info_plain`].
//! These macros allow for logging to standard out no matter how the log is
//! configured and for logging without the timestamp respectively. The main adjustment a developer
//! accustomed to using the rust log crate would need to make is that this crate returns
//! errors so you will have to add error handling which can be as simple as using
//! the question mark operator or using the [`bmw_err::map_err`] macro.
//!
//! # Examples
//!
//!```
//! use bmw_err::*;
//! use bmw_log::*;
//!
//! // set log level for this file. Anything below this scope will only be
//! // logged if it is equal to or less than log level 'INFO'.
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     let abc = 123;
//!     info!("v1={},v2={}", abc, "def")?; // will show up
//!     debug!("test")?; // will not show up
//!
//!     Ok(())
//! }
//!
//!```
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
//! If enabled color coding is included as well.
//!
//! Logging may be configured in many ways. The [`crate::log_init`] macro
//! allows for convenient configuration of logging.
//!
//! Some examples:
//!
//!```
//! use bmw_err::*;
//! use bmw_log::*;
//!
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     log_init!(LogConfig {
//!         colors: Colors(true), // Show colors
//!         stdout: Stdout(true), // Print to stdout
//!         max_size_bytes: MaxSizeBytes(1024 * 1024 * 5), // The maximum bytes before a log
//!                                                        // rotation occurs (only applicable when
//!                                                        // file logging is used)
//!         max_age_millis: MaxAgeMillis(1000 * 30 * 60),  // The maximum time, in milliseconds,
//!                                                        // before a log rotation occurs (only
//!                                                        // applicable when file logging is used)
//!         timestamp: Timestamp(true), // Show timestamps
//!         level: Level(true), // Show log level
//!         line_num: LineNum(false), // Show line numbers
//!         show_millis: ShowMillis(false), // Show miliseconds in the timestamp
//!         auto_rotate: AutoRotate(true), // Automatically rotate
//!         file_path: FilePath(None), // Log to this file location
//!         show_bt: ShowBt(true), // Show backtraces for error and fatal level logging
//!         line_num_data_max_len: LineNumDataMaxLen(20), // The maximum length shown of the
//!                                                       // linenum field.
//!         delete_rotation: DeleteRotation(false), // Delete log rotations (only used in testing)
//!         file_header: FileHeader("BitcoinMW Log V1.1".to_string()), // Header (first line) of the log files
//!         debug_invalid_metadata: false, // debugging parameter which must only be set in tests
//!         debug_invalid_os_str: false, // debugging parameter which must only be set in tests
//!         debug_lineno_none: false, // debugging parameter which must only be set in tests
//!         debug_process_resolve_frame_error: false, // debugging parameter which must only be set in tests
//!     })?;
//!
//!     info!("show this!")?;
//!
//!     Ok(())
//! }
//!
//! fn with_default() -> Result<(), Error> {
//!     // The default trait is implemented for [`crate::LogConfig`] so specifying all parameters
//!     // is not necessary.
//!     log_init!(LogConfig {
//!         colors: Colors(false),
//!         stdout: Stdout(true),
//!         ..Default::default()
//!     })?;
//!
//!     info!("show this!")?;
//!
//!     Ok(())
//! }
//!
//!```
//!
//! Log configuration options may be set after the log has been initialized. See the example
//! below. For all configuration options, see [`crate::LogConfigOption`].
//!
//!```
//! use bmw_err::*;
//! use bmw_log::*;
//!
//! info!();
//!
//! fn set_log_options_after_startup() -> Result<(), Error> {
//!     // Init log first
//!
//!     log_init!(LogConfig {
//!         colors: Colors(false),
//!         stdout: Stdout(true),
//!         ..Default::default()
//!     })?;
//!
//!     info!("show this!")?;
//!
//!     set_log_option!(LogConfigOption::Colors(true))?;
//!
//!     info!("show this with colors!")?;
//!
//!     Ok(())
//! }
//!
//!```

mod log;
mod macros;
mod types;

pub use crate::log::LogBuilder;
pub use crate::macros::{LogHolder, LOG_REF, STATIC_LOG};
pub use crate::types::{Log, LogConfig, LogConfigOption, LogConfigOptionName, LogLevel};
pub use crate::LogConfigOption::*;
