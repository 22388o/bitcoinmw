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
//! TBD
//!
//! # Using eventhandlers in your project
//!
//! Add the following to your Cargo.toml:
//!
//!```text
//! bmw_evh = { git = "https://github.com/37miners/bmw"  }
//!```
//!
//! Optionally, you may wish to use the other associated crates:
//!
//!```text
//! bmw_err    = { git = "https://github.com/37miners/bmw"  }
//! bmw_log    = { git = "https://github.com/37miners/bmw"  }
//! bmw_derive = { git = "https://github.com/37miners/bmw"  }
//! bmw_util   = { git = "https://github.com/37miners/bmw"  }
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
//! BitcoinMW is tested with the latest version of rust. Please ensure to update to it.
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
//!         tls_config: vec![],
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
mod types;
#[cfg(windows)]
mod win;

pub use crate::types::{
	AttachmentHolder, Builder, ClientConnection, ConnData, ConnectionData, EventHandler,
	EventHandlerConfig, ServerConnection, ThreadContext, TlsClientConfig, TlsServerConfig,
	WriteHandle,
};

pub use crate::evh::{create_listeners, READ_SLAB_DATA_SIZE};
