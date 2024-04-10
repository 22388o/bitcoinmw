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

use crate::types::ConfigImpl;
use crate::{Config, ConfigOption, ConfigOption::*, ConfigOptionName as CN};
use bmw_err::*;
use std::collections::{HashMap, HashSet};

// macro to simplify the process of checking the parameters
macro_rules! cc {
	($self:expr, $set:expr, $specified:expr, $option_name:expr, $dup:expr) => {{
		let config_option_name = $option_name;
		let i = $option_name as usize;
		$self.check_set(&$set, &config_option_name)?;
		$self.check_index(i, $specified, format!("{:?}", config_option_name), &$dup)?;
	}};
}

macro_rules! multi {
	($opt:ident, $name:expr, $ret:expr, $config:expr) => {
		match $config {
			ConfigOption::$opt(_) => {
				if $name == &CN::$opt {
					$ret.push($config.clone());
				}
			}
			_ => {}
		}
	};
}

// Config implementation just return values from the Impl structure.
impl Config for ConfigImpl {
	fn get(&self, name: &CN) -> Option<ConfigOption> {
		self.hash.get(name).cloned()
	}

	fn get_multi(&self, name: &CN) -> Vec<ConfigOption> {
		let mut ret = vec![];
		for config in &self.configs {
			multi!(HttpHeader, name, ret, config);
			multi!(FileHeader, name, ret, config);
		}

		ret
	}

	fn get_or_bool(&self, name: &CN, default: bool) -> bool {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::DisplayColors(v) => *v,
				ConfigOption::AutoRotate(v) => *v,
				ConfigOption::DeleteRotation(v) => *v,
				ConfigOption::DisplayLogLevel(v) => *v,
				ConfigOption::DisplayLineNum(v) => *v,
				ConfigOption::DisplayBackTrace(v) => *v,
				ConfigOption::DisplayMillis(v) => *v,
				ConfigOption::DisplayStdout(v) => *v,
				ConfigOption::DisplayTimestamp(v) => *v,
				ConfigOption::GlobalSlabAllocator(v) => *v,
				ConfigOption::IsCaseSensitive(v) => *v,
				ConfigOption::IsTerminationPattern(v) => *v,
				ConfigOption::IsMultiLine(v) => *v,
				ConfigOption::IsHashtable(v) => *v,
				ConfigOption::IsHashset(v) => *v,
				ConfigOption::IsList(v) => *v,
				ConfigOption::DebugNoChunks(v) => *v,
				ConfigOption::Debug(v) => *v,
				ConfigOption::IsSync(v) => *v,
				ConfigOption::DebugLargeSlabCount(v) => *v,
				_ => default,
			},
			None => default,
		}
	}

	fn get_or_usize(&self, name: &CN, default: usize) -> usize {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::MaxEntries(v) => *v,
				ConfigOption::SlabSize(v) => *v,
				ConfigOption::SlabCount(v) => *v,
				ConfigOption::MinSize(v) => *v,
				ConfigOption::MaxSize(v) => *v,
				ConfigOption::SyncChannelSize(v) => *v,
				ConfigOption::Start(v) => *v,
				ConfigOption::End(v) => *v,
				ConfigOption::MatchId(v) => *v,
				ConfigOption::PatternId(v) => *v,
				ConfigOption::TerminationLength(v) => *v,
				ConfigOption::MaxWildCardLength(v) => *v,
				ConfigOption::EvhThreads(v) => *v,
				ConfigOption::EvhHouseKeeperFrequencyMillis(v) => *v,
				ConfigOption::EvhStatsUpdateMillis(v) => *v,
				ConfigOption::EvhReadSlabSize(v) => *v,
				ConfigOption::EvhReadSlabCount(v) => *v,
				_ => default,
			},
			None => default,
		}
	}

	fn get_or_u16(&self, name: &CN, default: u16) -> u16 {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::EvhTimeout(v) => *v,
				ConfigOption::Port(v) => *v,
				_ => default,
			},
			None => default,
		}
	}

	fn get_or_u64(&self, name: &CN, default: u64) -> u64 {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::MaxSizeBytes(v) => *v,
				ConfigOption::MaxAgeMillis(v) => *v,
				ConfigOption::LineNumDataMaxLen(v) => *v,
				ConfigOption::HttpTimeoutMillis(v) => *v,
				_ => default,
			},
			None => default,
		}
	}

	fn get_or_string(&self, name: &CN, default: String) -> String {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::FileHeader(v) => v.to_string(),
				ConfigOption::Regex(v) => v.to_string(),
				ConfigOption::HttpAccept(v) => v.to_string(),
				ConfigOption::HttpMeth(v) => v.to_string(),
				ConfigOption::HttpVers(v) => v.to_string(),
				ConfigOption::HttpConnection(v) => v.to_string(),
				ConfigOption::HttpRequestUri(v) => v.to_string(),
				ConfigOption::HttpRequestUrl(v) => v.to_string(),
				ConfigOption::HttpUserAgent(v) => v.to_string(),
				ConfigOption::Host(v) => v.to_string(),
				ConfigOption::BaseDir(v) => v.to_string(),
				ConfigOption::ServerName(v) => v.to_string(),
				_ => default,
			},
			None => default,
		}
	}

	fn get_or_f64(&self, name: &CN, default: f64) -> f64 {
		match self.hash.get(name) {
			Some(v) => match v {
				ConfigOption::MaxLoadFactor(v) => *v,
				_ => default,
			},
			None => default,
		}
	}

	fn check_config(&self, allowed: Vec<CN>, required: Vec<CN>) -> Result<(), Error> {
		self.check_config_impl(allowed, required, vec![])
	}

	fn check_config_duplicates(
		&self,
		allowed: Vec<CN>,
		required: Vec<CN>,
		allow_duplicates: Vec<CN>,
	) -> Result<(), Error> {
		self.check_config_impl(allowed, required, allow_duplicates)
	}
}

impl ConfigImpl {
	// create a new config based on the specified input.
	pub fn new(configs: Vec<ConfigOption>) -> Self {
		// create a hashmap to insert configs for the ability to look them up later.
		let mut hash = HashMap::new();
		for config in &configs {
			let _ = match config {
				MaxSizeBytes(_) => hash.insert(CN::MaxSizeBytes, config.clone()),
				MaxAgeMillis(_) => hash.insert(CN::MaxAgeMillis, config.clone()),
				DisplayColors(_) => hash.insert(CN::DisplayColors, config.clone()),
				DisplayStdout(_) => hash.insert(CN::DisplayStdout, config.clone()),
				DisplayTimestamp(_) => hash.insert(CN::DisplayTimestamp, config.clone()),
				DisplayLogLevel(_) => hash.insert(CN::DisplayLogLevel, config.clone()),
				DisplayLineNum(_) => hash.insert(CN::DisplayLineNum, config.clone()),
				DisplayMillis(_) => hash.insert(CN::DisplayMillis, config.clone()),
				LogFilePath(_) => hash.insert(CN::LogFilePath, config.clone()),
				AutoRotate(_) => hash.insert(CN::AutoRotate, config.clone()),
				DisplayBackTrace(_) => hash.insert(CN::DisplayBackTrace, config.clone()),
				LineNumDataMaxLen(_) => hash.insert(CN::LineNumDataMaxLen, config.clone()),
				DeleteRotation(_) => hash.insert(CN::DeleteRotation, config.clone()),
				FileHeader(_) => hash.insert(CN::FileHeader, config.clone()),
				MaxEntries(_) => hash.insert(CN::MaxEntries, config.clone()),
				MaxLoadFactor(_) => hash.insert(CN::MaxLoadFactor, config.clone()),
				SlabSize(_) => hash.insert(CN::SlabSize, config.clone()),
				SlabCount(_) => hash.insert(CN::SlabCount, config.clone()),
				MinSize(_) => hash.insert(CN::MinSize, config.clone()),
				MaxSize(_) => hash.insert(CN::MaxSize, config.clone()),
				SyncChannelSize(_) => hash.insert(CN::SyncChannelSize, config.clone()),
				GlobalSlabAllocator(_) => hash.insert(CN::GlobalSlabAllocator, config.clone()),
				Start(_) => hash.insert(CN::Start, config.clone()),
				End(_) => hash.insert(CN::End, config.clone()),
				MatchId(_) => hash.insert(CN::MatchId, config.clone()),
				Regex(_) => hash.insert(CN::Regex, config.clone()),
				IsCaseSensitive(_) => hash.insert(CN::IsCaseSensitive, config.clone()),
				IsTerminationPattern(_) => hash.insert(CN::IsTerminationPattern, config.clone()),
				IsMultiLine(_) => hash.insert(CN::IsMultiLine, config.clone()),
				PatternId(_) => hash.insert(CN::PatternId, config.clone()),
				IsHashtable(_) => hash.insert(CN::IsHashtable, config.clone()),
				IsHashset(_) => hash.insert(CN::IsHashset, config.clone()),
				IsList(_) => hash.insert(CN::IsList, config.clone()),
				TerminationLength(_) => hash.insert(CN::TerminationLength, config.clone()),
				MaxWildCardLength(_) => hash.insert(CN::MaxWildCardLength, config.clone()),
				IsSync(_) => hash.insert(CN::IsSync, config.clone()),
				EvhThreads(_) => hash.insert(CN::EvhThreads, config.clone()),
				EvhHouseKeeperFrequencyMillis(_) => {
					hash.insert(CN::EvhHouseKeeperFrequencyMillis, config.clone())
				}
				EvhStatsUpdateMillis(_) => hash.insert(CN::EvhStatsUpdateMillis, config.clone()),
				EvhTimeout(_) => hash.insert(CN::EvhTimeout, config.clone()),
				EvhReadSlabSize(_) => hash.insert(CN::EvhReadSlabSize, config.clone()),
				EvhReadSlabCount(_) => hash.insert(CN::EvhReadSlabCount, config.clone()),
				Port(_) => hash.insert(CN::Port, config.clone()),
				Host(_) => hash.insert(CN::Host, config.clone()),
				BaseDir(_) => hash.insert(CN::BaseDir, config.clone()),
				ServerName(_) => hash.insert(CN::ServerName, config.clone()),
				HttpContentFile(_) => hash.insert(CN::HttpContentFile, config.clone()),
				HttpContentData(_) => hash.insert(CN::HttpContentData, config.clone()),
				HttpAccept(_) => hash.insert(CN::HttpAccept, config.clone()),
				HttpHeader(_) => hash.insert(CN::HttpHeader, config.clone()),
				HttpTimeoutMillis(_) => hash.insert(CN::HttpTimeoutMillis, config.clone()),
				HttpMeth(_) => hash.insert(CN::HttpMeth, config.clone()),
				HttpVers(_) => hash.insert(CN::HttpVers, config.clone()),
				HttpConnection(_) => hash.insert(CN::HttpConnection, config.clone()),
				HttpRequestUri(_) => hash.insert(CN::HttpRequestUri, config.clone()),
				HttpRequestUrl(_) => hash.insert(CN::HttpRequestUrl, config.clone()),
				HttpUserAgent(_) => hash.insert(CN::HttpUserAgent, config.clone()),
				DebugNoChunks(_) => hash.insert(CN::DebugNoChunks, config.clone()),
				Debug(_) => hash.insert(CN::Debug, config.clone()),
				DebugLargeSlabCount(_) => hash.insert(CN::DebugLargeSlabCount, config.clone()),
			};
		}
		Self { configs, hash }
	}

	// check the config: 1.) for duplicates, 2.) for allowed input 3.) for the required input.
	pub fn check_config_impl(
		&self,
		allowed: Vec<CN>,
		required: Vec<CN>,
		allow_duplicates: Vec<CN>,
	) -> Result<(), Error> {
		let mut d = HashSet::new();
		for dup in &allow_duplicates {
			d.insert(format!("{:?}", dup));
		}

		let mut t = HashSet::new();
		let mut s = vec![];
		for a in &allowed {
			t.insert(a);
		}

		// the cc macro handles #1 and #2 above
		for v in &self.configs {
			match v {
				MaxSizeBytes(_) => cc!(self, t, &mut s, CN::MaxSizeBytes, d),
				MaxAgeMillis(_) => cc!(self, t, &mut s, CN::MaxAgeMillis, d),
				DisplayColors(_) => cc!(self, t, &mut s, CN::DisplayColors, d),
				DisplayStdout(_) => cc!(self, t, &mut s, CN::DisplayStdout, d),
				DisplayTimestamp(_) => cc!(self, t, &mut s, CN::DisplayTimestamp, d),
				DisplayLogLevel(_) => cc!(self, t, &mut s, CN::DisplayLogLevel, d),
				DisplayLineNum(_) => cc!(self, t, &mut s, CN::DisplayLineNum, d),
				DisplayMillis(_) => cc!(self, t, &mut s, CN::DisplayMillis, d),
				LogFilePath(_) => cc!(self, t, &mut s, CN::LogFilePath, d),
				AutoRotate(_) => cc!(self, t, &mut s, CN::AutoRotate, d),
				DisplayBackTrace(_) => cc!(self, t, &mut s, CN::DisplayBackTrace, d),
				LineNumDataMaxLen(_) => cc!(self, t, &mut s, CN::LineNumDataMaxLen, d),
				DeleteRotation(_) => cc!(self, t, &mut s, CN::DeleteRotation, d),
				FileHeader(_) => cc!(self, t, &mut s, CN::FileHeader, d),
				MaxEntries(_) => cc!(self, t, &mut s, CN::MaxEntries, d),
				MaxLoadFactor(_) => cc!(self, t, &mut s, CN::MaxLoadFactor, d),
				SlabSize(_) => cc!(self, t, &mut s, CN::SlabSize, d),
				SlabCount(_) => cc!(self, t, &mut s, CN::SlabCount, d),
				MinSize(_) => cc!(self, t, &mut s, CN::MinSize, d),
				MaxSize(_) => cc!(self, t, &mut s, CN::MaxSize, d),
				SyncChannelSize(_) => cc!(self, t, &mut s, CN::SyncChannelSize, d),
				GlobalSlabAllocator(_) => cc!(self, t, &mut s, CN::GlobalSlabAllocator, d),
				Start(_) => cc!(self, t, &mut s, CN::Start, d),
				End(_) => cc!(self, t, &mut s, CN::End, d),
				MatchId(_) => cc!(self, t, &mut s, CN::MatchId, d),
				Regex(_) => cc!(self, t, &mut s, CN::Regex, d),
				IsCaseSensitive(_) => cc!(self, t, &mut s, CN::IsCaseSensitive, d),
				IsTerminationPattern(_) => cc!(self, t, &mut s, CN::IsTerminationPattern, d),
				IsMultiLine(_) => cc!(self, t, &mut s, CN::IsMultiLine, d),
				PatternId(_) => cc!(self, t, &mut s, CN::PatternId, d),
				IsHashtable(_) => cc!(self, t, &mut s, CN::IsHashtable, d),
				IsHashset(_) => cc!(self, t, &mut s, CN::IsHashset, d),
				IsList(_) => cc!(self, t, &mut s, CN::IsList, d),
				TerminationLength(_) => cc!(self, t, &mut s, CN::TerminationLength, d),
				MaxWildCardLength(_) => cc!(self, t, &mut s, CN::MaxWildCardLength, d),
				IsSync(_) => cc!(self, t, &mut s, CN::IsSync, d),
				EvhThreads(_) => cc!(self, t, &mut s, CN::EvhThreads, d),
				EvhHouseKeeperFrequencyMillis(_) => {
					cc!(self, t, &mut s, CN::EvhHouseKeeperFrequencyMillis, d)
				}
				EvhStatsUpdateMillis(_) => cc!(self, t, &mut s, CN::EvhStatsUpdateMillis, d),
				EvhTimeout(_) => cc!(self, t, &mut s, CN::EvhTimeout, d),
				EvhReadSlabSize(_) => cc!(self, t, &mut s, CN::EvhReadSlabSize, d),
				EvhReadSlabCount(_) => cc!(self, t, &mut s, CN::EvhReadSlabCount, d),
				Port(_) => cc!(self, t, &mut s, CN::Port, d),
				Host(_) => cc!(self, t, &mut s, CN::Host, d),
				BaseDir(_) => cc!(self, t, &mut s, CN::BaseDir, d),
				ServerName(_) => cc!(self, t, &mut s, CN::ServerName, d),
				HttpContentFile(_) => cc!(self, t, &mut s, CN::HttpContentFile, d),
				HttpContentData(_) => cc!(self, t, &mut s, CN::HttpContentData, d),
				HttpAccept(_) => cc!(self, t, &mut s, CN::HttpAccept, d),
				HttpHeader(_) => cc!(self, t, &mut s, CN::HttpHeader, d),
				HttpTimeoutMillis(_) => cc!(self, t, &mut s, CN::HttpTimeoutMillis, d),
				HttpMeth(_) => cc!(self, t, &mut s, CN::HttpMeth, d),
				HttpVers(_) => cc!(self, t, &mut s, CN::HttpVers, d),
				HttpConnection(_) => cc!(self, t, &mut s, CN::HttpConnection, d),
				HttpRequestUri(_) => cc!(self, t, &mut s, CN::HttpRequestUri, d),
				HttpRequestUrl(_) => cc!(self, t, &mut s, CN::HttpRequestUrl, d),
				HttpUserAgent(_) => cc!(self, t, &mut s, CN::HttpUserAgent, d),
				DebugNoChunks(_) => cc!(self, t, &mut s, CN::DebugNoChunks, d),
				Debug(_) => cc!(self, t, &mut s, CN::Debug, d),
				DebugLargeSlabCount(_) => cc!(self, t, &mut s, CN::DebugLargeSlabCount, d),
			}
		}

		// #3 is covered here (required)
		let s_len = s.len();
		for v in required {
			let v_as_usize = v.clone() as usize;
			if v_as_usize >= s_len || !s[v_as_usize] {
				return Err(err!(
					ErrKind::Configuration,
					"{:?} was required and not specified",
					v
				));
			}
		}

		Ok(())
	}

	// convenience fn to check if the set contains this option and returns appropriate error
	fn check_set(&self, set: &HashSet<&CN>, option: &CN) -> Result<(), Error> {
		if set.contains(option) {
			Ok(())
		} else {
			Err(err!(ErrKind::Configuration, "{:?} is not allowed", option))
		}
	}

	// this checks for duplicates
	fn check_index(
		&self,
		i: usize,
		specified: &mut Vec<bool>,
		name: String,
		dupes: &HashSet<String>,
	) -> Result<(), Error> {
		if specified.len() <= i {
			specified.resize(i + 1, false);
		}

		if specified[i] {
			Err(err!(
				ErrKind::Configuration,
				"{} was specified more than once",
				name
			))
		} else {
			if dupes.get(&name).is_none() {
				specified[i] = true;
			}
			Ok(())
		}
	}
}
