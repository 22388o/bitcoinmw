// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw and Grin Developers
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

//! Build hooks to spit out version+build time info
use std::env::var;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const CARGO_MANIFEST_ERROR: &str = "could not retrieve CARGO_MANIFEST_DIR env var";

fn main() {
	// Setting up git hooks in the project: rustfmt and so on.
	let git_hooks = format!(
		"git config core.hooksPath {}",
		PathBuf::from("./.hooks").to_str().unwrap()
	);

	// execute git_hooks
	if cfg!(target_os = "windows") {
		Command::new("cmd")
			.args(&["/C", &git_hooks])
			.output()
			.expect("failed to execute git config for hooks");
	} else {
		Command::new("sh")
			.args(&["-c", &git_hooks])
			.output()
			.expect("failed to execute git config for hooks");
	}

	// create build file with needed info
	let mut build_file = PathBuf::new();
	build_file.push(Path::new(
		&var("OUT_DIR").expect("Build error: OUT_DIR not set"),
	));
	build_file.push("bmw_build.rs");

	let pkg_version = var("CARGO_PKG_VERSION").expect(CARGO_MANIFEST_ERROR);
	let build_text = format!("pub const PKG_VERSION: &str = \"{}\";", pkg_version);

	let mut file = match File::create(build_file) {
		Ok(file) => file,
		Err(e) => panic!("Build Error: {}", e),
	};
	match file.write_all(build_text.as_bytes()) {
		Ok(_) => {}
		Err(e) => panic!("Build Error: {}", e),
	}
}
