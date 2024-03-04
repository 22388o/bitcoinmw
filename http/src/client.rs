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

use crate::constants::*;
use crate::types::{
	HttpClientAttachment, HttpClientAttachmentData, HttpClientContext, HttpClientImpl,
	HttpConnectionImpl, HttpContentReaderData, HttpRequestImpl, HttpResponseImpl,
};
use crate::{
	HttpClient, HttpClientConfig, HttpClientContainer, HttpConnection, HttpConnectionConfig,
	HttpContentReader, HttpHandler, HttpMethod, HttpRequest, HttpRequestConfig, HttpResponse,
	HttpVersion,
};
use bmw_deps::dirs;
use bmw_deps::lazy_static::lazy_static;
use bmw_deps::rand::random;
use bmw_deps::url::Url;
use bmw_err::*;
use bmw_evh::{
	tcp_stream_to_handle, AttachmentHolder, Builder, ClientConnection, ConnData, ConnectionData,
	EventHandler, EventHandlerConfig, EventHandlerController, EventHandlerData, ThreadContext,
	TlsClientConfig, WriteHandle, READ_SLAB_DATA_SIZE,
};
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Formatter};
use std::fs::{canonicalize, create_dir_all, metadata, File};
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::str::from_utf8;
use std::sync::{Arc, RwLock};
use std::thread::{current, ThreadId};

info!();

thread_local! {
		pub static HTTP_CLIENT_CONTEXT: RefCell<Option<(Box<dyn HttpRequest + Send + Sync>,Box<dyn HttpResponse + Send + Sync>)>> = RefCell::new(None);

}

lazy_static! {
	pub static ref HTTP_CLIENT_CONTAINER: Arc<RwLock<HashMap<ThreadId, Box<dyn HttpClient + Send + Sync>>>> =
		Arc::new(RwLock::new(HashMap::new()));
}

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

impl fmt::Display for Box<dyn HttpRequest + Send + Sync> {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self.request_url() {
			Some(url) => write!(f, "url={}", url)?,
			None => {}
		}
		match self.request_uri() {
			Some(uri) => write!(f, "uri={}", uri)?,
			None => {}
		}
		Ok(())
	}
}

impl fmt::Display for Box<dyn HttpResponse + Send + Sync> {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(
			f,
			"response_code={},status_text={},version={}",
			self.code().unwrap_or(0),
			self.status_text().unwrap_or(&"".to_string()),
			self.version().unwrap_or(&HttpVersion::UNKNOWN),
		)?;

		match self.headers() {
			Ok(headers) => {
				for header in headers {
					write!(f, "\nheader['{}']: '{}'", header.0, header.1)?;
				}
				write!(f, "\n")?;
			}
			Err(_) => {}
		}

		Ok(())
	}
}

impl HttpClientConfig {
	fn tmp_file_dir(&self) -> PathBuf {
		let mut file_dir = PathBuf::from(self.base_dir.clone());
		file_dir.push("tmp_files");
		file_dir
	}
}

impl Default for HttpConnectionConfig {
	fn default() -> Self {
		Self {
			host: "127.0.0.1".to_string(),
			port: 80,
			tls: false,
			full_chain_cert_file: None,
		}
	}
}

impl Default for HttpClientConfig {
	fn default() -> Self {
		Self {
			max_headers_len: 8192,
			debug: false,
			max_handles_per_thread: 100,
			threads: 4,
			slab_count: 100,
			base_dir: "~/.bitcoinmw".to_string(),
		}
	}
}

impl Default for HttpRequestConfig {
	fn default() -> Self {
		Self {
			request_url: None,
			request_uri: None,
			user_agent: format!("BitcoinMW/{}", built_info::PKG_VERSION.to_string()).to_string(),
			accept: "*/*".to_string(),
			headers: vec![],
			timeout_millis: 0,
			method: HttpMethod::GET,
			version: HttpVersion::HTTP11,
			full_chain: None,
			content_file: None,
			content_data: None,
			keep_alive: true,
		}
	}
}

fn do_send(
	request: Box<dyn HttpRequest + Send + Sync>,
	wh: &mut WriteHandle,
	uri: String,
	addr: String,
	keep_alive: bool,
) -> Result<(), Error> {
	let content_file = request.content_file();
	let content_data = request.content_data();
	if content_file.is_some() && content_data.is_some() {
		return Err(err!(
			ErrKind::IllegalArgument,
			"a request may not have both content_file and content_data specified"
		));
	}
	let user_agent = request.user_agent();
	let accept = request.accept();
	let headers = request.headers();
	let mut headers_str = "".to_string();
	for header in headers {
		headers_str = format!("{}\r\n{}: {}", headers_str, header.0, header.1);
	}
	let keep_alive = match keep_alive {
		true => "keep-alive",
		false => "close",
	};

	let method = request.method().to_string();
	let version = request.version().to_string();

	let content_length_header = match content_file {
		Some(content_file) => {
			format!("Content-Length: {}\r\n", metadata(content_file)?.len())
		}
		None => match content_data {
			Some(content_data) => {
				if content_data.len() == 0 {
					"".to_string()
				} else {
					format!("Content-Length: {}\r\n", content_data.len())
				}
			}
			None => "".to_string(),
		},
	};

	let req_str = format!(
		"{} {} {}\r\nHost: {}\r\nUser-Agent: {}\r\nAccept: {}\r\n{}Connection: {}{}\r\n\r\n",
		method,
		uri,
		version,
		addr,
		user_agent,
		accept,
		content_length_header,
		keep_alive,
		headers_str
	);

	wh.write(req_str.as_bytes())?;

	match content_data {
		Some(content_data) => {
			wh.write(content_data)?;
		}
		None => {}
	}

	match content_file {
		Some(content_file) => {
			let mut file = File::open(content_file)?;
			let mut buf = [0u8; 1024];
			loop {
				let len = file.read(&mut buf)?;

				if len == 0 {
					break;
				}

				wh.write(&buf[0..len])?;
			}
		}
		None => {}
	}

	Ok(())
}

impl HttpClientContainer {
	pub fn init(config: &HttpClientConfig) -> Result<(), Error> {
		let mut container = HTTP_CLIENT_CONTAINER.write()?;
		(*container).insert(current().id(), crate::Builder::build_http_client(&config)?);
		Ok(())
	}

	pub fn stop() -> Result<(), Error> {
		let id = current().id();
		let mut container = HTTP_CLIENT_CONTAINER.write()?;
		match (*container).remove(&id) {
			Some(mut http_client) => http_client.stop(),
			None => Err(err!(
				ErrKind::IllegalState,
				"http_client_init has not been called for this thread and stop was called!"
			)),
		}
	}
}

impl HttpClient for HttpClientImpl {
	fn send(
		&mut self,
		request: Box<dyn HttpRequest + Send + Sync>,
		handler: HttpHandler,
	) -> Result<(), Error> {
		match request.request_url() {
			Some(request_url) => {
				let url = Url::parse(&request_url)?;

				let scheme = url.scheme();
				let host = match url.host_str() {
					Some(host) => host,
					None => {
						return Err(err!(
							ErrKind::Http,
							"no host specified in the request_url: {}"
						));
					}
				};
				let port = url.port().unwrap_or(80);

				let mut uri = url.path().to_string();
				let query = url.query().unwrap_or("");
				if query.len() > 0 {
					uri = format!("{}?{}", uri, query);
				}
				let addr = format!("{}:{}", host, port);
				let tcp_stream = TcpStream::connect(addr.clone())?;
				tcp_stream.set_nonblocking(true)?;

				let client_connection = ClientConnection {
					handle: tcp_stream_to_handle(tcp_stream)?,
					tls_config: if scheme == "https" {
						Some(TlsClientConfig {
							sni_host: host.to_string(),
							trusted_cert_full_chain_file: request.full_chain().clone(),
						})
					} else if scheme == "http" {
						None
					} else {
						return Err(err!(ErrKind::Http, "scheme must be either http or https"));
					},
				};

				let mut vec = VecDeque::new();
				vec.push_back(HttpClientAttachmentData {
					request: request.clone(),
					close_on_complete: true,
					handler: handler,
				});

				let mut wh = self.controller.add_client(
					client_connection,
					Box::new(HttpClientAttachment {
						http_client_data: lock_box!(vec)?,
					}),
				)?;
				do_send(request, &mut wh, uri, addr, false)
			}
			None => Err(err!(ErrKind::Http, "request_url must be specified")),
		}
	}

	fn stop(&mut self) -> Result<(), Error> {
		self.do_stop()
	}

	fn controller(&mut self) -> &mut EventHandlerController {
		&mut self.controller
	}
}

impl HttpClientImpl {
	pub(crate) fn new(config: &HttpClientConfig) -> Result<HttpClientImpl, Error> {
		let mut config = config.clone();
		let home_dir = match dirs::home_dir() {
			Some(p) => p,
			None => PathBuf::new(),
		}
		.as_path()
		.display()
		.to_string();

		config.base_dir = config.base_dir.replace("~", &home_dir);
		let tmp_file_dir = format!("{}/client/tmp_files", config.base_dir);
		create_dir_all(tmp_file_dir)?;

		let evh_config = EventHandlerConfig {
			threads: config.threads,
			max_handles_per_thread: config.max_handles_per_thread,
			..Default::default()
		};
		let mut evh = Builder::build_evh(evh_config)?;
		let event_handler_data = evh.event_handler_data()?;

		let mut content_allocator = bmw_util::Builder::build_sync_slabs();
		content_allocator.init(SlabAllocatorConfig {
			slab_size: CONTENT_SLAB_SIZE,
			slab_count: config.slab_count,
		})?;
		let content_allocator = lock_box!(content_allocator)?;

		let config = config.clone();
		let config2 = config.clone();
		let config3 = config.clone();
		let config4 = config.clone();
		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config, conn_data, ctx, attach, content_allocator.clone())
		})?;
		evh.set_on_accept(move |conn_data, ctx| Self::process_on_accept(&config2, conn_data, ctx))?;
		evh.set_on_close(move |conn_data, ctx| Self::process_on_close(&config3, conn_data, ctx))?;
		evh.set_on_panic(move |ctx, e| Self::process_on_panic(ctx, e))?;
		evh.set_housekeeper(move |ctx| {
			Self::process_housekeeper(config4.clone(), ctx, event_handler_data.clone())
		})?;

		evh.start()?;

		Ok(Self {
			controller: evh.event_handler_controller()?,
		})
	}

	pub(crate) fn do_stop(&mut self) -> Result<(), Error> {
		self.controller.stop()?;
		Ok(())
	}

	fn build_ctx<'a>(
		ctx: &'a mut ThreadContext,
		config: &HttpClientConfig,
	) -> Result<&'a mut HttpClientContext, Error> {
		match ctx.user_data.downcast_ref::<HttpClientContext>() {
			Some(_) => {}
			None => {
				ctx.user_data = Box::new(Self::build_httpclient_context(config)?);
			}
		}

		Ok(ctx.user_data.downcast_mut::<HttpClientContext>().unwrap())
	}

	fn build_httpclient_context(config: &HttpClientConfig) -> Result<HttpClientContext, Error> {
		debug!("build ctx")?;

		let max_wildcard = config.max_headers_len;
		let termination_length = config.max_headers_len + 300;

		let matches = [bmw_util::Builder::build_match_default(); MATCH_ARRAY_SIZE];

		let slab_allocator = slab_allocator!()?;
		let mut list =
			bmw_util::Builder::build_list(ListConfig::default(), &Some(&slab_allocator))?;
		list.push(bmw_util::Builder::build_pattern(
			"\r\n\r\n",
			false,
			true,
			true,
			SUFFIX_TREE_TERMINATE_HEADERS_ID,
		)?)?;

		list.push(pattern!(
			Regex("Connection: close"),
			Id(CONNECTION_CLOSE_ID)
		)?)?;

		list.push(pattern!(Regex("\r\n.*: "), Id(SUFFIX_TREE_HEADER_ID))?)?;

		let suffix_tree = Box::new(suffix_tree!(
			list,
			TerminationLength(termination_length),
			MaxWildcardLength(max_wildcard)
		)?);

		Ok(HttpClientContext {
			slab_start: 0,
			suffix_tree,
			matches,
		})
	}

	fn process_on_read(
		config: &HttpClientConfig,
		conn_data: &mut ConnectionData,
		ctx: &mut ThreadContext,
		attachment: Option<AttachmentHolder>,
		slab_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	) -> Result<(), Error> {
		match attachment {
			Some(mut attachment) => {
				let mut attachment = attachment.attachment.wlock()?;
				let attachment = attachment.guard();
				match (**attachment).downcast_mut::<HttpClientAttachment>() {
					Some(ref mut attachment) => {
						let mut attachment = attachment.http_client_data.wlock()?;
						let guard = attachment.guard();
						loop {
							let pop_count = match (**guard).front_mut() {
								Some(ref mut attachment) => Self::process_on_read_data(
									conn_data,
									&attachment.request,
									&mut attachment.handler,
									ctx,
									config,
									attachment.close_on_complete,
									slab_allocator.clone(),
								)?,
								None => 0,
							};

							if pop_count == 0 {
								break;
							}
							for _ in 0..pop_count {
								(**guard).pop_front();
							}

							if (**guard).len() == 0 {
								break;
							}
						}
					}
					None => {
						warn!("process_on_read included invalid attachment. Could not process request!")?;
					}
				}
			}
			None => {
				warn!("process_on_read did not include attachment. Could not process request!")?;
			}
		}
		Ok(())
	}

	fn process_on_read_data(
		conn_data: &mut ConnectionData,
		req: &Box<dyn HttpRequest + Send + Sync>,
		handler: &mut HttpHandler,
		ctx: &mut ThreadContext,
		config: &HttpClientConfig,
		close_on_complete: bool,
		slab_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	) -> Result<usize, Error> {
		let ctx = Self::build_ctx(ctx, config)?;
		let id = conn_data.get_connection_id();
		debug!("id={},guid={}", id, req.guid())?;
		let first_slab = conn_data.first_slab();
		let last_slab = conn_data.last_slab();
		let slab_offset = conn_data.slab_offset();
		let slab_start = ctx.slab_start;

		let (res, slab_id_vec) = conn_data.borrow_slab_allocator(move |sa| {
			let mut ret: Vec<u8> = vec![];
			let mut slab_id_vec = vec![];
			let mut slab_id = first_slab;

			loop {
				if slab_id >= u32::MAX {
					break;
				}

				slab_id_vec.push(slab_id);
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = if slab_id == last_slab {
					slab_offset as usize
				} else {
					READ_SLAB_DATA_SIZE
				};

				let start = if slab_id == first_slab { slab_start } else { 0 };
				ret.extend(&slab_bytes[start..offset]);

				if slab_id == last_slab {
					break;
				}

				slab_id = u32::from_be_bytes(try_into!(
					slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
				)?);
			}

			Ok((ret, slab_id_vec))
		})?;

		let ret = if res.len() > 0 {
			Self::process_res(
				conn_data,
				res,
				req,
				handler,
				ctx,
				slab_id_vec,
				config,
				close_on_complete,
				slab_allocator,
			)?
		} else {
			0
		};

		Ok(ret)
	}

	fn process_res(
		conn_data: &mut ConnectionData,
		res: Vec<u8>,
		req: &Box<dyn HttpRequest + Send + Sync>,
		handler: &mut HttpHandler,
		ctx: &mut HttpClientContext,
		slab_id_vec: Vec<u32>,
		config: &HttpClientConfig,
		close_on_complete: bool,
		slab_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	) -> Result<usize, Error> {
		let mut pop_count = 0;
		let res_len = res.len();
		if config.debug {
			info!("res='{}'", std::str::from_utf8(&res).unwrap_or("utf8err"))?;
		}
		let count = ctx.suffix_tree.tmatch(&res, &mut ctx.matches)?;
		let tmp_file_dir = canonicalize(config.base_dir.clone())?;
		let mut response = HttpResponseImpl::new(
			HttpContentReaderData::new(),
			tmp_file_dir,
			slab_allocator.clone(),
			true,
		)?;

		for i in 0..count {
			if ctx.matches[i].id() == CONNECTION_CLOSE_ID {
				response.keep_alive = false;
			} else if ctx.matches[i].id() == SUFFIX_TREE_HEADER_ID {
				let start = ctx.matches[i].start() + 2;
				let end = ctx.matches[i].end() - 2;
				let header_name = if start < end && end < res_len {
					from_utf8(&res[start..end]).unwrap_or("")
				} else {
					""
				};

				let start = end + 2;
				let mut end = start;
				loop {
					if end > res_len {
						break;
					}

					if res[end] == '\r' as u8 || res[end] == '\n' as u8 {
						break;
					}
					end += 1;
				}
				let header_value = if start < end && end < res_len {
					from_utf8(&res[start..end]).unwrap_or("")
				} else {
					""
				};

				let header_name_lower = header_name.to_lowercase();
				if header_name_lower == "transfer-encoding"
					&& header_value.to_lowercase() == "chunked"
				{
					response.chunked = true;
				} else if header_name_lower == "content-length" {
					response.content_length = header_value.parse().unwrap_or(0);
				}

				if header_name != "" && header_value != "" {
					response
						.headers
						.push((header_name.to_string(), header_value.to_string()));
				}
			} else if ctx.matches[i].id() == SUFFIX_TREE_TERMINATE_HEADERS_ID {
				response.start_content = ctx.matches[i].end();
				debug!(
					"term matches[{}]={:?},transfer_encoding_chunked={},content_length={}",
					i, ctx.matches[i], response.chunked, response.content_length,
				)?;
			}
		}

		let mut clear_point = 0;

		// try to parse the first line
		if response.start_content > 0 {
			let mut itt = 0;
			loop {
				if itt >= res_len || res[itt] == '\r' as u8 || res[itt] == '\n' as u8 {
					break;
				}
				itt += 1;
			}

			if itt >= res_len {
				// first line not there yet, wait for more data
				return Ok(pop_count);
			}
			let first_line = from_utf8(&res[0..itt]).unwrap_or("");
			(response.version, response.code, response.status_text) =
				match Self::process_first_line(first_line) {
					Ok((version, code, status_text)) => (version, code, status_text),
					Err(e) => {
						warn!("Error parsing response: {}", e)?;
						(HttpVersion::HTTP10, 400u16, "bad request".to_string())
					}
				};
		}

		if response.start_content == 0 {
			// we are not ready to process so return
			return Ok(pop_count);
		} else if response.chunked {
			// we are ready to process the request and it's chunked

			let mut itt = response.start_content;
			loop {
				if itt >= res_len {
					break;
				}

				let start = itt;
				let mut end = start;

				loop {
					if end >= res_len || res[end] == '\r' as u8 || res[end] == '\n' as u8 {
						break;
					}
					end += 1;
				}

				if end >= res_len {
					debug!("not enough data yet")?;
					return Ok(pop_count);
				}

				let (val, line_len) = if start < end {
					let line = from_utf8(&res[start..end]).unwrap_or("0");
					debug!("line='{}'", line)?;
					let val = usize::from_str_radix(line, 16)?;
					debug!("firstline='{}',val={}", line, val)?;
					(val, line.len())
				} else {
					debug!("firstline wasn't available")?;
					(usize::MAX, 0)
				};

				debug!("val={}", val)?;

				if val == 0 {
					// ensure proper padding per http spec
					if itt + line_len + 4 > res_len {
						// not enough data, return for now
						return Ok(pop_count);
					}

					if !response.keep_alive {
						conn_data.write_handle().close()?;
					}
					// the request is complete
					let mut resp: Box<dyn HttpResponse + Send + Sync> = Box::new(response.clone());
					let req_clone = req.clone();
					pop_count += 1;
					handler(&req_clone, &mut resp)?;

					debug!("incr clear point {},itt={}", itt + line_len + 4, itt)?;
					clear_point = itt + line_len + 4;
					if close_on_complete {
						let _ = conn_data.write_handle().close();
					}
					let _ = response
						.http_content_reader_data
						.clear(slab_allocator, config.tmp_file_dir());
					break;
				} else if val == usize::MAX {
					debug!("invalid request. close conn and return.")?;
					let _ = conn_data.write_handle().close();
					let _ = response
						.http_content_reader_data
						.clear(slab_allocator, config.tmp_file_dir());
					return Ok(pop_count);
				} else {
					let start = itt + line_len + 2;
					let end = itt + line_len + 2 + val;

					if end <= res_len {
						// append data
						response.http_content_reader_data.extend(
							&res[start..end],
							slab_allocator.clone(),
							config.tmp_file_dir(),
						)?;
						itt += val + line_len + 4; // for '\r\n' twice
					} else {
						// not enough data, return for now
						let _ = response
							.http_content_reader_data
							.clear(slab_allocator, config.tmp_file_dir());
						return Ok(pop_count);
					}
				}
			}
		} else {
			if response.start_content + response.content_length > res_len {
				debug!("not enough data yet")?;

				let _ = response
					.http_content_reader_data
					.clear(slab_allocator, config.tmp_file_dir());

				return Ok(pop_count);
			}
			// we have the data.

			let start = response.start_content;
			let end = start + response.content_length;

			response.http_content_reader_data.extend(
				&res[start..end],
				slab_allocator.clone(),
				config.tmp_file_dir(),
			)?;

			if !response.keep_alive {
				conn_data.write_handle().close()?;
			}

			debug!(
				"is_closed={}, keep_alive={}",
				conn_data.write_handle().is_closed().unwrap_or(true),
				response.keep_alive
			)?;
			let mut resp: Box<dyn HttpResponse + Send + Sync> = Box::new(response.clone());
			pop_count += 1;
			handler(req, &mut resp)?;
			clear_point = end;
			if close_on_complete {
				let _ = conn_data.write_handle().close();
			}
			let _ = response
				.http_content_reader_data
				.clear(slab_allocator, config.tmp_file_dir());
		}

		// we processed data so we need to clear some slabs
		debug!("clear_point={},res_len={}", clear_point, res_len)?;
		let slab_id_vec_len = slab_id_vec.len();
		if clear_point >= res_len && clear_point > 0 && slab_id_vec_len >= 1 {
			let last_slab = slab_id_vec[slab_id_vec_len - 1];
			conn_data.clear_through(last_slab)?;
			ctx.slab_start = 0;
		} else if clear_point < res_len && clear_point > 0 && slab_id_vec_len >= 1 {
			let index = (clear_point + ctx.slab_start) / READ_SLAB_DATA_SIZE;
			if index > 0 {
				let last_slab =
					slab_id_vec[(clear_point + ctx.slab_start) / READ_SLAB_DATA_SIZE - 1];
				debug!("clear partial through {}", last_slab)?;
				conn_data.clear_through(last_slab)?;
			}
			let slab_start = (clear_point + ctx.slab_start) % READ_SLAB_DATA_SIZE;
			ctx.slab_start = slab_start;
		} else if clear_point > 0 {
			warn!(
				"unexpected condition: clear_point={},res_len={},slab_id_vec_len={}",
				clear_point, res_len, slab_id_vec_len
			)?;
		}

		Ok(pop_count)
	}

	fn process_first_line(first_line: &str) -> Result<(HttpVersion, u16, String), Error> {
		debug!("process_first_line: firstline='{}'", first_line)?;
		let first_line_spl = first_line.split(" ").collect::<Vec<&str>>();
		if first_line_spl.len() < 3 {
			return Err(err!(ErrKind::Http, "bad request"));
		}
		let version_str = first_line_spl[0];
		let version = if version_str == "HTTP/1.1" {
			HttpVersion::HTTP11
		} else if version_str == "HTTP/1.0" {
			HttpVersion::HTTP10
		} else {
			HttpVersion::OTHER
		};

		let code = first_line_spl[1].parse()?;

		let mut start_status_text = 0;
		let mut space_count = 0;
		for ch in first_line.chars() {
			if ch == ' ' {
				space_count += 1;
			}

			start_status_text += 1;
			if space_count == 2 {
				break;
			}
		}

		let status_text = if start_status_text < first_line.len() {
			&first_line[start_status_text..]
		} else {
			""
		};

		Ok((version, code, status_text.to_string()))
	}

	fn process_on_accept(
		_config: &HttpClientConfig,
		conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		let id = conn_data.get_connection_id();
		debug!("Process on accept. conn_id={}", id)?;

		Ok(())
	}

	fn process_on_close(
		_config: &HttpClientConfig,
		conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		debug!("Process on close: {}", conn_data.get_connection_id())?;
		Ok(())
	}

	fn process_on_panic(_ctx: &mut ThreadContext, _e: Box<dyn Any>) -> Result<(), Error> {
		Ok(())
	}

	fn process_housekeeper(
		_config: HttpClientConfig,
		_ctx: &mut ThreadContext,
		_event_handler_data: Array<Box<dyn LockBox<EventHandlerData>>>,
	) -> Result<(), Error> {
		Ok(())
	}
}

impl HttpConnection for HttpConnectionImpl {
	fn send(
		&mut self,
		request: Box<dyn HttpRequest + Send + Sync>,
		handler: HttpHandler,
	) -> Result<(), Error> {
		let keep_alive = request.keep_alive();
		match request.request_uri() {
			Some(uri) => {
				let host = &self.config.host;
				let port = self.config.port;
				let addr = format!("{}:{}", host, port);

				{
					let mut http_client_data = self.http_client_data.wlock()?;
					let guard = http_client_data.guard();
					(**guard).push_back(HttpClientAttachmentData {
						request: request.clone(),
						close_on_complete: false,
						handler,
					});
				}

				do_send(request, &mut self.wh, uri, addr, keep_alive)
			}
			None => Err(err!(ErrKind::Http, "request_url must be specified")),
		}
	}

	fn close(&mut self) -> Result<(), Error> {
		match self.wh.close() {
			Ok(_) => {}
			Err(e) => debug!("closing write handle generated error: {}", e)?,
		}

		let mut http_client_data = self.http_client_data.wlock()?;
		let http_client_data = http_client_data.guard();
		(**http_client_data).clear();
		(**http_client_data).shrink_to_fit();

		Ok(())
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(
		config: &HttpConnectionConfig,
		mut http_client: Box<dyn HttpClient + Send + Sync>,
	) -> Result<HttpConnectionImpl, Error> {
		let config = config.clone();
		let host = config.host.clone();
		let port = config.port;
		let addr = format!("{}:{}", host, port);
		let tcp_stream = TcpStream::connect(addr.clone())?;
		tcp_stream.set_nonblocking(true)?;

		let client_connection = ClientConnection {
			handle: tcp_stream_to_handle(tcp_stream)?,
			tls_config: if config.tls {
				Some(TlsClientConfig {
					sni_host: host,
					trusted_cert_full_chain_file: config.full_chain_cert_file.clone(),
				})
			} else {
				None
			},
		};

		let vec = VecDeque::new();
		let http_client_data = lock_box!(vec)?;
		let wh = http_client.controller().add_client(
			client_connection,
			Box::new(HttpClientAttachment {
				http_client_data: http_client_data.clone(),
			}),
		)?;

		Ok(Self {
			config,
			wh,
			http_client_data,
		})
	}
}

impl PartialEq for Box<dyn HttpRequest + Send + Sync> {
	fn eq(&self, other: &Box<dyn HttpRequest + Send + Sync>) -> bool {
		self.guid() == other.guid()
	}
}

impl HttpRequest for HttpRequestImpl {
	fn request_url(&self) -> Option<String> {
		self.config.request_url.clone()
	}
	fn request_uri(&self) -> Option<String> {
		self.config.request_uri.clone()
	}
	fn user_agent(&self) -> &String {
		&self.config.user_agent
	}
	fn accept(&self) -> &String {
		&self.config.accept
	}
	fn headers(&self) -> &Vec<(String, String)> {
		&self.config.headers
	}
	fn method(&self) -> &HttpMethod {
		&self.config.method
	}
	fn version(&self) -> &HttpVersion {
		&self.config.version
	}
	fn keep_alive(&self) -> bool {
		self.config.keep_alive
	}
	fn content_file(&self) -> &Option<String> {
		&self.config.content_file
	}
	fn content_data(&self) -> &Option<Vec<u8>> {
		&self.config.content_data
	}
	fn full_chain(&self) -> &Option<String> {
		&self.config.full_chain
	}
	fn timeout_millis(&self) -> u64 {
		self.config.timeout_millis
	}
	fn guid(&self) -> u128 {
		self.guid
	}
}

impl HttpRequestImpl {
	pub(crate) fn new(config: &HttpRequestConfig) -> Result<HttpRequestImpl, Error> {
		Ok(Self {
			config: config.clone(),
			guid: random(),
		})
	}
}

impl HttpResponse for HttpResponseImpl {
	fn headers(&self) -> Result<&Vec<(String, String)>, Error> {
		Ok(&self.headers)
	}

	fn code(&self) -> Result<u16, Error> {
		Ok(self.code)
	}

	fn status_text(&self) -> Result<&String, Error> {
		Ok(&self.status_text)
	}

	fn version(&self) -> Result<&HttpVersion, Error> {
		Ok(&self.version)
	}

	fn content_reader(&self) -> Result<HttpContentReader, Error> {
		let mut ncontent_reader = self.http_content_reader_data.clone();
		ncontent_reader.read_slab = self.http_content_reader_data.head_slab;
		ncontent_reader.read_offset = 0;
		let mut hcr = HttpContentReader::new();
		hcr.http_content_reader_data = Some(ncontent_reader);
		hcr.content_allocator = Some(self.content_allocator.clone());
		hcr.tmp_file_dir = self.tmp_file_dir.clone();
		Ok(hcr)
	}
}

impl HttpResponseImpl {
	fn new(
		http_content_reader_data: HttpContentReaderData,
		tmp_file_dir: PathBuf,
		content_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
		keep_alive: bool,
	) -> Result<Self, Error> {
		Ok(Self {
			headers: vec![],
			chunked: false,
			content_length: 0,
			start_content: 0,
			code: 0,
			status_text: "".to_string(),
			version: HttpVersion::UNKNOWN,
			http_content_reader_data,
			tmp_file_dir,
			content_allocator,
			keep_alive,
		})
	}
}

#[cfg(test)]
mod test {
	use crate::types::{HttpClientImpl, HttpRequestImpl};
	use crate::{
		Builder, HttpClient, HttpClientConfig, HttpConfig, HttpConnectionConfig, HttpInstance,
		HttpInstanceType, HttpRequest, HttpRequestConfig, HttpResponse, HttpVersion, PlainConfig,
	};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::lock_box;
	use std::collections::HashMap;
	use std::fs::File;
	use std::io::Read;
	use std::io::Write;
	use std::thread::sleep;
	use std::time::Duration;

	info!();

	#[test]
	fn test_http_client_basic() -> Result<(), Error> {
		let test_dir = ".test_http_client_basic.bmw";
		let data_text = "Hello World123!";
		setup_test_dir(test_dir)?;
		let mut file = File::create(format!("{}/foo.html", test_dir))?;
		file.write_all(data_text.as_bytes())?;
		let port = pick_free_port()?;
		info!("port={}", port)?;
		let addr = "127.0.0.1".to_string();

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			base_dir: test_dir.to_string(),
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		sleep(Duration::from_millis(1_000));

		let mut http_client = HttpClientImpl::new(&HttpClientConfig {
			debug: true,
			base_dir: test_dir.to_string(),
			..Default::default()
		})?;

		let http_client_request1 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_url: Some(format!("http://{}:{}/foo.html", addr, port).to_string()),
			headers: vec![
				("timeout".to_string(), "10".to_string()),
				("something".to_string(), "else".to_string()),
			],
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;
		let http_client_request1_clone = http_client_request1.clone();

		let mut found_count = lock_box!(0)?;
		let found_count_clone = found_count.clone();

		let handler1 = Box::pin(
			move |request: &Box<dyn HttpRequest + Send + Sync>,
			      response: &Box<dyn HttpResponse + Send + Sync>| {
				info!("in handler1,request.guid={}", request.guid())?;
				if request == &http_client_request1_clone {
					let guid = request.guid();
					assert_eq!(guid, http_client_request1_clone.guid());

					let mut hcr = response.content_reader()?;
					let mut buf = [0u8; 1_000];
					let mut content = vec![];
					loop {
						let len = hcr.read(&mut buf)?;
						if len == 0 {
							break;
						}
						content.extend(&buf[0..len]);
					}
					let content = std::str::from_utf8(&content).unwrap();

					assert_eq!(content, data_text);

					let headers = response.headers()?;

					let mut found = false;
					for (n, v) in headers {
						if n == &"Transfer-Encoding".to_string() && v == &"chunked" {
							found = true;
						}
					}

					assert!(found);
					assert_eq!(response.code()?, 200);
					assert_eq!(response.status_text()?, "OK");
					assert_eq!(response.version()?, &HttpVersion::HTTP11);

					let mut found_count = found_count.wlock()?;
					let guard = found_count.guard();
					(**guard) += 1;
				}
				Ok(())
			},
		);

		info!(
			"about to send request with guid = {}",
			http_client_request1.guid()
		)?;

		http_client.send(http_client_request1, handler1)?;

		let mut found404 = lock_box!(false)?;
		let found404_clone = found404.clone();
		let handler2 = Box::pin(
			move |_request: &Box<dyn HttpRequest + Send + Sync>,
			      response: &Box<dyn HttpResponse + Send + Sync>| {
				info!("got response should be 404!")?;
				assert_eq!(response.code()?, 404);
				let mut found404 = found404.wlock()?;
				let guard = found404.guard();
				(**guard) = true;

				Ok(())
			},
		);

		let http_client_request2 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_url: Some(format!("http://{}:{}/foo2.html", addr, port).to_string()),
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;

		http_client.send(http_client_request2, handler2)?;

		sleep(Duration::from_millis(1_000));

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if count == 10000 {
				break;
			}
			let found_count = found_count_clone.rlock()?;
			let guard = found_count.guard();

			let found404 = found404_clone.rlock()?;
			let guard2 = found404.guard();

			if (**guard) != 1 || !(**guard2) {
				info!("guard={},count={} of 10,000", (**guard), count)?;
				count += 1;
				continue;
			}
			assert_eq!((**guard), 1);
			break;
		}

		{
			let found_count = found_count_clone.rlock()?;
			let guard = found_count.guard();
			assert_eq!((**guard), 1);
		}

		{
			let found404 = found404_clone.rlock()?;
			let guard = found404.guard();
			assert_eq!((**guard), true);
		}

		tear_down_test_dir(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_http_connection_basic() -> Result<(), Error> {
		let test_dir = ".test_http_connection_basic.bmw";
		let data_text = "Hello World1234!";
		let data_text2 = "another file";
		setup_test_dir(test_dir)?;
		let mut file = File::create(format!("{}/foo.html", test_dir))?;
		file.write_all(data_text.as_bytes())?;
		let mut file = File::create(format!("{}/foo2.html", test_dir))?;
		file.write_all(data_text2.as_bytes())?;

		let port = pick_free_port()?;
		info!("port={}", port)?;
		let addr = "127.0.0.1".to_string();

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			base_dir: test_dir.to_string(),
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		let http_client = HttpClientImpl::new(&HttpClientConfig {
			base_dir: test_dir.to_string(),
			debug: true,
			..Default::default()
		})?;

		let connection_config = HttpConnectionConfig {
			host: addr.clone(),
			port,
			tls: false,
			..Default::default()
		};
		let mut http_connection =
			Builder::build_http_connection(&connection_config, Box::new(http_client.clone()))?;

		let http_client_request1 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_uri: Some("/foo.html".to_string()),
			headers: vec![
				("timeout".to_string(), "10".to_string()),
				("something".to_string(), "else".to_string()),
			],
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;
		let http_client_request1_clone = http_client_request1.clone();

		let mut found_count = lock_box!(0)?;
		let found_count_clone = found_count.clone();

		let handler1 = Box::pin(
			move |request: &Box<dyn HttpRequest + Send + Sync>,
			      response: &Box<dyn HttpResponse + Send + Sync>| {
				info!("in handler1,request.guid={}", request.guid())?;
				if request == &http_client_request1_clone {
					let mut hcr = response.content_reader()?;
					let mut buf = [0u8; 1_000];
					let mut content = vec![];
					loop {
						let len = hcr.read(&mut buf)?;
						if len == 0 {
							break;
						}
						content.extend(&buf[0..len]);
					}
					let content = std::str::from_utf8(&content).unwrap();
					assert_eq!(content, data_text);

					let headers = response.headers()?;

					let mut found = false;
					for (n, v) in headers {
						if n == &"Transfer-Encoding".to_string() && v == &"chunked" {
							found = true;
						}
					}

					assert!(found);
					assert_eq!(response.code()?, 200);
					assert_eq!(response.status_text()?, "OK");
					assert_eq!(response.version()?, &HttpVersion::HTTP11);

					let mut found_count = found_count.wlock()?;
					let guard = found_count.guard();
					(**guard) += 1;
				}
				Ok(())
			},
		);

		info!(
			"about to send request with guid = {}",
			http_client_request1.guid()
		)?;
		http_connection.send(http_client_request1, handler1)?;

		//sleep(Duration::from_millis(1_000));

		let http_client_request2 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_uri: Some("/foo2.html".to_string()),
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;

		let http_client_request3 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_uri: Some("/foo3.html".to_string()),
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;

		let http_client_request3_clone = http_client_request3.clone();
		let http_client_request2_clone = http_client_request2.clone();

		let mut found404 = lock_box!(false)?;
		let found404_clone = found404.clone();
		let mut found_another_file = lock_box!(false)?;
		let found_another_file_clone = found_another_file.clone();
		let handler2 = Box::pin(
			move |request: &Box<dyn HttpRequest + Send + Sync>,
			      response: &Box<dyn HttpResponse + Send + Sync>| {
				if request == &http_client_request3_clone {
					info!("got response should be 404!")?;
					assert_eq!(response.code()?, 404);
					let mut found404 = found404.wlock()?;
					let guard = found404.guard();
					assert_eq!(**guard, false);
					(**guard) = true;
				} else if request == &http_client_request2_clone {
					let mut hcr = response.content_reader()?;
					let mut buf = [0u8; 1_000];
					let mut content = vec![];
					loop {
						let len = hcr.read(&mut buf)?;
						if len == 0 {
							break;
						}
						content.extend(&buf[0..len]);
					}
					let content = std::str::from_utf8(&content).unwrap();
					assert_eq!(content, data_text2);

					let headers = response.headers()?;

					let mut found = false;
					for (n, v) in headers {
						if n == &"Transfer-Encoding".to_string() && v == &"chunked" {
							found = true;
						}
					}

					assert!(found);
					assert_eq!(response.code()?, 200);
					assert_eq!(response.status_text()?, "OK");
					assert_eq!(response.version()?, &HttpVersion::HTTP11);

					let mut found_another_file = found_another_file.wlock()?;
					let guard = found_another_file.guard();
					assert_eq!(**guard, false);
					(**guard) = true;
				}

				Ok(())
			},
		);

		http_connection.send(http_client_request2, handler2.clone())?;

		//sleep(Duration::from_millis(1_000));

		http_connection.send(http_client_request3, handler2)?;

		//sleep(Duration::from_millis(1_000));

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(100));
			if count == 100 {
				break;
			}
			let found404 = found404_clone.rlock()?;
			let found_count = found_count_clone.rlock()?;
			let found_another_file = found_another_file_clone.rlock()?;
			let guard = found_count.guard();
			let guard2 = found404.guard();
			let guard3 = found_another_file.guard();

			if (**guard) != 1 {
				info!("guard={},count={} of 10,000", (**guard), count)?;
				count += 1;
				continue;
			}

			if !**guard2 {
				info!(
					"404 not proc,guard={},count={} of 10,000",
					(**guard2),
					count
				)?;
				count += 1;
				continue;
			}

			if !**guard3 {
				info!("another file not found, guard={},count={}", **guard3, count)?;
				count += 1;
				continue;
			}
			assert_eq!((**guard), 1);
			break;
		}

		{
			let found_count = found_count_clone.rlock()?;
			let guard = found_count.guard();
			assert_eq!((**guard), 1);
		}

		{
			let found404 = found404_clone.rlock()?;
			let guard = found404.guard();
			assert_eq!((**guard), true);
		}

		{
			let found_another_file = found_another_file_clone.rlock()?;
			let guard = found_another_file.guard();
			assert_eq!((**guard), true);
		}

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
