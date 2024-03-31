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
			MaxSizeBytes(10),
			MaxAgeMillis(10),
			DisplayColors(true),
			DisplayStdout(true),
			DisplayTimestamp(true),
			DisplayLogLevel(true),
			DisplayLineNum(true),
			DisplayMillis(true),
			LogFilePath(None),
			DisplayBackTrace(true),
			LineNumDataMaxLen(10),
			FileHeader("test".to_string()),
			DeleteRotation(true),
			AutoRotate(true),
			MaxEntries(10),
			MaxLoadFactor(10.0),
			SlabSize(10),
			SlabCount(10),
			MinSize(10),
			MaxSize(10),
			SyncChannelSize(10),
			GlobalSlabAllocator(true),
			Start(10),
			End(10),
			MatchId(10),
			Regex("test".to_string()),
			IsCaseSensitive(true),
			IsTerminationPattern(true),
			IsMultiLine(true),
			PatternId(10),
			IsHashtable(true),
			IsHashset(true),
			IsList(true),
			EvhTimeout(123),
			Debug(true),
			DebugLargeSlabCount(true)
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
					CN::Start,
					CN::End,
					CN::MatchId,
					CN::Regex,
					CN::IsCaseSensitive,
					CN::IsTerminationPattern,
					CN::IsMultiLine,
					CN::PatternId,
					CN::IsHashtable,
					CN::IsHashset,
					CN::IsList,
					CN::Debug,
					CN::DebugLargeSlabCount,
					CN::EvhTimeout,
				],
				vec![]
			)
			.is_ok());

		assert!(config.get_or_bool(&CN::DisplayColors, false));
		assert!(config.get_or_bool(&CN::AutoRotate, false));
		assert!(config.get_or_bool(&CN::DisplayStdout, false));
		assert!(config.get_or_bool(&CN::DisplayTimestamp, false));
		assert!(config.get_or_bool(&CN::DisplayLogLevel, false));
		assert!(config.get_or_bool(&CN::DisplayLineNum, false));
		assert!(config.get_or_bool(&CN::DisplayMillis, false));
		assert!(config.get_or_bool(&CN::DisplayBackTrace, false));
		assert!(config.get_or_bool(&CN::DeleteRotation, false));
		assert!(config.get_or_bool(&CN::GlobalSlabAllocator, false));
		assert!(config.get_or_bool(&CN::IsCaseSensitive, false));
		assert!(config.get_or_bool(&CN::IsTerminationPattern, false));
		assert!(config.get_or_bool(&CN::IsMultiLine, false));
		assert!(config.get_or_bool(&CN::IsHashtable, false));
		assert!(config.get_or_bool(&CN::IsHashset, false));
		assert!(config.get_or_bool(&CN::IsList, false));
		assert!(config.get_or_bool(&CN::Debug, false));
		assert!(config.get_or_bool(&CN::DebugLargeSlabCount, false));
		assert!(!config.get_or_bool(&CN::MaxSize, false));

		assert_eq!(config.get_or_usize(&CN::MaxEntries, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::SlabSize, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::SlabCount, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::MinSize, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::MaxSize, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::SyncChannelSize, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::Start, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::End, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::MatchId, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::PatternId, usize::MAX), 10);
		assert_eq!(config.get_or_usize(&CN::AutoRotate, usize::MAX), usize::MAX);

		assert_eq!(config.get_or_u64(&CN::MaxSizeBytes, u64::MAX), 10);
		assert_eq!(config.get_or_u64(&CN::MaxAgeMillis, u64::MAX), 10);
		assert_eq!(config.get_or_u64(&CN::LineNumDataMaxLen, u64::MAX), 10);
		assert_eq!(config.get_or_u64(&CN::AutoRotate, u64::MAX), u64::MAX);

		assert_eq!(config.get_or_u16(&CN::EvhTimeout, u16::MAX), 123);
		assert_eq!(config.get_or_u16(&CN::AutoRotate, u16::MAX), u16::MAX);

		assert_eq!(
			config.get_or_string(&CN::FileHeader, "".to_string()),
			"test".to_string()
		);
		assert_eq!(
			config.get_or_string(&CN::Regex, "".to_string()),
			"test".to_string()
		);
		assert_eq!(
			config.get_or_string(&CN::AutoRotate, "".to_string()),
			"".to_string()
		);

		assert_eq!(config.get_or_f64(&CN::MaxLoadFactor, f64::MAX), 10.0);
		assert_eq!(config.get_or_f64(&CN::AutoRotate, f64::MAX), f64::MAX);

		let empty_config = config!();
		assert_eq!(empty_config.get_or_bool(&CN::MaxSize, false), false);
		assert_eq!(empty_config.get_or_usize(&CN::AutoRotate, 0), 0);
		assert_eq!(empty_config.get_or_u64(&CN::AutoRotate, 0), 0);
		assert_eq!(
			empty_config.get_or_string(&CN::AutoRotate, "".to_string()),
			"".to_string()
		);
		assert_eq!(empty_config.get_or_f64(&CN::AutoRotate, 0.0), 0.0);

		let config = config!(AutoRotate(false));

		assert_eq!(config.get_or_u16(&CN::EvhTimeout, u16::MAX), u16::MAX);

		Ok(())
	}

	#[test]
	fn test_config_with_debug() -> Result<(), Error> {
		let config = config!(Debug(true));
		assert!(config.check_config(vec![CN::Debug], vec![]).is_ok());
		Ok(())
	}
}
