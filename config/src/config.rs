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

macro_rules! cc {
	($self:expr, $set:expr, $specified:expr, $option_name:expr) => {{
		let config_option_name = $option_name;
		let i = $option_name as usize;
		$self.check_set(&$set, &config_option_name)?;
		$self.check_index(i, $specified, format!("{:?}", config_option_name))?;
	}};
}

impl Config for ConfigImpl {
	fn get(&self, name: &CN) -> Option<ConfigOption> {
		self.hash.get(name).cloned()
	}

	fn check_config(&self, allowed: Vec<CN>, required: Vec<CN>) -> Result<(), Error> {
		self.check_config_impl(allowed, required)
	}
}

impl ConfigImpl {
	pub fn new(configs: Vec<ConfigOption>) -> Self {
		let mut hash = HashMap::new();
		for config in &configs {
			match config {
				MaxSizeBytes(_) => {
					hash.insert(CN::MaxSizeBytes, config.clone());
				}
				_ => {}
			}
		}
		Self { configs, hash }
	}

	pub fn check_config_impl(&self, allowed: Vec<CN>, required: Vec<CN>) -> Result<(), Error> {
		let mut t = HashSet::new();
		let mut s = vec![];
		for a in &allowed {
			t.insert(a);
		}
		for v in &self.configs {
			match v {
				MaxSizeBytes(_) => cc!(self, t, &mut s, CN::MaxSizeBytes),
				MaxAgeMillis(_) => cc!(self, t, &mut s, CN::MaxAgeMillis),
				DisplayColors(_) => cc!(self, t, &mut s, CN::DisplayColors),
				DisplayStdout(_) => cc!(self, t, &mut s, CN::DisplayStdout),
				DisplayTimestamp(_) => cc!(self, t, &mut s, CN::DisplayTimestamp),
				DisplayLogLevel(_) => cc!(self, t, &mut s, CN::DisplayLogLevel),
				DisplayLineNum(_) => cc!(self, t, &mut s, CN::DisplayLineNum),
				DisplayMillis(_) => cc!(self, t, &mut s, CN::DisplayMillis),
				LogFilePath(_) => cc!(self, t, &mut s, CN::LogFilePath),
				AutoRotate(_) => cc!(self, t, &mut s, CN::AutoRotate),
				DisplayBackTrace(_) => cc!(self, t, &mut s, CN::DisplayBackTrace),
				LineNumDataMaxLen(_) => cc!(self, t, &mut s, CN::LineNumDataMaxLen),
				DeleteRotation(_) => cc!(self, t, &mut s, CN::DeleteRotation),
				FileHeader(_) => cc!(self, t, &mut s, CN::FileHeader),
			}
		}

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

	fn check_set(&self, set: &HashSet<&CN>, option: &CN) -> Result<(), Error> {
		if set.contains(option) {
			Ok(())
		} else {
			Err(err!(ErrKind::Configuration, "{:?} is not allowed", option))
		}
	}

	fn check_index(&self, i: usize, specified: &mut Vec<bool>, name: String) -> Result<(), Error> {
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
			specified[i] = true;
			Ok(())
		}
	}
}
