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

use crate::types::{
	AttachmentHolder, CloseHandle, ConnectionInfo, Event, EventHandlerContext, EventHandlerData,
	EventHandlerImpl, EventIn, EventType, EventTypeIn, Handle, LastProcessType, ListenerInfo,
	StreamInfo, Wakeup, WriteState,
};
use crate::{
	ClientConnection, ConnData, ConnectionData, EventHandler, EventHandlerConfig,
	EventHandlerController, ServerConnection, ThreadContext, WriteHandle,
};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::rand::random;
use bmw_deps::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use bmw_deps::rustls::{
	ClientConfig, ClientConnection as RCConn, RootCertStore, ServerConfig,
	ServerConnection as RSConn,
};
use bmw_deps::rustls_pemfile;
use bmw_deps::webpki_roots::TLS_SERVER_ROOTS;
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

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
/// pointer to the next slab in the list. This is why READ_SLAB_SIZE is 4 bytes greater than
/// READ_SLAB_DATA_SIZE (518 bytes).
pub const READ_SLAB_DATA_SIZE: usize = 514;

pub const READ_SLAB_SIZE: usize = 518;
pub const READ_SLAB_NEXT_OFFSET: usize = 514;

pub(crate) const HANDLE_SLAB_SIZE: usize = 42;
pub(crate) const CONNECTION_SLAB_SIZE: usize = 98;
#[cfg(target_os = "windows")]
pub(crate) const WRITE_SET_SIZE: usize = 42;

pub(crate) const WRITE_STATE_FLAG_PENDING: u8 = 0x1 << 0;
pub(crate) const WRITE_STATE_FLAG_CLOSE: u8 = 0x1 << 1;
pub(crate) const WRITE_STATE_FLAG_TRIGGER_ON_READ: u8 = 0x1 << 2;
pub(crate) const WRITE_STATE_FLAG_SUSPEND: u8 = 0x1 << 3;
pub(crate) const WRITE_STATE_FLAG_RESUME: u8 = 0x1 << 4;
pub(crate) const WRITE_STATE_FLAG_ASYNC: u8 = 0x1 << 5;

pub(crate) const EAGAIN: i32 = 11;
pub(crate) const ETEMPUNAVAILABLE: i32 = 35;
pub(crate) const WINNONBLOCKING: i32 = 10035;

pub(crate) const TLS_CHUNKS: usize = 3_072;
pub(crate) const MAX_WRITE_CHUNK_SIZE: usize = 1_000;

info!();

/// Close a handle (platform independant)
pub fn close_handle(handle: Handle) -> Result<(), Error> {
	close_handle_impl(handle)
}

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

pub fn tcp_stream_to_handle(strm: TcpStream) -> Result<Handle, Error> {
	strm.set_nonblocking(true)?;
	#[cfg(unix)]
	let connection_handle = strm.into_raw_fd();
	#[cfg(windows)]
	let connection_handle = strm.into_raw_socket().try_into()?;
	Ok(connection_handle)
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

			// note that tx can be None below because we only use tx with Queues which
			// are not serialized. ready must be true here because we're always ready
			// once it's serialized.
			let li = ListenerInfo {
				id,
				handle,
				is_reuse_port,
				tls_config,
				tx: None,
				ready: lock_box!(true)?,
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

			// note that tx can be None below because we only use tx with Queues which
			// are not serialized
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
				tx: None,
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

impl StreamInfo {
	pub(crate) fn clear_through_impl(
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
	pub fn new() -> Self {
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
	pub(crate) fn set_flag(&mut self, flag: u8) {
		self.flags |= flag;
	}

	pub(crate) fn unset_flag(&mut self, flag: u8) {
		self.flags &= !flag;
	}

	pub(crate) fn is_set(&self, flag: u8) -> bool {
		self.flags & flag != 0
	}

	pub fn set_async(&mut self, value: bool) {
		match value {
			true => self.set_flag(WRITE_STATE_FLAG_ASYNC),
			false => self.unset_flag(WRITE_STATE_FLAG_ASYNC),
		}
	}

	pub fn is_async(&self) -> bool {
		self.is_set(WRITE_STATE_FLAG_ASYNC)
	}
}

impl WriteHandle {
	pub(crate) fn new(
		handle: Handle,
		id: u128,
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

	/// Return true if the [`crate::WriteHandle`] is closed. Otherwise return false.
	pub fn is_closed(&self) -> Result<bool, Error> {
		let write_state = self.write_state.rlock()?;
		let guard = write_state.guard();
		Ok((**guard).is_set(WRITE_STATE_FLAG_CLOSE))
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
			(**guard).wakeup.wakeup()?;
		}
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
			(**guard).wakeup.wakeup()?;
		}
		Ok(())
	}

	/// Close the connection associated with this [`crate::WriteHandle`].
	pub fn close(&mut self) -> Result<(), Error> {
		handle_close(&mut self.write_state, self.id, &mut self.event_handler_data)
	}

	/// Write data to the connection associated with this [`crate::WriteHandle`].
	pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
		let mut start = 0;
		let mut end = MAX_WRITE_CHUNK_SIZE;
		let len = data.len();

		loop {
			if end >= len {
				self.chunk_write(&data[start..])?;
				break;
			}

			self.chunk_write(&data[start..end])?;

			start += MAX_WRITE_CHUNK_SIZE;
			end += MAX_WRITE_CHUNK_SIZE;
		}

		Ok(())
	}

	fn chunk_write(&mut self, data: &[u8]) -> Result<(), Error> {
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
			(**guard).wakeup.wakeup()?;
		}
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
			(**guard).wakeup.wakeup()?;
		}

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
	pub(crate) fn new(
		rwi: &'a mut StreamInfo,
		tid: usize,
		slabs: &'a mut Box<dyn SlabAllocator + Send + Sync>,
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
	pub(crate) fn new(
		write_queue_size: usize,
		nhandles_queue_size: usize,
		wakeup: Wakeup,
		debug_pending: bool,
		debug_suspended: bool,
		debug_write_error: bool,
		debug_write_queue: bool,
	) -> Result<Self, Error> {
		let connection_info = ConnectionInfo::ListenerInfo(ListenerInfo {
			handle: 0,
			id: 0,
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		});

		let evhd = EventHandlerData {
			write_queue: queue_sync_box!(write_queue_size, &0)?,
			nhandles: queue_sync_box!(nhandles_queue_size, &connection_info)?,
			stop: false,
			stopped: false,
			attachments: HashMap::new(),
			wakeup,
			debug_pending,
			debug_suspended,
			debug_write_error,
			debug_write_queue,
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
		let mut data = array!(
			config.threads,
			&lock_box!(EventHandlerData::new(
				1,
				1,
				Wakeup::new()?,
				false,
				false,
				false,
				false
			)?)?
		)?;
		for i in 0..config.threads {
			data[i] = lock_box!(EventHandlerData::new(
				config.write_queue_size,
				config.nhandles_queue_size,
				Wakeup::new()?,
				false,
				false,
				false,
				false
			)?)?;
		}

		let ret = Self {
			on_read: None,
			on_accept: None,
			on_close: None,
			housekeeper: None,
			on_panic: None,
			config,
			data,
			thread_pool_stopper: lock_box!(None)?,
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
			debug_close_handle: false,
		};
		Ok(ret)
	}

	#[cfg(test)]
	pub(crate) fn set_debug_close_handle(&mut self, value: bool) {
		self.debug_close_handle = value;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_write_queue(&mut self, value: bool) {
		self.debug_write_queue = value;
		for i in 0..self.data.size() {
			// unwrap ok in tests
			let mut data = self.data[i].wlock_ignore_poison().unwrap();
			let guard = data.guard();
			(**guard).debug_write_queue = value;
		}
	}

	#[cfg(test)]
	pub(crate) fn set_debug_pending(&mut self, value: bool) {
		self.debug_pending = value;
		for i in 0..self.data.size() {
			// unwrap ok in tests
			let mut data = self.data[i].wlock_ignore_poison().unwrap();
			let guard = data.guard();
			(**guard).debug_pending = value;
		}
	}

	#[cfg(test)]
	pub(crate) fn set_debug_write_error(&mut self, value: bool) {
		self.debug_write_error = value;
		for i in 0..self.data.size() {
			// unwrap ok in tests
			let mut data = self.data[i].wlock_ignore_poison().unwrap();
			let guard = data.guard();
			(**guard).debug_write_error = value;
		}
	}

	#[cfg(test)]
	pub(crate) fn set_debug_suspended(&mut self, value: bool) {
		self.debug_suspended = value;
		for i in 0..self.data.size() {
			// unwrap ok in tests
			let mut data = self.data[i].wlock_ignore_poison().unwrap();
			let guard = data.guard();
			(**guard).debug_suspended = value;
		}
	}

	#[cfg(test)]
	pub(crate) fn set_debug_read_error(&mut self, value: bool) {
		self.debug_read_error = value;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_fatal_error(&mut self, value: bool) {
		self.debug_fatal_error = value;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_tls_server_error(&mut self, value: bool) {
		self.debug_tls_server_error = value;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_tls_read(&mut self, value: bool) {
		self.debug_tls_read = value;
	}

	#[cfg(test)]
	pub(crate) fn set_on_panic_none(&mut self) {
		self.housekeeper = None;
		self.on_panic = None;
	}

	#[cfg(test)]
	pub(crate) fn set_on_read_none(&mut self) {
		self.on_read = None;
	}

	#[cfg(test)]
	pub(crate) fn set_attachment_none(&mut self) {
		self.debug_attachment_none = true;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_rw_accept_id_none(&mut self) {
		self.debug_rw_accept_id_none = true;
	}

	#[cfg(test)]
	pub(crate) fn set_on_accept_none(&mut self) {
		self.on_accept = None;
	}

	#[cfg(test)]
	pub(crate) fn set_on_close_none(&mut self) {
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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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
						let lock = rw.write_state.wlock();
						if lock.is_ok() && !self.debug_close_handle {
							let mut state = lock.unwrap();
							let guard = state.guard();
							(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
						} else {
							let _ = warn!("rw.write_state.wlock generated error");
						}
					}

					let _ = rw.clear_through_impl(rw.last_slab, &mut ctx.read_slabs);
					let _ = close_handle_impl(rw.handle);
				}
			}
		}

		let v = ctx.handle_hashtable.clear();
		if v.is_err() || self.debug_close_handle {
			let _ = warn!("handle_hashtable.clear generated error: {:?}", v);
		}

		let v = ctx.connection_hashtable.clear();
		if v.is_err() || self.debug_close_handle {
			let _ = warn!("connetion_hashtable.clear generated error: {:?}", v);
		}
		debug!("handles closed")?;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub(crate) fn process_write_queue(
		&mut self,
		ctx: &mut EventHandlerContext,
	) -> Result<(), Error> {
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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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
			let attachment: Option<AttachmentHolder>;
			let mut tx_to_send: Option<SyncSender<()>> = None;
			if tx_to_send.is_some() {} // supress compiler warning
			let id;

			let mut next = (**guard).nhandles.dequeue();
			match next {
				Some(ref mut nhandle) => {
					debug!("dequeue handle={:?} on tid={}", nhandle, ctx.tid)?;
					match nhandle {
						ConnectionInfo::ListenerInfo(li) => {
							tx_to_send = li.tx.clone();
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
							debug!(
								"rwi found id={},tid={},handle={}",
								rwi.id, ctx.tid, rwi.handle
							)?;
							match &mut self.on_accept {
								Some(on_accept) => {
									let tid = ctx.tid;
									let rslabs = &mut ctx.read_slabs;
									let data = data2[tid].clone();
									let q = self.debug_write_queue;
									let p = self.debug_pending;
									let we = self.debug_write_error;
									let s = self.debug_suspended;
									let mut rwi = rwi.clone();
									let handle = rwi.handle;
									let mut cd = ConnectionData::new(
										&mut rwi, tid, rslabs, data, q, p, we, s,
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

							tx_to_send = rwi.tx.clone();

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
							attachment = (**guard).attachments.remove(&id);
							debug!("att.is_none = {}", attachment.is_none())?;
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

			match tx_to_send {
				Some(tx) => {
					tx.send(())?;
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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub(crate) fn process_events(
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
	pub(crate) fn process_write(
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

					let mut conn_data = ConnectionData::new(
						rw,
						ctx.tid,
						&mut ctx.read_slabs,
						self.data[ctx.tid].clone(),
						self.debug_write_queue,
						self.debug_pending,
						self.debug_write_error,
						self.debug_suspended,
					);

					debug!("about to try to do a read")?;
					if !conn_data.write_handle().is_closed()? {
						match on_read(&mut conn_data, callback_context, attachment) {
							Ok(_) => {}
							Err(e) => {
								warn!("Callback on_read generated error: {}", e)?;
							}
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
			let end = len.try_into().unwrap_or(0);
			(**tls_conn).read_tls(&mut &ctx.buffer[0..end])?;

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
			let data = self.data[ctx.tid].clone();
			let d1 = self.debug_write_queue;
			let d2 = self.debug_pending;
			let d3 = self.debug_write_error;
			let d4 = self.debug_suspended;
			let connection_data = ConnectionData::new(rw, tid, rs, data, d1, d2, d3, d4);
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
			let data = self.data[ctx.tid].clone();
			let d1 = self.debug_write_queue;
			let d2 = self.debug_pending;
			let d3 = self.debug_write_error;
			let d4 = self.debug_suspended;
			let connection_data = ConnectionData::new(rw, tid, rs, data, d1, d2, d3, d4);
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
					let mut conn_data = ConnectionData::new(
						rw,
						ctx.tid,
						&mut ctx.read_slabs,
						self.data[ctx.tid].clone(),
						self.debug_write_queue,
						self.debug_pending,
						self.debug_write_error,
						self.debug_suspended,
					);

					debug!("about to try a read")?;
					if !conn_data.write_handle().is_closed()? {
						match on_read(&mut conn_data, callback_context, attachment) {
							Ok(_) => {}
							Err(e) => {
								warn!("Callback on_read generated error: {}", e)?;
							}
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
	pub(crate) fn process_close(
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

		rw.tls_server = None;
		rw.tls_client = None;

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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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
	pub(crate) fn process_accept(
		&mut self,
		li: &ListenerInfo,
		ctx: &mut EventHandlerContext,
		callback_context: &mut ThreadContext,
	) -> Result<Handle, Error> {
		// wait until we're ready (up to 10ms).
		let mut count = 0;
		loop {
			if rlock!(li.ready) || count >= 10 {
				break;
			}
			count += 1;
			sleep(Duration::from_millis(1));
		}
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
			tx: None,
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
				let acc_id = rwi.accept_id.unwrap();
				match ctx.attachments.get(&acc_id) {
					Some(attachment) => {
						(**guard).attachments.insert(id, attachment.clone());
					}
					None => {}
				};

				let ci = ConnectionInfo::StreamInfo(rwi);
				(**guard).nhandles.enqueue(ci)?;
				(**guard).wakeup.wakeup()?;
			}

			debug!("wakeup called on tid = {}", tid)?;
		}
		Ok(handle)
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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
				let data = self.data[ctx.tid].clone();
				let wq = self.debug_write_queue;
				let p = self.debug_pending;
				let we = self.debug_write_error;
				let s = self.debug_suspended;
				let mut cd = ConnectionData::new(rwi, tid, rslabs, data, wq, p, we, s);
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

	fn stop(&mut self) -> Result<(), Error> {
		let mut evhctlr = self.event_handler_controller()?;
		evhctlr.stop()
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
				self.data[i].clone(),
				ctx.clone(),
				thread_context.clone(),
			));
			v_panic.push((evh, self.data[i].clone(), ctx, thread_context));
		}

		let mut executor = lock_box!(tp.executor()?)?;
		let mut executor_clone = executor.clone();

		let on_panic = self.on_panic.clone();

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			let id: usize = id.try_into()?;
			let mut evh = v_panic[id].0.clone();
			let mut wakeup = {
				let mut data = v_panic[id].1.wlock_ignore_poison()?;
				let guard = data.guard();
				(**guard).wakeup.clone()
			};
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
					let ctx = &mut *ctx;
					let thc = &mut *thread_context;
					let isr = true;

					Self::execute_thread(evh, &mut wakeup, ctx, thc, isr)
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
			//let mut wakeup = v[i].1.clone();
			let mut wakeup = {
				let mut data = v[i].1.wlock_ignore_poison()?;
				let guard = data.guard();
				(**guard).wakeup.clone()
			};
			let mut ctx = v[i].2.clone();
			let mut thread_context = v[i].3.clone();

			execute!(tp, i.try_into()?, {
				let mut ctx = ctx.wlock_ignore_poison()?;
				let ctx = ctx.guard();

				let mut thread_context = thread_context.wlock_ignore_poison()?;
				let thread_context = thread_context.guard();

				let evh = &mut evh;
				let ctx = &mut *ctx;
				let thc = &mut *thread_context;
				let isr = false;
				Self::execute_thread(evh, &mut wakeup, ctx, thc, isr)
			})?;
		}

		let mut thread_pool_stopper = self.thread_pool_stopper.wlock()?;
		let thread_pool_stopper = thread_pool_stopper.guard();
		(**thread_pool_stopper) = Some(tp.stopper()?);

		Ok(())
	}

	fn add_client(
		&mut self,
		connection: ClientConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<WriteHandle, Error> {
		let mut evhctlr = self.event_handler_controller()?;
		evhctlr.add_client(connection, attachment)
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn add_server(
		&mut self,
		connection: ServerConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<(), Error> {
		let attachment = AttachmentHolder {
			attachment: lock_box!(attachment)?,
		};
		debug!("add server: {:?}", attachment)?;

		let tls_config = match connection.tls_config {
			Some(tls_config) => Some(Arc::new(
				ServerConfig::builder()
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

		let mut ready = lock_box!(false)?;

		for i in 0..connection.handles.size() {
			let handle = connection.handles[i];
			// check for 0 which means to skip this handle (port not reused)
			if handle != 0 {
				let (tx, rx) = sync_channel::<()>(1);
				{
					let mut data = self.data[i].wlock_ignore_poison()?;
					let guard = data.guard();
					let id = random();
					let li = ListenerInfo {
						id,
						handle,
						is_reuse_port: connection.is_reuse_port,
						tls_config: tls_config.clone(),
						tx: Some(tx),
						ready: ready.clone(),
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
					(**guard).wakeup.wakeup()?;
				}
				rx.recv()?;
			}
		}

		wlock!(ready) = true;
		Ok(())
	}

	fn event_handler_data(&self) -> Result<Array<Box<dyn LockBox<EventHandlerData>>>, Error> {
		Ok(self.data.clone())
	}

	fn event_handler_controller(&self) -> Result<EventHandlerController, Error> {
		Ok(EventHandlerController {
			data: self.data.clone(),
			thread_pool_stopper: self.thread_pool_stopper.clone(),
		})
	}
}

impl EventHandlerController {
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub fn stop(&mut self) -> Result<(), Error> {
		let mut thread_pool_stopper = self.thread_pool_stopper.wlock()?;
		let thread_pool_stopper = thread_pool_stopper.guard();
		if (**thread_pool_stopper).is_none() {
			let err = err!(ErrKind::IllegalState, "start must be called before stop");
			return Err(err);
		}
		for i in 0..self.data.size() {
			let mut data = self.data[i].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).stop = true;
			(**guard).wakeup.wakeup()?;
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

		(**thread_pool_stopper).as_mut().unwrap().stop()?;

		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	pub fn add_client(
		&mut self,
		connection: ClientConnection,
		attachment: Box<dyn Any + Send + Sync>,
	) -> Result<WriteHandle, Error> {
		let attachment = AttachmentHolder {
			attachment: lock_box!(attachment)?,
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
				let config = make_config(tls_config.trusted_cert_full_chain_file)?;
				let mut rc_conn = RCConn::new(config, tls_config.sni_host.try_into()?)?;
				rc_conn.set_buffer_limit(None);
				let tls_client = Some(lock_box!(rc_conn)?);
				tls_client
			}
			None => None,
		};

		let (tx, rx) = sync_channel::<()>(1);
		let wh = {
			let data_clone = self.data[tid].clone();
			let mut data = self.data[tid].wlock_ignore_poison()?;
			let guard = data.guard();

			let wh = WriteHandle::new(
				handle,
				id,
				write_state.clone(),
				data_clone,
				(**guard).debug_write_queue,
				(**guard).debug_pending,
				(**guard).debug_write_error,
				(**guard).debug_suspended,
				None,
				tls_client.clone(),
			);

			let tx = Some(tx);

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
				tx,
			};

			let ci = ConnectionInfo::StreamInfo(rwi);
			(**guard).nhandles.enqueue(ci)?;
			(**guard).attachments.insert(id, attachment);
			(**guard).wakeup.wakeup()?;
			wh
		};

		debug!("rx.recv handle={},id={}", handle, id)?;
		match rx.recv() {
			Ok(_) => {}
			Err(e) => warn!("rx.recv generated error: {}", e)?,
		}

		Ok(wh)
	}
}

impl Wakeup {
	pub(crate) fn new() -> Result<Self, Error> {
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

	pub(crate) fn wakeup(&mut self) -> Result<(), Error> {
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

	pub(crate) fn pre_block(&mut self) -> Result<(bool, RwLockReadGuardWrapper<bool>), Error> {
		let requested = self.requested.rlock()?;
		{
			let mut needed = self.needed.wlock()?;
			**needed.guard() = true;
		}
		let lock_guard = self.needed.rlock()?;
		let is_requested = **requested.guard();
		Ok((is_requested, lock_guard))
	}

	pub(crate) fn post_block(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let mut needed = self.needed.wlock()?;

		**requested.guard() = false;
		**needed.guard() = false;
		Ok(())
	}
}

pub(crate) fn read_bytes(handle: Handle, buf: &mut [u8]) -> isize {
	set_errno(Errno(0));
	read_bytes_impl(handle, buf)
}

pub(crate) fn write_bytes(handle: Handle, buf: &[u8]) -> isize {
	set_errno(Errno(0));
	let _ = debug!("write bytes to handle = {}", handle);
	write_bytes_impl(handle, buf)
}

pub(crate) fn make_config(
	trusted_cert_full_chain_file: Option<String>,
) -> Result<Arc<ClientConfig>, Error> {
	let mut root_store = RootCertStore::empty();
	for root in TLS_SERVER_ROOTS.iter() {
		root_store.roots.push(root.clone());
	}

	match trusted_cert_full_chain_file {
		Some(trusted_cert_full_chain_file) => {
			let full_chain_certs = load_certs(&trusted_cert_full_chain_file)?;
			for i in 0..full_chain_certs.len() {
				root_store.add(full_chain_certs[i].clone())?;
			}
		}
		None => {}
	}
	let config = ClientConfig::builder()
		.with_root_certificates(root_store)
		.with_no_client_auth();
	Ok(Arc::new(config))
}

pub(crate) fn load_certs(filename: &str) -> Result<Vec<CertificateDer<'static>>, Error> {
	let certs = rustls_pemfile::certs(&mut BufReader::new(&mut File::open(filename)?))
		.collect::<Result<Vec<_>, _>>()?;

	Ok(certs)
}

pub(crate) fn load_private_key(filename: &str) -> Result<PrivateKeyDer<'static>, Error> {
	match rustls_pemfile::private_key(&mut BufReader::new(&mut File::open(filename)?))? {
		Some(pkder) => Ok(pkder),
		None => Err(err!(
			ErrKind::Rustls,
			format!("private key for file '{}' was not found", filename)
		)),
	}
}

#[cfg(not(tarpaulin_include))] // assert full coverage for this function
pub(crate) fn handle_close(
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
