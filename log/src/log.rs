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
use crate::public::*;
use crate::types::{LogConfig, LogImpl};
use bmw_conf::*;
use bmw_deps::backtrace;
use bmw_deps::backtrace::{Backtrace, Symbol};
use bmw_deps::chrono::{DateTime, Local};
use bmw_deps::colored::Colorize;
use bmw_deps::dirs;
use bmw_deps::dyn_clone::clone_trait_object;
use bmw_deps::rand::random;
use bmw_deps::url_path::UrlPath;
use bmw_err::*;
use std::fmt::{Display, Formatter};
use std::fs::{remove_file, rename, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::Instant;

clone_trait_object!(Log);

impl Default for LogConfig {
	fn default() -> Self {
		Self {
			max_size_bytes: u64::MAX,
			max_age_millis: u64::MAX,
			line_num_data_max_len: DEFAULT_LINE_NUM_DATA_MAX_LEN,
			display_colors: true,
			display_stdout: true,
			display_timestamp: true,
			display_log_level: true,
			display_line_num: true,
			display_millis: true,
			display_backtrace: false,
			log_file_path: "".to_string(),
			delete_rotation: false,
			auto_rotate: false,
			file_header: "".to_string(),
			debug_invalid_metadata: false,
			debug_lineno_is_none: false,
			debug_process_resolve_frame_error: false,
		}
	}
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

	pub fn init(values: Vec<LogConfigOptions>) -> Result<(), Error> {
		let mut log = BMW_GLOBAL_LOG.write()?;
		let mut logger = LogBuilder::build_log(values)?;
		logger.set_log_level(LogLevel::Trace);
		logger.init()?;
		(*log) = Some(logger);
		Ok(())
	}

	pub fn set_log_option(option: LogConfigOptions) -> Result<(), Error> {
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

impl LogImpl {
	pub(crate) fn new(configs: Vec<LogConfigOptions>) -> Result<Self, Error> {
		let mut config = config!(LogConfig, LogConfigOptions, configs)?;

		// insert the home directory for ~
		let home_dir = dirs::home_dir()
			.unwrap_or(PathBuf::new())
			.as_path()
			.display()
			.to_string();

		config.log_file_path = config.log_file_path.replace("~", &home_dir);

		// to check if the file is ok we try to canonicalize its parent, but only if a file
		// is specified (len > 0). If there is no parent directory, canonicalize will fail
		if config.log_file_path.len() > 0 {
			let mut path_buf = PathBuf::from(&config.log_file_path);
			path_buf.pop();
			let path_buf_str = path_buf.display().to_string();
			let url_path = UrlPath::new(&path_buf_str).normalize();
			// make sure we can canonicalize the path
			PathBuf::from(url_path).as_path().canonicalize()?;
		}

		if config.max_age_millis < MINIMUM_MAX_AGE_MILLIS {
			let text = format!("MaxAgeMillis must be at least {}", MINIMUM_MAX_AGE_MILLIS);
			return Err(err!(ErrKind::Configuration, text));
		}

		if config.max_size_bytes < MINIMUM_MAX_SIZE_BYTES {
			let text = format!("MaxSizeBytes must be at least {}", MINIMUM_MAX_SIZE_BYTES);
			return Err(err!(ErrKind::Configuration, text));
		}

		if config.line_num_data_max_len < MINIMUM_LNDML {
			let text = format!("LineNumDataMaxLen must be at least {}", MINIMUM_LNDML);
			return Err(err!(ErrKind::Configuration, text));
		}

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
		if now
			.checked_duration_since(self.last_rotation)
			.unwrap_or(Duration::new(0, 0))
			.as_millis() > max_age_millis.into()
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
		if !self.is_init {
			let ekind = ErrKind::Log;
			let text = "logger has not been initalized. Call init() first.";
			return Err(err!(ekind, text));
		}

		if level as usize >= self.log_level as usize {
			self.rotate_if_needed()?;
			let show_stdout = self.config.display_stdout || logging_type == LoggingType::All;
			let show_timestamp =
				self.config.display_timestamp && logging_type != LoggingType::Plain;
			let show_colors = self.config.display_colors;
			let show_log_level =
				self.config.display_log_level && logging_type != LoggingType::Plain;
			let show_line_num = self.config.display_line_num && logging_type != LoggingType::Plain;
			let show_millis = self.config.display_millis && logging_type != LoggingType::Plain;
			let show_bt =
				self.config.display_backtrace && level as usize >= LogLevel::Error as usize;
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
				logging_type,
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
		max_len: u64,
		line: &str,
		logging_type: LoggingType,
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
						let formatted_len: u64 = try_into!(formatted_timestamp.len())?;
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
						let formatted_len: u64 = try_into!(formatted_level.len())?;
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
							print!("({})", format!("{}", level).magenta());
						}
						LogLevel::Debug => {
							print!("({})", format!("{}", level).cyan());
						}
						LogLevel::Info => {
							print!(" ({})", format!("{}", level).green());
						}
						LogLevel::Warn => {
							print!(" ({})", format!("{}", level).yellow());
						}
						LogLevel::Error => {
							print!("({})", format!("{}", level).bright_blue());
						}
						LogLevel::Fatal => {
							print!("({})", format!("{}", level).red());
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
			if len > try_into!(max_len)? {
				let start = len.saturating_sub(try_into!(max_len)?);
				logged_from_file = format!("..{}", &logged_from_file[start..]);
			}

			{
				let mut file = self.file.write()?;
				match (*file).as_mut() {
					Some(file) => {
						let logged_from_file = format!("[{}]: ", logged_from_file);
						let logged_from_file = logged_from_file.as_bytes();
						file.write(logged_from_file)?;
						let logged_from_file_len: u64 = try_into!(logged_from_file.len())?;
						self.cur_size += logged_from_file_len;
					}
					None => {}
				}
			}

			// if we're showing stdout, do so here
			if show_stdout {
				if show_colors {
					print!(" [{}]", logged_from_file.yellow());
				} else {
					print!(" [{}]", logged_from_file);
				}
			}
		}

		if show_stdout && logging_type != LoggingType::Plain {
			print!(": ");
		}

		// write the line to the file (if it exists)
		{
			let mut file = self.file.write()?;
			match (*file).as_mut() {
				Some(file) => {
					let line_bytes = line.as_bytes();

					file.write(line_bytes)?;
					file.write(NEWLINE)?;
					let mut line_bytes_len: u64 = try_into!(line_bytes.len())?;
					line_bytes_len += 1;
					self.cur_size += line_bytes_len;

					if show_bt {
						let bt = Backtrace::new();
						let bt_text = format!("{:?}", bt);
						let bt_bytes: &[u8] = bt_text.as_bytes();
						file.write(bt_bytes)?;
						let bt_bytes_u64: u64 = try_into!(bt_bytes.len())?;
						self.cur_size += bt_bytes_u64;
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
				format!("failed to retrieve metadata for file: {}", path.display())
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
				let header_len_u64: u64 = try_into!(header_len)?;
				self.cur_size = header_len_u64 + 1;
			} else {
				self.cur_size = 0;
			}
		} else {
			self.cur_size = len;
		}

		self.last_rotation = Instant::now();
		Ok(())
	}

	#[cfg(test)]
	pub(crate) fn debug_process_resolve_frame_error(&mut self) {
		self.config.debug_process_resolve_frame_error = true;
	}

	#[cfg(test)]
	pub(crate) fn debug_invalid_metadata(&mut self) {
		self.config.debug_invalid_metadata = true;
	}

	#[cfg(test)]
	pub(crate) fn debug_lineno_is_none(&mut self) {
		self.config.debug_lineno_is_none = true;
	}
}

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
					let text = "log file cannot be rotated because there is no file associated with this logger";
					return Err(err!(ErrKind::Log, text));
				}
			}
		}

		let now: DateTime<Local> = Local::now();
		// standard rotation string format
		let rotation_string = now.format(".r_%m_%d_%Y_%T").to_string().replace(":", "-");

		// get the original file path
		let original_file_path = PathBuf::from(self.config.log_file_path.clone());

		// get the parent directory and the file name
		let ekind = ErrKind::IllegalArgument;
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
		if now
			.checked_duration_since(self.last_rotation)
			.unwrap_or(Duration::new(0, 0))
			.as_millis() > max_age_millis.into()
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

		if self.config.log_file_path != "" {
			let path = PathBuf::from(self.config.log_file_path.clone());
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
			self.check_open(&mut f, &path)?;

			let mut file = self.file.write()?;
			*file = Some(f);
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
	fn set_config_option(&mut self, value: LogConfigOptions) -> Result<(), Error> {
		if !self.is_init {
			let ekind = ErrKind::Log;
			let text = "log file options cannot be set because init() was never called";
			return Err(err!(ekind, text));
		}

		let name = value.name();

		if name == "LogFilePath" {
			return Err(err!(ErrKind::Log, "cannot modify log file path after init"));
		}

		match value.value_u64() {
			Some(v) => self.config.set_u64(name, v),
			None => {}
		}

		match value.value_bool() {
			Some(v) => self.config.set_bool(name, v),
			None => {}
		}

		match value.value_string() {
			Some(v) => self.config.set_string(name, v),
			None => {}
		}

		Ok(())
	}
}
