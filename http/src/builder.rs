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

use crate::types::{HttpClientImpl, HttpConnectionImpl, HttpRequestImpl, HttpServerImpl};
use crate::{HttpBuilder, HttpClient, HttpConnection, HttpRequest, HttpServer};
use bmw_conf::ConfigOption;
use bmw_err::*;

impl HttpBuilder {
	pub fn build_http_server(configs: Vec<ConfigOption>) -> Result<Box<dyn HttpServer>, Error> {
		Ok(Box::new(HttpServerImpl::new(configs)?))
	}

	pub fn build_http_request(configs: Vec<ConfigOption>) -> Result<Box<dyn HttpRequest>, Error> {
		Ok(Box::new(HttpRequestImpl::new(configs)?))
	}

	pub fn build_http_client(configs: Vec<ConfigOption>) -> Result<Box<dyn HttpClient>, Error> {
		Ok(Box::new(HttpClientImpl::new(configs)?))
	}

	pub fn build_http_connection(
		configs: Vec<ConfigOption>,
		http_client: Box<dyn HttpClient>,
	) -> Result<Box<dyn HttpConnection>, Error> {
		Ok(Box::new(HttpConnectionImpl::new(configs, http_client)?))
	}
}
