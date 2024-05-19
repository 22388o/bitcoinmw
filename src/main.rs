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

use std::mem::size_of;

// include build information
#[doc(hidden)]
pub mod build_info {
	include!(concat!(env!("OUT_DIR"), "/bmw_build.rs"));
}

fn main() -> Result<(), Error> {
	real_main(false)?;
	Ok(())
}

fn real_main(debug_startup_32: bool) -> Result<(), Error> {
	// ensure we only support 64 bit
	match size_of::<&char>() == 8 && debug_startup_32 == false {
		true => {}
		false => panic!("unsuported arch"),
		/*ret_err!(MainErrorKind::UnsupportedArch, "Only 64 bit arch supported")*/
	}

	println!("main currently doesn't do anything");
	println!("build pkg_version = {}", build_info::PKG_VERSION);
	Ok(())
}

#[derive(Debug)]
pub enum Error {}

/*
#[ErrorKind]
enum MainErrorKind {
	UnsupportedArch,
}
*/
