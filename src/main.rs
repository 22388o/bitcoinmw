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

use bmw_err::{err, ErrKind, Error};
use bmw_evh::{EventHandlerConfig, ThreadContext, WriteHandle, WriteState};
use bmw_http::HttpInstanceType::Plain;
use bmw_http::PlainConfig;
use bmw_http::{
	Builder, HttpConfig, HttpContentReader, HttpHeaders, HttpInstance, WebSocketData,
	WebSocketHandle, WebSocketMessage, WebSocketMessageType,
};
use bmw_log::*;
use bmw_util::*;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::mem::size_of;
#[cfg(not(test))]
use std::thread::park;

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(not(tarpaulin_include))] // temporary test function, will be removed
fn callback(
	_headers: &HttpHeaders,
	_config: &HttpConfig,
	_instance: &HttpInstance,
	write_handle: &mut WriteHandle,
	mut http_connection_data: HttpContentReader,
	_write_state: Box<dyn LockBox<WriteState>>,
	_thread_context: &mut ThreadContext,
) -> Result<bool, Error> {
	info!("in callback!")?;

	let mut buf = [0; 10];

	info!("start read")?;
	loop {
		let len_read = http_connection_data.read(&mut buf[0..10])?;
		info!(
			"len_read={},data='{}'",
			len_read,
			std::str::from_utf8(&buf[0..len_read]).unwrap_or("utf8err")
		)?;
		if len_read == 0 {
			break;
		}
	}
	info!("end read")?;

	write_handle.write(
		"\
HTTP/1.1 200 OK\r\n\
Date: Thu, 12 Oct 2023 22:52:16 GMT\r\n\
Content-Length: 7\r\n\
\r\n\
callbk\n"
			.as_bytes(),
	)?;

	Ok(false)
}

#[cfg(not(tarpaulin_include))] // temporary test function, will be removed
fn ws_handler(
	message: &WebSocketMessage,
	_config: &HttpConfig,
	_instance: &HttpInstance,
	wsh: &mut WebSocketHandle,
	websocket_data: &WebSocketData,
) -> Result<(), Error> {
	let text = std::str::from_utf8(&message.payload[..]).unwrap_or("utf8err");
	info!(
		"in ws handler in main.rs. got message [proto='{:?}'] [path='{}'] [query='{}'] = '{}'",
		websocket_data.negotiated_protocol, websocket_data.uri, websocket_data.query, text
	)?;

	let wsm = WebSocketMessage {
		mtype: WebSocketMessageType::Text,
		payload: "abcd".as_bytes().to_vec(),
	};
	wsh.send_message(&wsm, false)?;
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

	let mut websocket_mappings = HashMap::new();
	let mut test_ws_protos = HashSet::new();
	test_ws_protos.insert("test".to_string());
	test_ws_protos.insert("testv2".to_string());
	websocket_mappings.insert("/chat".to_string(), HashSet::new());
	websocket_mappings.insert("/testws".to_string(), test_ws_protos);

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
			instance_type: Plain(PlainConfig {
				http_dir_map: HashMap::from([
					("*".to_string(), "~/.bmw/www".to_string()),
					("37miners.com".to_string(), "~/abc".to_string()),
				]),
			}),
			websocket_handler: Some(ws_handler),
			websocket_mappings,
			..Default::default()
		}],
		server_version: built_info::PKG_VERSION.to_string(),
		debug: false,
		evh_config: EventHandlerConfig {
			threads: 2,
			..Default::default()
		},
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
