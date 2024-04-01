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
	DebugInfo, Event, EventHandlerConfig, EventHandlerContext, EventType, EventTypeIn,
};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::kqueue_sys::{kevent, kqueue, EventFilter, EventFlag, FilterFlag};
use bmw_deps::libc::{
	self, accept, c_int, c_void, close, fcntl, listen, pipe, read, sockaddr, socket, timespec,
	write, F_SETFL, O_NONBLOCK,
};
use bmw_deps::nix::sys::socket::{bind, SockaddrIn, SockaddrIn6};
use bmw_err::*;
use bmw_log::*;
use std::mem::{size_of, zeroed};
use std::net::TcpStream;
use std::os::fd::IntoRawFd;
use std::os::fd::RawFd;
use std::str::FromStr;
use std::time::Duration;

info!();

pub(crate) type Handle = RawFd;

pub(crate) struct MacosContext {
	pub(crate) selector: Handle,
}

impl MacosContext {
	pub(crate) fn new() -> Result<Self, Error> {
		Ok(Self {
			selector: unsafe { kqueue() },
		})
	}
}

pub(crate) fn write_impl(handle: Handle, buf: &[u8]) -> Result<isize, Error> {
	set_errno(Errno(0));
	let cbuf: *const c_void = buf as *const _ as *const c_void;
	Ok(unsafe { write(handle, cbuf, buf.len().into()) })
}

pub(crate) fn wakeup_impl() -> Result<(Handle, Handle), Error> {
	set_errno(Errno(0));
	let mut retfds = [0i32; 2];
	let fds: *mut c_int = &mut retfds as *mut _ as *mut c_int;
	unsafe { pipe(fds) };
	unsafe { fcntl(retfds[0], F_SETFL, O_NONBLOCK) };
	unsafe { fcntl(retfds[1], F_SETFL, O_NONBLOCK) };
	Ok((retfds[0], retfds[1]))
}

pub(crate) fn close_impl(handle: Handle) -> Result<(), Error> {
	debug!("closing {}", handle)?;
	set_errno(Errno(0));
	unsafe {
		close(handle);
	}
	Ok(())
}

pub(crate) fn close_impl_ctx(handle: Handle, _ctx: &mut EventHandlerContext) -> Result<(), Error> {
	set_errno(Errno(0));
	debug!("close_impl_ctx handle = {}", handle)?;
	close_impl(handle)?;
	Ok(())
}

pub(crate) fn read_impl(
	handle: Handle,
	buf: &mut [u8],
	debug_info: &DebugInfo,
) -> Result<Option<usize>, Error> {
	set_errno(Errno(0));
	let cbuf: *mut c_void = buf as *mut _ as *mut c_void;
	let rlen = unsafe { read(handle, cbuf, buf.len()) };

	if rlen < 0 {
		let errno = errno();
		if errno.0 == libc::EAGAIN && !debug_info.is_os_error() {
			debug!("--------------------EAGAIN------------------")?;
			Ok(None)
		} else {
			let text = format!(
				"I/O error occurred while reading handle {}. Error msg: {}",
				handle, errno
			);
			Err(err!(ErrKind::IO, text))
		}
	} else {
		Ok(Some(try_into!(rlen)?))
	}
}

pub(crate) fn accept_impl(fd: RawFd, debug_info: &DebugInfo) -> Result<Option<Handle>, Error> {
	set_errno(Errno(0));
	let handle = unsafe {
		accept(
			fd,
			&mut sockaddr { ..zeroed() },
			&mut (size_of::<sockaddr>() as u32).try_into()?,
		)
	};

	debug!("accept handle = {}", handle)?;

	if handle < 0 {
		if errno().0 == libc::EAGAIN && !debug_info.is_os_error() {
			// would block, return the negative number
			return Ok(None);
		}
		let fmt = format!("accept failed: {}", errno());
		return Err(err!(ErrKind::IO, fmt));
	}

	unsafe {
		fcntl(handle, F_SETFL, O_NONBLOCK);
	}

	Ok(Some(handle))
}

pub(crate) fn create_connection(host: &str, port: u16) -> Result<Handle, Error> {
	let strm = TcpStream::connect(format!("{}:{}", host, port))?;
	strm.set_nonblocking(true)?;
	let fd = strm.into_raw_fd();

	Ok(fd)
}

pub(crate) fn create_listener(
	addr: &str,
	size: usize,
	debug_info: &DebugInfo,
) -> Result<Handle, Error> {
	set_errno(Errno(0));
	let fd = match SockaddrIn::from_str(addr) {
		Ok(sock_addr) => {
			let fd = unsafe { socket(libc::AF_INET, libc::SOCK_STREAM, 0) };

			unsafe {
				let optval: libc::c_int = 1;
				libc::setsockopt(
					fd,
					libc::SOL_SOCKET,
					libc::SO_REUSEADDR,
					&optval as *const _ as *const libc::c_void,
					std::mem::size_of_val(&optval) as libc::socklen_t,
				);
			}

			bind(fd, &sock_addr)?;
			fd
		}
		Err(_) => {
			let sock_addr = SockaddrIn6::from_str(addr)?;
			let fd = unsafe { socket(libc::AF_INET6, libc::SOCK_STREAM, 0) };

			unsafe {
				let optval: libc::c_int = 1;
				libc::setsockopt(
					fd,
					libc::SOL_SOCKET,
					libc::SO_REUSEADDR,
					&optval as *const _ as *const libc::c_void,
					std::mem::size_of_val(&optval) as libc::socklen_t,
				);
			}

			bind(fd, &sock_addr)?;
			fd
		}
	};

	unsafe {
		if listen(fd, try_into!(size)?) != 0 || debug_info.is_os_error() {
			return Err(err!(ErrKind::IO, "listen failed"));
		}
		fcntl(fd, F_SETFL, O_NONBLOCK);
	}
	debug!("ret fd = {}", fd)?;
	Ok(fd)
}

pub(crate) fn update_ctx(
	_ctx: &mut EventHandlerContext,
	_handle: Handle,
	_etype: EventTypeIn,
) -> Result<(), Error> {
	Ok(())
}

pub(crate) fn get_events(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<(), Error> {
	let kevs = get_events_in(config, ctx)?;
	get_events_out(config, ctx, kevs)?;
	Ok(())
}

pub(crate) fn get_events_in(
	_config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<Vec<kevent>, Error> {
	let mut kevs = vec![];
	for evt in &ctx.in_events {
		match evt.etype {
			EventTypeIn::Read => {
				kevs.push(kevent::new(
					evt.handle.try_into()?,
					EventFilter::EVFILT_READ,
					EventFlag::EV_ADD | EventFlag::EV_CLEAR,
					FilterFlag::empty(),
				));
			}
			EventTypeIn::Write => {
				kevs.push(kevent::new(
					evt.handle.try_into()?,
					EventFilter::EVFILT_WRITE,
					EventFlag::EV_ADD | EventFlag::EV_CLEAR,
					FilterFlag::empty(),
				));
			}
		}
	}
	Ok(kevs)
}

pub(crate) fn get_events_out(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
	kevs: Vec<kevent>,
) -> Result<(), Error> {
	let mut ret_kevs = vec![];
	for _ in 0..MAX_RET_HANDLES {
		ret_kevs.push(kevent::new(
			0,
			EventFilter::EVFILT_SYSCOUNT,
			EventFlag::empty(),
			FilterFlag::empty(),
		));
	}
	let results = {
		set_errno(Errno(0));
		let (requested, _lock) = ctx.wakeups[ctx.tid].pre_block()?;
		let timeout = Duration::from_millis(if requested { 0 } else { config.timeout.into() });
		unsafe {
			kevent(
				ctx.macos_ctx.selector,
				kevs.as_ptr(),
				kevs.len().try_into()?,
				ret_kevs.as_mut_ptr(),
				ret_kevs.len().try_into()?,
				&duration_to_timespec(timeout),
			)
		}
	};

	ctx.wakeups[ctx.tid].post_block()?;

	if results < 0 {
		return Err(err!(
			ErrKind::IO,
			format!("kqueue selector had an error: {}", errno())
		));
	}

	let results: usize = try_into!(results)?;

	ctx.ret_event_count = 0;
	for i in 0..results {
		let is_write = ret_kevs[i].filter == EventFilter::EVFILT_WRITE;
		let is_read = ret_kevs[i].filter == EventFilter::EVFILT_READ;
		if is_write {
			ctx.ret_events[ctx.ret_event_count] =
				Event::new(try_into!(ret_kevs[i].ident)?, EventType::ReadWrite);
			ctx.ret_event_count += 1;
		} else if is_read {
			ctx.ret_events[ctx.ret_event_count] =
				Event::new(try_into!(ret_kevs[i].ident)?, EventType::Read);
			ctx.ret_event_count += 1;
		}
	}

	Ok(())
}

fn duration_to_timespec(d: Duration) -> timespec {
	let tv_sec = d.as_secs() as i64;
	let tv_nsec = d.subsec_nanos() as i64;

	if tv_sec.is_negative() {
		panic!("Duration seconds is negative");
	}

	if tv_nsec.is_negative() {
		panic!("Duration nsecs is negative");
	}

	timespec { tv_sec, tv_nsec }
}
