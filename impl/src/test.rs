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
	use crate::{cbreak, err, kind, map_err, try_into, CoreErrorKind, Error, ErrorKind};
	use std::num::ParseIntError;

	fn ret_err() -> Result<(), Error> {
		err!(CoreErrorKind::Parse, "this is a test {}", 1)
	}

	fn ret_err2() -> Result<(), Error> {
		err!(CoreErrorKind::Parse, "this is a test")
	}

	fn ret_err3() -> Result<usize, ParseIntError> {
		"".parse::<usize>()
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		assert!(ret_err().is_err());

		let err: Error = ret_err().unwrap_err();
		let kind = err.kind();

		assert_eq!(kind, &kind!(CoreErrorKind::Parse, "this is a test 1"));
		assert_ne!(kind, &kind!(CoreErrorKind::Parse, "this is a test 2"));

		let err: Error = ret_err2().unwrap_err();
		assert_eq!(err.kind(), &kind!(CoreErrorKind::Parse, "this is a test"));

		Ok(())
	}

	#[test]
	fn test_map_err() -> Result<(), Error> {
		let e = map_err!(ret_err3(), CoreErrorKind::Parse, "1").unwrap_err();
		let exp_text = "1: cannot parse integer from empty string";
		assert_eq!(e.kind(), &kind!(CoreErrorKind::Parse, exp_text));

		let e = map_err!(ret_err3(), CoreErrorKind::Parse).unwrap_err();
		let exp_text = "cannot parse integer from empty string";
		assert_eq!(e.kind(), &kind!(CoreErrorKind::Parse, exp_text));

		Ok(())
	}

	#[test]
	fn test_cbreak() -> Result<(), Error> {
		let mut count = 0;
		loop {
			count += 1;
			cbreak!(count == 10);
		}
		assert_eq!(count, 10);
		Ok(())
	}

	#[test]
	fn test_try_into() -> Result<(), Error> {
		let x: u64 = try_into!(100u32)?;
		assert_eq!(x, 100u64);

		let x: u32 = try_into!(100u64)?;
		assert_eq!(x, 100u32);

		let x: Result<u32, Error> = try_into!(u64::MAX);
		let exp_text = "out of range integral type conversion attempted";
		assert_eq!(
			x.unwrap_err().kind(),
			&kind!(CoreErrorKind::TryInto, exp_text)
		);
		Ok(())
	}
}
