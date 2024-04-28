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

	struct MyStruct {}

	#[class {
                protected view3, view1_send_box, view2_box;
                protected view1_sync_box;
                clone abc, def  ;
                const c: Vec<(String, String)> = 1;
                public view1_box, view1_send, view1_sync, view2_send, view3_box, view3;
                const p2: Vec<String> = vec![];
                clone view2;
                var m: Vec<Option<usize>>;
                [abc, def,ghi]
                fn abcdef(&self, abc: Option<Vec<(usize, String, Box<dyn Display + Send + Sync + '_>)>>, v: usize, xxx: (bool, String)) -> usize {
println!("ok");
println!("no");
let x = 1234;
let y = x + 1;
println!("y={}", y);
0
                }


                fn builder(&const_values) -> Result<Self, Error> {
                        Ok(Self {
                                m: vec![],
                        })
                }

                [view1, view2, view3]
                fn test1(&self, abc: Vec<Box<dyn Display>>) -> Result<(), Error> {
                        println!("ok");
                        Ok(())
                }

                [view2, view3]
                fn test2(&mut self) -> Result<Option<(usize, Box<dyn Display + Send + Sync + '_>, Vec<(usize, bool, String)>)>, Error> {
                      Ok(None)
                }

                /// test
                /// another
                ///
                /// ok1
                [view1]
                fn ok() -> Result<Option<MyStruct>, Error> {
                    Ok(None)
                }

                fn abc(&self) {
                }

        }]
	impl<OnRead> Animal<OnRead> where OnRead: FnMut(usize) -> () + Send + 'static + Clone + Sync + Unpin {}

	#[test]
	fn test_class() -> Result<(), Error> {
		let _x = MyStruct {};
		Ok(())
	}
}
