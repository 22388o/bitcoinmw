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

mod log;
mod macros;
mod types;

pub use crate::log::LogBuilder;
pub use crate::macros::{LogHolder, LOG_REF, STATIC_LOG};
pub use crate::types::{Log, LogConfig, LogConfigOption, LogConfigOptionName, LogLevel};
