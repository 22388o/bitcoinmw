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

use crate::types::{AsyncContextImpl, RustletRequestImpl, RustletResponseImpl};
use crate::{AsyncContext, RustletRequest, RustletResponse};
use bmw_err::*;
use bmw_http::{HttpContentReader, HttpMethod, HttpVersion};
use bmw_log::*;

info!();

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
