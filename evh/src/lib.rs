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
//! Eventhandler provides a convenient interface to the low level eventing libraries on various
//! platforms. It is the basis for the HTTP server and the Rustlet library and eventually the
//! cryptocurrency that will be built on top of these libraries. The event handler is designed to
//! address the [C10m
//! problem](https://migratorydata.com/blog/migratorydata-solved-the-c10m-problem/). Benchmarks
//! indicate it is possible to get over 10 million connections if 32 GB of RAM are avilable.
//! Separately, the eventhandler can handle over 2 million messages per second. See the performance
//! section for further details.
//! # Performance
//! The /etc directory in the project inlcudes a subdirectory called "evh_perf". This subdirectory
//! is used for testing the performance of the eventhandler. The README for this tool can be found
//! on [Github](https://github.com/cgilliard/bitcoinmw/tree/main/etc/evh_perf).
//! # Examples
//!```
//! use bmw_err::*;
//! use bmw_evh::*;
//! use bmw_log::*;
//! use std::str::from_utf8;
//!
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     // create an evh with the specified configuration.
//!     // This example shows all possible configuration options, but all of
//!     // are optional. See the macro's documentation for full details.
//!     let mut evh = evh_oro!(
//!         EvhTimeout(100), // set timeout to 100 ms.
//!         EvhThreads(1), // 1 thread
//!         EvhReadSlabSize(100), // 100 byte slab size
//!         EvhReadSlabCount(100), // 100 slabs
//!         EvhHouseKeeperFrequencyMillis(1_000), // run the house keeper every 1_000 ms.
//!         EvhStatsUpdateMillis(5_000), // return updated stats every 5_000 ms.
//!         Debug(true) // print additional debugging information.
//!     )?;
//!
//!     // set the on read handler
//!     evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
//!         // loop through each of the available chunks and append data to a vec.
//!         let mut data: Vec<u8> = vec![];
//!
//!         loop {
//!             let next_chunk = ctx.next_chunk(connection)?;
//!             cbreak!(next_chunk.is_none());
//!             let next_chunk = next_chunk.unwrap();
//!             data.extend(next_chunk.data());
//!         }
//!
//!         // convert returned data to a utf8 string
//!         let dstring = from_utf8(&data)?;
//!         info!("data[{}]='{}'", connection.id(), dstring)?;
//!
//!         // get a write handle
//!         let mut wh = connection.write_handle()?;
//!
//!         // echo
//!         wh.write(dstring.as_bytes())?;
//!
//!         // clear all chunks from this connection. Note that partial
//!         // clearing is possible with the ctx.clear_through function
//!         // or no data can be cleared at all in which case it can
//!         // be accessed on a subsequent request. When the connection
//!         // is closed, all data is cleared automatically.
//!         ctx.clear_all(connection)?;
//!
//!         Ok(())
//!     })?;
//!
//!     // no other handlers are necessary
//!
//!     evh.start()?;
//!
//!     Ok(())
//! }
//!```
//! The above example uses the `on_read_only` implementation which does not require the user to
//! define the other handlers. See [`crate::evh!`] and [`crate::evh_oro`] for full details.
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
