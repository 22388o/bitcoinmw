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

use crate::types::{HttpClientImpl, HttpConnectionImpl, HttpRequestImpl};
use crate::{
	HttpClient, HttpClientConfig, HttpConnection, HttpConnectionConfig, HttpHandler, HttpRequest,
	HttpRequestConfig,
};
use bmw_err::*;

impl HttpClient for HttpClientImpl {
	fn send(&mut self, _req: &Box<dyn HttpRequest>, _handler: &HttpHandler) -> Result<(), Error> {
		todo!()
	}
}

impl HttpClientImpl {
	pub(crate) fn new(_config: &HttpClientConfig) -> Result<HttpClientImpl, Error> {
		Ok(Self {})
	}
}

impl HttpConnection for HttpConnectionImpl {
	fn send(&mut self, _req: &Box<dyn HttpRequest>, _handler: &HttpHandler) -> Result<(), Error> {
		todo!()
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(_config: &HttpConnectionConfig) -> Result<HttpConnectionImpl, Error> {
		Ok(Self {})
	}
}

impl HttpRequest for HttpRequestImpl {}

impl HttpRequestImpl {
	pub(crate) fn new(_config: &HttpRequestConfig) -> Result<HttpRequestImpl, Error> {
		Ok(Self {})
	}
}
