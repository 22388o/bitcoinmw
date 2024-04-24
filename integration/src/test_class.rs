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

#[class {
        public dog, dog_box, dog_send, dog_sync, dog_send_box, dog_sync_box;
        const y: usize = 1;
        const z: u8 = 10;
        const server_name: String = "my_server".to_string();
        var x: i32;
        var v: usize;

        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self { x: -100, v: *const_values.get_y() })
        }

        [dog, test2]
        fn bark(&mut self) -> Result<String, Error> {
                self.do_cool_stuff()?;
                *self.get_mut_x() += 1;
                println!("x={},y={}", self.get_x(), self.get_y());
                Ok("woof".to_string())
        }

        [cat, test2]
        fn meow(&mut self, v1: usize, v2: bool) -> Result<String, Error> {
                self.other();
                Ok("meow".to_string())
        }

        [dog]
        fn server_name(&self) -> Result<String, Error> {
                Ok(self.get_server_name().clone())
        }

        fn other(&self) {
                let value: u8 = *self.get_z();
                println!("v+1={}", value+1);
        }
}]
impl Animal {}

#[cfg(test)]
mod test {
	use crate::test_class::*;

	#[test]
	fn test_class_types() -> Result<(), Error> {
		let mut dog = dog_box!(Y(100))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		let mut dog = dog_send_box!(Y(80))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		let mut dog = dog_send!(Y(60))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		let mut dog = dog!(Y(40))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		let mut dog = dog_sync_box!(Y(20))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		let mut dog = dog_sync!(Y(0))?;
		assert_eq!(dog.bark()?, "woof".to_string());

		Ok(())
	}

	#[test]
	fn test_string() -> Result<(), Error> {
		let dog = dog!()?;
		assert_eq!(dog.server_name()?, "my_server".to_string());

		let dog = dog!(ServerName("bitcoinmw"))?;
		assert_eq!(dog.server_name()?, "bitcoinmw".to_string());

		Ok(())
	}
}
