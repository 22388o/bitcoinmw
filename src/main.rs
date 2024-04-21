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

use bmw_base::{err, BaseErrorKind, Error};
use std::mem::size_of;

// include build information
#[doc(hidden)]
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() -> Result<(), Error> {
	real_main(false)?;
	Ok(())
}

fn real_main(debug_startup_32: bool) -> Result<(), Error> {
	// ensure we only support 64 bit
	match size_of::<&char>() == 8 && debug_startup_32 == false {
		true => {}
		false => return err!(BaseErrorKind::IllegalState, "Only 64 bit arch supported"),
	}

	println!("main currently doesn't do anything");
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::{main, real_main};
	use bmw_base::Error;

	#[test]
	fn test_main() -> Result<(), Error> {
		assert!(main().is_ok());
		Ok(())
	}

	#[test]
	fn test_debug_startup_32() -> Result<(), Error> {
		assert!(real_main(true).is_err());
		Ok(())
	}
}
