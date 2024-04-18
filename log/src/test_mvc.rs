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
	use crate as bmw_log;
	use bmw_conf::{Configurable, InstanceType};
	use bmw_derive::*;
	use bmw_err::*;
	use bmw_log::*;

	info!();

	struct MyStruct {
		config: MyStructConfig,
		view: String,
		count: u128,
		debug_bark: bool,
	}

	//#[entity]
	struct MyData {
		view: String,
		count: u128,
		debug_bark: bool,
	}

	// build trait
	trait MyStructModel {
		fn set_view(&mut self, value: String);
		fn set_count(&self, value: u128);
		fn set_debug_bark(&self, value: bool);
		fn get_view(&self) -> &String;
		fn get_count(&self) -> &u128;
		fn get_debug_bark(&self) -> &bool;
	}

	struct MyStructModelBuilder {}
	impl MyStructBuilder {
		pub fn build_my_struct() -> Box<dyn MyStructModel + Send + Sync> {
			todo!()
		}
	}

	impl Default for MyStructConfig {
		fn default() -> Self {
			Self {
				threads: 1,
				timeout: 1_000,
				server_name: "".to_string(),
				headers: vec![],
				header: ("".to_string(), "".to_string()),
				abc: vec![],
				def: vec![],
			}
		}
	}

	#[mvc(
                views=[
                        pub Cat,
                        pub Dog,
                        pub(crate) Bird,
                        TestMyStruct,
                ],
                macros=[
                        IMPL,
                        BOX,
                        IMPL_SEND,
                        IMPL_SYNC,
                        BOX_SEND,
                        BOX_SYNC,
                ],
		config=[
                        struct MyStructConfig {
				threads: usize,
				timeout: u64,
				server_name: String,
				headers: Vec<(String, String)>,
				header: (String, String),
				abc: Vec<usize>,
				def: Vec<String>,
                        }
		],
	)]

	impl MyStruct {
		#[mvc(builder)]
		fn builder(
			config: MyStructConfig,
			view: String,
			macro_type: InstanceType,
		) -> Result<Self, Error> {
			Ok(Self {
				config,
				view,
				count: 0,
				debug_bark: false,
			})
		}

		#[mvc(add=[Dog, TestMyStruct])]
		fn bark(&self) -> Result<(), Error> {
			if self.debug_bark {
				warn!("woof!")?;
			} else {
				info!("woof!")?;
			}
			Ok(())
		}

		#[mvc(add=[Cat, TestMyStruct])]
		fn meow(&self) -> Result<(), Error> {
			info!("meow!")?;
			Ok(())
		}

		#[mvc(add=[Bird, TestMyStruct])]
		fn chirp(&self) -> Result<(), Error> {
			info!("chirp!")?;
			Ok(())
		}

		#[mvc(add=[Dog, Cat, Bird, TestMyStruct])]
		fn speak(&self) -> Result<(), Error> {
			match self.view.as_str() {
				"Dog" => self.bark(),
				"Cat" => self.meow(),
				"Bird" => self.chirp(),
				"TestMyStruct" => {
					self.bark()?;
					self.meow()?;
					self.chirp()?;
					Ok(())
				}
				_ => Err(err!(ErrKind::IllegalState, "unknown trait type")),
			}
		}

		#[mvc(add=[TestMyStruct])]
		fn set_debug_bark(&mut self) {
			self.debug_bark = true;
		}
	}

	#[test]
	fn test_mvc() -> Result<(), Error> {
		info!("test_mvc")?;
		Ok(())
	}
}
