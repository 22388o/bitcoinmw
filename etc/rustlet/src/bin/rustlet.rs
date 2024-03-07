// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_rustlet::*;
use clap::{load_yaml, App, ArgMatches};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread::park;

info!();

struct RustletContainerConfig {
	debug: bool,
	base_dir: String,
	port: u16,
}

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn load_config(args: ArgMatches<'_>) -> Result<RustletContainerConfig, Error> {
	let debug = args.is_present("debug");

	let base_dir = match args.is_present("base_dir") {
		true => args.value_of("base_dir").unwrap(),
		false => "~/.bmw",
	}
	.to_string();

	let port = match args.is_present("port") {
		true => args.value_of("port").unwrap().parse()?,
		false => 8080,
	};

	Ok(RustletContainerConfig {
		base_dir,
		debug,
		port,
	})
}

fn show_startup_config(config: &RustletContainerConfig) -> Result<(), Error> {
	info!("base_dir={},debug={}", config.base_dir, config.debug)?;

	Ok(())
}

fn init_rustlet_container(config: &RustletContainerConfig) -> Result<(), Error> {
	let mut instance_dir = PathBuf::new();
	instance_dir.push(config.base_dir.clone());
	instance_dir.push("www");
	let instance_dir = &instance_dir.display().to_string();
	rustlet_init!(
		BaseDir(&config.base_dir),
		Debug(config.debug),
		instance!(Port(config.port), BaseDir(&instance_dir))?
	)?;
	Ok(())
}

fn load_rustlets(_config: &RustletContainerConfig) -> Result<(), Error> {
	// for now just load some sample rustlets inline, but in the future
	// load rustlets via shared library.

	rustlet!("test", {
		let mut response = response!()?;
		let request = request!()?;
		info!(
			"in rustlet test_rustlet_simple_request test method={:?},path={}",
			request.method(),
			request.path()
		)?;
		response.write(b"abc")?;
		Ok(())
	})?;

	rustlet!("echo", {
		let request = request!()?;
		let mut response = response!()?;
		let query = request.query();
		response
			.write(format!("<br/><br/><br/><br/><br/><br/><br/>query was: {}", query).as_bytes())?;
		Ok(())
	})?;

	rustlet!("simple", Ok(()))?;

	rustlet!("redirect", {
		let mut response = response!()?;
		response.redirect("http://www.example.com")?;
		Ok(())
	})?;

	rustlet_mapping!("/abc", "test")?;
	rustlet_mapping!("/echo", "echo")?;
	rustlet_mapping!("/simple", "simple")?;
	rustlet_mapping!("/redirect", "redirect")?;

	Ok(())
}

fn main() -> Result<(), Error> {
	let yml = load_yaml!("rustlet.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let config = load_config(args)?;
	show_startup_config(&config)?;
	init_rustlet_container(&config)?;
	load_rustlets(&config)?;
	rustlet_start!()?;

	park();

	Ok(())
}
