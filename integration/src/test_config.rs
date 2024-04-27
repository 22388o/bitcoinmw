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
use bmw_derive2::*;

// define an error kind
#[ErrorKind]
enum IntErrorKind {
	/// integration error 7
	Integration,
	/// test error
	Test123,
	Abc123,
}

// define a second error kind
#[ErrorKind]
enum IntErrorKind2 {
	/// integration error
	Integration,
	/// test error
	Test123,
	Abc123,
}

#[cfg(test)]
mod test {
	use bmw_base::*;
	use bmw_derive2::*;

	#[derive(Configurable)]
	struct ConfTest {
		a: u8,
		b: u16,
		c: u32,
		d: u64,
		e: u128,
		f: usize,
		g: bool,
		h: String,
		i: (String, String),
		j: Vec<usize>,
	}

	impl Default for ConfTest {
		fn default() -> Self {
			Self {
				a: 1,
				b: 2,
				c: 3,
				d: 4,
				e: 5,
				f: 6,
				g: false,
				h: "".to_string(),
				i: ("x".to_string(), "y".to_string()),
				j: vec![],
			}
		}
	}

	#[test]
	fn test_config() -> Result<(), Error> {
		// test with defaults
		let x = configure!(ConfTest, ConfTestOptions, vec![])?;
		// all values should be what is returned by the default function
		assert_eq!(x.a, 1);
		assert_eq!(x.b, 2);
		assert_eq!(x.c, 3);
		assert_eq!(x.d, 4);
		assert_eq!(x.e, 5);
		assert_eq!(x.f, 6);
		assert_eq!(x.g, false);
		assert_eq!(x.h, "".to_string());
		assert_eq!(x.i, ("x".to_string(), "y".to_string()));
		assert_eq!(x.j, vec![]);

		// overwrite one value
		let x = configure!(ConfTest, ConfTestOptions, vec![ConfTestOptions::A(10)])?;
		// this value should be the overwritten value
		assert_eq!(x.a, 10);
		// default
		assert_eq!(x.b, 2);

		// configure some j values (Vec)
		let x = configure!(
			ConfTest,
			ConfTestOptions,
			vec![
				ConfTestOptions::J(1),
				ConfTestOptions::J(2),
				ConfTestOptions::J(7),
				ConfTestOptions::C(300)
			]
		)?;

		// all three values are in the array
		assert_eq!(x.j, vec![1, 2, 7]);
		assert_eq!(x.c, 300);
		assert_eq!(x.a, 1);

		// do some duplicates
		assert!(configure!(
			ConfTest,
			ConfTestOptions,
			vec![ConfTestOptions::A(0), ConfTestOptions::A(1)]
		)
		.is_err());
		Ok(())
	}
}
