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
use crate::types::{
	ConnectionImpl, EventHandlerCallbacks, EventHandlerConfig, EventHandlerImpl, UserContextImpl,
};
use crate::{ClientConnection, Connection, EventHandler, ServerConnection, UserContext};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::any::Any;

debug!();

impl UserContext for UserContextImpl {}

impl ClientConnection for ConnectionImpl {}

impl ServerConnection for ConnectionImpl {}

impl Connection for ConnectionImpl {}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext>, Box<dyn Any + Send>) -> Result<(), Error>
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
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Box<dyn Connection>, &mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = Self::build_config(configs)?;
		Ok(Self {
			callbacks: EventHandlerCallbacks {
				on_read: None,
				on_accept: None,
				on_close: None,
				on_panic: None,
				on_housekeeper: None,
			},
			config,
		})
	}

	fn start_impl(&mut self) -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(self.config.threads))?;

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			Self::process_thread_pool_panic(id, e)?;
			Ok(())
		})?;

		tp.start()?;

		for _ in 0..self.config.threads {
			let config = self.config.clone();
			let callbacks = self.callbacks.clone();
			execute!(tp, {
				Self::execute_thread(config, callbacks)?;
				Ok(())
			})?;
		}
		Ok(())
	}

	fn build_config(configs: Vec<ConfigOption>) -> Result<EventHandlerConfig, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(vec![CN::EvhThreads, CN::Debug], vec![])?;

		let threads = config.get_or_usize(&CN::EvhThreads, EVH_DEFAULT_THREADS);
		let debug = config.get_or_bool(&CN::Debug, false);
		Ok(EventHandlerConfig { threads, debug })
	}

	fn execute_thread(
		config: EventHandlerConfig,
		mut callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	) -> Result<(), Error> {
		info!("execute thread")?;

		let mut count = 0;
		loop {
			if config.debug {
				info!("Thread loop {}", count)?;
			}
			match &mut callbacks.on_read {
				Some(ref mut callback) => {
					let mut conn: Box<dyn Connection> = Box::new(ConnectionImpl {});
					let mut ctx: Box<dyn UserContext> = Box::new(UserContextImpl {});
					callback(&mut conn, &mut ctx)?;
				}
				None => {}
			}

			match &mut callbacks.on_accept {
				Some(ref mut callback) => {
					let mut conn: Box<dyn Connection> = Box::new(ConnectionImpl {});
					let mut ctx: Box<dyn UserContext> = Box::new(UserContextImpl {});
					callback(&mut conn, &mut ctx)?;
				}
				None => {}
			}

			match &mut callbacks.on_close {
				Some(ref mut callback) => {
					let mut conn: Box<dyn Connection> = Box::new(ConnectionImpl {});
					let mut ctx: Box<dyn UserContext> = Box::new(UserContextImpl {});
					callback(&mut conn, &mut ctx)?;
				}
				None => {}
			}

			match &mut callbacks.on_housekeeper {
				Some(ref mut callback) => {
					let mut ctx: Box<dyn UserContext> = Box::new(UserContextImpl {});
					callback(&mut ctx)?;
				}
				None => {}
			}

			match &mut callbacks.on_panic {
				Some(ref mut callback) => {
					let mut ctx: Box<dyn UserContext> = Box::new(UserContextImpl {});
					let e = Box::new("panic msg");
					callback(&mut ctx, e)?;
				}
				None => {}
			}

			count += 1;
			if count > 3 {
				break;
			}

			std::thread::sleep(std::time::Duration::from_millis(1000));
		}
		Ok(())
	}

	fn process_thread_pool_panic(_id: u128, _e: Box<dyn Any + Send>) -> Result<(), Error> {
		Ok(())
	}
}
