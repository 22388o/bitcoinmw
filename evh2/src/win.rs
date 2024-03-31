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
use bmw_deps::bitvec::vec::BitVec;
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::portpicker::pick_unused_port;
use bmw_deps::rand::random;
use bmw_deps::wepoll_sys::{
	epoll_create, epoll_ctl, epoll_data_t, epoll_event, epoll_wait, EPOLLIN, EPOLLONESHOT,
	EPOLLOUT, EPOLLRDHUP, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD,
};
use bmw_deps::windows_sys::Win32::Networking::WinSock::{
	accept, closesocket, ioctlsocket, recv, send, setsockopt, SOCKADDR,
};
use bmw_err::*;
use bmw_log::*;
use std::mem::{size_of, zeroed};
use std::net::{TcpListener, TcpStream};
use std::os::raw::{c_int, c_void};
use std::os::windows::io::IntoRawSocket;

info!();

const SOL_SOCKET: c_int = 0xFFFF;
const SO_SNDBUF: c_int = 0x1001;
const WINSOCK_BUF_SIZE: c_int = 100_000_000;

pub(crate) type Handle = usize;

pub(crate) struct WindowsContext {
	selector: Handle,
	filter_set: BitVec,
}

impl WindowsContext {
	pub(crate) fn new() -> Result<Self, Error> {
		let filter_set = BitVec::new();
		let selector = unsafe { epoll_create(1) } as usize;
		Ok(Self {
			selector,
			filter_set,
		})
	}
}

pub(crate) fn write_impl(handle: Handle, buf: &[u8]) -> Result<isize, Error> {
	let cbuf: *mut u8 = buf as *const _ as *mut u8;
	let len = try_into!(buf.len())?;
	let res = unsafe { send(handle, cbuf, len, 0) };
	Ok(try_into!(res)?)
}

pub(crate) fn wakeup_impl() -> Result<(Handle, Handle), Error> {
	let (port, listener) = loop {
		let port = pick_unused_port().unwrap_or(random());
		let listener = TcpListener::bind(format!("127.0.0.1:{}", port));
		match listener {
			Ok(listener) => break (port, listener),
			Err(_) => {} // try again
		}
	};

	let stream = TcpStream::connect(format!("127.0.0.1:{}", port))?;
	let listener = listener.accept()?;

	let listener_handle = try_into!(listener.0.into_raw_socket())?;
	let stream_handle = try_into!(stream.into_raw_socket())?;
	set_windows_socket_options(listener_handle)?;
	set_windows_socket_options(stream_handle)?;

	Ok((listener_handle, stream_handle))
}

pub(crate) fn close_impl(handle: Handle) -> Result<(), Error> {
	unsafe {
		closesocket(handle);
	}
	Ok(())
}

pub(crate) fn close_impl_ctx(handle: Handle, ctx: &mut EventHandlerContext) -> Result<(), Error> {
	let handle_as_usize: usize = try_into!(handle)?;

	if handle_as_usize >= ctx.windows_ctx.filter_set.len() {
		ctx.windows_ctx
			.filter_set
			.resize(handle_as_usize + 100, false);
	}

	ctx.windows_ctx.filter_set.replace(handle_as_usize, false);

	let data = epoll_data_t {
		fd: try_into!(handle)?,
	};

	let mut event = epoll_event { events: 0, data };

	set_errno(Errno(0));
	let res = unsafe {
		epoll_ctl(
			ctx.windows_ctx.selector as *mut c_void,
			EPOLL_CTL_DEL as i32,
			handle_as_usize,
			&mut event,
		)
	};

	if res < 0 {
		let e = errno();
		warn!(
			"epoll_ctl del error: {}, fd = {}, tid = {}",
			e, handle, ctx.tid
		)?;
	}

	close_impl(handle)?;

	Ok(())
}

pub(crate) fn read_impl(
	handle: Handle,
	buf: &mut [u8],
	debug_info: &DebugInfo,
) -> Result<Option<usize>, Error> {
	let cbuf: *mut u8 = buf as *mut _ as *mut u8;
	let len = try_into!(buf.len())?;
	let len = unsafe { recv(handle, cbuf, len, 0) };
	if len < 0 && errno().0 == WINNONBLOCKING && !debug_info.is_os_error() {
		Ok(None)
	} else if len < 0 {
		Err(err!(ErrKind::IO, "read err {}", errno()))
	} else {
		Ok(Some(try_into!(len)?))
	}
}

pub(crate) fn accept_impl(fd: RawFd, debug_info: &DebugInfo) -> Result<Option<Handle>, Error> {
	if debug_info.is_os_err() {
		return Err(err!(ErrKind::Test, "os error"));
	}
	let handle = unsafe {
		accept(
			handle,
			&mut SOCKADDR { ..zeroed() },
			&mut try_into!((size_of::<SOCKADDR>() as u32))?,
		)
	};

	if handle == usize::MAX {
		Ok(None)
	} else {
		Ok(Some(handle))
	}
}

pub(crate) fn create_connection(host: &str, port: u16) -> Result<Handle, Error> {
	let strm = TcpStream::connect(format!("{}:{}", host, port))?;
	strm.set_nonblocking(true)?;
	let fd = strm.into_raw_socket();

	Ok(try_into!(fd)?)
}

pub(crate) fn create_listener(
	addr: &str,
	size: usize,
	debug_info: &DebugInfo,
) -> Result<Handle, Error> {
	if debug_info.is_os_error() {
		return Err(err!(ErrKind::Test, "debug_info::os_error true"));
	}
	let handle = try_into!(TcpListener::bind(addr)?.into_raw_socket())?;
	let fionbio = 0x8004667eu32;
	if unsafe { ioctlsocket(handle, fionbio as c_int, &mut 1) } != 0 {
		Err(err!(
			ErrKind::IO,
			format!("complete fion with error: {}", errno().to_string())
		))
	} else {
		Ok(handle)
	}
}

pub(crate) fn update_ctx(
	ctx: &mut EventHandlerContext,
	handle: Handle,
	etype: EventTypeIn,
) -> Result<(), Error> {
	let fd_usize: usize = try_into!(handle)?;
	let fd_i32: i32 = try_into!(handle)?;

	let filter_len = ctx.windows_ctx.filter_set.len();
	if fd_usize >= filter_len {
		ctx.windows_ctx.filter_set.resize(fd_usize + 100, false);
	}

	let mut event = if etype == EventTypeIn::Read {
		debug!("proc handle adding read to {}", handle)?;
		let data = epoll_data_t { fd: fd_i32 };
		let event = epoll_event {
			events: EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
			data,
		};
		event
	} else {
		let data = epoll_data_t { fd: fd_i32 };
		let event = epoll_event {
			events: EPOLLIN | EPOLLOUT | EPOLLONESHOT | EPOLLRDHUP,
			data,
		};
		event
	};

	if *ctx.windows_ctx.filter_set.get(fd_usize).unwrap() {
		set_errno(Errno(0));
		let res = unsafe {
			epoll_ctl(
				ctx.windows_ctx.selector as *mut c_void,
				try_into!(EPOLL_CTL_MOD)?,
				fd_usize,
				&mut event,
			)
		};
		if res < 0 {
			warn!(
				"epoll_ctl1: {}, handle={}, tid={}",
				errno(),
				handle,
				ctx.tid
			)?;
		}
	} else {
		set_errno(Errno(0));
		let res = unsafe {
			epoll_ctl(
				ctx.windows_ctx.selector as *mut c_void,
				try_into!(EPOLL_CTL_ADD)?,
				fd_usize,
				&mut event,
			)
		};
		if res < 0 {
			warn!(
				"epoll_ctl1: {}, handle={}, tid={}",
				errno(),
				handle,
				ctx.tid
			)?;
		}
	}

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
		let fd_i32: i32 = try_into!(evt.handle)?;
		let filter_len = ctx.windows_ctx.filter_set.len();
		if fd_usize >= filter_len {
			ctx.windows_ctx.filter_set.resize(fd_usize + 100, false);
		}
		let mut event = if evt.etype == EventTypeIn::Read {
			debug!("proc handle adding read to {}", evt.handle)?;
			let data = epoll_data_t { fd: fd_i32 };
			let event = epoll_event {
				events: EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
				data,
			};
			event
		} else {
			let data = epoll_data_t { fd: fd_i32 };
			let event = epoll_event {
				events: EPOLLIN | EPOLLOUT | EPOLLONESHOT | EPOLLRDHUP,
				data,
			};
			event
		};
		if *ctx.windows_ctx.filter_set.get(fd_usize).unwrap() {
			set_errno(Errno(0));
			let res = unsafe {
				epoll_ctl(
					ctx.windows_ctx.selector as *mut c_void,
					try_into!(EPOLL_CTL_MOD)?,
					fd_usize,
					&mut event,
				)
			};
			if res < 0 {
				warn!(
					"epoll_ctl1: {}, handle={}, tid={}",
					errno(),
					evt.handle,
					ctx.tid
				)?;
			}
		} else {
			set_errno(Errno(0));
			let res = unsafe {
				epoll_ctl(
					ctx.windows_ctx.selector as *mut c_void,
					try_into!(EPOLL_CTL_ADD)?,
					fd_usize,
					&mut event,
				)
			};
			if res < 0 {
				warn!(
					"epoll_ctl1: {}, handle={}, tid={}",
					errno(),
					evt.handle,
					ctx.tid
				)?;
			}
		};

		ctx.windows_ctx.filter_set.replace(fd_usize, true);
	}

	Ok(())
}

pub(crate) fn get_events_out(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
) -> Result<(), Error> {
	let mut epoll_events: [epoll_event; MAX_RET_HANDLES as usize] = [epoll_event {
		events: 0,
		data: epoll_data_t { fd: 0 },
	}; MAX_RET_HANDLES as usize];
	set_errno(Errno(0));
	let results = {
		let (requested, _lock) = ctx.wakeups[ctx.tid].pre_block()?;

		unsafe {
			epoll_wait(
				ctx.windows_ctx.selector as *mut c_void,
				epoll_events.as_mut_ptr(),
				try_into!(MAX_RET_HANDLES)?,
				if requested { 0 } else { config.timeout.into() },
			)
		}
	};

	if results < 0 {
		let text = format!("wepoll error: {}", errno());
		return Err(err!(ErrKind::IO, text));
	}

	ctx.wakeups[ctx.tid].post_block()?;

	ctx.ret_event_count = 0;
	for i in 0..(results as usize) {
		let is_write = !((epoll_events[i].events & EPOLLOUT) == 0);
		let is_read = !((epoll_events[i].events & EPOLLIN) == 0);
		if is_read && is_write {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				unsafe { epoll_events[i].data.fd } as Handle,
				EventType::ReadWrite,
			);
			ctx.ret_event_count += 1;
		} else if is_read {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				unsafe { epoll_events[i].data.fd } as Handle,
				EventType::Read,
			);
			ctx.ret_event_count += 1;
		} else if is_write {
			ctx.ret_events[ctx.ret_event_count] = Event::new(
				unsafe { epoll_events[i].data.fd } as Handle,
				EventType::Write,
			);
			ctx.ret_event_count += 1;
		}
	}

	Ok(())
}

fn set_windows_socket_options(handle: Handle) -> Result<(), Error> {
	let fionbio = 0x8004667eu32;
	let ioctl_res = unsafe { ioctlsocket(handle, fionbio as c_int, &mut 1) };

	if ioctl_res != 0 {
		return Err(err!(
			ErrKind::IO,
			format!("complete fion with error: {}", errno().to_string())
		));
	}
	let sockoptres = unsafe {
		setsockopt(
			handle,
			SOL_SOCKET,
			SO_SNDBUF,
			&WINSOCK_BUF_SIZE as *const _ as *const u8,
			std::mem::size_of_val(&WINSOCK_BUF_SIZE) as c_int,
		)
	};

	if sockoptres != 0 {
		return Err(err!(
			ErrKind::IO,
			format!("setsocketopt resulted in error: {}", errno().to_string())
		));
	}

	Ok(())
}
