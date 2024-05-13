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

//! # The BitcoinMW base crate
//! The base crate defines several key traits such as, [`Configurable`], [`ConfigurableOptions`], [`ErrorKind`], and [`Serializable`], whose implementations
//! can be automatically generated by the proc macros in the derive crate.
//! For detailed examples, refer to the derive crate. Additionally, this crate includes several other
//! data types required by the derive crate. It also encompasses [`crate::Error`] and various core error-related
//! structures, alongside essential macros utilized in BitcoinMW. The primary objective of this crate is to
//! house all necessary components for implementing the derive proc macros utilized within BitcoinMW. This crate
//! should not be directly used. Instead `bmw_core`, which re-exports both this crate and the
//! derive crate, should be used.

mod config;
mod error;
mod macros;
mod ser;
mod utils;

pub use config::{Configurable, ConfigurableOptions, Passthrough, PassthroughValue};
pub use error::{CoreErrorKind, Error, ErrorKind};
pub use ser::{deserialize, serialize, BinReader, BinWriter, Reader, Serializable, Writer};
pub use utils::is_recursive;
