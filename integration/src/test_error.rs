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

use crate::test_class::Animal;
use bmw_base::*;
use bmw_derive::*;

impl Animal {
	pub(crate) fn do_cool_stuff(&self) -> Result<(), Error> {
		println!("in do cool stuff: {}", self.get_y());
		Ok(())
	}
}

#[ErrorKind]
enum IntErrorKind {
	/// integration error
	Integration,
	/// test error
	Test123,
	Abc123,
}

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

	#[test]
	fn test_error() -> Result<(), Error> {
		assert!(ret_err().is_err());

		let err: Error = ret_err().unwrap_err();
		let kind = err.kind();

		assert_eq!(kind, &kind!(IntErrorKind::Integration, "this is a test 1"));

		Ok(())
	}
}
