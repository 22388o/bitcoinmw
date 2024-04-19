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

	#[derive(Serializable)]
	struct MyStruct {
		v1: usize,
		v2: u8,
		v3: u16,
		v4: u32,
	}

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
		fn test2(&mut self, xyz: Box<dyn Any + '_>) -> Result<(usize, ConfigMe), Error>;
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
}
