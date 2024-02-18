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

#[macro_export]
macro_rules! rustlet_init {
	($config:expr) => {};
}

#[macro_export]
macro_rules! rustlet {
	($name:expr, $code:expr) => {};
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
	use bmw_err::*;

	#[test]
	fn test_rustlet_macros() -> Result<(), Error> {
		Ok(())
	}
}
