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

use bmw_deps::downcast::{downcast, Any};
use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::*;
use bmw_evh::*;
use bmw_util::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;

#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
pub enum HttpVersion {
	Http10,
	Http11,
	Other,
	Unknown,
}

#[derive(PartialEq, Debug, Clone)]
pub enum HttpConnectionType {
	KeepAlive,
	Close,
	Unknown,
}

#[derive(Clone)]
pub struct HttpInstance {
	pub(crate) addr: String,
	pub(crate) port: u16,
	pub(crate) dir_map: HashMap<String, String>,
	pub(crate) listen_queue_size: usize,
	pub(crate) callback: Option<HttpCallback>,
	pub(crate) websocket_callback: Option<WebSocketCallback>,
	pub(crate) callback_mappings: HashSet<String>,
	pub(crate) callback_extensions: HashSet<String>,
	pub(crate) websocket_mappings: HashMap<String, HashSet<String>>,
}

pub struct WebSocketMessage {}

pub type HttpCallback = fn(
	headers: &Box<dyn HttpHeaders + '_>,
	content_reader: &mut Option<Box<dyn LockBox<HttpContentReader>>>,
	write_handle: &mut WriteHandle,
	instance: &HttpInstance,
) -> Result<(), Error>;

pub type WebSocketCallback = fn(
	msg: &WebSocketMessage,
	write_handle: &mut WriteHandle,
	instance: &HttpInstance,
) -> Result<(), Error>;

pub type HttpResponseHandler = Pin<
	Box<
		dyn FnMut(&Box<dyn HttpRequest>, &mut Box<dyn HttpResponse>) -> Result<(), Error>
			+ Send
			+ Sync
			+ Unpin,
	>,
>;

/// Builder struct used to build all data strcutures in this crate
pub struct HttpBuilder {}

pub trait HttpServer {
	fn add_instance(&mut self, instance: HttpInstance) -> Result<(), Error>;
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

pub trait HttpRequest: DynClone + Any {
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
	fn http_content_reader(&mut self) -> &mut Box<dyn LockBox<HttpContentReader>>;
}

clone_trait_object!(HttpRequest);
downcast!(dyn HttpRequest);

pub trait HttpResponse {
	fn headers(&self) -> &Vec<(String, String)>;
	fn code(&self) -> u16;
	fn status_text(&self) -> &String;
	fn version(&self) -> &HttpVersion;
	fn http_content_reader(&mut self) -> &mut HttpContentReader;
}

pub trait WSClient {}

pub trait HttpHeaders {
	fn path(&self) -> String;
}

pub struct HttpStats {}

pub struct HttpContentReader {
	pub(crate) content: Option<Box<dyn Read>>,
	pub(crate) content_data: Vec<u8>,
	pub(crate) content_data_offset: usize,
}

// crate local
#[derive(Debug, Clone)]
pub(crate) struct HttpHeadersImpl {
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) content_length: usize,
	pub(crate) end_headers: usize,
	pub(crate) chunked: bool,
	pub(crate) method: HttpMethod,
	pub(crate) uri: String,
	pub(crate) version: HttpVersion,
	pub(crate) status_message: String,
	pub(crate) status_code: u16,
	pub(crate) connection_type: HttpConnectionType,
	pub(crate) host: String,
}

#[derive(Clone)]
pub(crate) struct HttpServerConfig {
	pub(crate) server: String,
	pub(crate) evh_slab_size: usize,
	pub(crate) evh_slab_count: usize,
	pub(crate) debug_no_chunks: bool,
}

#[derive(Clone)]
pub(crate) struct HttpClientConfig {
	pub(crate) tmp_dir: String,
	pub(crate) evh_read_slab_size: usize,
	pub(crate) evh_read_slab_count: usize,
}

pub(crate) struct HttpCache {}

pub(crate) struct HttpServerImpl {
	pub(crate) cache: HttpCache,
	pub(crate) controller: Option<EvhController>,
	pub(crate) config: HttpServerConfig,
	pub(crate) instances: Vec<HttpInstance>,
	pub(crate) instance_table: HashMap<u128, HttpInstance>,
}

pub(crate) struct HttpClientImpl {
	pub(crate) controller: EvhController,
	pub(crate) state: Box<dyn LockBox<HashMap<u128, HttpClientState>>>,
}

pub(crate) struct WSClientImpl {}

#[derive(Clone)]
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
	pub(crate) http_content_reader: Box<dyn LockBox<HttpContentReader>>,
}

pub(crate) struct HttpResponseImpl {
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) code: u16,
	pub(crate) status_text: String,
	pub(crate) version: HttpVersion,
	pub(crate) http_content_reader: HttpContentReader,
	pub(crate) drop_file: Option<PathBuf>,
}

pub(crate) struct HttpConnectionImpl {}

pub(crate) struct HttpClientState {
	pub(crate) queue: VecDeque<HttpClientData>,
	pub(crate) headers: Option<HttpHeadersImpl>,
	pub(crate) offset: usize,
	pub(crate) headers_cleared: bool,
	pub(crate) rid: u128,
}

pub(crate) struct HttpClientData {
	pub(crate) request: Box<dyn HttpRequest>,
	pub(crate) handler: HttpResponseHandler,
}

pub(crate) struct HttpClientContext {
	pub(crate) trie: Box<dyn SearchTrie + Send + Sync>,
}

pub(crate) struct HttpServerContext {
	pub(crate) trie: Box<dyn SearchTrie + Send + Sync>,
	pub(crate) connection_state: HashMap<u128, HttpConnectionState>,
}

#[derive(Clone)]
pub(crate) struct HttpConnectionState {
	pub(crate) is_async: Box<dyn LockBox<bool>>,
	pub(crate) offset: usize,
}
