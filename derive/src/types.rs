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

use bmw_deps::failure::Fail;
use proc_macro::{Span, TokenStream};
use std::collections::HashMap;

pub(crate) struct SerMacroState {
	pub(crate) ret_read: String,
	pub(crate) ret_write: String,
	pub(crate) expect_name: bool,
	pub(crate) name: String,
	pub(crate) field_names: Vec<String>,
	pub(crate) is_enum: bool,
}

pub(crate) struct ConfMacroState {
	pub(crate) count: usize,
	pub(crate) name: Option<String>,
	pub(crate) u8_configs: Vec<(String, bool, bool)>,
	pub(crate) u16_configs: Vec<(String, bool, bool)>,
	pub(crate) u32_configs: Vec<(String, bool, bool)>,
	pub(crate) u64_configs: Vec<(String, bool, bool)>,
	pub(crate) u128_configs: Vec<(String, bool, bool)>,
	pub(crate) usize_configs: Vec<(String, bool, bool)>,
	pub(crate) string_configs: Vec<(String, bool, bool)>,
	pub(crate) bool_configs: Vec<(String, bool, bool)>,
	pub(crate) string_tuple_configs: Vec<(String, bool, bool)>,
	pub(crate) options_name: Option<String>,
}

pub(crate) struct ObjectMacroState {
	pub(crate) ret: TokenStream,
	pub(crate) span: Option<Span>,
	pub(crate) name: Option<String>,
	pub(crate) expect_impl: bool,
	pub(crate) expect_name: bool,
	pub(crate) expect_tag: bool,
	pub(crate) in_builder: bool,
	pub(crate) expect_builder_name: bool,
	pub(crate) in_method: bool,

	pub(crate) cur_method: Option<Method>,
	pub(crate) expect_method_name: bool,

	pub(crate) in_field: bool,
	pub(crate) in_config: bool,

	pub(crate) const_list: Vec<ObjectConst>,
	pub(crate) field_list: Vec<ObjectField>,
	pub(crate) views: HashMap<String, Vec<Method>>,
	pub(crate) builder: Option<String>,
}

pub(crate) enum ConstType {
	U8(u8),
	U16(u16),
	U32(u32),
	U64(u64),
	U128(u128),
	Usize(usize),
	Bool(bool),
	Tuple((String, String)),
	ConfString(String),
	VecU8(Vec<u8>),
	VecU16(Vec<u16>),
	VecU32(Vec<u32>),
	VecU64(Vec<u64>),
	VecU128(Vec<u128>),
	VecUsize(Vec<usize>),
	VecBool(Vec<bool>),
	VecTuple(Vec<(String, String)>),
	VecString(Vec<String>),
}

pub(crate) struct ObjectConst {
	pub(crate) name: String,
	pub(crate) default: ConstType,
}

#[derive(Clone, Debug)]
pub(crate) struct Method {
	pub(crate) name: String,
	pub(crate) signature: String,
	pub(crate) views: Vec<String>,
	pub(crate) param_string: String,
}

pub(crate) struct ObjectField {
	pub(crate) name: String,
	pub(crate) otype: String,
}

pub(crate) struct DocMacroState {
	pub(crate) ret: TokenStream,
	pub(crate) in_add_doc: bool,
	pub(crate) in_punct: bool,
	pub(crate) ret_pre: TokenStream,
	pub(crate) ret_post: TokenStream,
	pub(crate) found_doc_point: bool,
	pub(crate) insert: bool,
	pub(crate) prev_single_tick: bool,
	pub(crate) prev_token: String,
	pub(crate) trait_name: Option<String>,
	pub(crate) add_docs: Vec<String>,
	pub(crate) in_fn_signature: bool,
	pub(crate) fn_str: String,
}

pub(crate) struct DocItem {
	pub(crate) trait_name: Option<String>,
	pub(crate) input_hash: HashMap<String, Input>,
	pub(crate) error_str: String,
	pub(crate) return_str: String,
	pub(crate) see_str: String,
	pub(crate) return_type_str: String,
}

#[derive(Eq)]
pub(crate) struct Input {
	pub(crate) name: String,
	pub(crate) text: String,
	pub(crate) type_str: String,
	pub(crate) is_ref: bool,
	pub(crate) is_mut: bool,
	pub(crate) seqno: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) enum TokenType {
	Ident,
	GroupItem,
	Literal,
	Punct,
}

#[derive(Debug, Fail)]
pub(crate) enum DeriveErrorKind {
	/// Log error
	#[fail(display = "log error: {}", _0)]
	Log(String),
}
