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

use bmw_log::*;

info!();

#[macro_export]
macro_rules! rustlet_init {
	($config:expr) => {};
}

#[macro_export]
macro_rules! rustlet {
	($name:expr, $code:expr) => {
		let container = bmw_rustlet::RUSTLET_CONTAINER.write();
		match container {
			Ok(mut container) => {
				let _res = (*container).add_rustlet(
					$name,
					Box::pin(
						move |request: &mut bmw_rustlet::RustletRequestImpl,
						      response: &mut bmw_rustlet::RustletResponseImpl| {
							bmw_rustlet::RUSTLET_CONTEXT.with(|f| {
								*f.borrow_mut() =
									(Some(((*request).clone(), (*response).clone())), None);
							});
							{
								$code
							}
							Ok(())
						},
					),
				);
			}
			Err(e) => {
				warn!("Couldn't add rustlet to the container due to error: {}", e)?;
			}
		}
	};
}

#[macro_export]
macro_rules! rustlet_mapping {
	($path:expr, $name:expr) => {};
}

#[macro_export]
macro_rules! request {
	() => {};
}

#[macro_export]
macro_rules! response {
	() => {};
}

#[macro_export]
macro_rules! websocket {
	() => {};
}

/// Returns [`crate::WebSocketRequest`].
#[macro_export]
macro_rules! websocket_request {
	() => {};
}

/// Three params: name, uri, [protocol list]
#[macro_export]
macro_rules! websocket_mapping {
	() => {};
}

#[macro_export]
macro_rules! session {
	// TODO: session will have CRUD for session. SessionOp::Set, SessionOp::Get,
	// SessionOp::Delete
	() => {};
}

#[cfg(test)]
mod test {
	use crate as bmw_rustlet;
	use bmw_err::*;
	use bmw_log::*;

	debug!();

	#[test]
	fn test_rustlet_macros() -> Result<(), Error> {
		rustlet!("test", {});
		Ok(())
	}
}
