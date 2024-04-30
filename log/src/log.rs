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
use crate::log::url_path::UrlPath;
use crate::types::LogConfig;
pub use crate::types::{LogLevel, LoggingType};
use crate::LogErrorKind::*;
use bmw_core::backtrace::Backtrace as LocalBacktrace;
use bmw_core::backtrace::Symbol;
use bmw_core::chrono::{DateTime, Local};
use bmw_core::colored::Colorize;
use bmw_core::rand::random;
use bmw_core::*;
use std::fmt::{Display, Formatter};
use std::fs::{remove_file, rename, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[doc(hidden)]
pub const DEFAULT_LINE_NUM_DATA_MAX_LEN: u16 = 30;

#[class{
    /// Log trait
    public logger;
    protected debug_logger;
    clone logger;
    /// The maximum age, in milliseconds, before a log should be rotated.
    const max_age_millis: u64 = u64::MAX;
    /// The maximum size, in bytes, before a log file should be rotated.
    const max_size_bytes: u64 = u64::MAX;
    /// The maximum length, in bytes, of the line number data which, if
    /// enabled, is included in the log line.
    const line_num_data_max_len: u16 = DEFAULT_LINE_NUM_DATA_MAX_LEN;
    /// The path of log file. If this value is set to "", file logging is
    /// disabled.
    const log_file_path: String = "".to_string();
    /// If [`true`], print logged data to standard output.
    const stdout: bool = true;
    /// If set to true, colors are displayed to make the log lines easier to
    /// read. Note that colors are only enabled on stdout.
    const colors: bool = true;
    /// If set to true, timestamps are displayed on each log line.
    const timestamp: bool = true;
    /// If set to true, milliseconds precision is displayed.
    const show_millis: bool = true;
    ///  If set to true, log level is displayed on each log line.
    const log_level: bool = true;
    /// If set to true, line number data are displayed on each log line.
    const line_num: bool = true;
    ///  If set to true, a backtrace will be logged when data are logged at
    ///  the [`crate::LogLevel::Error`] or [`crate::LogLevel::Fatal`] level.
    const backtrace: bool = false;
    /// If set to true, log files are automatically rotated. Rotation can
    /// only occur at the time data is logged.
    const auto_rotate: bool = false;
    /// If set to true, log files are immidately deleted upon log rotation.
    /// This option is useful for long running tests where logging to disk
    /// may result in running out of disk space. This value _MUST_ be set
    /// to false in a production environment.
    const delete_rotation: bool = false;
    /// The header to include at the top of all log files.
    const file_header: String = "".to_string();
    var cur_log_level: LogLevel;
    var cur_size: u64;
    var file: Arc<RwLock<Option<File>>>;
    var is_init: bool;
    var last_rotation: Instant;
    var log_config: LogConfig;
    var log_file_path_canonicalized: String;
    var debug_invalid_metadata: bool;
    var debug_lineno_is_none: bool;
    var debug_process_resolve_frame_error: bool;

    fn builder(&const_values) -> Result<Self, Error> {
        let home_dir = dirs::home_dir()
            .unwrap_or(PathBuf::new())
            .as_path()
            .display()
            .to_string();
        let file = Arc::new(RwLock::new(None));
        let log_file_path_canonicalized = const_values.log_file_path.replace("~", &home_dir);

        // to check if the file is ok we try to canonicalize its parent, but only if a file
        // is specified (len > 0). If there is no parent directory, canonicalize will fail
        if log_file_path_canonicalized.len() > 0 {
            let mut path_buf = PathBuf::from(&log_file_path_canonicalized);
            path_buf.pop();
            let path_buf_str = path_buf.display().to_string();
            let url_path = UrlPath::new(&path_buf_str).normalize();
            // make sure we can canonicalize the path
            PathBuf::from(url_path).as_path().canonicalize()?;
        }

        let debug_process_resolve_frame_error = false;
        let debug_lineno_is_none = false;
        let debug_invalid_metadata = false;

        if const_values.max_age_millis < MINIMUM_MAX_AGE_MILLIS {
            let text = format!("MaxAgeMillis must be at least {}", MINIMUM_MAX_AGE_MILLIS);
            err!(Configuration, text)
        } else if const_values.max_size_bytes < MINIMUM_MAX_SIZE_BYTES {
            let text = format!("MaxSizeBytes must be at least {}", MINIMUM_MAX_SIZE_BYTES);
            err!(Configuration, text)
        } else if const_values.line_num_data_max_len < MINIMUM_LNDML {
            let text = format!("LineNumDataMaxLen must be at least {}", MINIMUM_LNDML);
            err!(Configuration, text)
        } else {
            let log_config = LogConfig {
                max_age_millis: const_values.max_age_millis,
                max_size_bytes: const_values.max_size_bytes,
                line_num_data_max_len: const_values.line_num_data_max_len,
                stdout: const_values.stdout,
                colors: const_values.colors,
                timestamp: const_values.timestamp,
                show_millis: const_values.show_millis,
                log_level: const_values.log_level,
                line_num: const_values.line_num,
                backtrace: const_values.backtrace,
                auto_rotate: const_values.auto_rotate,
                file_header: const_values.file_header.clone(),
                delete_rotation: const_values.delete_rotation,
            };
            Ok(Self {
                cur_log_level: LogLevel::Info,
                cur_size: 0,
                file,
                is_init: false,
                last_rotation: Instant::now(),
                log_file_path_canonicalized,
                debug_process_resolve_frame_error,
                debug_lineno_is_none,
                debug_invalid_metadata,
                log_config,
            })
        }
    }

    [logger, debug_logger]
    fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
        self.log_impl(level, line, LoggingType::Standard)
    }

    [logger, debug_logger]
    fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
         self.log_impl(level, line, LoggingType::All)
    }

    [logger, debug_logger]
    fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
        self.log_impl(level, line, LoggingType::Plain)
    }

    [logger, debug_logger]
    fn need_rotate(&self) -> Result<bool, Error> {
        self.need_rotate_impl()
    }

    [logger, debug_logger]
    fn rotate(&mut self) -> Result<(), Error> {
        self.rotate_impl()
    }

    [logger, debug_logger]
    fn set_log_level(&mut self, level: LogLevel) {
        self.set_log_level_impl(level)
    }

    [logger, debug_logger]
    fn init(&mut self) -> Result<(), Error> {
        self.init_impl()
    }

    [logger, debug_logger]
    fn close(&mut self) -> Result<(), Error> {
        self.close_impl()
    }

    [logger, debug_logger]
    fn set_log_option(&mut self, value: LogConstOptions) -> Result<(), Error> {
        self.set_log_option_impl(value)
    }

    [debug_logger]
    fn set_debug_lineno_is_none(&mut self, value: bool) {
        *self.get_mut_debug_lineno_is_none() = value;
    }

    [debug_logger]
    fn set_debug_process_resolve_frame_error(&mut self, value: bool) {
        *self.get_mut_debug_process_resolve_frame_error() = value;
    }

    [debug_logger]
    fn set_debug_invalid_metadata(&mut self, value: bool) {
        *self.get_mut_debug_invalid_metadata() = value;
    }

    [debug_logger]
    fn get_log_config_debug(&self) -> LogConfig {
        self.get_log_config().clone()
    }
}]
impl Log {}

// convenience macro
macro_rules! some_or_err {
	($m:expr, $errkind:expr, $text:expr) => {{
		match $m {
			Some(m) => Ok(m),
			None => err!($errkind, $text),
		}
	}};
}

// convenience macro
macro_rules! none_or_err {
	($m:expr, $errkind:expr, $text:expr) => {{
		match $m {
			Some(_) => err!($errkind, $text),
			None => Ok(()),
		}
	}};
}

impl Log {
	fn set_log_option_impl(&mut self, value: LogConstOptions) -> Result<(), Error> {
		if !*self.get_is_init() {
			err!(
				NotInitialized,
				"logger has not been initalized. Call init() first."
			)
		} else {
			match value {
				LogConstOptions::Colors(v) => (*self.get_mut_log_config()).colors = v,
				LogConstOptions::Stdout(v) => (*self.get_mut_log_config()).stdout = v,
				LogConstOptions::MaxAgeMillis(v) => (*self.get_mut_log_config()).max_age_millis = v,
				LogConstOptions::MaxSizeBytes(v) => (*self.get_mut_log_config()).max_size_bytes = v,
				LogConstOptions::LineNumDataMaxLen(v) => {
					(*self.get_mut_log_config()).line_num_data_max_len = v
				}
				LogConstOptions::Timestamp(v) => (*self.get_mut_log_config()).timestamp = v,
				LogConstOptions::ShowMillis(v) => (*self.get_mut_log_config()).show_millis = v,
				LogConstOptions::LogLevel(v) => (*self.get_mut_log_config()).log_level = v,
				LogConstOptions::LineNum(v) => (*self.get_mut_log_config()).line_num = v,
				LogConstOptions::Backtrace(v) => (*self.get_mut_log_config()).backtrace = v,
				LogConstOptions::AutoRotate(v) => (*self.get_mut_log_config()).auto_rotate = v,
				LogConstOptions::DeleteRotation(v) => {
					(*self.get_mut_log_config()).delete_rotation = v
				}
				LogConstOptions::FileHeader(v) => {
					(*self.get_mut_log_config()).file_header = v.to_string()
				}
				LogConstOptions::LogFilePath(_) => {
					return err!(
						IllegalArgument,
						"cannot set log file path after initialization"
					)
				}
			}
			Ok(())
		}
	}

	fn init_impl(&mut self) -> Result<(), Error> {
		if *self.get_is_init() {
			// init already was called
			return err!(AlreadyInitialized, "log file has already ben initialized");
		}
		{
			let file = self.get_file().read()?;
			let text = "log.init() has already been called";
			none_or_err!((*file).as_ref(), IllegalState, text)?;
		}

		if self.get_log_file_path_canonicalized() != "" {
			let path = PathBuf::from(self.get_log_file_path().clone());
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

			let mut file = self.get_mut_file().write()?;
			*file = Some(f);
		}
		*self.get_mut_is_init() = true;

		Ok(())
	}

	fn log_impl(
		&mut self,
		level: LogLevel,
		line: &str,
		logging_type: LoggingType,
	) -> Result<(), Error> {
		if !*self.get_is_init() {
			return err!(
				NotInitialized,
				"logger has not been initalized. Call init() first."
			);
		}

		if level as usize >= *self.get_cur_log_level() as usize {
			self.rotate_if_needed()?;
			let show_stdout = (*self.get_log_config()).stdout || logging_type == LoggingType::All;
			let show_timestamp =
				(*self.get_log_config()).timestamp && logging_type != LoggingType::Plain;
			let show_colors = (*self.get_log_config()).colors;
			let show_log_level =
				(*self.get_log_config()).log_level && logging_type != LoggingType::Plain;
			let show_line_num =
				(*self.get_log_config()).line_num && logging_type != LoggingType::Plain;
			let show_millis =
				(*self.get_log_config()).show_millis && logging_type != LoggingType::Plain;
			let show_bt =
				(*self.get_log_config()).backtrace && level as usize >= LogLevel::Error as usize;
			let max_len = (*self.get_log_config()).line_num_data_max_len;

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
		max_len: u16,
		line: &str,
		logging_type: LoggingType,
	) -> Result<(), Error> {
		let debug_lineno_is_none = *self.get_debug_lineno_is_none();
		let debug_process_resolve_frame_error = *self.get_debug_process_resolve_frame_error();

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
				let formatted_len = {
					let mut file = (*self.get_mut_file()).write()?;
					match (*file).as_mut() {
						Some(file) => {
							let formatted_timestamp = format!("[{}]", formatted_timestamp);
							let formatted_timestamp = formatted_timestamp.as_bytes();
							file.write(formatted_timestamp)?;
							let formatted_len: u64 = try_into!(formatted_timestamp.len())?;
							formatted_len
						}
						None => 0,
					}
				};
				*self.get_mut_cur_size() += formatted_len;
			}

			if show_stdout {
				if show_colors {
					print!("[{}]", formatted_timestamp.to_string().dimmed());
				} else {
					print!("[{}]", formatted_timestamp);
				}
			}
		}
		// if log level needs to be shown we print/write it here
		if show_log_level {
			let tsp = if show_timestamp { " " } else { "" };
			{
				let formatted_len = {
					let mut file = (*self.get_mut_file()).write()?;
					match (*file).as_mut() {
						Some(file) => {
							let formatted_level =
								if level == LogLevel::Info || level == LogLevel::Warn {
									format!("{}({}) ", tsp, level)
								} else {
									format!("{}({})", tsp, level,)
								};
							let formatted_level = formatted_level.as_bytes();
							file.write(formatted_level)?;
							let formatted_len: u64 = try_into!(formatted_level.len())?;
							formatted_len
						}
						None => 0,
					}
				};
				*self.get_mut_cur_size() += formatted_len;
			}

			if show_stdout {
				if show_colors {
					// specific colors for each level
					match level {
						LogLevel::Trace => {
							print!("{}({})", tsp, format!("{}", level).magenta());
						}
						LogLevel::Debug => {
							print!("{}({})", tsp, format!("{}", level).cyan());
						}
						LogLevel::Info => {
							print!("{}({}) ", tsp, format!("{}", level).green());
						}
						LogLevel::Warn => {
							print!("{}({}) ", tsp, format!("{}", level).yellow());
						}
						LogLevel::Error => {
							print!("{}({})", tsp, format!("{}", level).bright_blue());
						}
						LogLevel::Fatal => {
							print!("{}({})", tsp, format!("{}", level).red());
						}
					}
				} else {
					// without color
					if level == LogLevel::Info || level == LogLevel::Warn {
						print!("{}({}) ", tsp, level);
					} else {
						print!("{}({}))", tsp, level);
					}
				}
			}
		}

		if show_line_num {
			let slp = if show_timestamp || show_log_level {
				" "
			} else {
				""
			};
			let mut found_logger = false;
			let mut found_frame = false;
			let mut logged_from_file = "*********unknown**********".to_string();

			// try to look through the backtrace to find the line where logging
			// occurred. This is especially useful for debugging
			backtrace::trace(|frame| {
				backtrace::resolve_frame(frame, |symbol| {
					found_frame = match Self::process_resolve_frame(
						debug_lineno_is_none,
						debug_process_resolve_frame_error,
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
				let logged_from_file_len = {
					let mut file = (*self.get_mut_file()).write()?;
					match (*file).as_mut() {
						Some(file) => {
							let logged_from_file = format!("{}[{}]", slp, logged_from_file);
							let logged_from_file = logged_from_file.as_bytes();
							file.write(logged_from_file)?;
							let logged_from_file_len: u64 = try_into!(logged_from_file.len())?;
							logged_from_file_len
						}
						None => 0,
					}
				};
				*self.get_mut_cur_size() += logged_from_file_len;
			}
			// if we're showing stdout, do so here
			if show_stdout {
				if show_colors {
					print!("{}[{}]", slp, logged_from_file.yellow());
				} else {
					print!("{}[{}]", slp, logged_from_file);
				}
			}
		}

		// write the line to the file (if it exists)
		{
			let bytes_len = {
				let mut file = (*self.get_mut_file()).write()?;
				match (*file).as_mut() {
					Some(file) => {
						let file_line = if logging_type != LoggingType::Plain {
							format!(": {}", line)
						} else {
							line.to_string()
						};
						let line_bytes = file_line.as_bytes();

						file.write(line_bytes)?;
						file.write(NEWLINE)?;
						let mut line_bytes_len: u64 = try_into!(line_bytes.len())?;
						line_bytes_len += 1;
						let mut ret_len = line_bytes_len;

						if show_bt {
							let bt = LocalBacktrace::new();
							let bt_text = format!("{:?}", bt);
							let bt_bytes: &[u8] = bt_text.as_bytes();
							file.write(bt_bytes)?;
							let bt_bytes_u64: u64 = try_into!(bt_bytes.len())?;
							ret_len += bt_bytes_u64;
						}
						ret_len
					}
					None => 0,
				}
			};
			*self.get_mut_cur_size() += bytes_len;
		}

		// finally print the actual line
		if show_stdout {
			if logging_type != LoggingType::Plain {
				println!(": {}", line);
			} else {
				println!("{}", line);
			}
			if show_bt {
				let bt = LocalBacktrace::new();
				let bt_text = format!("{:?}", bt);
				print!("{}", bt_text);
			}
		}

		Ok(())
	}

	fn rotate_if_needed(&mut self) -> Result<(), Error> {
		if !(*self.get_log_config()).auto_rotate {
			return Ok(()); // auto rotate not enabled
		}

		let now = Instant::now();

		let max_age_millis = (*self.get_log_config()).max_age_millis;
		let max_size_bytes = (*self.get_log_config()).max_size_bytes;

		// if the file is too old or too big we rotate
		if now
			.checked_duration_since(*self.get_last_rotation())
			.unwrap_or(Duration::new(0, 0))
			.as_millis() > max_age_millis.into()
			|| *self.get_cur_size() > max_size_bytes
		{
			self.rotate()?;
		}

		Ok(())
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
		if metadata.is_err() || *self.get_debug_invalid_metadata() {
			return err!(
				MetaData,
				format!("failed to retrieve metadata for file: {}", path.display())
			);
		}
		let metadata = metadata.unwrap();

		let len = metadata.len();
		if len == 0 {
			// do we need to add the file header?
			let header_len = (*self.get_log_config()).file_header.len();
			if header_len > 0 {
				// there's a header. We need to append it.
				file.write((*self.get_log_config()).file_header.as_bytes())?;
				file.write(NEWLINE)?;
				let header_len_u64: u64 = try_into!(header_len)?;
				*self.get_mut_cur_size() = header_len_u64 + 1;
			} else {
				*self.get_mut_cur_size() = 0;
			}
		} else {
			*self.get_mut_cur_size() = len;
		}

		*self.get_mut_last_rotation() = Instant::now();
		Ok(())
	}

	fn process_resolve_frame(
		debug_lineno_is_none: bool,
		debug_process_resolve_frame_error: bool,
		symbol: &Symbol,
		found_logger: &mut bool,
		logged_from_file: &mut String,
	) -> Result<bool, Error> {
		if debug_process_resolve_frame_error {
			return err!(Test, "test resolve_frame error");
		}
		let mut found_frame = false;

		// test mode (better data)
		#[cfg(debug_assertions)]
		if let Some(filename) = symbol.filename() {
			let filename = filename.display().to_string();
			let lineno = symbol.lineno();

			let lineno = if lineno.is_none() || debug_lineno_is_none {
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

	fn need_rotate_impl(&self) -> Result<bool, Error> {
		if !*self.get_is_init() {
			err!(
				NotInitialized,
				"logger has not been initalized. Call init() first."
			)
		} else {
			let now = Instant::now();

			let max_age_millis = (*self.get_log_config()).max_age_millis;
			let max_size_bytes = (*self.get_log_config()).max_size_bytes;

			// if the file is either too old or too big we need to rotate
			if now
				.checked_duration_since(*self.get_last_rotation())
				.unwrap_or(Duration::new(0, 0))
				.as_millis() > max_age_millis.into()
				|| *self.get_cur_size() > max_size_bytes
			{
				Ok(true)
			} else {
				Ok(false)
			}
		}
	}

	fn rotate_impl(&mut self) -> Result<(), Error> {
		if !*self.get_is_init() {
			// log hasn't been initialized yet, return error
			let text = "logger has not been initalized. Call init() first.";
			return err!(NotInitialized, text);
		}

		{
			// check if there's a file, if not return error
			let mut file = (*self.get_mut_file()).write()?;
			match (*file).as_mut() {
				Some(_file) => {}
				None => {
					let text = "log file cannot be rotated because there is no file associated with this logger";
					return err!(IllegalState, text);
				}
			}
		}

		let now: DateTime<Local> = Local::now();
		// standard rotation string format
		let rotation_string = now.format(".r_%m_%d_%Y_%T").to_string().replace(":", "-");

		// get the original file path
		let original_file_path = PathBuf::from(self.get_log_file_path_canonicalized().clone());

		// get the parent directory and the file name
		let text = "file_path has an unexpected illegal value of None for parent";
		let parent = some_or_err!(original_file_path.parent(), IllegalState, text)?;

		let text = "file_path has an unexpected illegal value of None for file_name";
		let file_name = some_or_err!(original_file_path.file_name(), IllegalState, text)?;

		let text = "file_path could not be converted to string";
		let file_name = some_or_err!(file_name.to_str(), IllegalState, text)?;

		// create the new rotated file
		let mut new_file_path_buf = parent.to_path_buf();
		let file_name = match file_name.rfind(".") {
			Some(pos) => &file_name[0..pos],
			_ => &file_name,
		};
		let file_name = format!("{}{}_{}.log", file_name, rotation_string, random::<u64>());
		new_file_path_buf.push(file_name);

		if (*self.get_log_config()).delete_rotation {
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
			let mut file = (*self.get_file()).write()?;
			*file = Some(nfile);
		}

		Ok(())
	}

	fn set_log_level_impl(&mut self, log_level: LogLevel) {
		(*self.get_mut_cur_log_level()) = log_level;
	}

	fn close_impl(&mut self) -> Result<(), Error> {
		if !*self.get_is_init() {
			let text = "logger has not been initalized. Call init() first.";
			err!(NotInitialized, text)
		} else {
			let mut file = (*self.get_mut_file()).write()?;
			// drop handler closes the handle
			*file = None;
			Ok(())
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
