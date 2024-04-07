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
	HttpCache, HttpContentReader, HttpRequestImpl, HttpResponseImpl, HttpServerImpl,
};
use crate::{
	HttpConnectionType, HttpMethod, HttpRequest, HttpResponse, HttpServer, HttpStats, HttpVersion,
};
use bmw_conf::ConfigOption::*;
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::*;
use bmw_deps::rand::random;
use bmw_err::*;
use bmw_log::*;
use std::fs::File;
use std::io::Read;

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

impl HttpServer for HttpServerImpl {
	fn start(&mut self) -> Result<(), Error> {
		if self.cache.contains("/".into()) {
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
		let cache = HttpCache::new(configs);
		Ok(Self { cache })
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
		self.http_content_reader().read(buf)
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
	fn http_content_reader(&mut self) -> &mut HttpContentReader {
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
				CN::HttpMethod,
				CN::HttpVersion,
				CN::HttpConnectionType,
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
			.get_or_string(&CN::HttpVersion, version_s.clone())
			.into();
		let method_s = DEFAULT_HTTP_METHOD.to_string();
		let method = config
			.get_or_string(&CN::HttpMethod, method_s.clone())
			.into();
		let ctype = DEFAULT_HTTP_CONNECTION_TYPE.to_string();
		let connection_type = config
			.get_or_string(&CN::HttpConnectionType, ctype.clone())
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

		let http_content_reader = HttpContentReader::new(content_data, content)?;

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
