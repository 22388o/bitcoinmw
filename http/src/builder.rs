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

use crate::types::{HttpClientImpl, HttpConnectionImpl, HttpRequestImpl, HttpServerImpl};
use crate::Builder;
use crate::{
	HttpClient, HttpClientConfig, HttpConfig, HttpConnection, HttpConnectionConfig, HttpRequest,
	HttpRequestConfig, HttpServer,
};
use bmw_err::*;

impl Builder {
	pub fn build_http_server(
		config: &HttpConfig,
	) -> Result<Box<dyn HttpServer + Send + Sync>, Error> {
		Ok(Box::new(HttpServerImpl::new(config)?))
	}

	pub fn build_http_client(
		config: &HttpClientConfig,
	) -> Result<Box<dyn HttpClient + Send + Sync>, Error> {
		Ok(Box::new(HttpClientImpl::new(config)?))
	}

	pub fn build_http_connection(
		config: &HttpConnectionConfig,
		http_client: Box<dyn HttpClient + Send + Sync>,
	) -> Result<Box<dyn HttpConnection + Send + Sync>, Error> {
		Ok(Box::new(HttpConnectionImpl::new(config, http_client)?))
	}

	pub fn build_http_request(
		config: &HttpRequestConfig,
	) -> Result<Box<dyn HttpRequest + Send + Sync>, Error> {
		Ok(Box::new(HttpRequestImpl::new(config)?))
	}
}

#[cfg(test)]
mod test {
	use crate::{Builder, HttpConfig, HttpInstance};
	use bmw_err::*;
	use bmw_test::*;

	#[test]
	fn test_http_builder() -> Result<(), Error> {
		let port = pick_free_port()?;
		let test_dir = ".test_http_builder.bmw";
		setup_test_dir(test_dir)?;

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				..Default::default()
			}],
			base_dir: test_dir.to_string(),
			..Default::default()
		};
		let mut server = Builder::build_http_server(&config)?;
		server.start()?;

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
