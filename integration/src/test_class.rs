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

//! The test_class module
use bmw_core::*;

#[class {
        module "bmw_int::test_class";
        protected viewx, viewx_send_box;
        /// top trait public
        /// top trait 2
        public viewx_send_box, viewx_box as boxy_box, viewy, viewy_sync_box as a9;
        /// a nice vec of (String, String).
        const c: Vec<(String, String)> = vec![("abc".to_string(), "def".to_string())];
        /// a nice usize value
        const threads: usize = 6;


        const my_value1: u8 = 1;
        const my_value2: u16 = 2;
        const my_value3: u32 = 3;
        const my_value4: u64 = 4;
        const my_value5: u128 = 5;
        const my_value6: bool = false;
        const my_value7: String =  "".to_string();
        const my_value8: (String, String) = ("123".to_string(), "abc".to_string());
        const my_value9: Vec<u32> = vec![1,2];

        /// a very nice vec of strings
        /// second comment
        const p2: Vec<String> = vec![];
        var m: Vec<Option<usize>>;

        /// to trait top
        /// more trait top
        /// next
        /// # some formatting
        /// ok ok ok
        /// here
        ///```
        /// fn main() -> Result<(), Error> {
        ///     Ok(())
        /// }
        ///```
        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self {
                        m: vec![],
                })
        }

        /// this
        /// is a test
        /// of the comments
        /// @param self an immutable ref
        /// @error bmw_base::CoreErrorKind::Parse if a parse error occurs
        /// @error bmw_base::CoreErrorKind::IllegalState if an illegal state occurs
        /// @error bmw_base::CoreErrorKind::IO if an i/o error occurs
        /// @return None is always returned
        /// @see crate::test_class::Viewx::next
        /// @deprecated
        /// # Examples
        ///```
        /// use bmw_base::*;
        ///
        /// fn main() -> Result<(), Error> {
        ///     Ok(())
        /// }
        ///```
        [viewx, viewy]
        fn ok(&self) -> Result<Option<usize>, Error> {
                Ok(None)
        }

        /// next comments here
        /// another next line
        /// viewx next
        /// @param abc the value to be printed
        /// @param self an immutable ref
        [viewx]
        fn next(&self, abc: usize) {
                println!("abc={}", abc);
        }

        fn abc(&self) {
        }

}]
impl Animal2 {}

#[cfg(test)]
mod test {
	use bmw_core::*;

	struct MyStruct {}

	#[class {
                protected view1, view1_send_box;
                public view1_send_box;
                const c: Vec<(String, String)> = vec![];
                const p2: Vec<String> = vec![];
                var m: Vec<Option<usize>>;
                fn builder(&const_values) -> Result<Self, Error> {
                        Ok(Self {
                                m: vec![],
                        })
                }

                /// ok ok ok ok
                [view1]
                fn ok(&self) -> Result<Option<MyStruct>, Error> {
                        Ok(None)
                }

                /// next
                /// next2
                /// next3
                [view1]
                fn next(&self, abc: usize) {
                    println!("abc={}", abc);
                }

                fn abc(&self) {
                }

        }]
	impl Animal {}

	#[test]
	fn test_class() -> Result<(), Error> {
		let _x = MyStruct {};
		Ok(())
	}
}
