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

use crate::types::{ConnectionType, DebugInfo, EventHandlerImpl};
use crate::{Connection, EventHandler, EvhBuilder, UserContext};
use bmw_conf::ConfigOption;
use bmw_err::*;
use bmw_log::*;
use std::any::Any;

info!();

impl EvhBuilder {
	/// Builds a [`crate::EventHandler`] with the specified vector of [`bmw_conf::ConfigOption`].
	/// This is generally not called directly, but instead done indirectly by calling the
	/// [`crate::evh!`] or [`crate::evh_oro`] macros.
	pub fn build_evh<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>(
		configs: Vec<ConfigOption>,
	) -> Result<
		Box<dyn EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> + Send + Sync>,
		Error,
	>
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
		Ok(Box::new(EventHandlerImpl::new(configs)?))
	}

	/// Builds a server side [`crate::Connection`] that can be added to the
	/// [`crate::EventHandler`] via the [`crate::EventHandler::add_server_connection`]
	/// function.
	/// # Input Parameters
	/// addr - The TCP/IP address to bind to. (e.g. "127.0.0.1", "0.0.0.0", or "`[::1]`").
	/// IPV6 is supported.
	/// backlog - The parameter passed to the [`bmw_deps::libc::listen`] as the backlog parameter. This
	/// value is used on Unix systems, but ignored on Windows.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] if an i/o error occurs.
	pub fn build_server_connection(addr: &str, backlog: usize) -> Result<Connection, Error> {
		let handle = create_listener(addr, backlog, &DebugInfo::default())?;
		Ok(Connection::new(
			handle,
			None,
			None,
			ConnectionType::Server,
			DebugInfo::default(),
		)?)
	}

	/// Builds a client side [`crate::Connection`] that can be added to the
	/// [`crate::EventHandler`] via the [`crate::EventHandler::add_client_connection`]
	/// function.
	/// # Input Parameters
	/// host - The remote host to bind to.
	/// port - The parameter passed to the [`bmw_deps::libc::listen`] as the backlog parameter. This
	/// value is used on Unix systems, but ignored on Windows.
	/// # Returns
	/// On success, [`unit`] is returned and on failure, [`bmw_err::Error`] is returned.
	/// # Errors
	/// [`bmw_err::ErrKind::IO`] if an i/o error occurs.
	pub fn build_client_connection(host: &str, port: u16) -> Result<Connection, Error> {
		let handle = create_connection(host, port)?;
		Ok(Connection::new(
			handle,
			None,
			None,
			ConnectionType::Client,
			DebugInfo::default(),
		)?)
	}
}
