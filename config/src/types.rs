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
use std::fmt::Debug;
use std::path::PathBuf;

/// The config trait allows for easy construction of configurations. Configurations can be
/// retrieved with the [`crate::Config::get`] function and configurations can be checked with the
/// [`crate::Config::check_config`] function.
pub trait Config {
	/// Get a configuration option if it exists or [`std::option::Option::None`] if no configuration
	/// with this name exists.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	///
	/// # Returns
	/// `Option<ConfigOption>` - If the configuration coresponding to this name exists, it is
	/// returned. Otherwise [`std::option::Option::None`] is returned.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(SlabCount(10), SlabSize(100));
	///
	///     assert_eq!(
	///         config.get(&ConfigOptionName::SlabCount).unwrap(),
	///         ConfigOption::SlabCount(10)
	///     );
	///
	///     assert_eq!(
	///         config.get(&ConfigOptionName::SlabSize).unwrap(),
	///         ConfigOption::SlabSize(100)
	///     );
	///
	///     assert_eq!(
	///         config.get(&ConfigOptionName::AutoRotate),
	///         None
	///     );
	///
	///     Ok(())
	/// }
	///```
	fn get(&self, name: &ConfigOptionName) -> Option<ConfigOption>;
	/// Gets a [`bool`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default [`bool`] - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `bool` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(AutoRotate(true), Debug(false));
	///
	///     assert_eq!(
	///         config.get_or_bool(&ConfigOptionName::AutoRotate, false),
	///         true
	///     );
	///
	///     assert_eq!(
	///         config.get_or_bool(&ConfigOptionName::Debug, false),
	///         false
	///     );
	///
	///     assert_eq!(
	///         config.get_or_bool(&ConfigOptionName::DisplayColors, false),
	///         false
	///     );
	///
	///     Ok(())
	/// }
	///```
	fn get_or_bool(&self, name: &ConfigOptionName, default: bool) -> bool;
	/// Gets a [`bool`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default [`bool`] - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `bool` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(AutoRotate(true), Debug(false), EvhTimeout(10));
	///
	///     assert_eq!(
	///         config.get_or_u16(&ConfigOptionName::AutoRotate, 0),
	///        0
	///     );
	///
	///     assert_eq!(
	///         config.get_or_u16(&ConfigOptionName::Debug, 0),
	///         0
	///     );
	///
	///     assert_eq!(
	///         config.get_or_u16(&ConfigOptionName::EvhTimeout, 0),
	///         10
	///     );
	///
	///     Ok(())
	/// }
	///```
	fn get_or_u16(&self, name: &ConfigOptionName, default: u16) -> u16;
	/// Gets a [`usize`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default ([`usize`]) - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `usize` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(SlabSize(500), SlabCount(100));
	///
	///     assert_eq!(
	///         config.get_or_usize(&ConfigOptionName::SlabSize, 0),
	///         500
	///     );
	///
	///     assert_eq!(
	///         config.get_or_usize(&ConfigOptionName::SlabCount, 0),
	///         100
	///     );
	///
	///     assert_eq!(
	///         config.get_or_usize(&ConfigOptionName::MaxEntries, 0),
	///         0
	///     );
	///
	///     Ok(())
	/// }
	///```
	fn get_or_usize(&self, name: &ConfigOptionName, default: usize) -> usize;
	/// Gets a [`u64`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default ([`u64`]) - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `u64` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(MaxSizeBytes(1_000_000), MaxAgeMillis(60 * 60 * 1_000));
	///
	///     assert_eq!(
	///         config.get_or_u64(&ConfigOptionName::MaxSizeBytes, 0),
	///         1_000_000
	///     );
	///
	///     assert_eq!(
	///         config.get_or_u64(&ConfigOptionName::MaxAgeMillis, 0),
	///         60 * 60 * 1_000
	///     );
	///
	///     assert_eq!(
	///         config.get_or_u64(&ConfigOptionName::LineNumDataMaxLen, 0),
	///         0
	///     );
	///
	///     Ok(())
	/// }
	///```
	fn get_or_u64(&self, name: &ConfigOptionName, default: u64) -> u64;
	/// Gets a [`std::string::String`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default ([`std::string::String`]) - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `std::string::String` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(FileHeader("myheader".to_string()), Regex("something".to_string()));
	///
	///     assert_eq!(
	///         config.get_or_string(&ConfigOptionName::FileHeader, "".to_string()),
	///         "myheader".to_string()
	///     );
	///
	///     assert_eq!(
	///         config.get_or_string(&ConfigOptionName::Regex, "".to_string()),
	///         "something".to_string()
	///     );
	///
	///     assert_eq!(
	///         config.get_or_string(&ConfigOptionName::LineNumDataMaxLen, "".to_string()),
	///         "".to_string()
	///     );
	///   
	///     Ok(())
	/// }
	///```
	fn get_or_string(&self, name: &ConfigOptionName, default: String) -> String;
	/// Gets a [`f64`] configuration option if it exists. If it doesn't exist, a default value
	/// is returned.
	///
	/// # Input Parameters
	/// * name ([`crate::ConfigOptionName`]) - The name of the configuration to retrieve.
	/// * default ([`f64`]) - This value is returned if the specified
	/// [`crate::ConfigOptionName`] was not found in the configuration.
	///
	/// # Returns
	/// `f64` - If the specified [`crate::ConfigOptionName`] is found, return it's value.
	/// Otherwise, return the `default` value.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(MaxSizeBytes(1_000_000), MaxLoadFactor(0.3));
	///
	///     assert_eq!(
	///         config.get_or_f64(&ConfigOptionName::MaxLoadFactor, 0.0),
	///         0.3
	///     );
	///
	///     assert_eq!(
	///         config.get_or_f64(&ConfigOptionName::MaxAgeMillis, 0.0),
	///         0.0
	///     );
	///   
	///     Ok(())
	/// }
	///```
	fn get_or_f64(&self, name: &ConfigOptionName, default: f64) -> f64;
	/// Checks a configuration and returns `Ok(())` if the configuration is valid. Otherwise,
	/// [`bmw_err::Error`] is returned.
	///
	/// # Input Parameters
	/// * allowed (`Vec<ConfigOptionName>`) - A [`std::vec::Vec`] of
	/// [`crate::ConfigOptionName`]s that are allowed to be specified in a configuration.
	/// * required (`Vec<ConfigOptionName>`) - A [`std::vec::Vec`] of
	/// [`crate::ConfigOptionName`]s that are required to be specified in a configuration.
	///
	/// # Returns
	/// Ok(()) on success or an error. See below.
	///
	/// # Errors
	/// * [`bmw_err::ErrorKind::Configuration`] - If a required [`crate::ConfigOptionName`] is
	///                                           not specifed, or if a [`crate::ConfigOptionName`]
	///                                           is specified more than once, or if a [`crate::ConfigOptionName`]
	///                                           is specified that is not in the input `allowed` options.
	///
	/// # Examples
	///
	///```
	/// use bmw_conf::*;
	/// use bmw_err::*;
	///
	/// fn main() -> Result<(), Error> {
	///     let config = config!(MaxSizeBytes(1_000_000), MaxLoadFactor(0.3));
	///
	///     // This configuration is ok
	///     assert_eq!(
	///         config.check_config(
	///             vec![ConfigOptionName::MaxSizeBytes, ConfigOptionName::MaxLoadFactor],
	///             vec![ConfigOptionName::MaxSizeBytes]
	///         ),
	///         Ok(())
	///     );
	///
	///     // This configuration is invalid because AutoRotate was not specified
	///     assert_eq!(
	///         config.check_config(
	///             vec![ConfigOptionName::MaxSizeBytes, ConfigOptionName::MaxLoadFactor],
	///             vec![ConfigOptionName::MaxSizeBytes, ConfigOptionName::AutoRotate]
	///         ),
	///         Err(err!(ErrKind::Configuration, "AutoRotate was required and not specified"))
	///     );
	///
	///     // This configuration was invalid because MaxLoadFactor was not allowed
	///     assert_eq!(
	///         config.check_config(
	///             vec![ConfigOptionName::MaxSizeBytes],
	///             vec![ConfigOptionName::MaxSizeBytes],
	///         ),
	///         Err(err!(ErrKind::Configuration, "MaxLoadFactor is not allowed"))
	///     );
	///
	///     let config = config!(MaxSizeBytes(1_000_000), MaxLoadFactor(0.3), MaxLoadFactor(0.3));
	///
	///     // This configuration had duplicate values
	///     assert_eq!(
	///         config.check_config(
	///             vec![ConfigOptionName::MaxSizeBytes, ConfigOptionName::MaxLoadFactor],
	///             vec![ConfigOptionName::MaxSizeBytes],
	///         ),
	///         Err(err!(ErrKind::Configuration, "MaxLoadFactor was specified more than once"))
	///     );
	///  
	///     Ok(())
	/// }
	///```
	fn check_config(
		&self,
		allowed: Vec<ConfigOptionName>,
		required: Vec<ConfigOptionName>,
	) -> Result<(), Error>;

	fn check_config_duplicates(
		&self,
		allowed: Vec<ConfigOptionName>,
		required: Vec<ConfigOptionName>,
		allow_duplicates: Vec<ConfigOptionName>,
	) -> Result<(), Error>;

	fn get_multi(&self, name: &ConfigOptionName) -> Vec<ConfigOption>;
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
	MaxEntries,
	MaxLoadFactor,
	SlabSize,
	SlabCount,
	MinSize,
	MaxSize,
	SyncChannelSize,
	GlobalSlabAllocator,
	Start,
	End,
	MatchId,
	Regex,
	IsCaseSensitive,
	IsTerminationPattern,
	IsMultiLine,
	PatternId,
	IsHashtable,
	IsHashset,
	IsList,
	TerminationLength,
	MaxWildCardLength,
	IsSync,
	EvhThreads,
	EvhHouseKeeperFrequencyMillis,
	EvhStatsUpdateMillis,
	EvhTimeout,
	EvhReadSlabSize,
	EvhReadSlabCount,
	HttpContentFile,
	HttpContentData,
	HttpAccept,
	HttpHeader,
	HttpTimeoutMillis,
	HttpMeth,
	HttpVers,
	HttpConnection,
	HttpRequestUri,
	HttpRequestUrl,
	HttpUserAgent,
	Port,
	Host,
	BaseDir,
	ServerName,
	DebugNoChunks,
	Debug,
	DebugLargeSlabCount,
}

/// Configuration options used throughout BMW via macro.
#[derive(PartialEq, Clone, Debug)]
pub enum ConfigOption {
	MaxSizeBytes(u64),
	MaxAgeMillis(u64),
	DisplayColors(bool),
	DisplayStdout(bool),
	DisplayTimestamp(bool),
	DisplayLogLevel(bool),
	DisplayLineNum(bool),
	DisplayMillis(bool),
	LogFilePath(Option<PathBuf>),
	AutoRotate(bool),
	DisplayBackTrace(bool),
	LineNumDataMaxLen(u64),
	DeleteRotation(bool),
	FileHeader(String),
	MaxEntries(usize),
	MaxLoadFactor(f64),
	SlabSize(usize),
	SlabCount(usize),
	MinSize(usize),
	MaxSize(usize),
	SyncChannelSize(usize),
	GlobalSlabAllocator(bool),
	Start(usize),
	End(usize),
	MatchId(usize),
	Regex(String),
	IsCaseSensitive(bool),
	IsTerminationPattern(bool),
	IsMultiLine(bool),
	PatternId(usize),
	IsHashtable(bool),
	IsHashset(bool),
	IsList(bool),
	TerminationLength(usize),
	MaxWildCardLength(usize),
	IsSync(bool),
	EvhThreads(usize),
	EvhHouseKeeperFrequencyMillis(usize),
	EvhStatsUpdateMillis(usize),
	EvhTimeout(u16),
	EvhReadSlabSize(usize),
	EvhReadSlabCount(usize),
	HttpContentFile(PathBuf),
	HttpContentData(Vec<u8>),
	HttpAccept(String),
	HttpHeader((String, String)),
	HttpTimeoutMillis(u64),
	HttpMeth(String),
	HttpVers(String),
	HttpConnection(String),
	HttpRequestUri(String),
	HttpRequestUrl(String),
	HttpUserAgent(String),
	Port(u16),
	Host(String),
	BaseDir(String),
	ServerName(String),
	DebugNoChunks(bool),
	Debug(bool),
	DebugLargeSlabCount(bool),
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
