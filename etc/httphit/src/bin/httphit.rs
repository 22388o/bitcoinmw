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

use bmw_err::*;
use bmw_http::*;
use bmw_log::*;
use bmw_util::*;
use clap::{load_yaml, App, ArgMatches};
use std::collections::HashSet;
use std::sync::mpsc::sync_channel;
use std::time::Instant;

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Clone)]
struct HttpHitConfig {
	threads: usize,
	iterations: usize,
	connections: usize,
	urls: Vec<String>,
	count: usize,
}

fn load_config(args: ArgMatches<'_>) -> Result<HttpHitConfig, Error> {
	let threads: usize = match args.is_present("threads") {
		true => args.value_of("threads").unwrap().parse()?,
		false => 1,
	};

	let iterations: usize = match args.is_present("iterations") {
		true => args.value_of("iterations").unwrap().parse()?,
		false => 1,
	};

	let connections: usize = match args.is_present("connections") {
		true => args.value_of("connections").unwrap().parse()?,
		false => 1,
	};

	let urls: Vec<String> = match args.is_present("url") {
		true => {
			let values = args.values_of("url").unwrap();
			let mut r = vec![];
			for v in values {
				r.push(v.to_string());
			}
			r
		}
		false => vec![],
	};

	let count: usize = match args.is_present("count") {
		true => args.value_of("count").unwrap().parse()?,
		false => 1,
	};

	Ok(HttpHitConfig {
		threads,
		iterations,
		connections,
		urls,
		count,
	})
}

fn show_startup_config(config: &HttpHitConfig) -> Result<(), Error> {
	info!(
		"threads={},iterations={},connections={},urls={:?}",
		config.threads, config.iterations, config.connections, config.urls
	)?;

	Ok(())
}

fn url_path(url: &String) -> Result<String, Error> {
	let url = bmw_deps::url::Url::parse(url)?;
	let path = match url.query() {
		Some(query) => format!("{}?{}", url.path(), query),
		None => url.path().to_string(),
	};
	Ok(path)
}

fn execute_thread(i: usize, config: &HttpHitConfig) -> Result<(), Error> {
	info!("Executing thread {}", i)?;

	let len = config.connections * config.urls.len();
	let mut url_hash = HashSet::new();
	let mut host = "".to_string();
	let mut port = 80;
	let mut tls = false;
	for url in &config.urls {
		let url_info = bmw_deps::url::Url::parse(url)?;
		host = match url_info.host() {
			Some(host) => host.to_string(),
			None => {
				return Err(err!(
					ErrKind::Http,
					format!("Host not specicifed for url: {}", url)
				));
			}
		};
		port = url_info.port().unwrap_or(80);
		tls = url_info.scheme().to_string() == "https";

		url_hash.insert((tls, host.clone(), port));
	}
	let mut connection = if url_hash.len() == 1 {
		// we have a single connection to make. Reuse for all requests
		let connection = http_connection!(Host(&host), Port(port), Tls(tls))?;
		Some(connection)
	} else {
		// different connections, so do not reuse connection.
		None
	};

	let mut requests = vec![];
	for url in &config.urls {
		for _ in 0..config.connections {
			let request = match connection {
				Some(ref _c) => {
					let path = url_path(&url)?;
					http_client_request!(Uri(&path))?
				}
				None => http_client_request!(Url(&url))?,
			};
			requests.push(request);
		}
	}

	for _ in 0..config.count {
		let count = lock_box!(0)?;
		let (tx, rx) = sync_channel(1);
		let mut count = count.clone();
		let tx = tx.clone();
		match connection {
			Some(ref mut connection) => {
				http_client_send!(requests.clone(), connection, {
					let mut count = count.wlock()?;
					let guard = count.guard();
					**guard += 1;
					//info!("guard={},len={},tid={}", **guard, len, i);
					if (**guard) == len {
						tx.send(())?;
					}
					Ok(())
				})?;
			}
			None => {
				http_client_send!(requests.clone(), {
					let mut count = count.wlock()?;
					let guard = count.guard();
					**guard += 1;
					if (**guard) == len {
						tx.send(())?;
					}
					Ok(())
				})?;
			}
		}
		rx.recv()?;
	}

	Ok(())
}

fn run_client(config: &HttpHitConfig) -> Result<(), Error> {
	let mut pool = thread_pool!(MaxSize(config.threads), MinSize(config.threads))?;
	pool.set_on_panic(move |_, e| -> Result<(), Error> {
		error!("thread panic: {:?}", e)?;
		Ok(())
	})?;

	let mut completions = vec![];
	for i in 0..config.threads {
		let config = config.clone();
		completions.push(execute!(pool, {
			http_client_init!(Threads(config.threads))?;
			for _ in 0..config.iterations {
				match execute_thread(i, &config) {
					Ok(_) => {}
					Err(e) => {
						error!("execute_thread generated error: {}", e)?;
					}
				}
			}

			Ok(())
		})?);
	}

	for completion in &completions {
		block_on!(completion);
	}

	Ok(())
}

fn main() -> Result<(), Error> {
	let yml = load_yaml!("httphit.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let config = load_config(args)?;
	show_startup_config(&config)?;
	let start = Instant::now();
	run_client(&config)?;

	let elapsed = start.elapsed().as_millis();
	let total =
		config.threads * config.connections * config.urls.len() * config.iterations * config.count;
	let qps = total as f64 * 1000 as f64 / elapsed as f64;
	info!("elapsed={},requests={},qps={}", elapsed, total, qps)?;

	Ok(())
}
