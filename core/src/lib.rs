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

//! The core crate spcifies the [`crate::Configurable`] and [`crate::Serializable`] trait whose impls can be
//! auto-generated by the proc macro `Configurable` and `Serializable` respectively. See more detailed examles in
//! the `derive` crate. Some other data types needed in the derive crate are included here as
//! well.

mod functions;
mod impls;
mod public;
mod ser;
mod test;

pub use crate::public::*;
