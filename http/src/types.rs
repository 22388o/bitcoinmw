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

use bmw_deps::rand::random;
use bmw_err::*;
use bmw_evh::{ConnectionData, EventHandlerConfig};
use bmw_util::*;
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub enum HttpRequestType {
	GET,
	POST,
	HEAD,
	UNKNOWN,
}

#[derive(Debug, PartialEq)]
pub enum HttpVersion {
	HTTP10,
	HTTP11,
	UNKNOWN,
	OTHER,
}

#[derive(Clone, Copy)]
pub struct HttpHeader {
	pub start_header_name: usize,
	pub end_header_name: usize,
	pub start_header_value: usize,
	pub end_header_value: usize,
}

pub struct HttpHeaders<'a> {
	pub(crate) termination_point: usize,
	pub(crate) start: usize,
	pub(crate) req: &'a Vec<u8>,
	pub(crate) start_uri: usize,
	pub(crate) end_uri: usize,
	pub(crate) http_request_type: HttpRequestType,
	pub(crate) headers: [HttpHeader; 100],
	pub(crate) header_count: usize,
	pub(crate) version: HttpVersion,
}

pub trait HttpCache {
	fn stream_file(
		&self,
		path: &String,
		len: u64,
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
	) -> Result<bool, Error>;
	fn write_block(&mut self, path: &String, offset: u64, data: &[u8]) -> Result<(), Error>;
	fn bring_to_front(&mut self, path: &String) -> Result<(), Error>;
}

pub trait HttpServer {
	fn start(&mut self) -> Result<(), Error>;
	fn stop(&mut self) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub struct PlainConfig {
	pub domainnames: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TlsConfig {
	pub cert_file: String,
	pub privkey_file: String,
	pub domainnames: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum HttpInstanceType {
	Plain(PlainConfig),
	Tls(TlsConfig),
}

#[derive(Clone, Debug)]
pub struct HttpInstance {
	pub port: u16,
	pub addr: String,
	pub listen_queue_size: usize,
	pub http_dir: String,
	pub instance_type: HttpInstanceType,
	pub default_file: Vec<String>,
	pub error_400file: String,
	pub error_403file: String,
	pub error_404file: String,
	pub callback: Option<HttpCallback>,
	pub callback_mappings: HashSet<String>,
	pub callback_extensions: HashSet<String>,
}

#[derive(Clone)]
pub struct HttpConfig {
	pub evh_config: EventHandlerConfig,
	pub instances: Vec<HttpInstance>,
	pub debug: bool,
}

pub struct Builder {}

// Crate local types
pub(crate) struct HttpServerImpl {
	pub(crate) config: HttpConfig,
	pub(crate) cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
}

pub(crate) struct HttpCacheImpl {
	pub(crate) hashtable: Box<dyn Hashtable<String, Vec<String>> + Send + Sync>,
}

pub(crate) struct HttpContext {
	pub(crate) suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	pub(crate) matches: [Match; 1_000],
	pub(crate) offset: usize,
}

type HttpCallback =
	fn(&HttpHeaders, &HttpConfig, &HttpInstance, &mut ConnectionData) -> Result<(), Error>;
