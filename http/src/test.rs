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

#[cfg(test)]
mod test {
	use crate as bmw_http;
	use bmw_err::*;
	use bmw_http::*;
	use bmw_log::*;
	use bmw_test::*;
	use std::collections::HashMap;
	use std::fs::File;
	use std::io::Write;

	debug!();

	fn build_server(directory: &str) -> Result<(u16, Box<dyn HttpServer>, &str), Error> {
		setup_test_dir(directory)?;
		let port = pick_free_port()?;
		let addr = "127.0.0.1".to_string();

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), directory.to_string())]),
				}),
				..Default::default()
			}],
			base_dir: directory.to_string(),
			server_name: "bitcoinmwtest".to_string(),
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;
		Ok((port, http, directory))
	}

	fn tear_down_server(mut sc: (u16, Box<dyn HttpServer>, &str)) -> Result<(), Error> {
		sc.1.stop()?;
		tear_down_test_dir(sc.2)?;
		Ok(())
	}

	#[test]
	fn test_http_client_server() -> Result<(), Error> {
		let test_dir = ".test_http_client_server.bmw";
		let http = build_server(test_dir)?;
		let addr = format!("http://127.0.0.1:{}", http.0);

		let data_text = "Hello test World!";
		{
			let mut file = File::create(format!("{}/foo.html", test_dir))?;
			file.write_all(data_text.as_bytes())?;
		}

		http_client_init!(BaseDir(test_dir))?;

		let request = http_client_request!(Url(&format!("{}/foo.html", addr)))?;

		let response = http_client_send!(request)?;
		let headers = response.headers()?;

		assert_eq!(response.code().unwrap(), 200);
		assert_eq!(
			std::str::from_utf8(response.content().unwrap()).unwrap(),
			"Hello test World!"
		);

		let mut found_server = false;
		let mut found_date = false;
		let mut found_content_type = false;
		let mut found_accept_ranges = false;
		let mut found_connection = false;
		let mut found_last_modified = false;
		let mut found_etag = false;
		let mut found_transfer_encoding = false;

		for header in headers {
			if header.0 == "Server" {
				found_server = true;
				assert_eq!(header.1, "bitcoinmwtest test1");
			} else if header.0 == "Date" {
				found_date = true;
			} else if header.0 == "Content-Type" {
				found_content_type = true;
				assert_eq!(header.1, "text/html");
			} else if header.0 == "Accept-Ranges" {
				found_accept_ranges = true;
				assert_eq!(header.1, "bytes");
			} else if header.0 == "Connection" {
				found_connection = true;
				// http client sends connection close for this type of request
				// since  it's the only request we're sending, so server should reply
				// with a close as well
				assert_eq!(header.1, "close");
			} else if header.0 == "Last-Modified" {
				found_last_modified = true;
			} else if header.0 == "ETag" {
				found_etag = true;
			} else if header.0 == "Transfer-Encoding" {
				found_transfer_encoding = true;
				assert_eq!(header.1, "chunked");
			}
		}

		assert!(found_server);
		assert!(found_date);
		assert!(found_content_type);
		assert!(found_accept_ranges);
		assert!(found_connection);
		assert!(found_last_modified);
		assert!(found_etag);
		assert!(found_transfer_encoding);

		tear_down_server(http)?;

		Ok(())
	}
}
