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
	($config:expr) => {
		let container = bmw_rustlet::RUSTLET_CONTAINER.write();
		match container {
			Ok(mut container) => {
				(*container) = bmw_rustlet::types::RustletContainer::new($config);
			}
			Err(e) => {
				error!("Couldn't obtain lock to add rustlet to container: {}", e)?;
			}
		}
	};
}

#[macro_export]
macro_rules! rustlet {
	($name:expr, $code:expr) => {
		let container = bmw_rustlet::RUSTLET_CONTAINER.write();
		match container {
			Ok(mut container) => {
				let res = (*container).add_rustlet(
					$name,
					Box::pin(
						move |request: &mut Box<dyn bmw_rustlet::RustletRequest>,
						      response: &mut Box<dyn bmw_rustlet::RustletResponse>| {
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

				match res {
					Ok(_) => {}
					Err(e) => {
						error!("error adding rustlet to container: {}", e)?;
					}
				}
			}
			Err(e) => {
				error!("Couldn't obtain lock to add rustlet to container: {}", e)?;
			}
		}
	};
}

#[macro_export]
macro_rules! rustlet_mapping {
	($path:expr, $name:expr) => {
		let container = bmw_rustlet::RUSTLET_CONTAINER.write();
		match container {
			Ok(mut container) => match (*container).add_rustlet_mapping($path, $name) {
				Ok(_) => {}
				Err(e) => {
					error!("error adding rustlet mapping to container: {}", e)?;
				}
			},
			Err(e) => {
				error!(
					"Couldn't obtain lock to add rustlet mapping to container: {}",
					e
				)?;
			}
		}
	};
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
	use bmw_rustlet::*;

	debug!();

	#[test]
	fn test_rustlet_macros() -> Result<(), Error> {
		rustlet_init!(RustletConfig::default());
		rustlet!("test", {});
		rustlet_mapping!("/abc", "test");
		Ok(())
	}
}
