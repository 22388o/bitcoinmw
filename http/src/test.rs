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
	use crate::constants::*;
	use crate::types::*;
	use bmw_conf::ConfigOption;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::fs::{create_dir_all, File};
	use std::io::Read;
	use std::io::Write;
	use std::path::PathBuf;
	use std::str::from_utf8;

	// include build information
	pub mod built_info {
		include!(concat!(env!("OUT_DIR"), "/built.rs"));
	}

	info!();

	#[test]
	fn test_http_basic() -> Result<(), Error> {
		Ok(())
	}

	#[test]
	fn test_http_request() -> Result<(), Error> {
		let test_info = test_info!()?;

		let path = format!("{}/foo.txt", test_info.directory());
		let mut file = File::create(path.clone())?;
		file.write_all(b"Hello, world!")?;
		let config = vec![ConfigOption::HttpContentFile(path.clone().into())];
		let mut request = HttpBuilder::build_http_request(config)?;
		let mut s = String::new();
		request.read_to_string(&mut s)?;
		assert_eq!(s, "Hello, world!".to_string());

		let config = vec![ConfigOption::HttpContentData(vec![b'a', b'b', b'c'])];
		let mut request = HttpBuilder::build_http_request(config)?;
		let mut s = String::new();
		request.read_to_string(&mut s)?;
		assert_eq!(s, "abc".to_string());

		let config = vec![];
		let mut request = HttpBuilder::build_http_request(config)?;
		let mut s = String::new();
		request.read_to_string(&mut s)?;
		assert_eq!(s, "".to_string());

		info!("request.guid={}", request.guid())?;
		assert_ne!(request.guid(), 0);
		assert_eq!(request.guid(), request.guid());

		// test defaults + request_uri
		let config = vec![ConfigOption::HttpRequestUri("/test.rsp".to_string())];
		let request = HttpBuilder::build_http_request(config)?;

		assert_eq!(request.request_url(), &None);
		assert_eq!(request.request_uri(), &Some("/test.rsp".to_string()));
		assert_eq!(request.version(), &HttpVersion::Http11);
		assert_eq!(request.method(), &HttpMethod::Get);
		assert_eq!(request.connection_type(), &HttpConnectionType::Close);
		assert_eq!(request.accept(), &DEFAULT_HTTP_ACCEPT.to_string());
		assert_eq!(request.timeout_millis(), DEFAULT_HTTP_TIMEOUT_MILLIS);

		let pkg_version = built_info::PKG_VERSION.to_string();
		let user_agent_default = format!("BitcoinMW/{}", pkg_version).to_string();
		assert_eq!(request.user_agent(), &user_agent_default);

		// with all configurations + request_url
		let config = vec![
			ConfigOption::HttpRequestUrl("http://www.example.com".to_string()),
			ConfigOption::HttpAccept("otherval".to_string()),
			ConfigOption::HttpHeader(("test1".to_string(), "value".to_string())),
			ConfigOption::HttpHeader(("test2".to_string(), "value2".to_string())),
			ConfigOption::HttpTimeoutMillis(1234),
			ConfigOption::HttpContentData(b"111".to_vec()),
			ConfigOption::HttpUserAgent("myagent".to_string()),
			ConfigOption::HttpMeth(HTTP_METHOD_DELETE.to_string()),
			ConfigOption::HttpVers(HTTP_VERSION_10.to_string()),
			ConfigOption::HttpConnection(HTTP_CONNECTION_TYPE_KEEP_ALIVE.to_string()),
		];
		let mut request = HttpBuilder::build_http_request(config)?;

		assert_eq!(request.accept(), &"otherval".to_string());
		assert_eq!(request.timeout_millis(), 1234);
		assert_eq!(request.request_uri(), &None);
		assert_eq!(
			request.request_url(),
			&Some("http://www.example.com".to_string())
		);
		assert_eq!(request.method(), &HttpMethod::Delete);
		assert_eq!(request.user_agent(), &"myagent".to_string());
		assert_eq!(request.version(), &HttpVersion::Http10);
		assert_eq!(request.connection_type(), &HttpConnectionType::KeepAlive);

		assert_eq!(
			request.headers(),
			&vec![
				("test1".to_string(), "value".to_string()),
				("test2".to_string(), "value2".to_string())
			]
		);

		let mut s = String::new();
		request.read_to_string(&mut s)?;
		assert_eq!(s, "111".to_string());

		// cannot have both content file and data
		let config = vec![
			ConfigOption::HttpContentFile(path.into()),
			ConfigOption::HttpContentData(vec![b'0']),
		];

		assert!(HttpBuilder::build_http_request(config).is_err());

		// invalid version
		let config = vec![ConfigOption::HttpVers("kasdjlkajlf".to_string())];
		assert!(HttpBuilder::build_http_request(config).is_err());

		Ok(())
	}

	#[test]
	fn test_http_response() -> Result<(), Error> {
		let mut response: Box<dyn HttpResponse> = Box::new(HttpResponseImpl::new(
			vec![
				("test1".to_string(), "v1".to_string()),
				("test2".to_string(), "v2".to_string()),
			],
			123,
			"OK".to_string(),
			HttpVersion::Http10,
			None,
			b"test".to_vec(),
			None,
		)?);

		assert_eq!(
			response.headers(),
			&vec![
				("test1".to_string(), "v1".to_string()),
				("test2".to_string(), "v2".to_string()),
			]
		);

		assert_eq!(response.code(), 123);

		assert_eq!(response.status_text(), &"OK".to_string());
		assert_eq!(response.version(), &HttpVersion::Http10);

		let mut s = String::new();
		response.read_to_string(&mut s)?;
		assert_eq!(s, "test".to_string());

		Ok(())
	}

	#[test]
	fn test_http_client_basic1() -> Result<(), Error> {
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("test.html");
		let mut file = File::create(path_buf)?;
		let buf = [b'x'; 1600];
		file.write_all(&buf)?;

		let mut path_buf2 = PathBuf::new();
		path_buf2.push(directory);
		path_buf2.push("test2.html");
		let mut file2 = File::create(path_buf2)?;
		let buf2 = [b'y'; 1700];
		file2.write_all(&buf2)?;

		let port = test_info.port();
		info!("port={}", port)?;
		let mut server =
			HttpBuilder::build_http_server(vec![ConfigOption::ServerName("myserver".to_string())])?;
		let instance = HttpBuilder::build_instance(vec![
			ConfigOption::Port(port),
			ConfigOption::BaseDir(directory.clone()),
		])?;
		server.add_instance(instance)?;
		server.start()?;

		let mut client =
			HttpBuilder::build_http_client(vec![ConfigOption::BaseDir(directory.clone())])?;
		let request = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test.html", port),
		)])?;

		let request2 = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test2.html", port),
		)])?;

		let mut recv = lock_box!(0)?;
		let mut recv2 = recv.clone();
		let recv_clone = recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let tx_clone = tx.clone();
		let handler: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				info!("headers = {:?}", response.headers())?;
				assert_eq!(s, from_utf8(&buf)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv2.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 2 {
					tx_clone.send(())?;
				}
				Ok(())
			});

		let handler2: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				assert_eq!(s, from_utf8(&buf2)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 2 {
					tx.send(())?;
				}
				Ok(())
			});

		client.send(&request, handler)?;
		client.send(&request2, handler2)?;

		rx.recv()?;

		assert_eq!(rlock!(recv_clone), 2);
		Ok(())
	}

	#[test]
	fn test_http_client_basic_no_chunks() -> Result<(), Error> {
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("test.html");
		let mut file = File::create(path_buf)?;
		let buf = [b'x'; 1600];
		file.write_all(&buf)?;

		let mut path_buf2 = PathBuf::new();
		path_buf2.push(directory);
		path_buf2.push("test2.html");
		let mut file2 = File::create(path_buf2)?;
		let buf2 = [b'y'; 1700];
		file2.write_all(&buf2)?;

		let port = test_info.port();
		info!("port={}", port)?;
		let mut server = HttpBuilder::build_http_server(vec![
			ConfigOption::ServerName("myserver".to_string()),
			ConfigOption::DebugNoChunks(true),
		])?;
		let instance = HttpBuilder::build_instance(vec![
			ConfigOption::Port(port),
			ConfigOption::BaseDir(directory.clone()),
		])?;
		server.add_instance(instance)?;
		server.start()?;

		let mut client =
			HttpBuilder::build_http_client(vec![ConfigOption::BaseDir(directory.clone())])?;
		let request = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test.html", port),
		)])?;

		let request2 = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test2.html", port),
		)])?;

		let mut recv = lock_box!(0)?;
		let mut recv2 = recv.clone();
		let recv_clone = recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let tx_clone = tx.clone();
		let handler: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				info!("headers = {:?}", response.headers())?;
				assert_eq!(s, from_utf8(&buf)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv2.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 2 {
					tx_clone.send(())?;
				}
				Ok(())
			});

		let handler2: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				assert_eq!(s, from_utf8(&buf2)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 2 {
					tx.send(())?;
				}
				Ok(())
			});

		client.send(&request, handler)?;
		client.send(&request2, handler2)?;

		rx.recv()?;

		assert_eq!(rlock!(recv_clone), 2);

		Ok(())
	}

	#[test]
	fn test_http_multi_directory() -> Result<(), Error> {
		let test_info = test_info!(true)?;
		let directory = test_info.directory();
		let port1 = test_info.port();
		let port2 = pick_free_port()?;

		// make 3 directories
		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("1");
		create_dir_all(path_buf.clone())?;
		path_buf.pop();
		path_buf.push("2");
		create_dir_all(path_buf.clone())?;
		path_buf.pop();
		path_buf.push("3");
		create_dir_all(path_buf)?;

		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("1");
		path_buf.push("test.html");
		let mut file = File::create(path_buf)?;
		let buf = [b'x'; 1600];
		file.write_all(&buf)?;

		let mut path_buf2 = PathBuf::new();
		path_buf2.push(directory);
		path_buf2.push("2");
		path_buf2.push("test.html");
		let mut file2 = File::create(path_buf2)?;
		let buf2 = [b'y'; 1700];
		file2.write_all(&buf2)?;

		let mut path_buf3 = PathBuf::new();
		path_buf3.push(directory);
		path_buf3.push("3");
		path_buf3.push("test.html");
		let mut file3 = File::create(path_buf3)?;
		let buf3 = [b'z'; 1800];
		file3.write_all(&buf3)?;

		info!("port={}", port1)?;
		let mut server =
			HttpBuilder::build_http_server(vec![ConfigOption::ServerName("myserver".to_string())])?;

		// add instance1
		let mut base_dir1 = PathBuf::new();
		base_dir1.push(directory);
		base_dir1.push("1");
		let base_dir1 = base_dir1.as_path().to_str().unwrap().to_string();
		let instance = HttpBuilder::build_instance(vec![
			ConfigOption::Port(port1),
			ConfigOption::BaseDir(base_dir1.clone()),
		])?;
		server.add_instance(instance)?;

		// add instance2
		let mut base_dir2 = PathBuf::new();
		base_dir2.push(directory);
		base_dir2.push("2");
		let base_dir2 = base_dir2.as_path().to_str().unwrap().to_string();
		let mut instance = HttpBuilder::build_instance(vec![
			ConfigOption::Port(port2),
			ConfigOption::BaseDir(base_dir2.clone()),
		])?;

		let mut base_dir3 = PathBuf::new();
		base_dir3.push(directory);
		base_dir3.push("3");
		let base_dir3 = base_dir3.as_path().to_str().unwrap().to_string();

		instance.add_dir_mapping(format!("localhost:{}", port2), base_dir3)?;

		server.add_instance(instance)?;

		server.start()?;

		info!("Using basedir = {}", base_dir1)?;
		let mut client =
			HttpBuilder::build_http_client(vec![ConfigOption::BaseDir(directory.clone())])?;
		let request = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test.html", port1),
		)])?;

		let recv = lock_box!(0)?;
		let mut recv2 = recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let tx_clone = tx.clone();
		let handler: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				info!("headers = {:?}", response.headers())?;
				assert_eq!(s, from_utf8(&buf)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv2.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 1 {
					tx_clone.send(())?;
				}
				Ok(())
			});

		client.send(&request, handler)?;
		rx.recv()?;

		assert_eq!(rlock!(recv), 1);

		// send a request to the second instance
		info!("Using basedir = {}", base_dir2)?;
		let mut client =
			HttpBuilder::build_http_client(vec![ConfigOption::BaseDir(directory.clone())])?;
		let request = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://127.0.0.1:{}/test.html", port2),
		)])?;

		let recv = lock_box!(0)?;
		let mut recv2 = recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let tx_clone = tx.clone();
		let handler: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				info!("headers = {:?}", response.headers())?;
				assert_eq!(s, from_utf8(&buf2)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv2.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 1 {
					tx_clone.send(())?;
				}
				Ok(())
			});

		client.send(&request, handler)?;
		rx.recv()?;

		assert_eq!(rlock!(recv), 1);

		// send a request to the second instance
		info!("Using basedir = {}", base_dir2)?;
		let mut client =
			HttpBuilder::build_http_client(vec![ConfigOption::BaseDir(directory.clone())])?;
		let request = HttpBuilder::build_http_request(vec![ConfigOption::HttpRequestUrl(
			format!("http://localhost:{}/test.html", port2),
		)])?;

		let recv = lock_box!(0)?;
		let mut recv2 = recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let tx_clone = tx.clone();
		let handler: HttpResponseHandler =
			Box::pin(move |_request, response| -> Result<(), Error> {
				let mut s = String::new();
				response.read_to_string(&mut s)?;
				info!("in handler[s.len={}]: {}", s.len(), s)?;
				info!("headers = {:?}", response.headers())?;
				assert_eq!(s, from_utf8(&buf3)?.to_string());
				assert_eq!(response.version(), &HttpVersion::Http11);
				assert_eq!(response.code(), 200);
				assert_eq!(response.status_text(), &"OK".to_string());

				let mut found_server_name = false;
				let mut found_date = false;
				for header in response.headers() {
					if header.0 == "Server".to_string() && header.1 == "myserver".to_string() {
						found_server_name = true;
					}
					if header.0 == "Date".to_string() {
						found_date = true;
					}
					if header.0 == "Transfer-Encoding".to_string()
						&& header.1 == "chunked".to_string()
					{}
				}

				assert!(found_server_name);
				assert!(found_date);

				let mut recv = recv2.wlock()?;
				let guard = recv.guard()?;
				(**guard) += 1;
				if **guard == 1 {
					tx_clone.send(())?;
				}
				Ok(())
			});

		client.send(&request, handler)?;
		rx.recv()?;

		assert_eq!(rlock!(recv), 1);

		Ok(())
	}
}
