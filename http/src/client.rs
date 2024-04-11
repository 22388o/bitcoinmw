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

use crate::constants::*;
use crate::types::{
	HttpClientConfig, HttpClientContext, HttpClientData, HttpClientImpl, HttpClientState,
	HttpConnectionImpl, HttpHeadersImpl, HttpResponseImpl,
};
use crate::{
	HttpClient, HttpConnection, HttpConnectionType, HttpHeaders, HttpMethod, HttpRequest,
	HttpResponse, HttpResponseHandler, HttpVersion,
};
use bmw_conf::ConfigOption;
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::*;
use bmw_deps::dirs;
use bmw_deps::rand::random;
use bmw_deps::url::Url;
use bmw_err::*;
use bmw_evh::*;
use bmw_log::*;
use bmw_util::*;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::from_utf8;

info!();

impl Display for HttpMethod {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self {
			HttpMethod::Get => write!(f, "GET"),
			HttpMethod::Post => write!(f, "POST"),
			HttpMethod::Head => write!(f, "HEAD"),
			HttpMethod::Put => write!(f, "PUT"),
			HttpMethod::Delete => write!(f, "DELETE"),
			HttpMethod::Options => write!(f, "OPTIONS"),
			HttpMethod::Connect => write!(f, "CONNECT"),
			HttpMethod::Trace => write!(f, "TRACE"),
			HttpMethod::Patch => write!(f, "PATCH"),
			HttpMethod::Unknown => Err(std::fmt::Error),
		}
	}
}

impl Drop for HttpClientImpl {
	fn drop(&mut self) {
		match self.controller.stop() {
			Ok(_) => {}
			Err(e) => {
				let _ = warn!("controller.stop generated error: {}", e);
			}
		}
	}
}

impl HttpClient for HttpClientImpl {
	fn send(
		&mut self,
		request: &Box<dyn HttpRequest>,
		handler: HttpResponseHandler,
	) -> Result<(), Error> {
		let (host, port, uri) = match request.request_url() {
			Some(url_str) => {
				let url = Url::parse(url_str)?;
				let host = match url.host_str() {
					Some(host) => host.to_string(),
					None => {
						return Err(err!(
							ErrKind::IllegalArgument,
							format!("specified url ({}) had no host", url_str)
						));
					}
				};
				let port = match url.port() {
					Some(port) => port,
					None => 80,
				};
				let uri = format!("{}{}", url.path(), url.query().unwrap_or(""));
				(host, port, uri)
			}
			None => {
				return Err(err!(
					ErrKind::IllegalArgument,
					"requests must have a request_url to send via HttpClient.send()"
				))
			}
		};

		let connection = EvhBuilder::build_client_connection(&host, port)?;
		let id = connection.id();
		let mut wh = self.controller.add_client_connection(connection)?;

		let mut http_client_state = HttpClientState::new();
		http_client_state
			.queue
			.push_back(HttpClientData::new(request, handler));

		let mut state = self.state.wlock()?;
		let guard = state.guard()?;
		(**guard).insert(id, http_client_state);

		let method = request.method().to_string();
		let version = "HTTP/1.1";
		wh.write(
			format!(
				"{} {} {}\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
				method, uri, version, host, port
			)
			.as_bytes(),
		)?;

		Ok(())
	}
}

impl HttpClientImpl {
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = Self::build_config(configs)?;
		let mut evh = evh!(
			EvhReadSlabSize(config.evh_read_slab_size),
			EvhReadSlabCount(config.evh_read_slab_count)
		)?;
		let state: Box<dyn LockBox<HashMap<u128, HttpClientState>>> = lock_box!(HashMap::new())?;
		let mut state_clone = state.clone();
		let mut matches = [tmatch!()?; HTTP_CLIENT_MAX_MATCHES];
		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			match HttpClientImpl::process_on_read(
				connection,
				ctx,
				&mut state_clone,
				&mut matches,
				&config,
			) {
				Ok(_) => {}
				Err(e) => {
					warn!("process_on_read generated error: {}", e)?;
					connection.write_handle()?.close()?;
				}
			}

			Ok(())
		})?;
		evh.set_on_accept(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> { Ok(()) })?;
		let controller = evh.controller()?;
		evh.start()?;

		Ok(Self { controller, state })
	}

	fn build_config(configs: Vec<ConfigOption>) -> Result<HttpClientConfig, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(vec![CN::BaseDir], vec![])?;
		let mut base_dir =
			config.get_or_string(&CN::BaseDir, HTTP_CLIENT_DEFAULT_BASE_DIR.to_string());

		let home_dir = match dirs::home_dir() {
			Some(p) => p,
			None => PathBuf::new(),
		}
		.as_path()
		.display()
		.to_string();

		base_dir = base_dir.replace("~", &home_dir);

		let mut tmp_dir_buf = PathBuf::new();
		tmp_dir_buf.push(base_dir);
		tmp_dir_buf.push("tmp");
		let tmp_dir = tmp_dir_buf.as_path().display().to_string();
		let evh_read_slab_size =
			config.get_or_usize(&CN::SlabSize, HTTP_CLIENT_DEFAULT_EVH_SLAB_SIZE);
		let evh_read_slab_count =
			config.get_or_usize(&CN::SlabCount, HTTP_CLIENT_DEFAULT_EVH_SLAB_COUNT);

		// first empty it if it exists (ignore errors, just means it didn't exist)
		let _ = remove_dir_all(tmp_dir_buf.as_path());
		// then create it
		create_dir_all(tmp_dir_buf.as_path())?;

		debug!("using tmp_dir={}", tmp_dir)?;
		Ok(HttpClientConfig {
			tmp_dir,
			evh_read_slab_size,
			evh_read_slab_count,
		})
	}

	fn process_on_read(
		connection: &mut Connection,
		ctx: &mut Box<dyn UserContext + '_>,
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		matches: &mut [Match],
		config: &HttpClientConfig,
	) -> Result<(), Error> {
		debug!("onRead")?;

		let mut data: Vec<u8> = vec![];
		let mut chunk_ids = vec![];

		loop {
			let next_chunk = ctx.next_chunk(connection)?;
			cbreak!(next_chunk.is_none());
			let next_chunk = next_chunk.unwrap();
			chunk_ids.push(next_chunk.slab_id());
			data.extend(next_chunk.data());
		}

		let user_ctx = Self::build_ctx(ctx)?;
		let headers = Self::build_headers(&data, user_ctx, matches, state, connection.id())?;
		match headers {
			Some((mut headers, offset)) => {
				debug!("offset set to {}", offset)?;
				let clear_point = Self::process_headers(
					&mut headers,
					connection,
					&data[offset..],
					state,
					config,
				)?;
				if data.len() == clear_point + offset {
					ctx.clear_all(connection)?;
				} else if clear_point != 0 {
					debug!("clear_point = {}, offset = {}", clear_point, offset)?;
					Self::clear_custom(
						clear_point + offset + headers.end_headers + 2,
						state,
						connection,
						ctx,
						chunk_ids,
						config,
					)?;
				}
			}
			None => {}
		}
		Ok(())
	}

	fn clear_custom(
		clear_point: usize,
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		connection: &mut Connection,
		ctx: &mut Box<dyn UserContext + '_>,
		chunk_ids: Vec<usize>,
		config: &HttpClientConfig,
	) -> Result<(), Error> {
		debug!("=======================clear custom {}", clear_point)?;
		let bytes_per_slab = config.evh_read_slab_size.saturating_sub(4);

		let mut state = state.wlock()?;
		let guard = state.guard()?;

		match (**guard).get_mut(&connection.id()) {
			Some(state) => {
				state.offset = clear_point % bytes_per_slab;
				state.headers_cleared = true;
			}
			None => {
				warn!("")?;
				return Err(err!(
					ErrKind::Http,
					"could not find state for connection {}",
					connection.id()
				));
			}
		}

		if clear_point >= bytes_per_slab {
			let th_chunk = (clear_point / bytes_per_slab).saturating_sub(1);
			if th_chunk >= chunk_ids.len() {
				return Err(err!(
					ErrKind::Http,
					"clear chunk had unexpected value={},chunk_ids.len()={}",
					th_chunk,
					chunk_ids.len()
				));
			}
			ctx.clear_through(chunk_ids[th_chunk], connection)?;
		}

		Ok(())
	}

	fn process_partial(
		_data: &[u8],
		_connection: &mut Connection,
		_config: &HttpClientConfig,
	) -> Result<usize, Error> {
		// for now we just don't clear anything. it builds up in ram, but this is not usually
		// the case as transfer encoding is usually used for large files
		// we can implement this later (just perf issue)
		Ok(0)
	}

	fn process_headers(
		headers: &mut HttpHeadersImpl,
		connection: &mut Connection,
		data: &[u8],
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		config: &HttpClientConfig,
	) -> Result<usize, Error> {
		let data_len = data.len();
		let needed = headers.content_length + headers.end_headers;
		let clear_point;
		if needed > data_len {
			clear_point = Self::process_partial(&data[headers.end_headers..], connection, config)?;
		} else if !headers.chunked {
			// enough data to process
			Self::call_handler(
				state,
				&data[headers.end_headers..headers.end_headers + headers.content_length],
				headers,
				None,
				connection.id(),
			)?;
			clear_point = needed;
		} else {
			debug!("=====================calling process_chunked")?;
			clear_point = Self::process_chunked(
				&data[headers.end_headers..],
				headers,
				connection,
				state,
				config,
			)?;
		}
		debug!("headers={:?},needed={},len={}", headers, needed, data_len)?;

		Ok(clear_point)
	}

	fn process_chunked(
		data: &[u8],
		headers: &mut HttpHeadersImpl,
		connection: &mut Connection,
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		config: &HttpClientConfig,
	) -> Result<usize, Error> {
		debug!("in process chunked")?;
		let data_len = data.len();
		let mut itt = 0;
		let mut complete = false;
		let mut ndata: Vec<u8> = vec![];
		let mut clear_point = 0;
		loop {
			let start_bytes = itt;
			let mut end_bytes = start_bytes;
			loop {
				if end_bytes >= data_len {
					break;
				}

				if data[end_bytes] == b'\r' || data[end_bytes] == b'\n' {
					break;
				}

				itt += 1;
				end_bytes += 1;
			}

			if itt >= data_len {
				break;
			}

			let bytes_slice = &data[start_bytes..end_bytes];
			let bytes_str = from_utf8(&bytes_slice)?;
			debug!("bytes_str='{}'", bytes_str)?;
			debug!("full data = '{}'", from_utf8(data).unwrap_or("utf8err"))?;
			let bytes_len = match usize::from_str_radix(bytes_str, 16) {
				Ok(b) => b,
				Err(e) => {
					debug!(
						"Error trying to parse bytes_slice = {:?}, bytes_str='{}'",
						bytes_slice, bytes_str
					)?;
					return Err(e.into());
				}
			};

			debug!("len='{}'", bytes_len)?;

			if bytes_len == 0 {
				clear_point = end_bytes;
				if end_bytes + 4 <= data_len {
					clear_point = end_bytes + 4;
					complete = true;
				}
				break;
			}

			let start = end_bytes + 2;
			let end = start + bytes_len;

			if end >= data_len {
				break;
			}
			clear_point = end;
			ndata.extend(&data[start..end]);

			itt = end_bytes + bytes_len + 4;
		}

		debug!("nbytes={:?}", ndata)?;
		debug!("clear_point={},data_len={}", clear_point, data_len)?;

		if complete {
			// enough data to process
			debug!("complete data")?;
			Self::process_complete(state, &ndata, headers, connection.id(), config)?;
		} else {
			debug!("data was incomplete")?;
			if clear_point != 0 {
				Self::process_incomplete(state, &ndata, headers, config, connection)?;
			}
		}

		Ok(clear_point)
	}

	fn process_complete(
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		ndata: &[u8],
		headers: &HttpHeadersImpl,
		id: u128,
		config: &HttpClientConfig,
	) -> Result<(), Error> {
		let mut path_buf = PathBuf::new();
		path_buf.push(config.tmp_dir.clone());
		path_buf.push(id.to_string());
		match rlock!(state).get(&id) {
			Some(state) => {
				path_buf.push(state.rid.to_string());
			}
			None => {
				return Err(err!(ErrKind::Http, "couldn't find state for {}", id));
			}
		}

		if path_buf.as_path().exists() {
			debug!("complete with file")?;
			let mut file = File::options().append(true).open(path_buf.clone())?;
			file.write_all(ndata)?;
			Self::call_handler(state, &[], headers, Some(path_buf), id)?;
		} else {
			Self::call_handler(state, &ndata, headers, None, id)?;
		}
		Ok(())
	}

	fn process_incomplete(
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		content: &[u8],
		headers: &HttpHeadersImpl,
		config: &HttpClientConfig,
		connection: &mut Connection,
	) -> Result<(), Error> {
		let mut path_buf = PathBuf::new();
		path_buf.push(config.tmp_dir.clone());
		path_buf.push(connection.id().to_string());

		{
			let mut state = state.wlock()?;
			let guard = state.guard()?;

			create_dir_all(path_buf.as_path())?;
			match (**guard).get_mut(&connection.id()) {
				Some(state) => {
					path_buf.push(state.rid.to_string());

					if state.headers.is_none() {
						state.headers = Some(headers.clone());
						File::create_new(path_buf.clone())?;
					}
				}
				None => {
					return Err(err!(
						ErrKind::Http,
						"couldn't find state for connection {}",
						connection.id()
					));
				}
			}
		}

		let mut file = File::options().append(true).open(path_buf)?;
		file.write_all(content)?;

		Ok(())
	}

	fn call_handler(
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		content: &[u8],
		headers: &HttpHeadersImpl,
		path_buf: Option<PathBuf>,
		id: u128,
	) -> Result<(), Error> {
		let mut next = {
			let mut state = state.wlock()?;
			let guard = state.guard()?;

			match (**guard).get_mut(&id) {
				Some(state) => {
					let next = state.queue.pop_front();
					match next {
						Some(next) => next,
						None => {
							return Err(err!(ErrKind::IllegalState, "expected a HttpClientState"));
						}
					}
				}
				None => {
					return Err(err!(
						ErrKind::IllegalState,
						"couldn't find state for {}",
						id
					))
				}
			}
		};

		let file: Option<Box<dyn Read>> = match path_buf {
			Some(ref path_buf) => Some(Box::new(File::open(path_buf)?)),
			None => None,
		};

		debug!("headers={:?}", headers)?;
		let mut response: Box<dyn HttpResponse> = Box::new(HttpResponseImpl::new(
			headers.headers.clone(),
			headers.status_code,
			headers.status_message.clone(),
			headers.version.clone(),
			file,
			content.to_vec(),
			path_buf,
		)?);
		(next.handler)(&next.request, &mut response)?;
		Ok(())
	}

	fn build_ctx<'a>(
		ctx: &'a mut Box<dyn UserContext + '_>,
	) -> Result<&'a mut HttpClientContext, Error> {
		match ctx.get_user_data() {
			Some(_) => {}
			None => {
				ctx.set_user_data(Box::new(HttpClientContext::new(10_000)?));
			}
		}

		let ret = ctx.get_user_data().as_mut().unwrap();
		Ok(ret.downcast_mut::<HttpClientContext>().unwrap())
	}

	fn build_headers(
		data: &Vec<u8>,
		ctx: &mut HttpClientContext,
		matches: &mut [Match],
		state: &mut Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
		id: u128,
	) -> Result<Option<(HttpHeadersImpl, usize)>, Error> {
		// check if we already have headers
		{
			let state = state.rlock()?;
			let guard = state.guard()?;

			match (**guard).get(&id) {
				Some(state) => {
					if state.headers.is_some() {
						debug!("state.headers is some, cleared = {}", state.headers_cleared)?;
						let mut headers = state.headers.as_ref().unwrap().clone();
						// headers cleared. It's all content.
						if state.headers_cleared {
							headers.end_headers = 0;
						}
						return Ok(Some((headers, state.offset)));
					}
				}
				None => {
					return Err(err!(
						ErrKind::IllegalState,
						"couldn't find state for {}",
						id
					))
				}
			}
		}

		let count = ctx.trie.tmatch(data, matches)?;
		let mut term = false;
		let mut headers = HttpHeadersImpl::new();
		for i in 0..count {
			let id = matches[i].id();
			if id == HTTP_SEARCH_TRIE_PATTERN_TERMINATION {
				headers.end_headers = matches[i].end();
				term = true;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_HEADER {
				let name = Self::header_name(data, matches[i])?;
				let value = Self::header_value(data, matches[i])?;
				headers.headers.push((name.to_string(), value.to_string()));
			} else if id == HTTP_SEARCH_TRIE_PATTERN_CONTENT_LENGTH {
				let value = Self::header_value(data, matches[i])?;
				headers.content_length = value.parse()?;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_TRANSFER_ENCODING {
				let value = Self::header_value(data, matches[i])?;
				if value.contains("chunked") {
					headers.chunked = true;
				}
			} else if id == HTTP_SEARCH_TRIE_PATTERN_HTTP11 {
				headers.version = HttpVersion::Http11;
				(headers.status_message, headers.status_code) = Self::parse_msg_and_code(data)?;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_HTTP10 {
				headers.version = HttpVersion::Http10;
				(headers.status_message, headers.status_code) = Self::parse_msg_and_code(data)?;
			}
		}

		if term {
			Ok(Some((headers, 0)))
		} else {
			Ok(None)
		}
	}

	fn parse_msg_and_code(data: &[u8]) -> Result<(String, u16), Error> {
		// first find first space.
		let mut itt = 0;
		let data_len = data.len();
		loop {
			if itt >= data_len {
				return Err(err!(
					ErrKind::Http,
					"bad request: could not parse msg or code"
				));
			}

			if data[itt] == b' ' {
				break;
			}
			itt += 1;
		}

		let start = itt + 1;
		let mut end = start;

		loop {
			if end >= data_len {
				return Err(err!(
					ErrKind::Http,
					"bad request: could not parse msg or code"
				));
			}

			if data[end] == b' ' {
				break;
			}

			end += 1;
		}

		let code = from_utf8(&data[start..end])?.parse()?;

		let start = end + 1;
		let mut end = start;

		loop {
			if end >= data_len {
				return Err(err!(
					ErrKind::Http,
					"bad request: could not parse msg or code"
				));
			}

			if data[end] == b'\r' || data[end] == b'\n' {
				break;
			}

			end += 1;
		}

		let msg = from_utf8(&data[start..end])?.to_string();

		Ok((msg, code))
	}

	fn header_value(data: &Vec<u8>, m: Match) -> Result<&str, Error> {
		let start = m.end();
		let mut end = start;
		loop {
			if data[end] == b'\r' || data[end] == b'\n' {
				break;
			}
			end += 1;
		}
		if start >= end {
			Err(err!(
				ErrKind::IllegalState,
				"invalid index returned from search start=({}),end=({})",
				start,
				end
			))
		} else if end >= data.len() {
			Err(err!(
				ErrKind::IllegalState,
				"invalid index returned from search end=({}),len=({})",
				end,
				data.len()
			))
		} else {
			Ok(from_utf8(&data[start..end])?)
		}
	}

	fn header_name(data: &Vec<u8>, m: Match) -> Result<&str, Error> {
		let start = m.start() + 2;
		let end = m.end().saturating_sub(2);
		if start >= end {
			Err(err!(
				ErrKind::IllegalState,
				"invalid index returned from search start=({}),end=({})",
				start,
				end
			))
		} else if end >= data.len() {
			Err(err!(
				ErrKind::IllegalState,
				"invalid index returned from search end=({}),len=({})",
				end,
				data.len()
			))
		} else {
			Ok(from_utf8(&data[start..end])?)
		}
	}
}

impl HttpConnection for HttpConnectionImpl {
	fn connect(&mut self) -> Result<(), Error> {
		todo!()
	}
	fn send(
		&mut self,
		_request: &Box<dyn HttpRequest>,
		_handler: HttpResponseHandler,
	) -> Result<(), Error> {
		todo!()
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(
		_configs: Vec<ConfigOption>,
		_client: Box<dyn HttpClient>,
	) -> Result<Self, Error> {
		todo!()
	}
}

impl HttpClientState {
	fn new() -> Self {
		Self {
			queue: VecDeque::new(),
			headers: None,
			offset: 0,
			headers_cleared: false,
			rid: random(),
		}
	}
}

impl HttpClientData {
	fn new(request: &Box<dyn HttpRequest>, handler: HttpResponseHandler) -> Self {
		Self {
			request: request.clone(),
			handler,
		}
	}
}

impl HttpClientContext {
	fn new(termination_length: usize) -> Result<Self, Error> {
		let trie = search_trie_box!(
			vec![
				pattern!(
					Regex("\r\n\r\n".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_TERMINATION),
					IsTerminationPattern(true),
				)?,
				pattern!(
					Regex("\r\nContent-Length: ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_CONTENT_LENGTH),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("\r\nServer: ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_SERVER),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("\r\nTransfer-Encoding: ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_TRANSFER_ENCODING),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("\r\n.*: ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_HEADER),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^HTTP/1.1 ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_HTTP11),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^HTTP/1.0 ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_HTTP10),
					IsCaseSensitive(true)
				)?,
			],
			TerminationLength(termination_length),
			MaxWildCardLength(termination_length),
		)?;
		Ok(Self { trie })
	}
}

impl HttpHeaders for &HttpHeadersImpl {
	fn path(&self) -> String {
		self.uri_path()
	}
}

impl HttpHeadersImpl {
	pub(crate) fn new() -> Self {
		Self {
			headers: vec![],
			content_length: 0,
			end_headers: 0,
			chunked: false,
			method: HttpMethod::Unknown,
			uri: "".to_string(),
			version: HttpVersion::Unknown,
			status_message: "".to_string(),
			status_code: 0,
			connection_type: HttpConnectionType::Unknown,
			host: "".to_string(),
		}
	}

	pub(crate) fn uri_path(&self) -> String {
		match self.uri.find('?') {
			Some(i) => (&self.uri[0..i]).to_string(),
			None => (&self.uri[..]).to_string(),
		}
	}
}

unsafe impl Send for HttpClientState {}
unsafe impl Sync for HttpClientState {}
