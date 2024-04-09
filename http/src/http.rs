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
	HttpCache, HttpConnectionState, HttpContentReader, HttpRequestImpl, HttpResponseImpl,
	HttpServerConfig, HttpServerContext, HttpServerImpl,
};
use crate::{
	HttpConnectionType, HttpHeaders, HttpMethod, HttpRequest, HttpResponse, HttpServer, HttpStats,
	HttpVersion,
};
use bmw_conf::ConfigOption::*;
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::*;
use bmw_deps::chrono::Utc;
use bmw_deps::dirs;
use bmw_deps::rand::random;
use bmw_err::*;
use bmw_evh::*;
use bmw_log::*;
use bmw_util::*;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::str::from_utf8;
use std::thread::spawn;

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

impl From<String> for HttpVersion {
	fn from(version: String) -> Self {
		if version == HTTP_VERSION_11 {
			HttpVersion::Http11
		} else if version == HTTP_VERSION_10 {
			HttpVersion::Http10
		} else if version == HTTP_VERSION_20 {
			HttpVersion::Http11
		} else {
			HttpVersion::Unknown
		}
	}
}

impl From<String> for HttpMethod {
	fn from(method: String) -> Self {
		if method == HTTP_METHOD_GET {
			HttpMethod::Get
		} else if method == HTTP_METHOD_POST {
			HttpMethod::Post
		} else if method == HTTP_METHOD_HEAD {
			HttpMethod::Head
		} else if method == HTTP_METHOD_PUT {
			HttpMethod::Put
		} else if method == HTTP_METHOD_DELETE {
			HttpMethod::Delete
		} else if method == HTTP_METHOD_OPTIONS {
			HttpMethod::Options
		} else if method == HTTP_METHOD_CONNECT {
			HttpMethod::Connect
		} else if method == HTTP_METHOD_TRACE {
			HttpMethod::Trace
		} else if method == HTTP_METHOD_PATCH {
			HttpMethod::Patch
		} else {
			HttpMethod::Unknown
		}
	}
}

impl From<String> for HttpConnectionType {
	fn from(ctype: String) -> Self {
		if ctype == HTTP_CONNECTION_TYPE_KEEP_ALIVE {
			HttpConnectionType::KeepAlive
		} else if ctype == HTTP_CONNECTION_TYPE_CLOSE {
			HttpConnectionType::Close
		} else {
			HttpConnectionType::Unknown
		}
	}
}

impl Drop for HttpServerImpl {
	fn drop(&mut self) {
		match self.controller.stop() {
			Ok(_) => {}
			Err(e) => {
				let _ = warn!("controller.stop generated error: {}", e);
			}
		}
	}
}

impl HttpServer for HttpServerImpl {
	fn start(&mut self) -> Result<(), Error> {
		if !self.cache.contains("/".into()) {
			warn!("test")?;
		}

		Ok(())
	}
	fn wait_for_stats(&self) -> Result<HttpStats, Error> {
		todo!()
	}
}

impl HttpServerImpl {
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = Self::build_config(configs)?;
		let config_clone = config.clone();

		let cache = HttpCache::new(vec![]);

		let mut matches = [tmatch!()?; 1_000];

		let mut evh = evh!(
			EvhReadSlabSize(config.evh_slab_size),
			EvhReadSlabCount(config.evh_slab_count),
		)?;
		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			HttpServerImpl::process_on_read(connection, ctx, &mut matches, &config_clone)
		})?;
		evh.set_on_accept(move |connection, ctx| -> Result<(), Error> {
			HttpServerImpl::process_on_accept(connection, ctx)
		})?;
		evh.set_on_close(move |connection, ctx| -> Result<(), Error> {
			HttpServerImpl::process_on_close(connection, ctx)
		})?;
		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> { Ok(()) })?;
		let controller = evh.controller()?;
		evh.start()?;

		let addr = format!("{}:{}", config.host, config.port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		Ok(Self { cache, controller })
	}

	fn process_on_accept(
		connection: &mut Connection,
		ctx: &mut Box<dyn UserContext + '_>,
	) -> Result<(), Error> {
		let id = connection.id();
		debug!("on accept: {}", id)?;
		let ctx = Self::build_ctx(ctx)?;
		let state = HttpConnectionState::new()?;
		ctx.connection_state.insert(id, state);
		Ok(())
	}

	fn process_on_close(
		connection: &mut Connection,
		ctx: &mut Box<dyn UserContext + '_>,
	) -> Result<(), Error> {
		let id = connection.id();
		debug!("on close: {}", id)?;
		let ctx = Self::build_ctx(ctx)?;
		Self::close_connection(id, ctx)?;
		Ok(())
	}

	fn close_connection(id: u128, ctx: &mut HttpServerContext) -> Result<(), Error> {
		ctx.connection_state.remove(&id);
		Ok(())
	}

	fn build_config(configs: Vec<ConfigOption>) -> Result<HttpServerConfig, Error> {
		let config = ConfigBuilder::build_config(configs.clone());

		let port = config.get_or_u16(&CN::Port, HTTP_SERVER_DEFAULT_PORT);
		let host = config.get_or_string(&CN::Host, HTTP_SERVER_DEFAULT_HOST.to_string());
		let mut base_dir =
			config.get_or_string(&CN::BaseDir, HTTP_SERVER_DEFAULT_BASE_DIR.to_string());
		let evh_slab_size = config.get_or_usize(&CN::SlabSize, HTTP_SERVER_DEFAULT_EVH_SLAB_SIZE);
		let evh_slab_count =
			config.get_or_usize(&CN::SlabCount, HTTP_SERVER_DEFAULT_EVH_SLAB_COUNT);
		let server = config.get_or_string(
			&CN::ServerName,
			format!("BitcoinMW/{}", built_info::PKG_VERSION.to_string()),
		);

		let home_dir = match dirs::home_dir() {
			Some(p) => p,
			None => PathBuf::new(),
		}
		.as_path()
		.display()
		.to_string();

		base_dir = base_dir.replace("~", &home_dir);

		create_dir_all(base_dir.clone())?;

		Ok(HttpServerConfig {
			base_dir,
			port,
			host,
			server,
			evh_slab_size,
			evh_slab_count,
		})
	}

	fn process_on_read(
		connection: &mut Connection,
		ctx: &mut Box<dyn UserContext + '_>,
		matches: &mut [Match],
		config: &HttpServerConfig,
	) -> Result<(), Error> {
		let mut data: Vec<u8> = vec![];
		let mut chunk_ids = vec![];

		loop {
			let next_chunk = ctx.next_chunk(connection)?;
			cbreak!(next_chunk.is_none());
			let next_chunk = next_chunk.unwrap();
			chunk_ids.push(next_chunk.slab_id());
			let data_extension = next_chunk.data();
			debug!("chunk len = {}", data_extension.len())?;
			data.extend(data_extension);
		}

		let uctx = Self::build_ctx(ctx)?;
		let mut conn_state = match uctx.connection_state.get_mut(&connection.id()) {
			Some(c) => c,
			None => {
				return Err(err!(
					ErrKind::Http,
					"internal err, couldn't find connection state"
				));
			}
		};

		debug!(
			"offset={},data='{}'",
			conn_state.offset,
			from_utf8(&data).unwrap_or("non-utf8 data")
		)?;

		// only continue processing if we're not in async mode
		if !conn_state.is_async() {
			let data_len = data.len();
			match Self::process_request(
				data,
				&mut uctx.trie,
				matches,
				connection,
				config,
				&mut conn_state,
			) {
				Ok(clear_point) => {
					debug!("clear_point={}, data.len={}", clear_point, data_len)?;
					if clear_point == data_len {
						debug!("clear all")?;
						conn_state.offset = 0;
						ctx.clear_all(connection)?;
					} else if clear_point != 0 {
						Self::clear_custom(clear_point, ctx, connection, chunk_ids, config)?;
					}
				}
				Err(e) => {
					debug!("process_request generated error: {}", e)?;
					// handle error here. Either send a response or close connection
				}
			}
		}
		Ok(())
	}

	fn clear_custom(
		clear_point: usize,
		ctx: &mut Box<dyn UserContext + '_>,
		connection: &mut Connection,
		chunk_ids: Vec<usize>,
		config: &HttpServerConfig,
	) -> Result<(), Error> {
		let bytes_per_slab = config.evh_slab_size.saturating_sub(4);
		debug!("clear custom: {}", clear_point)?;

		let uctx = Self::build_ctx(ctx)?;
		let conn_state = match uctx.connection_state.get_mut(&connection.id()) {
			Some(c) => c,
			None => {
				return Err(err!(
					ErrKind::Http,
					"internal err, couldn't find connection state"
				));
			}
		};

		conn_state.offset = clear_point % bytes_per_slab;

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

	fn process_request(
		data: Vec<u8>,
		trie: &mut Box<dyn SearchTrie + Send + Sync>,
		matches: &mut [Match],
		connection: &mut Connection,
		config: &HttpServerConfig,
		conn_state: &mut HttpConnectionState,
	) -> Result<usize, Error> {
		let headers = Self::build_headers(&data[conn_state.offset..], trie, matches)?;
		match headers {
			Some(headers) => {
				debug!("found headers with conn_state.offset={}", conn_state.offset)?;
				let clear_point = Self::process_headers(&headers, connection, config, conn_state)?;
				Ok(clear_point + conn_state.offset)
			}
			None => Ok(0),
		}
	}

	fn process_headers(
		headers: &HttpHeaders,
		connection: &mut Connection,
		config: &HttpServerConfig,
		conn_state: &mut HttpConnectionState,
	) -> Result<usize, Error> {
		debug!("in process headers: {:?}", headers)?;

		let file_path = canonicalize_base_path(&config.base_dir, &headers.uri)?;
		debug!("file requested = {}", file_path)?;

		// check cache here

		let mut write_handle = connection.write_handle()?;
		Self::send_file(file_path, &mut write_handle, config, conn_state)?;

		Ok(headers.end_headers)
	}

	fn build_date() -> String {
		let dt = Utc::now();
		dt.format("%a, %d %h %C%y %H:%M:%S GMT").to_string()
	}

	fn send_file(
		path: String,
		wh: &mut WriteHandle,
		config: &HttpServerConfig,
		conn_state: &mut HttpConnectionState,
	) -> Result<(), Error> {
		let file = File::open(path)?;
		let mut buf_reader = BufReader::new(file);
		let date = Self::build_date();
		wh.write(
			format!(
				"HTTP/1.1 200 OK\r\nServer: {}\r\nDate: {}\r\nTransfer-Encoding: chunked\r\n\r\n",
				config.server, date
			)
			.as_bytes(),
		)?;

		let mut wh = wh.clone();

		conn_state.set_async(true)?;
		let mut conn_state_clone = conn_state.clone();
		spawn(move || -> Result<(), Error> {
			let mut buf = [0u8; HTTP_SERVER_FILE_BUFFER_SIZE];
			let mut i = 0;
			loop {
				debug!("loop {} ", i)?;
				i += 1;
				let len = buf_reader.read(&mut buf)?;
				cbreak!(len <= 0);
				wh.write(&format!("{:X}\r\n", len).as_bytes()[..])?;
				wh.write(&buf[0..len])?;
				wh.write(b"\r\n")?;
			}

			wh.write(b"0\r\n\r\n")?;
			conn_state_clone.set_async(false)?;
			wh.trigger_on_read()?;
			Ok(())
		});

		Ok(())
	}

	fn build_ctx<'a>(
		ctx: &'a mut Box<dyn UserContext + '_>,
	) -> Result<&'a mut HttpServerContext, Error> {
		match ctx.get_user_data() {
			Some(_) => {}
			None => {
				ctx.set_user_data(Box::new(HttpServerContext::new(10_000)?));
			}
		}

		let ret = ctx.get_user_data().as_mut().unwrap();
		Ok(ret.downcast_mut::<HttpServerContext>().unwrap())
	}

	fn build_headers(
		data: &[u8],
		trie: &mut Box<dyn SearchTrie + Send + Sync>,
		matches: &mut [Match],
	) -> Result<Option<HttpHeaders>, Error> {
		let count = trie.tmatch(data, matches)?;
		let mut term = false;
		let mut headers = HttpHeaders::new();
		for i in 0..count {
			let id = matches[i].id();
			if id == HTTP_SEARCH_TRIE_PATTERN_TERMINATION {
				headers.end_headers = matches[i].end();
				term = true;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_HEADER {
				let name = Self::header_name(data, matches[i])?;
				let value = match Self::header_value(data, matches[i])? {
					Some(v) => v,
					None => return Ok(None),
				};
				headers.headers.push((name.to_string(), value.to_string()));
			} else if id == HTTP_SEARCH_TRIE_PATTERN_CONTENT_LENGTH {
				let value = match Self::header_value(data, matches[i])? {
					Some(v) => v,
					None => return Ok(None),
				};
				headers.content_length = value.parse()?;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_TRANSFER_ENCODING {
				let value = match Self::header_value(data, matches[i])? {
					Some(v) => v,
					None => return Ok(None),
				};
				if value.contains("chunked") {
					headers.chunked = true;
				}
			} else if id == HTTP_SEARCH_TRIE_PATTERN_GET {
				headers.method = HttpMethod::Get;
			} else if id == HTTP_SEARCH_TRIE_PATTERN_POST {
				headers.method = HttpMethod::Post;
			}
		}

		if term {
			if headers.method == HttpMethod::Unknown {
				return Err(err!(ErrKind::Http, "bad request"));
			}
			// get uri and version
			let start = if headers.method == HttpMethod::Get {
				4
			} else {
				5
			};

			let mut end = start;
			// guaranteed to have a \r\n because we have a termination match
			loop {
				if data[end] == b' ' || data[end] == b'\r' || data[end] == b'\n' {
					break;
				}
				end += 1;
			}

			headers.uri = from_utf8(&data[start..end])?.to_string();

			end += 1;
			let start = end;
			loop {
				if data[end] == b'\r' || data[end] == b'\n' {
					break;
				}
				end += 1;
			}

			if &data[start..end] == b"HTTP/1.0" {
				headers.version = HttpVersion::Http10;
			} else if &data[start..end] == b"HTTP/1.1" {
				headers.version = HttpVersion::Http11;
			} else if end > start {
				debug!("version_str='{}'", from_utf8(&data[start..end])?)?;
				headers.version = HttpVersion::Other;
			}

			Ok(Some(headers))
		} else {
			Ok(None)
		}
	}

	fn header_value(data: &[u8], m: Match) -> Result<Option<&str>, Error> {
		let start = m.end();
		let mut end = start;
		loop {
			if end >= data.len() {
				// not ready yet
				return Ok(None);
			}
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
			Ok(Some(from_utf8(&data[start..end])?))
		}
	}

	fn header_name(data: &[u8], m: Match) -> Result<&str, Error> {
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

impl HttpContentReader {
	fn new(content_data: Vec<u8>, content: Option<Box<dyn Read>>) -> Result<Self, Error> {
		if content_data.len() > 0 && content.is_some() {
			let text = "content_data must be 0 length if content is set";
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		Ok(Self {
			content,
			content_data,
			content_data_offset: 0,
		})
	}
}

impl Read for HttpContentReader {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
		match &mut self.content {
			Some(content) => content.read(buf),
			None => {
				let off = self.content_data_offset;
				let content_data = &self.content_data;
				let len = content_data.len();

				if off >= len {
					Ok(0)
				} else {
					let available = len.saturating_sub(off);
					let ret_len_max = buf.len();
					let ret_len = if ret_len_max < available {
						ret_len_max
					} else {
						available
					};
					buf[0..ret_len].clone_from_slice(&content_data[off..off + ret_len]);
					self.content_data_offset = off + ret_len;
					Ok(ret_len)
				}
			}
		}
	}
}

impl Read for Box<dyn HttpRequest> {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
		match self.http_content_reader().wlock() {
			Ok(mut http_content_reader) => match http_content_reader.guard() {
				Ok(guard) => guard.read(buf),
				Err(e) => Err(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					format!("error obtaining guard from http_content_reader: {}", e),
				)),
			},
			Err(e) => Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				format!("error obtaining write lock from http_content_reader: {}", e),
			)),
		}
	}
}

impl HttpRequest for HttpRequestImpl {
	fn request_url(&self) -> &Option<String> {
		&self.request_url
	}
	fn request_uri(&self) -> &Option<String> {
		&self.request_uri
	}
	fn user_agent(&self) -> &String {
		&self.user_agent
	}
	fn accept(&self) -> &String {
		&self.accept
	}
	fn headers(&self) -> &Vec<(String, String)> {
		&self.headers
	}
	fn method(&self) -> &HttpMethod {
		&self.method
	}
	fn version(&self) -> &HttpVersion {
		&self.version
	}
	fn timeout_millis(&self) -> u64 {
		self.timeout_millis
	}
	fn connection_type(&self) -> &HttpConnectionType {
		&self.connection_type
	}
	fn guid(&self) -> u128 {
		self.guid
	}
	fn http_content_reader(&mut self) -> &mut Box<dyn LockBox<HttpContentReader>> {
		&mut self.http_content_reader
	}
}

impl HttpRequestImpl {
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config_duplicates(
			vec![
				CN::HttpContentFile,
				CN::HttpContentData,
				CN::HttpAccept,
				CN::HttpHeader,
				CN::HttpTimeoutMillis,
				CN::HttpMeth,
				CN::HttpVers,
				CN::HttpConnection,
				CN::HttpRequestUrl,
				CN::HttpRequestUri,
				CN::HttpUserAgent,
			],
			vec![],
			vec![CN::HttpHeader],
		)?;

		let content: Option<Box<dyn Read>> = match config.get(&CN::HttpContentFile) {
			Some(co) => match co {
				HttpContentFile(file) => Some(Box::new(File::open(file)?)),
				_ => None,
			},
			None => None,
		};

		let content_data = match config.get(&CN::HttpContentData) {
			Some(co) => match co {
				HttpContentData(data) => data,
				_ => vec![],
			},
			None => vec![],
		};

		if content.is_some() && content_data.len() > 0 {
			let text = "HttpContentFile and HttpContentData may not both be set";
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		let headers_options = config.get_multi(&CN::HttpHeader);
		let mut headers = vec![];
		for header in headers_options {
			match header {
				ConfigOption::HttpHeader((n, v)) => {
					headers.push((n, v));
				}
				_ => {}
			}
		}

		let accept = config.get_or_string(&CN::HttpAccept, DEFAULT_HTTP_ACCEPT.to_string());
		let timeout_millis = config.get_or_u64(&CN::HttpTimeoutMillis, DEFAULT_HTTP_TIMEOUT_MILLIS);

		let version_s = DEFAULT_HTTP_VERSION.to_string();
		let version = config
			.get_or_string(&CN::HttpVers, version_s.clone())
			.into();
		let method_s = DEFAULT_HTTP_METHOD.to_string();
		let method = config.get_or_string(&CN::HttpMeth, method_s.clone()).into();
		let ctype = DEFAULT_HTTP_CONNECTION_TYPE.to_string();
		let connection_type = config
			.get_or_string(&CN::HttpConnection, ctype.clone())
			.into();

		let pkg_version = built_info::PKG_VERSION.to_string();
		let user_agent_default = format!("BitcoinMW/{}", pkg_version).to_string();
		let user_agent = config.get_or_string(&CN::HttpUserAgent, user_agent_default);

		let default_rul = DEFAULT_HTTP_REQUEST_URL.to_string();
		let request_url_s = config.get_or_string(&CN::HttpRequestUrl, default_rul.clone());
		let request_url = if request_url_s == default_rul {
			None
		} else {
			Some(request_url_s)
		};

		let default_uri_s = DEFAULT_HTTP_REQUEST_URI.to_string();
		let request_uri_s = config.get_or_string(&CN::HttpRequestUri, default_uri_s.clone());
		let request_uri = if request_uri_s == default_uri_s {
			None
		} else {
			Some(request_uri_s)
		};
		let guid = random();

		if version == HttpVersion::Unknown {
			let text = format!(
				"Unknown HttpVersion specified '{}'. Allowed values are: HTTP/1.0 and HTTP/1.1",
				version_s
			);
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		if method == HttpMethod::Unknown {
			let text = format!(
				"Unknown HttpMethod specified: {}. Allowed values are: {}",
				method_s, "GET/POST/HEAD/PUT/DELETE/OPTIONS/CONNECT/TRACE/PATCH"
			);
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		if connection_type == HttpConnectionType::Unknown {
			let text = format!(
				"Unknown HttpConnectionType specified '{}'. Allowed values are: close/keep-alive",
				ctype
			);
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		let http_content_reader = lock_box!(HttpContentReader::new(content_data, content)?)?;

		Ok(Self {
			http_content_reader,
			accept,
			connection_type,
			guid,
			request_uri,
			request_url,
			method,
			version,
			headers,
			timeout_millis,
			user_agent,
		})
	}
}

impl Read for Box<dyn HttpResponse> {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
		self.http_content_reader().read(buf)
	}
}

impl HttpResponse for HttpResponseImpl {
	fn headers(&self) -> &Vec<(String, String)> {
		&self.headers
	}
	fn code(&self) -> u16 {
		self.code
	}
	fn status_text(&self) -> &String {
		&self.status_text
	}
	fn version(&self) -> &HttpVersion {
		&self.version
	}
	fn http_content_reader(&mut self) -> &mut HttpContentReader {
		&mut self.http_content_reader
	}
}

#[allow(dead_code)]
impl HttpResponseImpl {
	pub(crate) fn new(
		headers: Vec<(String, String)>,
		code: u16,
		status_text: String,
		version: HttpVersion,
		content: Option<Box<dyn Read>>,
		content_data: Vec<u8>,
	) -> Result<Self, Error> {
		let http_content_reader = HttpContentReader::new(content_data, content)?;
		Ok(Self {
			headers,
			code,
			status_text,
			version,
			http_content_reader,
		})
	}
}

impl HttpServerContext {
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
					Regex("^GET ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_GET),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^POST ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_POST),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^HEAD ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_HEAD),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^PUT ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_PUT),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^DELETE ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_DELETE),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^OPTIONS ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_OPTIONS),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^CONNECT ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_CONNECT),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^TRACE ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_TRACE),
					IsCaseSensitive(true)
				)?,
				pattern!(
					Regex("^PATCH ".to_string()),
					PatternId(HTTP_SEARCH_TRIE_PATTERN_PATCH),
					IsCaseSensitive(true)
				)?,
			],
			TerminationLength(termination_length),
			MaxWildCardLength(termination_length),
		)?;
		let connection_state = HashMap::new();
		Ok(Self {
			trie,
			connection_state,
		})
	}
}

impl HttpConnectionState {
	fn new() -> Result<Self, Error> {
		Ok(Self {
			is_async: lock_box!(false)?,
			offset: 0,
		})
	}

	fn is_async(&self) -> bool {
		match self.is_async.rlock() {
			Ok(l) => match l.guard() {
				Ok(g) => **g,
				Err(_) => false,
			},
			Err(_) => false,
		}
	}

	fn set_async(&mut self, value: bool) -> Result<(), Error> {
		wlock!(self.is_async) = value;
		Ok(())
	}
}

unsafe impl Send for HttpContentReader {}

unsafe impl Sync for HttpContentReader {}
