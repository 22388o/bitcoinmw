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

use crate::types::{
	AttachmentHolder, CloseHandle, ConnectionInfo, Event, EventHandlerContext, EventHandlerData,
	EventHandlerImpl, EventIn, EventType, EventTypeIn, Handle, LastProcessType, ListenerInfo,
	StreamInfo, Wakeup, WriteState,
};
use crate::{
	ClientConnection, ConnData, ConnectionData, EventHandler, EventHandlerConfig,
	EventHandlerStats, ServerConnection, ThreadContext, WriteHandle,
};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::rand::random;
use bmw_deps::rustls::{
	Certificate, ClientConfig, ClientConnection as RCConn, OwnedTrustAnchor, PrivateKey,
	RootCertStore, ServerConfig, ServerConnection as RSConn,
};
use bmw_deps::rustls_pemfile::{certs, read_one, Item};
use bmw_deps::webpki_roots::TLS_SERVER_ROOTS;
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::any::type_name;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

#[cfg(target_os = "linux")]
use crate::linux::*;
#[cfg(target_os = "macos")]
use crate::mac::*;
#[cfg(windows)]
use crate::win::*;

#[cfg(target_os = "windows")]
use bmw_deps::bitvec::vec::BitVec;
#[cfg(target_os = "windows")]
use bmw_deps::wepoll_sys::{epoll_create, EPOLLIN, EPOLLONESHOT, EPOLLOUT, EPOLLRDHUP};
#[cfg(target_os = "windows")]
use std::os::raw::c_void;

#[cfg(target_os = "macos")]
use bmw_deps::kqueue_sys::{kevent, kqueue, EventFilter, EventFlag, FilterFlag};

#[cfg(target_os = "linux")]
use bmw_deps::bitvec::vec::BitVec;
#[cfg(target_os = "linux")]
use bmw_deps::nix::sys::epoll::{epoll_create1, EpollCreateFlags, EpollEvent, EpollFlags};

#[cfg(unix)]
use bmw_deps::libc::{fcntl, F_SETFL, O_NONBLOCK};

#[cfg(unix)]
use std::os::unix::io::{FromRawFd, IntoRawFd};
#[cfg(windows)]
use std::os::windows::io::{FromRawSocket, IntoRawSocket};

/// The size of the data which is stored in read slabs. This data is followed by 4 bytes which is a
/// pointer to the next slab in the list.
pub const READ_SLAB_DATA_SIZE: usize = 514;

const READ_SLAB_SIZE: usize = 518;
const READ_SLAB_NEXT_OFFSET: usize = 514;

const HANDLE_SLAB_SIZE: usize = 42;
const CONNECTION_SLAB_SIZE: usize = 98;
#[cfg(target_os = "windows")]
const WRITE_SET_SIZE: usize = 42;

const WRITE_STATE_FLAG_PENDING: u8 = 0x1 << 0;
const WRITE_STATE_FLAG_CLOSE: u8 = 0x1 << 1;
const WRITE_STATE_FLAG_TRIGGER_ON_READ: u8 = 0x1 << 2;
const WRITE_STATE_FLAG_SUSPEND: u8 = 0x1 << 3;
const WRITE_STATE_FLAG_RESUME: u8 = 0x1 << 4;

const EAGAIN: i32 = 11;
const ETEMPUNAVAILABLE: i32 = 35;
const WINNONBLOCKING: i32 = 10035;

const TLS_CHUNKS: usize = 5_120;

info!();

/// Create listeners for use with the [`crate::ServerConnection`] struct.
/// This function crates an array of handles which can be used to construct a [`crate::ServerConnection`]
/// object. `size` is the size of the array. It must be equal to the number of threads that the
/// [`crate::EventHandler`] has configured. `addr` is the socketaddress to bind to. (For example:
/// 127.0.0.1:80 or 0.0.0.0:443.). `listen_size` is the size of the listener backlog for this
/// tcp/ip connection. `reuse_port` specifies whether or not to reuse the port on a per thread
/// basis for this connection. This is only available on linux and will be ignored on other
/// platforms.
pub fn create_listeners(
	size: usize,
	addr: &str,
	listen_size: usize,
	reuse_port: bool,
) -> Result<Array<Handle>, Error> {
	create_listeners_impl(size, addr, listen_size, reuse_port)
}

impl Default for Event {
	fn default() -> Self {
		Self {
			handle: 0,
			etype: EventType::Read,
		}
	}
}

impl Default for EventHandlerConfig {
	fn default() -> Self {
		Self {
			threads: 6,
			sync_channel_size: 10,
			write_queue_size: 100_000,
			nhandles_queue_size: 1_000,
			max_events_in: 1_000,
			max_events: 100,
			housekeeping_frequency_millis: 1_000,
			read_slab_count: 1_000,
			max_handles_per_thread: 1_000,
		}
	}
}

impl Debug for ListenerInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(
			f,
			"ListenerInfo[id={},handle={},is_reuse_port={},has_tls_config={}]",
			self.id,
			self.handle,
			self.is_reuse_port,
			self.tls_config.is_some()
		)
	}
}

// Note about serialization of ConnectionInfo: We serialize the write state
// which is a LockBox. So we use the danger_to_usize fn. It must be deserialized
// once per serialization or it will leak and cause other memory related problems.
// The same applies to the TLS data structures.

impl Serializable for ConnectionInfo {
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn read<R>(reader: &mut R) -> Result<Self, Error>
	where
		R: Reader,
	{
		let r = reader.read_u8()?;
		if r == 0 {
			let id = reader.read_u128()?;
			debug!("listener deser for id = {}", id)?;
			let handle = Handle::read(reader)?;
			let is_reuse_port = match reader.read_u8()? {
				0 => false,
				_ => true,
			};
			let r = reader.read_u8()?;
			let tls_config = if r == 0 {
				None
			} else {
				let r = reader.read_usize()? as *mut ServerConfig;
				let tls_config: Arc<ServerConfig> = unsafe { Arc::from_raw(r) };
				Some(tls_config)
			};
			let li = ListenerInfo {
				id,
				handle,
				is_reuse_port,
				tls_config,
			};
			let ci = ConnectionInfo::ListenerInfo(li);
			Ok(ci)
		} else if r == 1 {
			let id = reader.read_u128()?;
			debug!("deserrw for id = {}", id)?;
			let handle = Handle::read(reader)?;
			let accept_handle: Option<Handle> = Option::read(reader)?;
			let accept_id: Option<u128> = Option::read(reader)?;
			let v = reader.read_usize()?;
			let write_state: Box<dyn LockBox<WriteState>> = lock_box_from_usize(v);
			let first_slab = reader.read_u32()?;
			let last_slab = reader.read_u32()?;
			let slab_offset = reader.read_u16()?;
			let is_accepted = reader.read_u8()? != 0;
			let r = reader.read_u8()?;
			let tls_server = if r == 0 {
				None
			} else {
				let v = reader.read_usize()?;
				let tls_server: Box<dyn LockBox<RSConn>> = lock_box_from_usize(v);
				Some(tls_server)
			};
			let r = reader.read_u8()?;
			let tls_client = if r == 0 {
				None
			} else {
				let v = reader.read_usize()?;
				let tls_client: Box<dyn LockBox<RCConn>> = lock_box_from_usize(v);
				Some(tls_client)
			};
			let rwi = StreamInfo {
				id,
				handle,
				accept_handle,
				accept_id,
				write_state,
				first_slab,
				last_slab,
				slab_offset,
				is_accepted,
				tls_client,
				tls_server,
			};
			Ok(ConnectionInfo::StreamInfo(rwi))
		} else {
			let err = err!(ErrKind::CorruptedData, "Unexpected type in ConnectionInfo");
			Err(err)
		}
	}
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn write<W>(&self, writer: &mut W) -> Result<(), Error>
	where
		W: Writer,
	{
		match self {
			ConnectionInfo::ListenerInfo(li) => {
				debug!("listener ser for id = {}", li.id)?;
				writer.write_u8(0)?;
				writer.write_u128(li.id)?;
				li.handle.write(writer)?;
				writer.write_u8(match li.is_reuse_port {
					true => 1,
					false => 0,
				})?;
				match &li.tls_config {
					Some(tls_config) => {
						let ptr = Arc::into_raw(tls_config.clone());
						let ptr_as_usize = ptr as usize;
						writer.write_u8(1)?;
						writer.write_usize(ptr_as_usize)?;
					}
					None => {
						writer.write_u8(0)?;
					}
				}
			}
			ConnectionInfo::StreamInfo(ri) => {
				debug!("serrw for id={}", ri.id)?;
				writer.write_u8(1)?;
				writer.write_u128(ri.id)?;
				ri.handle.write(writer)?;
				ri.accept_handle.write(writer)?;
				ri.accept_id.write(writer)?;
				writer.write_usize(ri.write_state.danger_to_usize())?;
				writer.write_u32(ri.first_slab)?;
				writer.write_u32(ri.last_slab)?;
				writer.write_u16(ri.slab_offset)?;
				writer.write_u8(match ri.is_accepted {
					true => 1,
					false => 0,
				})?;
				match &ri.tls_server {
					Some(tls_server) => {
						writer.write_u8(1)?;
						writer.write_usize(tls_server.danger_to_usize())?;
					}
					None => writer.write_u8(0)?,
				}
				match &ri.tls_client {
					Some(tls_client) => {
						writer.write_u8(1)?;
						writer.write_usize(tls_client.danger_to_usize())?;
					}
					None => writer.write_u8(0)?,
				}
			}
		}

		Ok(())
	}
}

impl EventHandlerStats {
	pub fn new() -> Self {
		Self {
			on_read_count: 0,
			on_accept_count: 0,
			on_close_count: 0,
			on_panic_count: 0,
			on_read_lat_sum: 0,
			on_accept_lat_sum: 0,
			on_close_lat_sum: 0,
			on_panic_lat_sum: 0,
		}
	}
}

impl StreamInfo {
	fn clear_through_impl(
		&mut self,
		slab_id: u32,
		slabs: &mut Box<dyn SlabAllocator + Send + Sync>,
	) -> Result<(), Error> {
		let mut next = self.first_slab;

		debug!("clear through impl with slab_id = {}", slab_id)?;
		loop {
			if next == u32::MAX {
				break;
			}

			let n = next.try_into()?;
			let next_slab = try_into!(&slabs.get(n)?.get()[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE])?;
			let next_slab: u32 = u32::from_be_bytes(next_slab);
			slabs.free(next.try_into()?)?;
			debug!("free {}", next)?;

			if next == slab_id {
				if slab_id == self.last_slab {
					self.first_slab = u32::MAX;
					self.last_slab = u32::MAX;
					self.slab_offset = 0;
				} else {
					self.first_slab = next_slab;
				}
				break;
			}
			next = next_slab;
		}

		Ok(())
	}
}

impl ThreadContext {
	fn new() -> Self {
		Self {
			user_data: Box::new(0),
		}
	}
}

impl EventHandlerContext {
	pub(crate) fn new(
		tid: usize,
		max_events_in: usize,
		max_events: usize,
		max_handles_per_thread: usize,
		read_slab_count: usize,
	) -> Result<Self, Error> {
		let events = array!(max_events * 2, &Event::default())?;
		let events_in = Vec::with_capacity(max_events_in);

		#[cfg(target_os = "linux")]
		let mut filter_set: BitVec = BitVec::with_capacity(max_handles_per_thread + 100);
		#[cfg(target_os = "linux")]
		filter_set.resize(max_handles_per_thread + 100, false);

		#[cfg(target_os = "windows")]
		let mut filter_set: BitVec = BitVec::with_capacity(max_handles_per_thread + 100);
		#[cfg(target_os = "windows")]
		filter_set.resize(max_handles_per_thread + 100, false);

		#[cfg(target_os = "windows")]
		for i in 0..(max_handles_per_thread + 100) {
			filter_set.set(i, false);
		}

		#[cfg(target_os = "linux")]
		for i in 0..(max_handles_per_thread + 100) {
			filter_set.set(i, false);
		}

		let handle_hashtable = hashtable_sync_box!(
			SlabSize(HANDLE_SLAB_SIZE),
			SlabCount(max_handles_per_thread)
		)?;
		let connection_hashtable = hashtable_sync_box!(
			SlabSize(CONNECTION_SLAB_SIZE),
			SlabCount(max_handles_per_thread)
		)?;

		#[cfg(target_os = "windows")]
		let write_set = hashset_sync_box!(SlabSize(WRITE_SET_SIZE), SlabCount(2 * MAX_EVENTS))?;

		#[cfg(target_os = "linux")]
		let epoll_events = {
			let mut epoll_events = vec![];
			epoll_events.resize(max_events * 2, EpollEvent::new(EpollFlags::empty(), 0));
			epoll_events
		};

		let mut read_slabs = Builder::build_sync_slabs();
		read_slabs.init(SlabAllocatorConfig {
			slab_size: READ_SLAB_SIZE,
			slab_count: read_slab_count,
		})?;

		Ok(EventHandlerContext {
			debug_bypass_acc_err: false,
			debug_trigger_on_read: false,
			connection_hashtable,
			handle_hashtable,
			read_slabs,
			events,
			events_in,
			tid,
			now: 0,
			last_housekeeper: 0,
			counter: 0,
			count: 0,
			last_process_type: LastProcessType::OnRead,
			last_rw: None,
			last_handle_oob: 0,
			#[cfg(target_os = "linux")]
			filter_set,
			#[cfg(target_os = "windows")]
			filter_set,
			#[cfg(target_os = "windows")]
			write_set,
			#[cfg(target_os = "macos")]
			selector: unsafe { kqueue() },
			#[cfg(target_os = "linux")]
			selector: epoll_create1(EpollCreateFlags::empty())?,
			#[cfg(target_os = "linux")]
			epoll_events,
			#[cfg(windows)]
			selector: unsafe { epoll_create(1) } as usize,
			buffer: vec![],
			do_write_back: true,
			attachments: HashMap::new(),
		})
	}
}

impl WriteState {
	fn set_flag(&mut self, flag: u8) {
		self.flags |= flag;
	}

	fn unset_flag(&mut self, flag: u8) {
		self.flags &= !flag;
	}

	fn is_set(&self, flag: u8) -> bool {
		self.flags & flag != 0
	}
}

impl WriteHandle {
	fn new(
		handle: Handle,
		id: u128,
		wakeup: Wakeup,
		write_state: Box<dyn LockBox<WriteState>>,
		event_handler_data: Box<dyn LockBox<EventHandlerData>>,
		debug_write_queue: bool,
		debug_pending: bool,
		debug_write_error: bool,
		debug_suspended: bool,
		tls_server: Option<Box<dyn LockBox<RSConn>>>,
		tls_client: Option<Box<dyn LockBox<RCConn>>>,
	) -> Self {
		Self {
			handle,
			id,
			wakeup,
			write_state,
			event_handler_data,
			debug_write_queue,
			debug_pending,
			tls_client,
			tls_server,
			debug_write_error,
			debug_suspended,
		}
	}

	/// Return the write state for this write_handle.
	pub fn write_state(&self) -> Result<Box<dyn LockBox<WriteState>>, Error> {
		Ok(self.write_state.clone())
	}

	/// Suspend any reads/writes in the [`crate::EventHandler`] for the connection associated
	/// with this [`crate::WriteHandle`]. This can be used to transfer large amounts of data in
	/// a separate thread while suspending reads/writes in the evh.
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub fn suspend(&mut self) -> Result<(), Error> {
		{
			debug!("wlock for {}", self.id)?;
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard();
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				// it's already closed no need to do anything
				return Ok(());
			}
			(**guard).unset_flag(WRITE_STATE_FLAG_RESUME);
			(**guard).set_flag(WRITE_STATE_FLAG_SUSPEND);
			debug!("unlockwlock for {}", self.id)?;
		}
		{
			let mut event_handler_data = self.event_handler_data.wlock()?;
			let guard = event_handler_data.guard();
			(**guard).write_queue.enqueue(self.id)?;
		}
		self.wakeup.wakeup()?;
		Ok(())
	}

	/// Resume reads/writes in the [`crate::EventHandler`]. This must be called after
	/// [`crate::WriteHandle::suspend`].
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub fn resume(&mut self) -> Result<(), Error> {
		{
			debug!("wlock for {}", self.id)?;
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard();
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				// it's already closed no need to do anything
				return Ok(());
			}
			(**guard).set_flag(WRITE_STATE_FLAG_RESUME);
			(**guard).unset_flag(WRITE_STATE_FLAG_SUSPEND);
			debug!("unlockwlock for {}", self.id)?;
		}
		{
			let mut event_handler_data = self.event_handler_data.wlock()?;
			let guard = event_handler_data.guard();
			(**guard).write_queue.enqueue(self.id)?;
		}
		self.wakeup.wakeup()?;
		Ok(())
	}

	/// Close the connection associated with this [`crate::WriteHandle`].
	pub fn close(&mut self) -> Result<(), Error> {
		handle_close(&mut self.write_state, self.id, &mut self.event_handler_data)
	}

	/// Write data to the connection associated with this [`crate::WriteHandle`].
	pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
		match &mut self.tls_client.clone() {
			Some(ref mut tls_conn) => {
				let mut tls_conn = tls_conn.wlock()?;
				let tls_conn = tls_conn.guard();
				let mut start = 0;
				loop {
					let mut wbuf = vec![];
					let mut end = data.len();
					if end.saturating_sub(start) > TLS_CHUNKS {
						end = start + TLS_CHUNKS;
					}
					(**tls_conn).writer().write_all(&data[start..end])?;
					(**tls_conn).write_tls(&mut wbuf)?;
					self.do_write(&wbuf)?;

					if end == data.len() {
						break;
					}
					start += TLS_CHUNKS;
				}
				Ok(())
			}
			None => match &mut self.tls_server.clone() {
				Some(ref mut tls_conn) => {
					let mut tls_conn = tls_conn.wlock()?;
					let tls_conn = tls_conn.guard();
					let mut start = 0;
					loop {
						let mut wbuf = vec![];
						let mut end = data.len();
						if end.saturating_sub(start) > TLS_CHUNKS {
							end = start + TLS_CHUNKS;
						}
						(**tls_conn).writer().write_all(&data[start..end])?;
						(**tls_conn).write_tls(&mut wbuf)?;
						self.do_write(&wbuf)?;

						if end == data.len() {
							break;
						}
						start += TLS_CHUNKS;
					}
					Ok(())
				}
				None => self.do_write(data),
			},
		}
	}

	fn do_write(&mut self, data: &[u8]) -> Result<(), Error> {
		let data_len = data.len();
		let len = {
			let write_state = self.write_state.rlock()?;
			if (**write_state.guard()).is_set(WRITE_STATE_FLAG_CLOSE) {
				return Err(err!(ErrKind::IO, "write handle closed"));
			}
			if (**write_state.guard()).is_set(WRITE_STATE_FLAG_SUSPEND) || self.debug_suspended {
				return Err(err!(ErrKind::IO, "write handle suspended"));
			}
			if (**write_state.guard()).is_set(WRITE_STATE_FLAG_PENDING)
				|| self.debug_pending
				|| self.debug_write_error
			{
				0
			} else {
				if self.debug_write_queue {
					if data[0] == 'a' as u8 {
						write_bytes(self.handle, &data[0..4])
					} else if data[0] == 'b' as u8 {
						write_bytes(self.handle, &data[0..3])
					} else if data[0] == 'c' as u8 {
						write_bytes(self.handle, &data[0..2])
					} else if data[0] == 'd' as u8 {
						write_bytes(self.handle, &data[0..1])
					} else {
						write_bytes(self.handle, data)
					}
				} else {
					write_bytes(self.handle, data)
				}
			}
		};
		if errno().0 != 0 || len < 0 || self.debug_pending || self.debug_write_error {
			// check for would block
			if (errno().0 != EAGAIN
				&& errno().0 != ETEMPUNAVAILABLE
				&& errno().0 != WINNONBLOCKING
				&& !self.debug_pending)
				|| self.debug_write_error
			{
				let fmt = format!("writing generated error: {}", errno());
				return Err(err!(ErrKind::IO, fmt));
			}
			self.queue_data(data)?;
		} else {
			let len: usize = len.try_into()?;
			if len < data_len {
				self.queue_data(&data[len..])?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub fn trigger_on_read(&mut self) -> Result<(), Error> {
		{
			debug!("wlock for {}", self.id)?;
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard();
			(**guard).set_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ);
			debug!("unlockwlock for {}", self.id)?;
		}
		{
			let mut event_handler_data = self.event_handler_data.wlock()?;
			let guard = event_handler_data.guard();
			(**guard).write_queue.enqueue(self.id)?;
		}
		self.wakeup.wakeup()?;
		Ok(())
	}

	pub fn handle(&self) -> Handle {
		self.handle
	}

	pub fn id(&self) -> u128 {
		self.id
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn queue_data(&mut self, data: &[u8]) -> Result<(), Error> {
		debug!("queue data = {:?}", data)?;
		let was_pending = {
			debug!("wlock for {}", self.id)?;
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard();
			let ret = (**guard).is_set(WRITE_STATE_FLAG_PENDING);
			(**guard).set_flag(WRITE_STATE_FLAG_PENDING);
			(**guard).write_buffer.extend(data);
			debug!("unlock wlock for {}", self.id)?;
			ret
		};
		if !was_pending {
			let mut event_handler_data = self.event_handler_data.wlock()?;
			let guard = event_handler_data.guard();
			(**guard).write_queue.enqueue(self.id)?;
		}
		self.wakeup.wakeup()?;

		Ok(())
	}
}

impl CloseHandle {
	pub fn new(
		write_state: &mut Box<dyn LockBox<WriteState>>,
		id: u128,
		event_handler_data: &mut Box<dyn LockBox<EventHandlerData>>,
	) -> Self {
		Self {
			write_state: write_state.clone(),
			id,
			event_handler_data: event_handler_data.clone(),
		}
	}
	/// Close the connection associated with this [`crate::CloseHandle`].
	pub fn close(&mut self) -> Result<(), Error> {
		handle_close(&mut self.write_state, self.id, &mut self.event_handler_data)
	}
}

impl<'a> ConnectionData<'a> {
	fn new(
		rwi: &'a mut StreamInfo,
		tid: usize,
		slabs: &'a mut Box<dyn SlabAllocator + Send + Sync>,
		wakeup: Wakeup,
		event_handler_data: Box<dyn LockBox<EventHandlerData>>,
		debug_write_queue: bool,
		debug_pending: bool,
		debug_write_error: bool,
		debug_suspended: bool,
	) -> Self {
		Self {
			rwi,
			tid,
			slabs,
			wakeup,
			event_handler_data,
			debug_write_queue,
			debug_pending,
			debug_write_error,
			debug_suspended,
		}
	}

	fn clear_through_impl(&mut self, slab_id: u32) -> Result<(), Error> {
		self.rwi.clear_through_impl(slab_id, &mut self.slabs)
	}
}

impl<'a> ConnData for ConnectionData<'a> {
	fn tid(&self) -> usize {
		self.tid
	}
	fn get_connection_id(&self) -> u128 {
		self.rwi.id
	}
	fn get_handle(&self) -> Handle {
		self.rwi.handle
	}
	fn get_accept_handle(&self) -> Option<Handle> {
		self.rwi.accept_handle
	}
	fn write_handle(&self) -> WriteHandle {
		WriteHandle::new(
			self.rwi.handle,
			self.rwi.id,
			self.wakeup.clone(),
			self.rwi.write_state.clone(),
			self.event_handler_data.clone(),
			self.debug_write_queue,
			self.debug_pending,
			self.debug_write_error,
			self.debug_suspended,
			self.rwi.tls_server.clone(),
			self.rwi.tls_client.clone(),
		)
	}
	fn borrow_slab_allocator<F, T>(&self, mut f: F) -> Result<T, Error>
	where
		F: FnMut(&Box<dyn SlabAllocator + Send + Sync>) -> Result<T, Error>,
	{
		f(&self.slabs)
	}
	fn slab_offset(&self) -> u16 {
		self.rwi.slab_offset
	}
	fn first_slab(&self) -> u32 {
		self.rwi.first_slab
	}
	fn last_slab(&self) -> u32 {
		self.rwi.last_slab
	}
	fn clear_through(&mut self, slab_id: u32) -> Result<(), Error> {
		self.clear_through_impl(slab_id)
	}
}

impl EventHandlerData {
	fn new(write_queue_size: usize, nhandles_queue_size: usize) -> Result<Self, Error> {
		let connection_info = ConnectionInfo::ListenerInfo(ListenerInfo {
			handle: 0,
			id: 0,
			is_reuse_port: false,
			tls_config: None,
		});

		let evhd = EventHandlerData {
			write_queue: queue_sync_box!(write_queue_size, &0)?,
			nhandles: queue_sync_box!(nhandles_queue_size, &connection_info)?,
			stop: false,
			stopped: false,
			attachments: HashMap::new(),
		};
		Ok(evhd)
	}
}

impl<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
	EventHandlerImpl<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
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
	pub(crate) fn new(config: EventHandlerConfig) -> Result<Self, Error> {
		Self::check_config(&config)?;
		let mut data = array!(config.threads, &lock_box!(EventHandlerData::new(1, 1,)?)?)?;
		for i in 0..config.threads {
			data[i] = lock_box!(EventHandlerData::new(
				config.write_queue_size,
				config.nhandles_queue_size
			)?)?;
		}
		let mut wakeup = array!(config.threads, &Wakeup::new()?)?;
		for i in 0..config.threads {
			wakeup[i] = Wakeup::new()?;
		}

		let ret = Self {
			on_read: None,
			on_accept: None,
			on_close: None,
			housekeeper: None,
			on_panic: None,
			config,
			data,
			wakeup,
			thread_pool_stopper: None,
			debug_write_queue: false,
			debug_pending: false,
			debug_write_error: false,
			debug_suspended: false,
			debug_fatal_error: false,
			debug_tls_server_error: false,
			debug_read_error: false,
			debug_tls_read: false,
			debug_attachment_none: false,
			debug_rw_accept_id_none: false,
			full_stats: lock_box!(EventHandlerStats::new())?,
			last_stats: lock_box!(EventHandlerStats::new())?,
		};
		Ok(ret)
	}

	#[cfg(test)]
	fn set_debug_write_queue(&mut self, value: bool) {
		self.debug_write_queue = value;
	}

	#[cfg(test)]
	fn set_debug_pending(&mut self, value: bool) {
		self.debug_pending = value;
	}

	#[cfg(test)]
	fn set_debug_write_error(&mut self, value: bool) {
		self.debug_write_error = value;
	}

	#[cfg(test)]
	fn set_debug_suspended(&mut self, value: bool) {
		self.debug_suspended = value;
	}

	#[cfg(test)]
	fn set_debug_read_error(&mut self, value: bool) {
		self.debug_read_error = value;
	}

	#[cfg(test)]
	fn set_debug_fatal_error(&mut self, value: bool) {
		self.debug_fatal_error = value;
	}

	#[cfg(test)]
	fn set_debug_tls_server_error(&mut self, value: bool) {
		self.debug_tls_server_error = value;
	}

	#[cfg(test)]
	fn set_debug_tls_read(&mut self, value: bool) {
		self.debug_tls_read = value;
	}

	#[cfg(test)]
	fn set_on_panic_none(&mut self) {
		self.housekeeper = None;
		self.on_panic = None;
	}

	#[cfg(test)]
	fn set_on_read_none(&mut self) {
		self.on_read = None;
	}

	#[cfg(test)]
	fn set_attachment_none(&mut self) {
		self.debug_attachment_none = true;
	}

	#[cfg(test)]
	fn set_debug_rw_accept_id_none(&mut self) {
		self.debug_rw_accept_id_none = true;
	}

	#[cfg(test)]
	fn set_on_accept_none(&mut self) {
		self.on_accept = None;
	}

	#[cfg(test)]
	fn set_on_close_none(&mut self) {
		self.on_close = None;
	}

	fn check_config(config: &EventHandlerConfig) -> Result<(), Error> {
		if config.read_slab_count >= u32::MAX.try_into()? {
			let fmt = "read_slab_count must be smaller than u32::MAX";
			let err = err!(ErrKind::Configuration, fmt);
			return Err(err);
		}
		Ok(())
	}

	fn execute_thread(
		&mut self,
		wakeup: &mut Wakeup,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
		is_restart: bool,
	) -> Result<(), Error> {
		debug!("Executing thread {}", ctx.tid)?;

		// add wakeup if this is the first start
		if !is_restart {
			let handle = wakeup.reader;
			debug!("wakeup handle is {}", handle)?;
			let e = EventIn {
				handle,
				etype: EventTypeIn::Read,
			};
			ctx.events_in.push(e);
		} else {
			// we have to do different cleanup for each type.
			debug!("last_proc_type={:?}", ctx.last_process_type)?;
			match ctx.last_process_type {
				LastProcessType::OnRead => {
					// unwrap is ok because last_rw always set before on_read
					let mut rw = ctx.last_rw.clone().unwrap();
					self.process_close(ctx, &mut rw, callback_context)?;
					ctx.counter += 1;
				}
				LastProcessType::OnAccept => {
					close_impl(ctx, ctx.events[ctx.counter].handle, false)?;
					ctx.counter += 1;
				}
				LastProcessType::OnAcceptOutOfBand => {
					debug!("close impl handle = {}", ctx.last_handle_oob)?;
					close_impl(ctx, ctx.last_handle_oob, false)?;
				}
				LastProcessType::OnClose => {
					ctx.counter += 1;
				}
				LastProcessType::Housekeeper => {}
			}

			// skip over the panicked request and continue processing remaining events
			self.process_events(ctx, wakeup, callback_context)?;
		}

		#[cfg(target_os = "macos")]
		let mut ret_kevs = vec![];
		#[cfg(target_os = "macos")]
		for _ in 0..self.config.max_events {
			ret_kevs.push(kevent::new(
				0,
				EventFilter::EVFILT_SYSCOUNT,
				EventFlag::empty(),
				FilterFlag::empty(),
			));
		}
		#[cfg(target_os = "macos")]
		let mut kevs = vec![];

		loop {
			debug!("start loop")?;

			// if there's an error in process_new connections, we shouldn't try to
			// execute anything.
			ctx.counter = 0;
			ctx.count = 0;
			let stop = self.process_new_connections(ctx, callback_context)?;
			if stop {
				break;
			}
			self.process_write_queue(ctx)?;
			ctx.count = {
				debug!("calling get_events")?;
				let count = {
					let (requested, _lock) = wakeup.pre_block()?;
					#[cfg(target_os = "macos")]
					let count = self.get_events(ctx, requested, &mut kevs, &mut ret_kevs)?;
					#[cfg(not(target_os = "macos"))]
					let count = self.get_events(ctx, requested)?;
					count
				};
				debug!("get_events returned with {} event", count)?;
				wakeup.post_block()?;
				#[cfg(target_os = "windows")]
				ctx.write_set.clear()?;
				count
			};

			ctx.counter = 0;
			self.process_housekeeper(ctx, callback_context)?;
			self.process_events(ctx, wakeup, callback_context)?;
		}
		debug!("thread {} stop ", ctx.tid)?;
		self.close_handles(ctx)?;
		{
			let mut data = self.data[ctx.tid].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).stopped = true;
		}
		Ok(())
	}

	fn process_housekeeper(
		&mut self,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		debug!("housekeep")?;

		if ctx.now.saturating_sub(ctx.last_housekeeper) >= self.config.housekeeping_frequency_millis
		{
			match &mut self.housekeeper {
				Some(housekeeper) => {
					ctx.last_process_type = LastProcessType::Housekeeper;
					match housekeeper(callback_context) {
						Ok(_) => {}
						Err(e) => {
							warn!("Callback housekeeper generated error: {}", e)?;
						}
					}
				}
				None => {}
			}
			ctx.last_housekeeper = ctx.now;
		}
		Ok(())
	}

	fn close_handles(&mut self, ctx: &mut EventHandlerContext) -> Result<(), Error> {
		debug!("close handles")?;
		// do a final iteration through the connection hashtable to deserialize each of the
		// listeners to remain consistent
		for (_id, conn_info) in ctx.connection_hashtable.iter() {
			match conn_info {
				ConnectionInfo::ListenerInfo(_) => {} // don't close listeners
				ConnectionInfo::StreamInfo(mut rw) => {
					{
						match rw.write_state.wlock() {
							Ok(mut state) => {
								let guard = state.guard();
								(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
							}
							Err(e) => {
								let _ = warn!("rw.write_state.wlock generated error: {:?}", e);
							}
						}
					}

					let _ = rw.clear_through_impl(rw.last_slab, &mut ctx.read_slabs);
					let _ = close_handle_impl(rw.handle);
				}
			}
		}
		match ctx.handle_hashtable.clear() {
			Ok(_) => {}
			Err(e) => {
				let _ = warn!("handle_hashtable.clear generated error: {:?}", e);
			}
		}
		match ctx.connection_hashtable.clear() {
			Ok(_) => {}
			Err(e) => {
				let _ = warn!("connetion_hashtable.clear generated error: {:?}", e);
			}
		}
		debug!("handles closed")?;
		Ok(())
	}

	fn type_of<T>(_: T) -> &'static str {
		type_name::<T>()
	}

	fn process_write_queue(&mut self, ctx: &mut EventHandlerContext) -> Result<(), Error> {
		debug!("process write queue")?;
		let mut data = self.data[ctx.tid].wlock_ignore_poison()?;
		loop {
			let guard = data.guard();
			match (**guard).write_queue.dequeue() {
				Some(next) => {
					debug!("write q hashtable.get({})", next)?;
					match ctx.connection_hashtable.get(&next)? {
						Some(mut ci) => match &mut ci {
							ConnectionInfo::StreamInfo(ref mut rwi) => {
								{
									let mut write_state = rwi.write_state.wlock()?;
									let guard = write_state.guard();
									if (**guard).is_set(WRITE_STATE_FLAG_SUSPEND) {
										let ev = EventIn {
											handle: rwi.handle,
											etype: EventTypeIn::Suspend,
										};
										ctx.events_in.push(ev);
										(**guard).unset_flag(WRITE_STATE_FLAG_SUSPEND);
										(**guard).unset_flag(WRITE_STATE_FLAG_RESUME);
										#[cfg(unix)]
										{
											let h = rwi.handle;
											let s = unsafe { TcpStream::from_raw_fd(h) };
											s.set_nonblocking(false)?;
											s.into_raw_fd();
										}
										#[cfg(windows)]
										{
											let h = rwi.handle;
											let s = unsafe { TcpStream::from_raw_socket(u64!(h)) };
											s.set_nonblocking(false)?;
											s.into_raw_socket();
										}
									} else if (**guard).is_set(WRITE_STATE_FLAG_RESUME) {
										let ev_in = EventIn {
											handle: rwi.handle,
											etype: EventTypeIn::Resume,
										};
										ctx.events_in.push(ev_in);
										(**guard).unset_flag(WRITE_STATE_FLAG_SUSPEND);
										(**guard).unset_flag(WRITE_STATE_FLAG_RESUME);
										#[cfg(unix)]
										{
											unsafe { fcntl(rwi.handle, F_SETFL, O_NONBLOCK) };
										}
										#[cfg(windows)]
										{
											set_windows_socket_options(rwi.handle)?;
										}
									} else {
										debug!("pushing a write event for handle={}", rwi.handle)?;
										let handle = rwi.handle;
										let ev_in = EventIn {
											handle,
											etype: EventTypeIn::Write,
										};
										ctx.events_in.push(ev_in);
									}
								}

								// we must update the hashtable to keep
								// things consistent in terms of our
								// deserializable/serializable
								ctx.connection_hashtable.insert(&next, &ci)?;
							}
							ConnectionInfo::ListenerInfo(li) => {
								warn!("Attempt to write to a listener: {:?}", li.handle)?;
								ctx.connection_hashtable.insert(&next, &ci)?;
							}
						},
						// already closed connection
						None => debug!("Couldn't look up conn info for (2) {}", next)?,
					}
				}
				None => break,
			}
		}
		Ok(())
	}

	fn process_new_connections(
		&mut self,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<bool, Error> {
		let data2 = self.data.clone();
		let mut data = self.data[ctx.tid].wlock_ignore_poison()?;
		let guard = data.guard();
		if (**guard).stop {
			return Ok(true);
		}
		loop {
			let mut attachment: Option<AttachmentHolder>;
			let id;

			let mut next = (**guard).nhandles.dequeue();
			match next {
				Some(ref mut nhandle) => {
					debug!("handle={:?} on tid={}", nhandle, ctx.tid)?;
					match nhandle {
						ConnectionInfo::ListenerInfo(li) => {
							match Self::insert_hashtables(ctx, li.id, li.handle, nhandle) {
								Ok(_) => {
									let ev_in = EventIn {
										handle: li.handle,
										etype: EventTypeIn::Read,
									};
									ctx.events_in.push(ev_in);
								}
								Err(e) => {
									warn!("inshash li generated error: {}. Closing.", e)?;
									close_impl(ctx, li.handle, true)?;
								}
							}
							id = li.id;
							attachment = (**guard).attachments.remove(&id);
							debug!("id={},att={:?}", id, attachment)?;
						}
						ConnectionInfo::StreamInfo(rwi) => {
							match &mut self.on_accept {
								Some(on_accept) => {
									let tid = ctx.tid;
									let rslabs = &mut ctx.read_slabs;
									let wakeup = self.wakeup[tid].clone();
									let data = data2[tid].clone();
									let q = self.debug_write_queue;
									let p = self.debug_pending;
									let we = self.debug_write_error;
									let s = self.debug_suspended;
									let mut rwi = rwi.clone();
									let handle = rwi.handle;
									let mut cd = ConnectionData::new(
										&mut rwi, tid, rslabs, wakeup, data, q, p, we, s,
									);
									ctx.last_process_type = LastProcessType::OnAcceptOutOfBand;
									ctx.last_handle_oob = handle;
									match on_accept(&mut cd, callback_context) {
										Ok(_) => {}
										Err(e) => {
											warn!("Callback on_accept generated error: {}", e)?;
										}
									}
								}
								None => {}
							}

							match Self::insert_hashtables(ctx, rwi.id, rwi.handle, nhandle) {
								Ok(_) => {
									let ev_in = EventIn {
										handle: rwi.handle,
										etype: EventTypeIn::Read,
									};
									ctx.events_in.push(ev_in);
								}
								Err(e) => {
									warn!("inshash rw generated error: {}. Closing.", e)?;
									close_impl(ctx, rwi.handle, true)?;
								}
							}
							id = rwi.id;
							let acc_id = rwi.accept_id;
							attachment = (**guard).attachments.remove(&id);
							if attachment.is_none() {
								match acc_id {
									Some(id) => attachment = (**guard).attachments.remove(&id),
									None => {}
								}
							}
						}
					}
				}
				None => break,
			}

			debug!("process att = {:?} on tid = {}", attachment, ctx.tid)?;
			match attachment {
				Some(attachment) => {
					ctx.attachments.insert(id, attachment);
				}
				None => {}
			}
		}
		Ok(false)
	}

	fn insert_hashtables(
		ctx: &mut EventHandlerContext,
		id: u128,
		handle: Handle,
		conn_info: &ConnectionInfo,
	) -> Result<(), Error> {
		ctx.connection_hashtable.insert(&id, conn_info)?;
		ctx.handle_hashtable.insert(&handle, &id)?;
		Ok(())
	}

	fn process_events(
		&mut self,
		ctx: &mut EventHandlerContext,
		wakeup: &Wakeup,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		debug!("event count = {}, tid={}", ctx.count, ctx.tid)?;
		loop {
			if ctx.counter == ctx.count {
				break;
			}
			if ctx.events[ctx.counter].handle == wakeup.reader {
				debug!("WAKEUP, handle={}, tid={}", wakeup.reader, ctx.tid)?;
				read_bytes(ctx.events[ctx.counter].handle, &mut [0u8; 1]);
				#[cfg(target_os = "windows")]
				epoll_ctl_impl(
					EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
					ctx.events[ctx.counter].handle,
					&mut ctx.filter_set,
					ctx.selector as *mut c_void,
					ctx.tid,
				)?;

				ctx.counter += 1;
				continue;
			}
			debug!("evt={:?}", ctx.events[ctx.counter])?;
			match ctx.handle_hashtable.get(&ctx.events[ctx.counter].handle)? {
				Some(id) => {
					match ctx.connection_hashtable.get(&id)? {
						Some(mut ci) => match &mut ci {
							ConnectionInfo::ListenerInfo(li) => {
								// write back to keep our hashtable consistent
								ctx.connection_hashtable
									.insert(&id, &ConnectionInfo::ListenerInfo(li.clone()))?;

								loop {
									let handle = self.process_accept(&li, ctx, callback_context)?;
									#[cfg(unix)]
									if handle <= 0 {
										break;
									}
									#[cfg(windows)]
									if handle == usize::MAX {
										break;
									}
								}
							}
							ConnectionInfo::StreamInfo(rw) => {
								ctx.do_write_back = true;
								match ctx.events[ctx.counter].etype {
									EventType::Read => {
										self.process_read(rw, ctx, callback_context)?
									}
									EventType::Write => {
										debug!("write event {:?}", ctx.events[ctx.counter])?;
										self.process_write(rw, ctx, callback_context)?
									}
								}

								// unless process close was called
								// and the entry was removed, we
								// reinsert the entry to keep the
								// table consistent
								if ctx.do_write_back {
									ctx.connection_hashtable
										.insert(&id, &ConnectionInfo::StreamInfo(rw.clone()))?;
								}
							}
						},
						// already closed connection
						None => debug!("Couldn't look up conn info for (1) {}", id)?,
					}
				}
				// normal because we can try to write to a closed connection
				None => debug!(
					"Couldn't look up id for handle {}, tid={}",
					ctx.events[ctx.counter].handle, ctx.tid,
				)?,
			}
			ctx.counter += 1;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn process_write(
		&mut self,
		rw: &mut StreamInfo,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		let mut do_close = false;
		let mut trigger_on_read = false;

		{
			debug!("wlock for {}", rw.id)?;
			let mut write_state = rw.write_state.wlock()?;
			let guard = write_state.guard();
			loop {
				let len = (**guard).write_buffer.len();
				if len == 0 {
					(**guard).unset_flag(WRITE_STATE_FLAG_PENDING);
					if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
						do_close = true;
					} else if (**guard).is_set(WRITE_STATE_FLAG_TRIGGER_ON_READ) {
						(**guard).unset_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ);
						trigger_on_read = true;
					}
					(**guard).write_buffer.shrink_to_fit();
					break;
				}
				let wlen = write_bytes(rw.handle, &(**guard).write_buffer);
				debug!(
					"write handle = {} bytes = {}, buf={}",
					rw.handle,
					wlen,
					&(**guard).write_buffer.len()
				)?;
				if wlen < 0 {
					// check if it's an actual error and not wouldblock
					if errno().0 != EAGAIN
						&& errno().0 != ETEMPUNAVAILABLE
						&& errno().0 != WINNONBLOCKING
					{
						do_close = true;
					}

					break;
				}
				(**guard).write_buffer.drain(0..wlen as usize);
				(**guard).write_buffer.shrink_to_fit();
			}
		}

		if ctx.debug_trigger_on_read {
			trigger_on_read = true;
		}

		debug!("trigger_on_read = {}", trigger_on_read)?;

		if trigger_on_read {
			match &mut self.on_read {
				Some(on_read) => {
					ctx.last_process_type = LastProcessType::OnRead;
					ctx.last_rw = Some(rw.clone());

					let attachment: Option<AttachmentHolder> =
						if self.debug_attachment_none || self.debug_rw_accept_id_none {
							None
						} else {
							match ctx.attachments.get(&rw.id) {
								Some(attachment) => Some(attachment.clone()),
								None => None,
							}
						};

					let attachment = match attachment {
						Some(attachment) => Some(attachment),
						None => {
							let target = if self.debug_rw_accept_id_none {
								None
							} else if self.debug_attachment_none {
								Some(1)
							} else {
								rw.accept_id
							};
							match target {
								Some(id) => {
									let target = if self.debug_attachment_none {
										None
									} else {
										ctx.attachments.get(&id)
									};
									match target {
										Some(attachment) => Some(attachment.clone()),
										None => None,
									}
								}
								None => None,
							}
						}
					};

					match on_read(
						&mut ConnectionData::new(
							rw,
							ctx.tid,
							&mut ctx.read_slabs,
							self.wakeup[ctx.tid].clone(),
							self.data[ctx.tid].clone(),
							self.debug_write_queue,
							self.debug_pending,
							self.debug_write_error,
							self.debug_suspended,
						),
						callback_context,
						attachment,
					) {
						Ok(_) => {}
						Err(e) => {
							warn!("Callback on_read generated error: {}", e)?;
						}
					}
				}
				None => {}
			}
		}

		if do_close {
			self.process_close(ctx, rw, callback_context)?;
		} else {
			#[cfg(target_os = "windows")]
			{
				epoll_ctl_impl(
					EPOLLIN | EPOLLOUT | EPOLLONESHOT | EPOLLRDHUP,
					rw.handle,
					&mut ctx.filter_set,
					ctx.selector as *mut c_void,
					ctx.tid,
				)?;
				ctx.write_set.insert(&rw.handle)?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn do_tls_server_read(
		&mut self,
		mut rw: StreamInfo,
		ctx: &mut EventHandlerContext,
	) -> Result<(isize, usize), Error> {
		let mut pt_len = 0;
		let handle = rw.handle;

		ctx.buffer.resize(TLS_CHUNKS, 0u8);
		ctx.buffer.shrink_to_fit();
		let len = read_bytes(handle, &mut ctx.buffer);
		if len >= 0 {
			ctx.buffer.truncate(len as usize);
		}
		let mut wbuf = vec![];
		if len > 0 {
			let mut tls_conn = rw.tls_server.as_mut().unwrap().wlock()?;
			let tls_conn = tls_conn.guard();
			(**tls_conn).read_tls(&mut &ctx.buffer[0..len.try_into().unwrap_or(0)])?;

			match (**tls_conn).process_new_packets() {
				Ok(io_state) => {
					pt_len = io_state.plaintext_bytes_to_read();
					if pt_len > ctx.buffer.len() {
						ctx.buffer.resize(pt_len, 0u8);
					}
					let buf = &mut ctx.buffer[0..pt_len];
					(**tls_conn).reader().read_exact(&mut buf[..pt_len])?;
				}
				Err(e) => {
					warn!("processing packets generated error: {}", e)?;
					return Ok((-1, 0)); // invalid text received. Close conn.
				}
			}
			(**tls_conn).write_tls(&mut wbuf)?;
		}

		if wbuf.len() > 0 {
			let rw = &mut rw;
			let tid = ctx.tid;
			let rs = &mut ctx.read_slabs;
			let wakeup = self.wakeup[ctx.tid].clone();
			let data = self.data[ctx.tid].clone();
			let d1 = self.debug_write_queue;
			let d2 = self.debug_pending;
			let d3 = self.debug_write_error;
			let d4 = self.debug_suspended;
			let connection_data = ConnectionData::new(rw, tid, rs, wakeup, data, d1, d2, d3, d4);
			connection_data.write_handle().do_write(&wbuf)?;
		}

		Ok((len, pt_len))
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn do_tls_client_read(
		&mut self,
		mut rw: StreamInfo,
		ctx: &mut EventHandlerContext,
	) -> Result<(isize, usize), Error> {
		let mut pt_len = 0;
		let handle = rw.handle;

		ctx.buffer.resize(TLS_CHUNKS, 0u8);
		ctx.buffer.shrink_to_fit();

		let len = read_bytes(handle, &mut ctx.buffer);
		if len >= 0 {
			ctx.buffer.truncate(len as usize);
		}

		let mut wbuf = vec![];
		if len > 0 {
			let mut tls_conn = rw.tls_client.as_mut().unwrap().wlock()?;
			let tls_conn = tls_conn.guard();
			(**tls_conn).read_tls(&mut &ctx.buffer[0..len.try_into().unwrap_or(0)])?;

			match (**tls_conn).process_new_packets() {
				Ok(io_state) => {
					pt_len = io_state.plaintext_bytes_to_read();
					if pt_len > ctx.buffer.len() {
						ctx.buffer.resize(pt_len, 0u8);
					}
					let buf = &mut ctx.buffer[0..pt_len];
					(**tls_conn).reader().read_exact(&mut buf[..pt_len])?;
				}
				Err(e) => {
					warn!("processing packets generated error: {}", e)?;
					return Ok((-1, 0)); // invalid text received. Close conn.
				}
			}
			(**tls_conn).write_tls(&mut wbuf)?;
		}

		if wbuf.len() > 0 {
			let rw = &mut rw;
			let tid = ctx.tid;
			let rs = &mut ctx.read_slabs;
			let wakeup = self.wakeup[ctx.tid].clone();
			let data = self.data[ctx.tid].clone();
			let d1 = self.debug_write_queue;
			let d2 = self.debug_pending;
			let d3 = self.debug_write_error;
			let d4 = self.debug_suspended;
			let connection_data = ConnectionData::new(rw, tid, rs, wakeup, data, d1, d2, d3, d4);
			connection_data.write_handle().do_write(&wbuf)?;
		}

		Ok((len, pt_len))
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn process_read(
		&mut self,
		rw: &mut StreamInfo,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		match rw.tls_server {
			Some(ref _tls_server) => {
				loop {
					let (raw_len, pt_len) = self.do_tls_server_read(rw.clone(), ctx)?;
					if raw_len <= 0 {
						// EAGAIN is would block. -2 is would block for windows
						if (errno().0 != EAGAIN
							&& errno().0 != ETEMPUNAVAILABLE
							&& errno().0 != WINNONBLOCKING
							&& raw_len != -2) || self.debug_tls_read
						{
							debug!("proc close: {} {}", rw.handle, self.debug_tls_read)?;
							self.process_close(ctx, rw, callback_context)?;
						} else if raw_len == -2 {
							#[cfg(target_os = "windows")]
							{
								if !ctx.write_set.contains(&rw.handle)? {
									epoll_ctl_impl(
										EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
										rw.handle,
										&mut ctx.filter_set,
										ctx.selector as *mut c_void,
										ctx.tid,
									)?;
								}
							}
						}
					} else if pt_len > 0 {
						ctx.buffer.truncate(pt_len);
						self.process_read_result(rw, ctx, callback_context, true)?;
					} else {
						#[cfg(target_os = "windows")]
						{
							if !ctx.write_set.contains(&rw.handle)? {
								epoll_ctl_impl(
									EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
									rw.handle,
									&mut ctx.filter_set,
									ctx.selector as *mut c_void,
									ctx.tid,
								)?;
							}
						}
					}
					if raw_len <= 0 {
						break;
					}
				}
				Ok(())
			}
			None => match rw.tls_client {
				Some(ref _tls_client) => {
					loop {
						let (raw_len, pt_len) = self.do_tls_client_read(rw.clone(), ctx)?;
						if raw_len <= 0 {
							// EAGAIN is would block. -2 is would block for windows
							if (errno().0 != EAGAIN
								&& errno().0 != ETEMPUNAVAILABLE && errno().0 != WINNONBLOCKING
								&& raw_len != -2) || self.debug_tls_read
							{
								debug!("proc close client")?;
								self.process_close(ctx, rw, callback_context)?;
							} else if raw_len == -2 {
								#[cfg(target_os = "windows")]
								{
									if !ctx.write_set.contains(&rw.handle)? {
										epoll_ctl_impl(
											EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
											rw.handle,
											&mut ctx.filter_set,
											ctx.selector as *mut c_void,
											ctx.tid,
										)?;
									}
								}
							}
						} else if pt_len > 0 {
							ctx.buffer.truncate(pt_len);
							self.process_read_result(rw, ctx, callback_context, true)?;
						} else {
							#[cfg(target_os = "windows")]
							{
								if !ctx.write_set.contains(&rw.handle)? {
									epoll_ctl_impl(
										EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
										rw.handle,
										&mut ctx.filter_set,
										ctx.selector as *mut c_void,
										ctx.tid,
									)?;
								}
							}
						}
						if raw_len <= 0 {
							break;
						}
					}
					Ok(())
				}
				None => self.process_read_result(rw, ctx, callback_context, false),
			},
		}
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn process_read_result(
		&mut self,
		rw: &mut StreamInfo,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
		tls: bool,
	) -> Result<(), Error> {
		let mut do_close = false;
		let mut total_len = 0;
		loop {
			// read as many slabs as we can
			let slabs = &mut ctx.read_slabs;
			let mut slab = if rw.last_slab == u32::MAX {
				debug!("pre allocate")?;
				let mut slab = match slabs.allocate() {
					Ok(slab) => slab,
					Err(e) => {
						// we could not allocate slabs. Drop connection.
						warn!("slabs.allocate1 generated error: {}", e)?;
						total_len = 0;
						do_close = true;
						break;
					}
				};
				debug!("allocate: {}/tid={}", slab.id(), ctx.tid)?;
				let slab_id: u32 = slab.id().try_into()?;
				rw.last_slab = slab_id;
				rw.first_slab = slab_id;

				rw.slab_offset = 0;
				slab.get_mut()[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
					.clone_from_slice(&u32::MAX.to_be_bytes());
				slab
			} else if rw.slab_offset as usize == READ_SLAB_NEXT_OFFSET {
				let slab_id: u32;
				{
					debug!("pre_allocate")?;
					let mut slab = match slabs.allocate() {
						Ok(slab) => slab,
						Err(e) => {
							// we could not allocate slabs. Drop connection.
							warn!("slabs.allocate2 generated error: {}", e)?;
							total_len = 0;
							do_close = true;
							break;
						}
					};
					slab_id = slab.id().try_into()?;
					debug!("allocatesecond: {}/tid={}", slab_id, ctx.tid)?;
					slab.get_mut()[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						.clone_from_slice(&u32::MAX.to_be_bytes());
				}

				slabs.get_mut(rw.last_slab.try_into()?)?.get_mut()
					[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
					.clone_from_slice(&(slab_id as u32).to_be_bytes());
				rw.last_slab = slab_id;
				rw.slab_offset = 0;
				debug!("rw.last_slab={}", rw.last_slab)?;

				slabs.get_mut(slab_id.try_into()?)?
			} else {
				slabs.get_mut(rw.last_slab.try_into()?)?
			};
			let len = if tls {
				let slab_offset = rw.slab_offset as usize;
				// this is a tls read
				let slen = READ_SLAB_NEXT_OFFSET.saturating_sub(slab_offset);
				let mut clen = ctx.buffer.len();
				if clen > slen {
					clen = slen;
				}
				slab.get_mut()[slab_offset..clen + slab_offset]
					.clone_from_slice(&ctx.buffer[0..clen]);
				ctx.buffer.drain(0..clen);
				ctx.buffer.shrink_to_fit();
				clen.try_into()?
			} else {
				// clear text read
				read_bytes(
					rw.handle,
					&mut slab.get_mut()[usize!(rw.slab_offset)..READ_SLAB_NEXT_OFFSET],
				)
			};

			if self.debug_fatal_error {
				if len > 0 {
					let index = usize!(rw.slab_offset);
					let slab = slab.get();
					debug!("debug fatal {}", slab[index])?;
					if slab[index] == '0' as u8 {
						let fmt = "test debug_fatal";
						let err = err!(ErrKind::Test, fmt);
						return Err(err);
					}
				}
			}

			debug!("len = {},handle={},e.0={}", len, rw.handle, errno().0)?;
			if len == 0 && !tls {
				do_close = true;
			}
			if len < 0 || self.debug_read_error {
				// EAGAIN is would block. -2 is would block for windows
				if (errno().0 != EAGAIN
					&& errno().0 != ETEMPUNAVAILABLE
					&& errno().0 != WINNONBLOCKING
					&& len != -2) || self.debug_read_error
				{
					do_close = true;
				}

				if len == -2 {
					#[cfg(target_os = "windows")]
					{
						if !ctx.write_set.contains(&rw.handle)? {
							epoll_ctl_impl(
								EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
								rw.handle,
								&mut ctx.filter_set,
								ctx.selector as *mut c_void,
								ctx.tid,
							)?;
						}
					}
				}
			}

			let target_len = READ_SLAB_NEXT_OFFSET
				.saturating_sub(rw.slab_offset.into())
				.try_into()?;
			if len < target_len {
				if len > 0 {
					total_len += len;
					rw.slab_offset += len as u16;

					#[cfg(target_os = "windows")]
					{
						if !ctx.write_set.contains(&rw.handle)? {
							epoll_ctl_impl(
								EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
								rw.handle,
								&mut ctx.filter_set,
								ctx.selector as *mut c_void,
								ctx.tid,
							)?;
						}
					}
				}

				break;
			}

			total_len += len;
			rw.slab_offset += len as u16;
		}
		debug!(
			"read {} on tid = {}, on_read.is_some()={}",
			total_len,
			ctx.tid,
			self.on_read.is_some()
		)?;

		if total_len > 0 {
			match &mut self.on_read {
				Some(on_read) => {
					ctx.last_process_type = LastProcessType::OnRead;
					ctx.last_rw = Some(rw.clone());
					debug!("trying id = {}, rid = {:?}", rw.id, rw.accept_id)?;
					let attachment: Option<AttachmentHolder> =
						if self.debug_attachment_none || self.debug_rw_accept_id_none {
							None
						} else {
							match ctx.attachments.get(&rw.id) {
								Some(attachment) => Some(attachment.clone()),
								None => None,
							}
						};

					let attachment = match attachment {
						Some(attachment) => Some(attachment),
						None => {
							let target = if self.debug_rw_accept_id_none {
								None
							} else {
								rw.accept_id
							};
							match target {
								Some(id) => {
									let target = if self.debug_attachment_none {
										None
									} else {
										ctx.attachments.get(&id)
									};
									match target {
										Some(attachment) => Some(attachment.clone()),
										None => None,
									}
								}
								None => None,
							}
						}
					};

					debug!("att set = {:?}", attachment)?;
					match on_read(
						&mut ConnectionData::new(
							rw,
							ctx.tid,
							&mut ctx.read_slabs,
							self.wakeup[ctx.tid].clone(),
							self.data[ctx.tid].clone(),
							self.debug_write_queue,
							self.debug_pending,
							self.debug_write_error,
							self.debug_suspended,
						),
						callback_context,
						attachment,
					) {
						Ok(_) => {}
						Err(e) => {
							warn!("Callback on_read generated error: {}", e)?;
						}
					}
				}
				None => {}
			}
		}

		if do_close {
			self.process_close(ctx, rw, callback_context)?;
		}

		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn process_close(
		&mut self,
		ctx: &mut EventHandlerContext,
		rw: &mut StreamInfo,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		debug!("proc close {}", rw.handle)?;

		// set the close flag to true so if another thread tries to
		// write there will be an error
		{
			let mut state = rw.write_state.wlock()?;
			let guard = state.guard();
			(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
			(**guard).write_buffer.clear();
			(**guard).write_buffer.shrink_to_fit();
		}

		let ci = ConnectionInfo::StreamInfo(rw.clone());
		// we must do an insert before removing to keep our arc's consistent
		ctx.connection_hashtable.insert(&rw.id, &ci)?;
		ctx.connection_hashtable.remove(&rw.id)?;
		ctx.attachments.remove(&rw.id);
		ctx.handle_hashtable.remove(&rw.handle)?;
		rw.clear_through_impl(rw.last_slab, &mut ctx.read_slabs)?;
		close_impl(ctx, rw.handle, false)?;
		ctx.do_write_back = false;

		match &mut self.on_close {
			Some(on_close) => {
				ctx.last_process_type = LastProcessType::OnClose;
				match on_close(
					&mut ConnectionData::new(
						rw,
						ctx.tid,
						&mut ctx.read_slabs,
						self.wakeup[ctx.tid].clone(),
						self.data[ctx.tid].clone(),
						self.debug_write_queue,
						self.debug_pending,
						self.debug_write_error,
						self.debug_suspended,
					),
					callback_context,
				) {
					Ok(_) => {}
					Err(e) => {
						warn!("Callback on_close generated error: {}", e)?;
					}
				}
			}
			None => {}
		}

		Ok(())
	}

	fn handle_invalid(handle: Handle) -> Result<Handle, Error> {
		#[cfg(unix)]
		if handle < 0 {
			return Err(err!(ErrKind::IllegalArgument, "invalid handle"));
		}
		#[cfg(windows)]
		if handle == usize::MAX {
			return Err(err!(ErrKind::IllegalArgument, "invalid handle"));
		}

		Ok(handle)
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn process_accept(
		&mut self,
		li: &ListenerInfo,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<Handle, Error> {
		set_errno(Errno(0));
		let handle = match accept_impl(li.handle) {
			Ok(handle) => handle,
			Err(e) => {
				warn!("Error accepting handle: {}", e)?;
				#[cfg(unix)]
				return Ok(-1);
				#[cfg(windows)]
				return Ok(usize::MAX);
			}
		};
		debug!("handle: {}, li.handle: {}", handle, li.handle)?;

		let handle = if ctx.debug_bypass_acc_err {
			handle
		} else {
			match Self::handle_invalid(handle) {
				Ok(handle) => handle,
				Err(_e) => {
					return Ok(handle);
				}
			}
		};

		let mut tls_server = None;
		if li.tls_config.is_some() {
			let tls_conn = RSConn::new(li.tls_config.as_ref().unwrap().clone());
			if tls_conn.is_err() || self.debug_tls_server_error {
				warn!("Error building tls_connection: {:?}", tls_conn)?;
			} else {
				let mut tls_conn = tls_conn.unwrap();
				tls_conn.set_buffer_limit(None);
				tls_server = Some(lock_box!(tls_conn)?);
			}

			if tls_server.is_none() {
				close_impl(ctx, handle, true)?;
				// send back the invalid handle to stay in the accept loop (we're not
				// able to stop until we get the blocking value).
				// don't add it to the data structures below though
				return Ok(handle);
			}
		}

		let id = random();
		let rwi = StreamInfo {
			id,
			handle,
			accept_handle: Some(li.handle),
			accept_id: Some(li.id),
			write_state: lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: true,
			tls_client: None,
			tls_server,
		};

		#[cfg(target_os = "windows")]
		epoll_ctl_impl(
			EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
			li.handle,
			&mut ctx.filter_set,
			ctx.selector as *mut c_void,
			ctx.tid,
		)?;

		if li.is_reuse_port {
			self.process_accepted_connection(ctx, handle, rwi, id, callback_context)?;
			debug!("process acc: {},tid={}", handle, ctx.tid)?;
		} else {
			let tid = random::<usize>() % self.config.threads;
			debug!("tid={},threads={}", tid, self.config.threads)?;

			ctx.last_process_type = LastProcessType::OnAccept;
			{
				let mut data = self.data[tid].wlock_ignore_poison()?;
				let guard = data.guard();

				// unwrap is ok because accept_id is always set above
				let id = rwi.accept_id.unwrap();
				match ctx.attachments.get(&id) {
					Some(attachment) => {
						(**guard).attachments.insert(id, attachment.clone());
					}
					None => {}
				};

				let ci = ConnectionInfo::StreamInfo(rwi);
				(**guard).nhandles.enqueue(ci)?;
			}

			debug!("wakeup called on tid = {}", tid)?;
			self.wakeup[tid].wakeup()?;
		}
		Ok(handle)
	}

	fn process_accepted_connection(
		&mut self,
		ctx: &mut EventHandlerContext,
		handle: Handle,
		mut rwi: StreamInfo,
		id: u128,
		callback_context: &mut ThreadContext,
	) -> Result<(), Error> {
		ctx.last_process_type = LastProcessType::OnAccept;
		match &mut self.on_accept {
			Some(on_accept) => {
				let rwi = &mut rwi;
				let tid = ctx.tid;
				let rslabs = &mut ctx.read_slabs;
				let wakeup = self.wakeup[ctx.tid].clone();
				let data = self.data[ctx.tid].clone();
				let wq = self.debug_write_queue;
				let p = self.debug_pending;
				let we = self.debug_write_error;
				let s = self.debug_suspended;
				let mut cd = ConnectionData::new(rwi, tid, rslabs, wakeup, data, wq, p, we, s);
				match on_accept(&mut cd, callback_context) {
					Ok(_) => {}
					Err(e) => {
						warn!("Callback on_accept generated error: {}", e)?;
					}
				}
			}
			None => {}
		}

		match Self::insert_hashtables(ctx, id, handle, &ConnectionInfo::StreamInfo(rwi)) {
			Ok(_) => {
				let ev_in = EventIn {
					handle,
					etype: EventTypeIn::Read,
				};
				ctx.events_in.push(ev_in);
			}
			Err(e) => {
				warn!("insert_hashtables generated error1: {}. Closing.", e)?;
				close_impl(ctx, handle, true)?;
			}
		}

		Ok(())
	}

	#[cfg(not(target_os = "macos"))]
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn get_events(&self, ctx: &mut EventHandlerContext, requested: bool) -> Result<usize, Error> {
		get_events_impl(&self.config, ctx, requested, false)
	}

	#[cfg(target_os = "macos")]
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn get_events(
		&self,
		ctx: &mut EventHandlerContext,
		requested: bool,
		kevs: &mut Vec<kevent>,
		ret_kevs: &mut Vec<kevent>,
	) -> Result<usize, Error> {
		get_events_impl(&self.config, ctx, requested, kevs, ret_kevs)
	}
}

impl<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
	EventHandler<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
	for EventHandlerImpl<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>
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
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error> {
		self.on_read = Some(Box::pin(on_read));
		Ok(())
	}
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error> {
		self.on_accept = Some(Box::pin(on_accept));
		Ok(())
	}
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error> {
		self.on_close = Some(Box::pin(on_close));
		Ok(())
	}
	fn set_housekeeper(&mut self, housekeeper: HouseKeeper) -> Result<(), Error> {
		self.housekeeper = Some(Box::pin(housekeeper));
		Ok(())
	}
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error> {
		self.on_panic = Some(Box::pin(on_panic));
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn stop(&mut self) -> Result<(), Error> {
		if self.thread_pool_stopper.is_none() {
			let err = err!(ErrKind::IllegalState, "start must be called before stop");
			return Err(err);
		}
		for i in 0..self.wakeup.size() {
			let mut data = self.data[i].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).stop = true;
			self.wakeup[i].wakeup()?;
		}

		loop {
			let mut stopped = true;
			for i in 0..self.data.size() {
				{
					let mut data = self.data[i].wlock_ignore_poison()?;
					let guard = data.guard();
					if !(**guard).stopped {
						stopped = false;
					}
				}
			}
			if stopped {
				break;
			}
		}

		self.thread_pool_stopper.as_mut().unwrap().stop()?;

		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn start(&mut self) -> Result<(), Error> {
		let config = ThreadPoolConfig {
			max_size: self.config.threads,
			min_size: self.config.threads,
			sync_channel_size: self.config.sync_channel_size,
			..Default::default()
		};
		let mut tp = Builder::build_thread_pool(config)?;

		let mut v = vec![];
		let mut v_panic = vec![];
		for i in 0..self.config.threads {
			let evh = self.clone();
			let wakeup = self.wakeup.clone();
			let ev_in = self.config.max_events_in;
			let max_ev = self.config.max_events;
			let max_hpt = self.config.max_handles_per_thread;
			let rsc = self.config.read_slab_count;
			let ctx = EventHandlerContext::new(i, ev_in, max_ev, max_hpt, rsc)?;
			let ctx = lock_box!(ctx)?;
			let thread_context = ThreadContext::new();
			let thread_context = lock_box!(thread_context)?;
			v.push((
				evh.clone(),
				wakeup.clone(),
				ctx.clone(),
				thread_context.clone(),
			));
			v_panic.push((evh, wakeup, ctx, thread_context));
		}

		let mut executor = lock_box!(tp.executor()?)?;
		let mut executor_clone = executor.clone();

		let on_panic = self.on_panic.clone();

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			let id: usize = id.try_into()?;
			let mut evh = v_panic[id].0.clone();
			let mut wakeup = v_panic[id].1.clone();
			let mut ctx = v_panic[id].2.clone();
			let mut thread_context = v_panic[id].3.clone();
			let mut thread_context_clone = thread_context.clone();
			let mut executor = executor.wlock()?;
			let executor = executor.guard();
			let mut on_panic = on_panic.clone();
			(**executor).execute(
				async move {
					debug!("calling on panic handler: {:?}", e)?;
					let mut thread_context = thread_context_clone.wlock_ignore_poison()?;
					let thread_context = thread_context.guard();
					match &mut on_panic {
						Some(on_panic) => match on_panic(thread_context, e) {
							Ok(_) => {}
							Err(e) => {
								warn!("Callback on_panic generated error: {}", e)?;
							}
						},
						None => {}
					}

					Ok(())
				},
				id.try_into()?,
			)?;
			(**executor).execute(
				async move {
					let mut ctx = ctx.wlock_ignore_poison()?;
					let ctx = ctx.guard();

					let mut thread_context = thread_context.wlock_ignore_poison()?;
					let thread_context = thread_context.guard();

					let evh = &mut evh;
					let wakeup = &mut wakeup[id];
					let ctx = &mut *ctx;
					let thc = &mut *thread_context;
					let isr = true;

					Self::execute_thread(evh, wakeup, ctx, thc, isr)
				},
				id.try_into()?,
			)?;
			Ok(())
		})?;

		tp.start()?;

		{
			let mut executor = executor_clone.wlock()?;
			let guard = executor.guard();
			(**guard) = tp.executor()?;
		}

		for i in 0..self.config.threads {
			let mut evh = v[i].0.clone();
			let mut wakeup = v[i].1.clone();
			let mut ctx = v[i].2.clone();
			let mut thread_context = v[i].3.clone();

			execute!(tp, i.try_into()?, {
				let tid = i;

				let mut ctx = ctx.wlock_ignore_poison()?;
				let ctx = ctx.guard();

				let mut thread_context = thread_context.wlock_ignore_poison()?;
				let thread_context = thread_context.guard();

				let evh = &mut evh;
				let wakeup = &mut wakeup[tid];
				let ctx = &mut *ctx;
				let thc = &mut *thread_context;
				let isr = false;
				Self::execute_thread(evh, wakeup, ctx, thc, isr)
			})?;
		}

		self.thread_pool_stopper = Some(tp.stopper()?);

		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn add_client(
		&mut self,
		connection: ClientConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<WriteHandle, Error> {
		let attachment = AttachmentHolder {
			attachment: Arc::new(attachment),
		};
		let tid: usize = random::<usize>() % self.data.size();
		let id: u128 = random::<u128>();
		let handle = connection.handle;
		let ws = WriteState {
			write_buffer: vec![],
			flags: 0,
		};
		let write_state = lock_box!(ws)?;

		let tls_client = match connection.tls_config {
			Some(tls_config) => {
				let server_name: &str = &tls_config.sni_host;
				let config = make_config(tls_config.trusted_cert_full_chain_file)?;
				let mut rc_conn = RCConn::new(config, server_name.try_into()?)?;
				rc_conn.set_buffer_limit(None);
				let tls_client = Some(lock_box!(rc_conn)?);
				tls_client
			}
			None => None,
		};
		let wh = WriteHandle::new(
			handle,
			id,
			self.wakeup[tid].clone(),
			write_state.clone(),
			self.data[tid].clone(),
			self.debug_write_queue,
			self.debug_pending,
			self.debug_write_error,
			self.debug_suspended,
			None,
			tls_client.clone(),
		);

		let rwi = StreamInfo {
			id,
			handle,
			accept_handle: None,
			accept_id: None,
			write_state,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client,
			tls_server: None,
		};

		{
			let mut data = self.data[tid].wlock_ignore_poison()?;
			let guard = data.guard();
			let ci = ConnectionInfo::StreamInfo(rwi);
			(**guard).nhandles.enqueue(ci)?;
			(**guard).attachments.insert(id, attachment);
		}

		self.wakeup[tid].wakeup()?;
		Ok(wh)
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn add_server(
		&mut self,
		connection: ServerConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<(), Error> {
		let attachment = AttachmentHolder {
			attachment: Arc::new(attachment),
		};
		debug!("type in add_ser = {:?}", Self::type_of(attachment.clone()))?;
		debug!("add server: {:?}", attachment)?;

		let tls_config = match connection.tls_config {
			Some(tls_config) => Some(Arc::new(
				ServerConfig::builder()
					.with_safe_defaults()
					.with_no_client_auth()
					.with_single_cert(
						load_certs(&tls_config.certificates_file)?,
						load_private_key(&tls_config.private_key_file)?,
					)?,
			)),
			None => None,
		};

		if connection.handles.size() != self.data.size() {
			let fmt = "connections.handles must equal the number of threads";
			let err = err!(ErrKind::IllegalArgument, fmt);
			return Err(err);
		}

		let attachment = Arc::new(attachment);

		for i in 0..connection.handles.size() {
			let handle = connection.handles[i];
			// check for 0 which means to skip this handle (port not reused)
			if handle != 0 {
				let mut data = self.data[i].wlock_ignore_poison()?;
				let wakeup = &mut self.wakeup[i];
				let guard = data.guard();
				let id = random();
				let li = ListenerInfo {
					id,
					handle,
					is_reuse_port: connection.is_reuse_port,
					tls_config: tls_config.clone(),
				};
				let ci = ConnectionInfo::ListenerInfo(li);
				(**guard).nhandles.enqueue(ci)?;
				(**guard)
					.attachments
					.insert(id, attachment.as_ref().clone());
				debug!(
					"add handle: {}, id={}, att={:?}",
					handle,
					id,
					attachment.clone()
				)?;
				wakeup.wakeup()?;
			}
		}
		Ok(())
	}

	fn event_handler_data(&self) -> Result<Array<Box<dyn LockBox<EventHandlerData>>>, Error> {
		Ok(self.data.clone())
	}

	fn stats_incremental(&self) -> Result<EventHandlerStats, Error> {
		// TODO: implement stats
		let last_stats = self.last_stats.rlock()?;
		let guard = last_stats.guard();
		Ok((**guard).clone())
	}

	fn stats(&self) -> Result<EventHandlerStats, Error> {
		// TODO: implement stats
		let full_stats = self.full_stats.rlock()?;
		let guard = full_stats.guard();
		Ok((**guard).clone())
	}
}

impl Wakeup {
	fn new() -> Result<Self, Error> {
		set_errno(Errno(0));
		let (reader, writer, _tcp_stream, _tcp_listener) = get_reader_writer()?;
		let requested = lock_box!(false)?;
		let needed = lock_box!(false)?;
		Ok(Self {
			_tcp_stream,
			_tcp_listener,
			reader,
			writer,
			requested,
			needed,
		})
	}

	fn wakeup(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let needed = self.needed.rlock()?;
		let need_wakeup = **needed.guard() && !(**requested.guard());
		**requested.guard() = true;
		if need_wakeup {
			debug!("wakeup writing to {}", self.writer)?;
			let len = write_bytes(self.writer, &[0u8; 1]);
			debug!("len={},errno={}", len, errno())?;
		}
		Ok(())
	}

	fn pre_block(&mut self) -> Result<(bool, RwLockReadGuardWrapper<bool>), Error> {
		let requested = self.requested.rlock()?;
		{
			let mut needed = self.needed.wlock()?;
			**needed.guard() = true;
		}
		let lock_guard = self.needed.rlock()?;
		let is_requested = **requested.guard();
		Ok((is_requested, lock_guard))
	}

	fn post_block(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let mut needed = self.needed.wlock()?;

		**requested.guard() = false;
		**needed.guard() = false;
		Ok(())
	}
}

fn read_bytes(handle: Handle, buf: &mut [u8]) -> isize {
	set_errno(Errno(0));
	read_bytes_impl(handle, buf)
}

fn write_bytes(handle: Handle, buf: &[u8]) -> isize {
	set_errno(Errno(0));
	let _ = debug!("write bytes to handle = {}", handle);
	write_bytes_impl(handle, buf)
}

fn make_config(trusted_cert_full_chain_file: Option<String>) -> Result<Arc<ClientConfig>, Error> {
	let mut root_store = RootCertStore::empty();
	root_store.add_server_trust_anchors(TLS_SERVER_ROOTS.iter().map(|ta| {
		OwnedTrustAnchor::from_subject_spki_name_constraints(
			ta.subject,
			ta.spki,
			ta.name_constraints,
		)
	}));

	match trusted_cert_full_chain_file {
		Some(trusted_cert_full_chain_file) => {
			let full_chain_certs = load_certs(&trusted_cert_full_chain_file)?;
			for i in 0..full_chain_certs.len() {
				root_store.add(&full_chain_certs[i])?;
			}
		}
		None => {}
	}

	let config = ClientConfig::builder()
		.with_safe_default_cipher_suites()
		.with_safe_default_kx_groups()
		.with_safe_default_protocol_versions()?
		.with_root_certificates(root_store)
		.with_no_client_auth();

	Ok(Arc::new(config))
}

fn load_certs(filename: &str) -> Result<Vec<Certificate>, Error> {
	let certfile = File::open(filename)?;
	let mut reader = BufReader::new(certfile);
	let certs = certs(&mut reader)?;
	Ok(certs.iter().map(|v| Certificate(v.clone())).collect())
}

fn load_private_key(filename: &str) -> Result<PrivateKey, Error> {
	match read_one(&mut BufReader::new(File::open(filename)?)) {
		Ok(Some(Item::RSAKey(key))) => Ok(PrivateKey(key)),
		Ok(Some(Item::PKCS8Key(key))) => Ok(PrivateKey(key)),
		Ok(Some(Item::ECKey(key))) => Ok(PrivateKey(key)),
		_ => {
			let fmt = format!("no key or unsupported type found in file: {}", filename);
			Err(err!(ErrKind::IllegalArgument, fmt))
		}
	}
}

#[cfg(not(tarpaulin_include))] // assert full coverage for this function
fn handle_close(
	write_state: &mut Box<dyn LockBox<WriteState>>,
	id: u128,
	event_handler_data: &mut Box<dyn LockBox<EventHandlerData>>,
) -> Result<(), Error> {
	{
		debug!("wlock for {}", id)?;
		let mut write_state = write_state.wlock()?;
		let guard = write_state.guard();
		if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
			// it's already closed no double closes
			return Ok(());
		}
		(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
		debug!("unlockwlock for {}", id)?;
	}
	{
		let mut event_handler_data = event_handler_data.wlock()?;
		let guard = event_handler_data.guard();
		(**guard).write_queue.enqueue(id)?;
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::evh::{create_listeners, make_config, read_bytes};
	use crate::evh::{load_private_key, READ_SLAB_NEXT_OFFSET, READ_SLAB_SIZE};
	use crate::types::{
		ConnectionInfo, Event, EventHandlerContext, EventHandlerImpl, EventType, ListenerInfo,
		StreamInfo, Wakeup, WriteState,
	};
	use crate::{
		ClientConnection, ConnData, EventHandler, EventHandlerConfig, ServerConnection,
		ThreadContext, TlsClientConfig, TlsServerConfig, READ_SLAB_DATA_SIZE,
	};

	use bmw_deps::rand::random;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::port::pick_free_port;
	use bmw_util::*;
	use std::io::Read;
	use std::io::Write;
	use std::net::TcpStream;
	#[cfg(unix)]
	use std::os::unix::io::IntoRawFd;
	#[cfg(windows)]
	use std::os::windows::io::IntoRawSocket;
	use std::sync::mpsc::sync_channel;
	use std::sync::Arc;
	use std::thread::{sleep, spawn};
	use std::time::Duration;

	#[cfg(unix)]
	use std::os::unix::io::{AsRawFd, FromRawFd};
	#[cfg(windows)]
	use std::os::windows::io::{AsRawSocket, FromRawSocket};

	#[cfg(target_os = "linux")]
	use crate::linux::*;
	#[cfg(target_os = "macos")]
	use crate::mac::*;
	#[cfg(windows)]
	use crate::win::*;

	info!();

	#[test]
	fn test_wakeup() -> Result<(), Error> {
		let check = lock!(0)?;
		let mut check_clone = check.clone();
		let mut wakeup = Wakeup::new()?;
		let wakeup_clone = wakeup.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			let mut wakeup = wakeup_clone;
			{
				let wakeup_clone = wakeup.clone();

				loop {
					let len;

					sleep(Duration::from_millis(100));
					{
						let (_requested, _lock) = wakeup.pre_block()?;
						let mut buffer = [0u8; 1];
						info!("reader = {}", wakeup_clone.reader)?;
						info!("writer = {}", wakeup_clone.writer)?;

						len = read_bytes(wakeup_clone.reader, &mut buffer);
						if len == 1 {
							break;
						}
					}
				}
			}
			wakeup.post_block()?;

			let mut check = check_clone.wlock()?;
			**check.guard() = 1;

			Ok(())
		});

		sleep(Duration::from_millis(300));
		wakeup.wakeup()?;

		loop {
			sleep(Duration::from_millis(1));
			let check = check.rlock()?;
			if **(check).guard() == 1 {
				break;
			}
		}
		Ok(())
	}

	#[test]
	fn test_evh_basic() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test2");
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_basic_read_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_basic read error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |conn_data, _thread_context| {
				info!("on close: {}", conn_data.get_handle())?;
				Ok(())
			})?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.set_debug_tls_read(true);
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);

			// connection will close because of the error
			assert_eq!(connection.read(&mut buf)?, 0);

			let port = pick_free_port()?;
			let addr2 = &format!("127.0.0.1:{}", port)[..];
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evhserver = EventHandlerImpl::new(config)?;
			evhserver.set_on_read(move |conn_data, _thread_context, _attachment| {
				debug!("on read slab_offset = {}", conn_data.slab_offset())?;
				let first_slab = conn_data.first_slab();
				let last_slab = conn_data.last_slab();
				let slab_offset = conn_data.slab_offset();
				debug!("first_slab={}", first_slab)?;
				let res = conn_data.borrow_slab_allocator(move |sa| {
					let slab = sa.get(first_slab.try_into()?)?;
					assert_eq!(first_slab, last_slab);
					info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				})?;
				conn_data.clear_through(first_slab)?;
				conn_data.write_handle().write(&res)?;
				info!("res={:?}", res)?;
				Ok(())
			})?;
			evhserver.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evhserver.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evhserver.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evhserver.set_housekeeper(move |_thread_context| Ok(()))?;
			evhserver.start()?;
			let handles = create_listeners(threads, addr2, 10, false)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evhserver.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let connection = TcpStream::connect(addr2)?;
			connection.set_nonblocking(true)?;
			#[cfg(unix)]
			let connection_handle = connection.into_raw_fd();
			#[cfg(windows)]
			let connection_handle = connection.into_raw_socket().try_into()?;
			let client = ClientConnection {
				handle: connection_handle,
				tls_config: Some(TlsClientConfig {
					sni_host: "localhost".to_string(),
					trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
				}),
			};
			let mut wh = evh.add_client(client, Box::new(""))?;
			assert!(evh.event_handler_data().is_ok());
			wh.write(b"test")?;
			sleep(Duration::from_millis(2000));

			evh.stop()?;
			evhserver.stop()?;
		}

		Ok(())
	}

	#[test]
	fn test_evh_tls_basic_server_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_basic server error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.set_debug_tls_server_error(true);
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let mut connection = TcpStream::connect(addr)?;
			let mut buf = vec![];
			buf.resize(100, 0u8);

			// connection will close because of the error
			assert_eq!(connection.read(&mut buf)?, 0);

			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_tls_basic() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_basic Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			let mut client_handle = lock_box!(0)?;
			let client_handle_clone = client_handle.clone();

			let mut client_received_test1 = lock_box!(false)?;
			let mut server_received_test1 = lock_box!(false)?;
			let mut server_received_abc = lock_box!(false)?;
			let client_received_test1_clone = client_received_test1.clone();
			let server_received_test1_clone = server_received_test1.clone();
			let server_received_abc_clone = server_received_abc.clone();

			evh.set_on_read(move |conn_data, _thread_context, _attachment| {
				info!(
					"on read handle={},id={}",
					conn_data.get_handle(),
					conn_data.get_connection_id()
				)?;
				let first_slab = conn_data.first_slab();
				let last_slab = conn_data.last_slab();
				let slab_offset = conn_data.slab_offset();
				debug!("first_slab={}", first_slab)?;
				let res = conn_data.borrow_slab_allocator(move |sa| {
					let slab = sa.get(first_slab.try_into()?)?;
					assert_eq!(first_slab, last_slab);
					info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				})?;
				conn_data.clear_through(first_slab)?;
				let client_handle = client_handle_clone.rlock()?;
				let guard = client_handle.guard();
				if conn_data.get_handle() != **guard {
					info!("client res = {:?}", res)?;
					if res[0] == 't' as u8 {
						conn_data.write_handle().write(&res)?;
						if res == b"test1".to_vec() {
							let mut server_received_test1 = server_received_test1.wlock()?;
							(**server_received_test1.guard()) = true;
						}
					}
					if res == b"abc".to_vec() {
						let mut server_received_abc = server_received_abc.wlock()?;
						(**server_received_abc.guard()) = true;
					}
				} else {
					info!("server res = {:?})", res)?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					if res == b"test1".to_vec() {
						let mut client_received_test1 = client_received_test1.wlock()?;
						(**client_received_test1.guard()) = true;
					}
				}
				info!("res={:?}", res)?;
				Ok(())
			})?;
			evh.set_on_accept(move |conn_data, _thread_context| {
				info!(
					"accept a connection handle = {},id={}",
					conn_data.get_handle(),
					conn_data.get_connection_id()
				)?;
				Ok(())
			})?;
			evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let connection = TcpStream::connect(addr)?;
			connection.set_nonblocking(true)?;
			#[cfg(unix)]
			let connection_handle = connection.into_raw_fd();
			#[cfg(windows)]
			let connection_handle = connection.into_raw_socket().try_into()?;
			{
				let mut client_handle = client_handle.wlock()?;
				(**client_handle.guard()) = connection_handle;
			}

			let client = ClientConnection {
				handle: connection_handle,
				tls_config: Some(TlsClientConfig {
					sni_host: "localhost".to_string(),
					trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
				}),
			};
			let mut wh = evh.add_client(client, Box::new(""))?;

			wh.write(b"test1")?;
			let mut count = 0;
			loop {
				sleep(Duration::from_millis(1));
				if !(**(client_received_test1_clone.rlock()?.guard())
					&& **(server_received_test1_clone.rlock()?.guard())
					&& **(server_received_abc_clone.rlock()?.guard()))
				{
					count += 1;
					if count < 2_000 {
						continue;
					}
				}
				assert!(**(client_received_test1_clone.rlock()?.guard()));
				assert!(**(server_received_test1_clone.rlock()?.guard()));
				assert!(**(server_received_abc_clone.rlock()?.guard()));
				break;
			}

			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_tls_client_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_client_error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |conn_data, _thread_context, _attachment| {
				debug!("on read slab_offset = {}", conn_data.slab_offset())?;
				let first_slab = conn_data.first_slab();
				let last_slab = conn_data.last_slab();
				let slab_offset = conn_data.slab_offset();
				debug!("first_slab={}", first_slab)?;
				conn_data.borrow_slab_allocator(move |sa| {
					let slab = sa.get(first_slab.try_into()?)?;
					assert_eq!(first_slab, last_slab);
					info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				})?;
				conn_data.clear_through(first_slab)?;
				for _ in 0..3 {
					conn_data.write_handle().write(b"test")?;
					sleep(Duration::from_millis(1_000));
				}
				Ok(())
			})?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: None,
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let connection = TcpStream::connect(addr)?;
			connection.set_nonblocking(true)?;
			#[cfg(unix)]
			let connection_handle = connection.into_raw_fd();
			#[cfg(windows)]
			let connection_handle = connection.into_raw_socket().try_into()?;

			let client = ClientConnection {
				handle: connection_handle,
				tls_config: Some(TlsClientConfig {
					sni_host: "localhost".to_string(),
					trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
				}),
			};
			info!("client handle = {}", connection_handle)?;
			let mut wh = evh.add_client(client, Box::new(""))?;

			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(1_000));
			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(1_000));
			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(10_000));
			evh.stop()?;
		}

		Ok(())
	}

	#[test]
	fn test_evh_tls_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			// connect and send clear text. Internally an error should occur and
			// warning printed. Processing continues though.
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;
			sleep(Duration::from_millis(1000));
			connection.write(b"test")?;
			sleep(Duration::from_millis(1000));
			connection.write(b"test")?;
			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_close1() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("close1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000_000,
			read_slab_count: 30,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!(
				"accept a connection handle = {},tid={}",
				conn_data.get_handle(),
				conn_data.tid()
			)?;
			Ok(())
		})?;
		evh.set_on_close(move |conn_data, _thread_context| {
			info!(
				"on close: {}/{}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			let mut close_count = close_count.wlock()?;
			(**close_count.guard()) += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut handle = lock_box!(None)?;
		let handle_clone = handle.clone();

		sleep(Duration::from_millis(5_000));

		std::thread::spawn(move || -> Result<(), Error> {
			std::thread::sleep(Duration::from_millis(600_000));
			let handle = handle_clone.rlock()?;
			let guard = handle.guard();
			match **guard {
				Some(handle) => {
					info!("due to timeout closing handle = {}", handle)?;
					close_handle_impl(handle)?;
				}
				_ => {}
			}
			Ok(())
		});

		let total = 500;
		for i in 0..total {
			info!("loop {}", i)?;
			let mut connection = TcpStream::connect(addr)?;
			#[cfg(unix)]
			let rhandle = connection.as_raw_fd();
			#[cfg(windows)]
			let rhandle = connection.as_raw_socket();

			{
				let mut handle = handle.wlock()?;
				let guard = handle.guard();
				**guard = Some(rhandle.try_into().unwrap());
			}

			info!("loop {} connected", i)?;
			connection.write(b"test1")?;
			info!("loop {} write complete", i)?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("loop {} about to read", i)?;
			let len = connection.read(&mut buf)?;
			info!("loop {} about read complete", i)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			info!("loop {} about to read2", i)?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
			info!("loop {} complete", i)?;

			{
				let mut handle = handle.wlock()?;
				let guard = handle.guard();
				**guard = None;
			}
		}

		info!("complete")?;

		let mut count_count = 0;
		loop {
			count_count += 1;
			sleep(Duration::from_millis(1));
			let count = **((close_count_clone.rlock()?).guard());
			if count != total && count_count < 60_000 {
				info!(
					"count = {}, total = {}, will try again in a millisecond",
					count, total
				)?;
				continue;
			}
			assert_eq!((**((close_count_clone.rlock()?).guard())), total);
			break;
		}

		evh.stop()?;

		sleep(Duration::from_millis(1_000));
		info!("stop complete. Waiting for 10 seconds")?;
		sleep(Duration::from_millis(10_000));
		info!("10 seconds elapsed")?;

		Ok(())
	}

	#[test]
	fn test_evh_server_close() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("server_close Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				if slab.get()[0] == 'x' as u8 {
					Ok(vec![])
				} else {
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				}
			})?;
			conn_data.clear_through(first_slab)?;
			if res.len() > 0 {
				info!("write back data")?;
				conn_data.write_handle().write(&res)?;
			} else {
				info!("do the close")?;
				conn_data.write_handle().close()?;
				assert!(conn_data.write_handle().write(b"test").is_err());
				assert!(conn_data.write_handle().suspend().is_ok());
				assert!(conn_data.write_handle().resume().is_ok());
				assert!(conn_data.write_handle().close().is_ok());
			}
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
			Ok(())
		})?;
		evh.set_on_close(move |conn_data, _thread_context| {
			info!(
				"on close: {}/{}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			let mut close_count = close_count.wlock()?;
			(**close_count.guard()) += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
			connection.write(b"xabc")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(len, 0);

			let mut count = 0;
			loop {
				count += 1;
				sleep(Duration::from_millis(1));
				if **((close_count_clone.rlock()?).guard()) == 0 && count < 2_000 {
					continue;
				}
				assert_eq!(**((close_count_clone.rlock()?).guard()), 1);
				break;
			}
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_multi_slab_message() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("multi_slab_message Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000,
			read_slab_count: 3,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				assert_ne!(first_slab, last_slab);

				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			// test returning an error on accept. It doesn't affect processing
			Err(err!(ErrKind::Test, "test on acc err"))
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		let mut message = ['a' as u8; 1024];
		for i in 0..1024 {
			message[i] = 'a' as u8 + (i % 26) as u8;
		}
		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(2000, 0u8);
		let len = connection.read(&mut buf)?;
		assert_eq!(len, 1024);
		for i in 0..len {
			assert_eq!(buf[i], 'a' as u8 + (i % 26) as u8);
		}

		sleep(Duration::from_millis(5_000));
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_client() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler client Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 1,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(false)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read handle={}", conn_data.get_handle())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				if res[0] == 't' as u8 {
					conn_data.write_handle().write(&res)?;
					if res == b"test1" {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
				if res == b"abc".to_vec() {
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;
				}
			} else {
				let mut x = vec![];
				x.extend(b"abc");
				conn_data.write_handle().write(&x)?;
				if res == b"test1".to_vec() {
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) = true;
				}
			}
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: None,
		};
		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(b"test1")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard())
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 25_000 {
					continue;
				}
			}
			assert!(**(client_received_test1_clone.rlock()?.guard()));
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_is_reuse_port() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("is_ reuse port Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut tid0count = lock_box!(0)?;
		let mut tid1count = lock_box!(0)?;
		let tid0count_clone = tid0count.clone();
		let tid1count_clone = tid1count.clone();
		evh.set_on_accept(move |conn_data, _thread_context| {
			if conn_data.tid() == 0 {
				let mut tid0count = tid0count.wlock()?;
				let guard = tid0count.guard();
				(**guard) += 1;
			} else if conn_data.tid() == 1 {
				let mut tid1count = tid1count.wlock()?;
				let guard = tid1count.guard();
				(**guard) += 1;
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
			let mut close_count = close_count.wlock()?;
			(**close_count.guard()) += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}

		let total = 100;
		for _ in 0..total {
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}

		let mut count_count = 0;
		loop {
			count_count += 1;
			sleep(Duration::from_millis(1));
			let count = **((close_count_clone.rlock()?).guard());
			if count != total + 1 && count_count < 1_000 {
				continue;
			}
			assert_eq!((**((close_count_clone.rlock()?).guard())), total + 1);
			break;
		}

		let tid0count = **(tid0count_clone.rlock()?.guard());
		let tid1count = **(tid1count_clone.rlock()?.guard());
		info!("tid0count={},tid1count={}", tid0count, tid1count)?;
		#[cfg(target_os = "linux")]
		{
			assert_ne!(tid0count, 0);
			assert_ne!(tid1count, 0);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_stop() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("stop Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		assert!(evh.stop().is_err());

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1000));
		evh.stop()?;
		sleep(Duration::from_millis(1000));

		let mut buf = vec![];
		buf.resize(100, 0u8);
		let res = connection.read(&mut buf);

		assert!(res.is_err() || res.unwrap() == 0);

		Ok(())
	}

	#[test]
	fn test_evh_partial_clear() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("partial clear Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 40,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let mut second_slab = usize::MAX;
			let slab_offset = conn_data.slab_offset();
			let (res, second_slab) = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				info!("on_read ")?;
				loop {
					info!("loop with id={}", slab_id)?;
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					debug!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					if slab_id != last_slab {
						ret.extend(&slab_bytes[0..READ_SLAB_DATA_SIZE as usize]);
					} else {
						ret.extend(&slab_bytes[0..slab_offset as usize]);
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
					if second_slab == usize::MAX {
						info!("set secondslab to {} ", slab_id)?;
						second_slab = slab_id.try_into()?;
					}
					info!("end loop")?;
				}
				Ok((ret, second_slab))
			})?;
			info!("second_slab={}", second_slab)?;
			assert_ne!(second_slab, usize::MAX);
			conn_data.clear_through(second_slab.try_into()?)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};

		sleep(Duration::from_millis(1000));
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		let mut message = ['a' as u8; 1036];
		for i in 0..1036 {
			message[i] = 'a' as u8 + (i % 26) as u8;
		}
		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(2000, 0u8);
		let len = connection.read(&mut buf)?;
		assert_eq!(len, 1036);
		for i in 0..len {
			assert_eq!(buf[i], 'a' as u8 + (i % 26) as u8);
		}

		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(5000, 0u8);
		let len = connection.read(&mut buf)?;
		// there are some remaining bytes left in the last of the three slabs.
		// only 8 bytes so we have 8 + 1036 = 1044.
		assert_eq!(len, 1044);

		assert_eq!(buf[0], 111);
		assert_eq!(buf[1], 112);
		assert_eq!(buf[2], 113);
		assert_eq!(buf[3], 114);
		assert_eq!(buf[4], 115);
		assert_eq!(buf[5], 116);
		assert_eq!(buf[6], 117);
		assert_eq!(buf[7], 118);
		for i in 8..1044 {
			assert_eq!(buf[i], 'a' as u8 + ((i - 8) % 26) as u8);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_different_lengths1() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("different len1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 41,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					let offset = if slab_id == last_slab {
						slab_offset as usize
					} else {
						READ_SLAB_DATA_SIZE
					};
					debug!("read bytes = {:?}", &slab.get()[0..offset as usize])?;
					ret.extend(&slab_bytes[0..offset]);

					if slab_id == last_slab {
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
				}
				Ok(ret)
			})?;
			conn_data.clear_through(last_slab)?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;

		sleep(Duration::from_millis(5_000));
		let mut stream = TcpStream::connect(addr)?;

		let mut bytes = [0u8; 10240];
		for i in 0..10240 {
			bytes[i] = 'a' as u8 + i as u8 % 26;
		}

		for i in 1..2000 {
			info!("i={}", i)?;
			stream.write(&bytes[0..i])?;
			let mut buf = vec![];
			buf.resize(i + 2_000, 0u8);
			let len = stream.read(&mut buf[0..i + 2_000])?;
			assert_eq!(len, i);
			assert_eq!(&buf[0..len], &bytes[0..len]);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_different_lengths_client() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("different lengths client Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 41,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					debug!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					if slab_id != last_slab {
						ret.extend(&slab_bytes[0..READ_SLAB_DATA_SIZE as usize]);
					} else {
						ret.extend(&slab_bytes[0..slab_offset as usize]);
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
				}
				Ok(ret)
			})?;
			conn_data.clear_through(last_slab)?;
			conn_data.write_handle().write(&res)?;
			debug!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		debug!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 21,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh2 = crate::Builder::build_evh(config.clone())?;

		let expected = lock_box!("".to_string())?;
		let mut expected_clone = expected.clone();
		let (tx, rx) = sync_channel(1);

		evh2.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let last_slab = conn_data.last_slab();
			let value = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut full = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					let offset = if slab_id == last_slab {
						slab_offset as usize
					} else {
						READ_SLAB_DATA_SIZE
					};

					full.extend(&slab_bytes[0..offset]);
					if slab_id == last_slab {
						break;
					} else {
						slab_id = u32::from_be_bytes(try_into!(
							slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
						)?);
					}
				}
				Ok(full)
			})?;

			let expected = expected.rlock()?;
			let guard = expected.guard();

			if value.len() == (**guard).len() {
				assert_eq!(std::str::from_utf8(&value)?, (**guard));
				tx.send(())?;
				conn_data.clear_through(last_slab)?;
			}
			Ok(())
		})?;

		evh2.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh2.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh2.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh2.set_housekeeper(move |_thread_context| Ok(()))?;
		evh2.start()?;

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: None,
		};
		let mut wh = evh2.add_client(client, Box::new(""))?;

		let mut bytes = [0u8; 2000];
		for i in 0..2000 {
			bytes[i] = 'a' as u8 + i as u8 % 26;
		}

		for i in 1..1024 {
			let rand: usize = random();
			let rand = rand % 26;
			info!("i={},rand[0]={}", i, bytes[rand])?;
			let s = std::str::from_utf8(&bytes[rand..i + rand])?.to_string();
			{
				let mut expected = expected_clone.wlock()?;
				let guard = expected.guard();
				**guard = s;
				for j in 0..i {
					let mut b = [0u8; 1];
					b[0] = bytes[j + rand];
					wh.write(&b[0..1])?;
				}
			}

			rx.recv()?;
		}

		evh.stop()?;
		evh2.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_out_of_slabs() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("out of slabs Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 1,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = slab_offset as usize;
				ret.extend(&slab_bytes[0..offset]);
				Ok(ret)
			})?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		sleep(Duration::from_millis(100));
		// do a request that uses 2 slabs (with capacity of only one)
		let mut buf = [10u8; 600];
		stream.write(&buf)?;
		assert!(stream.read(&mut buf).is_err());

		sleep(Duration::from_millis(100));
		// now we should be able to continue with small requests

		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"posterror")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 9);
		assert_eq!(&buf[0..len], b"posterror");

		// now make a new connection and run out of slabs
		let mut stream2 = TcpStream::connect(addr)?;
		stream2.write(b"posterror")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		assert!(stream2.read(&mut buf).is_err());

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_user_data() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("user data Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, thread_context, _attachment| {
			assert_eq!(
				thread_context.user_data.downcast_ref::<String>().unwrap(),
				&"something".to_string()
			);
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = slab_offset as usize;
				ret.extend(&slab_bytes[0..offset]);
				Ok(ret)
			})?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, thread_context| {
			thread_context.user_data = Box::new("something".to_string());
			Ok(())
		})?;

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read_error() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("trigger on_read error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			conn_data.write_handle().write(b"1234")?;
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				info!("new thread")?;
				sleep(Duration::from_millis(1000));
				wh.write(b"5678")?;
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Err(err!(ErrKind::Test, "on_read test err"))
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"5678");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("trigger on_read Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			conn_data.write_handle().write(b"1234")?;
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				info!("new thread")?;
				sleep(Duration::from_millis(1000));
				wh.write(b"5679")?;
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"5679");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_oob_panic() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("oob_panic Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;

		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		info!("adding server")?;
		evh.add_server(sc, Box::new(""))?;
		info!("server added - sleep 10 secs")?;
		sleep(Duration::from_millis(10_000));
		info!("5 second sleep complete connect now")?;

		let _connection = TcpStream::connect(addr)?;
		info!("sleeping 10 seconds")?;
		sleep(Duration::from_millis(10_000));
		info!("10 seconds sleep complete")?;
		// connection should be ok because the listener is still open we just close the
		// accepted handle
		assert!(TcpStream::connect(addr).is_ok());

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_thread_panic1() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("thread_panic1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config.clone())?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				panic!("test panic");
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		let handles = create_listeners(threads, addr, 10, false)?;
		let server_handle = handles[0];

		evh.set_on_accept(move |conn_data, _thread_context| {
			assert_eq!(server_handle, conn_data.get_accept_handle().unwrap());
			Ok(())
		})?;
		evh.set_on_close(move |_, _| Ok(()))?;

		let mut on_panic_callback = lock_box!(0)?;
		let on_panic_callback_clone = on_panic_callback.clone();
		evh.set_on_panic(move |_thread_context, e| {
			let e = e.downcast_ref::<&str>().unwrap();
			info!("on panic callback: '{}'", e)?;
			let mut on_panic_callback = on_panic_callback.wlock()?;
			**(on_panic_callback.guard()) += 1;
			if **(on_panic_callback.guard()) > 1 {
				return Err(err!(ErrKind::Test, "test on_panic err"));
			}
			Ok(())
		})?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// create a thread panic
		stream.write(b"aaa")?;
		// read and we should get 0 for close
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 0);
		// connect and send another request
		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test");

		// assert that the on_panic callback was called
		assert_eq!(**on_panic_callback_clone.rlock()?.guard(), 1);
		// create a thread panic
		stream.write(b"aaa")?;
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_no_panic_handler() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("no panic_handler Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config.clone())?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				panic!("test panic");
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		let handles = create_listeners(threads, addr, 10, false)?;
		let server_handle = handles[0];

		evh.set_on_accept(move |conn_data, _thread_context| {
			assert_eq!(server_handle, conn_data.get_accept_handle().unwrap());
			Ok(())
		})?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_panic_none();
		evh.start()?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// create a thread panic
		stream.write(b"aaa")?;
		sleep(Duration::from_millis(5000));

		// read and we should get 0 for close
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 0);

		// connect and send another request
		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_thread_panic_multi() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("thread_panic_multi Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let x = lock_box!(0)?;
		let mut x_clone = x.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			} else if res[0] == 'b' as u8 {
				loop {
					sleep(Duration::from_millis(10));
					if **(x.rlock()?.guard()) != 0 {
						break;
					}
				}
				wh.write(&res)?;
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, e| {
			let e = e.downcast_ref::<&str>().unwrap();
			info!("on panic callback: '{}'", e)?;
			Ok(())
		})?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		// make 4 connections
		let mut stream1 = TcpStream::connect(addr)?;
		let mut stream2 = TcpStream::connect(addr)?;
		let mut stream3 = TcpStream::connect(addr)?;
		let mut stream4 = TcpStream::connect(addr)?;

		// do a normal request on 1
		stream1.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// do a normal request on 2
		stream2.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream2.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// do a normal request on 3
		stream3.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream3.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// pause request
		stream4.write(b"bbbb")?;
		sleep(Duration::from_millis(100));

		// normal request
		stream1.write(b"1")?;
		sleep(Duration::from_millis(100));

		// create panic
		stream2.write(b"a")?;
		sleep(Duration::from_millis(100));

		// normal request
		stream3.write(b"c")?;
		sleep(Duration::from_millis(100));

		// unblock with guard
		**(x_clone.wlock()?.guard()) = 1;

		// read responses

		// normal echo after lock lifted
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream4.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"bbbb");

		//normal echo
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(&buf[0..len], b"1");

		// panic so close len == 0
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream2.read(&mut buf)?;
		assert_eq!(len, 0);

		// normal echo
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream3.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(&buf[0..len], b"c");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_too_many_connections() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("too many connections on_read Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 30,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			wh.write(&res)?;
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();
		evh.set_on_close(move |_conn_data, _thread_context| {
			let mut close_count = close_count.wlock()?;
			**close_count.guard() += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut stream1 = TcpStream::connect(addr)?;

			// do a normal request on 1
			stream1.write(b"test")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = stream1.read(&mut buf)?;
			assert_eq!(len, 4);
			assert_eq!(&buf[0..len], b"test");

			// there are already two connections on the single thread (listener/stream).
			// so another connection will fail

			let mut stream2 = TcpStream::connect(addr)?;
			assert_eq!(stream2.read(&mut buf)?, 0);
		}

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(1));
			if **(close_count_clone.rlock()?.guard()) == 0 && count < 2_000 {
				continue;
			}
			assert_eq!(**(close_count_clone.rlock()?.guard()), 1);
			break;
		}
		info!("sleep complete")?;
		// now that stream1 is dropped we should be able to reconnect

		let mut stream1 = TcpStream::connect(addr)?;

		// do a normal request on 1
		stream1.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		Ok(())
	}

	#[test]
	fn test_evh_write_error() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("write_error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_write_error(true);

		let mut count = lock_box!(0)?;
		let count_clone = count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			if res.len() > 0 && res[0] == '1' as u8 {
				conn_data.write_handle().write(&res)?;
			}
			let mut count = count.wlock()?;
			let g = count.guard();
			**g += 1;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		stream.write(b"12345")?;
		sleep(Duration::from_millis(1_000));
		stream.write(b"0000")?;

		sleep(Duration::from_millis(10_000));
		assert_eq!(**(count_clone.rlock()?.guard()), 1);

		Ok(())
	}

	#[test]
	fn test_evh_debug_suspend() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("debug_suspend Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_suspended(true);

		let mut success = lock_box!(false)?;
		let success_clone = success.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			assert!(conn_data.write_handle().write(&res).is_err());
			info!("res={:?}", res)?;
			**(success.wlock()?.guard()) = true;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"12345")?;

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			count += 1;
			if !**(success_clone.rlock()?.guard()) && count < 25_000 {
				continue;
			}
			assert!(**(success_clone.rlock()?.guard()));

			break;
		}

		Ok(())
	}

	#[test]
	fn test_evh_debug_pending() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("pending Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_pending(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;

		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		Ok(())
	}

	#[test]
	fn test_evh_write_queue() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("test_evh_write_queue Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_write_queue(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		// this request uses the write queue
		stream.write(b"a12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(1000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"a12345");

		// this request uses the write queue
		stream.write(b"b12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(5_000));
		let len = stream.read(&mut buf)?;
		info!("read = {:?}", &buf[0..len])?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"b12345");

		// this request uses the write queue
		info!("write second time to write queue")?;
		stream.write(b"c12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(3_000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"c12345");

		// this request uses the write queue
		stream.write(b"d12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(3_000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"d12345");

		// this request uses the write queue
		stream.write(b"e12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"e12345");
		Ok(())
	}

	#[test]
	fn test_evh_housekeeper() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("housekeeper Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;

		let mut x = lock_box!(0)?;
		let x_clone = x.clone();
		evh.set_housekeeper(move |thread_context: _| {
			info!("housekeep callback")?;
			match thread_context.user_data.downcast_mut::<u64>() {
				Some(value) => {
					*value += 1;
					let mut x = x.wlock()?;
					(**x.guard()) = *value;
					info!("value={}", *value)?;
				}
				None => {
					thread_context.user_data = Box::new(0u64);
				}
			}
			Ok(())
		})?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let v = **(x_clone.rlock()?.guard());
			info!("v={}", v)?;
		}

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(100));
			{
				let v = **(x_clone.rlock()?.guard());
				info!("v={}", v)?;
				if v < 10 && count < 10_000 {
					continue;
				}
			}

			assert!((**(x_clone.rlock()?.guard())) >= 10);
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_suspend_resume() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("suspend/resume Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		let complete = lock_box!(0)?;
		let complete_clone = complete.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;

			let mut wh = conn_data.write_handle();
			let mut complete_clone = complete_clone.clone();

			if res[0] == 't' as u8 {
				spawn(move || -> Result<(), Error> {
					wh.write(b"test")?;
					let handle = wh.handle();
					wh.suspend()?;
					sleep(Duration::from_millis(1_000));

					#[cfg(unix)]
					let mut strm = unsafe { TcpStream::from_raw_fd(handle) };
					#[cfg(windows)]
					let mut strm = unsafe { TcpStream::from_raw_socket(u64!(handle)) };

					let mut count = 0;
					loop {
						sleep(Duration::from_millis(1_000));
						strm.write(b"ok")?;
						if count > 3 {
							break;
						}
						count += 1;
					}

					let mut buf = vec![];
					buf.resize(100, 0u8);
					let len = strm.read(&mut buf)?;
					info!("read = {}", std::str::from_utf8(&buf[0..len]).unwrap())?;
					assert_eq!(std::str::from_utf8(&buf[0..len]).unwrap(), "next");

					wh.resume()?;
					info!("resume complete")?;

					#[cfg(unix)]
					strm.into_raw_fd();
					#[cfg(windows)]
					strm.into_raw_socket();

					let mut complete = complete_clone.wlock()?;
					**complete.guard() = 1;

					Ok(())
				});
			} else {
				wh.write(&res)?;
			}

			let response = std::str::from_utf8(&res)?;
			info!("res={:?}", response)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;

			sleep(Duration::from_millis(10_000));
			connection.write(b"next")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;

			let response = std::str::from_utf8(&buf[0..len])?;
			info!("buf={:?}", response)?;
			assert_eq!(response, "testokokokokok");

			sleep(Duration::from_millis(1_000));
			connection.write(b"resume")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			let response = std::str::from_utf8(&buf[0..len])?;
			info!("final={:?}", response)?;
			assert_eq!(response, "resume");

			let mut count = 0;
			loop {
				count += 1;
				sleep(Duration::from_millis(1));
				if **(complete.rlock()?.guard()) != 1 && count < 2_000 {
					continue;
				}

				assert_eq!(**(complete.rlock()?.guard()), 1);
				break;
			}
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_arc_to_ptr() -> Result<(), Error> {
		let x = Arc::new(10u32);

		let ptr = Arc::into_raw(x);
		let ptr_as_usize = ptr as usize;
		info!("ptr_as_usize = {}", ptr_as_usize)?;
		let ptr_ret = unsafe { Arc::from_raw(ptr_as_usize as *mut u32) };
		info!("ptr_val={:?}", ptr_ret)?;

		Ok(())
	}

	#[test]
	fn test_debug_listener_info() -> Result<(), Error> {
		let li = ListenerInfo {
			id: 0,
			handle: 0,
			is_reuse_port: false,
			tls_config: None,
		};
		info!("li={:?}", li)?;
		assert_eq!(li.id, 0);
		Ok(())
	}

	fn compare_ci(ci1: ConnectionInfo, ci2: ConnectionInfo) -> Result<(), Error> {
		match ci1 {
			ConnectionInfo::ListenerInfo(li1) => match ci2 {
				ConnectionInfo::ListenerInfo(li2) => {
					assert_eq!(li1.id, li2.id);
					assert_eq!(li1.handle, li2.handle);
					assert_eq!(li1.is_reuse_port, li2.is_reuse_port);
				}
				ConnectionInfo::StreamInfo(_rwi) => return Err(err!(ErrKind::IllegalArgument, "")),
			},
			ConnectionInfo::StreamInfo(rwi1) => match ci2 {
				ConnectionInfo::ListenerInfo(_li) => {
					return Err(err!(ErrKind::IllegalArgument, ""));
				}
				ConnectionInfo::StreamInfo(rwi2) => {
					assert_eq!(rwi1.id, rwi2.id);
					assert_eq!(rwi1.handle, rwi2.handle);
					assert_eq!(rwi1.accept_handle, rwi2.accept_handle);
					assert_eq!(rwi1.first_slab, rwi2.first_slab);
					assert_eq!(rwi1.last_slab, rwi2.last_slab);
					assert_eq!(rwi1.slab_offset, rwi2.slab_offset);
					assert_eq!(rwi1.is_accepted, rwi2.is_accepted);
				}
			},
		}
		Ok(())
	}

	#[test]
	fn test_connection_info_serialization() -> Result<(), Error> {
		let mut hashtable = hashtable!()?;
		let ci1 = ConnectionInfo::ListenerInfo(ListenerInfo {
			id: 7,
			handle: 8,
			is_reuse_port: false,
			tls_config: None,
		});
		hashtable.insert(&0, &ci1)?;
		let v = hashtable.get(&0)?.unwrap();
		compare_ci(v, ci1.clone())?;

		let ser_out = ConnectionInfo::ListenerInfo(ListenerInfo {
			id: 10,
			handle: 80,
			is_reuse_port: true,
			tls_config: None,
		});
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		v[0] = 2; // corrupt data
		let ser_in: Result<ConnectionInfo, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let ci2 = ConnectionInfo::StreamInfo(StreamInfo {
			accept_handle: None,
			accept_id: None,
			id: 0,
			handle: 0,
			first_slab: 0,
			last_slab: 0,
			slab_offset: 0,
			is_accepted: true,
			tls_client: None,
			tls_server: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0
			})?,
		});
		hashtable.insert(&0, &ci2)?;
		let v = hashtable.get(&0)?.unwrap();
		compare_ci(v, ci2.clone())?;

		assert!(compare_ci(ci1.clone(), ci2.clone()).is_err());
		assert!(compare_ci(ci2, ci1).is_err());

		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk_reuse_port_acc_err() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler tls_multi_chunk no reuse port acc err Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Err(err!(ErrKind::Test, "acc err")))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk_no_reuse_port() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler tls_multi_chunk no reuse port Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_accept_none();

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler tls_multi_chunk Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_accept_none();

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_bad_configs() -> Result<(), Error> {
		let mut evhs = vec![];
		evhs.push(EventHandlerImpl::new(EventHandlerConfig {
			read_slab_count: u32::MAX as usize,
			..Default::default()
		}));
		evhs.push(EventHandlerImpl::new(EventHandlerConfig {
			read_slab_count: 100 as usize,
			..Default::default()
		}));

		for i in 0..2 {
			if i == 0 {
				assert!(evhs[i].is_err());
			} else {
				let evh = evhs[i].as_mut().unwrap();
				evh.set_on_read(move |_, _, _| Ok(()))?;
				evh.set_on_accept(move |_, _| Ok(()))?;
				evh.set_on_close(move |_, _| Ok(()))?;
				evh.set_housekeeper(move |_| Ok(()))?;
				evh.set_on_panic(move |_, _| Ok(()))?;
			}
		}
		Ok(())
	}

	#[test]
	fn test_bad_keys() -> Result<(), Error> {
		// it's empty so it would be an error
		assert!(load_private_key("./resources/emptykey.pem").is_err());

		// key is ok to load but signing won't work
		assert!(load_private_key("./resources/badkey.pem").is_ok());

		// rsa
		assert!(load_private_key("./resources/rsa.pem").is_ok());

		// eckey
		assert!(load_private_key("./resources/ec256.pem").is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_panic_fatal() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("panic_fatal Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res.len() > 0 && res[0] == '1' as u8 {
				panic!("test start with '1'");
			}

			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"1panic")?;
		sleep(Duration::from_millis(1_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test2");

		connection.write(b"0test1")?;
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_other_situations() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("other_situations Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;

			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test2");

		connection.write(b"0test1")?;
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_other_panics_wo_err() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("test_evh_other_panics_wo_err Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;
		let mut close_count = lock_box!(0)?;
		let mut housekeeper_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
			let count = {
				let mut close_count = close_count.wlock()?;
				let count = **close_count.guard();
				**close_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on close panic");
			}
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| {
			let count = {
				let mut housekeeper_count = housekeeper_count.wlock()?;
				let count = **housekeeper_count.guard();
				**housekeeper_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on housekeeper panic");
			}
			Ok(())
		})?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let _connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1_000));
		// listener should be closed so this will fail
		assert!(TcpStream::connect(addr).is_err());

		let port = pick_free_port()?;
		info!("other_situations2 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let _connection = TcpStream::connect(addr)?;
		}
		sleep(Duration::from_millis(5_000));

		// last connection on close handler panics, but we should be able to still send
		// requests.
		{
			let mut connection = TcpStream::connect(addr)?;
			info!("about to write")?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("about to read")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			info!("read back buf[{}] = {:?}", len, buf)?;
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}
		sleep(Duration::from_millis(1_000));

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_other_panics_w_err() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("test_evh_other_panics_w_err Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;
		let mut close_count = lock_box!(0)?;
		let mut housekeeper_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
			let count = {
				let mut close_count = close_count.wlock()?;
				let count = **close_count.guard();
				**close_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on close panic");
			}
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Err(err!(ErrKind::Test, "")))?;
		evh.set_housekeeper(move |_thread_context| {
			let count = {
				let mut housekeeper_count = housekeeper_count.wlock()?;
				let count = **housekeeper_count.guard();
				**housekeeper_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on housekeeper panic");
			}
			Ok(())
		})?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let _connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1_000));
		// listener should be closed so this will fail
		assert!(TcpStream::connect(addr).is_err());

		let port = pick_free_port()?;
		info!("other_situations2 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let _connection = TcpStream::connect(addr)?;
		}
		sleep(Duration::from_millis(5_000));

		// last connection on close handler panics, but we should be able to still send
		// requests.
		{
			let mut connection = TcpStream::connect(addr)?;
			info!("about to write")?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("about to read")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			info!("read back buf[{}] = {:?}", len, buf)?;
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}
		sleep(Duration::from_millis(1_000));

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_housekeeper_error() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Err(err!(ErrKind::Test, "")))?;

		evh.start()?;
		sleep(Duration::from_millis(10_000));
		assert!(evh.stop().is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_invalid_write_queue() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		// enqueue an invalid handle. the function should just print the warning and still
		// succeed
		{
			let mut data = evh.data[0].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).write_queue.enqueue(100)?;
		}
		evh.process_write_queue(&mut ctx)?;

		// insert the listener. again an error should be printed but processing continue
		let li = ListenerInfo {
			id: 1_000,
			handle: 0,
			is_reuse_port: false,
			tls_config: None,
		};
		let ci = ConnectionInfo::ListenerInfo(li.clone());
		ctx.connection_hashtable.insert(&1_000, &ci)?;
		{
			let mut data = evh.data[0].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).write_queue.enqueue(1_000)?;
		}
		evh.process_write_queue(&mut ctx)?;

		Ok(())
	}

	#[test]
	fn test_evh_close_no_handler() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		// insert the rwi
		let mut rwi = StreamInfo {
			id: 1_000,
			handle: 0,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.connection_hashtable.insert(&1_000, &ci)?;

		// call on close to trigger the none on close. No error should return.
		evh.process_close(&mut ctx, &mut rwi, &mut ThreadContext::new())?;
		Ok(())
	}

	#[test]
	fn test_evh_on_read_none() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler on_read_none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_read_none();
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));
		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"test")?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read_none() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler trigger_on_read_none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_read_none();

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		sleep(Duration::from_millis(1_000));

		let connection2 = TcpStream::connect(addr)?;
		connection2.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection2.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection2.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: None,
		};
		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.trigger_on_read()?;
		sleep(Duration::from_millis(1_000));

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_ins_hashtable_err() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler test_evh_ins_hashtable_err Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let port2 = pick_free_port()?;
		let addr2 = &format!("127.0.0.1:{}", port2)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 1,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		evh.add_client(client, Box::new(""))?;
		sleep(Duration::from_millis(1_000));
		let handles = create_listeners(threads, addr2, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		// connection 2 closed so this will fail
		assert!(TcpStream::connect(addr2).is_err());

		Ok(())
	}

	#[test]
	fn test_evh_debug_read_error() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("debug debug_read_error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("debug read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_debug_read_error(true);

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		info!("write ok")?;
		let res = connection.read(&mut buf);
		assert!(res.is_err() || res.unwrap() == 0);
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_process_events_other_situations() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut wakeup = Wakeup::new()?;
		ctx.counter = 0;
		ctx.count = 1;
		ctx.events[0] = Event {
			handle: 1000,
			etype: EventType::Read,
		};

		// both of these will succeed with warning printed
		// TODO: would be good to do an assertion that verifies these
		evh.process_events(&mut ctx, &mut wakeup, &mut ThreadContext::new())?;
		ctx.handle_hashtable.insert(&1000, &2000)?;
		ctx.counter = 0;
		ctx.count = 1;
		evh.process_events(&mut ctx, &mut wakeup, &mut ThreadContext::new())?;

		Ok(())
	}

	#[test]
	fn test_evh_process_write_errors() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read2() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler trigger_on_read2 none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut on_read_count = lock_box!(0)?;
		let on_read_count_clone = on_read_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("in on read")?;
			let mut wh = conn_data.write_handle();
			assert!(wh.write_state().is_ok());
			let mut on_read_count = on_read_count.wlock()?;
			let guard = on_read_count.guard();
			**guard += 1;

			// only trigger on on read for the first request
			if **guard == 1 {
				info!("about to spawn thread")?;

				spawn(move || -> Result<(), Error> {
					info!("spawned thread")?;
					sleep(Duration::from_millis(1000));
					info!("trigger on read")?;
					wh.trigger_on_read()?;
					Ok(())
				});
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));
		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"test1")?;

		let mut count = 0;
		loop {
			{
				let on_read_count_clone = on_read_count_clone.rlock()?;
				let guard = on_read_count_clone.guard();
				if **guard == 2 || count > 10_000 {
					break;
				} else {
					info!("sleep {}", count)?;
					count += 1;
				}
			}
			sleep(Duration::from_millis(5));
		}

		sleep(Duration::from_millis(1_000));

		let on_read_count_clone = on_read_count_clone.rlock()?;
		let guard = on_read_count_clone.guard();
		assert_eq!(**guard, 2);

		Ok(())
	}

	#[test]
	fn test_evh_process_accept_no_attachment() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler test_evh_process_accept_no_attachment Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut on_read_count = lock_box!(0)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("in on read")?;
			let mut wh = conn_data.write_handle();
			assert!(wh.write_state().is_ok());
			let mut on_read_count = on_read_count.wlock()?;
			let guard = on_read_count.guard();
			**guard += 1;

			// only trigger on on read for the first request
			if **guard == 1 {
				info!("about to spawn thread")?;

				spawn(move || -> Result<(), Error> {
					info!("spawned thread")?;
					sleep(Duration::from_millis(1000));
					info!("trigger on read")?;
					wh.trigger_on_read()?;
					Ok(())
				});
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let mut ctx = EventHandlerContext::new(0, 10, 10, 10, 100)?;
		let mut tc = ThreadContext::new();
		let li = ListenerInfo {
			handle: 0,
			id: 0,
			is_reuse_port: false,
			tls_config: None,
		};
		let ret = evh.process_accept(&li, &mut ctx, &mut tc)?;

		#[cfg(unix)]
		assert_eq!(ret, -1);
		#[cfg(windows)]
		assert_eq!(ret, usize::MAX);

		info!("about to call create listeners")?;
		// try with an actual socket
		let handles = create_listeners(threads, addr, 1, false)?;
		let li = ListenerInfo {
			handle: handles[0],
			id: 1,
			is_reuse_port: false,
			tls_config: None,
		};
		ctx.debug_bypass_acc_err = true;

		info!("about to call process_accept")?;
		let ret = evh.process_accept(&li, &mut ctx, &mut tc)?;
		info!("ret={}", ret)?;

		#[cfg(unix)]
		assert_eq!(ret, -1);
		#[cfg(windows)]
		assert_eq!(ret, usize::MAX);

		Ok(())
	}

	#[test]
	fn test_evh_make_config() -> Result<(), Error> {
		assert!(make_config(None).is_ok());
		Ok(())
	}

	#[test]
	fn test_evh_debug_rw_accept_id_none() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler test_evh_debug_rw_accept_id_none Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_rw_accept_id_none();

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			info!("in on read: attachment.is_some()={}", attachment.is_some())?;
			let mut wh = conn_data.write_handle();
			if attachment.is_none() {
				wh.write(b"none")?;
			} else {
				wh.write(b"some")?;
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"none");
		info!("read back buf[{}] = {:?}", len, buf)?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_debug_attachment_none() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!(
			"eventhandler test_evh_debug_attachment_none none Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_attachment_none();

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			info!("in on read: attachment.is_some()={}", attachment.is_some())?;
			let mut wh = conn_data.write_handle();
			if attachment.is_none() {
				wh.write(b"none")?;
			} else {
				wh.write(b"some")?;
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"none");
		info!("read back buf[{}] = {:?}", len, buf)?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		evh.stop()?;

		evh.stop()?;

		Ok(())
	}
}
