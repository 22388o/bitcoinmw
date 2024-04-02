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

//! # The BMW Dependency crate
//! This is the dependency crate. All BMW dependencies are included in this crate as re-exports and
//! used by the other crates within the repo.

#[cfg(target_os = "windows")]
pub use wepoll_sys;
#[cfg(target_os = "windows")]
pub use windows_sys;

#[cfg(target_os = "macos")]
pub use kqueue_sys;
#[cfg(target_os = "macos")]
pub use libc;
#[cfg(target_os = "macos")]
pub use nix;

#[cfg(target_os = "linux")]
pub use libc;
#[cfg(target_os = "linux")]
pub use nix;

pub use backtrace;
pub use base64;
pub use bitvec;
pub use byteorder;
pub use chrono;
pub use colored;
pub use dirs;
pub use downcast;
pub use dyn_clone;
pub use errno;
pub use failure;
pub use failure_derive;
pub use flate2;
pub use futures;
pub use itertools;
pub use lazy_static;
pub use math;
pub use num_format;
pub use path_clean;
pub use portpicker;
pub use rand;
pub use rand_core;
pub use random_string;
pub use ring;
pub use sha1;
pub use substring;
pub use url;
