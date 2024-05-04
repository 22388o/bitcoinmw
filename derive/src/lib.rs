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

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

// errorkind section
mod errorkind;
use crate::errorkind::do_derive_errorkind;

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[allow(non_snake_case)]
pub fn ErrorKind(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_errorkind(attr, item)
}

// config section
mod config;
use crate::config::do_derive_configurable;

#[proc_macro_derive(Configurable, attributes(required))]
#[proc_macro_error]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm)
}
