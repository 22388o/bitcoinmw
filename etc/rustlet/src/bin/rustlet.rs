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

use bmw_deps::dirs;
use bmw_http::{HttpConfig, HttpInstance, HttpInstanceType, PlainConfig};
use bmw_rustlet::*;
use clap::{load_yaml, App, ArgMatches};
use std::collections::HashMap;
use std::fs::canonicalize;
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

	let home_dir = match dirs::home_dir() {
		Some(p) => p,
		None => PathBuf::new(),
	}
	.as_path()
	.display()
	.to_string();

	let base_dir = base_dir.replace("~", &home_dir);
	let base_dir = canonicalize(base_dir)?.display().to_string();

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
	let rc = RustletConfig {
		http_config: HttpConfig {
			instances: vec![HttpInstance {
				port: config.port,
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([(
						"*".to_string(),
						format!("{}/www", config.base_dir.clone()),
					)]),
				}),
				..Default::default()
			}],
			debug: config.debug,
			..Default::default()
		},
		..Default::default()
	};
	rustlet_init!(rc)?;
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
	})?;

	rustlet_mapping!("/abc", "test")?;

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
