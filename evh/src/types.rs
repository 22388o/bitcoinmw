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

use bmw_deps::rustls::client::ClientConnection as RustlsClientConnection;
use bmw_deps::rustls::server::{ServerConfig, ServerConnection as RustlsServerConnection};
use bmw_derive::Serializable;
use bmw_err::*;
use bmw_util::*;
use std::any::Any;
use std::collections::HashMap;
use std::net::TcpStream;
use std::pin::Pin;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::prelude::RawFd;

#[cfg(target_os = "linux")]
use bmw_deps::bitvec::vec::BitVec;
#[cfg(target_os = "linux")]
use bmw_deps::nix::sys::epoll::{Epoll, EpollEvent};

#[cfg(target_os = "windows")]
use bmw_deps::bitvec::vec::BitVec;

/// TlsServerConfig specifies the configuration for a tls server.
#[derive(Clone, Debug, PartialEq)]
pub struct TlsServerConfig {
	/// The location of the private_key file (privkey.pem).
	pub private_key_file: String,
	/// The location of the certificates file (fullchain.pem).
	pub certificates_file: String,
}

/// TlsClientConfig specifies the configuration for a tls client.
#[derive(PartialEq)]
pub struct TlsClientConfig {
	/// The sni_host you are connecting to.
	pub sni_host: String,
	/// An optional trusted cert full chain file for this tls connection.
	pub trusted_cert_full_chain_file: Option<String>,
}

/// A thread context which is passed to the callbacks specified by a [`crate::EventHandler`].
pub struct ThreadContext {
	/// User may set this to any value and it is passed to the callbacks as a mutable
	/// reference.
	pub user_data: Box<dyn Any + Send + Sync>,
}

/// A struct which specifies a client connection.
pub struct ClientConnection {
	/// The handle (file desciptor on Unix and file handle on Windows) of this client.
	pub handle: Handle,
	/// The optional tls configuration for this client.
	pub tls_config: Option<TlsClientConfig>,
}

/// A struct which specifies a server connection.
pub struct ServerConnection {
	/// An array of handles. The size of this array must be equal to the number of threads that
	/// the [`crate::EventHandler`] has configured.
	pub handles: Array<Handle>,
	/// This is an optional TlsServerConfig for this connection. If none is specified, plain
	/// text is used.
	pub tls_config: Option<TlsServerConfig>,
	pub is_reuse_port: bool,
}

/// A struct which is passed to several of the callbacks in [`crate::EventHandler`]. It provides
/// information on the connection from which data is read.
pub struct ConnectionData<'a> {
	pub(crate) rwi: &'a mut StreamInfo,
	pub(crate) tid: usize,
	pub(crate) slabs: &'a mut Box<dyn SlabAllocator + Send + Sync>,
	pub(crate) event_handler_data: Box<dyn LockBox<EventHandlerData>>,
	pub(crate) debug_write_queue: bool,
	pub(crate) debug_pending: bool,
	pub(crate) debug_write_error: bool,
	pub(crate) debug_suspended: bool,
}

/// A struct which is used to write to a connection.
#[derive(Clone)]
pub struct WriteHandle {
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) id: u128,
	pub(crate) handle: Handle,
	pub(crate) event_handler_data: Box<dyn LockBox<EventHandlerData>>,
	pub(crate) debug_write_queue: bool,
	pub(crate) debug_pending: bool,
	pub(crate) debug_write_error: bool,
	pub(crate) debug_suspended: bool,
	pub(crate) tls_server: Option<Box<dyn LockBox<RustlsServerConnection>>>,
	pub(crate) tls_client: Option<Box<dyn LockBox<RustlsClientConnection>>>,
}

/// A struct which can be used to close a connection (note: if writing is needed as well use
/// WriteHandle, but this is a minimal structure needed to close the connection
pub struct CloseHandle {
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) id: u128,
	pub(crate) event_handler_data: Box<dyn LockBox<EventHandlerData>>,
}

/// This trait which is implemented by [`crate::ConnectionData`]. This trait is used to interact
/// with a connection.
pub trait ConnData {
	/// Returns the thread id that this event occurred on.
	fn tid(&self) -> usize;
	/// Returns the connection id of this event.
	fn get_connection_id(&self) -> u128;
	/// Returns the handle of this event.
	fn get_handle(&self) -> Handle;
	/// Returns the handle (if any) which this connection was accepted on.
	fn get_accept_handle(&self) -> Option<Handle>;
	/// Returns a write handle which can be used to write to this connection.
	fn write_handle(&self) -> WriteHandle;
	/// borrows, immutably, the slab allocator associated with this connection.
	fn borrow_slab_allocator<F, T>(&self, f: F) -> Result<T, Error>
	where
		F: FnMut(&Box<dyn SlabAllocator + Send + Sync>) -> Result<T, Error>;
	/// Returns the offset that data has been read to in the most recent slab for this
	/// connection.
	fn slab_offset(&self) -> u16;
	/// Returns the first slab in which data has been read for this connection.
	fn first_slab(&self) -> u32;
	/// Returns the last slab in which data has been read for this connection.
	fn last_slab(&self) -> u32;
	/// Clears data in slabs from memory up to and including `slab_id` for this connection.
	fn clear_through(&mut self, slab_id: u32) -> Result<(), Error>;
}

/// The configuration for the [`crate::EventHandler`].
#[derive(Clone)]
pub struct EventHandlerConfig {
	/// the number of threads for this [`crate::EventHandler`]. The default value is 6.
	pub threads: usize,
	/// the size of the sync_channel for the [`bmw_util::ThreadPool`] in this [`crate::EventHandler`].
	/// The default value is 10.
	pub sync_channel_size: usize,
	/// the size of the write_queue for this [`crate::EventHandler`]. This is used when a write
	/// operation would block and the data needs to be queued for writing. The default value is
	/// 100,000.
	pub write_queue_size: usize,
	/// The queue size for the new handles in this [`crate::EventHandler`]. Each time a new
	/// connection is added it must be queued. If the connections are accepted and this
	/// [`crate::EventHandler`] is not configured with a reusable port, it must also be queued.
	/// The default value is 1,000.
	pub nhandles_queue_size: usize,
	/// The maximum events to register to the eventhandler per request. Note that this is only
	/// a hint and if the acutal need is greater than this value, the Vec that holds them is
	/// resized. After which the vec is resized back to this value. The default value is 100.
	pub max_events_in: usize,
	/// The maximum events that are returned by the eventhandler per request. The default value
	/// is 100.
	pub max_events: usize,
	/// The frequency at which the housekeeper callback is executed. Each thread executes the
	/// housekeeper at this frequency. The default value is 1,000 ms or 1 second.
	pub housekeeping_frequency_millis: u128,
	/// The number of read slabs for this eventhandler. The default value is 1,000.
	pub read_slab_count: usize,
	/// The maximum number of handles per thread. The default value is 1,000.
	pub max_handles_per_thread: usize,
}

/// This trait defines the behaviour of an eventhandler. See the module level documentation for
/// examples.
pub trait EventHandler<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
where
	OnRead: FnMut(
			&mut ConnectionData,
			&mut ThreadContext,
			Option<AttachmentHolder>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut ConnectionData, &mut ThreadContext) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut ConnectionData, &mut ThreadContext) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	HouseKeeper:
		FnMut(&mut ThreadContext) -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
	OnPanic: FnMut(&mut ThreadContext, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	/// sets the on read callback for this [`crate::EventHandler`]. This callback is exuected
	/// whenever data is read by a connection.
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error>;
	/// sets the on accept callback for this [`crate::EventHandler`]. This callback is executed
	/// whenever a connection is accepted.
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error>;
	/// sets the on close callback for this [`crate::EventHandler`]. This callback is executed
	/// whenever a connection is closed.
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error>;
	/// sets the housekeeper callback for this [`crate::EventHandler`]. This callback is
	/// executed once every [`crate::EventHandlerConfig::housekeeping_frequency_millis`]
	/// milliseconds. Each thread executes this callback independantly.
	fn set_housekeeper(&mut self, housekeeper: HouseKeeper) -> Result<(), Error>;
	/// sets the on panic callback for this [`crate::EventHandler`]. This callback is executed
	/// whenever a user callback generates a thread panic.
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
	/// Stop this [`crate::EventHandler`] and free all resources associated with it.
	fn stop(&mut self) -> Result<(), Error>;
	/// Start the [`crate::EventHandler`].
	fn start(&mut self) -> Result<(), Error>;
	/// Add a [`crate::ClientConnection`] to this [`crate::EventHandler`].
	fn add_client(
		&mut self,
		connection: ClientConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<WriteHandle, Error>;
	/// Add a [`crate::ServerConnection`] to this [`crate::EventHandler`].
	fn add_server(
		&mut self,
		connection: ServerConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<(), Error>;
	/// Get the eventhandler data array
	fn event_handler_data(&self) -> Result<Array<Box<dyn LockBox<EventHandlerData>>>, Error>;
	/// Return an instance of a [`crate::EventHandlerController`] that is associated with this
	/// [`crate::EventHandler``].
	fn event_handler_controller(&self) -> Result<EventHandlerController, Error>;
}

/// Since [`crate::EventHandler'] is returned as an impl trait, it is not possible to store it in
/// data structures. This struct can be used to do some operations after the
/// [`crate::EventHandler`] has gone out of scope.
#[derive(Clone)]
pub struct EventHandlerController {
	pub(crate) data: Array<Box<dyn LockBox<EventHandlerData>>>,
	pub(crate) thread_pool_stopper: Box<dyn LockBox<Option<ThreadPoolStopper>>>,
}

/// The structure that builds eventhandlers.
pub struct Builder {}

#[derive(Debug, Clone)]
pub struct AttachmentHolder {
	pub attachment: Box<dyn LockBox<Box<dyn Any + Send + Sync>>>,
}

// pub(crate) types

#[derive(Clone, Debug)]
pub(crate) enum LastProcessType {
	OnRead,
	OnClose,
	OnAccept,
	Housekeeper,
	OnAcceptOutOfBand,
}

#[derive(Clone)]
pub(crate) struct EventHandlerContext {
	pub(crate) debug_bypass_acc_err: bool,
	pub(crate) debug_trigger_on_read: bool,
	pub(crate) events: Array<Event>,
	pub(crate) events_in: Vec<EventIn>,
	pub(crate) tid: usize,
	#[cfg(target_os = "linux")]
	pub(crate) filter_set: BitVec,
	#[cfg(target_os = "windows")]
	pub(crate) filter_set: BitVec,
	#[cfg(target_os = "linux")]
	pub(crate) epoll_events: Vec<EpollEvent>,
	#[cfg(target_os = "linux")]
	pub(crate) selector: Arc<Epoll>,
	#[cfg(target_os = "macos")]
	pub(crate) selector: Handle,
	#[cfg(target_os = "windows")]
	pub(crate) selector: Handle,
	pub(crate) now: u128,
	pub(crate) last_housekeeper: u128,
	pub(crate) connection_hashtable: Box<dyn Hashtable<u128, ConnectionInfo> + Send + Sync>,
	pub(crate) handle_hashtable: Box<dyn Hashtable<Handle, u128> + Send + Sync>,
	pub(crate) read_slabs: Box<dyn SlabAllocator + Send + Sync>,
	#[cfg(target_os = "windows")]
	pub(crate) write_set: Box<dyn Hashset<Handle> + Send + Sync>,
	pub(crate) counter: usize,
	pub(crate) count: usize,
	pub(crate) last_process_type: LastProcessType,
	pub(crate) last_rw: Option<StreamInfo>,
	pub(crate) last_handle_oob: Handle,
	pub(crate) buffer: Vec<u8>,
	pub(crate) do_write_back: bool,
	pub(crate) attachments: HashMap<u128, AttachmentHolder>,
}

#[derive(Clone)]
pub(crate) struct EventHandlerImpl<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
where
	OnRead: FnMut(
			&mut ConnectionData,
			&mut ThreadContext,
			Option<AttachmentHolder>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut ConnectionData, &mut ThreadContext) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut ConnectionData, &mut ThreadContext) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	HouseKeeper:
		FnMut(&mut ThreadContext) -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
	OnPanic: FnMut(&mut ThreadContext, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	pub(crate) on_read: Option<Pin<Box<OnRead>>>,
	pub(crate) on_accept: Option<Pin<Box<OnAccept>>>,
	pub(crate) on_close: Option<Pin<Box<OnClose>>>,
	pub(crate) on_panic: Option<Pin<Box<OnPanic>>>,
	pub(crate) housekeeper: Option<Pin<Box<HouseKeeper>>>,
	pub(crate) config: EventHandlerConfig,
	pub(crate) data: Array<Box<dyn LockBox<EventHandlerData>>>,
	pub(crate) thread_pool_stopper: Box<dyn LockBox<Option<ThreadPoolStopper>>>,
	pub(crate) debug_write_queue: bool,
	pub(crate) debug_pending: bool,
	pub(crate) debug_write_error: bool,
	pub(crate) debug_suspended: bool,
	pub(crate) debug_fatal_error: bool,
	pub(crate) debug_tls_server_error: bool,
	pub(crate) debug_read_error: bool,
	pub(crate) debug_tls_read: bool,
	pub(crate) debug_attachment_none: bool,
	pub(crate) debug_rw_accept_id_none: bool,
	pub(crate) debug_close_handle: bool,
}

#[derive(Clone)]
pub(crate) struct Wakeup {
	pub(crate) _tcp_stream: Option<Arc<TcpStream>>,
	pub(crate) _tcp_listener: Option<Arc<TcpStream>>,
	pub(crate) reader: Handle,
	pub(crate) writer: Handle,
	pub(crate) requested: Box<dyn LockBox<bool>>,
	pub(crate) needed: Box<dyn LockBox<bool>>,
}

#[derive(Clone, Debug)]
pub(crate) enum ConnectionInfo {
	ListenerInfo(ListenerInfo),
	StreamInfo(StreamInfo),
}

unsafe impl Send for ConnectionInfo {}
unsafe impl Sync for ConnectionInfo {}

#[derive(Clone)]
pub(crate) struct ListenerInfo {
	pub(crate) id: u128,
	pub(crate) handle: Handle,
	pub(crate) is_reuse_port: bool,
	pub(crate) tls_config: Option<Arc<ServerConfig>>,
	pub(crate) tx: Option<SyncSender<()>>,
	pub(crate) ready: Box<dyn LockBox<bool>>,
}

#[derive(Clone, Debug)]
pub(crate) struct StreamInfo {
	pub(crate) id: u128,
	pub(crate) handle: Handle,
	pub(crate) accept_handle: Option<Handle>,
	pub(crate) accept_id: Option<u128>,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) first_slab: u32,
	pub(crate) last_slab: u32,
	pub(crate) slab_offset: u16,
	pub(crate) is_accepted: bool,
	pub(crate) tls_server: Option<Box<dyn LockBox<RustlsServerConnection>>>,
	pub(crate) tls_client: Option<Box<dyn LockBox<RustlsClientConnection>>>,
	pub(crate) tx: Option<SyncSender<()>>,
}

#[derive(Clone, Debug, Serializable)]
pub struct WriteState {
	pub(crate) write_buffer: Vec<u8>,
	pub(crate) flags: u8,
}

#[derive(Clone)]
pub struct EventHandlerData {
	pub(crate) write_queue: Box<dyn Queue<u128> + Send + Sync>,
	pub(crate) nhandles: Box<dyn Queue<ConnectionInfo> + Send + Sync>,
	pub(crate) stop: bool,
	pub(crate) stopped: bool,
	pub(crate) attachments: HashMap<u128, AttachmentHolder>,
	pub(crate) wakeup: Wakeup,
	pub(crate) debug_write_queue: bool,
	pub(crate) debug_pending: bool,
	pub(crate) debug_write_error: bool,
	pub(crate) debug_suspended: bool,
}

#[cfg(unix)]
pub type Handle = RawFd;
#[cfg(windows)]
pub type Handle = usize;

#[derive(Debug, Clone, Serializable, PartialEq)]
pub(crate) enum EventType {
	Read,
	Write,
}

#[derive(Debug, Clone, Serializable, PartialEq)]
pub(crate) enum EventTypeIn {
	Accept,
	Read,
	Write,
	Suspend,
	Resume,
}

#[derive(Debug, Clone, Serializable, PartialEq)]
pub(crate) struct Event {
	pub(crate) handle: Handle,
	pub(crate) etype: EventType,
}

#[derive(Debug, Clone, Serializable, PartialEq)]
pub(crate) struct EventIn {
	pub(crate) handle: Handle,
	pub(crate) etype: EventTypeIn,
}
