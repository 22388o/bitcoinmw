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
	use std::fs::File;
	use std::io::Read;
	use std::io::Write;

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
			ConfigOption::HttpMethod(HTTP_METHOD_DELETE.to_string()),
			ConfigOption::HttpVersion(HTTP_VERSION_10.to_string()),
			ConfigOption::HttpConnectionType(HTTP_CONNECTION_TYPE_KEEP_ALIVE.to_string()),
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
		let config = vec![ConfigOption::HttpVersion("kasdjlkajlf".to_string())];
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
}
