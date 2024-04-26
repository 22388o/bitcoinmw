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

use crate::test_class::IntegrationErr::Overloaded;
use bmw_base::*;
use bmw_derive::*;
use std::pin::Pin;

#[ErrorKind]
pub enum IntegrationErr {
	/// overloaded error
	Overloaded,
}

#[class {
		[]
		fn _x() {}

		/// comments here go to the trait top documentation only @see works
		/// if multiple are found these are appended. If you want separate stuff for dog
		/// vs cat use separate public lines
		public dog, dog_sync_box, dog_sync, dog_send, dog_box, dog_send_box;
		/// Cat trait here
		public cat;
		/// y coordinate if the item is last, otherwise, it's the first value in the list
		const y: usize = 1;
		/// the number of items in the list
		const z: u8 = 10;
		/// the name of the server
		const server_name: String = "my_server".to_string();
		/// number of threads
		const threads: usize = 6;
		/// timeout in milliseconds
		const timeout: usize = 3_000;
		var x: i32;
		var v: usize;

				/// @module bmw_int::test_class
		fn builder(&const_values) -> Result<Self, Error> {
			Ok(Self { x: -100, v: *const_values.get_y() })
		}

		/// function documentation moved over to the trait fns
		/// @param self this is a mutable reference to self
		[dog, test2]
		fn bark(&mut self) -> Result<String, Error> {
				self.do_cool_stuff()?;
				*self.get_mut_x() += 1;
				println!("x={},y={}", self.get_x(), self.get_y());
				Ok("woof".to_string())
		}

		[dog]
		fn dogg(&self) -> usize {
				self.get_v().clone()
		}

		/// The meow function returns a wonderful meow.
		/// @param v1 the version number to use
		/// @param v2 whether or not v2 is supported
		/// @param self mutable reference to self
		/// @return std::string::String value that has been created
		/// @error bmw_base::BaseErrorKind::IllegalState if the state becomes illegal
		/// @error bmw_base::BaseErrorKind::Configuration if the configuration is invalid
		/// @error crate::IntegrationErr::Overloaded if the system is overloaded
		/// @see crate::dog
		/// @see crate::cat
		/// @see bmw_base::Error
		/// @deprecated
		/// # Examples
		///```
		/// use bmw_base::*;
		/// // use bmw_int::test_class::*;
		/// use bmw_int::*;
		///
		/// fn main() -> Result<(), Error> {
		///     //let mut cat = cat!()?;
		///     // let s = cat.meow(1, false)?;
		///     // assert_eq!(s, "meow".to_string());
		///
		///     Ok(())
		/// }
		///```
		[cat, test2]
		fn meow(&mut self, v1: usize, v2: bool) -> Result<String, Error> {
				self.other();
				Ok("meow".to_string())
		}

		/// This function returns the server name for this instance.
		/// Once we're done we process the request
		/// Potential errors can occur.
		/// # test
		/// ok here's some more info
		/// * ok ok ok
		/// * hi
		/// * this is more
		///```
		/// fn main() {}
		///```
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

struct Context {
	buffer: [u8; 1024],
	counter: usize,
}

impl Context {
	fn new() -> Self {
		Self {
			buffer: [0u8; 1024],
			counter: 0,
		}
	}
}

#[class{
	public xserver_send_box;

	const threads: usize = 1;
		const headers: Vec<(String, String)> = vec![];
		const str: String = "".to_string();
		const header_conf: (String, String) = ("".to_string(), "ok".to_string());
		const bbvec: Vec<bool> = vec![false];
	var context: Context;
	var abc: usize;

		/// @module bmw_int::test_class
	fn builder(&const_values) -> Result<Self, Error> {
			let context = Context::new();
			let abc = 100;
		Ok(Self { context, abc })
	}

	[xserver, t1]
	fn start(&mut self) -> Result<(), Error> {
				(*self.get_mut_context()).counter += 1;
		Ok(())
	}

		[xserver]
		fn get_stats(&self) -> usize {
				(*self.get_context()).counter
		}

		/// Set the value of `offset` within the internal buffer to `value`.
		/// @param value the u8 value to set
		/// @param self mutable reference to this [`Xserver`]
		/// @param offset offset of the byte in the buffer to set
		/// or so they say
		/// @error bmw_base::BaseErrorKind::IllegalState if the offset is greater than the size of
		/// the buffer
		/// @return unit unit value
		/// that's it
		/// @see crate::xserver_send_box
		/// @see crate::test_class::AnimalBuilder
		/// # Example
		///```
		/// use bmw_int::*;
		/// use bmw_base::*;
		/// use bmw_int::test_class::*;
		/// use bmw_int::test_class::ServerConstOptions::*;
		///
		/// fn main() -> Result<(), Error> {
		///     let mut server = xserver_send_box!(Threads(10))?;
		///
		///     assert!(server.set_buffer(20, 0u8).is_ok());
		///
		///     Ok(())
		/// }
		///```
		[xserver]
		fn set_buffer(&mut self, offset: usize, value: u8) -> Result<(), Error> {
				(*self.get_mut_context()).buffer[offset] = value;
				Ok(())
		}

		[xserver]
		fn get_buffer(&mut self, offset: usize) -> Result<u8, Error> {
				Ok((*self.get_context()).buffer[offset])
		}

		[xserver]
		fn get_header(&self, offset: usize) -> Result<(String, String), Error> {
				if offset >= self.get_headers().len() {
						err!(BaseErrorKind::IllegalState, "out of bounds")
				} else {
						Ok(self.get_headers()[offset].clone())
				}
		}

		[xserver]
		fn test_server(&self) -> Result<(), Error> {
				err!(Overloaded, "integration test")
		}
}]
impl Server {}

#[class{
        public http_server_sync_box;

        var handler: Option<Pin<Box<OnRead>>>;
        var count: usize;

        /// @module bmw_int::test_class
        /// @add_test_init x.set_on_read_impl(|_| {})?; // define on_read handler
        fn builder(&const_values) -> Result<Self, Error> {
            Ok(Self { handler: None, count: 0 })
        }

        [http_server]
        fn set_on_read_impl(&mut self, on_read: OnRead) -> Result<(), Error> {
                (*self.get_mut_handler()) = Some(Box::pin(on_read));
                Ok(())
        }

        [http_server]
        fn start(&mut self) -> Result<(), Error> {
            let ret = self.get_count().clone();
                match (*self.get_mut_handler()) {
                        Some(ref mut on_read) => {
                                on_read(ret);
                        }
                        None => println!("none"),
                }
                Ok(())
        }

        [http_server]
        fn incr(&mut self) -> Result<(), Error> {
                *self.get_mut_count() += 1;
                Ok(())
        }

        [http_server]
        fn get_handler_impl(&self) -> Result<Option<Pin<Box<OnRead>>>, Error> {
                Ok(self.get_handler().clone())
        }
}]
impl<OnRead> HttpServerImpl<OnRead> where
	OnRead: FnMut(usize) -> () + Send + 'static + Clone + Sync + Unpin
{
}

#[cfg(test)]
mod test {
	use crate::test_class::*;

	#[test]
	fn test_http() -> Result<(), Error> {
		let mut http_server_sync_box = http_server_sync_box!()?;
		http_server_sync_box.set_on_read_impl(|size| {
			println!("in onread: {}", size);
		})?;
		assert!(http_server_sync_box.start().is_ok());
		http_server_sync_box.incr()?;
		assert!(http_server_sync_box.start().is_ok());

		Ok(())
	}

	#[test]
	fn test_server() -> Result<(), Error> {
		let mut server = xserver_send_box!(
			Headers(("name", "value")),
			Bbvec(false),
			Bbvec(false),
			Bbvec(true)
		)?;
		assert_eq!(server.get_stats(), 0);
		server.start()?;
		assert_eq!(server.get_stats(), 1);

		server.set_buffer(3, 9u8)?;
		assert_eq!(server.get_buffer(3)?, 9u8);

		assert_eq!(
			server.get_header(0)?,
			("name".to_string(), "value".to_string())
		);

		assert!(server.get_header(1).is_err());

		Ok(())
	}

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
