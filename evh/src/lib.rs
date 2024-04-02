// Copyright (c) 2023-2024, The BitcoinMW Developers
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

//! # The BMW Eventhandler crate
//! This crate defines and implements the [`crate::EventHandler`] trait. The event handler handles
//! events on tcp/ip connections. It manages both inbound and outbound connections. The underlying
//! mechanism used are Epoll on Linux, Kqueues on MacOS and WePoll on Windows. These libraries
//! allow for perfromant handling of reads and writes on multiple socket connections.
//! # Motivation
//! Eventhandler provides an fast interface to the low level eventing libraries on various
//! platforms. It will be the basis for the HTTP server and the rustlet library and eventually the
//! cryptocurrency that will be built on top of these libraries.
//! # Performance
//! The /etc directory in the projects inlcudes a subdirectory called "evh_perf". This subdirectory
//! is used for testing the performance of the eventhandler.
//! # Examples
//!
//!
mod builder;
mod constants;
mod evh;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
mod macros;
mod test;
mod types;
#[cfg(target_os = "windows")]
mod win;

pub use crate::types::{
	Chunk, Connection, EventHandler, EvhBuilder, EvhStats, UserContext, WriteHandle,
};
