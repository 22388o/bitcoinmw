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

#[cfg(test)]
mod test {
	use bmw_derive::*;
	use bmw_err::*;

	#[derive(Configurable)]
	struct ConfigMe {
		v1: usize,
		v2: u8,
	}

	impl Default for ConfigMe {
		fn default() -> Self {
			Self { v1: 0, v2: 1 }
		}
	}

	#[test]
	fn test_config_proc_macro() -> Result<(), Error> {
		Ok(())
	}
}
