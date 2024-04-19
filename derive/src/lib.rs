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

//! This crate is a convenience crate. It just re-exports both the [`bmw_core`] and [`bmw_impl`]
//! crates. Then the user can just do a "use bmw_derive::*;" and use the derive macros which will
//! have all the needed items from core as well.

//! # The BitcoinMW Derive crate
//! This crate is a proc_macro crate and it includes the Serializable macro and the Configurable
//! macro.
//! The serializable macro implements the bmw_ser::Serializable trait for any struct or enum.
//! The Configurable macro makes a struct configurable via the [`crate::config`] macro.
//!
//! # Serializable example
//!
//!```
//! use bmw_derive::Serializable;
//! use bmw_err::Error;
//!
//! #[derive(Serializable)]
//! struct MyStruct {
//!     id: u64,
//!     is_member: bool,
//!     name: String,
//! }
//!
//! #[derive(Serializable)]
//! enum MyEnum {
//!     Type1(String),
//!     Type2(u64),
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let _s1 = MyStruct {
//!         id: 1234,
//!         is_member: true,
//!         name: "Hagrid".to_string(),
//!     };
//!
//!     let _s2 = MyEnum::Type1("something".to_string());
//!
//!     Ok(())
//! }
//!
//!```  
//!
//! This macro is used in the bmw_util and other crates within BitcoinMW. For additional examples, see the bmw_util documentation.
//!
//! # Generics
//! It's important to note that generics are not currently supported and will result in an error. If you need
//! generics, currenly you must build your own bmw_ser::Serializable implementation.
//!
//! # Configurable
//! The configurable derive macro is used to allow for fast and easy configuration of specified structs.
//! Configurations can be of the following types: [`u8`], [`u16`], [`u32`], [`u64`], [`u128`],
//! [`usize`], [`bool`], [`std::string::String`], `(String, String)`. [`std::vec::Vec`] of any of
//! these types are also configurable. See the detailed example below for further details on how to
//! use this derive macro.
//!
//! # Configurable example
//!```
//! use bmw_derive::{config, Configurable}; // derive proc-macro and config macro
//!
//! // define your struct and derive 'Configurable'.
//! // the supported types are bool, u8, u16, u32, u64, u128, usize, bool,
//! // String, and (String, String). Vec of each of these types also are supported.
//! // (e.g. 'headers' below.)
//! #[derive(Configurable)]
//! #[options = "ConfigOptions"] // The name of the Options enum for this struct. (default
//!                              // {StructName}Options. Note that this will be a 'pub' enum
//!                              // so it can be exported and used in other crates, etc.
//! struct MyStruct {
//!     // the 'required' helper attribute indicates this field must always be specified in the
//!     // config macro.
//!     #[required] threads: usize,
//!     timeout: u128,
//!     stats_frequency: u64,
//!     log_file_location: String,
//!     headers: Vec<(String, String)>,
//! }
//!
//! // implement the Default trait for the struct and the proc-macro does the rest.
//! // note that if Vecs have elements in them, configuring data will not delete them, the new
//! // elements will just be appended to the vec.
//! impl Default for MyStruct {
//!     fn default() -> Self {
//!         Self {
//!             threads: 1,
//!             timeout: 10_000,
//!             stats_frequency: 5_000,
//!             log_file_location: "~/.bmw/mylog.log".to_string(),
//!             headers: vec![],
//!         }
//!     }
//! }   
//!     
//! fn main() {
//!     // call the config macro to build your struct with the specified values that
//!     // overwrite the default values. Note that the names of the configurations are
//!     // the names in the struct converted to 'Pascal' case. See the 'convert_case'
//!     // crate for details on this.
//!     let my_config = config!(
//!         MyStruct, // name of your struct
//!         ConfigOptions, // name of the options enum. This would be called
//!         // {struct_name}Options if the #[options = "ConfigOptions"] attribute was not
//!         // specified above.
//!         vec![
//!             Threads(6), // for non-vec, a single value is specified
//!             Headers(("Content-Type", "text/html")), // vec's may have multiple entries
//!             Headers(("Connection", "keep-alive")),
//!         ]
//!     );
//!
//!     // check for errors. Duplicates for non-vecs are considered errors
//!     // also if a 'required' field is missing an error will be returned.
//!     let my_config = match my_config {
//!         Ok(my_config) => my_config,
//!         Err(e) => {
//!             panic!("config returned err: {}", e);
//!         }
//!     };
//!
//!     assert_eq!(my_config.threads, 6); // specified
//!     assert_eq!(my_config.timeout, 10_000); // default
//!     assert_eq!(my_config.stats_frequency, 5_000); // default
//!     assert_eq!(my_config.log_file_location, "~/.bmw/mylog.log".to_string()); // default
//!     assert_eq!(
//!         my_config.headers,
//!         vec![
//!             ("Content-Type".to_string(), "text/html".to_string()),
//!             ("Connection".to_string(), "keep-alive".to_string())
//!         ]
//!     ); // specified with two values
//!
//!     // some errors
//!
//!     // error because required field (Threads) is not specified.
//!     assert!(config!(MyStruct, ConfigOptions, vec![Timeout(5_000)]).is_err());
//!
//!     // error because Threads is specified twice.
//!     assert!(config!(MyStruct, ConfigOptions, vec![Threads(10), Threads(20)]).is_err());
//! }
//!
//!```

mod macros;
mod test;

pub use bmw_core::*;
pub use bmw_impl::*;
