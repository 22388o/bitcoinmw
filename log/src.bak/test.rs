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

#[cfg(test)]
mod test {
	use crate as bmw_log;
	use crate::constants::*;
	use crate::public::*;
	use crate::types::{LogConfig, LogImpl};
	use crate::LogConfigOptions::*;
	use bmw_conf::*;
	use bmw_deps::lazy_static::lazy_static;
	use bmw_derive::*;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use std::fs::{read_dir, read_to_string, File, OpenOptions};
	use std::io::{Read, Write};
	use std::path::PathBuf;
	use std::sync::{Arc, RwLock};

	// lock used to prevent two tests from calling log_init at the same time
	lazy_static! {
		pub static ref LOCK: Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
	}

	trace!();

	#[derive(Configurable)]
	#[options = "MyOptions"]
	struct TestDerive {
		#[required]
		v1: usize,
	}

	impl Default for TestDerive {
		fn default() -> Self {
			Self { v1: 0 }
		}
	}

	#[test]
	fn test_log_basic() -> Result<(), Error> {
		let test_info = test_info!()?; // obtain test info struct
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("test.log");
		// create a logger with auto rotate on ( use impl so we can check the config )
		let path = buf.display().to_string();
		let configs = vec![AutoRotate(true), LogFilePath(&path)];
		let mut log = LogImpl::new(configs)?;

		// set this before init. It is an error.
		assert!(log
			.set_config_option(LogConfigOptions::DisplayColors(false))
			.is_err());

		// debug log level
		log.set_log_level(LogLevel::Debug);
		log.init()?; // in logger
		log.log(LogLevel::Debug, "test10")?; // log a message

		assert!(log
			.set_config_option(LogConfigOptions::LogFilePath(&path))
			.is_err());

		// check that display colors is true (default)
		assert_eq!(log.config.display_colors, true);

		// set display colors to false
		log.set_config_option(LogConfigOptions::DisplayColors(false))?;
		// confirm it was set
		assert_eq!(log.config.display_colors, false);

		// set back to true
		log.set_config_option(LogConfigOptions::DisplayColors(true))?;

		// confirm it's now true
		assert_eq!(log.config.display_colors, true);

		assert_eq!(
			log.config.line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN
		);

		log.set_config_option(LogConfigOptions::LineNumDataMaxLen(
			DEFAULT_LINE_NUM_DATA_MAX_LEN + 10,
		))?;
		// confirm it was set
		assert_eq!(
			log.config.line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN + 10
		);

		// set back to default
		log.set_config_option(LogConfigOptions::LineNumDataMaxLen(
			DEFAULT_LINE_NUM_DATA_MAX_LEN,
		))?;

		// confirm it's now  default
		assert_eq!(
			log.config.line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN
		);

		log.set_config_option(LogConfigOptions::FileHeader("testing123"))?;

		assert_eq!(log.config.file_header, "testing123".to_string());

		// do some more logging
		log.log(LogLevel::Debug, "test11")?;
		log.log(LogLevel::Debug, "test12")?;
		log.log(LogLevel::Debug, "test13")?;
		// log a plain fatal message
		log.log_plain(LogLevel::Fatal, "plaintextfatal")?;
		// log trace (will not show up)
		log.log(LogLevel::Trace, "thisdoesnotshowup")?;

		// open the log file to confirm these logged items
		let mut f = File::open(format!("{}/test.log", directory))?;
		let mut s = String::new();
		f.read_to_string(&mut s)?;

		// find the lines
		let test10_loc = s.find("test10").unwrap();
		let test11_loc = s.find("test11").unwrap();
		let test12_loc = s.find("test12").unwrap();
		let test13_loc = s.find("test13").unwrap();
		let plain_text_fatal_loc = s.find("\nplaintextfatal").unwrap();

		// assert they were found and in the correct order
		assert!(test10_loc > 0);
		assert!(test10_loc < test11_loc);
		assert!(test11_loc < test12_loc);
		assert!(test12_loc < test13_loc);
		assert!(plain_text_fatal_loc > test13_loc);

		// this wasn't found because it was logged at 'trace' level
		assert!(s.find("thisdoesnotshowup").is_none());

		Ok(())
	}

	#[test]
	fn test_log_macros() -> Result<(), Error> {
		// lock so we don't interfere with the other test's global logging
		let _lock = LOCK.write()?;
		// do these before init. they're not allowed and generate errors
		assert!(set_log_option!(AutoRotate(false)).is_err());
		assert!(log_rotate!().is_err());
		assert!(need_rotate!().is_err());

		// get a test_info struct
		let test_info = test_info!()?;

		// create a pathbuf for a log file
		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("log.log");

		// init log
		let path = buf.display().to_string();
		log_init!(LogFilePath(&path))?;

		// do logging at all levels and all styles
		trace!("mactest1")?;
		trace_plain!("plain1")?;
		trace_all!("all1")?;

		debug!("mactest1")?;
		debug_plain!("plain1")?;
		debug_all!("all1")?;

		info!("mactest1")?;
		info_plain!("plain1")?;
		info_all!("all1")?;

		warn!("mactest1")?;
		warn_plain!("plain1")?;
		warn_all!("all1")?;

		error!("mactest1")?;
		error_plain!("plain1")?;
		error_all!("all1")?;

		fatal!("mactest1")?;
		fatal_plain!("plain1")?;
		fatal_all!("all1")?;

		// ensure rotate is allowed and not an error now
		assert!(need_rotate!().is_ok());
		assert!(log_rotate!().is_ok());

		// now log without colors
		set_log_option!(DisplayColors(false))?;
		info!("nocolormactest1")?;
		info_plain!("nocolorplain1")?;
		info_all!("nocolorall1")?;

		// log a backtrace
		set_log_option!(DisplayBacktrace(true))?;
		error!("errbt")?;
		error_plain!("errorbt")?;

		// set the GLOBAL logger back to none for the other tests
		// only done in tests
		let mut lock = BMW_GLOBAL_LOG.write()?;
		*lock = None;

		Ok(())
	}

	#[test]
	fn test_log_rotate() -> Result<(), Error> {
		// get test_info for this test
		let test_info = test_info!()?;
		let directory = test_info.directory();

		// create rotate.log in our assigned directory
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let path = buf.display().to_string();
		let mut log = logger!(
			MaxSizeBytes(100),   // specific low byte count
			MaxAgeMillis(3_000), // specific low max age
			LogFilePath(&path)
		)?;

		log.init()?;
		log.set_log_level(LogLevel::Debug);
		assert!(!log.need_rotate()?); // no logging yet so no rotation needed

		// log 100 bytes of data + 10 newlines so a rotation is needed (autorotate is
		// false)
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		// we need a rotate
		assert!(log.need_rotate()?);
		log.rotate()?; // do the rotation
		assert!(!log.need_rotate()?); // now rotation is not needed

		// do some more logging that doesn't cross the 100 byte or 3000 ms threshold
		log.log_plain(LogLevel::Info, "test")?;
		assert!(!log.need_rotate()?); // not needed yet
		sleep(Duration::from_millis(6_000)); // wait 6 seconds
		assert!(log.need_rotate()?); // now it's needed based on log age
		log.rotate()?; // do the rotation

		// do some assertions on files
		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			// this is a rotated file
			if file_name.find("rotate.r") == Some(0) {
				rotated_files += 1;
			}

			// this is the non rotated file
			if file_name.find("rotate.log") == Some(0) {
				unrotated_files += 1;
			}
			count += 1;
		}

		assert_eq!(count, 3); // three files
		assert_eq!(rotated_files, 2); // two rotated files
		assert_eq!(unrotated_files, 1); // one unrotated file

		Ok(())
	}

	#[test]
	fn test_log_auto_rotate() -> Result<(), Error> {
		let test_info = test_info!()?; // get test info structure
		let directory = test_info.directory();

		// create log file in the directory assigned
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let path = buf.display().to_string();
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&path),
			AutoRotate(true)
		)?;

		log.init()?;
		log.set_log_level(LogLevel::Debug);

		// log enough to trigger a rotation
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		log.log_plain(LogLevel::Info, "test")?;
		sleep(Duration::from_millis(6_000));
		// second rotation should be triggered via autorotate
		log.log_plain(LogLevel::Info, "test")?;

		// assert that the rotations occurred
		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			if file_name.find("rotate.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name.find("rotate.log") == Some(0) {
				unrotated_files += 1;
			}
			count += 1;
		}

		assert_eq!(count, 3);
		assert_eq!(rotated_files, 2); // 2 rotated files
		assert_eq!(unrotated_files, 1); // 1 non-rotated file

		Ok(())
	}

	#[test]
	fn test_log_errors() -> Result<(), Error> {
		// configure a standard logger
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let buf = buf.display().to_string();
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf),
			AutoRotate(true)
		)?;

		// rotate cannot happen until init is called
		assert!(log.rotate().is_err());

		log.init()?;

		// second log.init is an error
		assert!(log.init().is_err());
		// closing is ok
		assert!(log.close().is_ok());
		Ok(())
	}

	#[test]
	fn test_log_stdoutonly() -> Result<(), Error> {
		// init a stdout logger only
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(""),
			AutoRotate(true),
		)?;

		// need rotate cannot be called before init
		assert!(log.need_rotate().is_err());
		log.init()?;
		// rotate is an error because we're stdout only
		assert!(log.rotate().is_err());
		Ok(())
	}

	#[test]
	fn test_log_logger_macro() -> Result<(), Error> {
		// test the macros
		let mut log = LogImpl::new(vec![
			MaxSizeBytes(103),
			MaxAgeMillis(3_000),
			LogFilePath(""),
		])?;
		log.init()?;
		// double init is an error
		assert!(log.init().is_err());

		// get the config option and assert it's equal to what we configured in the logger
		// macro
		assert_eq!(log.config.max_size_bytes, 103);
		Ok(())
	}

	#[test]
	fn test_log_no_dot_name() -> Result<(), Error> {
		// setup standard logger with quick/small rotations
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotatelog"); // no dot in log name
		let buf = buf.display().to_string();
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf),
			AutoRotate(true)
		)?;

		// init and log 110 bytes
		log.init()?;
		log.set_log_level(LogLevel::Debug);
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		// do some additional logging
		log.log_plain(LogLevel::Info, "test")?;
		sleep(Duration::from_millis(6_000));
		log.log_plain(LogLevel::Info, "test")?;

		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			if file_name.find("rotatelog.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name == "rotatelog" {
				unrotated_files += 1;
			}
			count += 1;
		}

		// assert that things are the same even though our log file has no dot in it's name
		assert_eq!(count, 3);
		assert_eq!(rotated_files, 2); // two rotated files
		assert_eq!(unrotated_files, 1); // one standard log file

		Ok(())
	}

	#[test]
	fn test_log_delete_rotation() -> Result<(), Error> {
		// configure a log file with 'DeleteRotation' configured
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotatelog");
		let buf = buf.display().to_string();
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf),
			AutoRotate(true),
			DeleteRotation(true)
		)?;

		// do some logging
		log.init()?;
		log.set_log_level(LogLevel::Debug);
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		log.log_plain(LogLevel::Info, "test")?;
		sleep(Duration::from_millis(6_000));
		log.log_plain(LogLevel::Info, "test")?;

		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			if file_name.find("rotatelog.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name == "rotatelog" {
				unrotated_files += 1;
			}
			count += 1;
		}

		assert_eq!(count, 1);
		assert_eq!(rotated_files, 0); // no rotated files because they were deleted
		assert_eq!(unrotated_files, 1); // our initial file is still there

		Ok(())
	}

	#[test]
	fn test_log_prexisting_file() -> Result<(), Error> {
		// create a regular logger based on test_info
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotatelog");
		let buf = buf.display().to_string();

		// create the file before creating the logger
		File::create(buf.clone())?;

		// create logger
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf),
			AutoRotate(true)
		)?;

		// closing is an error because we didn't call init yet
		assert!(log.close().is_err());

		// init and log 10 lines
		log.init()?;
		log.set_log_level(LogLevel::Debug);
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		// do some more logging
		log.log_plain(LogLevel::Info, "test")?;
		sleep(Duration::from_millis(6_000));
		log.log_plain(LogLevel::Info, "test")?;

		// confirm everything is ok even through the file existed
		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			if file_name.find("rotatelog.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name == "rotatelog" {
				unrotated_files += 1;
			}
			count += 1;
		}

		assert_eq!(count, 3);
		assert_eq!(rotated_files, 2);
		assert_eq!(unrotated_files, 1);

		Ok(())
	}

	#[test]
	fn test_log_prexisting_file_w_data() -> Result<(), Error> {
		// log with a prexisting file with content in it
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotatelog");
		let buf = buf.display().to_string();

		File::create(buf.clone())?;
		let mut file = OpenOptions::new().write(true).open(buf.clone())?;
		file.write(b"test")?; // write test to the file

		// init the logger
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf),
			AutoRotate(true)
		)?;

		// can't close until after init
		assert!(log.close().is_err());

		// init and write 10 lines
		log.init()?;
		log.set_log_level(LogLevel::Debug);
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		// do some more logging and sleep
		log.log_plain(LogLevel::Info, "test")?;
		sleep(Duration::from_millis(6_000));
		log.log_plain(LogLevel::Info, "test")?;

		// confirm all is as expected even with prexisting files
		let dir = read_dir(directory)?;
		let mut count = 0;
		let mut rotated_files = 0;
		let mut unrotated_files = 0;
		for path in dir {
			let file_name = path?.file_name().into_string()?;
			if file_name.find("rotatelog.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name == "rotatelog" {
				unrotated_files += 1;
			}
			count += 1;
		}

		assert_eq!(count, 3);
		assert_eq!(rotated_files, 2);
		assert_eq!(unrotated_files, 1);

		Ok(())
	}

	#[test]
	fn test_log_show_millis() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut buf1 = PathBuf::new();
		let mut buf2 = PathBuf::new();

		// setup two otherwise identical loggers (one with DisplayMillis(false), other
		// DisplayMillis(true)
		buf1.push(test_info.directory());
		buf2.push(test_info.directory());
		buf1.push("file1.log");
		buf2.push("file2.log");
		let buf1 = buf1.display().to_string();
		let buf2 = buf2.display().to_string();

		let mut logger1 = logger!(
			LogFilePath(&buf1),
			DisplayMillis(true),
			FileHeader("sometext")
		)?;
		let mut logger2 = logger!(
			LogFilePath(&buf2),
			DisplayMillis(false),
			FileHeader("sometext")
		)?;

		// set log level and init both loggers
		logger1.set_log_level(LogLevel::Debug);
		logger2.set_log_level(LogLevel::Debug);
		logger1.init()?;
		logger2.init()?;

		// log same text to both
		logger1.log(LogLevel::Info, "test")?;
		logger2.log(LogLevel::Info, "test")?;

		// close loggers
		logger1.close()?;
		logger2.close()?;

		// and read the files to get sizes confirm file2 is 4 bytes bigger due to the
		// milliseconds being displayed
		let dir = read_dir(test_info.directory())?;
		let mut file1_size = None;
		let mut file2_size = None;
		for path in dir {
			let path = path?;
			let file_name = path.file_name().into_string()?;
			let metadata = path.metadata()?;
			let len = metadata.len();
			if file_name == "file1.log" {
				file1_size = Some(len);
			} else if file_name == "file2.log" {
				file2_size = Some(len);
			}
		}

		// file1 is 4 bytes bigger because it has the milliseconds displayed
		assert_eq!(file1_size.unwrap(), file2_size.unwrap() + 4);

		Ok(())
	}

	#[test]
	fn test_process_resolve_frame_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut buf1 = PathBuf::new();

		buf1.push(test_info.directory());
		buf1.push("file1.log");
		let buf1 = buf1.display().to_string();

		// create a logger displaying millis and with FileHeader/LogPath specified
		let mut logger1 = LogImpl::new(vec![
			LogFilePath(&buf1),
			DisplayMillis(true),
			FileHeader("sometext"),
		])?;

		// set the debug flag to trigger specific state
		logger1.debug_process_resolve_frame_error();
		logger1.set_log_level(LogLevel::Debug);
		logger1.init()?;

		// even with the frame error we continue processing
		assert!(logger1.log(LogLevel::Info, "test").is_ok());

		Ok(())
	}

	#[test]
	fn test_invalid_metadata() -> Result<(), Error> {
		// get test_info
		let test_info = test_info!()?;
		let mut buf1 = PathBuf::new();

		buf1.push(test_info.directory());
		buf1.push("file1.log");
		let buf1 = buf1.display().to_string();

		// create a logger with a log file and header
		let mut logger1 = LogImpl::new(vec![
			LogFilePath(&buf1),
			DisplayMillis(true),
			FileHeader("sometext"),
		])?;
		logger1.debug_invalid_metadata();
		logger1.set_log_level(LogLevel::Debug);

		// with invalid metadata, init will fail
		assert!(logger1.init().is_err());

		Ok(())
	}

	#[test]
	fn test_log_cycle() -> Result<(), Error> {
		// ensure a long cycle of logging works
		let _lock = LOCK.write()?;
		for _ in 0..2000 {
			sleep(Duration::from_millis(1));
			info!("test")?;
		}

		// set the GLOBAL logger back to none for the other tests
		// only done in tests
		let mut lock = BMW_GLOBAL_LOG.write()?;
		*lock = None;

		Ok(())
	}

	#[test]
	fn test_log_lineno_is_none() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut buf1 = PathBuf::new();

		buf1.push(test_info.directory());
		buf1.push("file1.log");
		let buf1 = buf1.display().to_string();

		// create a logger
		let mut logger1 = LogImpl::new(vec![
			LogFilePath(&buf1),
			DisplayMillis(true),
			FileHeader("sometext"),
		])?;

		// set debugging flag to trigger a different state
		logger1.debug_lineno_is_none();
		logger1.set_log_level(LogLevel::Debug);
		logger1.init()?;

		// even with the lineno error processing continues
		assert!(logger1.log(LogLevel::Info, "test").is_ok());
		Ok(())
	}

	#[test]
	fn test_log_all_options() -> Result<(), Error> {
		// get test_info
		let test_info = test_info!()?;
		let mut buf1 = PathBuf::new();

		buf1.push(test_info.directory());
		buf1.push("file1.log");
		let buf1 = buf1.display().to_string();

		// create a logger with all options specified to exercise various parts off the code
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(&buf1),
			AutoRotate(true),
			DisplayColors(true),
			DisplayStdout(true),
			DisplayTimestamp(false),
			DisplayLogLevel(true),
			DisplayLineNum(false),
			DisplayMillis(true),
			DisplayBacktrace(true),
			LineNumDataMaxLen(32),
			DeleteRotation(false),
			FileHeader("header")
		)?;

		// init should be successful
		assert!(log.init().is_ok());
		Ok(())
	}

	#[test]
	fn test_log_invalid_configs() -> Result<(), Error> {
		// MaxSizeBytes must be 50 or more
		assert!(logger!(MaxSizeBytes(1)).is_err());
		// MaxAgeMillis must be 1_000 or more
		assert!(logger!(MaxAgeMillis(1)).is_err());
		// LineNumDataMaxLen must be 10 or more
		assert!(logger!(LineNumDataMaxLen(1)).is_err());

		// test an invalid path
		let mut buf = PathBuf::new();
		buf.push("a");
		buf.push("b");
		buf.push("c");
		buf.push("d");
		buf.push("e.log");
		let buf = buf.display().to_string();
		assert!(logger!(LogFilePath(&buf)).is_err());

		Ok(())
	}

	#[test]
	fn test_file_header() -> Result<(), Error> {
		// create a logger with a log file in our test directory 100 byte limit and a header
		// configured with autorotate
		let test_info = test_info!()?;
		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger.log");
		let buf = buf.display().to_string();
		let mut logger = logger!(
			MaxSizeBytes(100),
			LogFilePath(&buf),
			AutoRotate(true),
			FileHeader("myheader_abc")
		)?;

		// this is an error because we haven't called init yet
		assert!(logger.log(LogLevel::Info, "test").is_err());
		logger.init()?;
		logger.set_log_level(LogLevel::Debug);
		// do some logging
		for _ in 0..100 {
			logger.log(LogLevel::Info, "0123456789")?;
		}

		// there should be 50 files all should start with the FileHeader value
		let dir = read_dir(test_info.directory())?;
		let mut count = 0;
		for path in dir {
			let path = path?;
			let file_name = path.file_name().into_string()?;
			let mut path_buf = PathBuf::new();
			path_buf.push(test_info.directory());
			path_buf.push(file_name);
			let d = read_to_string(path_buf)?;
			assert!(d.find("myheader_abc").unwrap() == 0);
			assert!(d[1..].find("myeader_abc").is_none());
			count += 1;
		}
		assert_eq!(count, 50); // assert 50 files

		Ok(())
	}

	#[test]
	#[cfg(unix)]
	fn test_confirm_boundries() -> Result<(), Error> {
		// this test is not enabled on windows. BACK-R's added to files makes this calculation
		// complicated. BMW optimized for linux/macos, but approximately correct on Windows.
		let test_info = test_info!()?;
		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger.log");
		let buf = buf.display().to_string();
		let mut logger = logger!(
			MaxSizeBytes(363),
			LogFilePath(&buf),
			AutoRotate(false),
			FileHeader("myheader_abc")
		)?;

		logger.init()?;
		logger.set_log_level(LogLevel::Debug);

		logger.log(LogLevel::Info, "0123456789")?;
		logger.log_plain(LogLevel::Info, "0123456789")?;
		logger.log_all(LogLevel::Info, "0123456789")?;

		logger.log(LogLevel::Debug, "0123456789")?;
		logger.log_plain(LogLevel::Debug, "0123456789")?;
		logger.log_all(LogLevel::Debug, "01234567890123")?;
		assert!(!logger.need_rotate()?); // file exactly 363, no rotate needed
		logger.close()?;

		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger.log");
		let len = buf.metadata()?.len();
		assert_eq!(len, 363);

		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger2.log");
		let buf = buf.display().to_string();

		// try again one byte smaller MaxSizeBytes.
		let mut logger = logger!(
			MaxSizeBytes(362),
			LogFilePath(&buf),
			AutoRotate(false),
			FileHeader("myheader_abc")
		)?;

		logger.init()?;
		logger.set_log_level(LogLevel::Debug);

		logger.log(LogLevel::Info, "0123456789")?;
		logger.log_plain(LogLevel::Info, "0123456789")?;
		logger.log_all(LogLevel::Info, "0123456789")?;

		logger.log(LogLevel::Debug, "0123456789")?;
		logger.log_plain(LogLevel::Debug, "0123456789")?;
		logger.log_all(LogLevel::Debug, "01234567890123")?;

		assert!(logger.need_rotate()?); // this time we need a rotate
		logger.close()?;

		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger2.log");

		let len = buf.metadata()?.len();
		assert_eq!(len, 363);

		// try without header
		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger3.log");
		let buf = buf.display().to_string();

		// try without a header this time (12 bytes + 1 newline less)
		let mut logger = logger!(MaxSizeBytes(350), LogFilePath(&buf), AutoRotate(false),)?;

		logger.init()?;
		logger.set_log_level(LogLevel::Debug);

		logger.log(LogLevel::Info, "0123456789")?;
		logger.log_plain(LogLevel::Info, "0123456789")?;
		logger.log_all(LogLevel::Info, "0123456789")?;

		logger.log(LogLevel::Debug, "0123456789")?;
		logger.log_plain(LogLevel::Debug, "0123456789")?;
		logger.log_all(LogLevel::Debug, "01234567890123")?;

		assert!(!logger.need_rotate()?); // this time we don't need a rotate
		logger.close()?;

		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger3.log");
		let len = buf.metadata()?.len();
		assert_eq!(len, 350);

		// try again with one less byte MaxSizeByte
		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger4.log");
		let buf = buf.display().to_string();

		// try again one byte smaller MaxSizeBytes.
		let mut logger = logger!(MaxSizeBytes(349), LogFilePath(&buf), AutoRotate(false),)?;

		logger.init()?;
		logger.set_log_level(LogLevel::Debug);

		logger.log(LogLevel::Info, "0123456789")?;
		logger.log_plain(LogLevel::Info, "0123456789")?;

		logger.log_all(LogLevel::Info, "0123456789")?;

		logger.log(LogLevel::Debug, "0123456789")?;
		logger.log_plain(LogLevel::Debug, "0123456789")?;
		logger.log_all(LogLevel::Debug, "01234567890123")?;

		assert!(logger.need_rotate()?); // this time we need a rotate
		logger.close()?;

		let mut buf = PathBuf::new();
		buf.push(test_info.directory());
		buf.push("mylogger4.log");
		let len = buf.metadata()?.len();
		assert_eq!(len, 350);
		Ok(())
	}

	#[test]
	fn test_multi_loggers() -> Result<(), Error> {
		let test_info = test_info!(true)?;
		let directory = test_info.directory();

		let mut path1 = PathBuf::from(directory);
		let mut path2 = path1.clone();
		let mut path3 = path1.clone();

		path1.push("log1.log");
		path2.push("log2.log");
		path3.push("log3.log");

		let path1 = path1.display().to_string();
		let path2 = path2.display().to_string();
		let path3 = path3.display().to_string();

		let mut logger1 = logger!(
			LogFilePath(&path1),
			DisplayColors(true),
			DisplayLineNum(true),
			DisplayLogLevel(true),
			DisplayTimestamp(true),
			DisplayMillis(false)
		)?;
		let mut logger2 = logger!(LogFilePath(&path2), DisplayStdout(false))?;
		let mut logger3 = logger!(LogFilePath(&path3), DisplayStdout(false))?;

		logger1.init()?;
		logger2.init()?;
		logger3.init()?;

		logger1.set_log_level(LogLevel::Info);
		logger2.set_log_level(LogLevel::Debug);
		logger3.set_log_level(LogLevel::Warn);

		logger1.log(LogLevel::Trace, "test")?;
		logger2.log(LogLevel::Trace, "test")?;
		logger3.log(LogLevel::Trace, "test")?;

		logger1.log(LogLevel::Debug, "test")?;
		logger2.log(LogLevel::Debug, "test")?;
		logger3.log(LogLevel::Debug, "test")?;

		logger1.log(LogLevel::Info, "test")?;
		logger2.log(LogLevel::Info, "test")?;
		logger3.log(LogLevel::Info, "test")?;

		logger1.log_plain(LogLevel::Warn, "test plain")?;
		logger2.log_plain(LogLevel::Warn, "test plain")?;
		logger3.log_plain(LogLevel::Warn, "test plain")?;

		logger1.log(LogLevel::Error, "test")?;
		logger2.log(LogLevel::Error, "test")?;
		logger3.log(LogLevel::Error, "test")?;

		logger1.log(LogLevel::Fatal, "test")?;
		logger2.log(LogLevel::Fatal, "test")?;
		logger3.log(LogLevel::Fatal, "test")?;

		Ok(())
	}
}
