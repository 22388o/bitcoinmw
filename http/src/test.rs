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
	use bmw_evh::{EventHandlerConfig, WriteHandle};
	use bmw_http::*;
	use bmw_log::*;
	use bmw_test::*;
	use std::collections::{HashMap, HashSet};
	use std::fs::File;
	use std::io::{Read, Write};
	use std::thread::sleep;
	use std::time::Duration;

	debug!();

	fn callback(
		headers: &HttpHeaders,
		_config: &HttpConfig,
		_instance: &HttpInstance,
		write_handle: &mut WriteHandle,
		_http_connection_data: HttpContentReader,
	) -> Result<(), Error> {
		let path = headers.path()?;
		let query = headers.query()?;

		if path == "/sleep" {
			let mut query = query.split('=');
			let time = query.nth_back(0).unwrap();

			sleep(Duration::from_millis(time.parse()?));
			write_handle.write(
				b"HTTP/1.1 200 Ok\r\nServer: test\r\nContent-Length: 10\r\n\r\n0123456789",
			)?;
		}
		Ok(())
	}

	fn build_server(directory: &str) -> Result<(u16, Box<dyn HttpServer>, &str), Error> {
		setup_test_dir(directory)?;
		let port = pick_free_port()?;
		let addr = "127.0.0.1".to_string();

		let mut callback_mappings = HashSet::new();
		callback_mappings.insert("/sleep".to_string());

		let config = HttpConfig {
			evh_config: EventHandlerConfig {
				threads: 20,
				..Default::default()
			},
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), directory.to_string())]),
				}),
				callback_mappings,
				callback: Some(callback),
				..Default::default()
			}],
			base_dir: directory.to_string(),
			server_name: "bitcoinmwtest".to_string(),
			server_version: "test1".to_string(),
			debug: false,
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
	fn test_http_client_errors() -> Result<(), Error> {
		let test_dir = ".test_http_client_closed_port";
		setup_test_dir(test_dir)?;
		let port = pick_free_port()?;
		let url = format!("http://127.0.0.1:{}/", port);

		// error because Threads speecified twice
		assert!(http_client_init!(BaseDir(test_dir), Threads(10), Threads(20)).is_err());

		// BaseDir twice
		assert!(http_client_init!(BaseDir(test_dir), BaseDir("."), Threads(10)).is_err());

		http_client_init!(BaseDir(test_dir))?;

		let request = http_client_request!(Url(&url))?;

		// Error because no server is listening on this port
		assert!(http_client_send!(request).is_err());
		tear_down_test_dir(test_dir)?;

		// start a server
		let http = build_server(test_dir)?;

		let request = http_client_request!(Uri("/abc.html"))?;
		// Uris can only be specified on a URL connection. A url must be specfied on
		// http_client_sends without a connection that's already established.
		assert!(http_client_send!(request).is_err());

		// host entered twice
		assert!(http_connection!(Host("127.0.0.1"), Host("127.0.0.1")).is_err());

		// invalid host
		assert!(http_connection!(Host("")).is_err());

		let host = "127.0.0.1";
		let port = http.0;
		let mut connection = http_connection!(Host(host), Port(port), Tls(false))?;
		let request = http_client_request!(Uri("/test.html"))?;

		// can't send urls without a connection
		assert!(http_client_send!(request.clone()).is_err());
		assert!(http_client_send!([request], connection, {
			trace!("got response")?;
			Ok(())
		})
		.is_ok());

		let request = http_client_request!(Url("http://www.google.com"))?;
		// error because connections can only accept Uris not Urls
		assert!(http_client_send!([request], connection, {
			trace!("got response")?;
			Ok(())
		})
		.is_err());

		let request = http_client_request!(
			Url(&format!("http://localhost:{}/sleep?time=10000", port)),
			TimeoutMillis(3_000)
		)?;

		info!("about to send http_client request with 3 second timeout")?;
		// this will error out because the response sleeps for 10 seconds and timeout is
		// set to 3 seconds.
		assert!(http_client_send!(request).is_err());
		info!("error asserted correctly")?;

		let request = http_client_request!(
			Url(&format!("http://localhost:{}/sleep?time=3000", port)),
			TimeoutMillis(10_000)
		)?;

		// the inverse is ok because timeout is 10 seconds, but response happens in 3 seconds
		info!("sending request should respond in 3 seconds")?;
		let response = http_client_send!(request)?;
		info!("got response")?;
		assert_eq!(response.code().unwrap(), 200);

		tear_down_server(http)?;
		info!("tear down complete")?;

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

		let mut hcr = response.content_reader()?;
		let mut buf = [0u8; 1_000];
		let mut content = vec![];
		loop {
			let len = hcr.read(&mut buf)?;
			if len == 0 {
				break;
			}
			content.extend(&buf[0..len]);
		}
		let content = std::str::from_utf8(&content).unwrap();

		assert_eq!(content, "Hello test World!");

		tear_down_server(http)?;

		Ok(())
	}
}
