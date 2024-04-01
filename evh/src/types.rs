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

#[cfg(target_os = "macos")]
use crate::mac::*;
#[cfg(target_os = "windows")]
use crate::win::*;

#[cfg(target_os = "linux")]
use crate::linux::*;

use crate::constants::*;
use bmw_err::*;
use bmw_util::*;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::mpsc::SyncSender;

/// The [`crate::EventHandler`] trait is implemented by the returned value of the
/// [`crate::EvhBuilder::build_evh`] function.
/// # See also
/// See the [`crate`] documentation as well for the background information and motivation
/// for this crate as well as examples.
pub trait EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	/// Start the [`crate::EventHandler`]. This function must be called before connections may
	/// be added to the event handler.
	/// # Input Parameters
	/// none
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`]
	fn start(&mut self) -> Result<(), Error>;
	/// Set the OnRead handler for this [`crate::EventHandler`]. When data is ready on any
	/// connections that have been added to this event handler, the OnRead callback will be
	/// executed.
	/// # Input Parameters
	/// The OnRead handler to use as a callback for this [`crate::EventHandler`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::UserContext`]
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error>;
	/// Set the OnAccept handler for this [`crate::EventHandler`]. When new connections are
	/// accepted by a server connection, this callback will be executed.
	/// # Input Parameters
	/// The OnAccept handler to use as a callback for this [`crate::EventHandler`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::UserContext`]
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error>;
	/// Set the OnClose handler for this [`crate::EventHandler`]. When connections are
	/// closed, this callback will be executed.
	/// # Input Parameters
	/// The OnClose handler to use as a callback for this [`crate::EventHandler`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::UserContext`]
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error>;
	/// Set the OnHousekeeper handler for this [`crate::EventHandler`]. Periodically,
	/// housekeeping needs to occur. This function allows the user to specify a hook that is
	/// executed periodically. This can be used to close stale connections, log data, or
	/// anything the user wishes to implement.
	/// # Input Parameters
	/// The OnHousekeeper handler to use as a callback for this [`crate::EventHandler`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::UserContext`]
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error>;
	/// Sets the OnPanic handler for this  [`crate::EventHandler`]. This handler is executed
	/// when a thread panic occurs in the [`crate::EventHandler::set_on_read`] handler. The
	/// event handler implementation will tollorate thread panics in the OnRead handler only.
	/// Panics in other handlers result in undefined behavior.
	/// # Input Parameters
	/// The OnPanic handler to use as a callback for this [`crate::EventHandler`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::UserContext`]
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
	/// Add a server connection to this [`crate::EventHandler`].
	/// # Input Parameters
	/// connection - the [`crate::Connection`] to add to this [`crate::EventHandler`] instance.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IllegalArgument`] - If the connection is not a server connection.
	/// [`bmw_err::ErrKind::IO`] - If an i/o error occurs in the [`crate::EventHandler`] while
	/// adding this connection.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::EvhBuilder::build_server_connection`]
	fn add_server_connection(&mut self, connection: Connection) -> Result<(), Error>;
	/// Add a client connection to this [`crate::EventHandler`].
	/// # Input Parameters
	/// connection - the [`crate::Connection`] to add to this [`crate::EventHandler`] instance.
	/// # Returns
	/// On success, [`crate::WriteHandle`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IllegalArgument`] - If the connection is not a client connection.
	/// [`bmw_err::ErrKind::IO`] - If an i/o error occurs in the [`crate::EventHandler`] while
	/// adding this connection.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::EvhBuilder::build_client_connection`]
	fn add_client_connection(&mut self, connection: Connection) -> Result<WriteHandle, Error>;
	/// This function will block until statistical data is ready for this
	/// [`crate::EventHandler`]. The time this function blocks is specified by the
	/// [`bmw_conf::ConfigOption::EvhStatsUpdateMillis`] parameter. It is important to note
	/// that this function may block longer if an event loop does not occur at the time
	/// specified. So, if accurate timing is desired, a lower
	/// [`bmw_conf::ConfigOption::EvhTimeout`] parameter may be used.
	/// # Input Parameters
	/// none
	/// # Returns
	/// On success, [`crate::EvhStats`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] - If an i/o error occurs in the [`crate::EventHandler`] while
	/// waiting for the statistical data.
	/// # See Also
	/// [`crate`], [`crate::EventHandler`], [`crate::EvhStats`]
	fn wait_for_stats(&mut self) -> Result<EvhStats, Error>;
	#[doc(hidden)]
	fn set_debug_info(&mut self, debug_info: DebugInfo) -> Result<(), Error>;
}

/// The [`crate::UserContext`] trait is returned on all callbacks specified by the
/// [`crate::EventHandler`]. It can be used to read data from the underlying connections, clear
/// slabs that are not needed, and get or set a `user_data` structure that can be used as a context
/// variable by the caller. The `user_data` structure is of the type `Box<dyn Any + Send + Sync>`
/// so the user may use this for practically anything. This value stays consistent accross all
/// callbacks for the thread that is invoked on. Each thread of the [`crate::EventHandler`] does
/// store and return a distinct value.
/// See the [`crate`] documentation as well for the background information and motivation
/// for this crate as well as examples. See [`crate::EventHandler`] for the callback functions.
pub trait UserContext {
	/// Clone the next chunk of data, if available, that has been read by the
	/// [`crate::EventHandler`] for this [`crate::Connection`].
	/// # Input Parameters
	/// connection - the [`crate::Connection`] to get the next chunk from.
	/// buf - the mutable slice to clone the data into.
	/// # Returns
	/// On success, length in bytes, of data cloned is returned and on failure,
	/// [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IllegalArgument`] - If the buffer is too small to store the
	/// chunk's data. The maximum size returned is the value configured using
	/// [`bmw_conf::ConfigOption::EvhReadSlabSize`] parameter.
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::clear_all`]
	fn clone_next_chunk(
		&mut self,
		connection: &mut Connection,
		buf: &mut [u8],
	) -> Result<usize, Error>;
	/// Retrieve the `slab_id` of the current slab being processed by this
	/// [`crate::Connection`].
	/// # Input Parameters
	/// none
	/// # Returns
	/// The `slab_id` of the slab that is currently pointed to by this [`crate::UserContext`].
	/// If there are no more slabs associated with this [`crate::Connection`], a number equal
	/// to or greater than [`u32::MAX`] is returned.
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::clear_through`]
	fn cur_slab_id(&self) -> usize;
	/// Clear all slabs that are associated with this [`crate::Connection`].
	/// # Input Parameters
	/// connection - The [`crate::Connection`] to clear data from.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::clear_through`],
	/// [`crate::UserContext::cur_slab_id`]
	fn clear_all(&mut self, connection: &mut Connection) -> Result<(), Error>;
	/// Clear all slabs, through the `slab_id` specified, that are associated with this
	/// [`crate::Connection`].
	/// # Input Parameters
	/// slab_id - The `slab_id` of the slab to clear data through. This value can be obtained
	/// by calling the [`crate::UserContext::cur_slab_id`] as the
	/// [`crate::UserContext::clone_next_chunk`] function is called.
	/// connection - The [`crate::Connection`] to clear data from.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::clear_all`],
	/// [`crate::UserContext::cur_slab_id`]
	fn clear_through(&mut self, slab_id: usize, connection: &mut Connection) -> Result<(), Error>;
	/// Get the `user_data` object associated with this thread.
	/// # Input Parameters
	/// none
	/// # Returns
	/// The `user_data` which has been specified on a previous call to
	/// [`crate::UserContext::set_user_data`].
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::set_user_data`]
	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any + Send + Sync>>;
	/// Set the `user_data` object associated with this thread.
	/// # Input Parameters
	/// user_data - The value to set for this thread's context data.
	/// # Returns
	/// none
	/// # See Also
	/// [`crate`], [`crate::UserContext`], [`crate::UserContext::get_user_data`]
	fn set_user_data(&mut self, user_data: Box<dyn Any + Send + Sync>);
}

/// The [`crate::Connection`] struct represents a connection. It may be either a server side
/// connection or a client side connection. To create a server side connection, see
/// [`crate::EvhBuilder::build_server_connection`]. To create a client side connection, see
/// [`crate::EvhBuilder::build_client_connection`]. These connections can then be added to a
/// [`crate::EventHandler`] via the [`crate::EventHandler::add_server_connection`] and
/// [`crate::EventHandler::add_client_connection`] functions respectively.
pub struct Connection {
	pub(crate) handle: Handle,
	pub(crate) id: u128,
	pub(crate) slab_offset: usize,
	pub(crate) first_slab: usize,
	pub(crate) last_slab: usize,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) wakeup: Option<Wakeup>,
	pub(crate) state: Option<Box<dyn LockBox<EventHandlerState>>>,
	pub(crate) tx: Option<SyncSender<()>>,
	pub(crate) ctype: ConnectionType,
	pub(crate) debug_info: DebugInfo,
}

/// Builder struct for the crate. All implementations are created through this struct.
pub struct EvhBuilder {}

/// The [`crate::WriteHandle`] struct may be used to write data to the underlying connection. Since
/// [`std::clone::Clone`] is implmeneted, the WriteHandle may be cloned and passed to other
/// threads. This allows for asynchronous writes for applications like Websockets.
#[derive(Clone)]
pub struct WriteHandle {
	pub(crate) handle: Handle,
	pub(crate) id: u128,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) wakeup: Wakeup,
	pub(crate) state: Box<dyn LockBox<EventHandlerState>>,
	pub(crate) debug_info: DebugInfo,
}

/// Statistical information for the [`crate::EventHandler`]. This struct may be retrieved by
/// calling the [`crate::EventHandler::wait_for_stats`] function.
#[derive(Debug, Clone)]
pub struct EvhStats {
	/// The  number of connections accepted by the [`crate::EventHandler`] in the last statistical
	/// interval. See [`crate::EventHandler::wait_for_stats`].
	pub accepts: usize,
	/// The  number of connections closed by the [`crate::EventHandler`] in the last statistical
	/// interval. See [`crate::EventHandler::wait_for_stats`].
	pub closes: usize,
	/// The  number of reads completed by the [`crate::EventHandler`] in the last statistical
	/// interval. See [`crate::EventHandler::wait_for_stats`].
	pub reads: usize,
	/// The number of delayed writes completed by the [`crate::EventHandler`] in the last
	/// statistical interval. See [`crate::EventHandler::wait_for_stats`]. A delay write is a
	/// write that cannot complete at the time [`crate::WriteHandle::write`] is called. In the
	/// event that the operating system cannot complete the write of data fully, the
	/// [`crate::EventHandler`] will queue the data and do a `delay_write`.
	pub delay_writes: usize,
	/// The  number of event loops that occured by the [`crate::EventHandler`] in the last
	/// statistical interval. See [`crate::EventHandler::wait_for_stats`]. This is the number
	/// of times the event handler has gone through it's main loop. This can be controlled
	/// by specifying the [`bmw_conf::ConfigOption::EvhTimeout`] parameter on instantiation as
	/// that will affect the number of times the loop occurs. However, a loop will always
	/// occur if the server needs to read/write. So, the number will be higher for servers
	/// with a lot of I/O happening.
	pub event_loops: usize,
	pub bytes_read: u128,
	pub bytes_delay_write: u128,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct DebugInfo {
	pub(crate) pending: Box<dyn LockBox<bool>>,
	pub(crate) write_err: Box<dyn LockBox<bool>>,
	pub(crate) write_err2: Box<dyn LockBox<bool>>,
	pub(crate) read_err: Box<dyn LockBox<bool>>,
	pub(crate) wakeup_read_err: Box<dyn LockBox<bool>>,
	pub(crate) write_handle_err: Box<dyn LockBox<bool>>,
	pub(crate) stop_error: Box<dyn LockBox<bool>>,
	pub(crate) panic_fatal_error: Box<dyn LockBox<bool>>,
	pub(crate) normal_fatal_error: Box<dyn LockBox<bool>>,
	pub(crate) internal_panic: Box<dyn LockBox<bool>>,
	pub(crate) get_events_error: Box<dyn LockBox<bool>>,
	pub(crate) os_error: Box<dyn LockBox<bool>>,
}

// crate local structures

pub(crate) struct EventHandlerState {
	pub(crate) nconnections: VecDeque<ConnectionVariant>,
	pub(crate) write_queue: VecDeque<u128>,
	pub(crate) stop: bool,
}

#[derive(Clone)]
pub(crate) struct Wakeup {
	pub(crate) id: u128,
	pub(crate) reader: Handle,
	pub(crate) writer: Handle,
	pub(crate) requested: Box<dyn LockBox<bool>>,
	pub(crate) needed: Box<dyn LockBox<bool>>,
}

pub(crate) struct WriteState {
	pub(crate) flags: u8,
	pub(crate) write_buffer: Vec<u8>,
}

pub(crate) struct GlobalStats {
	pub(crate) stats: EvhStats,
	pub(crate) update_counter: usize,
	pub(crate) tx: Option<SyncSender<()>>,
}

pub(crate) struct UserContextImpl {
	pub(crate) read_slabs: Box<dyn SlabAllocator + Send + Sync>,
	pub(crate) user_data: Option<Box<dyn Any + Send + Sync>>,
	pub(crate) slab_cur: usize,
}

#[derive(Clone)]
pub(crate) struct EventHandlerConfig {
	pub(crate) threads: usize,
	pub(crate) debug: bool,
	pub(crate) timeout: u16,
	pub(crate) read_slab_size: usize,
	pub(crate) read_slab_count: usize,
	pub(crate) housekeeping_frequency_millis: usize,
	pub(crate) stats_update_frequency_millis: usize,
}
pub(crate) struct EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	pub(crate) callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	pub(crate) config: EventHandlerConfig,
	pub(crate) state: Array<Box<dyn LockBox<EventHandlerState>>>,
	pub(crate) wakeups: Array<Wakeup>,
	pub(crate) stopper: Option<ThreadPoolStopper>,
	pub(crate) stats: Box<dyn LockBox<GlobalStats>>,
	pub(crate) debug_info: DebugInfo,
}

#[derive(Clone)]
pub(crate) struct EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
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
	pub(crate) on_housekeeper: Option<Pin<Box<OnHousekeeper>>>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum EventType {
	Read,
	Write,
	ReadWrite,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Event {
	pub(crate) handle: Handle,
	pub(crate) etype: EventType,
}

#[derive(PartialEq)]
pub(crate) enum EventTypeIn {
	Read,
	Write,
}

pub(crate) struct EventIn {
	pub(crate) handle: Handle,
	pub(crate) etype: EventTypeIn,
}

pub(crate) struct EventHandlerContext {
	pub(crate) ret_event_count: usize,
	pub(crate) ret_events: [Event; MAX_RET_HANDLES],
	pub(crate) ret_event_itt: usize,
	pub(crate) in_events: Vec<EventIn>,
	pub(crate) handle_hash: HashMap<Handle, u128>,
	pub(crate) id_hash: HashMap<u128, ConnectionVariant>,
	pub(crate) wakeups: Array<Wakeup>,
	pub(crate) tid: usize,
	pub(crate) last_housekeeping: usize,
	pub(crate) trigger_on_read_list: Vec<Handle>,
	pub(crate) trigger_itt: usize,
	pub(crate) thread_stats: EvhStats,
	pub(crate) global_stats: Box<dyn LockBox<GlobalStats>>,
	pub(crate) last_stats_update: usize,

	#[cfg(target_os = "linux")]
	pub(crate) linux_ctx: LinuxContext,
	#[cfg(target_os = "macos")]
	pub(crate) macos_ctx: MacosContext,
	#[cfg(target_os = "windows")]
	pub(crate) windows_ctx: WindowsContext,
}

pub(crate) enum ConnectionVariant {
	ServerConnection(Connection),
	ClientConnection(Connection),
	Connection(Connection),
	Wakeup(Wakeup),
}

#[derive(PartialEq)]
pub(crate) enum ConnectionType {
	Server,
	Client,
	Connection,
}
