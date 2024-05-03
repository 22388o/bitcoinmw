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
	use TestErrorKind::*;

	/// Kinds of errors used in testing
	#[ErrorKind]
	enum TestErrorKind {
		TestAbc,
		TestDef,
		/// ghi error kind
		TestGhi,
		IllegalState,
	}

	fn ret_err() -> Result<(), Error> {
		ret_err!(TestAbc, "11234test abc");
	}

	fn ret_err2() -> Result<(), Error> {
		err!(TestGhi, "12345")
	}

	fn ret_err3() -> Result<(), Error> {
		match ret_err2() {
			Ok(_) => Ok(()),
			Err(e) => err!(IllegalState, "ret_err2 generated error: {}", e),
		}
	}

	#[test]
	fn test_errorkind() -> Result<(), Error> {
		let err1 = ret_err().unwrap_err();
		assert_eq!(
			err1.kind().to_string(),
			"test abc: 11234test abc".to_string()
		);
		let err2 = ret_err2().unwrap_err();
		assert_eq!(err2.kind().to_string(), "ghi error kind: 12345".to_string());

		let err3 = ret_err3().unwrap_err();
		println!("err3='{}'", err3);
		assert_eq!(err3.kind(), kind!(IllegalState));

		Ok(())
	}
}
