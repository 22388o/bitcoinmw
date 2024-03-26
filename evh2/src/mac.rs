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

use crate::types::{EventHandlerConfig, EventHandlerContext};
use bmw_err::*;
use std::os::fd::RawFd;

pub(crate) type Handle = RawFd;

pub(crate) fn write_impl(handle: Handle, buf: &[u8]) -> Result<isize, Error> {
	todo!()
}

pub(crate) fn wakeup_impl() -> Result<(Handle, Handle), Error> {
	todo!()
}

pub(crate) fn close_impl(handle: Handle) -> Result<(), Error> {
	todo!()
}

pub(crate) fn close_impl_ctx(handle: Handle, ctx: &mut EventHandlerContext) -> Result<(), Error> {
	todo!()
}

pub(crate) fn read_impl(handle: Handle, buf: &mut [u8]) -> Result<Option<usize>, Error> {
	todo!()
}

pub(crate) fn accept_impl(fd: RawFd) -> Result<Option<Handle>, Error> {
	todo!()
}

pub(crate) fn create_connection(host: &str, port: u16) -> Result<Handle, Error> {
	todo!()
}

pub(crate) fn create_listener(addr: &str, size: usize) -> Result<Handle, Error> {
	todo!()
}

pub(crate) fn get_events(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<(), Error> {
	todo!()
}
