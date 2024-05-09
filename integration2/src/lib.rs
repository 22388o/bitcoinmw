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
	use bmw_core::*;
	use bmw_int::test_class::*;
	use bmw_int::{http_server, instance};

	#[test]
	fn test_instance() -> Result<(), Error> {
		let http = http_server!(
			Threads(100),
			Instance(instance!(Port(1113))?),
			Instance(instance!(Port(1233), Address("0.0.0.0"))?),
			Instance(instance!()?),
		)?;
		http.show();
		Ok(())
	}
}
