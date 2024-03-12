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
	use crate as bmw_rustlet;
	use bmw_err::*;
	use bmw_http::*;
	use bmw_rustlet::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
	use std::io::Read;

	debug!();

	fn build_server(directory: &str, tls: bool) -> Result<u16, Error> {
		setup_test_dir(directory)?;
		let port = pick_free_port()?;

		let base_dir = format!("{}/www", directory);
		if tls {
			rustlet_init!(
				instance!(
					BaseDir(&base_dir),
					Port(port),
					TlsServerConfig(tls_config!(
						PrivKeyFile("./resources/key.pem"),
						FullChainCertFile("./resources/cert.pem")
					)?)
				)?,
				BaseDir(directory)
			)?;
		} else {
			rustlet_init!(
				instance!(BaseDir(&base_dir), Port(port))?,
				BaseDir(directory)
			)?;
		}

		websocket!("test", {
			let mut request = websocket_request!()?;
			assert_eq!(request.message().mtype, WebSocketMessageType::Text);
			assert_eq!(request.message().payload, "hello".as_bytes().to_vec());
			request.handle().send(&try_into!(b"test")?)?;
			info!("in test websocket")?;
			Ok(())
		})?;

		rustlet!("test", {
			let mut response = response!()?;
			let request = request!()?;
			info!(
				"in rustlet test_rustlet_simple_request test method={:?},path={}",
				request.method(),
				request.path()
			)?;
			response.write(b"abc")?;
			Ok(())
		})?;
		rustlet!("test2", {
			let mut response = response!()?;
			let request = request!()?;
			info!(
				"in rustlet test_rustlet_simple_request test2 method={:?},path={}",
				request.method(),
				request.path()
			)?;
			assert_eq!(request.method(), &HttpMethod::GET);
			response.set_connection_close()?;
			response.write(b"defg")?;
			Ok(())
		})?;
		rustlet!("method", {
			let mut response = response!()?;
			let request = request!()?;
			info!(
				"in rustlet method. method={:?},path={}",
				request.method(),
				request.path()
			)?;

			if request.query() == &"post" {
				assert_eq!(request.method(), &HttpMethod::POST);
			} else if request.query() == &"trace" {
				assert_eq!(request.method(), &HttpMethod::TRACE);
			} else {
				assert_eq!(request.method(), &HttpMethod::GET);
			}
			response.set_connection_close()?;
			response.write(b"defg")?;
			Ok(())
		})?;
		rustlet!("version", {
			let mut response = response!()?;
			let request = request!()?;
			info!(
				"in rustlet version. method={:?},path={},version={}",
				request.method(),
				request.path(),
				request.version(),
			)?;

			if request.query() == &"11" {
				assert_eq!(request.version(), &HttpVersion::HTTP11);
			} else if request.query() == &"10" {
				assert_eq!(request.version(), &HttpVersion::HTTP10);
			} else if request.query() == &"20" {
				assert_eq!(request.version(), &HttpVersion::OTHER);
			} else {
				assert_eq!(request.version(), &HttpVersion::HTTP11);
			}

			response.set_connection_close()?;
			response.write(b"defg")?;
			Ok(())
		})?;

		rustlet!("redirect", {
			info!("in redirect rustlet")?;
			let mut response = response!()?;
			response.redirect("http://www.example.com")?;

			Ok(())
		})?;

		rustlet!("headers", {
			let request = request!()?;
			let mut response = response!()?;

			for (k, v) in request.headers() {
				if k == "test" {
					assert_eq!(v, "value1");
					response.write(b"1")?;
				} else if k == "test2" {
					assert_eq!(v, "value2");
					response.write(b"2")?;
				}
			}

			Ok(())
		})?;

		rustlet!("content", {
			let request = request!()?;
			let mut r = request.content_reader();

			let mut buf = vec![];
			let len = r.read_to_end(&mut buf)?;

			assert_eq!(len, 7);
			assert_eq!(&buf[0..7], b"test123");
			Ok(())
		})?;

		rustlet!("add_headers", {
			let mut response = response!()?;
			response.add_header("myheader", "myvalue")?;
			response.write(b"success!")?;
			Ok(())
		})?;

		rustlet!("async", {
			let mut response = response!()?;
			let request = request!()?;

			response.write(b"part1")?;
			response.set_async()?;
			std::thread::spawn(move || {
				if false {
					Err(err!(ErrKind::Rustlet, "false"))
				} else {
					assert_eq!(request.method(), &HttpMethod::GET);
					std::thread::sleep(std::time::Duration::from_millis(3_000));
					response.write(b"part2")?;
					response.async_complete()?;
					Ok(())
				}
			});
			Ok(())
		})?;

		rustlet_mapping!("/abc", "test")?;
		rustlet_mapping!("/def", "test2")?;
		rustlet_mapping!("/method", "method")?;
		rustlet_mapping!("/version", "version")?;
		rustlet_mapping!("/redirect", "redirect")?;
		rustlet_mapping!("/headers", "headers")?;
		rustlet_mapping!("/content", "content")?;
		rustlet_mapping!("/add_headers", "add_headers")?;
		rustlet_mapping!("/async", "async")?;

		websocket_mapping!("/chat", "test", vec![])?;

		rustlet_start!()?;

		Ok(port)
	}

	fn tear_down_server(directory: &str) -> Result<(), Error> {
		rustlet_stop!()?;
		tear_down_test_dir(directory)?;
		Ok(())
	}

	#[test]
	fn test_rustlet_simple() -> Result<(), Error> {
		let test_dir = ".test_rustlet_simple.bmw";
		let port = build_server(test_dir, false)?;

		http_client_init!(BaseDir(test_dir))?;
		let url = &format!("http://127.0.0.1:{}/abc", port);
		let request = http_client_request!(Url(url), TimeoutMillis(30_000))?;
		let response = http_client_send!(request)?;

		assert_eq!(response.code().unwrap(), 200);
		let mut buf = vec![];
		assert_eq!(response.content_reader()?.read_to_end(&mut buf)?, 3);
		assert_eq!(buf, b"abc");

		let url = &format!("http://127.0.0.1:{}/def", port);
		let request = http_client_request!(Url(url), TimeoutMillis(30_000))?;
		let response = http_client_send!(request)?;

		assert_eq!(response.code().unwrap(), 200);
		let mut buf = vec![];
		assert_eq!(response.content_reader()?.read_to_end(&mut buf)?, 4);
		assert_eq!(buf, b"defg");

		let mut i = 0;
		for header in response.headers()? {
			info!("header[{}] = {}: {}", i, header.0, header.1)?;
			i += 1;
		}

		tear_down_server(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_rustlet_tls() -> Result<(), Error> {
		let test_dir = ".test_rustlet_tls.bmw";
		let port = build_server(test_dir, true)?;

		http_client_init!(BaseDir(test_dir))?;
		let url = &format!("https://localhost:{}/abc", port);
		let request = http_client_request!(
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;

		assert_eq!(response.code().unwrap(), 200);
		let mut buf = vec![];
		assert_eq!(response.content_reader()?.read_to_end(&mut buf)?, 3);
		assert_eq!(buf, b"abc");

		tear_down_server(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_rustlet_method() -> Result<(), Error> {
		let test_dir = ".test_rustlet_method.bmw";
		let port = build_server(test_dir, true)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("https://localhost:{}/method?post", port);
		let request = http_client_request!(
			Method(HttpMethod::POST),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/method", port);
		let request = http_client_request!(
			Method(HttpMethod::GET),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/method?trace", port);
		let request = http_client_request!(
			Method(HttpMethod::TRACE),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		tear_down_server(test_dir)?;
		Ok(())
	}

	#[test]
	fn test_rustlet_version() -> Result<(), Error> {
		let test_dir = ".test_rustlet_version.bmw";
		let port = build_server(test_dir, true)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("https://localhost:{}/version?11", port);
		let request = http_client_request!(
			Version(HttpVersion::HTTP11),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/version", port);
		let request = http_client_request!(
			Version(HttpVersion::HTTP11),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/version?10", port);
		let request = http_client_request!(
			Version(HttpVersion::HTTP10),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/version?10", port);
		let request = http_client_request!(
			Version(HttpVersion::UNKNOWN),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		let url = &format!("https://localhost:{}/version?20", port);
		let request = http_client_request!(
			Version(HttpVersion::OTHER),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;

		tear_down_server(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_rustlet_redirect() -> Result<(), Error> {
		let test_dir = ".test_rustlet_redirect.bmw";
		let port = build_server(test_dir, true)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("https://localhost:{}/redirect?1", port);
		let request = http_client_request!(
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;
		assert_eq!(response.code().unwrap(), 302);

		let mut found_redir = false;
		for header in response.headers()? {
			let (name, value) = header;
			if name == "Location" {
				assert_eq!(value, "http://www.example.com");
				found_redir = true;
			}
		}

		assert!(found_redir);

		tear_down_server(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_rustlet_headers() -> Result<(), Error> {
		let test_dir = ".test_rustlet_headers.bmw";
		let port = build_server(test_dir, true)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("https://localhost:{}/headers", port);
		let request = http_client_request!(
			Header(("test", "value1")),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;
		assert_eq!(response.code().unwrap(), 200);

		let mut buf = vec![];
		assert_eq!(response.content_reader()?.read_to_end(&mut buf)?, 1);
		assert_eq!(buf, b"1");

		let url = &format!("https://localhost:{}/headers", port);
		let request = http_client_request!(
			Header(("test2", "value2")),
			Url(url),
			TimeoutMillis(30_000),
			FullChainCertFile("./resources/cert.pem")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;
		assert_eq!(response.code().unwrap(), 200);

		let mut buf = vec![];
		assert_eq!(response.content_reader()?.read_to_end(&mut buf)?, 1);
		assert_eq!(buf, b"2");

		tear_down_server(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_rustlet_content_reader() -> Result<(), Error> {
		let test_dir = ".test_rustlet_content_reader.bmw";
		let port = build_server(test_dir, false)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("http://127.0.0.1:{}/content", port);
		let request = http_client_request!(
			Method(HttpMethod::POST),
			Url(url),
			TimeoutMillis(30_000),
			ContentData(b"test123")
		)?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;
		assert_eq!(response.code().unwrap(), 200);

		tear_down_server(test_dir)?;
		Ok(())
	}

	#[test]
	fn test_rustlet_additional_headers() -> Result<(), Error> {
		let test_dir = ".test_rustlet_additional_headers.bmw";
		let port = build_server(test_dir, false)?;

		http_client_init!(BaseDir(test_dir))?;

		let url = &format!("http://127.0.0.1:{}/add_headers", port);
		let request =
			http_client_request!(Url(url), TimeoutMillis(30_000), ContentData(b"test123"))?;
		let response = http_client_send!(request)?;
		info!("resp={}", response)?;
		assert_eq!(response.code().unwrap(), 200);

		let mut found = false;
		for (name, value) in response.headers()? {
			if name == "myheader" {
				assert_eq!(value, "myvalue");
				found = true;
			}
		}

		assert!(found);

		tear_down_server(test_dir)?;
		Ok(())
	}

	#[test]
	fn test_rustlet_async() -> Result<(), Error> {
		let test_dir = ".test_rustlet_async.bmw";
		let port = build_server(test_dir, false)?;

		http_client_init!(BaseDir(test_dir))?;
		let mut connection = http_connection!(Host("127.0.0.1"), Port(port), Tls(false))?;

		let request = http_client_request!(Uri("/async"))?;
		let mut lock = lock_box!(0)?;
		let lock_clone = lock.clone();

		let request2 = http_client_request!(Uri("/add_headers"))?;
		let (tx, rx) = sync_channel(1);
		let tx_clone = tx.clone();
		std::thread::spawn(move || {
			sleep(Duration::from_millis(30_000));
			let _ = tx_clone.send(());
		});

		let mut lock2 = lock_box!(0)?;
		let lock2_clone = lock2.clone();

		http_client_send!([request], connection, {
			let response = http_client_response!()?;

			info!("resp(async)={}", response)?;
			assert_eq!(response.code().unwrap(), 200);

			let mut reader = response.content_reader()?;

			let mut buf = vec![];
			let len = reader.read_to_end(&mut buf)?;

			assert_eq!(len, 10);
			assert_eq!(&buf[0..10], b"part1part2");
			wlock!(lock) += 1;

			Ok(())
		})?;

		http_client_send!([request2], connection, {
			let response = http_client_response!()?;
			info!("resp(add_header)={}", response)?;
			assert_eq!(response.code().unwrap(), 200);

			let mut found = false;
			for (name, value) in response.headers()? {
				if name == "myheader" {
					assert_eq!(value, "myvalue");
					found = true;
				}
			}

			assert!(found);

			info!("second request complete")?;
			wlock!(lock2) += 1;

			tx.send(())?;
			Ok(())
		})?;

		rx.recv()?;

		assert_eq!(rlock!(lock_clone), 1);
		assert_eq!(rlock!(lock2_clone), 1);
		tear_down_server(test_dir)?;
		Ok(())
	}

	#[test]
	fn test_websocket_with_client() -> Result<(), Error> {
		let test_dir = ".test_websocket_with_client.bmw";
		let port = build_server(test_dir, true)?;

		websocket_client_init!(Threads(2))?;

		let url = format!("wss://localhost:{}/chat", port);

		let config =
			websocket_connection_config!(Url(&url), FullChainCertFile("./resources/cert.pem"))?;

		let mut success = lock_box!(false)?;
		let success_clone = success.clone();

		let mut ws_conn = websocket_connection!(&config, {
			info!("got a message")?;
			let message = websocket_message!()?;
			assert_eq!(message.mtype, WebSocketMessageType::Binary);
			assert_eq!(message.payload, "test".as_bytes().to_vec());
			info!("msg={:?}", message)?;
			wlock!(success) = true;

			Ok(())
		})?;

		let msg = try_into!("hello")?;

		sleep(Duration::from_millis(1_000));
		ws_conn.send(&msg)?;

		sleep(Duration::from_millis(3_000));
		assert!(rlock!(success_clone));

		websocket_client_stop!()?;
		tear_down_server(test_dir)?;
		Ok(())
	}
}
