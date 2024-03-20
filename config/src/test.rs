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
	use crate as bmw_conf;
	use crate::{config, ConfigBuilder, ConfigOption, ConfigOption::*, ConfigOptionName as CN};
	use bmw_err::*;

	#[test]
	fn test_config_basic() -> Result<(), Error> {
		let config = ConfigBuilder::build_config(vec![ConfigOption::MaxSizeBytes(1_000)]);
		assert_eq!(
			config.get(&CN::MaxSizeBytes),
			Some(ConfigOption::MaxSizeBytes(1_000))
		);

		assert_eq!(config.get(&CN::MaxAgeMillis), None);

		// ok because MaxSizeBytes is allowed
		assert!(config.check_config(vec![CN::MaxSizeBytes], vec![]).is_ok());

		// err because MaxSizeBytes is not allowed
		assert!(config.check_config(vec![CN::MaxAgeMillis], vec![]).is_err());

		// ok because MaxSizeBytes is allowed
		assert!(config
			.check_config(vec![CN::AutoRotate, CN::MaxSizeBytes], vec![])
			.is_ok());

		let config = ConfigBuilder::build_config(vec![
			ConfigOption::MaxSizeBytes(1_000),
			ConfigOption::MaxSizeBytes(100),
		]);

		// err because it's a duplicate
		assert!(config.check_config(vec![CN::MaxSizeBytes], vec![]).is_err());

		let config = ConfigBuilder::build_config(vec![ConfigOption::MaxSizeBytes(100)]);

		// ok because it's both allowed and required and specified
		assert!(config
			.check_config(vec![CN::MaxSizeBytes], vec![CN::MaxSizeBytes])
			.is_ok());

		// err because MaxAgeMillis is not specified and it's required
		assert!(config
			.check_config(vec![CN::MaxSizeBytes], vec![CN::MaxAgeMillis])
			.is_err());

		Ok(())
	}

	#[test]
	fn test_config_macros() -> Result<(), Error> {
		let config = config!(FileHeader("test".to_string()), DeleteRotation(false));

		assert_eq!(
			config.get(&CN::FileHeader),
			Some(FileHeader("test".to_string()))
		);

		// ok because the two values set are allowed
		assert!(config
			.check_config(
				vec![CN::FileHeader, CN::DeleteRotation, CN::AutoRotate],
				vec![]
			)
			.is_ok());

		// err because AutoRotate was required and not specified
		assert!(config
			.check_config(
				vec![CN::FileHeader, CN::DeleteRotation, CN::AutoRotate],
				vec![CN::AutoRotate]
			)
			.is_err());

		// ok because only FileHeader was required now and it was specified
		assert!(config
			.check_config(
				vec![CN::FileHeader, CN::DeleteRotation, CN::AutoRotate],
				vec![CN::FileHeader]
			)
			.is_ok());

		// err because DeleteRotation was not allowed
		assert!(config
			.check_config(vec![CN::FileHeader, CN::AutoRotate], vec![CN::FileHeader])
			.is_err());

		Ok(())
	}

	#[test]
	fn test_config_all_options() -> Result<(), Error> {
		// create a config with everything
		let config = config!(
			MaxSizeBytes(100),
			MaxAgeMillis(200),
			DisplayColors(true),
			DisplayStdout(true),
			DisplayTimestamp(true),
			DisplayLogLevel(false),
			DisplayLineNum(false),
			DisplayMillis(false),
			LogFilePath(None),
			DisplayBackTrace(false),
			LineNumDataMaxLen(300),
			FileHeader("test".to_string()),
			DeleteRotation(false),
			AutoRotate(true),
			MaxEntries(100),
			MaxLoadFactor(0.5),
			SlabSize(100),
			SlabCount(200),
			MinSize(1),
			MaxSize(100),
			SyncChannelSize(1),
			GlobalSlabAllocator(false),
			Debug(true)
		);

		// since everything is allowed, it's ok
		assert!(config
			.check_config(
				vec![
					CN::MaxSizeBytes,
					CN::MaxAgeMillis,
					CN::DisplayColors,
					CN::DisplayStdout,
					CN::DisplayTimestamp,
					CN::DisplayLogLevel,
					CN::DisplayLineNum,
					CN::DisplayMillis,
					CN::DisplayBackTrace,
					CN::LogFilePath,
					CN::LineNumDataMaxLen,
					CN::DeleteRotation,
					CN::FileHeader,
					CN::AutoRotate,
					CN::MaxEntries,
					CN::MaxLoadFactor,
					CN::SlabSize,
					CN::SlabCount,
					CN::MinSize,
					CN::MaxSize,
					CN::SyncChannelSize,
					CN::GlobalSlabAllocator,
					CN::Debug
				],
				vec![]
			)
			.is_ok());
		Ok(())
	}

	#[test]
	fn test_config_with_debug() -> Result<(), Error> {
		let config = config!(Debug(true));
		assert!(config.check_config(vec![CN::Debug], vec![]).is_ok());
		Ok(())
	}
}
