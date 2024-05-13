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
	/// # Input Parameters
	/// * `self` - `[&self]` - the [`crate::TestInfo`] impl assigned to this test.
	/// # Return
	/// [`u16`] - a free tcp/ip port that can be used in this test. This value is guaranteed
	/// not to be assigned to other tests.
	/// # Also see
	/// * [`crate::TestInfo::directory`]
	/// * [`crate::TestInfo::sync_channel`]
	fn port(&self) -> u16;
	/// Return a directory that can be used by the test. It is automatically deleted when the
	/// [`crate::TestInfo`] goes out of scope.
	/// # Input Parameters
	/// * `self` - `[&self]` - the [`crate::TestInfo`] impl assigned to this test.
	/// # Return
	/// &[`std::string::String`] - a reference to a string which is the path to a newly created,
	/// empty directory which can be used by the test.
	/// This directory is deleted through the [`std::ops::Drop`] handler when the
	/// [`crate::TestInfo`] object goes out of scope.
	/// # Also see
	/// * [`crate::TestInfo::port`]
	/// * [`crate::TestInfo::sync_channel`]
	fn directory(&self) -> &String;
	/// Return a (std::sync::mpsc::SyncSender<()>, std::sync::mpsc::Receiver<()>) in which the sender
	/// will automatically send message after 60 seconds. This allows threads to timeout.
	/// This is useful in eventhandler / http / rustlet testing.
	/// # Input Parameters
	/// * `self` - `[&self]` - the [`crate::TestInfo`] impl assigned to this test.
	/// # Return
	/// `(SyncSender<()>, Receiver<()>)` - a send/receive sync channel pair. After one minute,
	/// the `SyncSender` will send a [`unit`] message to the reciever. This is helpful for
	/// causing timed out tests to fail so that useful info about why it failed may be
	/// obtained.
	/// # Also see
	/// * [`crate::TestInfo::port`]
	/// * [`crate::TestInfo::directory`]
	/// * [`std::sync::mpsc::SyncSender`]
	/// * [`std::sync::mpsc::Receiver`]
	fn sync_channel(&self) -> (SyncSender<()>, Receiver<()>);
}

/// Kinds of errors that can occur in or are related to tests.
pub enum TestErrorKind {
	/// A test generated error
	Test(String),
	/// The resource was not available
	ResourceNotAvailable(String),
}

// A builder that is used to construct TestInfo implementations. This is typically called through
// the test_info macro.
#[doc(hidden)]
pub struct TestBuilder {}

// re-export a few useful things for tests
#[doc(hidden)]
pub use std::sync::mpsc::sync_channel;
#[doc(hidden)]
pub use std::thread::sleep;
#[doc(hidden)]
pub use std::thread::spawn;
#[doc(hidden)]
pub use std::time::Duration;

/// A sleep time designed to allow execution of tests to run to completion. We avoid using this
/// sleep parameter, but it is needed in a few cases.
pub const QA_SLEEP: u64 = 3_000;
