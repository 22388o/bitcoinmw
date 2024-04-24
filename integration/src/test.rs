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
use bmw_derive::*;

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

#[class {
        const y: usize = 1;
        const z: u8 = 10;
        var x: i32;
        var v: usize;

        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self { x: -100, v: *const_values.get_y() })
        }

        [dog, test2]
        fn bark(&mut self) -> Result<String, Error> {
                *self.get_mut_x() += 1;
                println!("x={},y={}", self.get_x(), self.get_y());
                Ok("woof".to_string())
        }

        [cat, test2]
        fn meow(&mut self, v1: usize, v2: bool) -> Result<String, Error> {
                self.other();
                Ok("meow".to_string())
        }

        fn other(&self) {
                let value: u8 = *self.get_z();
                println!("v+1={}", value+1);
        }
}]
impl Animal3 {}

#[cfg(test)]
mod test {
	use crate::test::*;

	/*
	#[ErrorKind]
	enum IntErrorKind {
		/// integration error
		Integration,
		/// test error
		Test123,
		Abc123,
	}
		*/

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

	#[test]
	fn test_class() -> Result<(), Error> {
		let mut dog = dog_box!(Y(100))?;
		dog.bark()?;

		let mut dog = dog_send_box!(Y(80))?;
		dog.bark()?;

		let mut dog = dog_send!(Y(60))?;
		dog.bark()?;

		let mut dog = dog!(Y(40))?;
		dog.bark()?;

		let mut dog = dog_sync_box!(Y(20))?;
		dog.bark()?;

		let mut dog = dog_sync!(Y(0))?;
		dog.bark()?;

		Ok(())
	}
}
