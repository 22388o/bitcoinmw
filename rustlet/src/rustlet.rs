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

use crate::types::{
	AsyncContextImpl, Rustlet, RustletContainer, RustletContainerData, RustletRequestImpl,
	RustletResponseImpl, WebSocketRequest, WebSocketRequestImpl,
};
use crate::{AsyncContext, RustletConfig, RustletRequest, RustletResponse};
use bmw_deps::lazy_static::lazy_static;
use bmw_err::*;
use bmw_http::{
	HttpConfig, HttpContentReader, HttpMethod, HttpVersion, WebSocketData, WebSocketHandle,
	WebSocketMessage,
};
use bmw_log::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

info!();

thread_local!(
	pub static RUSTLET_CONTEXT: RefCell<(
					Option<(Box<dyn RustletRequest>,Box<dyn RustletResponse>)>,
					Option<Box<dyn WebSocketRequest>>
				)> = RefCell::new((None, None))
);

lazy_static! {
	pub static ref RUSTLET_CONTAINER: Arc<RwLock<RustletContainer>> =
		Arc::new(RwLock::new(RustletContainer::new_uninit()));
}

impl Default for RustletConfig {
	fn default() -> Self {
		Self {
			http_config: HttpConfig::default(),
		}
	}
}

impl RustletContainer {
	fn new_uninit() -> Self {
		Self {
			rustlet_container_data: None,
		}
	}
	pub fn new(_config: RustletConfig) -> Self {
		Self {
			rustlet_container_data: Some(RustletContainerData {
				rustlets: HashMap::new(),
				rustlet_mappings: HashMap::new(),
			}),
		}
	}
}

impl RustletRequest for RustletRequestImpl {
	fn method(&self) -> Result<&HttpMethod, Error> {
		todo!()
	}
	fn version(&self) -> Result<&HttpVersion, Error> {
		todo!()
	}
	fn path(&self) -> Result<String, Error> {
		todo!()
	}
	fn query(&self) -> Result<String, Error> {
		todo!()
	}
	fn cookie(&self, _name: &str) -> Result<String, Error> {
		todo!()
	}
	fn headers(&self) -> Result<usize, Error> {
		todo!()
	}
	fn header_name(&self, _n: usize) -> Result<String, Error> {
		todo!()
	}
	fn header_value(&self, _n: usize) -> Result<String, Error> {
		todo!()
	}
	fn header(&self, _name: &str) -> Result<String, Error> {
		todo!()
	}
	fn content(&self) -> Result<HttpContentReader, Error> {
		todo!()
	}
}

impl RustletResponse for RustletResponseImpl {
	fn write<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		let b = bytes.as_ref();
		debug!("bytes: {:?}", b)?;
		Ok(())
	}
	fn flush(&mut self) -> Result<(), Error> {
		todo!()
	}
	fn async_context(&mut self) -> Result<Box<dyn AsyncContext>, Error> {
		todo!()
	}
	fn add_header(&mut self, _name: &str, _value: &str) -> Result<(), Error> {
		todo!()
	}
	fn content_type(&mut self, _value: &str) -> Result<(), Error> {
		todo!()
	}
	fn set_cookie(&mut self, _name: &str, _value: &str) -> Result<(), Error> {
		todo!()
	}
	fn redirect(&mut self, _url: &str) -> Result<(), Error> {
		todo!()
	}
}

impl RustletResponseImpl {
	fn _new() -> Result<RustletResponseImpl, Error> {
		Ok(RustletResponseImpl {})
	}
}

impl AsyncContext for AsyncContextImpl {
	fn async_complete(&mut self) -> Result<(), Error> {
		todo!()
	}
}

impl WebSocketRequest for WebSocketRequestImpl {
	fn handle(&self) -> Result<WebSocketHandle, Error> {
		todo!()
	}
	fn message(&mut self) -> Result<WebSocketMessage, Error> {
		todo!()
	}
	fn data(&self) -> Result<WebSocketData, Error> {
		todo!()
	}
}

impl RustletContainer {
	pub fn add_rustlet(&mut self, name: &str, rustlet: Rustlet) -> Result<(), Error> {
		match &mut self.rustlet_container_data {
			Some(r) => {
				r.rustlets.insert(name.to_string(), rustlet);
				Ok(())
			}
			None => Err(err!(
				ErrKind::IllegalState,
				"rustlet container has not been initialized"
			)),
		}
	}

	pub fn add_rustlet_mapping(&mut self, path: &str, name: &str) -> Result<(), Error> {
		match &mut self.rustlet_container_data {
			Some(r) => {
				r.rustlet_mappings
					.insert(path.to_string(), name.to_string());
				Ok(())
			}
			None => Err(err!(
				ErrKind::IllegalState,
				"rustlet container has not been initialized"
			)),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::types::{RustletResponse, RustletResponseImpl};
	use bmw_err::*;

	#[test]
	fn test_rustlet_write_into() -> Result<(), Error> {
		let mut rri = RustletResponseImpl::_new()?;
		assert!(rri.write(b"test").is_ok());
		assert!(rri.write("test2").is_ok());
		assert!(rri.write("test3".to_string()).is_ok());
		Ok(())
	}
}
