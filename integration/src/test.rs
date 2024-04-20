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

	/*

	#[object]
	#[public(cat,dog)] // make dog and cat view pub (optional) - default is private
	#[protected(bird)] // make bird view protected pub(crate) (optional) - default is private
	#[no_send] // the send and sync builders will not be generated (optional)
	#[no_sync] // the sync builders will not be generated (optional)
	#[doc_hidden(test)] // hide the docs for test (optional) all traits are public with public
							// docs unless this is specified. Builder struct prevents building of
							// non-public views.
	public MyObject {
			// user-initialized
			let x: usize = 0;
			let s: String = "".to_string();
			let list: Vec<String> = vec!["".to_string(), "another".to_string()];

			// non user-initialised
			z: String;
			debug_bark: bool;
			counter: u64;
			arr: [i32; 3];

			builder {
					if config.x == 0 {
							return Err(err!(ErrKind::Configuration, "0 not legal for x"));
					}

					// all non user-initialized values must be included here
					Ok(Self {
							z: "ok".to_string(),
							debug_bark: false,
							counter: 0,
							arr: [0,0,0],
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

	#[object]
	#[config(threads: usize = 1)]
	#[config(timeout: usize = 10_000)]
	#[field(counter: u128)]
	#[field(debug: bool)]
	#[builder{
                if config.threads == 0 {
                        return Err(err!(ErrKind::Configuration, "Threads must be at least 1"));
                }

                if config.timeout > 1_000_000 {
                        return Err(err!(ErrKind::Configuration, "Timeout must be equal to or less than 1_000_000"));
                }

                Ok(Self {
                        counter: 0,
                        debug: false,
                })
        }]
	#[method(dog, test, [fn bark(&mut self) -> Result<(), Error>])]
	#[method(cat, test, [fn meow(&mut self) -> Result<(), Error>])]
	#[method(bird, test, [fn chirp(&mut self) -> Result<(), Error>])]
	#[method(dog, cat, bird, test, [fn speak(&mut self) -> Result<(), Error>])]
	#[method(test, [fn set_debug(&mut self, value: bool) -> Result<(), Error>])]
	#[public(dog, cat)]
	#[protected(bird)]
	#[doc_hidden(test)]
	#[no_send]
	#[no_sync]
	impl Animal {
		fn bark(&mut self) -> Result<(), Error> {
			if self.debug {
				println!("WOOF!");
			} else {
				println!("woof!");
			}

			self.update();
			Ok(())
		}

		fn meow(&mut self) -> Result<(), Error> {
			let now = Instant::now();
			if self.debug {
				println!("MEOW!");
			} else {
				println!("meow!");
			}

			if now.elapsed().as_millis() > self.config().timeout {
				return Err(err!(ErrKind::Timeout, "timeout err"));
			}

			self.update();
			Ok(())
		}

		fn chirp(&mut self) -> Result<(), Error> {
			if self.debug {
				println!("CHIRP!");
			} else {
				println!("chirp!");
			}

			self.update();
			Ok(())
		}

		fn set_debug(&mut self, value: bool) -> Result<(), Error> {
			self.get_mut_debug() = value;
			Ok(())
		}

		fn speak(&mut self) -> Result<(), Error> {
			self.show_config()?;
			match self.trait_name() {
				Animal::Dog => self.bark()?,
				Animal::Cat => self.meow()?,
				Animal::Bird => self.chirp()?,
				Animal::Test => {
					self.bark()?;
					self.meow()?;
					self.chirp()?;
				}
			}
			Ok(())
		}

		fn show_config(&self) -> Result<(), Error> {
			println!("self.config()={:?}", self.config());
			println!("self.trait_type()={:?}", self.trait_type());
			println!("self.trait_name=()={:?}", self.trait_name());
			println!("counter={}", self.get_counter());
		}

		fn update(&mut self) {
			self.get_mut_counter() += 1;
		}
	}

	enum MyErrKind {
		Dead,
		AlmostDead,
	}

	trait SayHi {
		fn say_hi(&self) -> String;
	}

	impl SayHi for MyErrKind {
		fn say_hi(&self) -> String {
			match self {
				Self::Dead => "hi".to_string(),
				_ => "hi2".to_string(),
			}
		}
	}

	#[test]
	fn test_object() -> Result<(), Error> {
		// let dog = dog!(Timeout(100), Threads(2))?;
		// let dog_dyn = dog_dyn!(Timeout(100))?;
		// let dog_dyn_send = dog_dyn_send(Threads(10))?;
		// let cat = cat!()?;

		Ok(())
	}
}
