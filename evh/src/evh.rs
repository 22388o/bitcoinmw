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

#[cfg(target_os = "linux")]
use crate::linux::*;
#[cfg(target_os = "macos")]
use crate::mac::*;
#[cfg(target_os = "windows")]
use crate::win::*;

use crate::constants::*;
use crate::types::{
	Chunk, ConnectionType, ConnectionVariant, DebugInfo, Event, EventHandlerCallbacks,
	EventHandlerConfig, EventHandlerContext, EventHandlerImpl, EventHandlerState, EventIn,
	EventType, EventTypeIn, EvhController, GlobalStats, UserContextImpl, Wakeup, WriteHandle,
	WriteState,
};
use crate::{Connection, EventHandler, EvhStats, UserContext};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::rand::random;
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::time::{SystemTime, UNIX_EPOCH};

info!();

fn add_connection(
	debug_info: &DebugInfo,
	state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
	wakeups: &mut Array<Wakeup>,
	mut connection: Connection,
	is_server: bool,
	tid: usize,
	handle: Handle,
) -> Result<(), Error> {
	let (tx, rx) = sync_channel(1);
	connection.set_tx(tx);
	connection.debug_info = debug_info.clone();
	debug!("adding server handle = {}, tid = {}", handle, tid)?;

	{
		let mut state = state[tid].wlock()?;
		let guard = state.guard()?;
		let nv = if is_server {
			ConnectionVariant::ServerConnection(connection)
		} else {
			ConnectionVariant::ClientConnection(connection)
		};
		(**guard).nconnections.push_back(nv);
	}

	debug!("about to wakeup")?;

	wakeups[tid].wakeup()?;
	rx.recv()?;

	Ok(())
}

fn do_wakeup_read_impl(
	handle: Handle,
	buf: &mut [u8],
	debug_info: &DebugInfo,
) -> Result<Option<usize>, Error> {
	if debug_info.is_wakeup_read_err() {
		let text = "simulated read error";
		return Err(err!(ErrKind::Test, text));
	}
	read_impl(handle, buf, debug_info)
}

fn do_read_impl(
	handle: Handle,
	buf: &mut [u8],
	debug_info: &DebugInfo,
) -> Result<Option<usize>, Error> {
	if debug_info.is_read_err() {
		let text = "simulated read error";
		return Err(err!(ErrKind::Test, text));
	}
	read_impl(handle, buf, debug_info)
}

fn do_write_impl(handle: Handle, buf: &[u8], debug_info: &DebugInfo) -> Result<isize, Error> {
	if debug_info.is_write_err() {
		let text = "simulated write error";
		return Err(err!(ErrKind::Test, text));
	}
	if debug_info.is_write_err2() {
		set_errno(Errno(1));
		Ok(-1)
	} else {
		write_impl(handle, buf)
	}
}

fn do_get_events(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
	debug_info: &DebugInfo,
) -> Result<(), Error> {
	if debug_info.is_get_events_error() {
		return Err(err!(ErrKind::Test, "get events err"));
	}
	get_events(config, ctx)
}

impl Default for DebugInfo {
	fn default() -> Self {
		Self {
			pending: lock_box!(false).unwrap(),
			write_err: lock_box!(false).unwrap(),
			write_err2: lock_box!(false).unwrap(),
			read_err: lock_box!(false).unwrap(),
			wakeup_read_err: lock_box!(false).unwrap(),
			write_handle_err: lock_box!(false).unwrap(),
			stop_error: lock_box!(false).unwrap(),
			panic_fatal_error: lock_box!(false).unwrap(),
			normal_fatal_error: lock_box!(false).unwrap(),
			internal_panic: lock_box!(false).unwrap(),
			get_events_error: lock_box!(false).unwrap(),
			os_error: lock_box!(false).unwrap(),
		}
	}
}

impl DebugInfo {
	fn is_stop_error(&self) -> bool {
		#[cfg(test)]
		{
			**self.stop_error.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_write_handle_err(&self) -> bool {
		#[cfg(test)]
		{
			**self.write_handle_err.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_pending(&self) -> bool {
		#[cfg(test)]
		{
			**self.pending.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_read_err(&self) -> bool {
		#[cfg(test)]
		{
			**self.read_err.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_wakeup_read_err(&self) -> bool {
		#[cfg(test)]
		{
			**self.wakeup_read_err.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_write_err(&self) -> bool {
		#[cfg(test)]
		{
			**self.write_err.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_write_err2(&self) -> bool {
		#[cfg(test)]
		{
			**self.write_err2.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_panic_fatal_error(&self) -> bool {
		#[cfg(test)]
		{
			**self.panic_fatal_error.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_normal_fatal_error(&self) -> bool {
		#[cfg(test)]
		{
			**self.normal_fatal_error.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_internal_panic(&self) -> bool {
		#[cfg(test)]
		{
			**self.internal_panic.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn is_get_events_error(&self) -> bool {
		#[cfg(test)]
		{
			**self.get_events_error.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	pub(crate) fn is_os_error(&self) -> bool {
		#[cfg(test)]
		{
			**self.os_error.rlock().unwrap().guard().unwrap()
		}
		#[cfg(not(test))]
		{
			false
		}
	}
	fn update(&mut self, debug_info: DebugInfo) -> Result<(), Error> {
		wlock!(self.pending) = rlock!(debug_info.pending);
		wlock!(self.write_err) = rlock!(debug_info.write_err);
		wlock!(self.write_err2) = rlock!(debug_info.write_err2);
		wlock!(self.read_err) = rlock!(debug_info.read_err);
		wlock!(self.wakeup_read_err) = rlock!(debug_info.wakeup_read_err);
		wlock!(self.write_handle_err) = rlock!(debug_info.write_handle_err);
		wlock!(self.stop_error) = rlock!(debug_info.stop_error);
		wlock!(self.panic_fatal_error) = rlock!(debug_info.panic_fatal_error);
		wlock!(self.normal_fatal_error) = rlock!(debug_info.normal_fatal_error);
		wlock!(self.internal_panic) = rlock!(debug_info.internal_panic);
		wlock!(self.get_events_error) = rlock!(debug_info.get_events_error);
		wlock!(self.os_error) = rlock!(debug_info.os_error);
		Ok(())
	}
}

impl Wakeup {
	pub(crate) fn new() -> Result<Self, Error> {
		set_errno(Errno(0));
		let (reader, writer) = wakeup_impl()?;
		let requested = lock_box!(false)?;
		let needed = lock_box!(false)?;
		let id = random();
		Ok(Self {
			id,
			reader,
			writer,
			requested,
			needed,
		})
	}

	pub(crate) fn wakeup(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let needed = self.needed.rlock()?;
		let need_wakeup = **needed.guard()? && !(**requested.guard()?);
		**requested.guard()? = true;
		if need_wakeup {
			debug!("wakeup writing to {}", self.writer)?;
			let len = write_impl(self.writer, &[0u8; 1])?;
			debug!("len={},errno={}", len, errno())?;
		}
		Ok(())
	}

	pub(crate) fn pre_block(&mut self) -> Result<(bool, RwLockReadGuardWrapper<bool>), Error> {
		let requested = self.requested.rlock()?;
		{
			let mut needed = self.needed.wlock()?;
			**needed.guard()? = true;
		}
		let lock_guard = self.needed.rlock()?;
		let is_requested = **requested.guard()?;
		Ok((is_requested, lock_guard))
	}

	pub(crate) fn post_block(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let mut needed = self.needed.wlock()?;

		**requested.guard()? = false;
		**needed.guard()? = false;
		Ok(())
	}
}

impl<'a> Chunk<'a> {
	/// Retrieves the `slab_id` of the slab for this [`crate::Chunk`]. See
	/// [`crate::UserContext::clear_through`].
	pub fn slab_id(&self) -> usize {
		self.slab.id()
	}
	/// Retrieves the data associated with this chunk as a [`slice`].
	pub fn data(&'a self) -> &'a [u8] {
		&self.slab.get()[0..self.len]
	}
}

impl UserContext for &mut UserContextImpl {
	fn next_chunk(&mut self, connection: &mut Connection) -> Result<Option<Chunk>, Error> {
		let last_slab = connection.get_last_slab();
		let slab_offset = connection.get_slab_offset();

		if self.slab_cur >= u32::MAX as usize {
			Ok(None)
		} else {
			let slab = self.read_slabs.get(self.slab_cur)?;
			let slab_bytes = slab.get();
			let next_ptr = slab_bytes.len().saturating_sub(4);
			let len = if self.slab_cur == last_slab {
				slab_offset
			} else {
				next_ptr
			};

			let bytes = try_into!(&slab_bytes[next_ptr..next_ptr + 4])?;
			self.slab_cur = u32::from_be_bytes(bytes) as usize;

			let chunk = Chunk { slab, len };
			Ok(Some(chunk))
		}
	}
	fn clear_all(&mut self, connection: &mut Connection) -> Result<(), Error> {
		self.clear_through(connection.get_last_slab(), connection)
	}
	fn clear_through(&mut self, slab_id: usize, connection: &mut Connection) -> Result<(), Error> {
		debug!("clear_through for {}", connection.handle())?;
		let mut cur = connection.get_first_slab();
		loop {
			if cur >= u32::MAX as usize {
				connection.set_first_slab(u32::MAX as usize);
				connection.set_last_slab(u32::MAX as usize);
				break;
			}
			let slab = self.read_slabs.get(cur)?;
			let slab = slab.get();
			let len = slab.len();
			let start = len.saturating_sub(4);
			let next = u32::from_be_bytes(try_into!(&slab[start..start + 4])?) as usize;
			debug!("free slab1 {}", cur)?;
			self.read_slabs.free(cur)?;

			connection.set_first_slab(next);
			if connection.get_first_slab() >= u32::MAX as usize {
				connection.set_last_slab(connection.get_first_slab());
			}

			if cur == slab_id {
				debug!("breaking because cur = {}, slab_id = {}", cur, slab_id)?;
				break;
			}

			cur = next;
		}

		debug!(
			"clear through complete first_slab={},last_slab={}",
			connection.get_first_slab(),
			connection.get_last_slab()
		)?;
		Ok(())
	}

	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any + Send + Sync>> {
		&mut self.user_data
	}

	fn set_user_data(&mut self, user_data: Box<dyn Any + Send + Sync>) {
		self.user_data = Some(user_data);
	}
}

impl WriteState {
	pub(crate) fn new() -> Self {
		Self {
			flags: 0,
			write_buffer: vec![],
		}
	}

	pub(crate) fn set_flag(&mut self, flag: u8) {
		self.flags |= flag;
	}

	pub(crate) fn unset_flag(&mut self, flag: u8) {
		self.flags &= !flag;
	}

	pub(crate) fn is_set(&self, flag: u8) -> bool {
		self.flags & flag != 0
	}
}

impl WriteHandle {
	/// Write data to the underlying connection for this [`crate::WriteHandle`].
	/// # Input Parameters
	/// data - the data to be written to the connecction.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] - if an I/O error occurs while writing to the connection.
	/// [`bmw_err::ErrKind::IO`] - if the connection is already closed.
	/// # See also
	/// See the [`crate`] documentation as well for the background information and motivation
	/// for this crate as well as examples.
	pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
		let data_len = data.len();
		let wlen = {
			let write_state = self.write_state.rlock()?;
			let guard = write_state.guard()?;

			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!("write on a closed handle: {}", self.handle);
				return Err(err!(ErrKind::IO, text));
			} else if (**guard).is_set(WRITE_STATE_FLAG_PENDING)
				|| self.debug_info.is_pending()
				|| self.debug_info.is_write_handle_err()
			{
				0
			} else {
				write_impl(self.handle, data)?
			}
		};

		if wlen < 0 || self.debug_info.is_write_handle_err() {
			let text = format!("write I/O error handle {}: {}", self.handle, errno());
			return Err(err!(ErrKind::IO, text));
		}

		let wlen: usize = try_into!(wlen)?;
		if wlen < data_len {
			self.queue_data(&data[wlen..])?;
		}
		Ok(())
	}
	/// Close the underlying connection for this [`crate::WriteHandle`].
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] - if an I/O error occurs while closing the connection.
	/// [`bmw_err::ErrKind::IO`] - if the connection is already closed.
	/// # See also
	/// See the [`crate`] documentation as well for the background information and motivation
	/// for this crate as well as examples.
	pub fn close(&mut self) -> Result<(), Error> {
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!(
					"try to close a handle that is already closed: {}",
					self.handle
				);
				return Err(err!(ErrKind::IO, text));
			}

			(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
		}

		{
			wlock!(self.state).write_queue.push_back(self.id);
		}

		self.wakeup.wakeup()?;
		Ok(())
	}

	/// Trigger a callback of the handler specified by [`crate::EventHandler::set_on_read`].
	/// This is useful in applications like pipelines where data is held up for later
	/// processing so that asynchronous threads can be executed.
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] - if an I/O error occurs.
	/// [`bmw_err::ErrKind::IO`] - if the connection is already closed.
	/// # See also
	/// See the [`crate`] documentation as well for the background information and motivation
	/// for this crate as well as examples.
	pub fn trigger_on_read(&mut self) -> Result<(), Error> {
		debug!("trigger on read {} ", self.handle)?;
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!("trigger_on_read on a closed handle: {}", self.handle);
				return Err(err!(ErrKind::IO, text));
			}

			(**guard).set_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ);
		}
		{
			wlock!(self.state).write_queue.push_back(self.id);
		}
		self.wakeup.wakeup()?;

		Ok(())
	}

	/// Retrieve the underlying connection's id.
	pub fn id(&mut self) -> u128 {
		self.id
	}

	fn is_set(&self, flag: u8) -> Result<bool, Error> {
		let write_state = self.write_state.rlock()?;
		let guard = write_state.guard()?;
		Ok((**guard).is_set(flag))
	}

	fn unset_flag(&mut self, flag: u8) -> Result<(), Error> {
		let mut write_state = self.write_state.wlock()?;
		let guard = write_state.guard()?;
		(**guard).unset_flag(flag);
		Ok(())
	}

	fn write_state(&mut self) -> Result<&mut Box<dyn LockBox<WriteState>>, Error> {
		Ok(&mut self.write_state)
	}
	pub(crate) fn new(connection_impl: &Connection, debug_info: DebugInfo) -> Result<Self, Error> {
		let wakeup = match &connection_impl.wakeup {
			Some(wakeup) => wakeup.clone(),
			None => {
				let text = "connection has no Wakeup";
				return Err(err!(ErrKind::IllegalState, text));
			}
		};
		let state = match &connection_impl.state {
			Some(state) => state.clone(),
			None => {
				let text = "connection has no WriteState";
				return Err(err!(ErrKind::IllegalState, text));
			}
		};

		Ok(Self {
			handle: connection_impl.handle,
			id: connection_impl.id,
			write_state: connection_impl.write_state.clone(),
			wakeup,
			state,
			debug_info,
		})
	}
	fn queue_data(&mut self, data: &[u8]) -> Result<(), Error> {
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			(**guard).set_flag(WRITE_STATE_FLAG_PENDING);
			(**guard).write_buffer.extend(data);
		}

		{
			wlock!(self.state).write_queue.push_back(self.id);
		}

		self.wakeup.wakeup()?;
		Ok(())
	}
}

impl Connection {
	/// Retrieves the `id` for this Connection. The id is a unique random u128 value.
	pub fn id(&self) -> u128 {
		self.id
	}

	/// Returns a [`crate::WriteHandle`] which can be used to write data, close the
	/// connection, or trigger and on_read event for the underlying connection. See
	/// [`crate::WriteHandle`].
	pub fn write_handle(&self) -> Result<WriteHandle, Error> {
		let wh = WriteHandle::new(self, self.debug_info.clone())?;
		Ok(wh)
	}
	pub(crate) fn new(
		handle: Handle,
		wakeup: Option<Wakeup>,
		state: Option<Box<dyn LockBox<EventHandlerState>>>,
		ctype: ConnectionType,
		debug_info: DebugInfo,
	) -> Result<Self, Error> {
		Ok(Self {
			handle,
			id: random(),
			first_slab: usize::MAX,
			last_slab: usize::MAX,
			slab_offset: 0,
			write_state: lock_box!(WriteState::new())?,
			wakeup,
			state,
			tx: None,
			ctype,
			debug_info,
		})
	}
	pub(crate) fn handle(&self) -> Handle {
		self.handle
	}
	pub(crate) fn set_state(
		&mut self,
		state: Box<dyn LockBox<EventHandlerState>>,
	) -> Result<(), Error> {
		self.state = Some(state);
		Ok(())
	}

	fn set_wakeup(&mut self, wakeup: Wakeup) -> Result<(), Error> {
		self.wakeup = Some(wakeup);
		Ok(())
	}

	fn set_tx(&mut self, tx: SyncSender<()>) {
		self.tx = Some(tx);
	}
	fn get_tx(&mut self) -> Option<&mut SyncSender<()>> {
		self.tx.as_mut()
	}
	fn get_slab_offset(&self) -> usize {
		self.slab_offset
	}
	fn get_first_slab(&self) -> usize {
		self.first_slab
	}
	fn get_last_slab(&self) -> usize {
		self.last_slab
	}
	fn set_slab_offset(&mut self, slab_offset: usize) {
		self.slab_offset = slab_offset;
	}
	fn set_first_slab(&mut self, first_slab: usize) {
		self.first_slab = first_slab;
	}
	fn set_last_slab(&mut self, last_slab: usize) {
		self.last_slab = last_slab;
	}
}

impl EventHandlerState {
	pub(crate) fn new() -> Result<Self, Error> {
		Ok(Self {
			nconnections: VecDeque::new(),
			write_queue: VecDeque::new(),
			stop: false,
		})
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> Drop
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	fn drop(&mut self) {
		let _ = debug!("drop evh");
		let stop_res = self.stop();
		if stop_res.is_err() {
			let _ = error!("Error occurred while dropping: {}", stop_res.unwrap_err());
		}
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	fn start(&mut self) -> Result<(), Error> {
		self.start_impl()
	}
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error> {
		self.callbacks.on_read = Some(Box::pin(on_read));
		Ok(())
	}
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error> {
		self.callbacks.on_accept = Some(Box::pin(on_accept));
		Ok(())
	}
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error> {
		self.callbacks.on_close = Some(Box::pin(on_close));
		Ok(())
	}
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error> {
		self.callbacks.on_housekeeper = Some(Box::pin(on_housekeeper));
		Ok(())
	}
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error> {
		self.callbacks.on_panic = Some(Box::pin(on_panic));
		Ok(())
	}
	fn set_debug_info(&mut self, debug_info: DebugInfo) -> Result<(), Error> {
		self.debug_info.update(debug_info)?;
		Ok(())
	}
	fn add_server_connection(&mut self, connection: Connection) -> Result<(), Error> {
		if connection.ctype != ConnectionType::Server {
			let text = "trying to add a non-server connection as a server!";
			return Err(err!(ErrKind::IllegalArgument, text));
		}
		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		add_connection(
			&self.debug_info,
			&mut self.state,
			&mut self.wakeups,
			connection,
			true,
			tid,
			handle,
		)
	}
	fn add_client_connection(&mut self, mut connection: Connection) -> Result<WriteHandle, Error> {
		if connection.ctype != ConnectionType::Client {
			let text = "trying to add a non-server connection as a server!";
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		connection.set_state(self.state[tid].clone())?;
		connection.set_wakeup(self.wakeups[tid].clone())?;
		let ret = connection.write_handle()?;
		add_connection(
			&self.debug_info,
			&mut self.state,
			&mut self.wakeups,
			connection,
			false,
			tid,
			handle,
		)?;
		Ok(ret)
	}

	fn controller(&mut self) -> Result<EvhController, Error> {
		self.has_controller = true;
		Ok(EvhController {
			state: self.state.clone(),
			wakeups: self.wakeups.clone(),
			stopper: self.stopper.clone(),
			stats: self.stats.clone(),
			debug_info: self.debug_info.clone(),
			config: self.config.clone(),
		})
	}

	fn wait_for_stats(&mut self) -> Result<EvhStats, Error> {
		self.wait_for_stats()
	}
}

impl EvhController {
	pub fn add_server_connection(&mut self, connection: Connection) -> Result<(), Error> {
		if connection.ctype != ConnectionType::Server {
			let text = "trying to add a non-server connection as a server!";
			return Err(err!(ErrKind::IllegalArgument, text));
		}
		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		add_connection(
			&self.debug_info,
			&mut self.state,
			&mut self.wakeups,
			connection,
			true,
			tid,
			handle,
		)
	}

	pub fn add_client_connection(
		&mut self,
		mut connection: Connection,
	) -> Result<WriteHandle, Error> {
		if connection.ctype != ConnectionType::Client {
			let text = "trying to add a non-server connection as a server!";
			return Err(err!(ErrKind::IllegalArgument, text));
		}

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		connection.set_state(self.state[tid].clone())?;
		connection.set_wakeup(self.wakeups[tid].clone())?;
		let ret = connection.write_handle()?;
		add_connection(
			&self.debug_info,
			&mut self.state,
			&mut self.wakeups,
			connection,
			false,
			tid,
			handle,
		)?;
		Ok(ret)
	}

	pub fn wait_for_stats(&mut self) -> Result<EvhStats, Error> {
		let mut ret = EvhStats::new();
		let (tx, rx) = sync_channel(1);
		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;
			(**guard).tx = Some(tx);
		}

		rx.recv()?;

		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;

			let _ = std::mem::replace(&mut ret, (**guard).stats.clone());
			(**guard).stats.reset();
		}

		Ok(ret)
	}

	pub fn stop(&mut self) -> Result<(), Error> {
		if self.debug_info.is_stop_error() {
			let text = "simulated stop error";
			return Err(err!(ErrKind::Test, text));
		}

		// stop thread pool and all threads
		if self.stopper.is_some() {
			self.stopper.as_mut().unwrap().stop()?;
		}
		for i in 0..self.config.threads {
			wlock!(self.state[i]).stop = true;
			self.wakeups[i].wakeup()?;
		}

		Ok(())
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = Self::build_config(configs)?;
		let mut state = array!(config.threads, &lock_box!(EventHandlerState::new()?)?)?;

		let w = Wakeup::new()?;
		close_impl(w.reader)?;
		close_impl(w.writer)?;
		let mut wakeups = array!(config.threads, &w)?;

		for i in 0..config.threads {
			state[i] = lock_box!(EventHandlerState::new()?)?;
			wakeups[i] = Wakeup::new()?;
		}

		let global_stats = GlobalStats {
			stats: EvhStats::new(),
			update_counter: 0,
			tx: None,
		};
		let stats = lock_box!(global_stats)?;

		let debug_info = DebugInfo::default();
		let on_read = None;
		let on_accept = None;
		let on_close = None;
		let on_panic = None;
		let on_housekeeper = None;
		let callbacks = EventHandlerCallbacks {
			on_read,
			on_accept,
			on_close,
			on_panic,
			on_housekeeper,
		};

		let stopper = None;
		let has_controller = false;

		let ret = Self {
			callbacks,
			config,
			state,
			wakeups,
			stats,
			stopper,
			debug_info,
			has_controller,
		};

		Ok(ret)
	}

	fn wait_for_stats(&mut self) -> Result<EvhStats, Error> {
		let mut ret = EvhStats::new();
		let (tx, rx) = sync_channel(1);
		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;
			(**guard).tx = Some(tx);
		}

		rx.recv()?;

		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;

			let _ = std::mem::replace(&mut ret, (**guard).stats.clone());
			(**guard).stats.reset();
		}

		Ok(ret)
	}

	fn start_impl(&mut self) -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(self.config.threads))?;
		let mut executor = lock_box!(tp.executor()?)?;
		let mut executor_clone = executor.clone();
		self.stopper = Some(tp.stopper()?);

		let config = self.config.clone();
		let mut callbacks = self.callbacks.clone();
		let state = self.state.clone();
		let wakeups = self.wakeups.clone();

		let read_slabs = slab_allocator!(
			SlabSize(config.read_slab_size),
			SlabCount(config.read_slab_count)
		)?;
		let user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};

		let wakeups_cl = wakeups.clone();
		let stats_cl = self.stats.clone();
		let sample = EventHandlerContext::new(wakeups_cl, 0, stats_cl)?;
		let sample = lock_box!(sample)?;
		let mut ctx_arr = array!(config.threads, &sample)?;
		let mut user_context_arr = array!(config.threads, &lock_box!(user_context)?)?;

		for i in 0..config.threads {
			let mut evhc = EventHandlerContext::new(wakeups.clone(), i, self.stats.clone())?;
			let wakeup_reader = wakeups[i].reader;
			let evt = EventIn::new(wakeup_reader, EventTypeIn::Read);
			evhc.in_events.push(evt);

			ctx_arr[i] = lock_box!(evhc)?;
			let read_slabs = slab_allocator!(
				SlabSize(config.read_slab_size),
				SlabCount(config.read_slab_count)
			)?;
			let user_context = UserContextImpl {
				read_slabs,
				user_data: None,
				slab_cur: usize::MAX,
			};
			user_context_arr[i] = lock_box!(user_context)?;

			let nv = ConnectionVariant::Wakeup(wakeups[i].clone());
			wlock!(self.state[i]).nconnections.push_back(nv);
		}

		let ctx_arr_clone = ctx_arr.clone();
		let debug_info_clone = self.debug_info.clone();
		let mut user_context_arr_clone = user_context_arr.clone();

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			{
				let id = try_into!(id)?;
				let mut user_context = user_context_arr_clone[id].wlock_ignore_poison()?;
				let guard = user_context.guard()?;
				Self::call_on_panic(&mut callbacks.on_panic, &mut *guard, e)?;
			}
			let config = config.clone();
			let callbacks = callbacks.clone();
			let state = state.clone();
			let user_ctx_arr_clone = user_context_arr_clone.clone();
			let ctx_arr = ctx_arr_clone.clone();
			let debug_info = debug_info_clone.clone();

			wlock!(executor).execute(
				async move {
					let i = try_into!(id)?;
					let c = config;
					let d = callbacks;
					let e = state;
					let f = ctx_arr;
					let g = user_ctx_arr_clone;
					let h = true;
					let x = &debug_info;
					let r = EventHandlerImpl::execute_thread(c, d, e, f, g, i, h, x);

					if r.is_err() {
						let e = r.unwrap_err();
						fatal!("Execute thread had an unexpected error: {}", e)?;
					}
					Ok(())
				},
				try_into!(id)?,
			)?;
			Ok(())
		})?;

		tp.start()?;

		{
			let mut executor = executor_clone.wlock()?;
			let guard = executor.guard()?;
			(**guard) = tp.executor()?;
		}

		for i in 0..self.config.threads {
			let config = self.config.clone();
			let callbacks = self.callbacks.clone();
			let state = self.state.clone();
			let ctx_arr = ctx_arr.clone();
			let user_context_arr = user_context_arr.clone();
			let debug_info = self.debug_info.clone();
			execute!(tp, try_into!(i)?, {
				let c = config;
				let a = callbacks;
				let s = state;
				let r = ctx_arr.clone();
				let u = user_context_arr.clone();
				let f = false;
				let d = &debug_info;

				let t = Self::execute_thread(c, a, s, r, u, i, f, d);
				if t.is_err() {
					let e = t.unwrap_err();
					fatal!("Execute thread had an unexpected error: {}", e)?;
				}
				Ok(())
			})?;
		}
		Ok(())
	}

	fn build_config(configs: Vec<ConfigOption>) -> Result<EventHandlerConfig, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(
			vec![
				CN::EvhReadSlabSize,
				CN::EvhReadSlabCount,
				CN::EvhTimeout,
				CN::EvhThreads,
				CN::EvhHouseKeeperFrequencyMillis,
				CN::EvhStatsUpdateMillis,
				CN::Debug,
			],
			vec![],
		)?;

		let threads = config.get_or_usize(&CN::EvhThreads, EVH_DEFAULT_THREADS);
		let evhrlc = &CN::EvhReadSlabCount;
		let read_slab_count = config.get_or_usize(evhrlc, EVH_DEFAULT_READ_SLAB_COUNT);
		let read_slab_size = config.get_or_usize(&CN::EvhReadSlabSize, EVH_DEFAULT_READ_SLAB_SIZE);
		let debug = config.get_or_bool(&CN::Debug, false);
		let timeout = config.get_or_u16(&CN::EvhTimeout, EVH_DEFAULT_TIMEOUT);
		let evhkfm = &CN::EvhHouseKeeperFrequencyMillis;
		let default = EVH_DEFAULT_HOUSEKEEPING_FREQUENCY_MILLIS;
		let housekeeping_frequency_millis = config.get_or_usize(evhkfm, default);
		let evhsum = &CN::EvhStatsUpdateMillis;
		let default = EVH_DEFAULT_STATS_UPDATE_MILLIS;
		let stats_update_frequency_millis = config.get_or_usize(evhsum, default);

		if read_slab_count == 0 {
			let text = "EvhReadSlabCount count must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		if read_slab_size < 25 {
			let text = "EvhReadSlabSize must be at least 25";
			return Err(err!(ErrKind::Configuration, text));
		}

		if timeout == 0 {
			let text = "EvhTimeout must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		if housekeeping_frequency_millis == 0 {
			let text = "EvhHouseKeeperFrequencyMillis must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		let evhc = EventHandlerConfig {
			threads,
			debug,
			timeout,
			read_slab_size,
			read_slab_count,
			housekeeping_frequency_millis,
			stats_update_frequency_millis,
		};
		Ok(evhc)
	}

	pub(crate) fn execute_thread(
		config: EventHandlerConfig,
		mut callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut state: Array<Box<dyn LockBox<EventHandlerState>>>,
		mut ctx_arr: Array<Box<dyn LockBox<EventHandlerContext>>>,
		mut user_context_arr: Array<Box<dyn LockBox<UserContextImpl>>>,
		tid: usize,
		panic_recovery: bool,
		debug_info: &DebugInfo,
	) -> Result<(), Error> {
		debug!("panic rec")?;
		if panic_recovery && debug_info.is_panic_fatal_error() {
			return Err(err!(ErrKind::Test, "panic fatal err"));
		} else if !panic_recovery && debug_info.is_normal_fatal_error() {
			return Err(err!(ErrKind::Test, "normal fatal err"));
		}
		debug!("execute thread {}", tid)?;

		let mut ctx = ctx_arr[tid].wlock_ignore_poison()?;
		let mut user_context = user_context_arr[tid].wlock_ignore_poison()?;
		let ctx_guard = ctx.guard()?;
		let user_context_guard = user_context.guard()?;

		let mut count = 0u128;

		if panic_recovery {
			let ret_event_itt = (**ctx_guard).ret_event_itt;
			let ret_event_count = (**ctx_guard).ret_event_count;

			let trigger_itt = (**ctx_guard).trigger_itt;
			let trigger_count = (**ctx_guard).trigger_on_read_list.len();
			warn!("panic occurred, trying to recover")?;

			if trigger_itt < trigger_count && !debug_info.is_internal_panic() {
				let handle = (**ctx_guard).trigger_on_read_list[trigger_itt];
				debug!("handle to close (trigger_on_read) = {}", handle)?;

				let h = handle;
				let g = &mut (**ctx_guard);
				let c = &mut callbacks;
				let u = &mut (**user_context_guard);
				Self::process_close(h, g, c, u)?;

				// skip over errant event
				(**ctx_guard).trigger_itt += 1;
			} else if ret_event_itt < ret_event_count && !debug_info.is_internal_panic() {
				debug!("itt={},count={}", ret_event_itt, ret_event_count)?;
				// the error must have been in the on_read regular read events
				let handle = (**ctx_guard).ret_events[(**ctx_guard).ret_event_itt].handle;
				debug!("handle to close (regular) = {}", handle)?;

				let h = handle;
				let g = &mut (**ctx_guard);
				let c = &mut callbacks;
				let u = &mut (**user_context_guard);
				Self::process_close(h, g, c, u)?;

				// skip over errant event
				(**ctx_guard).ret_event_itt += 1;
			} else {
				// something's wrong
				warn!("panic, but no pending events. Internal panic?")?;
			}

			let c = &config;
			let g = &mut (**ctx_guard);
			let ca = &mut callbacks;
			let s = &mut state;
			let u = &mut (**user_context_guard);
			let r = Self::process_events(c, g, ca, s, u, debug_info);

			if r.is_err() {
				let e = r.unwrap_err();
				fatal!("Process events generated an unexpected error: {}", e)?;
			}
		}

		let s = &mut state[tid];
		let c = &mut (**ctx_guard);
		let ca = &mut callbacks;
		let u = &mut (**user_context_guard);
		let co = &config;

		let proc_state_res = Self::process_state(s, c, ca, u, co, debug_info);
		let stop = match proc_state_res {
			Ok(stop) => stop,
			Err(e) => {
				fatal!("Process events generated an unexpected error: {}", e)?;
				!debug_info.is_internal_panic()
			}
		};

		if !stop {
			loop {
				let r = do_get_events(&config, &mut (**ctx_guard), debug_info);
				if r.is_err() {
					let e = r.unwrap_err();
					fatal!("get_events generated an unexpected error: {}", e,)?;
				}

				let cg = &mut **ctx_guard;
				cg.thread_stats.event_loops += 1;
				cg.in_events.clear();
				cg.in_events.shrink_to(EVH_DEFAULT_IN_EVENTS_SIZE);

				if config.debug {
					info!("Thread loop {}", count)?;
				}

				let s = &mut state[tid];
				let c = &mut callbacks;
				let u = &mut (**user_context_guard);
				let co = &config;

				let r = Self::process_state(s, cg, c, u, co, debug_info);
				match r {
					Ok(stop) => cbreak!(stop),
					Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
				}

				debug!("calling proc events")?;
				// set iterator to 0 outside function in case of thread panic
				cg.ret_event_itt = 0;
				cg.trigger_itt = 0;
				let s = &mut state;
				let r = Self::process_events(co, cg, c, s, u, debug_info);
				if r.is_err() {
					let e = r.unwrap_err();
					fatal!("Process events generated an unexpected error: {}", e)?;
				}
				count += 1;
			}
		}
		Ok(())
	}

	pub(crate) fn close_handles(
		ctx: &mut EventHandlerContext,
		nconnections: &VecDeque<ConnectionVariant>,
		_callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	) -> Result<(), Error> {
		debug!("in close_handles {}", ctx.tid)?;
		let reader = ctx.wakeups[ctx.tid].reader;
		let writer = ctx.wakeups[ctx.tid].writer;
		for (handle, id) in &ctx.handle_hash {
			debug!("close handle = {}, id = {}", handle, id)?;
			if *handle != reader && *handle != writer {
				close_impl(*handle)?;
			}
		}

		for conn in nconnections {
			match conn {
				ConnectionVariant::ServerConnection(c) => {
					let handle = c.handle();
					if handle != reader && handle != writer {
						close_impl(handle)?;
					}
				}
				ConnectionVariant::ClientConnection(c) => {
					let handle = c.handle();
					if handle != reader && handle != writer {
						close_impl(handle)?;
					}
				}
				ConnectionVariant::Connection(c) => {
					let handle = c.handle();
					if handle != reader && handle != writer {
						close_impl(handle)?;
					}
				}
				ConnectionVariant::Wakeup(_w) => {}
			}
		}

		close_impl(reader)?;
		close_impl(writer)?;

		Ok(())
	}

	fn process_housekeeper(
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		config: &EventHandlerConfig,
	) -> Result<(), Error> {
		let now: usize = try_into!(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
		if now.saturating_sub(ctx.last_housekeeping) > config.housekeeping_frequency_millis {
			Self::call_on_housekeeper(user_context, &mut callbacks.on_housekeeper)?;
			ctx.last_housekeeping = now;
		}

		if now.saturating_sub(ctx.last_stats_update) > config.stats_update_frequency_millis {
			Self::update_stats(ctx, config)?;
			ctx.last_stats_update = now;
		}
		Ok(())
	}

	fn update_stats(
		ctx: &mut EventHandlerContext,
		config: &EventHandlerConfig,
	) -> Result<(), Error> {
		{
			let mut global_stats = ctx.global_stats.wlock()?;
			let guard = global_stats.guard()?;
			(**guard).stats.incr_stats(&ctx.thread_stats);
			(**guard).update_counter += 1;
			if (**guard).update_counter >= config.threads {
				if (**guard).tx.is_some() {
					let tx = &(**guard).tx.as_mut().unwrap();
					tx.send(())?;
				}

				(**guard).update_counter = 0;
			}
		}

		ctx.thread_stats.reset();
		Ok(())
	}

	fn process_state(
		state: &mut Box<dyn LockBox<EventHandlerState>>,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		config: &EventHandlerConfig,
		debug_info: &DebugInfo,
	) -> Result<bool, Error> {
		if debug_info.is_internal_panic() {
			return Err(err!(ErrKind::Test, "internal panic"));
		}

		debug!("in process state tid={}", ctx.tid)?;

		Self::process_write_pending(ctx, callbacks, user_context, state)?;

		let mut state = state.wlock()?;
		let guard = state.guard()?;
		debug!("guard.stop={}", (**guard).stop)?;
		if (**guard).stop {
			debug!("stopping thread")?;
			Self::close_handles(ctx, &(**guard).nconnections, callbacks)?;
			Ok(true)
		} else {
			Self::process_housekeeper(ctx, callbacks, user_context, config)?;
			debug!("nconnections.size={}", (**guard).nconnections.len())?;
			loop {
				let next = (**guard).nconnections.pop_front();
				cbreak!(next.is_none());
				let mut next = next.unwrap();
				let (handle, id) = match &mut next {
					ConnectionVariant::ServerConnection(conn) => {
						debug!("server in process state")?;
						let mut tx = conn.get_tx();
						if tx.is_some() {
							let _ = tx.as_mut().unwrap().send(());
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::ClientConnection(conn) => {
						debug!("client in process state")?;
						let mut tx = conn.get_tx();
						if tx.is_some() {
							let _ = tx.as_mut().unwrap().send(());
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::Connection(conn) => {
						ctx.thread_stats.accepts += 1;
						Self::call_on_accept(user_context, conn, &mut callbacks.on_accept)?;
						(conn.handle(), conn.id())
					}
					ConnectionVariant::Wakeup(wakeup) => (wakeup.reader, wakeup.id),
				};

				debug!("found handle = {}, id = {}", handle, id)?;
				ctx.id_hash.insert(id, next);
				ctx.handle_hash.insert(handle, id);
				let event_in = EventIn::new(handle, EventTypeIn::Read);
				ctx.in_events.push(event_in);
			}

			Ok(false)
		}
	}

	fn process_write_pending(
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		state: &mut Box<dyn LockBox<EventHandlerState>>,
	) -> Result<(), Error> {
		debug!("in process write pending")?;
		let mut ids = vec![];
		{
			let mut state = state.wlock()?;
			let guard = state.guard()?;
			debug!("write_queue.len={}", (**guard).write_queue.len())?;
			while (**guard).write_queue.len() != 0 {
				let id = (**guard).write_queue.pop_front();
				ids.push(id.unwrap());
			}
		}

		ctx.trigger_on_read_list.clear();
		for id in ids {
			Self::process_write_id(ctx, id, callbacks, user_context)?;
		}

		Ok(())
	}

	pub(crate) fn process_write_id(
		ctx: &mut EventHandlerContext,
		id: u128,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		let mut close_list = vec![];
		let mut conn = ctx.id_hash.get_mut(&id);
		if conn.is_some() {
			let conn = conn.as_mut().unwrap();
			match conn {
				ConnectionVariant::ServerConnection(_conn) => {}
				ConnectionVariant::ClientConnection(conn) => {
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(conn)?;

					// if data is pending complete the write first
					if close && !pending {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						ctx.trigger_on_read_list.push(conn.handle());
					}

					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Connection(conn) => {
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(conn)?;

					// if data is pending complete the write first
					if close && !pending {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						ctx.trigger_on_read_list.push(conn.handle());
					}
					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Wakeup(_wakeup) => {}
			}
		} else {
			warn!("none1 in process_write_id")?;
		}

		for handle in close_list {
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}
		Ok(())
	}

	fn write_conn(conn: &mut Connection) -> Result<(bool, bool, bool), Error> {
		let mut write_handle = conn.write_handle()?;
		let ret1 = write_handle.is_set(WRITE_STATE_FLAG_CLOSE)?;
		let ret2 = write_handle.is_set(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		let ret3 = write_handle.is_set(WRITE_STATE_FLAG_PENDING)?;
		if ret2 {
			write_handle.unset_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		}
		Ok((ret1, ret2, ret3))
	}

	pub(crate) fn process_events(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		u: &mut UserContextImpl,
		d: &DebugInfo,
	) -> Result<(), Error> {
		if d.is_internal_panic() || d.is_get_events_error() {
			return Err(err!(ErrKind::Test, "internal panic"));
		}

		// first call the trigger on reads
		debug!("trig list = {:?}", ctx.trigger_on_read_list)?;
		let list_len = ctx.trigger_on_read_list.len();
		loop {
			cbreak!(ctx.trigger_itt == list_len);

			let handle = ctx.trigger_on_read_list[ctx.trigger_itt];
			let id = ctx.handle_hash.get(&handle);
			if id.is_some() {
				let id = id.unwrap();
				let mut conn = ctx.id_hash.get_mut(&id);
				if conn.is_some() {
					match conn.as_mut().unwrap() {
						ConnectionVariant::Connection(conn) => {
							Self::call_on_read(u, conn, &mut callbacks.on_read)?
						}
						ConnectionVariant::ClientConnection(conn) => {
							Self::call_on_read(u, conn, &mut callbacks.on_read)?
						}
						_ => warn!("unexpected Conection variant for trigger_on_read")?,
					}
				} else {
					warn!("none in trigger_on_read1")?;
				}
			} else {
				warn!("none in trigger_on_read2")?;
			}
			ctx.trigger_itt += 1;
		}

		// next process events
		debug!("events to process = {}", ctx.ret_event_count)?;
		loop {
			cbreak!(ctx.ret_event_itt == ctx.ret_event_count);

			debug!("proc event = {:?}", ctx.ret_events[ctx.ret_event_itt])?;
			let h = ctx.ret_events[ctx.ret_event_itt].handle;
			let mut need_read_update = false;
			let mut need_write_update = false;
			if ctx.ret_events[ctx.ret_event_itt].etype == EventType::Read
				|| ctx.ret_events[ctx.ret_event_itt].etype == EventType::ReadWrite
			{
				need_read_update =
					Self::process_read_event(config, ctx, callbacks, h, state, u, d)?;
			}
			if ctx.ret_events[ctx.ret_event_itt].etype == EventType::Write
				|| ctx.ret_events[ctx.ret_event_itt].etype == EventType::ReadWrite
			{
				need_write_update = Self::process_write_event(config, ctx, callbacks, h, u)?;
			}

			if need_write_update {
				update_ctx(ctx, h, EventTypeIn::Write)?;
			} else if need_read_update {
				update_ctx(ctx, h, EventTypeIn::Read)?;
			}
			ctx.ret_event_itt += 1;
		}

		Ok(())
	}

	pub(crate) fn process_read_event(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		user_context: &mut UserContextImpl,
		debug_info: &DebugInfo,
	) -> Result<bool, Error> {
		let mut ret = false;
		let mut accepted = vec![];
		let mut close = false;
		let mut read_count = 0;
		let mut read_sum = 0;
		debug!("process read event= {}", handle)?;
		let id = ctx.handle_hash.get(&handle);
		if id.is_some() {
			let id = id.unwrap();
			let mut conn = ctx.id_hash.get_mut(&id);
			if conn.is_some() {
				let conn = conn.as_mut().unwrap();
				match conn {
					ConnectionVariant::ServerConnection(conn) => {
						Self::process_accept(conn, &mut accepted, debug_info, callbacks)?;
						ret = true;
					}
					ConnectionVariant::ClientConnection(conn) => {
						(close, read_count, read_sum) =
							Self::process_read(conn, config, callbacks, user_context, debug_info)?;
						ret = !close;
					}
					ConnectionVariant::Connection(conn) => {
						(close, read_count, read_sum) =
							Self::process_read(conn, config, callbacks, user_context, debug_info)?;
						ret = !close;
					}
					ConnectionVariant::Wakeup(_wakeup) => {
						let mut buf = [0u8; 1000];
						loop {
							let rlen = match do_wakeup_read_impl(handle, &mut buf, debug_info) {
								Ok(rlen) => rlen,
								Err(e) => {
									warn!("wakeup read_impl err: {}", e)?;
									None
								}
							};
							debug!("wakeup read rlen = {:?}", rlen)?;
							cbreak!(rlen.is_none());
						}
						ret = true;
					}
				}
			} else {
				warn!("none1")?;
			}
		} else {
			warn!("none2")?;
		}
		debug!("close was {}", close)?;
		if close {
			debug!("closing handle {}", handle)?;
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}
		ctx.thread_stats.reads += read_count;
		ctx.thread_stats.bytes_read += read_sum;

		Self::process_accepted_connections(accepted, config, state, &mut ctx.wakeups, debug_info)?;
		Ok(ret)
	}

	fn process_accepted_connections(
		accepted: Vec<Handle>,
		config: &EventHandlerConfig,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		wakeups: &mut Array<Wakeup>,
		debug_info: &DebugInfo,
	) -> Result<(), Error> {
		debug!("accepted connections = {:?}", accepted)?;
		for a in accepted {
			let accept_usize: usize = try_into!(a)?;
			let tid = accept_usize % config.threads;
			let wakeup = Some(wakeups[tid].clone());
			let cstate = Some(state[tid].clone());
			let ctype = ConnectionType::Connection;
			let connection = Connection::new(a, wakeup, cstate, ctype, debug_info.clone())?;

			{
				let mut state = state[tid].wlock()?;
				let guard = state.guard()?;
				let nv = ConnectionVariant::Connection(connection);
				(**guard).nconnections.push_back(nv);
			}

			wakeups[tid].wakeup()?;
		}
		Ok(())
	}

	fn process_read(
		conn: &mut Connection,
		config: &EventHandlerConfig,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		debug_info: &DebugInfo,
	) -> Result<(bool, usize, u128), Error> {
		debug!("in process_read")?;
		let mut close = false;
		let mut read_count = 0;
		let mut read_sum = 0u128;
		let handle = conn.handle();
		// loop through and read as many slabs as we can

		while TRUE {
			let last_slab = conn.get_last_slab();
			let slab_offset = conn.get_slab_offset();
			let len = config.read_slab_size;
			let read_slab_next_offset = len.saturating_sub(4);
			let mut slab = if last_slab >= u32::MAX as usize {
				let slab = match user_context.read_slabs.allocate() {
					Ok(slab) => Some(slab),
					Err(e) => {
						warn!("cannot allocate any more slabs1 due to: {}", e)?;
						close = true;
						None
					}
				};

				cbreak!(slab.is_none());
				let mut slab = slab.unwrap();

				let id = slab.id();
				debug!("allocate a slab1 {}", id)?;
				// initialize connection with values
				conn.set_last_slab(id);
				conn.set_first_slab(id);
				conn.set_slab_offset(0);

				// set next pointer to u32::MAX (end of chain)
				slab.get_mut()[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&u32::MAX.to_be_bytes());
				slab
			} else if slab_offset == read_slab_next_offset {
				let slab = match user_context.read_slabs.allocate() {
					Ok(slab) => Some(slab),
					Err(e) => {
						warn!("cannot allocate any more slabs2 due to: {}", e)?;
						close = true;
						None
					}
				};

				cbreak!(slab.is_none());
				let slab = slab.unwrap();

				let slab_id = slab.id();
				debug!("allocate a slab2 {}", slab_id)?;
				user_context.read_slabs.get_mut(last_slab)?.get_mut()
					[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&(slab_id as u32).to_be_bytes());
				conn.set_last_slab(slab_id);
				conn.set_slab_offset(0);
				let mut ret = user_context.read_slabs.get_mut(slab_id)?;

				ret.get_mut()[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&u32::MAX.to_be_bytes());

				ret
			} else {
				user_context.read_slabs.get_mut(last_slab)?
			};

			let slab_offset = conn.get_slab_offset();
			let slab_id = slab.id();
			let slab_bytes = &mut slab.get_mut()[slab_offset..read_slab_next_offset];
			let rlen = match do_read_impl(handle, slab_bytes, debug_info) {
				Ok(rlen) => rlen,
				Err(_e) => {
					// read error. Close the connection
					// we don't log this because it pollutes the logs
					close = true;
					None
				}
			};
			debug!("rlen={:?}, slab_offset={}", rlen, slab_offset)?;

			cbreak!(close);

			if rlen.is_some() {
				let rlen = rlen.unwrap();
				if rlen > 0 {
					conn.set_slab_offset(slab_offset + rlen);
					read_count += 1;
					let rlen_u128: u128 = try_into!(rlen)?;
					read_sum += rlen_u128;
				}

				let cur = slab_offset + rlen;
				debug!("rlen={},slab_id={},slab_offset={}", rlen, slab_id, cur)?;

				if rlen == 0 {
					debug!("connection closed")?;
					close = true;
					cbreak!(true);
				}
			} else {
				debug!("no more data to read for now")?;
				// if the slab doesn't have any data, we free it
				if slab_offset == 0 {
					debug!("free slab2 {}", slab_id)?;
					user_context.read_slabs.free(slab_id)?;
					conn.set_last_slab(last_slab);

					if last_slab < u32::MAX as usize {
						let mut slab_mut = user_context.read_slabs.get_mut(last_slab)?;
						let slab = slab_mut.get_mut();
						slab[read_slab_next_offset..read_slab_next_offset + 4]
							.clone_from_slice(&u32::MAX.to_be_bytes());
					} else {
						conn.set_first_slab(last_slab);
					}
				}

				// no more to read for now
				cbreak!(true);
			}

			debug!("call onread")?;
			Self::call_on_read(user_context, conn, &mut callbacks.on_read)?;
		}

		Ok((close, read_count, read_sum))
	}

	fn call_on_housekeeper(
		user_context: &mut UserContextImpl,
		callback: &mut Option<Pin<Box<OnHousekeeper>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		if callback.is_some() {
			let callback = callback.as_mut().unwrap();
			let mut user_context: Box<dyn UserContext> = Box::new(user_context);
			let res = callback(&mut user_context);
			if res.is_err() {
				let e = res.unwrap_err();
				warn!("on_housekeeper callback generated error: {}", e)?;
			}
		}
		Ok(())
	}

	fn call_on_panic(
		callback: &mut Option<Pin<Box<OnPanic>>>,
		user_context: &mut UserContextImpl,
		e: Box<dyn Any + Send>,
	) -> Result<(), Error> {
		if callback.is_some() {
			let mut user_context: Box<dyn UserContext> = Box::new(user_context);
			let callback = callback.as_mut().unwrap();
			let res = callback(&mut user_context, e);
			if res.is_err() {
				let e = res.unwrap_err();
				warn!("on_panic callback generated error: {}", e)?;
			}
		}
		Ok(())
	}

	fn call_on_read(
		user_context: &mut UserContextImpl,
		conn: &mut Connection,
		callback: &mut Option<Pin<Box<OnRead>>>,
	) -> Result<(), Error> {
		if !conn.write_handle()?.is_set(WRITE_STATE_FLAG_CLOSE)? {
			user_context.slab_cur = conn.get_first_slab();
			if callback.is_some() {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				let callback = callback.as_mut().unwrap();
				let res = callback(conn, &mut user_context);
				if res.is_err() {
					let e = res.unwrap_err();
					warn!("on_read callback generated error: {}", e)?;
				}
			}
		}
		Ok(())
	}

	fn call_on_accept(
		user_context: &mut UserContextImpl,
		conn: &mut Connection,
		callback: &mut Option<Pin<Box<OnAccept>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		if callback.is_some() {
			let mut user_context: Box<dyn UserContext> = Box::new(user_context);
			let callback = callback.as_mut().unwrap();
			let res = callback(conn, &mut user_context);
			if res.is_err() {
				let e = res.unwrap_err();
				warn!("on_accept callback generated error: {}", e)?;
			}
		}
		Ok(())
	}

	pub(crate) fn call_on_close(
		user_context: &mut UserContextImpl,
		handle: Handle,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		ctx: &mut EventHandlerContext,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callbacks.on_close {
			Some(ref mut callback) => match ctx.handle_hash.get(&handle) {
				Some(id) => match ctx.id_hash.get_mut(id) {
					Some(conn) => match conn {
						ConnectionVariant::Connection(conn) => {
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							let res = callback(conn, &mut user_context);
							if res.is_err() {
								let e = res.unwrap_err();
								warn!("on_close callback generated error: {}", e)?;
							}
						}
						ConnectionVariant::ClientConnection(conn) => {
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							let res = callback(conn, &mut user_context);
							if res.is_err() {
								let e = res.unwrap_err();
								warn!("on_close callback generated error: {}", e)?;
							}
						}
						_ => warn!("on_close called on unexpected type. tid = {}", ctx.tid)?,
					},
					None => warn!("onclose noneA")?,
				},
				None => warn!("onclose noneB")?,
			},
			None => warn!("onclose noneC")?,
		}
		Ok(())
	}

	pub(crate) fn process_close(
		handle: Handle,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		ctx.thread_stats.closes += 1;
		Self::call_on_close(user_context, handle, callbacks, ctx)?;

		let id = ctx.handle_hash.remove(&handle).unwrap_or(u128::MAX);
		debug!("removing handle={},id={}", handle, id)?;
		match ctx.id_hash.remove(&id) {
			Some(conn) => match conn {
				ConnectionVariant::Connection(mut conn) => {
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				ConnectionVariant::ClientConnection(mut conn) => {
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				_ => warn!("unexpected process_close server/wakeup tid = {}", ctx.tid)?,
			},
			None => warn!("expected a connection")?,
		}
		close_impl_ctx(handle, ctx)?;
		debug!("id hash rem")?;
		Ok(())
	}

	pub(crate) fn process_accept(
		conn: &Connection,
		accepted: &mut Vec<Handle>,
		debug_info: &DebugInfo,
		_callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	) -> Result<(), Error> {
		let handle = conn.handle();
		let id = conn.id();
		debug!("process read event on handle={},id={}", handle, id)?;
		while TRUE {
			let accept_res = accept_impl(handle, debug_info);

			if accept_res.is_ok() {
				let next = accept_res.unwrap();
				cbreak!(next.is_none());
				accepted.push(next.unwrap());
			} else {
				let e = accept_res.unwrap_err();
				warn!("accept generated error: {}", e)?;
				cbreak!(true);
			}
		}
		Ok(())
	}

	pub(crate) fn process_write_event(
		_config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
		user_context: &mut UserContextImpl,
	) -> Result<bool, Error> {
		let mut close = false;
		let mut write_count = 0;
		let mut write_sum = 0;
		match ctx.handle_hash.get(&handle) {
			Some(id) => match ctx.id_hash.get_mut(id) {
				Some(conn) => match conn {
					ConnectionVariant::Connection(conn) => {
						(close, write_count, write_sum) = Self::write_loop(conn, callbacks)?;
					}
					ConnectionVariant::ClientConnection(conn) => {
						(close, write_count, write_sum) = Self::write_loop(conn, callbacks)?;
					}
					_ => warn!("unexpected ConnectionVariant in process_write_event")?,
				},
				None => warn!("id hash lookup failed for id: {}, handle: {}", id, handle)?,
			},
			None => warn!("handle lookup failed for  handle: {}", handle)?,
		}

		let ret = if close {
			Self::process_close(handle, ctx, callbacks, user_context)?;
			false
		} else {
			true
		};

		ctx.thread_stats.delay_writes += write_count;
		ctx.thread_stats.bytes_delay_write += write_sum;

		Ok(ret)
	}

	pub(crate) fn write_loop(
		conn: &mut Connection,
		_callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	) -> Result<(bool, usize, u128), Error> {
		let mut write_count = 0;
		let mut write_sum = 0;
		let mut wh = conn.write_handle()?;
		let write_state = wh.write_state()?;
		let mut write_state = write_state.wlock()?;
		let guard = write_state.guard()?;
		let mut close = false;
		let mut rem = true;

		loop {
			let len = (**guard).write_buffer.len();
			if len == 0 && !conn.debug_info.is_write_err2() {
				rem = false;
				cbreak!(true);
			}
			let wlen = match do_write_impl(conn.handle(), &(**guard).write_buffer, &conn.debug_info)
			{
				Ok(wlen) => wlen,
				Err(_e) => {
					// write i/o error. Don't log these because they would pollute
					// the logs
					close = true;
					0
				}
			};
			cbreak!(close);

			if wlen < 0 {
				let err = errno().0;
				if err != EAGAIN && err != ETEMPUNAVAILABLE && err != WINNONBLOCKING {
					close = true;
				}
				cbreak!(true);
			} else {
				let wlen: usize = try_into!(wlen)?;
				if wlen > 0 {
					write_count += 1;
					let wlen_u128: u128 = try_into!(wlen)?;
					write_sum += wlen_u128;
				}

				(**guard).write_buffer.drain(0..wlen);
				(**guard).write_buffer.shrink_to_fit();
			}
		}

		if !rem {
			(**guard).unset_flag(WRITE_STATE_FLAG_PENDING);

			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				close = true;
			}
		}

		Ok((close, write_count, write_sum))
	}

	fn stop(&mut self) -> Result<(), Error> {
		if !self.has_controller {
			if self.debug_info.is_stop_error() {
				let text = "simulated stop error";
				return Err(err!(ErrKind::Test, text));
			}

			// stop thread pool and all threads
			if self.stopper.is_some() {
				self.stopper.as_mut().unwrap().stop()?;
			}
			for i in 0..self.config.threads {
				wlock!(self.state[i]).stop = true;
				self.wakeups[i].wakeup()?;
			}
		}

		Ok(())
	}
}

impl Event {
	pub(crate) fn new(handle: Handle, etype: EventType) -> Self {
		Self { handle, etype }
	}

	fn empty() -> Self {
		Self {
			etype: EventType::Read,
			handle: 0,
		}
	}
}

impl EventIn {
	pub(crate) fn new(handle: Handle, etype: EventTypeIn) -> Self {
		Self { handle, etype }
	}
}

impl EventHandlerContext {
	pub(crate) fn new(
		wakeups: Array<Wakeup>,
		tid: usize,
		global_stats: Box<dyn LockBox<GlobalStats>>,
	) -> Result<Self, Error> {
		let ret_event_count = 0;
		let ret_events = [Event::empty(); MAX_RET_HANDLES];
		let in_events = vec![];
		let id_hash = HashMap::new();
		let handle_hash = HashMap::new();

		Ok(Self {
			ret_event_count,
			ret_events,
			in_events,
			id_hash,
			handle_hash,
			wakeups,
			tid,
			last_housekeeping: 0,
			trigger_on_read_list: vec![],
			trigger_itt: 0,
			ret_event_itt: 0,
			thread_stats: EvhStats::new(),
			global_stats,
			last_stats_update: 0,
			#[cfg(target_os = "linux")]
			linux_ctx: LinuxContext::new()?,
			#[cfg(target_os = "macos")]
			macos_ctx: MacosContext::new()?,
			#[cfg(target_os = "windows")]
			windows_ctx: WindowsContext::new()?,
		})
	}
}

impl EvhStats {
	pub(crate) fn new() -> Self {
		Self {
			accepts: 0,
			closes: 0,
			reads: 0,
			delay_writes: 0,
			event_loops: 0,
			bytes_delay_write: 0,
			bytes_read: 0,
		}
	}

	fn reset(&mut self) {
		self.accepts = 0;
		self.closes = 0;
		self.reads = 0;
		self.delay_writes = 0;
		self.event_loops = 0;
		self.bytes_read = 0;
		self.bytes_delay_write = 0;
	}

	fn incr_stats(&mut self, stats: &EvhStats) {
		self.accepts += stats.accepts;
		self.closes += stats.closes;
		self.reads += stats.reads;
		self.delay_writes += stats.delay_writes;
		self.event_loops += stats.event_loops;
		self.bytes_read += stats.bytes_read;
		self.bytes_delay_write += stats.bytes_delay_write;
	}
}
