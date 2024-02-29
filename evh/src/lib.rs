// Copyright (c) 2023, The BitcoinMW Developers
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

//! This crate defines and implements the [`crate::EventHandler`]. EventHandlers process
//! nonblocking i/o events. They are implemented for linux, windows, and macos. Each platform has
//! a different implementation due to the differences between these platforms. For linux, epoll is
//! used. On macos kqueues are used. On windows, wepoll is used. The result is a cross-platform,
//! performant nonblocking i/o event handler.
//!
//! # Performance
//!
//! The performance tool included in `etc/evh_perf` shows the performance of the eventhandler. The output below
//! shows a run which completed successfully with 30 million messages with an average of
//! just over 1.5 million messages per second and an average latency just under 10 ms.
//! This run was on a six core linux box with 2.9 ghz cpus. The details on the performance tool can
//! be found in the <project_directory>/etc/evh_perf directory.
//!
//!```text
//!$ ./target/release/evh_perf -e -c -t 30 --count 1000 -i 100 --reconns 10  --read_slab_count 10000 --max_handles_per_thread 1000
//! [2024-02-11 20:24:04.893]: evh_perf Client/0.0.3-beta.1
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:04.894]: debug:                  'false'
//! [2024-02-11 20:24:04.894]: host:                   '127.0.0.1'
//! [2024-02-11 20:24:04.894]: max_handles_per_thread: '1,000'
//! [2024-02-11 20:24:04.894]: port:                   '8081'
//! [2024-02-11 20:24:04.894]: read_slab_count:        '10,000'
//! [2024-02-11 20:24:04.894]: reuse_port:             'false'
//! [2024-02-11 20:24:04.894]: threads:                '30'
//! [2024-02-11 20:24:04.894]: tls:                    'false'
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:05.027]: (INFO) Server started in 140 ms.
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:05.027]: (INFO) clients:                '1'
//! [2024-02-11 20:24:05.027]: (INFO) count:                  '1,000'
//! [2024-02-11 20:24:05.027]: (INFO) debug:                  'false'
//! [2024-02-11 20:24:05.027]: (INFO) histo:                  'false'
//! [2024-02-11 20:24:05.027]: (INFO) histo_delta_micros:     '10'
//! [2024-02-11 20:24:05.027]: (INFO) host:                   '127.0.0.1'
//! [2024-02-11 20:24:05.027]: (INFO) iterations:             '100'
//! [2024-02-11 20:24:05.027]: (INFO) max:                    '10'
//! [2024-02-11 20:24:05.027]: (INFO) max_handles_per_thread: '1,000'
//! [2024-02-11 20:24:05.027]: (INFO) min:                    '3'
//! [2024-02-11 20:24:05.027]: (INFO) port:                   '8081'
//! [2024-02-11 20:24:05.027]: (INFO) read_slab_count:        '10,000'
//! [2024-02-11 20:24:05.027]: (INFO) reconns:                '10'
//! [2024-02-11 20:24:05.027]: (INFO) sleep:                  '0'
//! [2024-02-11 20:24:05.027]: (INFO) threads:                '30'
//! [2024-02-11 20:24:05.027]: (INFO) tls:                    'false'
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:05.032]: (INFO) Client started in 5 ms.
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:08.032]: (INFO) 4,683,006 of 30,000,000 messages received. [15.61% complete]
//! [2024-02-11 20:24:08.032]: (INFO) incremental_messages=[4,683,006],elapsed_time=[3.00s]
//! [2024-02-11 20:24:08.032]: (INFO) incremental_mps=[1,561,002],incremental_avg_latency=[9357.27µs]
//! [2024-02-11 20:24:08.032]: (INFO) total_messages=[4,683,006],elapsed_time=[3.01s]
//! [2024-02-11 20:24:08.032]: (INFO) total_mps=[1,558,183],total_avg_latency=[9357.27µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:11.032]: (INFO) 9,241,172 of 30,000,000 messages received. [30.80% complete]
//! [2024-02-11 20:24:11.033]: (INFO) incremental_messages=[4,558,166],elapsed_time=[3.00s]
//! [2024-02-11 20:24:11.033]: (INFO) incremental_mps=[1,519,389],incremental_avg_latency=[9487.47µs]
//! [2024-02-11 20:24:11.033]: (INFO) total_messages=[9,241,172],elapsed_time=[6.01s]
//! [2024-02-11 20:24:11.033]: (INFO) total_mps=[1,538,750],total_avg_latency=[9421.49µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:14.033]: (INFO) 13,847,143 of 30,000,000 messages received. [46.16% complete]
//! [2024-02-11 20:24:14.033]: (INFO) incremental_messages=[4,605,971],elapsed_time=[3.00s]
//! [2024-02-11 20:24:14.033]: (INFO) incremental_mps=[1,535,324],incremental_avg_latency=[10050.20µs]
//! [2024-02-11 20:24:14.033]: (INFO) total_messages=[13,847,143],elapsed_time=[9.01s]
//! [2024-02-11 20:24:14.033]: (INFO) total_mps=[1,537,567],total_avg_latency=[9630.62µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:17.033]: (INFO) 18,472,782 of 30,000,000 messages received. [61.58% complete]
//! [2024-02-11 20:24:17.033]: (INFO) incremental_messages=[4,625,639],elapsed_time=[3.00s]
//! [2024-02-11 20:24:17.033]: (INFO) incremental_mps=[1,541,880],incremental_avg_latency=[9338.16µs]
//! [2024-02-11 20:24:17.033]: (INFO) total_messages=[18,472,782],elapsed_time=[12.01s]
//! [2024-02-11 20:24:17.033]: (INFO) total_mps=[1,538,619],total_avg_latency=[9557.39µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:20.033]: (INFO) 23,062,987 of 30,000,000 messages received. [76.88% complete]
//! [2024-02-11 20:24:20.033]: (INFO) incremental_messages=[4,590,205],elapsed_time=[3.00s]
//! [2024-02-11 20:24:20.033]: (INFO) incremental_mps=[1,530,068],incremental_avg_latency=[10248.32µs]
//! [2024-02-11 20:24:20.033]: (INFO) total_messages=[23,062,987],elapsed_time=[15.01s]
//! [2024-02-11 20:24:20.033]: (INFO) total_mps=[1,536,893],total_avg_latency=[9694.90µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:23.033]: (INFO) 27,576,604 of 30,000,000 messages received. [91.92% complete]
//! [2024-02-11 20:24:23.033]: (INFO) incremental_messages=[4,513,617],elapsed_time=[3.00s]
//! [2024-02-11 20:24:23.033]: (INFO) incremental_mps=[1,504,539],incremental_avg_latency=[10197.02µs]
//! [2024-02-11 20:24:23.033]: (INFO) total_messages=[27,576,604],elapsed_time=[18.01s]
//! [2024-02-11 20:24:23.033]: (INFO) total_mps=[1,531,489],total_avg_latency=[9777.09µs]
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:24:25.007]: (INFO) Perf test completed successfully!
//! [2024-02-11 20:24:25.007]: (INFO) total_messages=[30,000,000],elapsed_time=[19.98s]
//! [2024-02-11 20:24:25.007]: (INFO) messages_per_second=[1,501,488],average_latency=[9451.47µs]
//!```
//!
//! The evh_perf tool also has a --histo option that can display a histogram. Below is the output
//! of a histogram on a run of the evh_perf tool on the same system as above.
//!
//!```text
//!$ ./target/release/evh_perf -e -c -t 3 --count 1 -i 100 --reconns 10  --read_slab_count 10000 --max_handles_per_thread 1000 --histo --histo_delta_micros 3
//! [2024-02-11 20:35:20.101]: evh_perf Client/0.0.3-beta.1
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:35:20.101]: debug:                  'false'
//! [2024-02-11 20:35:20.101]: host:                   '127.0.0.1'
//! [2024-02-11 20:35:20.101]: max_handles_per_thread: '1,000'
//! [2024-02-11 20:35:20.101]: port:                   '8081'
//! [2024-02-11 20:35:20.101]: read_slab_count:        '10,000'
//! [2024-02-11 20:35:20.101]: reuse_port:             'false'
//! [2024-02-11 20:35:20.101]: threads:                '3'
//! [2024-02-11 20:35:20.101]: tls:                    'false'
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:35:20.114]: (INFO) Server started in 19 ms.
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:35:20.115]: (INFO) clients:                '1'
//! [2024-02-11 20:35:20.115]: (INFO) count:                  '1'
//! [2024-02-11 20:35:20.115]: (INFO) debug:                  'false'
//! [2024-02-11 20:35:20.115]: (INFO) histo:                  'true'
//! [2024-02-11 20:35:20.115]: (INFO) histo_delta_micros:     '3'
//! [2024-02-11 20:35:20.115]: (INFO) host:                   '127.0.0.1'
//! [2024-02-11 20:35:20.115]: (INFO) iterations:             '100'
//! [2024-02-11 20:35:20.115]: (INFO) max:                    '10'
//! [2024-02-11 20:35:20.115]: (INFO) max_handles_per_thread: '1,000'
//! [2024-02-11 20:35:20.115]: (INFO) min:                    '3'
//! [2024-02-11 20:35:20.115]: (INFO) port:                   '8081'
//! [2024-02-11 20:35:20.115]: (INFO) read_slab_count:        '10,000'
//! [2024-02-11 20:35:20.115]: (INFO) reconns:                '10'
//! [2024-02-11 20:35:20.115]: (INFO) sleep:                  '0'
//! [2024-02-11 20:35:20.115]: (INFO) threads:                '3'
//! [2024-02-11 20:35:20.115]: (INFO) tls:                    'false'
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:35:20.116]: (INFO) Client started in 1 ms.
//! ----------------------------------------------------------------------------------------------------
//! [2024-02-11 20:35:20.153]: (INFO) Perf test completed successfully!
//! [2024-02-11 20:35:20.153]: (INFO) total_messages=[3,000],elapsed_time=[0.04s]
//! [2024-02-11 20:35:20.153]: (INFO) messages_per_second=[77,097],average_latency=[22.73µs]
//! ----------------------------------------------------------------------------------------------------
//! Latency Histogram
//! ----------------------------------------------------------------------------------------------------
//! [12µs  - 15µs ]=> 25 (0.83%)
//! [15µs  - 18µs ]====================> 613 (20.43%)
//! [18µs  - 21µs ]=============================================> 1,361 (45.37%)
//! [21µs  - 24µs ]===============> 463 (15.43%)
//! [24µs  - 27µs ]====> 131 (4.37%)
//! [27µs  - 30µs ]====> 120 (4.00%)
//! [30µs  - 33µs ]===> 77 (2.57%)
//! [33µs  - 36µs ]==> 63 (2.10%)
//! [36µs  - 39µs ]=> 44 (1.47%)
//! [39µs  - 42µs ]=> 18 (0.60%)
//! [42µs  - 45µs ]> 13 (0.43%)
//! [45µs  - 48µs ]> 7 (0.23%)
//! [48µs  - 51µs ]> 3 (0.10%)
//! [51µs  - 54µs ]> 1 (0.03%)
//! [54µs  - 57µs ]> 1 (0.03%)
//! [57µs  - 60µs ]> 3 (0.10%)
//! [60µs  - 63µs ]> 5 (0.17%)
//! [63µs  - 66µs ]=> 18 (0.60%)
//! [66µs  - 69µs ]> 1 (0.03%)
//! [69µs  - 72µs ]> 3 (0.10%)
//! [72µs  - 75µs ]> 4 (0.13%)
//! [75µs  - 78µs ]> 6 (0.20%)
//! [78µs  - 81µs ]> 4 (0.13%)
//! [84µs  - 87µs ]> 1 (0.03%)
//! [87µs  - 90µs ]> 4 (0.13%)
//! [90µs  - 93µs ]> 2 (0.07%)
//! [93µs  - 96µs ]> 1 (0.03%)
//! [96µs  - 99µs ]> 3 (0.10%)
//! [99µs  - 102µs]> 2 (0.07%)
//! [111µs - 114µs]> 1 (0.03%)
//! [831µs - 834µs]> 1 (0.03%)
//! [849µs - 852µs]> 1 (0.03%)
//! ----------------------------------------------------------------------------------------------------
//!```
//!
//! As seen above, with fewer requests per second, the latency improves.
//!
//! # Using eventhandlers in your project
//!
//! Add the following to your Cargo.toml:
//!
//!```text
//! bmw_evh = { git = "https://github.com/cgilliard/bitcoinmw"  }
//!```
//!
//! Optionally, you may wish to use the other associated crates:
//!
//!```text
//! bmw_err    = { git = "https://github.com/cgilliard/bitcoinmw"  }
//! bmw_log    = { git = "https://github.com/cgilliard/bitcoinmw"  }
//! bmw_derive = { git = "https://github.com/cgilliard/bitcoinmw"  }
//! bmw_util   = { git = "https://github.com/cgilliard/bitcoinmw"  }
//!```
//!
//! The linux dependencies can be installed with the following commands on ubuntu:
//!
//!```text
//! $ sudo apt-get update -yqq
//! $ sudo apt-get install -yqq --no-install-recommends libncursesw5-dev libssl-dev
//!```
//!
//! The macos dependencies can be installed with the following commands
//! ```text
//! $ brew install llvm
//! ```
//!
//! The windows dependencies can be installed with the following commands
//!
//! ```text
//! $ choco install -y llvm
//! ```
//!
//! BitcoinMW is tested with the latest version of rust. Please ensure to update it to the latest version.
//!
//! # Examples
//!
//!```
//! // Echo Server
//!
//! // import the error, log, evh crate and several other things
//! use bmw_err::*;
//! use bmw_evh::*;
//! use bmw_log::*;
//! use bmw_test::port::pick_free_port;
//! use std::net::TcpStream;
//! use std::io::{Read,Write};
//!
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     // create an evh instance with the default configuration
//!     let mut evh = eventhandler!()?;
//!
//!     // set the on read handler for this evh
//!     evh.set_on_read(move |cd, _ctx, _attachment| {
//!         // log the connection_id of this connection. The connection_id is a random u128
//!         //value. Each connection has a unique id.
//!         info!("read data on connection {}", cd.get_connection_id())?;
//!
//!         // data read is stored in a linked list of slabs. first_slab returns the first
//!         // slab in the list.
//!         let first_slab = cd.first_slab();
//!
//!         // in this example, we don't use it, but we could get the last slab in the list
//!         // if more than one slab of data may be returned.
//!         let _last_slab = cd.last_slab();
//!
//!         // get the slab_offset. This is the offset in the last slab read. The slabs
//!         // before the last slab will be full so no offset is needed for them. In this
//!         // example, we always have only a single slab so the offset is always the offset
//!         // of the slab we are looking at.
//!         let slab_offset = cd.slab_offset();
//!
//!         // the borrow slab allocator function allows for the on_read callback to analyze
//!         // the data that has been read by this connection. The slab_allocator that is
//!         // passed to the closure is immutable so none of the data can be modified.
//!         let res = cd.borrow_slab_allocator(move |sa| {
//!             // get the first slab
//!             let slab = sa.get(first_slab.try_into()?)?;
//!
//!             // log the number of bytes that have been read
//!             info!("read {} bytes", slab_offset)?;
//!
//!             // create a vec and extend it with the data that was read
//!             let mut ret: Vec<u8> = vec![];
//!             ret.extend(&slab.get()[0..slab_offset as usize]);
//!
//!             // Return the data that was read. The return value is a generic so it
//!             // could be any type. In this case, we return a Vec<u8>.
//!             Ok(ret)
//!         })?;
//!
//!         // Clear all the data through the first slab, which in this example is assumed
//!         // to be the last slab. Once this function is called, the subsequent executions
//!         // of this callback will not include this slab.
//!         cd.clear_through(first_slab)?;
//!
//!         // Return a write handle and echo back the data that was read.
//!         cd.write_handle().write(&res)?;
//!
//!         Ok(())
//!     })?;
//!     evh.set_on_accept(move |cd, _ctx| {
//!         // The on_accept callback is executed when a connection is accepted.
//!         info!("accepted connection id = {}", cd.get_connection_id())?;
//!         Ok(())
//!     })?;
//!     evh.set_on_close(move |cd, _ctx| {
//!         // The on_close callback is executed when a connection is closed.
//!         info!("closed connection id = {}", cd.get_connection_id())?;
//!         Ok(())
//!     })?;
//!     evh.set_on_panic(move |_ctx, e| {
//!         // The error is returned by the panic handler as a Box<dyn Any> so we downcast
//!         // to &str to get the message.
//!         let e = e.downcast_ref::<&str>().unwrap();
//!         // The on_panic callback is executed when a thread panic occurs.
//!         warn!("callback generated thread panic: {}", e)?;
//!         Ok(())
//!     })?;
//!     evh.set_housekeeper(move |_ctx| {
//!         // The housekeper callback is executed once per thread every second by default.
//!         info!("Housekeeper executed")?;
//!         Ok(())
//!     })?;
//!
//!     // start the evh
//!     evh.start()?;
//!
//!     // pick a free port for our server to bind to
//!     let (addr, handles) = loop {
//!         let port = pick_free_port()?;
//!         info!("using port = {}", port);
//!         // bind to the loopback interface.
//!         let addr = format!("127.0.0.1:{}", port).clone();
//!
//!         // create our server handles for the default 6 threads of the evh.
//!         // We use a tcp_listener backlog of 10 in this example and we're setting
//!         // SO_REUSE_PORT to true.
//!         let handles = create_listeners(6, &addr, 10, true);
//!         match handles {
//!             Ok(handles) => break (addr, handles),
//!             Err(_e) => {}
//!         }
//!     };
//!
//!     // create a ServerConnection with no tls configurations so it will be plain
//!     // text.
//!     let sc = ServerConnection {
//!         tls_config: None,
//!         handles,
//!         is_reuse_port: true,
//!     };
//!
//!     // add our server connection to the evh.
//!     evh.add_server(sc, Box::new(""))?;
//!
//!     // create a client connection to test the evh
//!     let mut connection = TcpStream::connect(addr)?;
//!
//!     // send a message "test1".
//!     connection.write(b"test1")?;
//!
//!     // assert that the response is an echo of our message.
//!     let mut buf = vec![];
//!     buf.resize(100, 0u8);
//!     let len = connection.read(&mut buf)?;
//!     assert_eq!(&buf[0..len], b"test1");
//!
//!     // send a second message "test2".
//!     connection.write(b"test2")?;
//!
//!     // assert that the response is an echo of our message.
//!     let len = connection.read(&mut buf)?;
//!     assert_eq!(&buf[0..len], b"test2");
//!
//!     // stop the evh
//!     evh.stop()?;
//!
//!     Ok(())
//! }
//!
//!```

mod builder;
mod evh;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
mod macros;
mod test;
mod types;
#[cfg(windows)]
mod win;

pub use crate::types::{
	AttachmentHolder, Builder, ClientConnection, CloseHandle, ConnData, ConnectionData,
	EventHandler, EventHandlerConfig, EventHandlerController, EventHandlerData, Handle,
	ServerConnection, ThreadContext, TlsClientConfig, TlsServerConfig, WriteHandle, WriteState,
};

pub use crate::evh::{
	close_handle, create_listeners, tcp_stream_to_handle, READ_SLAB_DATA_SIZE,
	READ_SLAB_NEXT_OFFSET, READ_SLAB_SIZE,
};
