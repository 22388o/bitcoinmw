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

/// This macro returns a free port that is not currently in use. Importantly, it ensures
/// that the port won't be allocated to another test that calls this macro or the
/// [`crate::test_info`] macro, eliminating timing concerns.
/// # Return
/// Result < [`u16`], [`bmw_base::Error`] > - a tcp/ip port that can be used in a test
/// # Errors
/// [`crate::TestErrorKind::ResourceNotAvailable`] - if the port cannot be assigned.
/// # Also see
/// * [`crate::test_info`]
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_test::*;
///
/// fn test_my_fn() -> Result<(), Error> {
///     let port = free_port!()?;
///     println!("Port={}", port);
///     // .. use port here
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! free_port {
	() => {{
		use bmw_base::{err, BaseErrorKind};
		use bmw_test::{TestBuilder, TestErrorKind};
		match TestBuilder::build_test_info(false) {
			Ok(ti) => Ok(ti.port()),
			Err(e) => err!(
				TestErrorKind::ResourceNotAvailable,
				"could not assign a port due to: {}",
				e
			),
		}
	}};
}

/// Macro to setup a test directory based on the function name. A free port
/// is also returned. The directory is removed when the returned value goes
/// out of scope unless the `preserve` value is specifed and set to true.
/// Specifically a [`crate::TestInfo`] is returned by this macro.
/// # Input Parameters
/// * `preserve` - [`bool`] - If set to [`true`] the directory associated with the
/// returned [`crate::TestInfo`] will be preserved at the end of the test. Otherwise, it will be
/// deleted.
/// # Return
/// [`crate::TestInfo`] - a test info impl that can be used to find a unique usable directory and
/// port for this test.
/// # Errors
/// [`crate::TestErrorKind::ResourceNotAvailable`] - if the port cannot be assigned.
/// # Also see
/// * [`crate::free_port`]
/// * [`crate::TestInfo`]
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_test::*;
///
/// fn test_my_fn() -> Result<(), Error> {
///     let test_info = test_info!()?;
///     
///     let directory = test_info.directory();
///     let port = test_info.port();
///
///     // use the directory to write/read files and the port for tcp/ip connections
///     // the directory will be deleted when the test_info impl is dropped (at the
///     // end of this test function.
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! test_info {
	() => {{
		test_info!(false)
	}};
	($preserve:expr) => {{
		use bmw_test::TestBuilder;
		TestBuilder::build_test_info($preserve)
	}};
}
