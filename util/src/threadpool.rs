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

use crate::constants::*;
use crate::types::{FutureWrapper, Lock, ThreadPoolImpl, ThreadPoolState};
use crate::{
	LockBox, PoolResult, ThreadPool, ThreadPoolConfig, ThreadPoolExecutor, ThreadPoolStopper,
	UtilBuilder,
};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{Config, ConfigBuilder, ConfigOption};
use bmw_deps::futures::executor::block_on;
use bmw_err::{err, Error};
use bmw_log::*;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

info!();

unsafe impl<T, E> Send for PoolResult<T, E> {}
unsafe impl<T, E> Sync for PoolResult<T, E> {}

impl Default for ThreadPoolConfig {
	fn default() -> Self {
		Self {
			min_size: 3,
			max_size: 7,
			sync_channel_size: 7,
		}
	}
}

impl<T, OnPanic> ThreadPoolImpl<T, OnPanic>
where
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	T: 'static + Send + Sync,
{
	/*
	#[cfg(test)]
	pub(crate) fn new_with_on_panic_and_t(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		Self::new(configs)
	}
	*/

	fn get_option(option: CN, config: &Box<dyn Config>, default: usize) -> usize {
		match config.get(&option) {
			Some(v) => match v {
				ConfigOption::MaxSize(v) => v,
				ConfigOption::MinSize(v) => v,
				ConfigOption::SyncChannelSize(v) => v,
				_ => default,
			},
			None => default,
		}
	}

	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(vec![CN::SyncChannelSize, CN::MinSize, CN::MaxSize], vec![])?;
		let min_size = THREAD_POOL_DEFAULT_MIN_SIZE;
		let min_size = Self::get_option(CN::MinSize, &config, min_size);
		let max_size = Self::get_option(CN::MaxSize, &config, min_size);
		let sync_channel_size = THREAD_POOL_DEFAULT_SYNC_CHANNEL_SIZE;
		let sync_channel_size = Self::get_option(CN::SyncChannelSize, &config, sync_channel_size);

		if min_size == 0 || min_size > max_size {
			let fmt = "min_size must be > 0 and <= max_size";
			return Err(err!(ErrKind::Configuration, fmt));
		}

		if sync_channel_size == 0 {
			let fmt = "sync_channel_size must be greater than 0";
			return Err(err!(ErrKind::Configuration, fmt));
		}

		let config = ThreadPoolConfig {
			min_size,
			max_size,
			sync_channel_size,
		};

		let waiting = 0;
		let cur_size = min_size;
		let config_clone = config.clone();
		let stop = false;
		let tps = ThreadPoolState {
			waiting,
			cur_size,
			config,
			stop,
		};
		let state = UtilBuilder::build_lock_box(tps)?;

		let config = config_clone;
		let rx = None;
		let tx = None;

		let ret = Self {
			config,
			tx,
			rx,
			state,
			on_panic: None,
		};
		Ok(ret)
	}

	fn run_thread<R: 'static>(
		rx: Arc<Mutex<Receiver<FutureWrapper<R>>>>,
		mut state: Box<dyn LockBox<ThreadPoolState>>,
		mut on_panic: Option<Pin<Box<OnPanic>>>,
	) -> Result<(), Error> {
		spawn(move || -> Result<(), Error> {
			loop {
				let rx = rx.clone();
				let mut state_clone = state.clone();
				let on_panic_clone = on_panic.clone();
				let mut id = UtilBuilder::build_lock(0)?;
				let id_clone = id.clone();
				let jh = spawn(move || -> Result<(), Error> {
					loop {
						let (next, do_run_thread) = {
							let mut do_run_thread = false;
							{
								let mut state = state_clone.wlock()?;
								let guard = &mut **state.guard();

								debug!("state = {:?}", guard)?;
								// we have too many threads or stop
								// was set. Exit this one.
								if guard.stop || guard.waiting >= guard.config.min_size {
									return Ok(());
								}
								guard.waiting += 1;
							}
							let rx = rx.lock()?;
							let ret = rx.recv()?;
							let mut state = state_clone.wlock()?;
							let guard = &mut **state.guard();
							guard.waiting = guard.waiting.saturating_sub(1);
							if guard.waiting == 0 {
								if guard.cur_size < guard.config.max_size {
									guard.cur_size += 1;
									do_run_thread = true;
								}
							}
							debug!("cur state = {:?}", guard)?;
							(ret, do_run_thread)
						};

						if do_run_thread {
							debug!("spawning a new thread")?;
							Self::run_thread(
								rx.clone(),
								state_clone.clone(),
								on_panic_clone.clone(),
							)?;
						}

						{
							let mut id = id.wlock()?;
							let guard = id.guard();
							(**guard) = next.id;
						}
						match block_on(next.f) {
							Ok(res) => {
								let send_res = next.tx.send(PoolResult::Ok(res));
								if send_res.is_err() {
									let e = send_res.unwrap_err();
									debug!("error sending response: {}", e)?;
								}
							}
							Err(e) => {
								debug!("sending an err")?;
								// if the reciever is not there we
								// just ignore the error that would
								// occur
								let _ = next.tx.send(PoolResult::Err(e));
							}
						}
					}
				});

				match jh.join() {
					Ok(_) => {
						let mut state = state.wlock()?;
						let guard = &mut **state.guard();
						guard.cur_size = guard.cur_size.saturating_sub(1);
						debug!("exiting a thread, ncur={}", guard.cur_size)?;
						break;
					} // reduce thread count so exit this one
					Err(e) => match on_panic.as_mut() {
						Some(on_panic) => {
							debug!("found an onpanic")?;
							let id = id_clone.rlock()?;
							let guard = id.guard();
							match on_panic(**guard, e) {
								Ok(_) => {}
								Err(e) => warn!("on_panic handler generated error: {}", e)?,
							}
						}
						None => {
							debug!("no onpanic")?;
						}
					},
				}
			}
			Ok(())
		});
		Ok(())
	}
}

impl<T, OnPanic> ThreadPool<T, OnPanic> for ThreadPoolImpl<T, OnPanic>
where
	T: 'static + Send + Sync,
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	fn execute<F>(&self, f: F, id: u128) -> Result<Receiver<PoolResult<T, Error>>, Error>
	where
		F: Future<Output = Result<T, Error>> + Send + 'static,
	{
		if self.tx.is_none() {
			let fmt = "Thread pool has not been initialized";
			return Err(err!(ErrKind::IllegalState, fmt));
		}

		let (tx, rx) = sync_channel::<PoolResult<T, Error>>(self.config.max_size);
		let fw = FutureWrapper {
			f: Box::pin(f),
			tx,
			id,
		};
		self.tx.as_ref().unwrap().send(fw)?;
		Ok(rx)
	}

	fn start(&mut self) -> Result<(), Error> {
		let (tx, rx) = sync_channel(self.config.sync_channel_size);
		let rx = Arc::new(Mutex::new(rx));
		self.rx = Some(rx.clone());
		self.tx = Some(tx.clone());
		for _ in 0..self.config.min_size {
			Self::run_thread(rx.clone(), self.state.clone(), self.on_panic.clone())?;
		}

		loop {
			sleep(Duration::from_millis(1));
			{
				let state = self.state.rlock()?;
				let guard = &**state.guard();
				if guard.waiting == self.config.min_size {
					break;
				}
			}
		}

		Ok(())
	}

	fn stop(&mut self) -> Result<(), Error> {
		let mut state = self.state.wlock()?;
		(**state.guard()).stop = true;
		self.tx = None;
		Ok(())
	}

	fn size(&self) -> Result<usize, Error> {
		let state = self.state.rlock()?;
		Ok((**state.guard()).cur_size)
	}

	fn stopper(&self) -> Result<ThreadPoolStopper, Error> {
		Ok(ThreadPoolStopper {
			state: self.state.clone(),
		})
	}

	fn executor(&self) -> Result<ThreadPoolExecutor<T>, Error> {
		Ok(ThreadPoolExecutor {
			tx: self.tx.clone(),
			config: self.config.clone(),
		})
	}

	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error> {
		self.on_panic = Some(Box::pin(on_panic));
		Ok(())
	}

	#[cfg(test)]
	fn set_on_panic_none(&mut self) -> Result<(), Error> {
		self.on_panic = None;
		Ok(())
	}
}

impl<T> ThreadPoolExecutor<T>
where
	T: Send + Sync,
{
	pub fn execute<F>(&self, f: F, id: u128) -> Result<Receiver<PoolResult<T, Error>>, Error>
	where
		F: Future<Output = Result<T, Error>> + Send + 'static,
	{
		if self.tx.is_none() {
			let fmt = "Thread pool has not been initialized";
			return Err(err!(ErrKind::IllegalState, fmt));
		}

		let (tx, rx) = sync_channel::<PoolResult<T, Error>>(self.config.max_size);
		let fw = FutureWrapper {
			f: Box::pin(f),
			tx,
			id,
		};
		self.tx.as_ref().unwrap().send(fw)?;
		Ok(rx)
	}
}

impl ThreadPoolStopper {
	/// Stop all threads in the thread pool from executing new tasks.
	/// note that this does not terminate the threads if they are idle, it
	/// will just make the threads end after their next task is executed.
	/// The main purpose of this function is so that the state can be stored
	/// in a struct, but caller must ensure that the threads stop.
	/// This is not the case with [`crate::ThreadPool::stop`] and that function
	/// should be used where possible.
	pub fn stop(&mut self) -> Result<(), Error> {
		(**self.state.wlock()?.guard()).stop = true;
		Ok(())
	}
}
