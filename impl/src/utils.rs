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

use bmw_deps::substring::Substring;

pub(crate) fn trim_brackets(s: &String) -> String {
	let ret = s.trim();
	let ret = match ret.find("[") {
		Some(pos) => {
			if pos == 0 {
				ret.substring(1, ret.len()).to_string()
			} else {
				ret.to_string()
			}
		}
		None => ret.to_string(),
	};
	let ret = match ret.rfind("]") {
		Some(pos) => {
			let last = ret.len().saturating_sub(1);
			if pos == last {
				ret.substring(0, last).to_string()
			} else {
				ret
			}
		}
		None => ret,
	};
	ret
}

pub(crate) fn trim_braces(s: &String) -> String {
	let ret = s.trim();
	let ret = match ret.find("{") {
		Some(pos) => {
			if pos == 0 {
				ret.substring(1, ret.len()).to_string()
			} else {
				ret.to_string()
			}
		}
		None => ret.to_string(),
	};
	let ret = match ret.rfind("}") {
		Some(pos) => {
			let last = ret.len().saturating_sub(1);
			if pos == last {
				ret.substring(0, last).to_string()
			} else {
				ret
			}
		}
		None => ret,
	};
	ret
}
