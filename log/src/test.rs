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

#[cfg(test)]
mod test {
	use crate::log::DebugLog;
	use crate::log::LogBuilder;
	use crate::log::LogConstOptions::*;
	use crate::types::LogLevel;
	use crate::LogErrorKind::*;
	use crate::*;
	use bmw_core::lazy_static::lazy_static;
	use bmw_core::*;
	use bmw_test::*;
	use std::fs::{read_dir, File};
	use std::io::Read;
	use std::path::PathBuf;
	use std::sync::{Arc, RwLock};

	// lock used to prevent two tests from calling log_init at the same time
	lazy_static! {
		pub static ref LOCK: Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
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
		let mut log = LogBuilder::build_debug_log(configs)?;

		// set this before init. It is an error.
		assert_eq!(
			log.set_log_option(Colors(false)).unwrap_err().kind(),
			&kind!(
				NotInitialized,
				"logger has not been initalized. Call init() first."
			)
		);

		// debug log level
		log.set_log_level(LogLevel::Debug);
		log.init()?; // in logger
		log.log(LogLevel::Debug, "test10")?; // log a message

		assert!(log
			.set_log_option(LogConstOptions::LogFilePath(&path))
			.is_err());

		// check that display colors is true (default)
		assert_eq!(log.get_log_config_debug().colors, true);

		// set display colors to false
		log.set_log_option(LogConstOptions::Colors(false))?;
		// confirm it was set
		assert_eq!(log.get_log_config_debug().colors, false);

		// set back to true
		log.set_log_option(LogConstOptions::Colors(true))?;

		// confirm it's now true
		assert_eq!(log.get_log_config_debug().colors, true);

		assert_eq!(
			log.get_log_config_debug().line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN
		);

		log.set_log_option(LogConstOptions::LineNumDataMaxLen(
			DEFAULT_LINE_NUM_DATA_MAX_LEN + 10,
		))?;
		// confirm it was set
		assert_eq!(
			log.get_log_config_debug().line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN + 10
		);

		// set back to default
		log.set_log_option(LogConstOptions::LineNumDataMaxLen(
			DEFAULT_LINE_NUM_DATA_MAX_LEN,
		))?;

		// confirm it's now  default
		assert_eq!(
			log.get_log_config_debug().line_num_data_max_len,
			DEFAULT_LINE_NUM_DATA_MAX_LEN
		);

		log.set_log_option(LogConstOptions::FileHeader("testing123"))?;

		assert_eq!(
			log.get_log_config_debug().file_header,
			"testing123".to_string()
		);

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
}
