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
use bmw_derive2::{document, ErrorKind};
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;

#[ErrorKind]
pub enum IntegrationError {
	/// document err
	DocumentError,
}

//#[document]
/// test
/// next
/// final
#[macro_export]
macro_rules! test {
	() => {{}};
	($line:expr,$($values:tt)*) => {
		println!("ok");
		let x = 1 + 1;
		println!("x={}", x);
	};
}

//#[document]
/// ok
/// here
pub trait test_trait {
	//#[document]
	/// This is a test
	/// more
	/// more2
	fn test(&mut self) -> Result<impl Display + Send + Unpin, Error>;

	//#[document]
	/// ok ok ok
	/// asjk
	/// @param self immutable ref to self
	/// @param x the size of the item to display
	/// @param y optional Any attribute
	/// @param z a very nice bool
	/// @return the [`unit`] is returned
	/// @see bmw_base::Error
	/// @see bmw_base::ErrorKind
	/// @see bmw_derive::document()
	/// @error bmw_base::BaseErrorKind::IllegalState if there is an illegal state
	/// @error bmw_base::BaseErrorKind::Parse if a parse error occurs
	/// @error crate::test_document::IntegrationError::DocumentError if a documenting error occurs
	fn test2(&self, x: usize, y: Option<Box<dyn Any + Send + Sync>>, z: bool);

	//#[document]
	///
	///
	fn test3(&mut self, x: Result<(), Error>, y: String) -> usize;
}

pub struct MyStruct {}

impl MyStruct {
	#[document]
	/// This is a test
	/// more
	/// more2
	/// additional
	/// @param f hi
	/// @param timeout some comment here!
	/// @param group_name gggggggg
	/// next g
	/// next next g
	/// @param threads 123
	/// @param self abc
	/// second comment
	/// ok another
	/// # Examples
	///```
	/// use bmw_base::*;
	///
	/// fn main() -> Result<(), Error> {
	///     Ok((())
	/// }
	///```
	pub fn test(
		&mut self,
		threads: usize,
		context: Result<(), Error>,
		timeout: Result<Option<Vec<String>>, Error>,
		other_space_param: (
			Box<dyn Any + Send + Sync>,
			String,
			Vec<Option<Box<dyn Debug>>>,
		),
		f: Box<dyn Any>,
		group_name: &mut Box<dyn Any + Sync + Unpin>,
	) -> Result<(), Error> {
		Ok(())
	}
}
