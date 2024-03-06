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
	use bmw_http::*;
	use bmw_rustlet::*;
	use bmw_test::*;
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
			assert_eq!(request.method(), HttpMethod::GET);
			response.set_connection_close()?;
			response.write(b"defg")?;
			Ok(())
		})?;
		rustlet_mapping!("/abc", "test")?;
		rustlet_mapping!("/def", "test2")?;

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
}
