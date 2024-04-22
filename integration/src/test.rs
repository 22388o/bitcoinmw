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
	use bmw_base::*;
	use bmw_deps::failure::Fail;
	use bmw_derive::*;

	#[derive(ErrorKind, Debug, Fail)]
	enum IntErrorKind {
		/// Test the integration of errors
		#[fail(display = "integration error: {}", _0)]
		Integration(String),
	}

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

	#[object]
	impl Animal {
		#[config(y: usize = 1)]
		#[config(z: u8 = 10)]
		#[field(x: i32)]
		#[builder]
		fn builder10(config: AnimalConfig) -> Result<Self, Error> {
			Ok(Self { config, x: -100 })
		}

		#[method(dog, test)]
		fn bark(&mut self) -> Result<String, Error> {
			if self.config.y == 1 {
				Ok("WOOF!".to_string())
			} else {
				Ok("woof!".to_string())
			}
		}

		#[method(dog, cat)]
		fn wag(&self) -> Result<i32, Error> {
			Ok(self.x + 1)
		}

		#[method(test)]
		fn as_dog(&mut self) -> Result<Box<dyn Dog + '_>, Error> {
			let dog: Box<dyn Dog> = Box::new(self);
			Ok(dog)
		}

		fn impl_dog(&mut self) -> Result<impl Dog + '_, Error> {
			Ok(self)
		}

		fn impl_test(&mut self) -> Result<impl Test + '_, Error> {
			Ok(self)
		}
	}

	/*
	#[object{
			Animal {
				const y: usize = 1;
				const z: u8 = 10;
				x: i32;
				fn builder(config: AnimalConfig) -> Result<Self, Error> {
					Ok(Self { config, x: -100 })
				}

				fn bark(&mut self) -> Result<String, Error> {

				}
			}
		}]
	impl {}
		*/

	#[test]
	fn test_object() -> Result<(), Error> {
		let mut dog = dog!(Y(2))?;
		assert_eq!(dog.bark()?, "woof!");
		let mut dog = dog!(Y(1))?;
		assert_eq!(dog.bark()?, "WOOF!");
		let mut dog = dog!(Y(10))?;
		assert_eq!(dog.bark()?, "woof!");
		assert_eq!(dog.wag()?, -99);
		let mut test = test!(Y(20))?;
		assert_eq!(test.bark()?, "woof!");

		let mut test = test!(Y(1))?;
		assert_eq!(test.bark()?, "WOOF!");

		let mut test2 = test.as_dog()?;
		assert_eq!(test2.bark()?, "WOOF!");

		Ok(())
	}
}
