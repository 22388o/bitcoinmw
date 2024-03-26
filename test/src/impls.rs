// Copyright (c) 2023-2024, The BitcoinMW Developers
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

use crate::types::TestInfoImpl;
use crate::TestInfo;
use bmw_deps::backtrace;
use bmw_deps::portpicker::is_free;
use bmw_deps::rand::random;
use bmw_err::Error;
use std::fs::{create_dir_all, remove_dir_all};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread::{sleep, spawn};
use std::time::Duration;

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

// global counter for getting a port number
static GLOBAL_NEXT_PORT: AtomicU16 = AtomicU16::new(9000);

// pick a free port that does not collide with recently assigned ports
fn pick_free_port() -> Result<u16, Error> {
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

impl TestInfo for TestInfoImpl {
	fn directory(&self) -> &String {
		&self.directory
	}

	fn port(&self) -> u16 {
		self.port
	}
	fn sync_channel(&self) -> (SyncSender<()>, Receiver<()>) {
		let (tx, rx) = sync_channel(1);
		let tx_clone = tx.clone();
		spawn(move || -> Result<(), Error> {
			sleep(Duration::from_millis(60_000));
			match tx_clone.send(()) {
				Ok(_) => {}
				Err(e) => println!(
					"an error occurred while trying to send timeout (TestInfoImpl): {}",
					e
				),
			}
			Ok(())
		});
		(tx, rx)
	}
}

impl TestInfoImpl {
	pub(crate) fn new(preserve: bool) -> Result<Self, Error> {
		let mut directory = Default::default();
		backtrace::trace(|frame| {
			backtrace::resolve_frame(frame, |symbol| {
				// don't think symbol.name() can be none, but this is only used in
				// tests, so even if it is, it's ok.
				directory = symbol.name().unwrap().to_string();
			});
			// wait until we get to the actual test directory name.
			if !directory.starts_with("backtrace")
				&& !directory.contains("bmw_test::types::TestInfoImpl")
				&& !directory.contains("bmw_test::builder::")
				&& !directory.contains("bmw_test::types::Builder")
			{
				false
			} else {
				true
			}
		});

		let port = pick_free_port()?;
		let directory = directory.replace("::", "_").to_string();
		let directory = format!(".{}.bmw", directory);
		let d = directory.clone();
		// remove the directory if it existed from a previous failed run
		let _ = remove_dir_all(d);
		let d = directory.clone();
		let _ = create_dir_all(d);

		let ret = Self {
			directory,
			port,
			preserve,
		};
		Ok(ret)
	}
}

impl Drop for TestInfoImpl {
	fn drop(&mut self) {
		// if we're not preserving the directory, delete it on drop.
		if !self.preserve {
			let _ = remove_dir_all(self.directory.clone());
		}
	}
}
