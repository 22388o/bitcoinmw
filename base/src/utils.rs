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

use bmw_deps::backtrace::Backtrace;

pub fn is_recursive() -> bool {
	let backtrace = Backtrace::new();
	let frames = backtrace.frames();
	let mut symbols_vec: Vec<String> = vec![];
	for frame in frames {
		let symbols = frame.symbols();
		for symbol in symbols {
			match symbol.name() {
				Some(name) => {
					let name = format!("{:?}", name);
					let name = name.trim();
					symbols_vec.push(name.to_string());
				}
				None => {}
			}
		}
	}

	let mut found_base = false;
	let mut after_base = None;
	for symbol in symbols_vec {
		if found_base {
			if after_base.is_none() {
				after_base = Some(symbol.clone());
			} else {
				if after_base.clone().unwrap() == symbol {
					return true;
				} else {
					break;
				}
			}
		} else if symbol.find("bmw_base::utils::is_recursive") == Some(0) {
			found_base = true;
		}
	}

	false
}
