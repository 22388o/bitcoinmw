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

use crate::constants::*;
use crate::types::{LogConfig, LogImpl};
use crate::{u64, GlobalLogContainer, Log, LogBuilder, LogLevel, LoggingType, BMW_GLOBAL_LOG};
use bmw_conf::*;
use bmw_deps::backtrace;
use bmw_deps::backtrace::{Backtrace, Symbol};
use bmw_deps::chrono::{DateTime, Local};
use bmw_deps::colored::Colorize;
use bmw_deps::rand::random;
use bmw_err::*;
use std::fmt::{Display, Formatter};
use std::fs::{remove_file, rename, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

// convenience macro
macro_rules! some_or_err {
	($m:expr, $errkind:expr, $text:expr) => {{
		use bmw_err::*;
		match $m {
			Some(m) => Ok(m),
			None => Err(err!($errkind, $text)),
		}
	}};
}

// convenience macro
macro_rules! none_or_err {
	($m:expr, $errkind:expr, $text:expr) => {{
		use bmw_err::*;
		match $m {
			Some(_) => Err(err!($errkind, $text)),
			None => Ok(()),
		}
	}};
}

impl Display for LogLevel {
	fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self {
			LogLevel::Trace => write!(w, "TRACE"),
			LogLevel::Debug => write!(w, "DEBUG"),
			LogLevel::Info => write!(w, "INFO"),
			LogLevel::Warn => write!(w, "WARN"),
			LogLevel::Error => write!(w, "ERROR"),
			LogLevel::Fatal => write!(w, "FATAL"),
		}
	}
}

impl GlobalLogContainer {
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

	pub fn init(values: Vec<ConfigOption>) -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		let mut logger = LogBuilder::build_log(values)?;
		logger.set_log_level(LogLevel::Trace);
		logger.init()?;
		(*log) = Some(logger);
		Ok(())
	}

	pub fn set_log_option(option: ConfigOption) -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		match (*log).as_mut() {
			Some(logger) => logger.set_config_option(option),
			None => {
				let text = "global logger has not been initalized";
				let err = err!(ErrKind::Configuration, text);
				Err(err)
			}
		}
	}

	pub fn get_log_option(option: ConfigOptionName) -> Result<ConfigOption, Error> {
		let log = BMW_GLOBAL_LOG.read()?;
		match (*log).as_ref() {
			Some(logger) => logger.get_config_option(option),
			None => {
				let text = "global logger has not been initialized";
				let err = err!(ErrKind::Configuration, text);
				Err(err)
			}
		}
	}

	pub fn rotate() -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		match (*log).as_mut() {
			Some(logger) => logger.rotate(),
			None => {
				let text = "global logger has not been initalized";
				let err = err!(ErrKind::Configuration, text);
				Err(err)
			}
		}
	}

	pub fn need_rotate() -> Result<bool, Error> {
		let log = BMW_GLOBAL_LOG.read()?;
		match (*log).as_ref() {
			Some(logger) => logger.need_rotate(),
			None => {
				let text = "global logger has not been initialized";
				let err = err!(ErrKind::Configuration, text);
				Err(err)
			}
		}
	}

	fn check_init() -> Result<(), Error> {
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

impl Log for LogImpl {
	// all logging goes through the log_impl fn.
	fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
		self.log_impl(level, line, LoggingType::Standard)
	}
	// all logging goes through the log_impl fn.
	fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
		self.log_impl(level, line, LoggingType::All)
	}
	// all logging goes through the log_impl fn.
	fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
		self.log_impl(level, line, LoggingType::Plain)
	}
	fn rotate(&mut self) -> Result<(), Error> {
		if !self.is_init {
			// log hasn't been initialized yet, return error
			let text = "log file cannot be rotated because init() was never called";
			return Err(err!(ErrKind::Log, text));
		}

		{
			// check if there's a file, if not return error
			let mut file = self.file.write()?;
			match (*file).as_mut() {
				Some(_file) => {}
				None => {
					return Err(err!(ErrKind::Log, "log file cannot be rotated because there is no file associated with this logger"));
				}
			}
		}

		let now: DateTime<Local> = Local::now();
		// standard rotation string format
		let rotation_string = now.format(".r_%m_%d_%Y_%T").to_string().replace(":", "-");

		let ekind = ErrKind::IllegalArgument;
		let text = "log file cannot be rotated. There is no file associated with this logger";

		// get the original file path
		let original_file_path = some_or_err!(self.config.file_path.clone(), ekind, text)?;

		// get the parent directory and the file name
		let text = "file_path has an unexpected illegal value of None for parent";
		let parent = some_or_err!(original_file_path.parent(), ekind, text)?;

		let text = "file_path has an unexpected illegal value of None for file_name";
		let file_name = some_or_err!(original_file_path.file_name(), ekind, text)?;

		let text = "file_path could not be converted to string";
		let file_name = some_or_err!(file_name.to_str(), ekind, text)?;

		// create the new rotated file
		let mut new_file_path_buf = parent.to_path_buf();
		let file_name = match file_name.rfind(".") {
			Some(pos) => &file_name[0..pos],
			_ => &file_name,
		};
		let file_name = format!("{}{}_{}.log", file_name, rotation_string, random::<u64>());
		new_file_path_buf.push(file_name);

		if self.config.delete_rotation {
			remove_file(&original_file_path.as_path())?;
		} else {
			rename(&original_file_path.as_path(), new_file_path_buf.as_path())?;
		}

		let mut open_options = OpenOptions::new();
		let open_options = open_options.append(true).create(true);
		let mut nfile = open_options.open(&original_file_path.as_path())?;
		// reopen the original file so we can continue logging
		self.check_open(&mut nfile, &original_file_path)?;

		{
			let mut file = self.file.write()?;
			*file = Some(nfile);
		}

		Ok(())
	}
	fn need_rotate(&self) -> Result<bool, Error> {
		if !self.is_init {
			return Err(err!(ErrKind::Log, "log not initialized"));
		}

		let now = Instant::now();

		let max_age_millis = self.config.max_age_millis;
		let max_size_bytes = self.config.max_size_bytes;

		// if the file is either too old or too big we need to rotate
		if now.duration_since(self.last_rotation).as_millis() > max_age_millis
			|| self.cur_size > max_size_bytes
		{
			Ok(true)
		} else {
			Ok(false)
		}
	}
	fn set_log_level(&mut self, log_level: LogLevel) {
		self.log_level = log_level;
	}
	fn init(&mut self) -> Result<(), Error> {
		if self.is_init {
			// init already was called
			return Err(err!(ErrKind::Log, "log file has already ben initialized"));
		}
		{
			let file = self.file.read()?;
			let errkind = ErrKind::IllegalState;
			let text = "log.init() has already been called";
			none_or_err!((*file).as_ref(), errkind, text)?;
		}

		match self.config.file_path.clone().as_ref() {
			Some(path) => {
				let mut f = match File::options().append(true).open(path.as_path()) {
					Ok(f) => {
						// already exists just return file here
						f
					}
					Err(_) => {
						// try to create it
						File::create(path.as_path())?
					}
				};
				self.check_open(&mut f, path)?;

				let mut file = self.file.write()?;
				*file = Some(f);
			}
			None => {}
		}
		self.is_init = true;

		Ok(())
	}
	fn close(&mut self) -> Result<(), Error> {
		if !self.is_init {
			let ekind = ErrKind::Log;
			let text = "log file cannot be closed because init() was never called";
			return Err(err!(ekind, text));
		}
		let mut file = self.file.write()?;
		// drop handler closes the handle
		*file = None;
		Ok(())
	}
	fn set_config_option(&mut self, value: ConfigOption) -> Result<(), Error> {
		// set the specified option, LogFilePath results in an error.
		use bmw_conf::ConfigOption as CO;
		let errkind = ErrKind::Configuration;
		let text = "cannot set LogFilePath after logging has been started";
		match value {
			CO::DisplayColors(v) => self.config.colors = v,
			CO::DisplayTimestamp(v) => self.config.timestamp = v,
			CO::MaxSizeBytes(v) => self.config.max_size_bytes = v,
			CO::MaxAgeMillis(v) => self.config.max_age_millis = v,
			CO::DisplayStdout(v) => self.config.stdout = v,
			CO::DisplayLogLevel(v) => self.config.level = v,
			CO::DisplayLineNum(v) => self.config.line_num = v,
			CO::DisplayMillis(v) => self.config.show_millis = v,
			CO::AutoRotate(v) => self.config.auto_rotate = v,
			CO::DisplayBackTrace(v) => self.config.show_backtrace = v,
			CO::LineNumDataMaxLen(v) => self.config.line_num_data_max_len = v,
			CO::DeleteRotation(v) => self.config.delete_rotation = v,
			CO::FileHeader(v) => self.config.file_header = v,
			CO::LogFilePath(_) => return Err(err!(errkind, text)),
			_ => return Err(err!(ErrKind::Configuration, "unknown config option")),
		}
		Ok(())
	}
	fn get_config_option(&self, option: ConfigOptionName) -> Result<ConfigOption, Error> {
		// get any specified options
		use bmw_conf::ConfigOption as CO;
		use bmw_conf::ConfigOptionName as CN;
		Ok(match option {
			CN::DisplayColors => CO::DisplayColors(self.config.colors),
			CN::DisplayTimestamp => CO::DisplayTimestamp(self.config.timestamp),
			CN::MaxSizeBytes => CO::MaxSizeBytes(self.config.max_size_bytes),
			CN::MaxAgeMillis => CO::MaxAgeMillis(self.config.max_age_millis),
			CN::DisplayStdout => CO::DisplayStdout(self.config.stdout),
			CN::DisplayLogLevel => CO::DisplayLogLevel(self.config.level),
			CN::DisplayLineNum => CO::DisplayLineNum(self.config.line_num),
			CN::DisplayMillis => CO::DisplayMillis(self.config.show_millis),
			CN::LogFilePath => CO::LogFilePath(self.config.file_path.clone()),
			CN::AutoRotate => CO::AutoRotate(self.config.auto_rotate),
			CN::DisplayBackTrace => CO::DisplayBackTrace(self.config.show_backtrace),
			CN::LineNumDataMaxLen => CO::LineNumDataMaxLen(self.config.line_num_data_max_len),
			CN::DeleteRotation => CO::DeleteRotation(self.config.delete_rotation),
			CN::FileHeader => CO::FileHeader(self.config.file_header.clone()),
			_ => return Err(err!(ErrKind::Configuration, "unknown config option")),
		})
	}

	#[cfg(test)]
	fn debug_process_resolve_frame_error(&mut self) {
		self.config.debug_process_resolve_frame_error = true;
	}

	#[cfg(test)]
	fn debug_invalid_metadata(&mut self) {
		self.config.debug_invalid_metadata = true;
	}

	#[cfg(test)]
	fn debug_lineno_is_none(&mut self) {
		self.config.debug_lineno_is_none = true;
	}
}

impl LogImpl {
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = LogConfig::new(configs)?;
		let log_level = LogLevel::Info;
		let cur_size = 0;
		let file = Arc::new(RwLock::new(None));
		let is_init = false;
		let last_rotation = Instant::now();
		Ok(Self {
			config,
			log_level,
			cur_size,
			file,
			is_init,
			last_rotation,
		})
	}

	fn rotate_if_needed(&mut self) -> Result<(), Error> {
		if !self.config.auto_rotate {
			return Ok(()); // auto rotate not enabled
		}

		let now = Instant::now();

		let max_age_millis = self.config.max_age_millis;
		let max_size_bytes = self.config.max_size_bytes;

		// if the file is too old or too big we rotate
		if now.duration_since(self.last_rotation).as_millis() > max_age_millis
			|| self.cur_size > max_size_bytes
		{
			self.rotate()?;
		}

		Ok(())
	}

	fn log_impl(
		&mut self,
		level: LogLevel,
		line: &str,
		logging_type: LoggingType,
	) -> Result<(), Error> {
		if level as usize >= self.log_level as usize {
			self.rotate_if_needed()?;
			let show_stdout = self.config.stdout || logging_type == LoggingType::All;
			let show_timestamp = self.config.timestamp && logging_type != LoggingType::Plain;
			let show_colors = self.config.colors;
			let show_log_level = self.config.level && logging_type != LoggingType::Plain;
			let show_line_num = self.config.line_num && logging_type != LoggingType::Plain;
			let show_millis = self.config.show_millis && logging_type != LoggingType::Plain;
			let show_bt = self.config.show_backtrace && level as usize >= LogLevel::Error as usize;
			let max_len = self.config.line_num_data_max_len;

			// call the main logging function with the specified params
			self.do_log_impl(
				show_stdout,
				show_timestamp,
				show_colors,
				show_log_level,
				show_line_num,
				show_millis,
				show_bt,
				level,
				max_len,
				line,
			)?;
		}
		Ok(())
	}

	fn do_log_impl(
		&mut self,
		show_stdout: bool,
		show_timestamp: bool,
		show_colors: bool,
		show_log_level: bool,
		show_line_num: bool,
		show_millis: bool,
		show_bt: bool,
		level: LogLevel,
		max_len: usize,
		line: &str,
	) -> Result<(), Error> {
		// if timestamp needs to be shown we print/write it here
		if show_timestamp {
			let date = Local::now();
			let millis = date.timestamp_millis() % 1_000;
			let millis_format = self.format_millis(millis);
			let formatted_timestamp = if show_millis {
				format!("{}.{}", date.format("%Y-%m-%d %H:%M:%S"), millis_format)
			} else {
				format!("{}", date.format("%Y-%m-%d %H:%M:%S"))
			};

			{
				let mut file = self.file.write()?;
				match (*file).as_mut() {
					Some(file) => {
						let formatted_timestamp = format!("[{}]: ", formatted_timestamp);
						let formatted_timestamp = formatted_timestamp.as_bytes();
						file.write(formatted_timestamp)?;
						let formatted_len: u64 = u64!(formatted_timestamp.len());
						self.cur_size += formatted_len;
					}
					None => {}
				}
			}

			if show_stdout {
				if show_colors {
					print!("[{}]: ", formatted_timestamp.to_string().dimmed());
				} else {
					print!("[{}]: ", formatted_timestamp);
				}
			}
		}
		// if log level needs to be shown we print/write it here
		if show_log_level {
			{
				let mut file = self.file.write()?;
				match (*file).as_mut() {
					Some(file) => {
						let formatted_level = if level == LogLevel::Info || level == LogLevel::Warn
						{
							format!("({})  ", level)
						} else {
							format!("({}) ", level)
						};
						let formatted_level = formatted_level.as_bytes();
						file.write(formatted_level)?;
						let formatted_len: u64 = u64!(formatted_level.len());
						self.cur_size += formatted_len;
					}
					None => {}
				}
			}

			if show_stdout {
				if show_colors {
					// specific colors for each level
					match level {
						LogLevel::Trace => {
							print!("({}) ", format!("{}", level).magenta());
						}
						LogLevel::Debug => {
							print!("({}) ", format!("{}", level).cyan());
						}
						LogLevel::Info => {
							print!("({})  ", format!("{}", level).green());
						}
						LogLevel::Warn => {
							print!("({})  ", format!("{}", level).yellow());
						}
						LogLevel::Error => {
							print!("({}) ", format!("{}", level).bright_blue());
						}
						LogLevel::Fatal => {
							print!("({}) ", format!("{}", level).red());
						}
					}
				} else {
					// without color
					print!("({}) ", level);
				}
			}
		}
		if show_line_num {
			let mut found_logger = false;
			let mut found_frame = false;
			let mut logged_from_file = "*********unknown**********".to_string();
			let config = self.config.clone();

			// try to look through the backtrace to find the line where logging
			// occurred. This is especially useful for debugging
			backtrace::trace(|frame| {
				backtrace::resolve_frame(frame, |symbol| {
					found_frame = match Self::process_resolve_frame(
						&config,
						symbol,
						&mut found_logger,
						&mut logged_from_file,
					) {
						Ok(ff) => ff,
						Err(e) => {
							let _ = println!("error processing frame: {}", e);
							true
						}
					};
				});
				!found_frame
			});
			let len = logged_from_file.len();
			if len > max_len {
				let start = len.saturating_sub(max_len);
				logged_from_file = format!("..{}", &logged_from_file[start..]);
			}

			{
				let mut file = self.file.write()?;
				match (*file).as_mut() {
					Some(file) => {
						let logged_from_file = format!("[{}]: ", logged_from_file);
						let logged_from_file = logged_from_file.as_bytes();
						file.write(logged_from_file)?;
						let logged_from_file_len: u64 = u64!(logged_from_file.len());
						self.cur_size += logged_from_file_len;
					}
					None => {}
				}
			}

			// if we're showing stdout, do so here
			if show_stdout {
				if show_colors {
					print!("[{}]: ", logged_from_file.yellow());
				} else {
					print!("[{}]: ", logged_from_file);
				}
			}
		}

		// write the line to the file (if it exists)
		{
			let mut file = self.file.write()?;
			match (*file).as_mut() {
				Some(file) => {
					let line_bytes = line.as_bytes();

					file.write(line_bytes)?;
					file.write(NEWLINE)?;
					let mut line_bytes_len: u64 = u64!(line_bytes.len());
					line_bytes_len += 1;
					self.cur_size += line_bytes_len;

					if show_bt {
						let bt = Backtrace::new();
						let bt_text = format!("{:?}", bt);
						let bt_bytes: &[u8] = bt_text.as_bytes();
						file.write(bt_bytes)?;
						self.cur_size += u64!(bt_bytes.len());
					}
				}
				None => {}
			}
		}

		// finally print the actual line
		if show_stdout {
			println!("{}", line);
			if show_bt {
				let bt = Backtrace::new();
				let bt_text = format!("{:?}", bt);
				print!("{}", bt_text);
			}
		}

		Ok(())
	}

	fn process_resolve_frame(
		config: &LogConfig,
		symbol: &Symbol,
		found_logger: &mut bool,
		logged_from_file: &mut String,
	) -> Result<bool, Error> {
		if config.debug_process_resolve_frame_error {
			let e = err!(ErrKind::Test, "test resolve_frame error");
			return Err(e);
		}
		let mut found_frame = false;

		// test mode (better data)
		#[cfg(debug_assertions)]
		if let Some(filename) = symbol.filename() {
			let filename = filename.display().to_string();
			let lineno = symbol.lineno();

			let lineno = if lineno.is_none() || config.debug_lineno_is_none {
				"".to_string()
			} else {
				lineno.unwrap().to_string()
			};

			if filename.find("/log/src/log.rs").is_some()
				|| filename.find("\\log\\src\\log.rs").is_some()
			{
				*found_logger = true;
			}
			if (filename.find("/log/src/log.rs").is_none()
				&& filename.find("\\log\\src\\log.rs").is_none())
				&& *found_logger
			{
				*logged_from_file = format!("{}:{}", filename, lineno);
				found_frame = true;
			}
		}

		// release mode
		#[cfg(not(debug_assertions))]
		if let Some(name) = symbol.name() {
			let name = name.to_string();
			if name.find("as bmw_log::types::Log").is_some() {
				*found_logger = true;
			}
			if name.find("as bmw_log::types::Log").is_none() && *found_logger {
				let pos = name.rfind(':');
				let name = match pos {
					Some(pos) => match pos > 1 {
						true => &name[0..pos - 1],
						false => &name[..],
					},
					None => &name[..],
				};
				*logged_from_file = format!("{}", name);
				found_frame = true;
			}
		}
		Ok(found_frame)
	}

	// correctly format the milliseconds
	fn format_millis(&self, millis: i64) -> String {
		let mut millis_format = format!("{}", millis);
		if millis < 100 {
			millis_format = format!("0{}", millis_format);
		}
		if millis < 10 {
			millis_format = format!("0{}", millis_format);
		}

		millis_format
	}

	fn check_open(&mut self, file: &mut File, path: &PathBuf) -> Result<(), Error> {
		let metadata = file.metadata();
		if metadata.is_err() || self.config.debug_invalid_metadata {
			return Err(err!(
				ErrKind::Log,
				format!("failed to retreive metadata for file: {}", path.display())
			));
		}
		let metadata = metadata.unwrap();

		let len = metadata.len();
		if len == 0 {
			// do we need to add the file header?
			let header_len = self.config.file_header.len();
			if header_len > 0 {
				// there's a header. We need to append it.
				file.write(self.config.file_header.as_bytes())?;
				file.write(NEWLINE)?;
				self.cur_size = u64!(header_len) + 1;
			} else {
				self.cur_size = 0;
			}
		} else {
			self.cur_size = len;
		}

		self.last_rotation = Instant::now();
		Ok(())
	}
}

impl LogConfig {
	// create the log config based on the specified data
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(
			vec![
				ConfigOptionName::MaxSizeBytes,
				ConfigOptionName::MaxAgeMillis,
				ConfigOptionName::DisplayColors,
				ConfigOptionName::DisplayStdout,
				ConfigOptionName::DisplayTimestamp,
				ConfigOptionName::DisplayLogLevel,
				ConfigOptionName::DisplayLineNum,
				ConfigOptionName::DisplayMillis,
				ConfigOptionName::DisplayBackTrace,
				ConfigOptionName::LogFilePath,
				ConfigOptionName::LineNumDataMaxLen,
				ConfigOptionName::DeleteRotation,
				ConfigOptionName::FileHeader,
				ConfigOptionName::AutoRotate,
			],
			vec![],
		)?;
		Ok(Self {
			auto_rotate: match config.get(&ConfigOptionName::AutoRotate) {
				Some(v) => match v {
					ConfigOption::AutoRotate(v) => v,
					_ => false,
				},
				None => false,
			},
			colors: match config.get(&ConfigOptionName::DisplayColors) {
				Some(v) => match v {
					ConfigOption::DisplayColors(v) => v,
					_ => true,
				},
				None => true,
			},
			delete_rotation: match config.get(&ConfigOptionName::DeleteRotation) {
				Some(v) => match v {
					ConfigOption::DeleteRotation(v) => v,
					_ => false,
				},
				None => false,
			},
			file_header: match config.get(&ConfigOptionName::FileHeader) {
				Some(v) => match v {
					ConfigOption::FileHeader(v) => v,
					_ => "".to_string(),
				},
				None => "".to_string(),
			},
			file_path: match config.get(&ConfigOptionName::LogFilePath) {
				Some(v) => match v {
					ConfigOption::LogFilePath(v) => v,
					_ => None,
				},
				None => None,
			},
			level: match config.get(&ConfigOptionName::DisplayLogLevel) {
				Some(v) => match v {
					ConfigOption::DisplayLogLevel(v) => v,
					_ => true,
				},
				None => true,
			},
			line_num: match config.get(&ConfigOptionName::DisplayLineNum) {
				Some(v) => match v {
					ConfigOption::DisplayLineNum(v) => v,
					_ => true,
				},
				None => true,
			},
			line_num_data_max_len: match config.get(&ConfigOptionName::LineNumDataMaxLen) {
				Some(v) => match v {
					ConfigOption::LineNumDataMaxLen(v) => v,
					_ => DEFAULT_LINE_NUM_DATA_MAX_LEN,
				},
				None => DEFAULT_LINE_NUM_DATA_MAX_LEN,
			},
			max_age_millis: match config.get(&ConfigOptionName::MaxAgeMillis) {
				Some(v) => match v {
					ConfigOption::MaxAgeMillis(v) => v,
					_ => u128::MAX,
				},
				None => u128::MAX,
			},
			max_size_bytes: match config.get(&ConfigOptionName::MaxSizeBytes) {
				Some(v) => match v {
					ConfigOption::MaxSizeBytes(v) => v,
					_ => u64::MAX,
				},
				None => u64::MAX,
			},
			show_backtrace: match config.get(&ConfigOptionName::DisplayBackTrace) {
				Some(v) => match v {
					ConfigOption::DisplayBackTrace(v) => v,
					_ => false,
				},
				None => false,
			},
			show_millis: match config.get(&ConfigOptionName::DisplayMillis) {
				Some(v) => match v {
					ConfigOption::DisplayMillis(v) => v,
					_ => true,
				},
				None => true,
			},
			stdout: match config.get(&ConfigOptionName::DisplayStdout) {
				Some(v) => match v {
					ConfigOption::DisplayStdout(v) => v,
					_ => true,
				},
				None => true,
			},
			timestamp: match config.get(&ConfigOptionName::DisplayTimestamp) {
				Some(v) => match v {
					ConfigOption::DisplayTimestamp(v) => v,
					_ => true,
				},
				None => true,
			},
			debug_process_resolve_frame_error: false,
			debug_invalid_metadata: false,
			debug_lineno_is_none: false,
		})
	}
}
