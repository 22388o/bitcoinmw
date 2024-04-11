// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_conf::*;
use bmw_err::*;
use bmw_evh::*;
use bmw_http::*;
use bmw_log::*;
use bmw_util::*;
use std::thread::park;

debug!();

const SEPARATOR: &str =
        "------------------------------------------------------------------------------------------------------------------------";

fn callback(
	headers: &Box<dyn HttpHeaders + '_>,
	_content_reader: &mut Option<Box<dyn LockBox<HttpContentReader>>>,
	wh: &mut WriteHandle,
	_instance: &HttpInstance,
) -> Result<(), Error> {
	if headers.path() == "/error" {
		Err(err!(ErrKind::Test, "simulate internal server errror"))
	} else {
		wh.write(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n012345\r\n\r\n")?;
		Ok(())
	}
}

fn main() -> Result<(), Error> {
	log_init!(
		DisplayLineNum(false),
		DisplayBackTrace(false),
		DisplayMillis(false),
		LogFilePath(Some("~/.bmw/main.log".into()))
	)?;

	let directory = "~/.bmw/www".to_string();
	let port = 8080;
	let mut server = HttpBuilder::build_http_server(vec![
		ConfigOption::ServerName("myserver".to_string()),
		ConfigOption::HttpShowRequest(true),
	])?;
	let mut instance = HttpBuilder::build_instance(vec![
		ConfigOption::Port(port),
		ConfigOption::BaseDir(directory.clone()),
	])?;
	instance.add_callback_mapping("/test.html".to_string())?;
	instance.add_callback_mapping("/error".to_string())?;
	instance.set_callback(Some(callback))?;
	server.add_instance(instance)?;
	server.start()?;
	info!("Server is ready!")?;
	info_plain!(SEPARATOR)?;

	park();

	Ok(())
}
