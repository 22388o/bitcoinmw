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
	use crate as bmw_test;
	use crate::{test_info, TestInfo};
	use bmw_err::Error;

	#[test]
	fn test_test_info_macro() -> Result<(), Error> {
		let test_info = test_info!()?;
		assert!(test_info.port() >= 9000);
		assert!(test_info.directory().ends_with("bmw"));
		Ok(())
	}

	#[test]
	fn test_other_test() -> Result<(), Error> {
		let test_info = test_info!()?;
		assert!(test_info.port() >= 9000);
		assert!(test_info.directory().ends_with("bmw"));
		Ok(())
	}
}
