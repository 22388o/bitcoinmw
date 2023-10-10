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

use bmw_err::*;
use bmw_evh::EventHandlerConfig;
use bmw_util::*;

pub struct HttpHeaders<'a> {
	pub(crate) termination_point: usize,
	pub(crate) start: usize,
	pub(crate) req: &'a Vec<u8>,
	pub(crate) start_uri: usize,
	pub(crate) end_uri: usize,
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
	pub default_file: String,
}

#[derive(Clone)]
pub struct HttpConfig {
	pub evh_config: EventHandlerConfig,
	pub instances: Vec<HttpInstance>,
}

pub struct Builder {}

// Crate local types
pub(crate) struct HttpServerImpl {
	pub(crate) config: HttpConfig,
}

pub(crate) struct HttpContext {
	pub(crate) suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	pub(crate) matches: [Match; 1_000],
	pub(crate) offset: usize,
}
