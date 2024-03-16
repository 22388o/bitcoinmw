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
	use crate as bmw_log;
	use bmw_conf::*;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use std::fs::{read_dir, File};
	use std::io::Read;
	use std::path::PathBuf;

	trace!();

	#[test]
	fn test_log_basic() -> Result<(), Error> {
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("test.log");
		let mut log = logger!(AutoRotate(true), LogFilePath(Some(buf)))?;
		log.set_log_level(LogLevel::Debug);
		log.init()?;
		log.log(LogLevel::Debug, "test10")?;
		assert_eq!(
			log.get_config_option(ConfigOptionName::DisplayColors)?,
			ConfigOption::DisplayColors(true)
		);
		log.set_config_option(ConfigOption::DisplayColors(false))?;
		assert_eq!(
			log.get_config_option(ConfigOptionName::DisplayColors)?,
			ConfigOption::DisplayColors(false)
		);

		log.set_config_option(ConfigOption::DisplayColors(true))?;
		assert_eq!(
			log.get_config_option(ConfigOptionName::DisplayColors)?,
			ConfigOption::DisplayColors(true)
		);

		log.log(LogLevel::Debug, "test11")?;
		log.log(LogLevel::Debug, "test12")?;
		log.log(LogLevel::Debug, "test13")?;
		log.log_plain(LogLevel::Fatal, "plaintextfatal")?;
		log.log(LogLevel::Trace, "thisdoesnotshowup")?;

		let mut f = File::open(format!("{}/test.log", directory))?;
		let mut s = String::new();
		f.read_to_string(&mut s)?;

		let test10_loc = s.find("test10").unwrap();
		let test11_loc = s.find("test11").unwrap();
		let test12_loc = s.find("test12").unwrap();
		let test13_loc = s.find("test13").unwrap();
		let plain_text_fatal_loc = s.find("\nplaintextfatal").unwrap();

		assert!(test10_loc > 0);
		assert!(test10_loc < test11_loc);
		assert!(test11_loc < test12_loc);
		assert!(test12_loc < test13_loc);
		assert!(plain_text_fatal_loc > test13_loc);
		assert!(s.find("thisdoesnotshowup").is_none());

		Ok(())
	}

	#[test]
	fn test_log_macros() -> Result<(), Error> {
		assert!(set_log_option!(AutoRotate(false)).is_err());
		assert!(get_log_option!(AutoRotate).is_err());

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

		assert!(set_log_option!(Debug(false)).is_err());
		assert!(get_log_option!(Debug).is_err());

		assert!(set_log_option!(AutoRotate(false)).is_ok());
		assert!(get_log_option!(AutoRotate).is_ok());

		Ok(())
	}

	#[test]
	fn test_log_rotate() -> Result<(), Error> {
		let test_info = test_info!(true)?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(Some(buf))
		)?;

		log.init()?;
		log.set_log_level(LogLevel::Debug);
		assert!(!log.need_rotate()?);
		for _ in 0..10 {
			log.log_plain(LogLevel::Info, "0123456789")?;
		}

		assert!(log.need_rotate()?);
		log.rotate()?;
		assert!(!log.need_rotate()?);

		log.log_plain(LogLevel::Info, "test")?;
		assert!(!log.need_rotate()?);
		sleep(Duration::from_millis(6_000));
		assert!(log.need_rotate()?);
		log.rotate()?;

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
		assert_eq!(rotated_files, 2);
		assert_eq!(unrotated_files, 1);

		Ok(())
	}

	#[test]
	fn test_log_auto_rotate() -> Result<(), Error> {
		let test_info = test_info!(true)?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(Some(buf)),
			AutoRotate(true)
		)?;

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
			if file_name.find("rotate.r") == Some(0) {
				rotated_files += 1;
			}
			if file_name.find("rotate.log") == Some(0) {
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
	fn test_log_errors() -> Result<(), Error> {
		let test_info = test_info!(true)?;
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("rotate.log");
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(Some(buf)),
			AutoRotate(true)
		)?;

		assert!(log.rotate().is_err());

		log.init()?;
		assert!(log.init().is_err());
		assert!(log.close().is_ok());
		Ok(())
	}

	#[test]
	fn test_log_stdoutonly() -> Result<(), Error> {
		let mut log = logger!(
			MaxSizeBytes(100),
			MaxAgeMillis(3_000),
			LogFilePath(None),
			AutoRotate(true),
		)?;
		assert!(log.need_rotate().is_err());
		log.init()?;
		assert!(log.rotate().is_err());
		Ok(())
	}

	#[test]
	fn test_log_logger_macro() -> Result<(), Error> {
		let mut log = logger!(MaxSizeBytes(103), MaxAgeMillis(3_000), LogFilePath(None))?;
		log.init()?;
		assert!(log.init().is_err());

		assert_eq!(
			log.get_config_option(ConfigOptionName::MaxSizeBytes)?,
			ConfigOption::MaxSizeBytes(103)
		);
		Ok(())
	}
}
