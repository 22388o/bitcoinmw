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

/// Returns a free port that is not used at the time of the call. It is also guaranteed to not be
/// allocated to another test which calls this macro, so there are no timing concerns.
#[macro_export]
macro_rules! free_port {
	() => {{
		use bmw_test::pick_free_port;
		pick_free_port()
	}};
}

/// Macro to setup a test directory based on the function name. A free port
/// is also returned. The directory is removed when the returned value goes
/// out of scope unless the `preserve` value is specifed and set to true.
/// Specifically a [`crate::TestInfo`] is returned by this macro.
#[macro_export]
macro_rules! test_info {
	() => {{
		test_info!(false)
	}};
	($preserve:expr) => {{
		use bmw_test::TestInfo;
		TestInfo::new($preserve)
	}};
}
