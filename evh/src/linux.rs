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

use crate::types::{
	Event, EventHandlerConfig, EventHandlerContext, EventType, EventTypeIn, Handle,
};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::libc::{
	self, accept, c_void, close, fcntl, listen, pipe, read, sockaddr, socket, write, F_SETFL,
	O_NONBLOCK,
};
use bmw_deps::nix::poll::PollTimeout;
use bmw_deps::nix::sys::epoll::{Epoll, EpollEvent, EpollFlags};
use bmw_deps::nix::sys::socket::{bind, SockaddrIn, SockaddrIn6};
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::mem::{self, size_of, zeroed};
use std::net::TcpStream;
use std::os::fd::BorrowedFd;
use std::os::raw::c_int;
use std::os::unix::prelude::RawFd;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

info!();

pub(crate) fn get_reader_writer() -> Result<
	(
		Handle,
		Handle,
		Option<Arc<TcpStream>>,
		Option<Arc<TcpStream>>,
	),
	Error,
> {
	let mut retfds = [0i32; 2];
	let fds: *mut c_int = &mut retfds as *mut _ as *mut c_int;
	unsafe { pipe(fds) };
	unsafe { fcntl(retfds[0], F_SETFL, O_NONBLOCK) };
	unsafe { fcntl(retfds[1], F_SETFL, O_NONBLOCK) };
	Ok((retfds[0], retfds[1], None, None))
}

pub(crate) fn read_bytes_impl(handle: Handle, buf: &mut [u8]) -> isize {
	let cbuf: *mut c_void = buf as *mut _ as *mut c_void;
	unsafe { read(handle, cbuf, buf.len()) }
}

pub(crate) fn write_bytes_impl(handle: Handle, buf: &[u8]) -> isize {
	let cbuf: *const c_void = buf as *const _ as *const c_void;
	unsafe { write(handle, cbuf, buf.len().into()) }
}

pub(crate) fn close_impl(
	ctx: &mut EventHandlerContext,
	handle: Handle,
	_partial: bool,
) -> Result<(), Error> {
	let handle_as_usize = handle.try_into()?;
	debug!("filter set remove {}, tid={}", handle_as_usize, ctx.tid)?;
	if handle_as_usize >= ctx.filter_set.len().try_into()? {
		ctx.filter_set.resize(handle_as_usize + 100, false);
	}
	ctx.filter_set.replace(handle_as_usize, false);
	unsafe {
		close(handle);
	}
	Ok(())
}

pub(crate) fn close_handle_impl(handle: Handle) -> Result<(), Error> {
	unsafe {
		close(handle);
	}
	Ok(())
}

pub(crate) fn accept_impl(fd: RawFd) -> Result<RawFd, Error> {
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
			return Ok(handle);
		}
		let fmt = format!("accept failed: {}", errno());
		return Err(err!(ErrKind::IO, fmt));
	}

	unsafe {
		fcntl(handle, F_SETFL, O_NONBLOCK);
	}

	Ok(handle)
}

pub(crate) fn create_listeners_impl(
	size: usize,
	addr: &str,
	listen_size: usize,
	reuse_port: bool,
) -> Result<Array<Handle>, Error> {
	let mut ret = array!(size, &0)?;
	let mut fd = setup_fd(reuse_port, addr, listen_size)?;
	ret[0] = fd;
	for i in 1..size {
		if reuse_port {
			fd = setup_fd(reuse_port, addr, listen_size)?;
		}
		ret[i] = fd;
	}

	Ok(ret)
}

fn setup_fd(reuse_port: bool, addr: &str, listen_size: usize) -> Result<RawFd, Error> {
	let fd = match SockaddrIn::from_str(addr) {
		Ok(sock_addr) => {
			let fd = get_socket(reuse_port, libc::AF_INET)?;

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
			let fd = get_socket(reuse_port, libc::AF_INET6)?;

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
		if listen(fd, try_into!(listen_size)?) != 0 {
			return Err(err!(ErrKind::IO, "listen failed"));
		}
		fcntl(fd, F_SETFL, O_NONBLOCK);
	}
	Ok(fd)
}

fn get_socket(reuseport: bool, family: c_int) -> Result<RawFd, Error> {
	let raw_fd = unsafe { socket(family, libc::SOCK_STREAM, 0) };

	if reuseport {
		let optval: libc::c_int = 1;
		unsafe {
			libc::setsockopt(
				raw_fd,
				libc::SOL_SOCKET,
				libc::SO_REUSEPORT,
				&optval as *const _ as *const libc::c_void,
				mem::size_of_val(&optval) as libc::socklen_t,
			)
		};

		unsafe {
			libc::setsockopt(
				raw_fd,
				libc::SOL_SOCKET,
				libc::SO_REUSEADDR,
				&optval as *const _ as *const libc::c_void,
				mem::size_of_val(&optval) as libc::socklen_t,
			)
		};
	}

	Ok(raw_fd)
}

pub(crate) fn get_events_impl(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
	wakeup_requested: bool,
	debug_err: bool,
) -> Result<usize, Error> {
	debug!("in get_events_impl in_count={}", ctx.events_in.len())?;
	for evt in &ctx.events_in {
		let mut interest = EpollFlags::empty();
		if evt.etype == EventTypeIn::Read
			|| evt.etype == EventTypeIn::Accept
			|| evt.etype == EventTypeIn::Resume
		{
			let fd = evt.handle;
			debug!("add in read fd = {},tid={}", fd, ctx.tid)?;
			if fd >= ctx.filter_set.len().try_into()? {
				ctx.filter_set.resize((fd + 100).try_into()?, false);
			}

			interest |= EpollFlags::EPOLLIN;
			interest |= EpollFlags::EPOLLET;
			interest |= EpollFlags::EPOLLRDHUP;

			let handle_as_usize: usize = fd.try_into()?;
			let mut event = EpollEvent::new(interest, evt.handle.try_into()?);
			debug!("fd={},ctx.tid={}", fd, ctx.tid)?;

			let res = if *ctx.filter_set.get(handle_as_usize).unwrap() {
				(*ctx.selector).modify(unsafe { BorrowedFd::borrow_raw(evt.handle) }, &mut event)
			} else {
				(*ctx.selector).add(unsafe { BorrowedFd::borrow_raw(evt.handle) }, event)
			};
			ctx.filter_set.replace(handle_as_usize, true);
			if res.is_err() || debug_err {
				warn!("Error epoll_ctl1: {:?}, fd={}, tid={}", res, fd, ctx.tid)?
			}
		} else if evt.etype == EventTypeIn::Write {
			let fd = evt.handle;
			debug!("add in write fd = {},tid={}", fd, ctx.tid)?;
			if fd > ctx.filter_set.len().try_into()? {
				ctx.filter_set.resize((fd + 100).try_into()?, false);
			}
			interest |= EpollFlags::EPOLLOUT;
			interest |= EpollFlags::EPOLLIN;
			interest |= EpollFlags::EPOLLRDHUP;
			interest |= EpollFlags::EPOLLET;

			let handle_as_usize: usize = fd.try_into()?;

			let mut event = EpollEvent::new(interest, evt.handle.try_into()?);
			debug!("fd={},ctx.tid={}", fd, ctx.tid)?;

			let res = if *ctx.filter_set.get(handle_as_usize).unwrap() {
				(*ctx.selector).modify(unsafe { BorrowedFd::borrow_raw(evt.handle) }, &mut event)
			} else {
				(*ctx.selector).add(unsafe { BorrowedFd::borrow_raw(evt.handle) }, event)
			};
			ctx.filter_set.replace(handle_as_usize, true);
			if res.is_err() || debug_err {
				warn!("Error epoll_ctl2: {:?}, fd={}, tid={}", res, fd, ctx.tid)?
			}
		} else if evt.etype == EventTypeIn::Suspend {
			let fd = evt.handle;
			debug!("add in write fd = {},tid={}", fd, ctx.tid)?;
			if fd > ctx.filter_set.len().try_into()? {
				ctx.filter_set.resize((fd + 100).try_into()?, false);
			}

			let handle_as_usize: usize = fd.try_into()?;
			ctx.filter_set.replace(handle_as_usize, false);

			let res = (*ctx.selector).delete(unsafe { BorrowedFd::borrow_raw(evt.handle) });
			if res.is_err() || debug_err {
				warn!("Error epoll_ctl3: {:?}, fd={}, tid={}", res, fd, ctx.tid)?
			}
		}
	}
	ctx.events_in.clear();
	ctx.events_in.shrink_to(config.max_events_in);

	let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
	let diff = now - ctx.now;
	let sleep: Duration = Duration::from_millis(if wakeup_requested {
		0
	} else {
		try_into!(config.housekeeping_frequency_millis.saturating_sub(diff))?
	});

	debug!("epoll_wait tid = {}", ctx.tid)?;
	let results = Epoll::wait(
		&*(ctx.selector),
		&mut ctx.epoll_events,
		PollTimeout::try_from(sleep).unwrap_or(0u8.into()),
	);

	ctx.now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

	let mut res_count = 0;
	if results.is_ok() && !debug_err {
		let results = results.unwrap();
		if results > 0 {
			for i in 0..results {
				if !(ctx.epoll_events[i].events() & EpollFlags::EPOLLOUT).is_empty() {
					ctx.events[res_count] = Event {
						handle: ctx.epoll_events[i].data() as Handle,
						etype: EventType::Write,
					};
					res_count += 1;
				}
				if !(ctx.epoll_events[i].events() & EpollFlags::EPOLLIN).is_empty() {
					ctx.events[res_count] = Event {
						handle: ctx.epoll_events[i].data() as Handle,
						etype: EventType::Read,
					};
					res_count += 1;
				}
			}
		}
	} else {
		warn!("epoll wait generated error: {:?}", results)?;
	}

	Ok(res_count)
}

#[cfg(test)]
mod test {
	use crate::linux::*;
	use crate::types::{EventHandlerContext, EventIn};
	use bmw_test::port::pick_free_port;
	use std::thread::sleep;
	use std::time::Duration;

	#[test]
	fn test_evh_linux() -> Result<(), Error> {
		sleep(Duration::from_millis(5_000));
		let mut ctx = EventHandlerContext::new(0, 10, 10, 10, 10)?;
		ctx.tid = 100;
		let handle = get_socket(true, libc::AF_INET)?;
		assert!(accept_impl(handle).is_err());
		ctx.filter_set.resize(1, false);
		assert_eq!(ctx.filter_set.len(), 1);
		close_impl(&mut ctx, handle, false)?;
		assert_eq!(ctx.filter_set.len(), 100 + handle as usize);
		ctx.filter_set.resize(10_100, false);
		let addr = &format!("127.0.0.1:{}", pick_free_port()?)[..];
		let ret = create_listeners_impl(1, addr, 10, true)?;
		ctx.filter_set.resize(1, false);
		ctx.events_in.push(EventIn {
			handle: ret[0],
			etype: EventTypeIn::Write,
		});
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		assert_eq!(get_events_impl(&config, &mut ctx, false, true)?, 0);
		assert_eq!(ctx.filter_set.len(), 100 + ret[0] as usize);
		ctx.filter_set.replace(ret[0] as usize, true);
		ctx.events_in.push(EventIn {
			handle: ret[0],
			etype: EventTypeIn::Read,
		});
		assert_eq!(get_events_impl(&config, &mut ctx, false, true)?, 0);
		ctx.filter_set.resize(1, false);
		ctx.events_in.push(EventIn {
			handle: ret[0],
			etype: EventTypeIn::Suspend,
		});
		assert_eq!(get_events_impl(&config, &mut ctx, false, true)?, 0);

		assert_eq!(ctx.filter_set.len(), 100 + ret[0] as usize);

		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let fmt = format!("[::1]:{}", port);
		assert!(setup_fd(false, &fmt, 10).is_ok());

		let port2 = pick_free_port()?;
		let addr = format!("127.0.0.1:{}", port2);
		assert!(create_listeners_impl(2, &addr, 10, true).is_ok());

		Ok(())
	}
}
