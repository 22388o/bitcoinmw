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

use bmw_err::Error;
use std::collections::HashMap;
use std::path::PathBuf;

/// The config trait allows for easy construction of configurations. Configurations can be
/// retreived with the [`crate::Config::get`] function and configurations can be checked with the
/// [`crate::Config::check_config`] function.
pub trait Config {
	fn get(&self, name: &ConfigOptionName) -> Option<ConfigOption>;
	fn check_config(
		&self,
		allowed: Vec<ConfigOptionName>,
		required: Vec<ConfigOptionName>,
	) -> Result<(), Error>;
}

/// Names of configuration options used throughout BMW via macro. This correspondes to the values
/// in [`crate::ConfigOption`].
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum ConfigOptionName {
	MaxSizeBytes,
	MaxAgeMillis,
	DisplayColors,
	DisplayStdout,
	DisplayTimestamp,
	DisplayLogLevel,
	DisplayLineNum,
	DisplayMillis,
	LogFilePath,
	AutoRotate,
	DisplayBackTrace,
	LineNumDataMaxLen,
	DeleteRotation,
	FileHeader,
	Debug,
}

/// Configuration options used throughout BMW via macro.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum ConfigOption {
	MaxSizeBytes(u64),
	MaxAgeMillis(u128),
	DisplayColors(bool),
	DisplayStdout(bool),
	DisplayTimestamp(bool),
	DisplayLogLevel(bool),
	DisplayLineNum(bool),
	DisplayMillis(bool),
	LogFilePath(Option<Box<PathBuf>>),
	AutoRotate(bool),
	DisplayBackTrace(bool),
	LineNumDataMaxLen(usize),
	DeleteRotation(bool),
	FileHeader(String),
	Debug(bool),
}

/// A builder struct which can be used to build configs. This is typically done using the
/// [`crate::config!`] macro which calls this builder.
pub struct ConfigBuilder {}

// Crate local structures

#[derive(Clone, Debug)]
pub(crate) struct ConfigImpl {
	pub(crate) configs: Vec<ConfigOption>,
	pub(crate) hash: HashMap<ConfigOptionName, ConfigOption>,
}
