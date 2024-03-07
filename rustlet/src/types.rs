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

use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::*;
use bmw_evh::WriteHandle;
use bmw_http::{
	HttpConfig, HttpContentReader, HttpMethod, HttpServer, HttpVersion, WebSocketData,
	WebSocketHandle, WebSocketMessage,
};
use bmw_util::*;
use std::collections::HashMap;
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
	fn method(&self) -> &HttpMethod;
	fn version(&self) -> &HttpVersion;
	fn path(&self) -> &String;
	fn query(&self) -> &String;
	fn headers(&self) -> &Vec<(String, String)>;
	fn content(&self) -> Result<HttpContentReader, Error>;
}

clone_trait_object!(RustletRequest);

pub trait RustletResponse: DynClone {
	fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;
	fn print(&mut self, text: &str) -> Result<(), Error>;
	fn flush(&mut self) -> Result<(), Error>;
	fn async_context(&mut self) -> Result<Box<dyn AsyncContext>, Error>;
	fn add_header(&mut self, name: &str, value: &str) -> Result<(), Error>;
	fn set_content_type(&mut self, value: &str) -> Result<(), Error>;
	fn redirect(&mut self, url: &str) -> Result<(), Error>;
	fn set_connection_close(&mut self) -> Result<(), Error>;
	fn complete(&mut self) -> Result<(), Error>;
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

pub struct RustletContainer {
	pub(crate) rustlets: HashMap<String, Rustlet>,
	pub(crate) rustlet_mappings: HashMap<String, String>,
	pub(crate) http_config: HttpConfig,
	pub(crate) http_server: Option<Box<dyn HttpServer + Send + Sync>>,
}

// Crate local structs

#[derive(Clone)]
pub(crate) struct RustletRequestImpl {
	pub(crate) path: String,
	pub(crate) query: String,
	pub(crate) method: HttpMethod,
	pub(crate) version: HttpVersion,
	pub(crate) headers: Vec<(String, String)>,
}

#[derive(Clone)]
pub(crate) struct RustletResponseImpl {
	pub(crate) wh: WriteHandle,
	pub(crate) state: Box<dyn LockBox<RustletResponseState>>,
}

pub(crate) struct RustletResponseState {
	pub(crate) sent_headers: bool,
	pub(crate) completed: bool,
	pub(crate) close: bool,
	pub(crate) content_type: String,
	pub(crate) buffer: Vec<u8>,
	pub(crate) redirect: Option<String>,
}
pub(crate) struct AsyncContextImpl {}

#[derive(Clone)]
pub(crate) struct WebSocketRequestImpl {}
