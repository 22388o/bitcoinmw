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

use bmw_deps::failure;

mod class;
mod config;
mod document;
mod errorkind;
mod impls;
mod object;
mod ser;
mod types;
mod utils;

extern crate proc_macro;
use crate::class::do_derive_class;
use crate::config::do_derive_configurable;
use crate::document::do_derive_document;
use crate::errorkind::do_derive_errorkind;
use crate::object::do_derive_object;
use crate::ser::do_derive_serialize;
use bmw_deps::proc_macro_error::proc_macro_error;
use proc_macro::TokenStream;

// derive proc macros

#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serialize(strm: TokenStream) -> TokenStream {
	do_derive_serialize(strm)
}

#[proc_macro_derive(Configurable, attributes(required, options))]
#[cfg(not(tarpaulin_include))]
pub fn derive_configurable(strm: TokenStream) -> TokenStream {
	do_derive_configurable(strm)
}

// attribute proc macros

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn errorkind(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_errorkind(attr, item)
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[proc_macro_error]
pub fn object(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_object(attr, item)
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[proc_macro_error]
pub fn class(attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_class(attr, item)
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn public(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn protected(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn no_send(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn no_sync(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn doc_hidden(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn method(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn field(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn config(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn builder(_attr: TokenStream, item: TokenStream) -> TokenStream {
	item
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn document(_attr: TokenStream, item: TokenStream) -> TokenStream {
	do_derive_document(item)
}

#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
pub fn add_doc(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// add doc doesn't actually change anything, it's just a marker used by the document attribute
	// which modifies the TokenStream. So, we just return the input token stream unchanged.
	item
}
