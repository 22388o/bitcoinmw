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
use bmw_deps::kqueue_sys::{kevent, EventFilter, EventFlag, FilterFlag};
use bmw_deps::libc::{
	self, accept, c_void, close, fcntl, listen, pipe, read, sockaddr, socket, timespec, write,
	F_SETFL, O_NONBLOCK,
};
use bmw_deps::nix::sys::socket::{bind, SockaddrIn, SockaddrIn6};
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::mem::{size_of, zeroed};
use std::net::TcpStream;
use std::os::raw::c_int;
use std::os::unix::prelude::RawFd;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub(crate) fn create_listeners_impl(
	size: usize,
	addr: &str,
	listen_size: usize,
	_reuse_port: bool,
) -> Result<Array<Handle>, Error> {
	let fd = match SockaddrIn::from_str(addr) {
		Ok(sock_addr) => {
			let fd = get_socket(libc::AF_INET)?;
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
			let fd = get_socket(libc::AF_INET6)?;
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

	let mut ret = array!(size, &0)?;
	for i in 0..size {
		ret[i] = fd;
	}

	Ok(ret)
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

pub(crate) fn close_impl(
	_ctx: &mut EventHandlerContext,
	handle: Handle,
	_partial: bool,
) -> Result<(), Error> {
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

pub(crate) fn get_events_impl(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
	wakeup_requested: bool,
	kevs: &mut Vec<kevent>,
	ret_kevs: &mut Vec<kevent>,
) -> Result<usize, Error> {
	debug!(
		"get_impl_mac: {}, pushing {} fd",
		ctx.tid,
		ctx.events_in.len()
	)?;
	kevs.clear();
	for evt in &ctx.events_in {
		match evt.etype {
			EventTypeIn::Accept => {
				kevs.push(kevent::new(
					evt.handle.try_into()?,
					EventFilter::EVFILT_READ,
					EventFlag::EV_ADD | EventFlag::EV_CLEAR,
					FilterFlag::empty(),
				));
			}
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
			EventTypeIn::Suspend => {
				kevs.push(kevent::new(
					evt.handle.try_into()?,
					EventFilter::EVFILT_READ,
					EventFlag::EV_DISABLE,
					FilterFlag::empty(),
				));
			}
			EventTypeIn::Resume => {
				kevs.push(kevent::new(
					evt.handle.try_into()?,
					EventFilter::EVFILT_READ,
					EventFlag::EV_ENABLE,
					FilterFlag::empty(),
				));
			}
		}
	}
	ctx.events_in.clear();
	ctx.events_in.shrink_to(config.max_events_in);

	let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
	let diff = now - ctx.now;
	let sleep = if wakeup_requested {
		0
	} else {
		config.housekeeping_frequency_millis.saturating_sub(diff)
	};
	set_errno(Errno(0));
	let ret_count = unsafe {
		kevent(
			ctx.selector,
			kevs.as_ptr(),
			kevs.len().try_into()?,
			ret_kevs.as_mut_ptr(),
			ret_kevs.len().try_into()?,
			&duration_to_timespec(Duration::from_millis(sleep as u64)),
		)
	};
	ctx.now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
	debug!("ret_count = {}", ret_count)?;
	if ret_count < 0 {
		return Err(err!(
			ErrKind::IO,
			format!("kqueue selector had an error: {}", errno())
		));
	}

	for i in 0..ret_count as usize {
		ctx.events[i] = Event {
			handle: ret_kevs[i].ident.try_into()?,
			etype: match ret_kevs[i].filter {
				EventFilter::EVFILT_READ => EventType::Read,
				EventFilter::EVFILT_WRITE => EventType::Write,
				_ => {
					return Err(err!(
						ErrKind::IO,
						format!(
							"unexpected event type returned by kqueue: {:?}",
							ret_kevs[i]
						)
					));
				}
			},
		};
		debug!("ev = {:?}", ctx.events[i])?;
	}

	Ok(ret_count.try_into()?)
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

fn get_socket(family: c_int) -> Result<RawFd, Error> {
	let mut count = 0;
	loop {
		count += 1;
		let raw_fd = unsafe { socket(family, libc::SOCK_STREAM, 0) };

		if raw_fd <= 0 {
			if count > 10 {
				return Err(err!(ErrKind::IO, "socket returned an invalid fd"));
			}
			sleep(Duration::from_millis(1));
		} else {
			return Ok(raw_fd);
		}
	}
}
