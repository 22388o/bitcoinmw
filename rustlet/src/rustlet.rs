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

use crate::types::{
	Rustlet, RustletContainer, RustletRequestImpl, RustletResponseImpl, RustletResponseState,
	WebSocketRequest, WebSocketRequestImpl,
};
use crate::{RustletConfig, RustletRequest, RustletResponse};
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use bmw_evh::WriteHandle;
use bmw_http::{
	Builder, HttpConfig, HttpContentReader, HttpHeaders, HttpInstance, HttpMethod, HttpVersion,
	WebSocketData, WebSocketHandle, WebSocketMessage,
};
use bmw_log::*;
use bmw_util::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread::{current, ThreadId};

const HTTP_NEW_LINE_BYTES: &[u8] = b"\r\n";
const HTTP_COMPLETE_BYTES: &[u8] = b"0\r\n\r\n";

info!();

thread_local!(
	pub static RUSTLET_CONTEXT: RefCell<(
					Option<(Box<dyn RustletRequest>,Box<dyn RustletResponse>)>,
					Option<Box<dyn WebSocketRequest>>
				)> = RefCell::new((None, None))
);

lazy_static! {
	pub static ref RUSTLET_CONTAINER: Arc<RwLock<HashMap<ThreadId, RustletContainer>>> =
		Arc::new(RwLock::new(HashMap::new()));
}

impl Default for RustletConfig {
	fn default() -> Self {
		Self {
			http_config: HttpConfig::default(),
		}
	}
}

impl RustletContainer {
	pub fn init(config: RustletConfig) -> Result<(), Error> {
		let mut container = RUSTLET_CONTAINER.write()?;
		(*container).insert(
			current().id(),
			RustletContainer {
				rustlets: HashMap::new(),
				rustlet_mappings: HashMap::new(),
				http_server: None,
				http_config: config.http_config,
			},
		);

		Ok(())
	}

	pub fn start(&mut self) -> Result<(), Error> {
		let tid = current().id();
		match self.http_server {
			Some(_) => Err(err!(
				ErrKind::IllegalState,
				"rustlet container has already been started"
			)),
			None => {
				let mut callback_mappings = HashSet::new();
				for key in self.rustlet_mappings.keys() {
					callback_mappings.insert(key.clone());
				}
				for instance in &mut self.http_config.instances {
					instance.callback = Some(Self::callback);
					instance.callback_mappings = callback_mappings.clone();
					instance.attachment = Box::new(tid);
				}
				let mut http_server = Builder::build_http_server(&self.http_config)?;
				http_server.start()?;
				self.http_server = Some(http_server);
				Ok(())
			}
		}
	}

	pub fn stop(&mut self) -> Result<(), Error> {
		match &mut self.http_server {
			Some(http_server) => http_server.stop(),
			None => Err(err!(ErrKind::Rustlet, "rustlet container was not started")),
		}
	}

	fn callback(
		headers: &HttpHeaders,
		_config: &HttpConfig,
		instance: &HttpInstance,
		write_handle: &mut WriteHandle,
		http_connection_data: HttpContentReader,
	) -> Result<(), Error> {
		let container = RUSTLET_CONTAINER.read()?;
		let tid = instance
			.attachment
			.clone()
			.downcast::<ThreadId>()
			.unwrap_or(Box::new(current().id()));
		debug!("tid={:?},current={:?}", tid, current().id())?;
		match (*container).get(&tid) {
			Some(rcd) => {
				let path = headers.path()?;
				debug!("in callback: {}", path)?;

				let rustlet_request = RustletRequestImpl::new(headers, http_connection_data)?;
				let rustlet_response = RustletResponseImpl::new(write_handle.clone())?;
				let rustlet_request: &mut Box<dyn RustletRequest> =
					&mut (Box::new(rustlet_request) as Box<dyn RustletRequest>);
				let rustlet_response: &mut Box<dyn RustletResponse> =
					&mut (Box::new(rustlet_response) as Box<dyn RustletResponse>);

				match rcd.rustlet_mappings.get(&path) {
					Some(name) => match rcd.rustlets.get(name) {
						Some(rustlet) => {
							debug!("found a rustlet")?;
							match (rustlet)(rustlet_request, rustlet_response) {
								Ok(_) => rustlet_response.complete(),
								Err(e) => Err(err!(
									ErrKind::Rustlet,
									format!("rustlet callback generated error: {}", e)
								)),
							}
						}
						None => Err(err!(
							ErrKind::Rustlet,
							format!("rustlet '{}' not found", name)
						)),
					},
					None => Err(err!(
						ErrKind::Rustlet,
						format!("rustlet mapping '{}' not found", path)
					)),
				}
			}
			None => Err(err!(ErrKind::Rustlet, "rustlet container not initialized")),
		}
	}

	pub fn add_rustlet(&mut self, name: &str, rustlet: Rustlet) -> Result<(), Error> {
		debug!("add rustlet name: {}", name)?;
		self.rustlets.insert(name.to_string(), rustlet);
		Ok(())
	}

	pub fn add_rustlet_mapping(&mut self, path: &str, name: &str) -> Result<(), Error> {
		debug!("add rustlet path: {} -> name: {}", path, name)?;
		self.rustlet_mappings
			.insert(path.to_string(), name.to_string());
		Ok(())
	}

	pub fn request(&self) -> Result<Box<dyn RustletRequest>, Error> {
		RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((request, _)) => Ok(request.clone()),
			None => Err(err!(ErrKind::Rustlet, "Could not find rustlet context")),
		})
	}

	pub fn response(&self) -> Result<Box<dyn RustletResponse>, Error> {
		RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((_, response)) => Ok(response.clone()),
			None => Err(err!(ErrKind::Rustlet, "Could not find rustlet context")),
		})
	}
}

impl RustletRequest for RustletRequestImpl {
	fn method(&self) -> &HttpMethod {
		&self.method
	}
	fn version(&self) -> &HttpVersion {
		&self.version
	}
	fn path(&self) -> &String {
		&self.path
	}
	fn query(&self) -> &String {
		&self.query
	}
	fn headers(&self) -> &Vec<(String, String)> {
		&self.headers
	}
	fn content_reader(&self) -> HttpContentReader {
		self.reader.clone()
	}
}

impl RustletRequestImpl {
	fn new(headers: &HttpHeaders, reader: HttpContentReader) -> Result<Self, Error> {
		Ok(Self {
			path: headers.path()?,
			query: headers.query()?,
			method: headers.method()?.clone(),
			version: headers.version()?.clone(),
			headers: headers.headers()?,
			reader,
		})
	}
}

impl RustletResponse for RustletResponseImpl {
	fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
		self.send_headers(bytes)?;
		Ok(())
	}
	fn flush(&mut self) -> Result<(), Error> {
		self.flush_impl(false)
	}
	fn set_async(&mut self) -> Result<(), Error> {
		wlock!(self.state).is_async = true;
		Ok(())
	}
	fn add_header(&mut self, name: &str, value: &str) -> Result<(), Error> {
		wlock!(self.state)
			.additional_headers
			.push((name.to_string(), value.to_string()));
		Ok(())
	}
	fn set_content_type(&mut self, value: &str) -> Result<(), Error> {
		if rlock!(self.state).sent_headers {
			Err(err!(
				ErrKind::Rustlet,
				"Cannot call set_content_type after headers have been sent"
			))
		} else {
			wlock!(self.state).content_type = value.to_string();
			Ok(())
		}
	}
	fn set_connection_close(&mut self) -> Result<(), Error> {
		if rlock!(self.state).sent_headers {
			Err(err!(
				ErrKind::Rustlet,
				"Cannot call set_connection_close after headers have been sent"
			))
		} else {
			wlock!(self.state).close = true;
			Ok(())
		}
	}
	fn redirect(&mut self, url: &str) -> Result<(), Error> {
		if rlock!(self.state).sent_headers {
			Err(err!(
				ErrKind::Rustlet,
				"Cannot call set_connection_close after headers have been sent"
			))
		} else {
			wlock!(self.state).redirect = Some(url.to_string());
			Ok(())
		}
	}
	fn async_complete(&mut self) -> Result<(), Error> {
		wlock!(self.state).is_async = false;
		self.complete_impl()
	}
	fn complete(&mut self) -> Result<(), Error> {
		self.complete_impl()
	}
}

impl RustletResponseImpl {
	fn new(wh: WriteHandle) -> Result<Self, Error> {
		Ok(RustletResponseImpl {
			wh,
			state: lock_box!(RustletResponseState {
				sent_headers: false,
				completed: false,
				close: false,
				content_type: "text/html".to_string(),
				buffer: vec![],
				redirect: None,
				additional_headers: vec![],
				is_async: false,
			})?,
		})
	}

	fn send_headers(&mut self, bytes: &[u8]) -> Result<(), Error> {
		debug!("send headers")?;

		let bytes_len = bytes.len();
		let mut state = self.state.wlock()?;
		let guard = state.guard();

		let (close_text, content_type_text, sent_headers) = {
			let close_text = if (**guard).close {
				"Connection: close\r\n"
			} else {
				"Connecction: keep-alive\r\n"
			};
			let content_type_text = format!("Content-Type: {}\r\n", (**guard).content_type);
			let sent_headers = (**guard).sent_headers;

			(close_text, content_type_text, sent_headers)
		};

		if !sent_headers {
			let mut additional_header_str = "".to_string();
			for (name, value) in &(**guard).additional_headers {
				additional_header_str = format!("{}{}: {}\r\n", additional_header_str, name, value);
			}

			match (**guard).redirect.clone() {
				Some(redirect) => {
					if bytes_len > 0 {
						return Err(err!(
							ErrKind::Rustlet,
							"cannot redirect a url and also write content"
						));
					}
					(**guard).buffer.extend(
						format!(
							"HTTP/1.1 302 Found{}\r\n{}Location: {}\r\n\r\n",
							additional_header_str, close_text, redirect,
						)
						.as_bytes(),
					);
				}
				None => {
					(**guard).buffer.extend(
						format!(
							"HTTP/1.1 200 OK\r\n{}{}{}Transfer-Encoding: chunked\r\n\r\n",
							additional_header_str, close_text, content_type_text,
						)
						.as_bytes(),
					);
				}
			}

			(**guard).additional_headers.clear();
			(**guard).additional_headers.shrink_to_fit();
		}

		if bytes_len > 0 {
			let msglen = format!("{:X}\r\n", bytes_len);
			(**guard).buffer.extend(&msglen.as_bytes()[..]);
			(**guard).buffer.extend(bytes);
			(**guard).buffer.extend(HTTP_NEW_LINE_BYTES);
		}

		(**guard).sent_headers = true;

		Ok(())
	}

	fn complete_impl(&mut self) -> Result<(), Error> {
		if rlock!(self.state).is_async {
			// we'er async we don't complete in the normal way
			return Ok(());
		}
		self.send_headers(&[])?;
		let close = {
			let mut state = self.state.wlock()?;
			let guard = state.guard();
			let completed = (**guard).completed;
			let close = (**guard).close;
			debug!("in complete")?;
			if !completed {
				debug!("has not completed yet")?;
				(**guard).buffer.extend(HTTP_COMPLETE_BYTES);
				(**guard).completed = true;
			}
			close
		};

		self.flush_impl(true)?;

		if close {
			self.wh.close()?;
		}
		Ok(())
	}

	fn flush_impl(&mut self, shrink: bool) -> Result<(), Error> {
		let mut state = self.state.wlock()?;
		let guard = state.guard();
		self.wh.write(&(**guard).buffer)?;
		(**guard).buffer.clear();
		if shrink {
			(**guard).buffer.shrink_to_fit();
		}

		Ok(())
	}
}

impl WebSocketRequest for WebSocketRequestImpl {
	fn handle(&self) -> Result<WebSocketHandle, Error> {
		todo!()
	}
	fn message(&mut self) -> Result<WebSocketMessage, Error> {
		todo!()
	}
	fn data(&self) -> Result<WebSocketData, Error> {
		todo!()
	}
}
