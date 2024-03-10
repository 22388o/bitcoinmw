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
use bmw_deps::downcast::{downcast, Any};
use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::*;
use bmw_evh::{
	ConnectionData, EventHandlerConfig, EventHandlerController, Handle, ThreadContext, WriteHandle,
	WriteState,
};
use bmw_util::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::pin::Pin;

#[derive(Debug, PartialEq, Clone)]
pub enum HttpMethod {
	GET,
	POST,
	HEAD,
	PUT,
	DELETE,
	OPTIONS,
	CONNECT,
	TRACE,
	PATCH,
	UNKNOWN,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HttpVersion {
	HTTP10,
	HTTP11,
	UNKNOWN,
	OTHER,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectionType {
	KeepAlive,
	CLOSE,
}

#[derive(Debug, Clone, Copy)]
pub struct HttpHeader {
	pub start_header_name: usize,
	pub end_header_name: usize,
	pub start_header_value: usize,
	pub end_header_value: usize,
}

#[derive(Debug)]
pub struct HttpHeaders<'a> {
	pub(crate) termination_point: usize,
	pub(crate) start: usize,
	pub(crate) req: &'a Vec<u8>,
	pub(crate) start_uri: usize,
	pub(crate) end_uri: usize,
	pub(crate) http_request_type: HttpMethod,
	pub(crate) headers: [HttpHeader; 100],
	pub(crate) header_count: usize,
	pub(crate) version: HttpVersion,
	pub(crate) host: String,
	pub(crate) connection: ConnectionType,
	pub(crate) range_start: usize,
	pub(crate) range_end: usize,
	pub(crate) if_none_match: String,
	pub(crate) if_modified_since: String,
	pub(crate) is_websocket_upgrade: bool,
	pub(crate) sec_websocket_key: String,
	pub(crate) sec_websocket_protocol: String,
	pub(crate) accept_gzip: bool,
	pub(crate) content_length: usize,
}

#[derive(Debug)]
pub struct HttpConnectionData {
	pub(crate) last_active: u128,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) tid: usize,
	pub(crate) websocket_data: Option<WebSocketData>,
	pub(crate) headers: Vec<u8>,
	pub(crate) http_content_reader_data: HttpContentReaderData,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpContentReaderData {
	pub(crate) offset: u16,
	pub(crate) head_slab: usize,
	pub(crate) tail_slab: usize,
	pub(crate) read_slab: usize,
	pub(crate) read_offset: usize,
	pub(crate) read_cumulative: usize,
	pub(crate) len: usize,
	pub(crate) content_offset: usize,
	pub(crate) file_id: Option<u128>,
}

#[derive(Clone)]
pub struct HttpContentReader {
	pub(crate) http_content_reader_data: Option<HttpContentReaderData>,
	pub(crate) content_allocator: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
	pub(crate) tmp_file_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WebSocketData {
	pub uri: String,
	pub query: String,
	pub negotiated_protocol: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WebSocketMessageType {
	Text,
	Binary,
	Close,
	Ping,
	Pong,
	Open,
	Accept,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WebSocketMessage {
	pub mtype: WebSocketMessageType,
	pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct WebSocketHandle {
	pub(crate) write_handle: WriteHandle,
}

pub trait HttpServer {
	fn start(&mut self) -> Result<(), Error>;
	fn stop(&mut self) -> Result<(), Error>;
	fn stats(&self) -> Result<HttpStats, Error>;
}

pub struct HttpStats {}

#[derive(Clone, Debug)]
pub struct PlainConfig {
	pub http_dir_map: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct TlsConfig {
	pub cert_file: String,
	pub privkey_file: String,
	pub http_dir_map: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum HttpInstanceType {
	Plain(PlainConfig),
	Tls(TlsConfig),
}

pub trait Attachment: Send + Sync + DynClone + Any {}
clone_trait_object!(Attachment);
downcast!(dyn Attachment);

impl<T: Clone + Any + Send + Sync> Attachment for T {}

impl fmt::Debug for dyn Attachment {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Attachment").finish_non_exhaustive()
	}
}

#[derive(Clone, Debug)]
pub struct HttpInstance {
	pub port: u16,
	pub addr: String,
	pub listen_queue_size: usize,
	pub instance_type: HttpInstanceType,
	pub default_file: Vec<String>,
	pub error_400file: String,
	pub error_403file: String,
	pub error_404file: String,
	pub callback: Option<HttpCallback>,
	pub callback_mappings: HashSet<String>,
	pub callback_extensions: HashSet<String>,
	pub websocket_mappings: HashMap<String, HashSet<String>>,
	pub websocket_handler: Option<WebsocketHandler>,
	pub attachment: Box<dyn Attachment>,
}

#[derive(Clone)]
pub struct HttpConfig {
	pub evh_config: EventHandlerConfig,
	pub instances: Vec<HttpInstance>,
	pub debug: bool,
	pub cache_slab_count: usize,
	pub idle_timeout: u128,
	pub server_name: String,
	pub server_version: String,
	pub mime_map: Vec<(String, String)>,
	pub bring_to_front_weight: f64,
	pub restat_file_frequency_in_millis: u64,
	pub base_dir: String,
	pub content_slab_count: usize,
	pub max_headers_len: usize,
	pub max_header_count: usize,
	pub max_uri_len: usize,
	pub attachment: Option<Box<dyn Attachment>>,
}

pub struct Builder {}

type HttpCallback = fn(
	&HttpHeaders,
	&HttpConfig,
	&HttpInstance,
	&mut WriteHandle,
	HttpContentReader,
	Box<dyn LockBox<WriteState>>,
	&mut ThreadContext,
) -> Result<bool, Error>;

type WebsocketHandler = fn(
	&WebSocketMessage,
	&HttpConfig,
	&HttpInstance,
	&mut WebSocketHandle,
	&WebSocketData,
) -> Result<(), Error>;

pub type HttpHandler = Pin<
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

pub trait HttpRequest: DynClone + Any {
	fn request_url(&self) -> Option<String>;
	fn request_uri(&self) -> Option<String>;
	fn user_agent(&self) -> &String;
	fn accept(&self) -> &String;
	fn headers(&self) -> &Vec<(String, String)>;
	fn method(&self) -> &HttpMethod;
	fn version(&self) -> &HttpVersion;
	fn full_chain(&self) -> &Option<String>;
	fn timeout_millis(&self) -> u64;
	fn content_file(&self) -> &Option<String>;
	fn content_data(&self) -> &Option<Vec<u8>>;
	fn keep_alive(&self) -> bool;
	fn guid(&self) -> u128;
}

clone_trait_object!(HttpRequest);
downcast!(dyn HttpRequest);

pub trait HttpResponse: DynClone + Any {
	fn headers(&self) -> Result<&Vec<(String, String)>, Error>;
	fn code(&self) -> Result<u16, Error>;
	fn status_text(&self) -> Result<&String, Error>;
	fn version(&self) -> Result<&HttpVersion, Error>;
	fn content_reader(&self) -> Result<HttpContentReader, Error>;
}

clone_trait_object!(HttpResponse);
downcast!(dyn HttpResponse);

pub trait HttpClient: DynClone + Any {
	fn send(
		&mut self,
		req: &Box<dyn HttpRequest + Send + Sync>,
		handler: HttpHandler,
	) -> Result<(), Error>;

	fn stop(&mut self) -> Result<(), Error>;

	// TODO: make this crate(pub) by splitting into a separate mod (see
	// https://stackoverflow.com/questions/66786429/how-to-have-a-public-trait-with-a-pubcrate-method-in-a-library)
	fn controller(&mut self) -> &mut EventHandlerController;
	fn config(&self) -> &HttpClientConfig;
}

clone_trait_object!(HttpClient);
downcast!(dyn HttpClient);

pub trait HttpConnection {
	fn send(
		&mut self,
		req: &Box<dyn HttpRequest + Send + Sync>,
		handler: HttpHandler,
	) -> Result<(), Error>;

	fn close(&mut self) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct HttpClientConfig {
	pub max_headers_len: usize,
	pub debug: bool,
	pub evh_threads: usize,
	pub evh_max_handles_per_thread: usize,
	pub evh_sync_channel_size: usize,
	pub evh_write_queue_size: usize,
	pub evh_nhandles_queue_size: usize,
	pub evh_max_events_in: usize,
	pub evh_housekeeping_frequency_millis: u128,
	pub evh_read_slab_count: usize,
	pub evh_max_events: usize,
	pub slab_count: usize,
	pub base_dir: String,
}

#[derive(Clone)]
pub struct HttpConnectionConfig {
	pub host: String,
	pub port: u16,
	pub tls: bool,
	pub full_chain_cert_file: Option<String>,
}

#[derive(Clone)]
pub struct HttpRequestConfig {
	pub request_url: Option<String>,
	pub request_uri: Option<String>,
	pub user_agent: String,
	pub accept: String,
	pub headers: Vec<(String, String)>,
	pub timeout_millis: u64,
	pub method: HttpMethod,
	pub version: HttpVersion,
	pub keep_alive: bool,
	pub full_chain: Option<String>,
	pub content_file: Option<String>,
	pub content_data: Option<Vec<u8>>,
}

pub struct HttpClientContainer {}

/// Configuration options used throughout this crate via macro.
#[derive(Clone, Debug)]
pub enum ConfigOption<'a> {
	/// An HttpInstance.
	Instance(HttpInstance),
	/// Number of threads. Used in [`crate::http_client_init`].
	Threads(usize),
	/// The maximum handles per thread. Used to configure the http client in
	/// [`crate::http_client_init`].
	MaxHandlesPerThread(usize),
	/// Whether to print debugging information. Used to configure http client in
	/// [`crate::http_client_init`].
	Debug(bool),
	/// Maximum headers length. Used to configure http client in
	/// [`crate::http_client_init`].
	MaxHeadersLen(usize),
	/// EventHandler SyncChannel size. Used to configure http client in
	/// [`crate::http_client_init`].
	SyncChannelSize(usize),
	/// EventHandler WriteQueueSize. Used to configure http client in
	/// [`crate::http_client_init`].
	WriteQueueSize(usize),
	/// EventHandler Nhandles queue size. Used to configure http client in
	/// [`crate::http_client_init`].
	NhandlesQueueSize(usize),
	/// Max Events In for EventHandler. Used to configure http client in
	/// [`crate::http_client_init`].
	MaxEventsIn(usize),
	/// MaxEvents for EventHandler. Used to configure http client in
	/// [`crate::http_client_init`].
	MaxEvents(usize),
	/// Eventhandler housekeeping frequency. Used to configure http client in
	/// [`crate::http_client_init`].
	HouseKeepingFrequencyMillis(u128),
	/// EventHandler Read Slab count. Used to configure http client in
	/// [`crate::http_client_init`].
	EvhReadSlabCount(usize),
	/// Http client's own slab count. Used to configure http client in
	/// [`crate::http_client_init`].
	SlabCount(usize),
	/// Url. Used to specify the url in [`crate::http_client_request`].
	Url(&'a str),
	/// Uri. Used to specify the uri in [`crate::http_client_request`].
	Uri(&'a str),
	/// Http User-Agent. Used to specify the User-Agent header in [`crate::http_client_request`].
	/// The default value is BitcoinMW/`BuildVersion`.
	UserAgent(&'a str),
	/// Http Accept. Used to specify the Http Accept header in [`crate::http_client_request`].
	/// The default value is `*/*`.
	Accept(&'a str),
	/// Http Header name/value pair. Used to add an http header to a request in
	/// [`crate::http_client_request`].
	Header((&'a str, &'a str)),
	/// Host to connect to. Used for [`crate::http_connection`].
	Host(&'a str),
	/// Port to connect to. Used for [`crate::http_connection`].
	Port(u16),
	/// Whether to use TLS for a connection. Used for [`crate::http_connection`].
	Tls(bool),
	/// Base directory for the [`crate::HttpClient`]. The default value is `~/.bmw`.
	BaseDir(&'a str),
	/// Timeout in milliseconds for [`crate::http_client_request`]. Note: this may only be used
	/// for synchronous requests.
	TimeoutMillis(u64),
	/// HttpMethod used in [`crate::http_client_request`].
	Method(HttpMethod),
	/// HttpVersion used in [`crate::http_client_request`].
	Version(HttpVersion),
	/// Full chain cert file to trust for an [`crate::http_client_request`].
	FullChainCertFile(&'a str),
	/// Private key file.
	PrivKeyFile(&'a str),
	/// Specify tls configuration files.
	TlsServerConfig(TlsConfigFiles),
	/// Content to be sent in a [`crate::http_client_request`].
	ContentData(&'a [u8]),
	/// The contents of the specified path will be sent to the server as the content for this
	/// [`crate::http_client_request`].
	ContentFile(&'a str),
	/// For [`crate::http_connection`]'s, whether or not to keep the connection alive.
	KeepAlive(bool),
}

#[derive(Debug, Clone)]
pub struct TlsConfigFiles {
	pub cert_file: String,
	pub privkey_file: String,
}

// Crate local types

#[derive(Clone)]
pub(crate) struct HttpClientImpl {
	pub(crate) controller: EventHandlerController,
	pub(crate) config: HttpClientConfig,
}

#[derive(Clone)]
pub(crate) struct HttpRequestImpl {
	pub(crate) config: HttpRequestConfig,
	pub(crate) guid: u128,
}

#[derive(Clone)]
pub(crate) struct HttpResponseImpl {
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) chunked: bool,
	pub(crate) content_length: usize,
	pub(crate) start_content: usize,
	pub(crate) code: u16,
	pub(crate) status_text: String,
	pub(crate) version: HttpVersion,
	pub(crate) keep_alive: bool,
	pub(crate) content_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	pub(crate) tmp_file_dir: PathBuf,
	pub(crate) http_content_reader_data: HttpContentReaderData,
}
pub(crate) struct HttpConnectionImpl {
	pub(crate) config: HttpConnectionConfig,
	pub(crate) wh: WriteHandle,
	pub(crate) http_client_data: Box<dyn LockBox<VecDeque<HttpClientAttachmentData>>>,
}

pub(crate) struct HttpClientContext {
	pub(crate) slab_start: usize,
	pub(crate) suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	pub(crate) matches: [Match; 1_000],
}

pub(crate) struct HttpClientAttachment {
	pub(crate) http_client_data: Box<dyn LockBox<VecDeque<HttpClientAttachmentData>>>,
}

pub(crate) struct HttpClientAttachmentData {
	pub(crate) request: Box<dyn HttpRequest + Send + Sync>,
	pub(crate) close_on_complete: bool,
	pub(crate) handler: HttpHandler,
}

pub(crate) struct HttpServerImpl {
	pub(crate) config: HttpConfig,
	pub(crate) cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
	pub(crate) controller: Option<EventHandlerController>,
	pub(crate) handles: Option<Array<Handle>>,
}

pub(crate) struct HttpCacheImpl {
	pub(crate) hashtable: Box<dyn Hashtable<String, usize> + Send + Sync>,
}

pub(crate) struct HttpContext {
	pub(crate) suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	pub(crate) matches: [Match; 1_000],
	pub(crate) connections: HashMap<u128, HttpConnectionData>,
	pub(crate) mime_map: HashMap<String, String>,
	pub(crate) mime_lookup: HashMap<u32, String>,
	pub(crate) mime_rev_lookup: HashMap<String, u32>,
	pub(crate) now: u128,
	pub(crate) content_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	pub(crate) thread_context: ThreadContext,
}

#[derive(PartialEq, Debug)]
pub(crate) enum CacheStreamResult {
	Hit,
	Miss,
	Modified,
	NotModified,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FrameType {
	Continuation,
	Text,
	Binary,
	Close,
	Ping,
	Pong,
}

#[derive(Debug, PartialEq)]
pub(crate) struct FrameHeader {
	pub(crate) ftype: FrameType,     // which type of frame is this?
	pub(crate) mask: bool,           // is this frame masked?
	pub(crate) fin: bool,            // is this the last piece of data in the frame?
	pub(crate) payload_len: usize,   // size of the payload
	pub(crate) masking_key: u32,     // masking key
	pub(crate) start_content: usize, // start of the content of the message
}

pub(crate) trait HttpCache {
	fn stream_file(
		&self,
		path: &String,
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
		ctx: &HttpContext,
		config: &HttpConfig,
		headers: &HttpHeaders,
		gzip: bool,
	) -> Result<CacheStreamResult, Error>;
	fn write_metadata(
		&mut self,
		path: &String,
		len: usize,
		last_modified: u64,
		mime_type: u32,
		now: u64,
		gzip: bool,
	) -> Result<bool, Error>;
	fn write_block(
		&mut self,
		path: &String,
		offset: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
		gzip: bool,
	) -> Result<(), Error>;
	fn bring_to_front(&mut self, path: &String, gzip: bool) -> Result<(), Error>;
	fn remove(&mut self, path: &String, gzip: bool) -> Result<(), Error>;
	fn update_last_checked_if_needed(
		&mut self,
		fpath: &String,
		ctx: &HttpContext,
		config: &HttpConfig,
		gzip: bool,
	) -> Result<(), Error>;
}
