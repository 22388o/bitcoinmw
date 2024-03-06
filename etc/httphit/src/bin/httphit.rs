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
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::sync_channel;
use std::thread::sleep;
use std::time::{Duration, Instant};

info!();

const CONTENT_LENGTH_BYTES: &[u8] = b"Content-Length: ";
const TRANSFER_ENCODING_CHUNKED: &[u8] = b"Transfer-Encoding: chunked\r\n";

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
	let mut path = "/".to_string();
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
		path = url_info.path().to_string();

		url_hash.insert((tls, host.clone(), port));
	}

	if url_hash.len() != 1 {
		return Err(err!(
			ErrKind::Http,
			"Httphit does not support multiple host/port/schemes at this time"
		));
	}

	let mut buf = [0u8; 100_000];

	let addr = format!("{}:{}", host, port);
	let mut stream = TcpStream::connect(addr)?;
	let req_str = format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host);

	for _ in 0..config.count {
		stream.write(req_str.as_bytes())?;

		let mut len_sum = 0;
		let mut close = false;

		loop {
			// append data to our buffer
			let len = stream.read(&mut buf[len_sum..])?;
			debug!("len={}", len)?;

			if len <= 0 {
				close = true;
				break;
			}

			len_sum += len;

			debug!(
				"read data loop = '{}'",
				std::str::from_utf8(&buf[0..len_sum]).unwrap_or("utf8err")
			)?;

			if find_page(&buf[0..len_sum])? {
				break;
			}
		}

		debug!("page complete")?;
	}

	Ok(())
}

fn find_page(buf: &[u8]) -> Result<bool, Error> {
	let len = buf.len();
	let mut headers_complete = false;
	let mut headers_completion_point = 0;
	let mut content_len = 0usize;
	let mut is_chunked = false;
	for i in 4..len {
		if &buf[i - 4..i] == b"\r\n\r\n" {
			debug!("headers complete at {}", i)?;
			headers_completion_point = i;
			headers_complete = true;
			break;
		}
		let clen = CONTENT_LENGTH_BYTES.len();
		if i >= clen && &buf[i - clen..i] == CONTENT_LENGTH_BYTES {
			debug!("found contentlen at {}", i)?;
			content_len = read_len_to_nl(&buf[i..])?.0;
		}
		let clen = TRANSFER_ENCODING_CHUNKED.len();
		if i >= clen && &buf[i - clen..i] == TRANSFER_ENCODING_CHUNKED {
			is_chunked = true;
			debug!("found chunks at {}", i)?;
		}
	}

	let target_end = headers_completion_point + content_len;
	debug!(
		"headers_complete={},len={},is_chunked={},content_len={},headers_completion_point={},target_end={}",
		headers_complete, len, is_chunked, content_len, headers_completion_point, target_end
	)?;

	if headers_complete && !is_chunked && target_end >= len {
		Ok(true)
	} else if !is_chunked {
		Ok(false)
	} else if !headers_complete {
		Ok(false)
	} else {
		let mut itt = headers_completion_point;
		loop {
			if itt > buf.len() {
				return Ok(false);
			}
			let s = skip_chunk(&buf[itt..])?;
			if s == usize::MAX {
				debug!("page not ready")?;
				return Ok(false);
			} else if s == 0 {
				debug!("page ready")?;
				return Ok(true);
			}

			itt += s;
		}
	}
}

fn skip_chunk(buf: &[u8]) -> Result<usize, Error> {
	match read_len_to_nl(buf) {
		Ok((len, offset)) => {
			if len == 0 {
				Ok(0)
			} else {
				Ok(len + offset)
			}
		}
		Err(_) => Ok(usize::MAX),
	}
}

fn read_len_to_nl(buf: &[u8]) -> Result<(usize, usize), Error> {
	let len = buf.len();
	let mut end = usize::MAX;
	for i in 0..len {
		if buf[i] == b'\r' || buf[i] == b'\n' {
			end = i;
			break;
		}
	}

	if end <= len {
		let rlen =
			usize::from_str_radix(std::str::from_utf8(&buf[0..end]).unwrap_or("utf8err"), 16)?;
		Ok((rlen, end + 4))
	} else {
		Err(err!(
			ErrKind::Http,
			"could not read a length from specified location. Invalid http response"
		))
	}
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
