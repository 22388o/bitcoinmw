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
	use crate::log::Logger;
	use crate::LogBuilder;
	use crate::LogConstOptions::*;
	use crate::LogLevel;
	use bmw_core::*;
	use bmw_test::*;
	use std::fs::File;
	use std::io::Read;
	use std::path::PathBuf;

	const DEFAULT_LINE_NUM_DATA_MAX_LEN: u16 = 30;

	#[test]
	fn test_log_basic() -> Result<(), Error> {
		let test_info = test_info!()?; // obtain test info struct
		let directory = test_info.directory();
		let mut buf = PathBuf::new();
		buf.push(directory);
		buf.push("test.log");
		// create a logger with auto rotate on ( use impl so we can check the config )
		let path = buf.display().to_string();
		let configs = vec![
			AutoRotate(true),
			LogFilePath(&path),
			FileHeader("testing123"),
		];
		let mut log = LogBuilder::build_logger(configs)?;

		// debug log level
		log.set_log_level(LogLevel::Debug);
		log.init()?; // in logger
		log.log(LogLevel::Debug, "test10")?; // log a message

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
}
