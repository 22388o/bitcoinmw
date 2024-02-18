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

use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::*;
use bmw_http::{
	HttpConfig, HttpContentReader, HttpMethod, HttpVersion, WebSocketData, WebSocketHandle,
	WebSocketMessage,
};
use std::pin::Pin;

pub type Rustlet = Pin<
	Box<
		dyn Fn(&mut Box<dyn RustletRequest>, &mut Box<dyn RustletResponse>) -> Result<(), Error>
			+ Send
			+ Sync,
	>,
>;

/// The main trait used for processing requests in a rustlet. It has all the information needed in
/// it. It can be accessed by the [`crate::request`] macro.
pub trait RustletRequest: DynClone {
	fn method(&self) -> Result<&HttpMethod, Error>;
	fn version(&self) -> Result<&HttpVersion, Error>;
	fn path(&self) -> Result<String, Error>;
	fn query(&self) -> Result<String, Error>;
	fn cookie(&self, name: &str) -> Result<String, Error>;
	fn headers(&self) -> Result<usize, Error>;
	fn header_name(&self, n: usize) -> Result<String, Error>;
	fn header_value(&self, n: usize) -> Result<String, Error>;
	fn header(&self, name: &str) -> Result<String, Error>;
	fn content(&self) -> Result<HttpContentReader, Error>;
}

clone_trait_object!(RustletRequest);

pub trait RustletResponse: DynClone {
	fn write<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error>
	where
		Self: Sized;
	fn flush(&mut self) -> Result<(), Error>;
	fn async_context(&mut self) -> Result<Box<dyn AsyncContext>, Error>;
	fn add_header(&mut self, name: &str, value: &str) -> Result<(), Error>;
	fn content_type(&mut self, value: &str) -> Result<(), Error>;
	fn set_cookie(&mut self, name: &str, value: &str) -> Result<(), Error>;
	fn redirect(&mut self, url: &str) -> Result<(), Error>;
}

clone_trait_object!(RustletResponse);

pub trait WebSocketRequest: DynClone {
	fn handle(&self) -> Result<WebSocketHandle, Error>;
	fn message(&mut self) -> Result<WebSocketMessage, Error>;
	fn data(&self) -> Result<WebSocketData, Error>;
}

clone_trait_object!(WebSocketRequest);

pub trait AsyncContext {
	fn async_complete(&mut self) -> Result<(), Error>;
}

pub struct RustletConfig {
	pub http_config: HttpConfig,
}

pub struct RustletContainer {}

#[derive(Clone)]
pub(crate) struct RustletRequestImpl {}

#[derive(Clone)]
pub(crate) struct RustletResponseImpl {}

pub(crate) struct AsyncContextImpl {}

#[derive(Clone)]
pub(crate) struct WebSocketRequestImpl {}
