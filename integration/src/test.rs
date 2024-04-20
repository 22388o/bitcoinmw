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
	use bmw_derive::*;
	use bmw_err::*;
	use std::any::Any;

	/*
	//#[derive(Serializable)]
	enum Other {
		ABC(usize),
		DEF(u8),
	}

	//#[derive(hibernate)]
	//#[name(MyTestDB)] // optional default name would be the struct name.
	struct TestUser {
		v1: usize,
		//#[unique]
		name: String,
		a: (Vec<String>, Option<Other>),
		//#[unique]
		email: String,
		list: Vec<String>,
	}

	impl default for TestUser {
		...
	}
	fn test_hibernate() -> Result<(), Error> {
		let mut hibernate = hibernate!(Host("127.0.0.1"), Port(1234))?;
		// or let mut hibernate = hibernate!(File("/path/to/lmdbfile"))?;
		let mut user = test_user!(&mut hibernate)?;
		if user.read(vec![Name("Joe"), Email("joe@example.com")])? {
			user.setList(vec!["test1".to_string(), "test2".to_string()]);
			user.update()?;
		}

		user.commit()?;
		Ok(())
	}
	 * */

	/*
	#[derive(Serializable)]
	struct MyStruct {
		v1: usize,
		v2: u8,
		v3: u16,
		v4: u32,
	}
		*/

	#[test]
	fn test_serializable() -> Result<(), Error> {
		Ok(())
	}

	#[derive(Configurable)]
	struct ConfigMe {
		v1: usize,
		v2: u8,
	}

	impl Default for ConfigMe {
		fn default() -> Self {
			Self { v1: 0, v2: 1 }
		}
	}

	#[document]
	/// pre comment1
	/// pre comment2
	#[add_doc(see: "bmw_derive::Serializable")]
	/// pre comment3
	///
	#[add_doc(doc_point)]
	/// this is a regular comment (post)
	/// another comment (post)
	pub trait MyTrait {
		#[document]
		/// reg doc
		#[add_doc(see: "std::fmt")]
		#[add_doc(return: "anything here")]
		/// ok doc too
		fn test(&self, abc: usize) -> Result<(), Error>;
		#[document]
		/// abc
		/// def
		#[add_doc(input: "xyz" - "something unexpected")]
		/// ghi
		#[add_doc(doc_point)]
		/// 123
		/// 456
		/// 789
		/// ok
		fn test2(
			&mut self,
			v: &[u8],
			b: [u8; 10],
			xyz: Box<dyn Any + '_>,
		) -> Result<(usize, ConfigMe), Error>;
	}

	#[test]
	fn test_config_proc_macro() -> Result<(), Error> {
		let config = config!(ConfigMe, ConfigMeOptions, vec![V1(2), V2(3)])?;
		assert_eq!(config.v1, 2);
		assert_eq!(config.v2, 3);
		let config = config!(ConfigMe, ConfigMeOptions, vec![])?;
		assert_eq!(config.v1, 0);
		assert_eq!(config.v2, 1);
		Ok(())
	}

	struct MyStruct {
		config: MyStructConfig,
		view: String,
		trait_type: TraitType,
		count: u128,
		debug_bark: bool,
	}

	impl Default for MyStructConfig {
		fn default() -> Self {
			Self {
				threads: 1,
				timeout: 5_000,
				server_name: "my_server".to_string(),
				headers: vec![],
				header: ("".to_string(), "".to_string()),
				abc: vec![],
				def: vec![],
			}
		}
	}

	#[traitify(
                views=[
                        pub Cat,
                        pub Dog,
                        pub(crate) Bird,
                        TestMyStruct,
                ],
                type=[
                        IMPL,
                        DYN,
                        IMPL_SEND,
                        IMPL_SYNC,
                        DYN_SEND,
                        DYN_SYNC,
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
		#[traitify(builder)]
		fn builder(
			config: MyStructConfig,
			view: String,
			trait_type: TraitType,
		) -> Result<Self, Error> {
			Ok(Self {
				config,
				view,
				trait_type,
				count: 0,
				debug_bark: false,
			})
		}

		#[traitify(add=[Dog, TestMyStruct])]
		fn bark(&self) -> Result<(), Error> {
			if self.debug_bark {
				println!("WOOF!");
			} else {
				println!("woof!");
			}
			Ok(())
		}

		#[traitify(add=[Dog])]
		fn set_count(&mut self, value: u128) -> Result<u128, Error> {
			let ret = self.count;
			self.count = value;
			Ok(ret)
		}

		#[traitify(add=[Cat, TestMyStruct])]
		fn meow(&self) -> Result<(), Error> {
			println!("meow!");
			Ok(())
		}

		#[traitify(add=[Bird, TestMyStruct])]
		fn chirp(&self) -> Result<(), Error> {
			println!("chirp!");
			Ok(())
		}

		#[traitify(add=[Dog, Cat, Bird, TestMyStruct])]
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

		#[traitify(add=[TestMyStruct])]
		fn set_debug_bark(&mut self) {
			self.debug_bark = true;
		}
	}

	// generated stuff

	/*
	trait Dog {
		fn bark(&self) -> Result<(), Error>;
		fn speak(&self) -> Result<(), Error>;
		fn set_count(&mut self, value: u128) -> Result<u128, Error>;
	}

	impl Dog for MyStruct {
		fn bark(&self) -> Result<(), Error> {
			MyStruct::bark(self)
		}
		fn speak(&self) -> Result<(), Error> {
			MyStruct::speak(self)
		}
		fn set_count(&mut self, value: u128) -> Result<u128, Error> {
			MyStruct::set_count(self, value)
		}
	}
				*/

	/*

	#[objectify]
	#[public(cat,dog)] // make dog and cat pub (optional) - default is private
	#[protected(bird)] // make bird protected pub(crate) (optional) - default is private
	#[no_send] // the send and sync builders will not be generated (optional)
	#[no_sync] // the sync builders will not be generated (optional)
	{
			x: [usize, 0],
			y: [String, "".to_string()],
			z: String,
			debug_bark: bool,
			counter: u64,

			build {
					if config.x == 0 {
							return Err(err!(ErrKind::Configuration, "0 not legal for x"));
					}
					Ok(Self {
							z: "ok".to_string(),
							debug_bark: false,
							counter: 0,
					})
			}

			[dog, cat, test]
			fn print_x(&self) {
					println!("x={}", self.config.x);
			}

			[dog, test]
			fn bark(&self) -> Result<(), Error> {
					if self.debug_bark {
							println!("WOOF!");
					} else {
							println!("woof!");
					}
					self.update();
					Ok(())
			}

			[cat, test]
			fn meow(&self) -> Result<(), Error> {
					println!("meow!");
					self.update();
					Ok(())
			}

			[bird, test]
			fn chirp(&self) -> Result<(), Error> {
					println!("chirp")?;
					self.update();
					Ok(())
			}

			[test]
			fn debug_bark(&mut self) {
					self.debug_bark = true;
					Ok(())
			}

			fn update(&mut self) {
					self.counter += 1;
			}

	}

	let dog = dog!(X(1), Y("test"))?;
	let dog = dog_dyn!(X(2))?;
	let dog = dog_dyn_sync!()?;


		 */

	#[test]
	fn test_traitify() -> Result<(), Error> {
		let my_str = config!(
			MyStructConfig,
			MyStructConfigOptions,
			vec![Timeout(100), Threads(4)]
		)?;

		assert_eq!(my_str.threads, 4);
		assert_eq!(my_str.timeout, 100);
		assert_eq!(my_str.server_name, "my_server".to_string());
		Ok(())
	}
}
