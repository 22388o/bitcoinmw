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

use crate::types::{HttpClientImpl, HttpConnectionImpl, HttpRequestImpl};
use crate::{
	HttpClient, HttpClientConfig, HttpConnection, HttpConnectionConfig, HttpHandler, HttpRequest,
	HttpRequestConfig,
};
use bmw_err::*;
use bmw_evh::{
	AttachmentHolder, Builder, ConnData, ConnectionData, EventHandler, EventHandlerConfig,
	EventHandlerData, ThreadContext,
};
use bmw_log::*;
use bmw_util::*;
use std::any::Any;

debug!();

impl Default for HttpClientConfig {
	fn default() -> Self {
		Self {}
	}
}

impl Default for HttpRequestConfig {
	fn default() -> Self {
		Self {}
	}
}

impl HttpClient for HttpClientImpl {
	fn send(&mut self, _req: &Box<dyn HttpRequest>, _handler: &HttpHandler) -> Result<(), Error> {
		Ok(())
	}
}

impl HttpClientImpl {
	pub(crate) fn new(config: &HttpClientConfig) -> Result<HttpClientImpl, Error> {
		let evh_config = EventHandlerConfig {
			..Default::default()
		};
		let mut evh = Builder::build_evh(evh_config)?;
		let event_handler_data = evh.event_handler_data()?;

		let config = config.clone();
		let config2 = config.clone();
		let config3 = config.clone();
		let config4 = config.clone();
		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config, conn_data, ctx, attach)
		})?;
		evh.set_on_accept(move |conn_data, ctx| Self::process_on_accept(&config2, conn_data, ctx))?;
		evh.set_on_close(move |conn_data, ctx| Self::process_on_close(&config3, conn_data, ctx))?;
		evh.set_on_panic(move |ctx, e| Self::process_on_panic(ctx, e))?;
		evh.set_housekeeper(move |ctx| {
			Self::process_housekeeper(config4.clone(), ctx, event_handler_data.clone())
		})?;

		evh.start()?;
		Ok(Self {})
	}

	fn process_on_read(
		_config: &HttpClientConfig,
		conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
		_attachment: Option<AttachmentHolder>,
	) -> Result<(), Error> {
		let id = conn_data.get_connection_id();
		debug!("Process on read. conn_id={}", id)?;
		Ok(())
	}

	fn process_on_accept(
		_config: &HttpClientConfig,
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_close(
		_config: &HttpClientConfig,
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_panic(_ctx: &mut ThreadContext, _e: Box<dyn Any>) -> Result<(), Error> {
		Ok(())
	}

	fn process_housekeeper(
		_config: HttpClientConfig,
		_ctx: &mut ThreadContext,
		_event_handler_data: Array<Box<dyn LockBox<EventHandlerData>>>,
	) -> Result<(), Error> {
		Ok(())
	}
}

impl HttpConnection for HttpConnectionImpl {
	fn send(&mut self, _req: &Box<dyn HttpRequest>, _handler: &HttpHandler) -> Result<(), Error> {
		todo!()
	}
}

impl HttpConnectionImpl {
	pub(crate) fn new(_config: &HttpConnectionConfig) -> Result<HttpConnectionImpl, Error> {
		Ok(Self {})
	}
}

impl HttpRequest for HttpRequestImpl {}

impl HttpRequestImpl {
	pub(crate) fn new(_config: &HttpRequestConfig) -> Result<HttpRequestImpl, Error> {
		Ok(Self {})
	}
}

mod test {
	use crate::types::{HttpClientImpl, HttpRequestImpl};
	use crate::{
		HttpClient, HttpClientConfig, HttpHandler, HttpRequest, HttpRequestConfig, HttpResponse,
	};
	use bmw_err::*;
	use bmw_log::*;
	use std::thread::sleep;
	use std::time::Duration;

	info!();

	#[test]
	fn test_client_basic() -> Result<(), Error> {
		let mut http_client = HttpClientImpl::new(&HttpClientConfig {
			..Default::default()
		})?;

		let handler1: HttpHandler = Box::pin(
			move |_request: &mut Box<dyn HttpRequest>, _response: &mut Box<dyn HttpResponse>| Ok(()),
		);

		let http_client_request1 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			..Default::default()
		})?) as Box<dyn HttpRequest>;

		http_client.send(&http_client_request1, &handler1)?;

		let handler2: HttpHandler = Box::pin(
			move |_request: &mut Box<dyn HttpRequest>, _response: &mut Box<dyn HttpResponse>| {
				info!("in handler2")?;
				Ok(())
			},
		);

		let http_client_request2 = Box::new(HttpRequestImpl::new(&HttpRequestConfig {
			..Default::default()
		})?) as Box<dyn HttpRequest>;

		http_client.send(&http_client_request2, &handler2)?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}
}
