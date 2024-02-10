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

use crate::types::{FutureWrapper, Lock, ThreadPoolImpl, ThreadPoolState};
use crate::{
	Builder, LockBox, PoolResult, ThreadPool, ThreadPoolConfig, ThreadPoolExecutor,
	ThreadPoolStopper,
};
use bmw_deps::futures::executor::block_on;
use bmw_err::{err, ErrKind, Error};
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
	#[cfg(test)]
	pub(crate) fn new_with_on_panic_and_t(
		config: ThreadPoolConfig,
		_on_panic: OnPanic,
		_t: T,
	) -> Result<Self, Error> {
		Self::new(config)
	}

	pub(crate) fn new(config: ThreadPoolConfig) -> Result<Self, Error> {
		if config.min_size == 0 || config.min_size > config.max_size {
			let fmt = "min_size must be > 0 and < max_size";
			return Err(err!(ErrKind::Configuration, fmt));
		}
		let waiting = 0;
		let cur_size = config.min_size;
		let config_clone = config.clone();
		let stop = false;
		let tps = ThreadPoolState {
			waiting,
			cur_size,
			config,
			stop,
		};
		let state = Builder::build_lock_box(tps)?;

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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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
				let mut id = Builder::build_lock(0)?;
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

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::execute;
	use crate::types::ThreadPoolImpl;
	use crate::{lock, lock_box, Builder, Lock, PoolResult, ThreadPool, ThreadPoolConfig};
	use bmw_err::{err, ErrKind, Error};
	use bmw_log::*;
	use std::thread::sleep;
	use std::time::Duration;

	info!();

	#[test]
	fn test_threadpool1() -> Result<(), Error> {
		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 10,
			max_size: 10,
			..Default::default()
		})?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		// simple execution, return value
		let res = tp.execute(async move { Ok(1) }, 0)?;
		assert_eq!(res.recv()?, PoolResult::Ok(1));

		// increment value using locks
		let mut x = lock!(1)?;
		let x_clone = x.clone();
		tp.execute(
			async move {
				**x.wlock()?.guard() = 2;
				Ok(1)
			},
			0,
		)?;

		let mut count = 0;
		loop {
			count += 1;
			assert!(count < 500);
			sleep(Duration::from_millis(10));
			if **x_clone.rlock()?.guard() == 2 {
				break;
			}
		}

		// return an error
		let res = tp.execute(async move { Err(err!(ErrKind::Test, "test")) }, 0)?;

		assert_eq!(res.recv()?, PoolResult::Err(err!(ErrKind::Test, "test")));

		// handle panic
		let res = tp.execute(
			async move {
				let x: Option<u32> = None;
				Ok(x.unwrap())
			},
			0,
		)?;

		assert!(res.recv().is_err());

		// 10 more panics to ensure pool keeps running
		for _ in 0..10 {
			let res = tp.execute(
				async move {
					let x: Option<u32> = None;
					Ok(x.unwrap())
				},
				0,
			)?;

			assert!(res.recv().is_err());
		}

		// now do a regular request
		let res = tp.execute(async move { Ok(5) }, 0)?;
		assert_eq!(res.recv()?, PoolResult::Ok(5));

		sleep(Duration::from_millis(1000));
		info!("test sending errors")?;

		// send an error and ignore the response
		{
			let res = tp.execute(async move { Err(err!(ErrKind::Test, "")) }, 0)?;
			drop(res);
			sleep(Duration::from_millis(1000));
		}
		{
			let res = tp.execute(async move { Err(err!(ErrKind::Test, "test")) }, 0)?;
			assert_eq!(res.recv()?, PoolResult::Err(err!(ErrKind::Test, "test")));
		}
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_sizing() -> Result<(), Error> {
		let mut tp = ThreadPoolImpl::new(ThreadPoolConfig {
			min_size: 2,
			max_size: 4,
			..Default::default()
		})?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;
		let mut v = vec![];

		let x = lock!(0)?;

		loop {
			sleep(Duration::from_millis(10));
			{
				let state = tp.state.rlock()?;
				if (**state.guard()).waiting == 2 {
					break;
				}
			}
		}
		assert_eq!(tp.size()?, 2);
		// first use up all the min_size threads
		let y = lock!(0)?;
		for _ in 0..2 {
			let mut y_clone = y.clone();
			let x_clone = x.clone();
			let res = tp.execute(
				async move {
					**(y_clone.wlock()?.guard()) += 1;
					loop {
						if **(x_clone.rlock()?.guard()) != 0 {
							break;
						}
						sleep(Duration::from_millis(50));
					}
					Ok(1)
				},
				0,
			)?;
			v.push(res);
		}
		loop {
			sleep(Duration::from_millis(100));
			{
				let y = y.rlock()?;
				if (**y.guard()) == 2 {
					break;
				}
			}
		}
		assert_eq!(tp.size()?, 3);

		// confirm we can still process
		for _ in 0..10 {
			let mut x_clone = x.clone();
			let res = tp.execute(
				async move {
					**(x_clone.wlock()?.guard()) = 1;

					Ok(2)
				},
				0,
			)?;
			assert_eq!(res.recv()?, PoolResult::Ok(2));
		}

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 2);

		let mut i = 0;
		for res in v {
			assert_eq!(res.recv()?, PoolResult::Ok(1));
			info!("res complete {}", i)?;
			i += 1;
		}

		sleep(Duration::from_millis(1000));

		let mut x2 = lock!(0)?;

		// confirm that the maximum is in place

		// block all 4 threads waiting on x2
		for _ in 0..4 {
			let mut x2_clone = x2.clone();
			tp.execute(
				async move {
					info!("x0a")?;
					loop {
						if **(x2_clone.rlock()?.guard()) != 0 {
							break;
						}
						sleep(Duration::from_millis(50));
					}
					info!("x2")?;
					**(x2_clone.wlock()?.guard()) += 1;

					Ok(0)
				},
				0,
			)?;
		}

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 4);

		// confirm that the next thread cannot be processed
		let mut x2_clone = x2.clone();
		tp.execute(
			async move {
				info!("x0")?;
				**(x2_clone.wlock()?.guard()) += 1;
				info!("x1")?;
				Ok(0)
			},
			0,
		)?;

		// wait
		sleep(Duration::from_millis(2_000));

		// confirm situation hasn't changed
		assert_eq!(**(x2.rlock()?.guard()), 0);

		// unlock the threads by setting x2 to 1
		**(x2.wlock()?.guard()) = 1;

		// wait
		sleep(Duration::from_millis(4_000));
		assert_eq!(**(x2.rlock()?.guard()), 6);
		info!("exit all")?;

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 2);

		tp.stop()?;
		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 0);

		Ok(())
	}

	#[test]
	fn test_stop() -> Result<(), Error> {
		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 2,
			max_size: 4,
			..Default::default()
		})?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		sleep(Duration::from_millis(1000));
		assert_eq!(tp.size()?, 2);
		tp.execute(async move { Ok(()) }, 0)?;

		sleep(Duration::from_millis(1000));
		info!("stopping pool")?;
		assert_eq!(tp.size()?, 2);
		tp.stop()?;

		sleep(Duration::from_millis(1000));
		assert_eq!(tp.size()?, 0);
		Ok(())
	}

	#[test]
	fn pass_to_threads() -> Result<(), Error> {
		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 2,
			max_size: 4,
			..Default::default()
		})?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let tp = lock!(tp)?;
		for _ in 0..6 {
			let tp = tp.clone();
			std::thread::spawn(move || -> Result<(), Error> {
				let tp = tp.rlock()?;
				execute!((**tp.guard()), {
					info!("executing in thread pool")?;
					Ok(1)
				})?;
				Ok(())
			});
		}
		Ok(())
	}

	#[test]
	fn test_bad_configs() -> Result<(), Error> {
		assert!(ThreadPoolImpl::new_with_on_panic_and_t(
			ThreadPoolConfig {
				min_size: 5,
				max_size: 4,
				..Default::default()
			},
			move |_, _| -> Result<(), Error> { Ok(()) },
			0u32
		)
		.is_err());

		assert!(ThreadPoolImpl::new_with_on_panic_and_t(
			ThreadPoolConfig {
				min_size: 0,
				max_size: 4,
				..Default::default()
			},
			move |_, _| -> Result<(), Error> { Ok(()) },
			0u32
		)
		.is_err());

		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 5,
			max_size: 6,
			..Default::default()
		})?;
		tp.set_on_panic(move |_, _| -> Result<(), Error> { Ok(()) })?;

		assert_eq!(
			tp.execute(async move { Ok(()) }, 0).unwrap_err(),
			err!(
				ErrKind::IllegalState,
				"Thread pool has not been initialized"
			)
		);
		Ok(())
	}

	#[test]
	fn test_no_on_panic_handler() -> Result<(), Error> {
		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 1,
			max_size: 1,
			..Default::default()
		})?;

		tp.set_on_panic(move |_, _| -> Result<(), Error> { Ok(()) })?;
		tp.set_on_panic_none()?;
		tp.start()?;
		tp.execute(
			async move {
				if true {
					panic!("2");
				}
				Ok(())
			},
			0,
		)?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_on_panic_error() -> Result<(), Error> {
		let mut tp = Builder::build_thread_pool(ThreadPoolConfig {
			min_size: 1,
			max_size: 1,
			..Default::default()
		})?;

		let mut count = lock_box!(0)?;
		let count_clone = count.clone();
		tp.set_on_panic(move |_, _| -> Result<(), Error> {
			let mut count = count.wlock()?;
			**count.guard() += 1;
			return Err(err!(ErrKind::Test, "panic errored"));
		})?;

		// test that unstarted pool returns err
		let executor = tp.executor()?;
		assert!(executor.execute(async move { Ok(0) }, 0,).is_err());

		tp.start()?;

		tp.execute(
			async move {
				panic!("err");
			},
			0,
		)?;

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(1));
			if **(count_clone.rlock()?.guard()) != 1 && count < 5_000 {
				continue;
			}
			assert_eq!(**(count_clone.rlock()?.guard()), 1);
			break;
		}

		// ensure processing can still occur (1 thread means that thread recovered after
		// panic)
		let res = tp.execute(
			async move {
				info!("execute")?;
				Ok(1)
			},
			0,
		)?;

		assert_eq!(res.recv()?, PoolResult::Ok(1));

		Ok(())
	}
}
