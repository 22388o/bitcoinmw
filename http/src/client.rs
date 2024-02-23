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
	HttpClientAttachment, HttpClientContext, HttpClientImpl, HttpConnectionImpl, HttpRequestImpl,
	HttpResponseImpl,
};
use crate::{
	HttpClient, HttpClientConfig, HttpConnection, HttpConnectionConfig, HttpHandler, HttpRequest,
	HttpRequestConfig, HttpResponse,
};
use bmw_deps::rand::random;
use bmw_deps::url::Url;
use bmw_err::*;
use bmw_evh::{
	tcp_stream_to_handle, AttachmentHolder, Builder, ClientConnection, ConnData, ConnectionData,
	EventHandler, EventHandlerConfig, EventHandlerData, ThreadContext, READ_SLAB_DATA_SIZE,
};
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::net::TcpStream;
use std::str::from_utf8;

debug!();

impl Default for HttpClientConfig {
	fn default() -> Self {
		Self {
			max_headers_len: 8192,
			debug: false,
		}
	}
}

impl Default for HttpRequestConfig {
	fn default() -> Self {
		Self { request_url: None }
	}
}

impl HttpClient for HttpClientImpl {
	fn send(
		&mut self,
		request: Box<dyn HttpRequest + Send + Sync>,
		handler: HttpHandler,
	) -> Result<(), Error> {
		let request_url = request.request_url();
		debug!("request url = {:?}", request_url)?;

		match request_url {
			Some(request_url) => {
				let url = Url::parse(&request_url)?;
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

				let path = url.path();
				let query = url.query().unwrap_or("");
				let addr = format!("{}:{}", host, port);
				let tcp_stream = TcpStream::connect(addr.clone())?;

				debug!(
					"host={},port={},path={},query={},guid={}",
					host,
					port,
					path,
					query,
					request.guid()
				)?;
				let client_connection = ClientConnection {
					handle: tcp_stream_to_handle(tcp_stream)?,
					tls_config: None,
				};
				let mut wh = self.controller.add_client(
					client_connection,
					Box::new(HttpClientAttachment { handler, request }),
				)?;

				let req_str = format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, addr);
				wh.write(req_str.as_bytes())?;
			}
			None => {}
		}

		Ok(())
	}
}

impl HttpClientImpl {
	pub(crate) fn new(config: &HttpClientConfig) -> Result<HttpClientImpl, Error> {
		let evh_config = EventHandlerConfig {
			..Default::default()
		};
		let mut evh = Builder::build_evh(evh_config)?;
		let event_handler_data = evh.event_handler_data()?;

		let config = config.clone();
		let config2 = config.clone();
		let config3 = config.clone();
		let config4 = config.clone();
		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config, conn_data, ctx, attach)
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
	) -> Result<(), Error> {
		match attachment {
			Some(mut attachment) => {
				let mut attachment = attachment.attachment.wlock()?;
				let attachment = attachment.guard();

				match (**attachment).downcast_mut::<HttpClientAttachment>() {
					Some(ref mut attachment) => {
						Self::process_on_read_data(
							conn_data,
							&mut attachment.handler,
							&attachment.request,
							ctx,
							config,
						)?;
					}
					None => {
						warn!("process_on_read included invalid attachment. Could not process request!")?;
					}
				}
			}
			None => {
				warn!("process_on_read did not include attachment. Could not process request!")?;
			}
		};
		Ok(())
	}

	fn process_on_read_data(
		conn_data: &mut ConnectionData,
		handler: &mut HttpHandler,
		req: &Box<dyn HttpRequest + Send + Sync>,
		ctx: &mut ThreadContext,
		config: &HttpClientConfig,
	) -> Result<(), Error> {
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

		Self::process_res(conn_data, res, handler, req, ctx, slab_id_vec, config)?;

		Ok(())
	}

	fn process_res(
		conn_data: &mut ConnectionData,
		res: Vec<u8>,
		handler: &mut HttpHandler,
		req: &Box<dyn HttpRequest + Send + Sync>,
		ctx: &mut HttpClientContext,
		slab_id_vec: Vec<u32>,
		config: &HttpClientConfig,
	) -> Result<(), Error> {
		let res_len = res.len();
		if config.debug {
			info!("res='{}'", std::str::from_utf8(&res).unwrap_or("utf8err"))?;
		}
		let count = ctx.suffix_tree.tmatch(&res, &mut ctx.matches)?;
		let mut response = HttpResponseImpl::new();

		for i in 0..count {
			if ctx.matches[i].id() == SUFFIX_TREE_HEADER_ID {
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
				debug!(
					"matches[{}]={:?},header_name='{}',header_value='{}'",
					i, ctx.matches[i], header_name, header_value
				)?;

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

		if response.start_content == 0 {
			// we are not ready to process so return
			return Ok(());
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
					return Ok(());
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
						return Ok(());
					}
					// the request is complete
					let mut resp: Box<dyn HttpResponse> = Box::new(response);
					handler(req, &mut resp)?;
					clear_point = itt + line_len + 4;
					break;
				} else if val == usize::MAX {
					debug!("invalid request. close conn and return.")?;
					conn_data.write_handle().close()?;
					return Ok(());
				} else {
					// append data
					response
						.content
						.extend(&res[itt + line_len + 2..itt + line_len + 2 + val]);
					itt += val + line_len + 4; // for '\r\n' twice
				}
			}
		} else {
			// TODO: content_len is set
		}

		// we processed data so we need to clear some slabs
		debug!("clear_point={},res_len={}", clear_point, res_len)?;
		let slab_id_vec_len = slab_id_vec.len();
		if clear_point >= res_len && clear_point > 0 && slab_id_vec_len >= 1 {
			let last_slab = slab_id_vec[slab_id_vec_len - 1];
			debug!("clear through {}", last_slab)?;
			conn_data.clear_through(last_slab)?;
			ctx.slab_start = 0;
		} else if clear_point < res_len && clear_point > 0 && slab_id_vec_len >= 1 {
			let last_slab = slab_id_vec[((clear_point + ctx.slab_start) / READ_SLAB_DATA_SIZE) - 1];
			debug!("clear partial through {}", last_slab)?;
			conn_data.clear_through(last_slab)?;
			ctx.slab_start = (clear_point + ctx.slab_start) % READ_SLAB_DATA_SIZE;
		} else {
			warn!(
				"unexpected condition: clear_point={},res_len={},slab_id_vec_len={}",
				clear_point, res_len, slab_id_vec_len
			)?;
		}

		Ok(())
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
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
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
		_req: Box<dyn HttpRequest + Send + Sync>,
		_handler: HttpHandler,
	) -> Result<(), Error> {
		todo!()
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(_config: &HttpConnectionConfig) -> Result<HttpConnectionImpl, Error> {
		Ok(Self {})
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
	fn content(&self) -> Result<&Vec<u8>, Error> {
		Ok(&self.content)
	}

	fn headers(&self) -> Result<&Vec<(String, String)>, Error> {
		Ok(&self.headers)
	}
}

impl HttpResponseImpl {
	fn new() -> Self {
		Self {
			headers: vec![],
			chunked: false,
			content_length: 0,
			start_content: 0,
			content: vec![],
		}
	}
}

#[cfg(test)]
mod test {
	use crate::types::{HttpClientImpl, HttpRequestImpl};
	use crate::{
		Builder, HttpClient, HttpClientConfig, HttpConfig, HttpHandler, HttpInstance,
		HttpInstanceType, HttpRequest, HttpRequestConfig, HttpResponse, PlainConfig,
	};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::lock_box;
	use std::collections::HashMap;
	use std::fs::File;
	use std::io::Write;
	use std::str::from_utf8;
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
			server_version: "test1".to_string(),
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		let mut http_client = HttpClientImpl::new(&HttpClientConfig {
			debug: true,
			..Default::default()
		})?;

		let http_client_request1 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			request_url: Some(format!("http://{}:{}/foo.html", addr, port).to_string()),
			..Default::default()
		})?) as Box<dyn HttpRequest + Send + Sync>;
		let http_client_request1_clone = http_client_request1.clone();

		let mut found_count = lock_box!(0)?;
		let found_count_clone = found_count.clone();

		let handler1: HttpHandler = Box::pin(move |request, response| {
			info!("in handler1,request.guid={}", request.guid())?;
			if request == &http_client_request1_clone {
				let content = response.content()?;
				let content = from_utf8(&content).unwrap_or("utf8_err");
				info!("recv req 1: '{}'", content)?;
				assert_eq!(content, data_text);

				let headers = response.headers()?;

				let mut found = false;
				for (n, v) in headers {
					if n == &"Transfer-Encoding".to_string() && v == &"chunked" {
						found = true;
					}
				}

				assert!(found);

				let mut found_count = found_count.wlock()?;
				let guard = found_count.guard();
				(**guard) += 1;
			}
			Ok(())
		});

		info!(
			"about to send request with guid = {}",
			http_client_request1.guid()
		)?;
		http_client.send(http_client_request1, handler1)?;

		let handler2: HttpHandler = Box::pin(
			move |_request: &Box<dyn HttpRequest + Send + Sync>,
			      _response: &mut Box<dyn HttpResponse>| {
				info!("in handler2")?;

				Ok(())
			},
		);

		let http_client_request2 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
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
			if (**guard) != 1 {
				info!("guard={}", (**guard))?;
				count += 1;
				continue;
			}
			assert_eq!((**guard), 1);
			break;
		}

		let found_count = found_count_clone.rlock()?;
		let guard = found_count.guard();
		assert_eq!((**guard), 1);

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
