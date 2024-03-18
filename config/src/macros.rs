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

/// The config macro allows for a configuration to be specified and checked conveniently. This
/// macro is used throughout BMW.
///
/// # Examples
///
///```
/// use bmw_conf::*;
/// use bmw_err::*;
///
///
/// // create a config using the macro and check it
/// fn main() -> Result<(), Error> {
///     // create a simple config
///     let config = config!(FileHeader("test".to_string()), DeleteRotation(false));
///
///     // check it
///     let res = config.check_config(
///         vec![
///             ConfigOptionName::FileHeader,
///             ConfigOptionName::DeleteRotation
///         ],
///         vec![ConfigOptionName::DeleteRotation]
///     );
///
///     // this configuration is ok because both FileHeader and DeleteRotation are allowed
///     // and the only required configuration 'DeleteRotation' is specified
///     assert!(res.is_ok());
///     Ok(())
/// }
///```
///
#[macro_export]
macro_rules! config {
	( $( $config:expr ),* ) => {{
                use bmw_conf::{ConfigBuilder, ConfigOption, ConfigOption::*};
                let mut config_values: Vec<ConfigOption> = vec![];
                $(
                        config_values.push($config);
                )*

                ConfigBuilder::build_config(config_values)
        }};
}
