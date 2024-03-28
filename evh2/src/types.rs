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
	fn start(&mut self) -> Result<(), Error>;
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error>;
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error>;
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error>;
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error>;
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
	fn add_server_connection(&mut self, connection: Connection) -> Result<(), Error>;
	fn add_client_connection(&mut self, connection: Connection) -> Result<WriteHandle, Error>;
	fn wait_for_stats(&mut self) -> Result<EvhStats, Error>;
}

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
}

pub trait UserContext {
	fn clone_next_chunk(
		&mut self,
		connection: &mut Connection,
		buf: &mut [u8],
	) -> Result<usize, Error>;
	fn cur_slab_id(&self) -> usize;
	fn clear_all(&mut self, connection: &mut Connection) -> Result<(), Error>;
	fn clear_through(&mut self, slab_id: usize, connection: &mut Connection) -> Result<(), Error>;
	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any + Send + Sync>>;
	fn set_user_data(&mut self, user_data: Box<dyn Any + Send + Sync>);
}

pub struct EvhBuilder {}

pub struct WriteHandle {
	pub(crate) handle: Handle,
	pub(crate) id: u128,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) wakeup: Wakeup,
	pub(crate) state: Box<dyn LockBox<EventHandlerState>>,
}

#[derive(Debug, Clone)]
pub struct EvhStats {
	pub accepts: usize,
	pub closes: usize,
	pub reads: usize,
	pub delay_writes: usize,
	pub event_loops: usize,
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

pub(crate) enum ConnectionType {
	Server,
	Client,
	Connection,
}
