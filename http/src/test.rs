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

#[cfg(test)]
mod test {
	use crate as bmw_http;
	use bmw_err::*;
	use bmw_evh::{EventHandlerConfig, ThreadContext, WriteHandle, WriteState};
	use bmw_http::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::{HashMap, HashSet};
	use std::fs::File;
	use std::io::{Read, Write};
	use std::sync::mpsc::sync_channel;
	use std::thread::sleep;
	use std::time::Duration;

	debug!();

	fn callback(
		headers: &HttpHeaders,
		_config: &HttpConfig,
		_instance: &HttpInstance,
		write_handle: &mut WriteHandle,
		mut http_connection_data: HttpContentReader,
		_write_state: Box<dyn LockBox<WriteState>>,
		_thread_context: &mut ThreadContext,
	) -> Result<bool, Error> {
		let path = headers.path()?;
		let query = headers.query()?;

		if path == "/sleep" {
			let mut query = query.split('=');
			let time = query.nth_back(0).unwrap();

			let millis = time.parse()?;
			if millis > 0 {
				sleep(Duration::from_millis(millis));
			}
			write_handle.write(
				b"HTTP/1.1 200 OK\r\nServer: test\r\nContent-Length: 10\r\n\r\n0123456789",
			)?;
		} else if path == "/content" {
			let mut buf = [0u8; 100];
			http_connection_data.read(&mut buf)?;
			assert_eq!(&buf[0..4], b"test");
			write_handle.write(
				b"HTTP/1.1 200 OK\r\nServer: test\r\nContent-Length: 10\r\n\r\nabcdefghij",
			)?;
		}
		Ok(false)
	}

	fn ws_handler(
		message: &WebSocketMessage,
		_config: &HttpConfig,
		_instance: &HttpInstance,
		wsh: &mut WebSocketHandle,
		websocket_data: &WebSocketData,
		_thread_context: &mut ThreadContext,
	) -> Result<(), Error> {
		let text = std::str::from_utf8(&message.payload[..]).unwrap_or("utf8err");
		let message_type = &message.mtype;
		if text == "hello2" {
			info!("hello2 recv")?;
		} else {
			assert_eq!(text, "hello");
			info!(
				"in test ws handler. got message [proto='{:?}'] [path='{}'] [query='{}'] = '{}', mtype={:?}",
				websocket_data.negotiated_protocol, websocket_data.path, websocket_data.query, text, message_type
			)?;

			let wsm = WebSocketMessage {
				mtype: WebSocketMessageType::Text,
				payload: "abcde".as_bytes().to_vec(),
			};
			wsh.send(&wsm)?;
			let wsm = WebSocketMessage {
				mtype: WebSocketMessageType::Text,
				payload: "xyz".as_bytes().to_vec(),
			};
			wsh.send(&wsm)?;
		}
		Ok(())
	}

	fn build_server(
		directory: &str,
		tls: bool,
		port: u16,
	) -> Result<(u16, Box<dyn HttpServer>, &str), Error> {
		let addr = "127.0.0.1".to_string();

		let mut callback_mappings = HashSet::new();
		callback_mappings.insert("/sleep".to_string());
		callback_mappings.insert("/content".to_string());

		let mut websocket_mappings = HashMap::new();
		let mut test_ws_protos = HashSet::new();
		test_ws_protos.insert("test".to_string());
		test_ws_protos.insert("testv2".to_string());
		websocket_mappings.insert("/chat".to_string(), HashSet::new());
		websocket_mappings.insert("/testws".to_string(), test_ws_protos);

		let config = HttpConfig {
			evh_config: EventHandlerConfig {
				threads: 10,
				..Default::default()
			},
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: match tls {
					true => HttpInstanceType::Tls(TlsConfig {
						http_dir_map: HashMap::from([("*".to_string(), directory.to_string())]),
						cert_file: "./resources/cert.pem".to_string(),
						privkey_file: "./resources/key.pem".to_string(),
					}),

					false => HttpInstanceType::Plain(PlainConfig {
						http_dir_map: HashMap::from([("*".to_string(), directory.to_string())]),
					}),
				},
				callback_mappings,
				callback: Some(callback),
				websocket_handler: Some(ws_handler),
				websocket_mappings,
				..Default::default()
			}],
			base_dir: directory.to_string(),
			server_name: "bitcoinmwtest".to_string(),
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = bmw_http::Builder::build_http_server(&config)?;
		http.start()?;
		Ok((port, http, directory))
	}

	fn tear_down_server(mut sc: (u16, Box<dyn HttpServer>, &str)) -> Result<(), Error> {
		sc.1.stop()?;
		Ok(())
	}

	#[test]
	fn test_http_client_tls() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();
		let port = test_info.port();
		let http = build_server(test_dir, true, port)?;

		http_client_init!(BaseDir(test_dir))?;
		let url = &format!("https://localhost:{}/sleep?time=0", http.0);
		let request = http_client_request!(
			Url(url),
			FullChainCertFile("./resources/cert.pem"),
			TimeoutMillis(30_000)
		)?;
		let response = http_client_send!(request)?;

		assert_eq!(response.code().unwrap(), 200);

		info!("response: {}", response)?;

		let mut connection = http_connection!(
			Host("localhost"),
			Port(http.0),
			Tls(true),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let request = http_client_request!(Uri("/sleep?time=0"))?;
		let guid = request.guid();
		let (tx, rx) = sync_channel(1);
		let count = lock_box!(0)?;
		let mut count_clone = count.clone();
		http_client_send!([request], connection, {
			let request = http_client_request!()?;
			let response = http_client_response!()?;
			info!("got a response")?;
			assert_eq!(request.guid(), guid);
			assert_eq!(response.code()?, 200);
			assert_eq!(response.status_text()?, "OK");
			info!("guid match")?;
			wlock!(count_clone) += 1;

			tx.send(())?;
			Ok(())
		})?;

		rx.recv()?;
		assert_eq!(rlock!(count), 1);

		tear_down_server(http)?;
		Ok(())
	}

	#[test]
	fn test_http_client_errors() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();
		let port = test_info.port();
		let url = format!("http://127.0.0.1:{}/", port);

		// error because Threads speecified twice
		assert!(http_client_init!(BaseDir(test_dir), Threads(10), Threads(20)).is_err());

		// BaseDir twice
		assert!(http_client_init!(BaseDir(test_dir), BaseDir("."), Threads(10)).is_err());

		http_client_init!(BaseDir(test_dir))?;

		let request = http_client_request!(Url(&url))?;

		// Error because no server is listening on this port
		assert!(http_client_send!(request).is_err());

		// start a server
		let http = build_server(test_dir, false, port)?;

		let data_text = "Hello test World!";
		{
			let mut file = File::create(format!("{}/foo.html", test_dir))?;
			file.write_all(data_text.as_bytes())?;
		}

		let request = http_client_request!(Uri("/abc.html"))?;
		// Uris can only be specified on a URL connection. A url must be specfied on
		// http_client_sends without a connection that's already established.
		assert!(http_client_send!(request).is_err());

		// host entered twice
		assert!(http_connection!(Host("127.0.0.1"), Host("127.0.0.1")).is_err());

		let host = "127.0.0.1";
		let port = http.0;
		let mut connection = http_connection!(Host(host), Port(port), Tls(false))?;
		let request = http_client_request!(Uri("/test.html"))?;

		// can't send uris without a connection
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
			Url(&format!("http://localhost:{}/sleep?time=3000", port)),
			TimeoutMillis(10_000)
		)?;

		// can't set timeouts for async
		assert!(http_client_send!([request], {
			trace!("got response")?;
			Ok(())
		})
		.is_err());

		let request = http_client_request!(
			Url(&format!("http://localhost:{}/sleep?time=3000", port)),
			TimeoutMillis(10_000)
		)?;

		// can't set timeouts for async (connections also)
		assert!(http_client_send!([request], connection, {
			trace!("got response")?;
			Ok(())
		})
		.is_err());

		let request = http_client_request!(
			Url(&format!("http://localhost:{}/sleep?time=3000", port)),
			TimeoutMillis(10_000)
		)?;

		// this request is ok because timeout is 10 seconds, but response happens in 3 seconds
		info!("sending request should respond in 3 seconds")?;
		let response = http_client_send!(request)?;
		info!("got response")?;
		assert_eq!(response.code().unwrap(), 200);

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
			Url(&format!("http://localhost:{}/foo.html", port)),
			Method(HttpMethod::GET)
		)?;

		let response = http_client_send!(request)?;
		assert_eq!(response.code().unwrap(), 200);

		// now send a POST which will not be allowed (error code 405)
		let request = http_client_request!(
			Url(&format!("http://localhost:{}/foo.html", port)),
			Method(HttpMethod::POST)
		)?;

		let response = http_client_send!(request)?;
		assert_eq!(response.code().unwrap(), 405);

		// now use a post on the sleep callback (should succeed)
		let request = http_client_request!(
			Url(&format!("http://localhost:{}/sleep?time=0", port)),
			Method(HttpMethod::POST)
		)?;

		let response = http_client_send!(request)?;
		assert_eq!(response.code().unwrap(), 200);

		let count = lock_box!(0)?;
		let mut count_clone1 = count.clone();
		let mut count_clone2 = count.clone();
		info!("send request")?;
		let mut connection = http_connection!(Host(host), Port(port), Tls(false))?;
		let request = http_client_request!(Uri("/foo.html"), Version(HttpVersion::HTTP11))?;
		http_client_send!([request], connection, {
			let response = http_client_response!()?;
			assert_eq!(response.version().unwrap(), &HttpVersion::HTTP11);
			info!("got response. version: {}", response.version().unwrap())?;
			wlock!(count_clone1) += 1;
			Ok(())
		})?;

		let request = http_client_request!(Uri("/test.html"), Version(HttpVersion::HTTP10))?;
		http_client_send!([request], connection, {
			let response = http_client_response!()?;
			assert_eq!(response.version().unwrap(), &HttpVersion::HTTP10);
			info!("got response2. version: {}", response.version().unwrap())?;
			wlock!(count_clone2) += 1;
			Ok(())
		})?;

		let mut counter = 0;
		loop {
			sleep(Duration::from_millis(1));
			counter += 1;
			if counter > 10_000 || rlock!(count) == 2 {
				break;
			}
		}

		assert_eq!(rlock!(count), 2);
		tear_down_server(http)?;

		Ok(())
	}

	#[test]
	fn test_http_client_server() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let test_dir = test_info.directory();
		let http = build_server(test_dir, false, port)?;

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

	#[test]
	fn test_http_client_send_content() -> Result<(), Error> {
		let test_info = test_info!()?;
		let test_dir = test_info.directory();

		let http = build_server(test_dir, false, test_info.port())?;
		let addr = format!("http://127.0.0.1:{}", http.0);

		http_client_init!(BaseDir(test_dir))?;

		let request = http_client_request!(
			Url(&format!("{}/content", addr)),
			ContentData(b"test\n"),
			Method(HttpMethod::POST)
		)?;

		let response = http_client_send!(request)?;

		assert_eq!(response.code()?, 200);

		let mut reader = response.content_reader()?;
		let mut buf = [0u8; 100];
		let len = reader.read(&mut buf)?;

		assert_eq!(len, 10);
		assert_eq!(&buf[0..len], b"abcdefghij");

		let request = http_client_request!(
			Url(&format!("{}/content", addr)),
			ContentFile("./resources/content_test.txt"),
			Method(HttpMethod::POST)
		)?;

		let response = http_client_send!(request)?;

		let mut reader = response.content_reader()?;
		let len = reader.read(&mut buf)?;

		assert_eq!(len, 10);
		assert_eq!(&buf[0..len], b"abcdefghij");

		Ok(())
	}

	#[test]
	fn test_http_connection_keep_alive() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let test_dir = test_info.directory();

		let http = build_server(test_dir, false, port)?;

		let data_text = "Hello test World!";
		{
			let mut file = File::create(format!("{}/foo.html", test_dir))?;
			file.write_all(data_text.as_bytes())?;
		}

		http_client_init!(BaseDir(test_dir))?;
		let mut connection = http_connection!(Host("127.0.0.1"), Port(http.0), Tls(false))?;

		let request = http_client_request!(Uri("/foo.html"), KeepAlive(true))?;

		let (tx, rx) = sync_channel(1);
		http_client_send!([request], connection, {
			let response = http_client_response!()?;
			assert_eq!(response.code()?, 200);
			tx.send(())?;
			Ok(())
		})?;

		rx.recv()?;

		info!("complete")?;

		let request = http_client_request!(Uri("/foo.html"), KeepAlive(false))?;

		let (tx, rx) = sync_channel(1);
		http_client_send!([request], connection, {
			let response = http_client_response!()?;
			assert_eq!(response.code()?, 200);
			tx.send(())?;
			Ok(())
		})?;

		rx.recv()?;

		info!("complete2")?;

		let request = http_client_request!(Uri("/foo.html"))?;

		assert!(http_client_send!([request], connection, {
			let response = http_client_response!()?;
			assert_eq!(response.code()?, 200);
			Ok(())
		})
		.is_err());

		tear_down_server(http)?;

		Ok(())
	}

	#[test]
	fn simple1() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let test_dir = test_info.directory();

		let http = build_server(test_dir, false, port)?;
		let addr = format!("http://127.0.0.1:{}", http.0);

		http_client_init!(BaseDir(test_dir))?;

		let request = http_client_request!(
			Url(&format!("{}/content", addr)),
			ContentData(b"test\n"),
			Method(HttpMethod::POST)
		)?;

		let response = http_client_send!(request)?;

		assert_eq!(response.code()?, 200);

		tear_down_server(http)?;

		Ok(())
	}

	#[test]
	fn test_ws_client_basic() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let test_dir = test_info.directory();

		let http = build_server(test_dir, false, port)?;

		let config = WebSocketClientConfig {
			..Default::default()
		};
		let mut ws = crate::Builder::build_websocket_client(&config)?;
		let config = WebSocketConnectionConfig {
			host: "127.0.0.1".to_string(),
			port,
			tls: false,
			path: "/chat".to_string(),
			..Default::default()
		};

		let mut abcde_lock = lock_box!(0)?;
		let mut xyz_lock = lock_box!(0)?;
		let abcde_lock_clone = abcde_lock.clone();
		let xyz_lock_clone = xyz_lock.clone();

		let handler = Box::pin(move |msg: &WebSocketMessage, wsh: &mut WebSocketHandle| {
			info!("in handler: {:?}", msg)?;
			if msg.payload == &[97, 98, 99, 100, 101] {
				wlock!(abcde_lock) += 1;
			} else if msg.payload == &[120, 121, 122] {
				wlock!(xyz_lock) += 1;
				let wsm = WebSocketMessage {
					mtype: WebSocketMessageType::Text,
					payload: b"hello2".to_vec(),
				};
				wsh.send(&wsm)?;
			}
			Ok(())
		});
		let mut ws_connection = ws.connect(&config, handler)?;
		let wsm = WebSocketMessage {
			mtype: WebSocketMessageType::Text,
			payload: b"hello".to_vec(),
		};
		sleep(Duration::from_millis(100));
		ws_connection.send(&wsm)?;

		sleep(Duration::from_millis(3_000));

		assert_eq!(rlock!(abcde_lock_clone), 1);
		assert_eq!(rlock!(xyz_lock_clone), 1);
		tear_down_server(http)?;

		Ok(())
	}

	#[test]
	fn test_websocket_macros() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let test_dir = test_info.directory();

		let http = build_server(test_dir, false, port)?;
		let port = http.0;

		websocket_client_init!(Threads(2))?;

		let url = format!("ws://127.0.0.1:{}/chat", port);
		let config = websocket_connection_config!(Url(&url))?;
		let mut first = lock_box!(true)?;
		let first_clone = first.clone();

		let mut ws_conn = websocket_connection!(&config, {
			info!("got a message")?;
			let message = websocket_message!()?;
			info!("msg={:?}", message)?;
			let mut handle = websocket_handle!()?;

			if rlock!(first) {
				info!("sending first message")?;
				let to_send = try_into!(b"hello")?;
				handle.send(&to_send)?;
			}
			wlock!(first) = false;
			Ok(())
		})?;

		let msg = try_into!("hello")?;

		sleep(Duration::from_millis(1_000));
		ws_conn.send(&msg)?;

		sleep(Duration::from_millis(3_000));

		assert!(!rlock!(first_clone));
		websocket_client_stop!()?;

		tear_down_server(http)?;

		Ok(())
	}
}
