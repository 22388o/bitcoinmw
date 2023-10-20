// Copyright (c) 2023, The BitcoinMW Developers
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

use bmw_err::{err, ErrKind, Error};
use bmw_evh::{ConnData, ConnectionData};
use bmw_http::{Builder, HttpConfig, HttpHeaders, HttpInstance};
use bmw_log::*;
use std::collections::HashSet;
use std::mem::size_of;
#[cfg(not(test))]
use std::thread::park;

info!();

fn callback(
	_headers: &HttpHeaders,
	_config: &HttpConfig,
	_instance: &HttpInstance,
	connection_data: &mut ConnectionData,
) -> Result<(), Error> {
	info!("in callback!")?;

	connection_data.write_handle().write(
		"\
HTTP/1.1 200 OK\r\n\
Date: Thu, 12 Oct 2023 22:52:16 GMT\r\n\
Content-Length: 7\r\n\
\r\n\
callbk\n"
			.as_bytes(),
	)?;

	Ok(())
}

fn main() -> Result<(), Error> {
	real_main(false)?;
	Ok(())
}

fn real_main(debug_startup_32: bool) -> Result<(), Error> {
	// ensure we only support 64 bit
	match size_of::<&char>() == 8 && debug_startup_32 == false {
		true => {}
		false => return Err(err!(ErrKind::IllegalState, "Only 64 bit arch supported")),
	}

	log_init!(LogConfig {
		show_bt: ShowBt(true),
		show_millis: ShowMillis(true),
		line_num: LineNum(false),
		level: Level(false),
		..Default::default()
	})?;

	let port = 8080;
	let mut callback_mappings = HashSet::new();
	let mut callback_extensions = HashSet::new();
	callback_mappings.insert("/callbacktest".to_string());
	callback_extensions.insert("rsp".to_string());
	let config = HttpConfig {
		instances: vec![HttpInstance {
			port,
			//addr: "[::1]".to_string(), // ipv6
			addr: "127.0.0.1".to_string(), // ipv4
			callback_mappings,
			callback_extensions,
			callback: Some(callback),
			..Default::default()
		}],
		debug: true,
		..Default::default()
	};

	let mut server = Builder::build_http_server(&config)?;
	server.start()?;
	info!("listener on port {}", port)?;

	#[cfg(not(test))]
	park();

	Ok(())
}

#[cfg(test)]
mod test {
	use crate::{main, real_main};
	use bmw_err::Error;

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
