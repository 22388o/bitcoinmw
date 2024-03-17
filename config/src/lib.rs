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

//! # The BMW Configuration crate
//! The Configuration crate is used by other crates in the BMW repo to build and check configurations.
//! Configurations should generally be built using the [`crate::config!`] macro and the
//! [`crate::Config.check_config`] function should be used to confirm the resulting configuration
//! has only allowed values, has all required values, and has no duplicates.
//!
//! # Examples
//!
//!```
//! use bmw_err::*;
//! use bmw_conf::*;
//!
//! fn main() -> Result<(), Error> {
//!     // create a simple config
//!     let config = config!(
//!         AutoRotate(true),
//!         MaxAgeMillis(60 * 60 * 1_000),
//!         FileHeader("myheader".to_string())
//!     );
//!
//!     let res = config.check_config(
//!         vec![
//!             ConfigOptionName::AutoRotate,
//!             ConfigOptionName::MaxAgeMillis,
//!             ConfigOptionName::FileHeader
//!         ],
//!         vec![ConfigOptionName::AutoRotate]
//!     );
//!
//!     // this configuration is ok because all fields specified are allowed (AutoRotate,
//!     // FileHeader, and MaxAgeMillis) and all required fields (AutoRotate) are specified.
//!     assert!(res.is_ok());
//!
//!     // create an invalid config
//!     let config = config!(MaxAgeMillis(60 * 60 * 1_000), FileHeader("myheader".to_string()));
//!
//!     let res = config.check_config(
//!         vec![
//!             ConfigOptionName::AutoRotate,
//!             ConfigOptionName::MaxAgeMillis,
//!             ConfigOptionName::FileHeader],
//!         vec![ConfigOptionName::AutoRotate]
//!     );
//!
//!     // this configuration is invalid because AutoRotate is not specified.
//!     assert!(res.is_err());
//!
//!     Ok(())
//! }
//!
//!```

mod builder;
mod config;
mod macros;
mod test;
mod types;

pub use crate::types::{Config, ConfigBuilder, ConfigOption, ConfigOptionName};
