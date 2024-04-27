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

pub(crate) struct SerMacroState {
	pub(crate) ret_read: String,
	pub(crate) ret_write: String,
	pub(crate) expect_name: bool,
	pub(crate) name: String,
	pub(crate) field_names: Vec<String>,
	pub(crate) is_enum: bool,
}

#[derive(Debug, Fail)]
pub(crate) enum DeriveErrorKind {
	#[fail(display = "log error: {}", _0)]
	Log(String),
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
