// Copyright (c) 2022, 37 Miners, LLC
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

use bmw_deps::portpicker::is_free;
use bmw_deps::rand::random;
use bmw_err::Error;
use std::sync::atomic::{AtomicU16, Ordering};

static GLOBAL_NEXT_PORT: AtomicU16 = AtomicU16::new(9000);

pub fn pick_free_port() -> Result<u16, Error> {
	loop {
		let port = GLOBAL_NEXT_PORT.fetch_add(1, Ordering::SeqCst);
		let port = if port == 9000 {
			let rand: u16 = random();
			let rand = rand % 10_000;
			GLOBAL_NEXT_PORT.fetch_add(rand, Ordering::SeqCst);
			rand + 9000
		} else {
			port
		};

		if is_free(port) {
			return Ok(port);
		}
	}
}
