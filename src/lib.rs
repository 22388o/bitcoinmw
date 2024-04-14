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

//! # Bitcoin Mimblewimble (BMW)
//!
//! &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;[![Build Status](https://dev.azure.com/mwc-project/bitcoinmw/_apis/build/status/cgilliard.bitcoinmw?branchName=main)](https://dev.azure.com/mwc-project/bitcoinmw/_build?definitionId=13)
//! [![Release Version](https://img.shields.io/github/v/release/cgilliard/bitcoinmw.svg)](https://github.com/cgilliard/bitcoinmw/releases)
//! [![Code Coverage](https://img.shields.io/static/v1?label=Code%20Coverage&message=84.93%&color=purple)](https://cgilliard.github.io/bitcoinmw/code_coverage.html)
//! [![Docmentation](https://img.shields.io/static/v1?label=Documentation&message=Rustdoc&color=red)](https://cgilliard.github.io/bitcoinmw/doc/bmw/index.html)
//! [![License](https://img.shields.io/github/license/cgilliard/bitcoinmw.svg)](https://github.com/cgilliard/bitcoinmw/blob/master/LICENSE)
//!
//! <p align="center">
//! <img src="https://raw.githubusercontent.com/cgilliard/bitcoinmw/main/.github/images/bmw_logo.jpg"/>
//! </p>
//! <p align="center"> Core libraries for Bitcoin Mimblewimble (BMW).</p>
//!
//! # Development Status
//!
//! Bitcoin Mimblewimble (BMW) will eventually be a cryptocurrency. It will be based on these core libraries. As they are
//! available, we will document them here.
//!
//! # BMW Eventhandler crate
//!
//! The BMW Eventhandler crate is used to handle events on tcp/ip connections. Both inbound and
//! outbound connections can be handled. It uses epoll on Linux, kqueues on MacOS, and Wepoll on
//! Windows. It is highly scalable and includes a performance measuring tool as well. A simple
//! example can be found below:
//!
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
//! For full details, see [`bmw_evh`].
//!
//! # BMW Utility crate
//!
//! The BMW Utility crate includes utilities used throughout BMW. The [`bmw_util::Hashtable`], [`bmw_util::Hashset`],
//! [`bmw_util::List`], [`bmw_util::Array`], [`bmw_util::ArrayList`], [`bmw_util::Stack`], [`bmw_util::Queue`]
//! data structures are designed to reduce memory allocations after initialization. The
//! [`bmw_util::lock_box`] macro is used for locking, the rand module contains
//! cryptographically secure system random number generators, and [`bmw_util::thread_pool`]
//! implements a thread pool. For full details on the utility module, see [`bmw_util`].
//!
//! # BMW Configuration crate
//!
//! The BMW Configuration library is used to configure other crates within the BMW project. An
//! example of how it may be used can be found below:
//!
//!```
//! // use all from bmw_conf
//! use bmw_conf::*;
//! // use all from bmw_err
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // build a config using the bmw_conf::config macro.
//!     let config = config!(MaxLoadFactor(0.4), SlabSize(100), SlabCount(200));
//!     // check the config based on allowed and required configuration options.
//!     config.check_config(
//!         vec![
//!             ConfigOptionName::MaxLoadFactor,
//!             ConfigOptionName::SlabSize,
//!             ConfigOptionName::SlabCount,
//!             ConfigOptionName::AutoRotate
//!         ],
//!         vec![
//!             ConfigOptionName::SlabCount
//!         ]
//!     )?;
//!
//!     // retrieve specified values or use defaults.
//!     assert_eq!(config.get_or_f64(&ConfigOptionName::MaxLoadFactor, 0.5), 0.4);
//!     assert_eq!(config.get_or_usize(&ConfigOptionName::SlabSize, 12), 100);
//!     assert_eq!(config.get_or_usize(&ConfigOptionName::SlabCount, 100), 200);
//!     Ok(())
//! }
//!```
//!
//! Full details of the BMW configuration crate can be found here: [`bmw_conf`].
//!
//! # BMW Logging crate
//!
//! The BMW Logging library is used to log data in other crates within the BMW project. An example
//! of how it may be used can be found below:
//!
//!```
//! use bmw_err::*;
//! use bmw_log::*;
//! use bmw_test::*;
//! use std::path::PathBuf;
//!
//! info!(); // set the log level of the global logger to 'info'.
//!
//! fn global_logger() -> Result<(), Error> {
//!     // get test_info for a unique test directory
//!     let test_info = test_info!()?;
//!
//!     // create a path_buf
//!     let mut buf = PathBuf::new();
//!     buf.push(test_info.directory());
//!     buf.push("mylog.log");
//!     let buf = buf.display().to_string();
//!
//!     // init the log. Important to do this before any logging takes place or a default log
//!     // config will be applied
//!     log_init!(
//!         AutoRotate(true), // turn on autorotation
//!         LogFilePath(&buf), // log to our log file
//!         MaxSizeBytes(1024 * 1024), // do a rotation when the log file reaches 1mb
//!         MaxAgeMillis(60 * 60 * 1000) // do a rotation when the log file is over 1 hour old
//!     )?;
//!
//!     // log at the info level
//!     info!("Starting up the logger")?;
//!
//!     // log at the debug level
//!     debug!("This will not show up because 'debug' is below 'info'")?;
//!     Ok(())
//! }
//!
//! // example of an independent logger
//! fn independent_logger() -> Result<(), Error> {
//!     // get a test_info to get a unique test directory
//!     let test_info = test_info!()?;
//!
//!     // create the path buffer with our log name
//!     let mut buf = PathBuf::new();
//!     buf.push(test_info.directory());
//!     buf.push("some_log.log");
//!     let buf = buf.display().to_string();
//!
//!     // create the logger with the logger macro.
//!     let mut logger = logger!(
//!         LogFilePath(&buf), // our path
//!         MaxAgeMillis(1000 * 30 * 60), // log max age before rotation
//!         DisplayColors(false), // don't display colors
//!         DisplayBacktrace(false) // don't show the backtrace on error/fatal log lines
//!     )?;
//!
//!     logger.init()?;
//!     logger.set_log_level(LogLevel::Debug);
//!     logger.log(LogLevel::Debug, "this is a test")?;
//!
//!     Ok(())
//! }
//! fn main() -> Result<(), Error> {
//!     global_logger()?;
//!     independent_logger()?;
//!     Ok(())
//! }
//!```
//!
//! The default output will look something like this:
//!
//!```text
//! [2022-02-24 13:52:24.123]: (FATAL) [..ibconcord/src/main.rs:116]: fatal
//! [2022-02-24 13:52:24.123]: (ERROR) [..ibconcord/src/main.rs:120]: error
//! [2022-02-24 13:52:24.123]: (WARN) [..ibconcord/src/main.rs:124]: warn
//! [2022-02-24 13:52:24.123]: (INFO) [..ibconcord/src/main.rs:128]: info
//! [2022-02-24 13:52:24.123]: (DEBUG) [..ibconcord/src/main.rs:132]: debug
//! [2022-02-24 13:52:24.123]: (TRACE) [..ibconcord/src/main.rs:136]: trace
//!```
//!
//! If enabled, color coding is included as well.
//!
//! Full details of the BMW logging crate can be found here: [`bmw_log`].
//!
//! # BMW Error crate
//!
//! The BMW Error crate is used to handle errors in the other BMW crates. The two main useful
//! macros from this crate are the [`bmw_err::err!`] macro and the [`bmw_err::map_err`] macro.
//!
//! Full details of the BMW logging crate can be found here: [`bmw_err`].
