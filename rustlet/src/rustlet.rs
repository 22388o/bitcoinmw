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
	Rustlet, RustletContainer, RustletContext, RustletRequestImpl, RustletResponseImpl,
	RustletResponseState, WebSocketRequest, WebSocketRequestImpl,
};
use crate::{RustletConfig, RustletContainerConfig, RustletRequest, RustletResponse};
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use bmw_evh::{ThreadContext, WriteHandle, WriteState};
use bmw_http::HttpInstanceType::Plain;
use bmw_http::HttpInstanceType::Tls;
use bmw_http::{
	Builder, HttpConfig, HttpContentReader, HttpHeaders, HttpInstance, HttpMethod, HttpVersion,
	WebSocketData, WebSocketHandle, WebSocketMessage,
};
use bmw_log::*;
use bmw_util::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::str::from_utf8;
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
			rustlet_config: RustletContainerConfig::default(),
		}
	}
}

impl Default for RustletContainerConfig {
	fn default() -> Self {
		Self {
			main_log_file: None,
		}
	}
}

impl RustletContext {
	fn new(_config: &RustletContainerConfig) -> Result<Self, Error> {
		let slab_allocator = slab_allocator!()?;
		let mut list =
			bmw_util::Builder::build_list(ListConfig::default(), &Some(&slab_allocator))?;
		list.push(pattern!(Regex("$(@"), Id(0))?)?;
		let suffix_tree = Box::new(suffix_tree!(
			list,
			TerminationLength(1_000_000_000),
			MaxWildcardLength(1_000)
		)?);
		let matches = [bmw_util::Builder::build_match_default(); 1_000];
		Ok(RustletContext {
			suffix_tree,
			matches,
		})
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
				config,
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
				let mut callback_extensions = HashSet::new();
				for key in self.rustlet_mappings.keys() {
					callback_mappings.insert(key.clone());
				}
				callback_extensions.insert("rsp".to_string());
				for instance in &mut self.config.http_config.instances {
					instance.callback = Some(Self::callback);
					instance.callback_mappings = callback_mappings.clone();
					instance.callback_extensions = callback_extensions.clone();
					instance.attachment = Box::new(tid);
				}
				self.config.http_config.attachment =
					Some(Box::new(self.config.rustlet_config.clone()));
				let mut http_server = Builder::build_http_server(&self.config.http_config)?;
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

	fn build_ctx<'a>(
		ctx: &'a mut ThreadContext,
		config: &HttpConfig,
	) -> Result<&'a mut RustletContext, Error> {
		match ctx.user_data.downcast_ref::<RustletContext>() {
			Some(_) => {}
			None => {
				ctx.user_data = Box::new(Self::build_rustlet_context(config)?);
			}
		}

		Ok(ctx.user_data.downcast_mut::<RustletContext>().unwrap())
	}

	fn build_rustlet_context(config: &HttpConfig) -> Result<RustletContext, Error> {
		match &config.attachment {
			Some(attachment) => match attachment.downcast_ref::<RustletContainerConfig>() {
				Ok(rustlet_container_config) => {
					debug!(
						"returning rustlet context based on config: {:?}",
						rustlet_container_config
					)?;
					RustletContext::new(rustlet_container_config)
				}
				Err(e) => Err(err!(
					ErrKind::Rustlet,
					format!(
						"could not obtain RustletContainerConfig from attachment due to: {}",
						e
					)
				)),
			},
			None => Err(err!(
				ErrKind::Rustlet,
				"could not obtain attachment from HttpConfig"
			)),
		}
	}

	fn callback(
		headers: &HttpHeaders,
		config: &HttpConfig,
		instance: &HttpInstance,
		write_handle: &mut WriteHandle,
		http_connection_data: HttpContentReader,
		write_state: Box<dyn LockBox<WriteState>>,
		thread_context: &mut ThreadContext,
	) -> Result<bool, Error> {
		let ctx = Self::build_ctx(thread_context, config)?;
		match Self::callback_impl(
			headers,
			config,
			instance,
			write_handle,
			http_connection_data,
			write_state,
			ctx,
		) {
			Ok(v) => Ok(v),
			Err(e) => {
				let content = format!("An error occurred while processing page: {}.", e);
				let clen = content.len();
				write_handle.write(
					format!(
						"HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\r\n{}",
						clen, content
					)
					.as_bytes(),
				)?;

				Ok(false)
			}
		}
	}

	fn execute_rustlet(
		ctx: &mut RustletContext,
		write_handle: &mut WriteHandle,
		rustlet_request: &mut Box<dyn RustletRequest>,
		rustlet_response: RustletResponseImpl,
		path: &String,
		host: &String,
		instance: &HttpInstance,
		rcd: &RustletContainer,
	) -> Result<bool, Error> {
		debug!("execute {}. depth={}", path, rustlet_response.depth)?;
		match rcd.rustlet_mappings.get(path) {
			Some(name) => match rcd.rustlets.get(name) {
				Some(rustlet) => {
					debug!("found a rustlet")?;
					let rustlet_response: &mut Box<dyn RustletResponse> =
						&mut (Box::new(rustlet_response) as Box<dyn RustletResponse>);
					match (rustlet)(rustlet_request, rustlet_response) {
						Ok(_) => {
							let is_async = rustlet_response.complete()?;
							Ok(is_async)
						}
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
			None => {
				let default = ".".to_string();
				let base_dir = match &instance.instance_type {
					Plain(config) => config
						.http_dir_map
						.get(host)
						.unwrap_or(config.http_dir_map.get("*").unwrap_or(&default))
						.clone(),
					Tls(config) => config
						.http_dir_map
						.get(host)
						.unwrap_or(config.http_dir_map.get("*").unwrap_or(&default))
						.clone(),
				};
				let rsp_path = match canonicalize_base_path(&base_dir, &path) {
					Ok(rsp_path) => rsp_path,
					Err(e) => {
						return Err(err!(
							ErrKind::Rustlet,
							format!("rsp: '{}{}' not found due to: {}", base_dir, path, e)
						));
					}
				};

				Self::execute_rsp(
					rsp_path,
					write_handle,
					ctx,
					rustlet_request,
					rustlet_response,
					host,
					instance,
					rcd,
				)?;

				Ok(false)
			}
		}
	}

	fn callback_impl(
		headers: &HttpHeaders,
		_config: &HttpConfig,
		instance: &HttpInstance,
		write_handle: &mut WriteHandle,
		http_connection_data: HttpContentReader,
		write_state: Box<dyn LockBox<WriteState>>,
		ctx: &mut RustletContext,
	) -> Result<bool, Error> {
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
				let host = headers.host()?;
				debug!("in callback: {}", path)?;

				let rustlet_request = RustletRequestImpl::new(headers, http_connection_data)?;
				let rustlet_response = RustletResponseImpl::new(write_handle.clone(), write_state)?;
				let rustlet_request: &mut Box<dyn RustletRequest> =
					&mut (Box::new(rustlet_request) as Box<dyn RustletRequest>);

				Self::execute_rustlet(
					ctx,
					write_handle,
					rustlet_request,
					rustlet_response,
					&path,
					host,
					instance,
					rcd,
				)
			}
			None => Err(err!(ErrKind::Rustlet, "rustlet container not initialized")),
		}
	}

	fn execute_rsp(
		rsp_path: String,
		write_handle: &mut WriteHandle,
		ctx: &mut RustletContext,
		request: &mut Box<dyn RustletRequest>,
		mut response: RustletResponseImpl,
		host: &String,
		instance: &HttpInstance,
		rcd: &RustletContainer,
	) -> Result<(), Error> {
		let mut rsp_text = String::new();
		let mut file = File::open(rsp_path.clone())?;
		file.read_to_string(&mut rsp_text)?;
		let rsp_bytes = rsp_text.as_bytes();
		let rsp_bytes_len = rsp_bytes.len();

		let count = ctx.suffix_tree.tmatch(&rsp_bytes, &mut ctx.matches)?;

		let mut itt = 0;
		for i in 0..count {
			let start = ctx.matches[i].start() + 3;
			let mut end = start;
			loop {
				if end >= rsp_bytes_len || rsp_bytes[end] == b')' {
					break;
				}

				end += 1;
			}
			if end >= rsp_bytes_len {
				return Err(err!(
					ErrKind::Rustlet,
					format!(
						"RSP: {} generated error: unterminated tag in RSP",
						&rsp_path
					)
				));
			}
			let path = from_utf8(&rsp_bytes[start..end])?.to_string();
			response.write(&rsp_bytes[itt..start.saturating_sub(3)])?;
			response.depth += 1;
			Self::execute_rustlet(
				ctx,
				write_handle,
				request,
				response.clone(),
				&path,
				host,
				instance,
				rcd,
			)?;
			response.depth -= 1;
			itt = end + 1;
			debug!("name='{}'", path)?;
		}
		response.write(&rsp_bytes[itt..])?;

		response.complete()?;

		Ok(())
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
	fn header_host(&self) -> &String {
		&self.header_host
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
			header_host: headers.host()?.to_string(),
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
		wlock!(self.write_state).set_async(true);
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
		self.complete_impl(true)?;
		Ok(())
	}
	fn complete(&mut self) -> Result<bool, Error> {
		self.complete_impl(false)
	}
}

impl RustletResponseImpl {
	fn new(wh: WriteHandle, write_state: Box<dyn LockBox<WriteState>>) -> Result<Self, Error> {
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
			write_state,
			depth: 0,
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

	fn complete_impl(&mut self, async_complete: bool) -> Result<bool, Error> {
		if rlock!(self.state).is_async || self.depth > 0 {
			// we're async we don't complete in the normal way
			// also if not the lowest depth, we don't complete
			return Ok(true);
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
		} else if async_complete {
			wlock!(self.write_state).set_async(false);
			self.wh.trigger_on_read()?;
		}
		Ok(false)
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
