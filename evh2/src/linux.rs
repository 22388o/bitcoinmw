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
use crate::types::{Event, EventHandlerConfig, EventHandlerContext, EventType, EventTypeIn};
use bmw_deps::bitvec::vec::BitVec;
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::libc::{
	self, accept, c_int, c_void, close, fcntl, listen, pipe, read, sockaddr, socket, write,
	F_SETFL, O_NONBLOCK,
};
use bmw_deps::nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags};
use bmw_deps::nix::sys::socket::{bind, SockaddrIn, SockaddrIn6};
use bmw_err::*;
use bmw_log::*;
use std::mem::{size_of, zeroed};
use std::net::TcpStream;
use std::os::fd::IntoRawFd;
use std::os::fd::{BorrowedFd, RawFd};
use std::str::FromStr;
use std::sync::Arc;

info!();

pub(crate) type Handle = RawFd;

pub(crate) struct LinuxContext {
	selector: Arc<Epoll>,
	epoll_events: [EpollEvent; MAX_RET_HANDLES],
	filter_set: BitVec,
}

impl LinuxContext {
	pub(crate) fn new() -> Result<Self, Error> {
		let selector = Arc::new(Epoll::new(EpollCreateFlags::empty())?);
		let epoll_events: [EpollEvent; MAX_RET_HANDLES] = [EpollEvent::empty(); MAX_RET_HANDLES];
		let filter_set = BitVec::new();
		Ok(Self {
			selector,
			epoll_events,
			filter_set,
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

pub(crate) fn close_impl_ctx(handle: Handle, ctx: &mut EventHandlerContext) -> Result<(), Error> {
	set_errno(Errno(0));
	debug!("close_impl_ctx handle = {}", handle)?;
	close_impl(handle)?;
	let handle_as_usize = try_into!(handle)?;
	if handle_as_usize >= ctx.linux_ctx.filter_set.len() {
		ctx.linux_ctx
			.filter_set
			.resize(handle_as_usize + 100, false);
	}
	ctx.linux_ctx.filter_set.replace(handle_as_usize, false);
	Ok(())
}

pub(crate) fn read_impl(handle: Handle, buf: &mut [u8]) -> Result<Option<usize>, Error> {
	set_errno(Errno(0));
	let cbuf: *mut c_void = buf as *mut _ as *mut c_void;
	let rlen = unsafe { read(handle, cbuf, buf.len()) };

	if rlen < 0 {
		let errno = errno();
		if errno.0 == libc::EAGAIN {
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

pub(crate) fn accept_impl(fd: RawFd) -> Result<Option<Handle>, Error> {
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
		if errno().0 == libc::EAGAIN {
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

pub(crate) fn create_listener(addr: &str, size: usize) -> Result<Handle, Error> {
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
		if listen(fd, try_into!(size)?) != 0 {
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
	get_events_in(config, ctx)?;
	get_events_out(config, ctx)?;
	Ok(())
}

pub(crate) fn get_events_in(
	_config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<(), Error> {
	for evt in &ctx.in_events {
		let fd_usize: usize = try_into!(evt.handle)?;
		let fd_u64: u64 = try_into!(evt.handle)?;
		let filter_len = ctx.linux_ctx.filter_set.len();
		let mut interest = EpollFlags::empty();
		if fd_usize >= filter_len {
			ctx.linux_ctx.filter_set.resize(fd_usize + 100, false);
		}
		if evt.etype == EventTypeIn::Read {
			debug!("proc handle adding read to {}", evt.handle)?;
			interest |= EpollFlags::EPOLLIN;
			interest |= EpollFlags::EPOLLET;
			interest |= EpollFlags::EPOLLRDHUP;
			let mut event = EpollEvent::new(interest, fd_u64);
			if *ctx.linux_ctx.filter_set.get(fd_usize).unwrap() {
				(*ctx.linux_ctx.selector)
					.modify(unsafe { BorrowedFd::borrow_raw(evt.handle) }, &mut event)?;
			} else {
				(*ctx.linux_ctx.selector)
					.add(unsafe { BorrowedFd::borrow_raw(evt.handle) }, event)?;
			};

			ctx.linux_ctx.filter_set.replace(fd_usize, true);
		} else if evt.etype == EventTypeIn::Write {
			interest |= EpollFlags::EPOLLOUT;
			interest |= EpollFlags::EPOLLIN;
			interest |= EpollFlags::EPOLLRDHUP;
			interest |= EpollFlags::EPOLLET;

			let mut event = EpollEvent::new(interest, fd_u64);

			if *ctx.linux_ctx.filter_set.get(fd_usize).unwrap() {
				(*ctx.linux_ctx.selector)
					.modify(unsafe { BorrowedFd::borrow_raw(evt.handle) }, &mut event)?;
			} else {
				(*ctx.linux_ctx.selector)
					.add(unsafe { BorrowedFd::borrow_raw(evt.handle) }, event)?;
			};

			ctx.linux_ctx.filter_set.replace(fd_usize, true);
		}
	}

	Ok(())
}

pub(crate) fn get_events_out(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<(), Error> {
	let results = {
		let (requested, _lock) = ctx.wakeups[ctx.tid].pre_block()?;

		Epoll::wait(
			&*ctx.linux_ctx.selector,
			&mut ctx.linux_ctx.epoll_events,
			if requested { 0 } else { config.timeout },
		)?
	};

	ctx.wakeups[ctx.tid].post_block()?;

	ctx.ret_event_count = 0;
	for i in 0..results {
		let is_write = !(ctx.linux_ctx.epoll_events[i].events() & EpollFlags::EPOLLOUT).is_empty();
		let is_read = !(ctx.linux_ctx.epoll_events[i].events() & EpollFlags::EPOLLIN).is_empty();
		if is_read && is_write {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				ctx.linux_ctx.epoll_events[i].data() as Handle,
				EventType::ReadWrite,
			);
			ctx.ret_event_count += 1;
		} else if is_read {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				ctx.linux_ctx.epoll_events[i].data() as Handle,
				EventType::Read,
			);
			ctx.ret_event_count += 1;
		} else if is_write {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				ctx.linux_ctx.epoll_events[i].data() as Handle,
				EventType::Write,
			);
			ctx.ret_event_count += 1;
		}
	}

	Ok(())
}
