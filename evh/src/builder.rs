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

use crate::types::EventHandlerImpl;
use crate::{
	AttachmentHolder, Builder, ConnectionData, EventHandler, EventHandlerConfig, ThreadContext,
};
use bmw_err::*;
use std::any::Any;

impl Builder {
	/// Builds a [`crate::EventHandler`] instance based on the specified
	/// [`crate::EventHandlerConfig`].
	pub fn build_evh<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>(
		config: EventHandlerConfig,
	) -> Result<impl EventHandler<OnRead, OnAccept, OnClose, HouseKeeper, OnPanic>, Error>
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
		EventHandlerImpl::new(config)
	}
}
