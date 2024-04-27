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

use bmw_base::*;
use bmw_derive2::*;

// define an error kind
#[ErrorKind]
enum IntErrorKind {
	/// integration error 7
	Integration,
	/// test error
	Test123,
	Abc123,
}

// define a second error kind
#[ErrorKind]
enum IntErrorKind2 {
	/// integration error
	Integration,
	/// test error
	Test123,
	Abc123,
}

#[cfg(test)]
mod test {
	use crate::test_error::*;

	fn ret_err() -> Result<(), Error> {
		err!(IntErrorKind::Integration, "this is a test {}", 1)
	}

	fn ret_err2() -> Result<(), Error> {
		err!(IntErrorKind2::Abc123, "some msg")
	}

	fn ret_err3() -> Result<(), Error> {
		let ret_err = ret_err2().unwrap_err();
		err!(
			IntErrorKind2::Test123,
			"generated error: {}",
			ret_err.kind()
		)
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		// ret_err returns an error
		assert!(ret_err().is_err());
		// unwrap it
		let err: Error = ret_err().unwrap_err();
		// get its kind
		let kind = err.kind();
		// check that it's equal to one we construct
		assert_eq!(kind, &kind!(IntErrorKind::Integration, "this is a test 1"));
		println!("kind={}", kind);
		// check that comment gets integrated properly
		assert_eq!(kind.to_string(), "integration error 7: this is a test 1");

		// ret_err2 returns an error
		assert!(ret_err2().is_err());
		// unwrap it
		let err: Error = ret_err2().unwrap_err();
		// get its kind
		let kind = err.kind();
		// check that it's equal to one we construct
		assert_eq!(kind, &kind!(IntErrorKind2::Abc123, "some msg"));
		println!("kind={}", kind);
		// check that comment gets integrated properly
		assert_eq!(kind.to_string(), "Abc123: some msg");

		// test error stacking
		let err = ret_err3().unwrap_err();
		println!("kind='{}'", err.kind().to_string());
		//assert_eq!(err.kind().to_string(), "Abc123: some msg",);
		assert_eq!(
			err.kind().to_string(),
			"test error: generated error: Abc123: some msg"
		);

		Ok(())
	}
}
