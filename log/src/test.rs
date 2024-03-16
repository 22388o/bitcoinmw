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
	use bmw_log::{LogBuilder, LogLevel};
	use bmw_test::*;
	use std::path::Path;

	trace!();

	#[test]
	fn test_log_basic() -> Result<(), Error> {
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut log = LogBuilder::build_log(vec![
			ConfigOption::AutoRotate(true),
			ConfigOption::LogFilePath(Some(Box::new(
				Path::new(&format!("{}/test.log", directory)).to_path_buf(),
			))),
		])?;
		log.set_log_level(LogLevel::Debug)?;
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

		sleep(Duration::from_millis(1_000));
		log.log(LogLevel::Debug, "test11")?;
		sleep(Duration::from_millis(1_000));
		log.log(LogLevel::Debug, "test12")?;
		sleep(Duration::from_millis(1_000));
		log.log(LogLevel::Debug, "test13")?;
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_log_macros() -> Result<(), Error> {
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

		sleep(Duration::from_millis(1_000));
		Ok(())
	}
}
