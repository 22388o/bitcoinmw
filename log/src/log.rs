// Copyright (c) 2023-2024, The BitcoinMW Developers
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

use crate::types::{Log, LogConfig, LogConfigOption, LogConfigOptionName, LogLevel};
use crate::u64;
use crate::LogConfigOption::{
	AutoRotate, Colors, DeleteRotation, FileHeader, FilePath, Level, LineNum, LineNumDataMaxLen,
	MaxAgeMillis, MaxSizeBytes, ShowBt, ShowMillis, Stdout, Timestamp,
};
use bmw_deps::backtrace;
use bmw_deps::backtrace::{Backtrace, Symbol};
use bmw_deps::chrono::{DateTime, Local};
use bmw_deps::colored::Colorize;
use bmw_deps::rand::random;
use bmw_err::{err, Error};
use std::cell::RefCell;
use std::fs::{canonicalize, remove_file, rename, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

const NEWLINE: &[u8] = &['\n' as u8];

#[derive(PartialEq)]
enum LogType {
	Regular,
	All,
	Plain,
}

#[derive(Debug, Clone)]
struct LogImpl {
	config: LogConfig,
	init: bool,
	file: Option<Rc<RefCell<File>>>,
	cur_size: u64,
	last_rotation: Instant,
}

unsafe impl Send for LogImpl {}

unsafe impl Sync for LogImpl {}

impl Log for LogImpl {
	fn log_all(&mut self, level: LogLevel, line: &str, now: Option<Instant>) -> Result<(), Error> {
		self.log_impl(level, line, now, LogType::All)
	}
	fn log_plain(
		&mut self,
		level: LogLevel,
		line: &str,
		now: Option<Instant>,
	) -> Result<(), Error> {
		self.log_impl(level, line, now, LogType::Plain)
	}
	fn log(&mut self, level: LogLevel, line: &str, now: Option<Instant>) -> Result<(), Error> {
		self.log_impl(level, line, now, LogType::Regular)
	}

	fn rotate(&mut self) -> Result<(), Error> {
		if !self.init {
			return Err(err!(ErrKind::Log, "log not initialized"));
		}

		if self.file.is_none() {
			// no file, nothing to rotate
			return Ok(());
		}

		let now: DateTime<Local> = Local::now();
		let rotation_string = now.format(".r_%m_%d_%Y_%T").to_string().replace(":", "-");

		let original_file_path = match self.config.file_path.clone() {
			FilePath(file_path) => match file_path {
				Some(file_path) => file_path,
				None => {
					let text = "file_path must be of the for FilePath(Option<PathBuf>)";
					return Err(err!(ErrKind::IllegalArgument, text));
				}
			},
			_ => {
				let text = "file_path must be of the for FilePath(Option<PathBuf>)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		// get the parent directory and the file name
		let parent = match original_file_path.parent() {
			Some(parent) => parent,
			None => {
				let text = "file_path has an unexpected illegal value of None for parent";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		let file_name = match original_file_path.file_name() {
			Some(file_name) => file_name,
			None => {
				let text = "file_path has an unexpected illegal value of None for file_name";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		let file_name = file_name.to_str();
		if file_name.is_none() || self.config.debug_invalid_os_str {
			let text = "file_path has an unexpected illegal value of None for file_name";
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		let file_name = file_name.unwrap();

		let mut new_file_path_buf = parent.to_path_buf();
		let file_name = match file_name.rfind(".") {
			Some(pos) => &file_name[0..pos],
			_ => &file_name,
		};
		let file_name = format!("{}{}_{}.log", file_name, rotation_string, random::<u64>());
		new_file_path_buf.push(file_name);

		match self.config.delete_rotation {
			DeleteRotation(delete_rotation) => {
				if delete_rotation {
					remove_file(&original_file_path)?;
				} else {
					rename(&original_file_path, new_file_path_buf.clone())?;
				}
			}
			_ => {
				let text = "delete_rotation has an unexpected illegal value";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		}

		let mut open_options = OpenOptions::new();
		let open_options = open_options.append(true).create(true);
		let mut file = open_options.open(&original_file_path)?;
		self.check_open(&mut file, &original_file_path)?;
		self.file = Some(Rc::new(RefCell::new(file)));

		Ok(())
	}

	fn need_rotate(&self, now: Option<Instant>) -> Result<bool, Error> {
		if !self.init {
			return Err(err!(ErrKind::Log, "log not initialized"));
		}

		let now = now.unwrap_or(Instant::now());

		let max_age_millis = match self.config.max_age_millis {
			MaxAgeMillis(m) => m,
			_ => {
				// should not get here, but return an error if we do
				let text = "max_age_millis must be of form MaxAgeMillis(u128)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let max_size_bytes = match self.config.max_size_bytes {
			MaxSizeBytes(m) => m,
			_ => {
				// should not get here, but return an error if we do
				let text = "max_size_bytes must be of form MaxSizeBytes(u64)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		if now.duration_since(self.last_rotation).as_millis() > max_age_millis
			|| self.cur_size > max_size_bytes
		{
			Ok(true)
		} else {
			Ok(false)
		}
	}

	fn init(&mut self) -> Result<(), Error> {
		if self.init {
			return Err(err!(ErrKind::Log, "log already initialized"));
		}

		match self.config.file_path.clone() {
			FilePath(file_path) => match file_path {
				Some(file_path) => {
					let mut file = if file_path.exists() {
						OpenOptions::new().append(true).open(file_path.clone())
					} else {
						OpenOptions::new()
							.append(true)
							.create(true)
							.open(file_path.clone())
					}?;
					self.check_open(&mut file, &file_path)?;
					self.file = Some(Rc::new(RefCell::new(file)));
				}
				None => {}
			},
			_ => {
				// should not be able to get here
				let text = "file_path must be of the form FilePath(Option<PathBuf>)";
				return Err(err!(ErrKind::Log, text));
			}
		}

		self.init = true;

		Ok(())
	}
	fn set_config_option(&mut self, value: LogConfigOption) -> Result<(), Error> {
		match value {
			Colors(_) => {
				self.config.colors = value;
			}
			Stdout(_) => {
				self.config.stdout = value;
			}
			MaxSizeBytes(_) => {
				self.config.max_size_bytes = value;
			}
			MaxAgeMillis(_) => {
				self.config.max_age_millis = value;
			}
			Timestamp(_) => {
				self.config.timestamp = value;
			}
			Level(_) => {
				self.config.level = value;
			}
			LineNum(_) => {
				self.config.line_num = value;
			}
			ShowMillis(_) => {
				self.config.show_millis = value;
			}
			AutoRotate(_) => {
				self.config.auto_rotate = value;
			}
			FilePath(_) => {
				let text = "filepath cannot be set after log is initialized";
				return Err(err!(ErrKind::Log, text));
			}
			ShowBt(_) => {
				self.config.show_bt = value;
			}
			LineNumDataMaxLen(_) => {
				self.config.line_num_data_max_len = value;
			}
			DeleteRotation(_) => {
				self.config.delete_rotation = value;
			}
			FileHeader(_) => {
				self.config.file_header = value;
			}
		}

		Ok(())
	}
	fn get_config_option(&self, option: LogConfigOptionName) -> Result<&LogConfigOption, Error> {
		Ok(match option {
			LogConfigOptionName::Colors => &self.config.colors,
			LogConfigOptionName::Stdout => &self.config.stdout,
			LogConfigOptionName::MaxSizeBytes => &self.config.max_size_bytes,
			LogConfigOptionName::MaxAgeMillis => &self.config.max_age_millis,
			LogConfigOptionName::Timestamp => &self.config.timestamp,
			LogConfigOptionName::Level => &self.config.level,
			LogConfigOptionName::LineNum => &self.config.line_num,
			LogConfigOptionName::ShowMillis => &self.config.show_millis,
			LogConfigOptionName::AutoRotate => &self.config.auto_rotate,
			LogConfigOptionName::FilePath => &self.config.file_path,
			LogConfigOptionName::ShowBt => &self.config.show_bt,
			LogConfigOptionName::LineNumDataMaxLen => &self.config.line_num_data_max_len,
			LogConfigOptionName::DeleteRotation => &self.config.delete_rotation,
			LogConfigOptionName::FileHeader => &self.config.file_header,
		})
	}
}

impl LogImpl {
	fn new(config: LogConfig) -> Result<Self, Error> {
		Self::check_config(&config)?;
		Ok(Self {
			config,
			init: false,
			file: None,
			cur_size: 0,
			last_rotation: Instant::now(),
		})
	}

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

	fn process_resolve_frame(
		symbol: &Symbol,
		_config: &LogConfig,
		found_logger: &mut bool,
		logged_from_file: &mut String,
	) -> Result<bool, Error> {
		if _config.debug_process_resolve_frame_error {
			let e = err!(ErrKind::Test, "test resolve_frame error");
			return Err(e);
		}
		let mut found_frame = false;
		#[cfg(debug_assertions)]
		if let Some(filename) = symbol.filename() {
			let filename = filename.display().to_string();
			let lineno = symbol.lineno();

			let lineno = if lineno.is_none() || _config.debug_lineno_none {
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

	fn log_impl(
		&mut self,
		level: LogLevel,
		line: &str,
		now: Option<Instant>,
		log_type: LogType,
	) -> Result<(), Error> {
		if !self.init {
			return Err(err!(ErrKind::Log, "log not initialized"));
		}

		self.rotate_if_needed(now)?;
		let show_stdout = match self.config.stdout {
			Stdout(s) => s || log_type == LogType::All,
			_ => {
				let text = "stdout must be of the form Stdout(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let show_timestamp = match self.config.timestamp {
			Timestamp(t) => t && log_type != LogType::Plain,
			_ => {
				let text = "timestamp must be of the form Timestamp(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let show_colors = match self.config.colors {
			Colors(c) => c,
			_ => {
				let text = "colors must be of the form Colors(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let show_log_level = match self.config.level {
			Level(l) => l && log_type != LogType::Plain,
			_ => {
				let text = "log_level must be of the form LogLevel(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let show_line_num = match self.config.line_num {
			LineNum(l) => l && log_type != LogType::Plain,
			_ => {
				let text = "line_num must be of the form LineNum(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		let show_millis = match self.config.show_millis {
			ShowMillis(m) => m && log_type != LogType::Plain,
			_ => {
				let text = "show_millis must be of the form ShowMillis(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		let show_bt = match self.config.show_bt {
			ShowBt(b) => b && (level == LogLevel::Error || level == LogLevel::Fatal),
			_ => {
				let text = "show_bt must be of the form ShowBt(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		let file_is_some = self.file.is_some();
		if show_timestamp {
			let date = Local::now();
			let millis = date.timestamp_millis() % 1_000;
			let millis_format = self.format_millis(millis);
			let formatted_timestamp = if show_millis {
				format!("{}.{}", date.format("%Y-%m-%d %H:%M:%S"), millis_format)
			} else {
				format!("{}", date.format("%Y-%m-%d %H:%M:%S"))
			};

			if file_is_some {
				let formatted_timestamp = format!("[{}]: ", formatted_timestamp);
				let formatted_timestamp = formatted_timestamp.as_bytes();
				self.file
					.as_mut()
					.unwrap()
					.borrow_mut()
					.write(formatted_timestamp)?;
				let formatted_len: u64 = u64!(formatted_timestamp.len());
				self.cur_size += formatted_len;
			}

			if show_stdout {
				if show_colors {
					print!("[{}]: ", formatted_timestamp.to_string().dimmed());
				} else {
					print!("[{}]: ", formatted_timestamp);
				}
			}
		}
		if show_log_level {
			if file_is_some {
				let formatted_level = format!("({}) ", level);
				let formatted_level = formatted_level.as_bytes();
				self.file
					.as_mut()
					.unwrap()
					.borrow_mut()
					.write(formatted_level)?;
				let formatted_len: u64 = u64!(formatted_level.len());
				self.cur_size += formatted_len;
			}

			if show_stdout {
				if show_colors {
					match level {
						LogLevel::Trace | LogLevel::Debug => {
							print!("({}) ", format!("{}", level).magenta());
						}
						LogLevel::Info => {
							print!("({}) ", format!("{}", level).green());
						}
						LogLevel::Warn => {
							print!("({}) ", format!("{}", level).yellow());
						}
						LogLevel::Error | LogLevel::Fatal => {
							print!("({}) ", format!("{}", level).red());
						}
					}
				} else {
					print!("({}) ", level);
				}
			}
		}
		if show_line_num {
			let mut found_logger = false;
			let mut found_frame = false;
			let mut logged_from_file = "*********unknown**********".to_string();
			backtrace::trace(|frame| {
				backtrace::resolve_frame(frame, |symbol| {
					found_frame = match Self::process_resolve_frame(
						symbol,
						&self.config,
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
			let max_len = match self.config.line_num_data_max_len {
				LineNumDataMaxLen(max_len) => max_len,
				_ => {
					let text = "unexpected illegal value for LineNumDataMaxLen";
					return Err(err!(ErrKind::IllegalArgument, text));
				}
			};
			let len = logged_from_file.len();
			if len > max_len {
				let start = len.saturating_sub(max_len);
				logged_from_file = format!("..{}", &logged_from_file[start..]);
			}

			if file_is_some {
				let logged_from_file = format!("[{}]: ", logged_from_file);
				let logged_from_file = logged_from_file.as_bytes();
				self.file
					.as_mut()
					.unwrap()
					.borrow_mut()
					.write(logged_from_file)?;
				let logged_from_file_len: u64 = u64!(logged_from_file.len());
				self.cur_size += logged_from_file_len;
			}

			if show_stdout {
				if show_colors {
					print!("[{}]: ", logged_from_file.yellow());
				} else {
					print!("[{}]: ", logged_from_file);
				}
			}
		}

		if file_is_some {
			let line_bytes = line.as_bytes();
			let mut file = self.file.as_mut().unwrap().borrow_mut();

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

	fn rotate_if_needed(&mut self, now_param: Option<Instant>) -> Result<(), Error> {
		match self.config.auto_rotate {
			AutoRotate(r) => {
				if !r {
					return Ok(()); // auto rotate not enabled
				}
			}
			_ => {
				// should not get here, but return an error if we do
				let text = "autorotate must be of form AutoRotate(bool)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		}
		let now = match now_param {
			Some(now) => now,
			None => Instant::now(),
		};

		let max_age_millis = match self.config.max_age_millis {
			MaxAgeMillis(m) => m,
			_ => {
				// should not get here, but return an error if we do
				let text = "max_age_millis must be of form MaxAgeMillis(u128)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};
		let max_size_bytes = match self.config.max_size_bytes {
			MaxSizeBytes(m) => m,
			_ => {
				// should not get here, but return an error if we do
				let text = "max_size_bytes must be of form MaxSizeBytes(u64)";
				return Err(err!(ErrKind::IllegalArgument, text));
			}
		};

		if now.duration_since(self.last_rotation).as_millis() > max_age_millis
			|| self.cur_size > max_size_bytes
		{
			self.rotate()?;
		}

		Ok(())
	}

	fn check_config(config: &LogConfig) -> Result<(), Error> {
		match &config.file_path {
			FilePath(file_path) => match &file_path {
				Some(file_path) => {
					match canonicalize(file_path) {
						Ok(file_path) => {
							if file_path.is_dir() {
								let text = "file_path must not be a directory";
								return Err(err!(ErrKind::Configuration, text));
							}
						}
						Err(_) => {} // it might not have been created yet which is ok. Just check dir here.
					}

					if file_path.parent().is_none() {
						let text = "if file_path specifies a PathBuf, the PathBuf must\
not terminate in a root or a prefix";
						return Err(err!(ErrKind::Configuration, text));
					}
				}
				None => {}
			},
			_ => {
				let text = "file_path must be of the form FilePath(Option<PathBuf>)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.colors {
			Colors(_) => {}
			_ => {
				let text = "colors must be of the form Colors(bool)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.stdout {
			Stdout(_) => {}
			_ => {
				let text = "stdout must be of the form Stdout(bool)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.max_size_bytes {
			MaxSizeBytes(_) => {}
			_ => {
				let text = "max_size_bytes must be of the form MaxSizeBytes(u64)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.max_age_millis {
			MaxAgeMillis(_) => {}
			_ => {
				let text = "max_age_millis must be of the form MaxAgeMillis(u64)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}

		match config.timestamp {
			Timestamp(_) => {}
			_ => {
				let text = "timestamp must be of the form Timestamp(bool)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.level {
			Level(_) => {}
			_ => {
				let text = "level must be of the form Level(bool)";
				return Err(err!(ErrKind::Configuration, text));
			}
		}
		match config.line_num {
			LineNum(_) => {}
			_ => {
				let text = "line_num must be of the form LineNum(bool)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}
		match config.show_millis {
			ShowMillis(_) => {}
			_ => {
				let text = "show_millis must be of the form ShowMillis(bool)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}
		match config.auto_rotate {
			AutoRotate(_) => {}
			_ => {
				let text = "auto_rotate must be of the form AutoRotate(bool)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}
		match config.show_bt {
			ShowBt(_) => {}
			_ => {
				let text = "show_bt must be of the form ShowBt(bool)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}
		match config.line_num_data_max_len {
			LineNumDataMaxLen(_) => {}
			_ => {
				let text = "line_num_data_max_len must be of the form LineNumDataMaxLen(u64)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}
		match config.delete_rotation {
			DeleteRotation(_) => {}
			_ => {
				let text = "delete_rotation must be of the form DeleteRotation(bool)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}

		match config.file_header {
			FileHeader(_) => {}
			_ => {
				let text = "file_header must be of the form FileHeader(String)";
				let e = err!(ErrKind::Configuration, text);
				return Err(e);
			}
		}

		Ok(())
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
			match &self.config.file_header {
				FileHeader(header) => {
					if header.len() > 0 {
						// there's a header. We need to append it.
						file.write(header.as_bytes())?;
						file.write(NEWLINE)?;
						self.cur_size = u64!(header.len()) + 1;
					} else {
						self.cur_size = 0;
					}
				}
				_ => {
					let text = "file_header must be of the form FileHeader(String)";
					let e = err!(ErrKind::Configuration, text);
					return Err(e);
				}
			}
		} else {
			self.cur_size = len;
		}

		// update our canonicalization. This makes it so if we change directories later we
		// have the full path to the log file.
		self.config.file_path = FilePath(Some(canonicalize(path)?));
		self.last_rotation = Instant::now();
		Ok(())
	}
}

/// The publicly accessible builder struct. This is the only way a log can be created outside of
/// this crate. See [`LogBuilder::build`].
pub struct LogBuilder {}

impl LogBuilder {
	/// Build a [`crate::Log`] based on specified [`crate::LogConfig`] or return a
	/// [`bmw_err::Error`] if the configuration is invalid.
	pub fn build_send_sync(config: LogConfig) -> Result<Box<dyn Log + Send + Sync>, Error> {
		Ok(Box::new(LogImpl::new(config)?))
	}

	pub fn build(config: LogConfig) -> Result<Box<dyn Log>, Error> {
		Ok(Box::new(LogImpl::new(config)?))
	}
}

#[cfg(test)]
mod test {
	use crate::log::LogImpl;
	use crate::types::Log;
	use crate::LogConfigOption::{
		AutoRotate, Colors, DeleteRotation, FileHeader, FilePath, Level, LineNum,
		LineNumDataMaxLen, MaxAgeMillis, MaxSizeBytes, ShowBt, ShowMillis, Stdout, Timestamp,
	};
	use crate::{LogBuilder, LogConfig, LogConfigOption, LogConfigOptionName, LogLevel};
	use bmw_err::Error;
	use bmw_test::*;
	use std::fs::{read_dir, read_to_string};
	use std::path::PathBuf;
	use std::thread::sleep;
	use std::time::Duration;
	use std::time::Instant;

	#[test]
	fn test_bad_configs() -> Result<(), Error> {
		assert!(LogBuilder::build(LogConfig::default()).is_ok());

		assert!(LogBuilder::build(LogConfig {
			colors: Colors(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			colors: DeleteRotation(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			stdout: Stdout(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			stdout: DeleteRotation(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			max_size_bytes: MaxSizeBytes(1_000_000),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			max_size_bytes: Level(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			max_age_millis: MaxAgeMillis(3_600_000),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			max_age_millis: Colors(true),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			timestamp: Timestamp(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			timestamp: FilePath(None),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			level: Level(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			level: ShowBt(true),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			line_num: LineNum(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			line_num: FileHeader("file_header_value".to_string()),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			show_millis: ShowMillis(true),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			show_millis: DeleteRotation(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			auto_rotate: AutoRotate(true),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			auto_rotate: Stdout(false),
			..Default::default()
		})
		.is_err());

		let mut path_buf = PathBuf::new();
		path_buf.push("./anything");
		assert!(LogBuilder::build(LogConfig {
			file_path: FilePath(Some(path_buf)),
			..Default::default()
		})
		.is_ok());

		// error because it's a root path
		let path_buf = PathBuf::new();
		assert!(LogBuilder::build(LogConfig {
			file_path: FilePath(Some(path_buf)),
			..Default::default()
		})
		.is_err());

		// error because it's a directory
		let mut path_buf = PathBuf::new();
		path_buf.push(".");
		assert!(LogBuilder::build(LogConfig {
			file_path: FilePath(Some(path_buf)),
			..Default::default()
		})
		.is_err());

		// none is ok (stdout only)
		assert!(LogBuilder::build(LogConfig {
			file_path: FilePath(None),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			file_path: Stdout(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			show_bt: ShowBt(true),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			show_bt: Stdout(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			line_num_data_max_len: LineNumDataMaxLen(30),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			line_num_data_max_len: FileHeader("".to_string()),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			delete_rotation: DeleteRotation(false),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			delete_rotation: Colors(false),
			..Default::default()
		})
		.is_err());

		assert!(LogBuilder::build(LogConfig {
			file_header: FileHeader("some_value".to_string()),
			..Default::default()
		})
		.is_ok());

		assert!(LogBuilder::build(LogConfig {
			file_header: AutoRotate(false),
			..Default::default()
		})
		.is_err());

		Ok(())
	}

	#[test]
	fn test_config_options() -> Result<(), Error> {
		let mut log = LogBuilder::build(LogConfig::default())?;

		assert_eq!(
			log.get_config_option(LogConfigOptionName::Colors)?,
			&Colors(true)
		);
		log.set_config_option(Colors(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::Colors)?,
			&Colors(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::Stdout)?,
			&Stdout(true)
		);
		log.set_config_option(Stdout(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::Stdout)?,
			&Stdout(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::MaxAgeMillis)?,
			&MaxAgeMillis(60 * 60 * 1000)
		);
		log.set_config_option(MaxAgeMillis(30 * 60 * 1000))?;

		assert_eq!(
			log.get_config_option(LogConfigOptionName::MaxAgeMillis)?,
			&MaxAgeMillis(30 * 60 * 1000)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::Timestamp)?,
			&Timestamp(true)
		);
		log.set_config_option(Timestamp(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::Timestamp)?,
			&Timestamp(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::Level)?,
			&Level(true)
		);
		log.set_config_option(Level(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::Level)?,
			&Level(false)
		);

		#[cfg(windows)]
		{
			assert_eq!(
				log.get_config_option(LogConfigOptionName::LineNum)?,
				&LineNum(false)
			);
		}
		#[cfg(not(windows))]
		{
			assert_eq!(
				log.get_config_option(LogConfigOptionName::LineNum)?,
				&LineNum(true)
			);

			log.set_config_option(LineNum(false))?;
			assert_eq!(
				log.get_config_option(LogConfigOptionName::LineNum)?,
				&LineNum(false)
			);
		}

		assert_eq!(
			log.get_config_option(LogConfigOptionName::ShowMillis)?,
			&ShowMillis(true)
		);
		log.set_config_option(ShowMillis(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::ShowMillis)?,
			&ShowMillis(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::AutoRotate)?,
			&AutoRotate(true)
		);
		log.set_config_option(AutoRotate(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::AutoRotate)?,
			&AutoRotate(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::FilePath)?,
			&FilePath(None)
		);
		let mut path = PathBuf::new();
		path.push("./anything");
		// file_path cannot be set
		assert!(log.set_config_option(FilePath(Some(path.clone()))).is_err());

		assert_eq!(
			log.get_config_option(LogConfigOptionName::ShowBt)?,
			&ShowBt(true)
		);
		log.set_config_option(ShowBt(false))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::ShowBt)?,
			&ShowBt(false)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::LineNumDataMaxLen)?,
			&LineNumDataMaxLen(25)
		);
		log.set_config_option(LineNumDataMaxLen(35))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::LineNumDataMaxLen)?,
			&LineNumDataMaxLen(35)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::DeleteRotation)?,
			&DeleteRotation(false)
		);
		log.set_config_option(DeleteRotation(true))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::DeleteRotation)?,
			&DeleteRotation(true)
		);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::FileHeader)?,
			&FileHeader("".to_string())
		);
		log.set_config_option(FileHeader("anything".to_string()))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::FileHeader)?,
			&FileHeader("anything".to_string())
		);

		Ok(())
	}

	#[test]
	fn test_init() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(format!("{}/test.log", test_dir)))),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;
		log.log(LogLevel::Info, "test", None)?;
		log.log(LogLevel::Debug, "test2", None)?;

		assert!(PathBuf::from(format!("{}/test.log", test_dir)).exists());

		Ok(())
	}

	#[test]
	fn test_configuration_options() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);

		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};

		let mut log = LogBuilder::build(config)?;
		log.init()?;
		log.set_config_option(ShowBt(false))?;
		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;

		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("trace"), Some(66));
		assert_eq!(contents.rfind("debug"), Some(138));
		assert_eq!(contents.rfind("info"), Some(209));
		assert_eq!(contents.rfind("warn"), Some(279));
		assert_eq!(contents.rfind("error"), Some(350));
		assert_eq!(contents.rfind("fatal"), Some(422));
		assert_eq!(contents.len(), 428);

		//try with a header
		let log_file = format!("{}/test2.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			file_header: FileHeader("this is a test".to_string()),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		// original location + length of header + 1 for the newline
		assert_eq!(
			contents.rfind("trace"),
			Some(66 + "this is a test".len() + 1)
		);

		// no millis
		let log_file = format!("{}/test3.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			show_millis: ShowMillis(false),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(
			contents.rfind("trace"),
			Some(66 - 4) // millis are four chars (one for the dot and three decimal places).
		);

		// no level
		let log_file = format!("{}/test4.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			level: Level(false),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(
			contents.rfind("trace"),
			Some(66 - 8) // len is 5 for "TRACE" and the '(' and ')' and one space.
		);

		// no timestamp
		let log_file = format!("{}/test5.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			timestamp: Timestamp(false),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("trace"), Some(39));

		// no linenum
		let log_file = format!("{}/test6.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			line_num: LineNum(false),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("trace"), Some(66 - 31)); // 31 less (25 maxlen, 2 brackets, colon, space, and two dots

		// 20 as line_num_data_max_len (default is 25).
		let log_file = format!("{}/test7.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			line_num_data_max_len: LineNumDataMaxLen(20),
			line_num: LogConfigOption::LineNum(true),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Trace, "trace", None)?;
		log.log(LogLevel::Debug, "debug", None)?;
		log.log(LogLevel::Info, "info", None)?;
		log.log(LogLevel::Warn, "warn", None)?;
		log.log(LogLevel::Error, "error", None)?;
		log.log(LogLevel::Fatal, "fatal", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("trace"), Some(66 - 5)); // 31 less (25 maxlen, 2 brackets, colon, space, and two dots

		// test a dynamic configuration change
		let log_file = format!("{}/test8.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			line_num_data_max_len: LineNumDataMaxLen(20),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;

		log.log(LogLevel::Info, "abc", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("abc"), Some(60));
		log.set_config_option(ShowMillis(false))?;
		log.log(LogLevel::Info, "abc", None)?;
		let contents = read_to_string(log_file.clone())?;
		assert_eq!(contents.rfind("abc"), Some(120));

		Ok(())
	}

	#[test]
	fn test_log_rotation() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_size_bytes: MaxSizeBytes(100),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		log.init()?;
		log.log(LogLevel::Info, "1", None)?;
		log.log(LogLevel::Info, "2", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 1);

		// this triggers an auto rotate
		log.log(LogLevel::Info, "3", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_manual_rotation() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_size_bytes: MaxSizeBytes(100),
			auto_rotate: AutoRotate(false),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		log.init()?;
		log.log(LogLevel::Info, "a", None)?;
		log.log(LogLevel::Info, "b", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 1);

		// this would trigger an auto rotate, but set to false so it doesn't
		log.log(LogLevel::Info, "c", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 1);

		assert!(log.need_rotate(None)?);
		log.rotate()?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 2);
		assert_eq!(log.need_rotate(None)?, false);

		Ok(())
	}

	#[test]
	fn test_time_based_rotation() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_age_millis: MaxAgeMillis(10_000),
			auto_rotate: AutoRotate(false),
			show_bt: ShowBt(false),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		log.init()?;
		log.log(LogLevel::Info, "a", None)?;
		log.log(LogLevel::Info, "b", None)?;
		assert_eq!(log.need_rotate(None)?, false);
		sleep(Duration::from_millis(20_000));
		assert_eq!(log.need_rotate(None)?, true);
		log.rotate()?;
		assert_eq!(log.need_rotate(None)?, false);

		let log_file = format!("{}/othertestlog", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_age_millis: MaxAgeMillis(10_000),
			auto_rotate: AutoRotate(false),
			line_num: LogConfigOption::LineNum(true),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		log.init()?;
		log.log(LogLevel::Info, "a", None)?;
		log.log(LogLevel::Info, "b", None)?;
		assert_eq!(log.need_rotate(None)?, false);
		sleep(Duration::from_millis(20_000));
		assert_eq!(log.need_rotate(None)?, true);
		log.rotate()?;
		assert_eq!(log.need_rotate(None)?, false);

		Ok(())
	}

	#[test]
	fn test_other_situations() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			stdout: Stdout(false),
			line_num: LogConfigOption::LineNum(true),
			max_age_millis: MaxAgeMillis(1_000),
			auto_rotate: AutoRotate(false),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		assert!(log.rotate().is_err());
		assert!(log.log(LogLevel::Info, "a", None).is_err());
		assert!(log.need_rotate(None).is_err());

		log.init()?;
		log.log(LogLevel::Info, "a", None)?;
		log.log_all(LogLevel::Info, "b", None)?;
		let contents = read_to_string(log_file.clone())?;

		// both entries logged
		assert_eq!(contents.len(), 134);

		// log without a file and try to rotate
		let config = LogConfig::default();
		let mut log = LogBuilder::build(config)?;
		log.init()?;
		log.rotate()?; // should return without error

		// use log impl to get into situations we normally wouldn't see
		let mut log_as_impl = LogImpl::new(LogConfig {
			file_path: FilePath(Some(PathBuf::from(format!("{}/test.log", test_dir)))),
			..Default::default()
		})?;
		log_as_impl.init()?;
		log_as_impl.log(LogLevel::Info, "a", None)?;
		log_as_impl.config.file_path = FilePath(None);
		assert!(log_as_impl.rotate().is_err());
		log_as_impl.config.file_path = FileHeader("".to_string());
		assert!(log_as_impl.rotate().is_err());
		log_as_impl.config.file_path = FilePath(Some(PathBuf::new()));
		assert!(log_as_impl.rotate().is_err());
		log_as_impl.config.file_path = FilePath(Some(PathBuf::from(format!("{}/..", test_dir))));
		assert!(log_as_impl.rotate().is_err());
		log_as_impl.config.stdout = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.stdout = Stdout(false);
		log_as_impl.config.timestamp = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.timestamp = Timestamp(false);
		log_as_impl.config.level = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.level = Level(false);
		log_as_impl.config.line_num = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.line_num = LineNum(false);
		log_as_impl.config.show_millis = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.show_millis = ShowMillis(false);
		log_as_impl.config.show_bt = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.show_bt = ShowBt(true);
		log_as_impl.config.colors = AutoRotate(false);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.colors = Colors(false);
		log_as_impl.config.stdout = Stdout(true);
		log_as_impl.config.timestamp = Timestamp(true);
		log_as_impl.config.level = Level(true);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_ok());
		log_as_impl.config.line_num_data_max_len = AutoRotate(false);
		log_as_impl.config.line_num = LineNum(true);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_err());
		log_as_impl.config.colors = Colors(false);
		log_as_impl.config.stdout = Stdout(true);
		log_as_impl.config.line_num = LineNum(true);
		log_as_impl.config.line_num_data_max_len = LineNumDataMaxLen(20);
		assert!(log_as_impl.log(LogLevel::Info, "test", None).is_ok());
		log_as_impl.config.file_header = AutoRotate(false);
		assert!(log_as_impl.rotate().is_err());
		log_as_impl.config.auto_rotate = Colors(false);
		assert!(log_as_impl
			.log(LogLevel::Info, "test", Some(Instant::now()))
			.is_err());
		log_as_impl.config.auto_rotate = AutoRotate(true);
		log_as_impl.config.max_age_millis = AutoRotate(true);
		assert!(log_as_impl
			.log(LogLevel::Info, "test", Some(Instant::now()))
			.is_err());
		log_as_impl.config.max_age_millis = MaxAgeMillis(100_000);
		log_as_impl.config.max_size_bytes = AutoRotate(false);
		assert!(log_as_impl
			.log(LogLevel::Info, "test", Some(Instant::now()))
			.is_err());

		assert!(LogImpl::new(LogConfig {
			file_path: FilePath(Some(PathBuf::from(test_dir))),
			..Default::default()
		})
		.is_err());

		assert!(LogImpl::new(LogConfig {
			file_path: FilePath(Some(PathBuf::from(format!("{}/..", test_dir)))),
			..Default::default()
		})
		.is_err());

		Ok(())
	}

	#[test]
	fn test_delete_rotation() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_size_bytes: MaxSizeBytes(100),
			auto_rotate: AutoRotate(false),
			delete_rotation: DeleteRotation(true),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;

		log.init()?;
		log.log(LogLevel::Info, "a", None)?;
		log.log(LogLevel::Info, "b", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 1);

		// this would trigger an auto rotate, but set to false so it doesn't
		log.log(LogLevel::Info, "c", None)?;

		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}
		assert_eq!(count, 1);

		assert!(log.need_rotate(None)?);
		log.rotate()?;
		let files = read_dir(test_dir).unwrap();
		let mut count = 0;
		for file in files {
			let file = file.unwrap().path().display().to_string();
			if file.find(".log").is_some() {
				count += 1;
			}
		}

		// rotation occurs but it's deleted so still just one log file
		assert_eq!(count, 1);

		// log some more
		log.log(LogLevel::Info, "a", None)?;
		log.log(LogLevel::Info, "b", None)?;
		log.log(LogLevel::Info, "c", None)?;
		assert!(log.need_rotate(None)?);

		// use the log_impl
		let log_file = format!("{}/test2.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			max_size_bytes: MaxSizeBytes(100),
			auto_rotate: AutoRotate(false),
			delete_rotation: DeleteRotation(true),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogImpl::new(config)?;

		// test a bad filepath
		log.config.file_path = AutoRotate(true);
		assert!(log.init().is_err());
		// fix to correct path
		log.config.file_path = FilePath(Some(PathBuf::from(log_file.clone())));

		log.init()?;

		log.log(LogLevel::Info, "x", None)?;
		log.log(LogLevel::Info, "y", None)?;
		log.log(LogLevel::Info, "z", None)?;
		assert!(log.need_rotate(None)?);
		// set the delete_rotation to an illegal value
		log.config.delete_rotation = AutoRotate(true);
		assert!(log.rotate().is_err());

		// test need_rotate with bad values
		log.config.max_size_bytes = AutoRotate(true);
		assert!(log.need_rotate(None).is_err());

		log.config.max_age_millis = AutoRotate(true);
		assert!(log.need_rotate(None).is_err());
		assert!(log.init().is_err());

		Ok(())
	}

	#[test]
	fn test_other_config_options() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		const STR: &str = "01234567890123456789012345678901234567890123456789";

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			auto_rotate: AutoRotate(false),
			delete_rotation: DeleteRotation(true),
			show_bt: ShowBt(false),
			..Default::default()
		};
		let mut log = LogImpl::new(config)?;
		log.init()?;

		log.log(LogLevel::Info, STR, None)?;
		log.log(LogLevel::Info, STR, None)?;
		log.log(LogLevel::Info, STR, None)?;
		log.log(LogLevel::Info, STR, None)?;
		log.log(LogLevel::Info, STR, None)?;

		// this is more than 100 bytes check if rotation needed.

		assert!(!log.need_rotate(None)?);

		assert_eq!(
			log.get_config_option(LogConfigOptionName::MaxSizeBytes)?,
			&MaxSizeBytes(1024 * 1024)
		);
		log.set_config_option(MaxSizeBytes(100))?;
		assert_eq!(
			log.get_config_option(LogConfigOptionName::MaxSizeBytes)?,
			&MaxSizeBytes(100)
		);

		// we now need a rotation
		assert!(log.need_rotate(None)?);

		Ok(())
	}

	#[test]
	fn test_log_file_and_bt() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			show_bt: ShowBt(true),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;
		log.log(LogLevel::Debug, "test", None)?;
		let contents = read_to_string(log_file.clone())?;
		// since we're info level we don't log the bt, so standard length for info
		// with all options set
		assert_eq!(contents.len(), 71);

		// now with bt
		let log_file = format!("{}/test2.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			show_bt: ShowBt(true),
			line_num: LogConfigOption::LineNum(true),
			..Default::default()
		};
		let mut log = LogBuilder::build(config)?;
		log.init()?;
		log.log(LogLevel::Error, "test", None)?;
		let contents = read_to_string(log_file.clone())?;
		// we don't know exactly how big the stack trace will be so just assert it's larger
		assert!(contents.len() > 71);

		Ok(())
	}

	#[test]
	fn test_invalid_flags() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let log_file = format!("{}/test.log", test_dir);
		let config = LogConfig {
			file_path: FilePath(Some(PathBuf::from(log_file.clone()))),
			debug_invalid_metadata: true,
			..Default::default()
		};
		let mut log = LogImpl::new(config)?;
		assert!(log.init().is_err());

		log.config.debug_invalid_metadata = false;
		log.config.file_header = AutoRotate(true);
		assert!(log.init().is_err());
		log.config.file_header = FileHeader("".to_string());
		log.config.debug_invalid_os_str = true;
		log.init()?;
		assert!(log.rotate().is_err());
		log.config.debug_invalid_os_str = false;
		log.config.debug_lineno_none = true;
		assert!(log.log(LogLevel::Info, "", None).is_ok());

		Ok(())
	}

	#[test]
	fn test_format_millis() -> Result<(), Error> {
		let log = LogImpl::new(LogConfig::default())?;
		assert_eq!(log.format_millis(777), "777".to_string());
		assert_eq!(log.format_millis(77), "077".to_string());
		assert_eq!(log.format_millis(7), "007".to_string());
		assert_eq!(log.format_millis(0), "000".to_string());
		Ok(())
	}

	#[test]
	fn test_resolve_frame_error() -> Result<(), Error> {
		let config = LogConfig {
			debug_process_resolve_frame_error: true,
			..Default::default()
		};
		let mut log = LogImpl::new(config)?;
		log.init()?;

		// this will return Ok(()) even though the error occurs.
		assert_eq!(log.log(LogLevel::Info, "", None), Ok(()));
		Ok(())
	}
}
