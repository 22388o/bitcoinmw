// Copyright (c) 2023, The BitcoinMW Developers
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

//! This is the dependency crate. All bmw dependencies are included in this crate as re-exports and
//! used by the other crates within the repo.

#[cfg(windows)]
pub use wepoll_sys;

#[cfg(target_os = "macos")]
pub use kqueue_sys;

pub use aes;
pub use arrayref;
pub use backtrace;
pub use bitvec;
pub use chrono;
pub use cipher;
pub use colored;
pub use digest;
pub use dyn_clone;
pub use ed25519_dalek;
pub use errno;
pub use failure;
pub use failure_derive;
pub use futures;
pub use generic_array;
pub use interprocess;
pub use lazy_static;
pub use libc;
pub use nix;
pub use num_format;
pub use old_rand_core;
pub use openssl;
pub use path_clean;
pub use pem;
pub use portpicker;
pub use rand;
pub use rand_core;
pub use random_string;
pub use rcgen;
pub use rustls;
pub use rustls_pemfile;
pub use sha3;
pub use substring;
pub use subtle;
pub use try_traits;
pub use typenum;
pub use webpki_roots;
pub use winapi;
pub use x25519_dalek;
pub use x509_signature;
pub use zeroize;
