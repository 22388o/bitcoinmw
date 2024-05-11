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

use crate::LogErrorKind::*;
use crate::{LogBuilder, LogConstOptions, LogLevel, Logger, LoggingType};
use bmw_core::lazy_static::lazy_static;
use bmw_core::*;
use std::sync::{Arc, RwLock};

// Holder for the global logger
#[doc(hidden)]
pub struct GlobalLogFunctions {}

//  global logger
lazy_static! {
		#[doc(hidden)]
		pub static ref BMW_GLOBAL_LOG: Arc<RwLock<Option<Box<dyn Logger + Send + Sync>>>> = Arc::new(RwLock::new(None));

		#[doc(hidden)]
		pub static ref CHECK_INIT_LOCK: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
}

impl GlobalLogFunctions {
	pub fn log(
		level: LogLevel,
		line: &str,
		global_level: LogLevel,
		logging_type: LoggingType,
	) -> Result<(), Error> {
		if level as usize >= global_level as usize {
			Self::check_init()?; // check if we need to call init
			let mut log = BMW_GLOBAL_LOG.write()?;

			// call logger based on logging type (unwrap ok because check_init ensures
			// there's a logger
			match logging_type {
				LoggingType::Standard => (*log).as_mut().unwrap().log(level, line)?,
				LoggingType::Plain => (*log).as_mut().unwrap().log_plain(level, line)?,
				LoggingType::All => (*log).as_mut().unwrap().log_all(level, line)?,
			}
		}
		Ok(())
	}

	pub fn init(values: Vec<LogConstOptions>) -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		if (*log).is_some() {
			let text = "global logger has already been initialized";
			return err!(AlreadyInitialized, text);
		}
		let mut logger = LogBuilder::build_logger(values)?;
		logger.set_log_level(LogLevel::Trace);
		logger.init()?;
		(*log) = Some(Box::new(logger));
		Ok(())
	}

	pub fn set_log_option(option: LogConstOptions) -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		match (*log).as_mut() {
			Some(logger) => logger.set_log_option(option),
			None => {
				let text = "global logger has not been initalized";
				return err!(NotInitialized, text);
			}
		}
	}

	pub fn rotate() -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		match (*log).as_mut() {
			Some(logger) => logger.rotate(),
			None => {
				let text = "global logger has not been initalized";
				return err!(NotInitialized, text);
			}
		}
	}

	pub fn need_rotate() -> Result<bool, Error> {
		let log = BMW_GLOBAL_LOG.read()?;
		match (*log).as_ref() {
			Some(logger) => logger.need_rotate(),
			None => {
				let text = "global logger has not been initialized";
				return err!(NotInitialized, text);
			}
		}
	}

	fn check_init() -> Result<(), Error> {
		let _check_init_lock = CHECK_INIT_LOCK.write()?;
		let need_init;
		{
			let log = BMW_GLOBAL_LOG.read()?;
			match *log {
				Some(_) => {
					need_init = false;
				}
				None => {
					need_init = true;
				}
			}
		}

		// haven't initialized yet, so call init
		if need_init {
			Self::init(vec![])?;
		}
		Ok(())
	}
}
