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

#[macro_export]
macro_rules! config {
	( $configurable:ident, $enum_name:ident, $vec:expr ) => {{
		use bmw_err::*;
		use std::collections::HashSet;
		use $enum_name::*;

		let mut ret = $configurable::new();

		let mut name_set: HashSet<String> = HashSet::new();
		let mut err = None;

		for cfg in $vec {
			let name = cfg.name();
			if name_set.contains(name.clone()) && !ret.allow_dupes().contains(name.clone()) {
				let text = format!("config option ({}) was specified more than once", name);
				err = Some(Err(err!(ErrKind::Configuration, text)));
			}
			name_set.insert(name.to_string());
			match cfg.value_u8() {
				Some(value) => ret.set_u8(name, value),
				None => {}
			}
			match cfg.value_u16() {
				Some(value) => ret.set_u16(name, value),
				None => {}
			}
			match cfg.value_u32() {
				Some(value) => ret.set_u32(name, value),
				None => {}
			}
			match cfg.value_u64() {
				Some(value) => ret.set_u64(name, value),
				None => {}
			}
			match cfg.value_u128() {
				Some(value) => ret.set_u128(name, value),
				None => {}
			}
			match cfg.value_usize() {
				Some(value) => ret.set_usize(name, value),
				None => {}
			}
			match cfg.value_string() {
				Some(value) => ret.set_string(name, value),
				None => {}
			}
			match cfg.value_bool() {
				Some(value) => ret.set_bool(name, value),
				None => {}
			}
			match cfg.value_string_tuple() {
				Some(value) => ret.set_string_tuple(name, value),
				None => {}
			}
		}

		for r in $configurable::required() {
			if !name_set.contains(&r) {
				let text = format!("required option ({}) was not specified", r);
				err = Some(Err(err!(ErrKind::Configuration, text)));
			}
		}

		match err {
			Some(e) => e,
			None => Ok(ret),
		}
	}};
}
