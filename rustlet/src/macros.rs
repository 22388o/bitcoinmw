// Copyright (c) 2023, The BitcoinMW Developers
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

use bmw_log::*;

info!();

#[macro_export]
macro_rules! rustlet_init {
	($config:expr) => {{
		match bmw_rustlet::RustletContainer::init($config) {
			Ok(_) => Ok(()),
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not initialize rustlet container due to: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet_start {
	() => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.start()?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to start rustlet container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet {
	($name:expr, $code:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.add_rustlet(
					$name,
					Box::pin(
						move |request: &mut Box<dyn bmw_rustlet::RustletRequest>,
						      response: &mut Box<dyn bmw_rustlet::RustletResponse>| {
							bmw_rustlet::RUSTLET_CONTEXT.with(|f| {
								*f.borrow_mut() =
									(Some(((*request).clone(), (*response).clone())), None);
							});
							{
								$code
							}
							Ok(())
						},
					),
				)?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to add rustlet to container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet_mapping {
	($path:expr, $name:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut containers) => match ((*containers).get_mut(&std::thread::current().id())) {
				Some(container) => container.add_rustlet_mapping($path, $name),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain request for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to get request from container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! request {
	() => {{
		RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((request, _)) => Ok(request.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::Rustlet,
				"Could not find rustlet context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! response {
	() => {{
		bmw_rustlet::RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((_, response)) => Ok(response.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::Rustlet,
				"Could not find rustlet context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! websocket {
	() => {};
}

/// Returns [`crate::WebSocketRequest`].
#[macro_export]
macro_rules! websocket_request {
	() => {};
}

/// Three params: name, uri, [protocol list]
#[macro_export]
macro_rules! websocket_mapping {
	() => {};
}

#[macro_export]
macro_rules! session {
	// TODO: session will have CRUD for session. SessionOp::Set, SessionOp::Get,
	// SessionOp::Delete
	() => {};
}

#[cfg(test)]
mod test {
	use crate as bmw_rustlet;
	use bmw_http::{HttpConfig, HttpInstance, HttpInstanceType, HttpMethod, PlainConfig};
	use bmw_rustlet::*;
	use bmw_test::port::pick_free_port;
	use bmw_test::testdir::{setup_test_dir, tear_down_test_dir};
	use std::collections::HashMap;
	use std::io::Read;
	use std::io::Write;
	use std::net::TcpStream;
	use std::thread::{current, sleep};
	use std::time::Duration;

	debug!();

	#[test]
	fn test_rustlet_macros() -> Result<(), Error> {
		info!("1tid={:?}", current().id())?;
		rustlet_init!(RustletConfig::default())?;
		rustlet!("test", {})?;
		rustlet_mapping!("/abc", "test")?;
		rustlet_start!()?;
		Ok(())
	}

	#[test]
	fn test_rustlet_simple_request() -> Result<(), Error> {
		info!("2tid={:?}", current().id())?;
		let port = pick_free_port()?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let test_dir = ".test_rustlet_simple_request.bmw";
		setup_test_dir(test_dir)?;
		let rc = RustletConfig {
			http_config: HttpConfig {
				instances: vec![HttpInstance {
					port,
					instance_type: HttpInstanceType::Plain(PlainConfig {
						http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
					}),
					..Default::default()
				}],
				..Default::default()
			},
			..Default::default()
		};
		rustlet_init!(rc)?;
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
		rustlet!("test2", {
			let mut response = response!()?;
			let request = request!()?;
			info!(
				"in rustlet test_rustlet_simple_request test2 method={:?},path={}",
				request.method(),
				request.path()
			)?;
			assert_eq!(request.method(), HttpMethod::GET);
			response.write(b"def")?;
		})?;
		rustlet_mapping!("/abc", "test")?;
		rustlet_mapping!("/def", "test2")?;
		rustlet_start!()?;
		sleep(Duration::from_millis(1_000));

		info!("connection to port {}", port)?;
		let mut client = TcpStream::connect(addr)?;

		client.write(b"GET /abc?a=1 HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
		sleep(Duration::from_millis(1_000));
		client.write(b"GET /def?a=1 HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
		sleep(Duration::from_millis(1_000));

		let mut buf = [0u8; 100];
		let len = client.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"abcdef");

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
