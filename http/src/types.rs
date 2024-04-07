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

use bmw_err::*;
use std::io::Read;
use std::pin::Pin;

#[derive(PartialEq, Debug)]
pub enum HttpMethod {
	Get,
	Post,
	Head,
	Put,
	Delete,
	Options,
	Connect,
	Trace,
	Patch,
	Unknown,
}

#[derive(PartialEq, Debug)]
pub enum HttpVersion {
	Http10,
	Http11,
	Other,
	Unknown,
}

#[derive(PartialEq, Debug)]
pub enum HttpConnectionType {
	KeepAlive,
	Close,
	Unknown,
}

pub type HttpResponseHandler = Pin<
	Box<
		dyn FnMut(
				&Box<dyn HttpRequest + Send + Sync>,
				&Box<dyn HttpResponse + Send + Sync>,
			) -> Result<(), Error>
			+ Send
			+ Sync
			+ Unpin,
	>,
>;

/// Builder struct used to build all data strcutures in this crate
pub struct HttpBuilder {}

pub trait HttpServer {
	fn start(&mut self) -> Result<(), Error>;
	fn wait_for_stats(&self) -> Result<HttpStats, Error>;
}

pub trait HttpClient {
	fn send(
		&mut self,
		request: &Box<dyn HttpRequest>,
		handler: HttpResponseHandler,
	) -> Result<(), Error>;
}

pub trait HttpConnection {
	fn connect(&mut self) -> Result<(), Error>;
	fn send(
		&mut self,
		request: &Box<dyn HttpRequest>,
		handler: HttpResponseHandler,
	) -> Result<(), Error>;
}

pub trait HttpRequest {
	fn request_url(&self) -> &Option<String>;
	fn request_uri(&self) -> &Option<String>;
	fn user_agent(&self) -> &String;
	fn accept(&self) -> &String;
	fn headers(&self) -> &Vec<(String, String)>;
	fn method(&self) -> &HttpMethod;
	fn version(&self) -> &HttpVersion;
	fn timeout_millis(&self) -> u64;
	fn connection_type(&self) -> &HttpConnectionType;
	fn guid(&self) -> u128;
	fn http_content_reader(&mut self) -> &mut HttpContentReader;
}

pub trait HttpResponse {
	fn headers(&self) -> &Vec<(String, String)>;
	fn code(&self) -> u16;
	fn status_text(&self) -> &String;
	fn version(&self) -> &HttpVersion;
	fn http_content_reader(&mut self) -> &mut HttpContentReader;
}

pub trait WSClient {}

pub struct HttpStats {}

pub struct HttpContentReader {
	pub(crate) content: Option<Box<dyn Read>>,
	pub(crate) content_data: Vec<u8>,
	pub(crate) content_data_offset: usize,
}

// crate local

pub(crate) struct HttpCache {}

pub(crate) struct HttpServerImpl {
	pub(crate) cache: HttpCache,
}

pub(crate) struct HttpClientImpl {}

pub(crate) struct WSClientImpl {}

pub(crate) struct HttpRequestImpl {
	pub(crate) guid: u128,
	pub(crate) request_url: Option<String>,
	pub(crate) request_uri: Option<String>,
	pub(crate) user_agent: String,
	pub(crate) accept: String,
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) timeout_millis: u64,
	pub(crate) method: HttpMethod,
	pub(crate) version: HttpVersion,
	pub(crate) connection_type: HttpConnectionType,
	pub(crate) http_content_reader: HttpContentReader,
}

pub(crate) struct HttpResponseImpl {
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) code: u16,
	pub(crate) status_text: String,
	pub(crate) version: HttpVersion,
	pub(crate) http_content_reader: HttpContentReader,
}

pub(crate) struct HttpConnectionImpl {}
