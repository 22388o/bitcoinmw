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

use bmw_log::*;

info!();

#[macro_export]
macro_rules! http_client_init {
	() => {{
		bmw_http::HttpClientContainer::init(&bmw_http::HttpClientConfig::default())
	}};
}

#[cfg(test)]
mod test {
	use crate as bmw_http;
	use crate::{Builder, HttpConfig, HttpInstance, HttpInstanceType, PlainConfig};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use std::collections::HashMap;

	info!();

	#[test]
	fn test_http_macros_basic() -> Result<(), Error> {
		let test_dir = ".test_http_macros_basic.bmw";
		setup_test_dir(test_dir)?;

		let port = pick_free_port()?;
		info!("port={}", port)?;
		let addr = "127.0.0.1".to_string();

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		// begin macros
		http_client_init!()?;

		tear_down_test_dir(test_dir)?;
		Ok(())
	}
}
