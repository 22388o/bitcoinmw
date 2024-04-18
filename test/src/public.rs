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

use std::sync::mpsc::{Receiver, SyncSender};

/// This trait defines the data which a test can use. A test can obtain one of these by calling the
/// [`crate::test_info`] macro.
pub trait TestInfo {
	/// Return a port that can be used by the test.
	fn port(&self) -> u16;
	/// Return a directory that can be used by the test. It is automatically deleted when the
	/// [`crate::TestInfo`] goes out of scope.
	fn directory(&self) -> &String;
	/// Return a (std::sync::mpsc::SyncSender<()>, std::sync::mpsc::Receiver<()>) in which the sender
	/// will automatically send message after 60 seconds. This allows threads to timeout.
	/// This is useful in eventhandler / http / rustlet testing.
	fn sync_channel(&self) -> (SyncSender<()>, Receiver<()>);
}

/// A builder that is used to construct TestInfo implementations. This is typically called through
/// the [`crate::test_info`] macro.
pub struct TestBuilder {}
