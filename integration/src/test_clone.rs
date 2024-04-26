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

#[class{
		public testview, otherview;
		var test: usize;

		/// @module bmw_int::test_clone
		fn builder(&const_values) -> Result<Self, Error> {
				Ok(Self { test: 0 })
		}

		[testview, otherview]
		fn set_test(&mut self, value: usize) {
			(*self.get_mut_test()) = value;
		}

		[testview, otherview]
		fn test1(&self) -> Result<(), Error> {
				Ok(())
		}

                [testview]
                fn as_otherview(&mut self) -> Result<Box<dyn Otherview + '_>, Error> {
                        let other: Box<dyn Otherview> = Box::new(self);
                        Ok(other)
                }

                [otherview]
                fn getv(&self) -> usize {
                        10
                }

}]
impl TestImpl {}

#[class{
        var test: usize;
        public display_box;
        clone display2;

        /// @module bmw_int::test_clone
        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self { test: 1 })
        }

        [display2, test]
        fn display2(&self) {
                println!("test={}", self.get_test());
        }
}]
impl Clonable {}

struct MyStruct {}

#[class {
        public nonclone;
        var test: MyStruct;

        /// @module bmw_int::test_clone
        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self { test: MyStruct {} })
        }

        [nonclone]
        fn do_stuff(&self) -> Result<(), Error> {
                println!("do_stuff");
                Ok(())
        }
}]
impl NonClonable {}

#[cfg(test)]
mod test {
	use crate::test_clone::*;

	#[test]
	fn test_non_clonable() -> Result<(), Error> {
		let x = nonclone!()?;
		x.do_stuff()?;
		Ok(())
	}

	#[test]
	fn test_clone() -> Result<(), Error> {
		let display = display2_box!()?;
		let display2 = display.clone();
		display2.display2();
		Ok(())
	}

	#[test]
	fn test_timpl() -> Result<(), Error> {
		let mut testview = testview!()?;
		testview.test1()?;
		let otherview = otherview!()?;
		otherview.test1()?;

		let x = testview.as_otherview()?;

		assert_eq!(x.getv(), 10);
		Ok(())
	}
}
