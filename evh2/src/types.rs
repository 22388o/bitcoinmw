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

use bmw_err::*;
use std::any::Any;
use std::pin::Pin;

pub trait EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	fn start(&mut self) -> Result<(), Error>;
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error>;
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error>;
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error>;
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error>;
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
}

pub trait ClientConnection {}

pub trait ServerConnection {}

pub trait Connection {}

pub trait WriteHandle {}

pub trait UserContext {}

pub struct EvhBuilder {}

// crate local structures

pub(crate) struct ConnectionImpl {}
pub(crate) struct UserContextImpl {}

#[derive(Clone)]
pub(crate) struct EventHandlerConfig {
	pub(crate) threads: usize,
	pub(crate) debug: bool,
}
pub(crate) struct EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	pub(crate) callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	pub(crate) config: EventHandlerConfig,
}

#[derive(Clone)]
pub(crate) struct EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
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
	pub(crate) on_read: Option<Pin<Box<OnRead>>>,
	pub(crate) on_accept: Option<Pin<Box<OnAccept>>>,
	pub(crate) on_close: Option<Pin<Box<OnClose>>>,
	pub(crate) on_panic: Option<Pin<Box<OnPanic>>>,
	pub(crate) on_housekeeper: Option<Pin<Box<OnHousekeeper>>>,
}
