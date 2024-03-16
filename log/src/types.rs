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

use bmw_conf::{ConfigOption, ConfigOptionName};
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

lazy_static! {
	#[doc(hidden)]
	pub static ref BMW_GLOBAL_LOG: Arc<RwLock<Option<Box<dyn Log + Send + Sync>>>> = Arc::new(RwLock::new(None));
}

#[derive(PartialEq)]
pub enum LoggingType {
	Standard,
	Plain,
	All,
}

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

pub trait Log {
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error>;
	fn rotate(&mut self) -> Result<(), Error>;
	fn need_rotate(&self) -> Result<bool, Error>;
	fn set_log_level(&mut self, level: LogLevel) -> Result<(), Error>;
	fn init(&mut self) -> Result<(), Error>;
	fn close(&mut self) -> Result<(), Error>;
	fn set_config_option(&mut self, value: ConfigOption) -> Result<(), Error>;
	fn get_config_option(&self, option: ConfigOptionName) -> Result<ConfigOption, Error>;
}

pub struct LogBuilder {}

pub struct GlobalLogContainer {}

// Crate local types

#[derive(Clone)]
pub(crate) struct LogConfig {
	pub(crate) colors: bool,
	pub(crate) stdout: bool,
	pub(crate) max_size_bytes: u64,
	pub(crate) max_age_millis: u128,
	pub(crate) timestamp: bool,
	pub(crate) level: bool,
	pub(crate) line_num: bool,
	pub(crate) show_millis: bool,
	pub(crate) auto_rotate: bool,
	pub(crate) file_path: Option<Box<PathBuf>>,
	pub(crate) show_backtrace: bool,
	pub(crate) line_num_data_max_len: usize,
	pub(crate) delete_rotation: bool,
	pub(crate) file_header: String,
	pub(crate) debug_process_resolve_frame_error: bool,
	pub(crate) debug_invalid_metadata: bool,
}

pub(crate) struct LogImpl {
	pub(crate) config: LogConfig,
	pub(crate) log_level: LogLevel,
	pub(crate) cur_size: u64,
	pub(crate) file: Arc<RwLock<Option<File>>>,
	pub(crate) is_init: bool,
	pub(crate) last_rotation: Instant,
}
