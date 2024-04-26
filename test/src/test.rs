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

#[cfg(test)]
mod test {
	use crate as bmw_test;
	use crate::types::TestInfoImpl;
	use crate::{test_info, TestErrorKind, TestInfo};
	use bmw_base::{kind, Error, ErrorKind};
	use std::sync::{Arc, RwLock};
	use std::thread::spawn;

	#[test]
	fn test_test_info_macro() -> Result<(), Error> {
		// create a TestInfo
		let test_info = test_info!()?;

		// port is always at least 9_000
		assert!(test_info.port() >= 9_000);
		// directory ends with bmw
		assert!(test_info.directory().ends_with("bmw"));
		Ok(())
	}

	#[test]
	fn test_other_test() -> Result<(), Error> {
		// once again test the test_info object
		let test_info = test_info!()?;
		assert!(test_info.port() >= 9000);
		assert!(test_info.directory().ends_with("bmw"));

		// assert the type of error returned is as expected
		let err = TestErrorKind::Test("test".to_string());
		let err: Error = err.into();
		assert_eq!(err.kind(), &kind!(TestErrorKind::Test, "test"));

		Ok(())
	}

	#[test]
	fn test_impl() -> Result<(), Error> {
		// test the rx.recv returns in 100 ms
		let test_info = TestInfoImpl::new(false)?;
		let (_tx, rx) = test_info.sync_channel_impl(100);
		assert!(rx.recv().is_ok());

		Ok(())
	}

	#[test]
	fn test_sync_channel() -> Result<(), Error> {
		// create a test info
		let test_info = test_info!()?;

		// create a sync_channel
		let (tx, rx) = test_info.sync_channel();

		// create a rwlock and it's clone
		let v = Arc::new(RwLock::new(false));
		let vc = v.clone();

		// create a new thread
		spawn(move || -> Result<(), Error> {
			// obtain write lock
			let mut v = v.write()?;

			// set the value to true
			*v = true;

			// notify the receiver
			tx.send(())?;
			Ok(())
		});

		// wait for thread to notify us
		rx.recv()?;

		// obtain read lock
		let v = vc.read()?;

		// assert that v is true
		assert!(*v);

		Ok(())
	}
}
