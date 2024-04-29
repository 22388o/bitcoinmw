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

use bmw_core::*;

#[class {
        protected viewx, viewx_send_box;
        /// top trait public
        /// top trait 2
        public viewx_send_box, viewx_box, viewy;
        const c: Vec<(String, String)> = vec![];
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
        /// @error bmw_base::BaseErrorKind::Parse if a parse error occurs
        /// @error bmw_base::BaseErrorKind::IllegalState if an illegal state occurs
        /// @error bmw_base::BaseErrorKind::IO if an i/o error occurs
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
