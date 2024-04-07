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

use crate::types::{HttpClientImpl, HttpConnectionImpl};
use crate::{HttpClient, HttpConnection, HttpRequest, HttpResponseHandler};
use bmw_conf::ConfigOption;
use bmw_err::*;
use bmw_evh::*;
use bmw_log::*;
use std::sync::mpsc::sync_channel;
use std::thread::spawn;

info!();

impl HttpClient for HttpClientImpl {
	fn send(
		&mut self,
		_request: &Box<dyn HttpRequest>,
		_handler: HttpResponseHandler,
	) -> Result<(), Error> {
		todo!()
	}
}

impl HttpClientImpl {
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let (tx, rx) = sync_channel(1);
		spawn(move || -> Result<(), Error> {
			let mut evh = EvhBuilder::build_evh(configs)?;
			evh.set_on_read(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;
			evh.set_on_accept(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;
			evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;

			evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> { Ok(()) })?;

			evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> { Ok(()) })?;
			evh.start()?;
			tx.send(())?;

			loop {
				let stats = evh.wait_for_stats()?;
				info!("stats={:?}", stats)?;
				cbreak!(false);
			}

			Ok(())
		});
		rx.recv()?;

		Ok(Self {})
	}
}

impl HttpConnection for HttpConnectionImpl {
	fn connect(&mut self) -> Result<(), Error> {
		todo!()
	}
	fn send(
		&mut self,
		_request: &Box<dyn HttpRequest>,
		_handler: HttpResponseHandler,
	) -> Result<(), Error> {
		todo!()
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(
		_configs: Vec<ConfigOption>,
		_client: Box<dyn HttpClient>,
	) -> Result<Self, Error> {
		todo!()
	}
}
