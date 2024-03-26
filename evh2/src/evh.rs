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
	ConnectionImpl, ConnectionVariant, Event, EventHandlerCallbacks, EventHandlerConfig,
	EventHandlerContext, EventHandlerImpl, EventHandlerState, EventIn, EventType, EventTypeIn,
	UserContextImpl, Wakeup, WriteHandleImpl, WriteState,
};
use crate::{
	ClientConnection, Connection, EventHandler, ServerConnection, UserContext, WriteHandle,
};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_deps::errno::errno;
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

impl Wakeup {
	fn new() -> Result<Self, Error> {
		let lock = lock_box!(false)?;
		let lock2 = lock_box!(())?;
		let (reader, writer) = wakeup_impl()?;
		let id = random();
		Ok(Self {
			lock,
			lock2,
			reader,
			writer,
			id,
		})
	}

	fn wakeup(&mut self) -> Result<(), Error> {
		let buf = [0u8; 1];
		let _lock = self.lock2.wlock()?;
		write_impl(self.writer, &buf)?;
		wlock!(self.lock) = true;
		Ok(())
	}

	pub(crate) fn get_lock(&mut self) -> &mut Box<dyn LockBox<bool>> {
		&mut self.lock
	}

	pub(crate) fn get_lock2(&mut self) -> &mut Box<dyn LockBox<()>> {
		&mut self.lock2
	}
}

impl UserContext for &mut UserContextImpl {
	fn clone_next_chunk(
		&mut self,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
		buf: &mut [u8],
	) -> Result<usize, Error> {
		let last_slab = connection.get_last_slab();
		let slab_offset = connection.get_slab_offset();

		if self.slab_cur >= u32::MAX as usize {
			Ok(0)
		} else {
			let slab = self.read_slabs.get(self.slab_cur)?;
			let slab = slab.get();
			let start_ptr = slab.len().saturating_sub(4);

			let mut offset = if self.slab_cur == last_slab {
				slab_offset
			} else {
				start_ptr
			};

			let buf_len = buf.len();
			if buf_len < offset {
				offset = buf_len;
			}
			buf[0..offset].clone_from_slice(&slab[0..offset]);
			self.slab_cur =
				u32::from_be_bytes(try_into!(&slab[start_ptr..start_ptr + 4])?) as usize;
			Ok(offset)
		}
	}
	fn cur_slab_id(&self) -> usize {
		self.slab_cur
	}
	fn clear_all(
		&mut self,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
	) -> Result<(), Error> {
		self.clear_through(connection.get_last_slab(), connection)
	}
	fn clear_through(
		&mut self,
		slab_id: usize,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
	) -> Result<(), Error> {
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
			debug!("free slab {}", cur)?;
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

	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any>> {
		&mut self.user_data
	}

	fn set_user_data(&mut self, user_data: Box<dyn Any>) {
		self.user_data = Some(user_data);
	}
}

impl WriteState {
	fn new() -> Self {
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

impl WriteHandle for WriteHandleImpl {
	fn write(&mut self, data: &[u8]) -> Result<(), Error> {
		let data_len = data.len();
		let wlen = {
			let write_state = self.write_state.rlock()?;
			let guard = write_state.guard()?;

			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!("write on a closed handle: {}", self.handle);
				return Err(err!(ErrKind::IO, text));
			} else if (**guard).is_set(WRITE_STATE_FLAG_PENDING) {
				0
			} else {
				write_impl(self.handle, data)?
			}
		};

		if wlen < 0 {
			let text = format!(
				"An i/o error occurred while trying to write to handle {}: {}",
				self.handle,
				errno()
			);
			return Err(err!(ErrKind::IO, text));
		}

		let wlen: usize = try_into!(wlen)?;

		if wlen < data_len {
			self.queue_data(&data[wlen..])?;
		}
		Ok(())
	}
	fn close(&mut self) -> Result<(), Error> {
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
	fn trigger_on_read(&mut self) -> Result<(), Error> {
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

	fn is_set(&self, flag: u8) -> Result<bool, Error> {
		let write_state = self.write_state.rlock()?;
		let guard = write_state.guard()?;
		Ok((**guard).is_set(flag))
	}

	fn set_flag(&mut self, flag: u8) -> Result<(), Error> {
		let mut write_state = self.write_state.wlock()?;
		let guard = write_state.guard()?;
		(**guard).set_flag(flag);
		Ok(())
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
}

impl WriteHandleImpl {
	fn new(connection_impl: &ConnectionImpl) -> Result<Self, Error> {
		let wakeup = match &connection_impl.wakeup {
			Some(wakeup) => wakeup.clone(),
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"cannot create a write handle on a connection that has no wakeup set"
				));
			}
		};
		let state = match &connection_impl.state {
			Some(state) => state.clone(),
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"cannot create a write handle on a connection that has no state set"
				));
			}
		};
		Ok(Self {
			handle: connection_impl.handle,
			id: connection_impl.id,
			write_state: connection_impl.write_state.clone(),
			wakeup,
			state,
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

impl ClientConnection for ConnectionImpl {
	fn as_connection(&mut self) -> Box<dyn Connection + '_ + Send + Sync> {
		Box::new(self)
	}

	fn set_state(&mut self, state: Box<dyn LockBox<EventHandlerState>>) -> Result<(), Error> {
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
}

impl ServerConnection for ConnectionImpl {
	fn as_connection(&mut self) -> Box<dyn Connection + '_ + Send + Sync> {
		Box::new(self)
	}
	fn set_tx(&mut self, tx: SyncSender<()>) {
		self.tx = Some(tx);
	}
	fn get_tx(&mut self) -> Option<&mut SyncSender<()>> {
		self.tx.as_mut()
	}
}

impl Connection for &mut ConnectionImpl {
	fn handle(&self) -> Handle {
		self.handle
	}
	fn id(&self) -> u128 {
		self.id
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
	fn set_slab_offset(&mut self, offset: usize) {
		self.slab_offset = offset;
	}
	fn set_first_slab(&mut self, first_slab: usize) {
		self.first_slab = first_slab;
	}
	fn set_last_slab(&mut self, last_slab: usize) {
		self.last_slab = last_slab;
	}

	fn write_handle(&self) -> Result<Box<dyn WriteHandle + Send + Sync>, Error> {
		let wh: Box<dyn WriteHandle + Send + Sync> = Box::new(WriteHandleImpl::new(self)?);
		Ok(wh)
	}
}

impl Connection for ConnectionImpl {
	fn handle(&self) -> Handle {
		self.handle
	}
	fn id(&self) -> u128 {
		self.id
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

	fn write_handle(&self) -> Result<Box<dyn WriteHandle + Send + Sync>, Error> {
		let wh: Box<dyn WriteHandle + Send + Sync> = Box::new(WriteHandleImpl::new(self)?);
		Ok(wh)
	}
}

impl ConnectionImpl {
	pub(crate) fn new(
		handle: Handle,
		wakeup: Option<Wakeup>,
		state: Option<Box<dyn LockBox<EventHandlerState>>>,
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
		})
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
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
		match self.stop() {
			Ok(_) => {}
			Err(e) => {
				let _ = error!("Error occurred while dropping: {}", e);
			}
		}
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
	fn add_server_connection(
		&mut self,
		mut connection: Box<dyn ServerConnection + Send + Sync>,
	) -> Result<(), Error> {
		let (tx, rx) = sync_channel(1);
		connection.set_tx(tx);

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		debug!(
			"adding server handle = {}, tid = {}",
			connection.handle(),
			tid
		)?;

		{
			let mut state = self.state[tid].wlock()?;
			let guard = state.guard()?;
			(**guard)
				.nconnections
				.push_back(ConnectionVariant::ServerConnection(connection));
		}

		debug!("about to wakeup")?;

		self.wakeups[tid].wakeup()?;

		debug!("add complete")?;

		rx.recv()?;

		Ok(())
	}
	fn add_client_connection(
		&mut self,
		mut connection: Box<dyn ClientConnection + Send + Sync>,
	) -> Result<Box<dyn WriteHandle + Send + Sync>, Error> {
		let (tx, rx) = sync_channel(1);
		connection.set_tx(tx);

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;

		connection.set_state(self.state[tid].clone())?;
		connection.set_wakeup(self.wakeups[tid].clone())?;
		let ret = connection.write_handle()?;

		debug!(
			"adding client handle = {}, tid = {}, id = {}",
			connection.handle(),
			tid,
			connection.id(),
		)?;
		{
			let mut state = self.state[tid].wlock()?;
			let guard = state.guard()?;
			(**guard)
				.nconnections
				.push_back(ConnectionVariant::ClientConnection(connection));
		}
		debug!("push client connection to tid = {}", tid)?;

		self.wakeups[tid].wakeup()?;

		rx.recv()?;
		Ok(ret)
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
		let mut wakeups = array!(config.threads, &w)?;

		for i in 0..config.threads {
			state[i] = lock_box!(EventHandlerState::new()?)?;
			wakeups[i] = Wakeup::new()?;
		}

		Ok(Self {
			callbacks: EventHandlerCallbacks {
				on_read: None,
				on_accept: None,
				on_close: None,
				on_panic: None,
				on_housekeeper: None,
			},
			config,
			state,
			wakeups,
			stopper: None,
		})
	}

	fn start_impl(&mut self) -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(self.config.threads))?;
		self.stopper = Some(tp.stopper()?);

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			Self::process_thread_pool_panic(id, e)?;
			Ok(())
		})?;

		tp.start()?;

		for i in 0..self.config.threads {
			let config = self.config.clone();
			let callbacks = self.callbacks.clone();
			let state = self.state.clone();
			let wakeups = self.wakeups.clone();
			execute!(tp, {
				match Self::execute_thread(config, callbacks, state, wakeups, i) {
					Ok(_) => {}
					Err(e) => {
						fatal!("Execute thread had an unexpected error: {}", e)?;
					}
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
				CN::Debug,
			],
			vec![],
		)?;

		let threads = config.get_or_usize(&CN::EvhThreads, EVH_DEFAULT_THREADS);
		let read_slab_count =
			config.get_or_usize(&CN::EvhReadSlabCount, EVH_DEFAULT_READ_SLAB_COUNT);
		let read_slab_size = config.get_or_usize(&CN::EvhReadSlabSize, EVH_DEFAULT_READ_SLAB_SIZE);
		let debug = config.get_or_bool(&CN::Debug, false);
		let timeout = config.get_or_u16(&CN::EvhTimeout, EVH_DEFAULT_TIMEOUT);
		let housekeeping_frequency_millis = config.get_or_usize(
			&CN::EvhHouseKeeperFrequencyMillis,
			EVH_DEFAULT_HOUSEKEEPING_FREQUENCY_MILLIS,
		);
		Ok(EventHandlerConfig {
			threads,
			debug,
			timeout,
			read_slab_size,
			read_slab_count,
			housekeeping_frequency_millis,
		})
	}

	fn execute_thread(
		config: EventHandlerConfig,
		mut callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut state: Array<Box<dyn LockBox<EventHandlerState>>>,
		wakeups: Array<Wakeup>,
		tid: usize,
	) -> Result<(), Error> {
		debug!("execute thread {}", tid)?;

		let mut count = 0u128;
		let wakeup_reader = wakeups[tid].reader;

		wlock!(state[tid])
			.nconnections
			.push_back(ConnectionVariant::Wakeup(wakeups[tid].clone()));
		let mut ctx = EventHandlerContext::new(wakeups, tid)?;
		let read_slabs = slab_allocator!(
			SlabSize(config.read_slab_size),
			SlabCount(config.read_slab_count)
		)?;
		let mut user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};

		// listen for wakeups
		let evt = EventIn::new(wakeup_reader, EventTypeIn::Read);
		ctx.in_events.push(evt);

		match Self::process_state(
			&mut state[tid],
			&mut ctx,
			&mut callbacks,
			&mut user_context,
			&config,
		) {
			Ok(_) => {}
			Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
		}

		loop {
			match get_events(&config, &mut ctx) {
				Ok(_) => {}
				Err(e) => fatal!("get_events generated an unexpected error: {}", e)?,
			}
			ctx.in_events.clear();
			ctx.in_events.shrink_to(EVH_DEFAULT_IN_EVENTS_SIZE);

			if config.debug {
				info!("Thread loop {}", count)?;
			}

			match Self::process_events(
				&config,
				&mut ctx,
				&mut callbacks,
				&mut state,
				&mut user_context,
			) {
				Ok(_) => {}
				Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
			}
			match Self::process_state(
				&mut state[tid],
				&mut ctx,
				&mut callbacks,
				&mut user_context,
				&config,
			) {
				Ok(stop) => {
					if stop {
						break Ok(());
					}
				}
				Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
			}
			count += 1;
		}
	}

	fn close_handles(ctx: &mut EventHandlerContext) -> Result<(), Error> {
		for (handle, id) in &ctx.handle_hash {
			debug!("close handle = {}, id = {}", handle, id)?;
			close_impl(*handle)?;
		}
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
		Ok(())
	}

	fn process_state(
		state: &mut Box<dyn LockBox<EventHandlerState>>,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		config: &EventHandlerConfig,
	) -> Result<bool, Error> {
		debug!("in process state tid={}", ctx.tid)?;

		Self::process_write_pending(ctx, callbacks, user_context, state)?;

		let mut state = state.wlock()?;
		let guard = state.guard()?;

		if (**guard).stop {
			debug!("stopping thread")?;
			Self::close_handles(ctx)?;
			Ok(true)
		} else {
			Self::process_housekeeper(ctx, callbacks, user_context, config)?;
			debug!("nconnections.size={}", (**guard).nconnections.len())?;
			loop {
				let next = (**guard).nconnections.pop_front();
				if next.is_none() {
					break;
				}
				let mut next = next.unwrap();
				let (handle, id) = match &mut next {
					ConnectionVariant::ServerConnection(conn) => {
						debug!("server in process state")?;
						match conn.get_tx() {
							Some(tx) => {
								// attempt to send notification
								let _ = tx.send(());
							}
							None => {}
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::ClientConnection(conn) => {
						debug!("client in process state")?;
						match conn.get_tx() {
							Some(tx) => {
								// attempt to send notification
								let _ = tx.send(());
							}
							None => {}
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::Connection(conn) => {
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
			loop {
				match (**guard).write_queue.pop_front() {
					Some(id) => {
						debug!("popped id = {}", id)?;
						ids.push(id);
					}
					None => break,
				}
			}
		}

		for id in ids {
			Self::process_write_id(ctx, id, callbacks, user_context)?;
		}

		Ok(())
	}

	fn process_write_id(
		ctx: &mut EventHandlerContext,
		id: u128,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		let mut close_list = vec![];
		let mut trigger_on_read_list = vec![];
		match ctx.id_hash.get_mut(&id) {
			Some(conn) => match conn {
				ConnectionVariant::ServerConnection(_conn) => {}
				ConnectionVariant::ClientConnection(conn) => {
					let mut conn = conn.as_connection();
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(&mut conn)?;
					if close {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						trigger_on_read_list.push(conn.handle());
					}

					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Connection(conn) => {
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(conn)?;
					if close {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						trigger_on_read_list.push(conn.handle());
					}
					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Wakeup(_wakeup) => {}
			},
			None => {
				warn!("none1 in process_write_id")?;
			}
		}

		for handle in close_list {
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}
		for handle in trigger_on_read_list {
			match ctx.handle_hash.get(&handle) {
				Some(id) => match ctx.id_hash.get_mut(&id) {
					Some(conn) => match conn {
						ConnectionVariant::Connection(conn) => {
							Self::call_on_read(user_context, conn, &mut callbacks.on_read)?
						}
						_ => todo!(),
					},
					None => {
						warn!("none in trigger_on_read1")?;
					}
				},
				None => {
					warn!("none in trigger_on_read2")?;
				}
			}
		}
		Ok(())
	}

	fn write_conn(
		conn: &mut Box<dyn Connection + '_ + Send + Sync>,
	) -> Result<(bool, bool, bool), Error> {
		let mut write_handle = conn.write_handle()?;
		let ret1 = write_handle.is_set(WRITE_STATE_FLAG_CLOSE)?;
		let ret2 = write_handle.is_set(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		let ret3 = write_handle.is_set(WRITE_STATE_FLAG_PENDING)?;
		if ret2 {
			write_handle.unset_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		}
		Ok((ret1, ret2, ret3))
	}

	fn process_events(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		debug!("events to process = {}", ctx.ret_event_count)?;
		for i in 0..ctx.ret_event_count {
			debug!("proc event = {:?}", ctx.ret_events[i])?;
			if ctx.ret_events[i].etype == EventType::Read
				|| ctx.ret_events[i].etype == EventType::ReadWrite
			{
				Self::process_read_event(
					config,
					ctx,
					callbacks,
					ctx.ret_events[i].handle,
					state,
					user_context,
				)?;
			}
			if ctx.ret_events[i].etype == EventType::Write
				|| ctx.ret_events[i].etype == EventType::ReadWrite
			{
				Self::process_write_event(config, ctx, callbacks, ctx.ret_events[i].handle)?;
			}
		}
		Ok(())
	}

	fn process_read_event(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		let mut accepted = vec![];
		let mut close = false;
		debug!("lookup handle = {}", handle)?;
		match ctx.handle_hash.get(&handle) {
			Some(id) => match ctx.id_hash.get_mut(&id) {
				Some(conn) => match conn {
					ConnectionVariant::ServerConnection(conn) => {
						Self::process_accept(conn, &mut accepted)?;
					}
					ConnectionVariant::ClientConnection(conn) => {
						close = Self::process_read(
							&mut conn.as_connection(),
							config,
							callbacks,
							user_context,
						)?;
					}
					ConnectionVariant::Connection(conn) => {
						close = Self::process_read(conn, config, callbacks, user_context)?;
					}
					ConnectionVariant::Wakeup(_wakeup) => {
						let mut buf = [0u8; 1000];
						loop {
							let rlen = read_impl(handle, &mut buf)?;
							debug!("wakeup read rlen = {:?}", rlen)?;
							cbreak!(rlen.is_none());
						}
					}
				},
				None => {
					warn!("none1")?;
				}
			},
			None => {
				warn!("none2")?;
			}
		}
		debug!("close was {}", close)?;
		if close {
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}

		Self::process_accepted_connections(accepted, config, state, &mut ctx.wakeups)
	}

	fn process_accepted_connections(
		accepted: Vec<Handle>,
		config: &EventHandlerConfig,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		wakeups: &mut Array<Wakeup>,
	) -> Result<(), Error> {
		debug!("accepted connections = {:?}", accepted)?;
		for accept in accepted {
			let accept_usize: usize = try_into!(accept)?;
			let tid = accept_usize % config.threads;
			let connection: Box<dyn Connection + Send + Sync> = Box::new(ConnectionImpl::new(
				accept,
				Some(wakeups[tid].clone()),
				Some(state[tid].clone()),
			)?);

			{
				let mut state = state[tid].wlock()?;
				let guard = state.guard()?;
				(**guard)
					.nconnections
					.push_back(ConnectionVariant::Connection(connection));
			}

			wakeups[tid].wakeup()?;
		}
		Ok(())
	}

	fn process_read(
		conn: &mut Box<dyn Connection + '_ + Send + Sync>,
		config: &EventHandlerConfig,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
	) -> Result<bool, Error> {
		debug!("in process_read")?;
		let mut close = false;
		let handle = conn.handle();
		// loop through and read as many slabs as we can
		loop {
			let last_slab = conn.get_last_slab();
			let slab_offset = conn.get_slab_offset();
			let len = config.read_slab_size;
			let read_slab_next_offset = len.saturating_sub(4);
			let mut slab = if last_slab >= u32::MAX as usize {
				let mut slab = match user_context.read_slabs.allocate() {
					Ok(slab) => slab,
					Err(e) => {
						warn!("cannot allocate any more slabs due to: {}", e)?;
						// close connection
						close = true;
						break;
					}
				};
				let id = slab.id();
				debug!(
					"----------------------allocate a slab1----------------({})",
					id
				)?;
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
					Ok(slab) => slab,
					Err(e) => {
						warn!("cannot allocate any more slabs due to: {}", e)?;
						close = true;
						break;
					}
				};
				let slab_id = slab.id();
				debug!(
					"----------------------allocate a slab2----------------({})",
					slab_id
				)?;
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
			let rlen = read_impl(
				handle,
				&mut slab.get_mut()[slab_offset..read_slab_next_offset],
			)?;

			match rlen {
				Some(rlen) => {
					if rlen > 0 {
						conn.set_slab_offset(slab_offset + rlen);
					}

					debug!(
						"rlen={},slab_id={},slab_offset={}",
						rlen,
						slab.id(),
						slab_offset + rlen
					)?;

					if rlen == 0 {
						debug!("connection closed")?;
						close = true;
						break;
					}
				}
				None => {
					debug!("no more data to read for now")?;
					// no more to read for now
					break;
				}
			}

			Self::call_on_read(user_context, conn, &mut callbacks.on_read)?;
		}

		Ok(close)
	}

	fn call_on_housekeeper(
		user_context: &mut UserContextImpl,
		callback: &mut Option<Pin<Box<OnHousekeeper>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut on_housekeeper) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_housekeeper(&mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_housekeeper callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on housekeeper handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_read(
		user_context: &mut UserContextImpl,
		conn: &mut Box<dyn Connection + '_ + Send + Sync>,
		callback: &mut Option<Pin<Box<OnRead>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = conn.get_first_slab();
		match callback {
			Some(ref mut on_read) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_read(conn, &mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_read callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on read handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_accept(
		user_context: &mut UserContextImpl,
		conn: &mut Box<dyn Connection + '_ + Send + Sync>,
		callback: &mut Option<Pin<Box<OnAccept>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut on_accept) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_accept(conn, &mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_accept callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on accept handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_close(
		user_context: &mut UserContextImpl,
		handle: Handle,
		callback: &mut Option<Pin<Box<OnClose>>>,
		ctx: &mut EventHandlerContext,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut callback) => match ctx.handle_hash.get(&handle) {
				Some(id) => match ctx.id_hash.get_mut(id) {
					Some(conn) => match conn {
						ConnectionVariant::Connection(conn) => {
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							match callback(conn, &mut user_context) {
								Ok(_) => {}
								Err(e) => warn!("on_close callback generated error: {}", e)?,
							}
						}
						ConnectionVariant::ClientConnection(conn) => {
							let mut conn = conn.as_connection();
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							match callback(&mut conn, &mut user_context) {
								Ok(_) => {}
								Err(e) => warn!("on_close callback generated error: {}", e)?,
							}
						}
						ConnectionVariant::ServerConnection(_conn) => {
							warn!(
								"unexpected on_close called on server connection tid = {}",
								ctx.tid
							)?;
						}
						ConnectionVariant::Wakeup(_wakeup) => {
							warn!("unexpected on_close called on wakeup tid = {}", ctx.tid)?;
						}
					},
					None => warn!("noneA")?,
				},
				None => warn!("noneB")?,
			},
			None => {
				warn!("noneC")?;
			}
		}
		Ok(())
	}

	fn process_close(
		handle: Handle,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		debug!("Calling close")?;
		Self::call_on_close(user_context, handle, &mut callbacks.on_close, ctx)?;

		let id = ctx.handle_hash.remove(&handle).unwrap_or(u128::MAX);
		debug!("removing handle={},id={}", handle, id)?;
		match ctx.id_hash.remove(&id) {
			Some(conn) => match conn {
				ConnectionVariant::Connection(mut conn) => {
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				ConnectionVariant::ClientConnection(mut conn) => {
					let mut conn = conn.as_connection();
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				ConnectionVariant::ServerConnection(_conn) => {
					warn!(
						"unexpected process_close called on server_connection tid = {}",
						ctx.tid
					)?;
				}
				ConnectionVariant::Wakeup(_wakeup) => {
					warn!(
						"unexpected process_close called on wakeup tid = {}",
						ctx.tid
					)?;
				}
			},
			None => warn!("expected a connection")?,
		}
		close_impl_ctx(handle, ctx)?;
		debug!("id hash rem")?;
		Ok(())
	}

	fn process_accept(
		conn: &Box<dyn ServerConnection + Send + Sync>,
		accepted: &mut Vec<Handle>,
	) -> Result<(), Error> {
		let handle = conn.handle();
		let id = conn.id();
		debug!("process read event on handle={},id={}", handle, id)?;
		loop {
			match accept_impl(handle) {
				Ok(next) => {
					cbreak!(next.is_none());
					accepted.push(next.unwrap());
				}
				Err(e) => {
					warn!("accept generated error: {}", e)?;
					break;
				}
			}
		}
		Ok(())
	}

	fn process_write_event(
		_config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		_callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
	) -> Result<(), Error> {
		let mut close = false;
		match ctx.handle_hash.get(&handle) {
			Some(id) => match ctx.id_hash.get_mut(id) {
				Some(conn) => match conn {
					ConnectionVariant::Connection(conn) => {
						close = Self::write_loop(conn)?;
					}
					ConnectionVariant::ClientConnection(conn) => {
						let mut conn = conn.as_connection();
						close = Self::write_loop(&mut conn)?;
					}
					_ => todo!(),
				},
				None => {
					warn!("id hash lookup failed for id: {}, handle: {}", id, handle)?;
				}
			},
			None => {
				warn!("handle lookup failed for  handle: {}", handle)?;
			}
		}

		if close {
			close_impl_ctx(handle, ctx)?;
		}
		Ok(())
	}

	fn write_loop(conn: &mut Box<dyn Connection + '_ + Send + Sync>) -> Result<bool, Error> {
		let mut wh = conn.write_handle()?;
		let write_state = wh.write_state()?;
		let mut write_state = write_state.wlock()?;
		let guard = write_state.guard()?;
		let mut close = false;
		let mut rem = true;
		loop {
			let len = (**guard).write_buffer.len();
			if len == 0 {
				rem = false;
				break;
			}
			let wlen = write_impl(conn.handle(), &(**guard).write_buffer)?;
			if wlen < 0 {
				let err = errno().0;
				if err != EAGAIN && err != ETEMPUNAVAILABLE && err != WINNONBLOCKING {
					close = true;
				}
				break;
			} else {
				let wlen: usize = try_into!(wlen)?;

				(**guard).write_buffer.drain(0..wlen);
				(**guard).write_buffer.shrink_to_fit();
			}
		}

		if !rem {
			(**guard).unset_flag(WRITE_STATE_FLAG_PENDING);
		}

		Ok(close)
	}

	fn process_thread_pool_panic(_id: u128, _e: Box<dyn Any + Send>) -> Result<(), Error> {
		Ok(())
	}

	fn stop(&mut self) -> Result<(), Error> {
		// stop thread pool and all threads
		match &mut self.stopper {
			Some(ref mut stopper) => stopper.stop()?,
			None => {}
		}
		for i in 0..self.config.threads {
			wlock!(self.state[i]).stop = true;
			self.wakeups[i].wakeup()?;
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
	fn new(wakeups: Array<Wakeup>, tid: usize) -> Result<Self, Error> {
		let ret_event_count = 0;
		let ret_events = [Event::empty(); MAX_RET_HANDLES];
		let in_events = vec![];
		let id_hash = HashMap::new();
		let handle_hash = HashMap::new();
		Ok(Self {
			ret_event_count,
			ret_events,
			#[cfg(target_os = "linux")]
			linux_ctx: LinuxContext::new()?,
			in_events,
			id_hash,
			handle_hash,
			wakeups,
			tid,
			last_housekeeping: 0,
		})
	}
}
