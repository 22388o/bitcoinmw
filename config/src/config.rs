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

use crate::Config;
use crate::ConfigOption::*;
use bmw_err::*;

impl Config {
	pub fn check_no_duplicates(&self) -> Result<(), Error> {
		let mut display_line_num_specified = false;
		for v in &self.configs {
			match v {
				DisplayLineNum(_) => {
					if display_line_num_specified {
						return Err(err!(
							ErrKind::Configuration,
							"DisplayLineNum was specified more than once"
						));
					}
					display_line_num_specified = true;
				}
				DisplayColors(_) => {}
				_ => {}
			}
		}
		Ok(())
	}
}
