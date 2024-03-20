// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # The BMW Serialization crate
//! This crate includes the [`crate::Serializable`] trait, the [`crate::Reader`] trait and the
//! [`crate::Writer`] trait. They are separated from the bmw_util crate so that the util crate
//! does not have to be a dependency of bmw_derive and can therefore use the Serializable
//! proc_macro that is included in that crate. The Serializable trait is the key to several of the
//! data structures in the bmw_util crate. It allows a specific way for data to be serialized so
//! that it can be stored in various forms. The Reader and Writer traits are abstractions
//! for reading and writing serializable data structures. The [`crate::Serializable`] macro is implemented for
//! several data structures in this crate as well.
//! # Generics
//! It's important to note that generics are not currently supported and will result in an error. If you need
//! generics, currenly you must build your own Serializable implementation.

mod ser;
mod test;
mod types;

pub use crate::types::{BinReader, BinWriter, Reader, Serializable, Writer};

pub use crate::ser::{deserialize, serialize};
