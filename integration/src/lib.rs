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

//! Integration testing crate

mod test;

use bmw_derive::*;
use bmw_err::*;
#[document]
/// Test123
///
#[add_doc(see: "bmw_err::Error")]
pub trait TestDoc {
	#[document]
	#[add_doc(see: "bmw_err::Error")]
	#[add_doc(return: "unit type is returned")]
	#[add_doc(error: "bmw_err::ErrKind::IllegalState" - "some err text")]
	fn test1(&self, s: &String) -> Result<(), Error>;
	#[document]
	/// ok
	/// ok1
	/// ok2
	/// ok3
	#[add_doc(doc_point)]
	#[add_doc(see: "bmw_err::err!")]
	#[add_doc(see: "crate::TestDoc")]
	#[add_doc(see: "crate::TestDoc::test1")]
	#[add_doc(input: "s" - "this is a test.")]
	#[add_doc(input: "s" - "more on s.")]
	#[add_doc(input: "self" - "the reference to itself")]
	#[add_doc(input: "self" - "more on self")]
	#[add_doc(error: "bmw_err::ErrKind::Log" - "error message1")]
	#[add_doc(error: "bmw_err::ErrKind::Configuration" - "error 2")]
	#[add_doc(return: "returns an i32 that's correct.")]
	#[add_doc(return: "more info on return.")]
	fn test2(&mut self, s: usize) -> Result<i32, Error>;

	#[document]
	/// Other doc
	#[add_doc(return: "the value is a usize")]
	#[add_doc(see: "bmw_err::ErrKind")]
	#[add_doc(error: "bmw_err::ErrKind::Log" - "log err")]
	#[add_doc(doc_point)]
	///
	/// ok here
	fn test3() -> i64;

	#[document]
	/// pre message
	#[add_doc(return: "unit is returned")]
	#[add_doc(see: "bmw_err::Error")]
	#[add_doc(doc_point)]
	///
	/// post message
	fn ok();
}

#[document]
/// ok pre
#[add_doc(doc_point)]
///
/// ok post
#[add_doc(see: "bmw_err::ErrKind")]
#[add_doc(return: "Some return message2" - "[`Box`] < dyn [`std::any::Any`] + [`Send`] + [`Sync`]>")]
#[add_doc(input: "xyz" - "this is the comment for xyz" - " u64 ")]
#[add_doc(error: "bmw_err::ErrKind::Log" - "log err occurred")]
#[add_doc(error: "bmw_err::ErrKind::Log" - "log err occurred2")]
#[macro_export]
macro_rules! test_macro {
	($xyz:expr) => {{
		println!("ok");
	}};
}
