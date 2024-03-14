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

use crate::types::{Event, EventHandlerContext, EventType, EventTypeIn, Handle};
use crate::EventHandlerConfig;
use bmw_deps::bitvec::vec::*;
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::windows_sys::Win32::Networking::WinSock::{
	accept, closesocket, ioctlsocket, recv, send, setsockopt, SOCKADDR,
};
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::mem::{size_of, zeroed};
use std::time::{SystemTime, UNIX_EPOCH};

use bmw_deps::wepoll_sys::{
	epoll_ctl, epoll_data_t, epoll_event, epoll_wait, EPOLLIN, EPOLLONESHOT, EPOLLOUT, EPOLLRDHUP,
	EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD,
};
use std::net::{TcpListener, TcpStream};
use std::os::raw::{c_int, c_void};
use std::os::windows::io::{AsRawSocket, IntoRawSocket};
use std::sync::Arc;

const SOL_SOCKET: c_int = 0xFFFF;
const SO_SNDBUF: c_int = 0x1001;
const WINSOCK_BUF_SIZE: c_int = 100_000_000;
pub(crate) const MAX_EVENTS: usize = 100;

info!();

pub(crate) fn socket_pipe(fds: *mut i32) -> Result<(TcpStream, TcpStream), Error> {
	let (port, listener) = loop {
		let port = bmw_deps::portpicker::pick_unused_port().unwrap_or(9999);
		let listener = TcpListener::bind(format!("127.0.0.1:{}", port));
		match listener {
			Ok(listener) => {
				break (port, listener);
			}
			Err(_) => {} // error binding to a port, try again
		}
	};
	let stream = TcpStream::connect(format!("127.0.0.1:{}", port))?;
	debug!("port={}", port)?;
	let listener = listener.accept()?;
	let fds: &mut [i32] = unsafe { std::slice::from_raw_parts_mut(fds, 2) };
	fds[0] = listener.0.as_raw_socket().try_into().unwrap();
	fds[1] = stream.as_raw_socket().try_into().unwrap();
	set_windows_socket_options(fds[0].try_into()?)?;
	set_windows_socket_options(fds[1].try_into()?)?;

	Ok((listener.0, stream))
}

pub(crate) fn set_windows_socket_options(handle: Handle) -> Result<(), Error> {
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

pub(crate) fn read_bytes_impl(handle: Handle, buf: &mut [u8]) -> isize {
	set_errno(Errno(0));
	let cbuf: *mut u8 = buf as *mut _ as *mut u8;
	match buf.len().try_into() {
		Ok(len) => {
			let mut len = unsafe { recv(handle, cbuf, len, 0) };
			if errno().0 == 10035 {
				// would block
				len = -2;
			}
			len
		}
		Err(_e) => {
			let _ = error!("couldn't convert length");
			-1
		}
	}
	.try_into()
	.unwrap_or(-1)
}

pub(crate) fn write_bytes_impl(handle: Handle, buf: &[u8]) -> isize {
	set_errno(Errno(0));
	let cbuf: *mut u8 = buf as *const _ as *mut u8;
	match buf.len().try_into() {
		Ok(len) => unsafe { send(handle, cbuf, len, 0).try_into().unwrap_or(-1) },
		Err(_) => -1,
	}
}

pub(crate) fn get_reader_writer() -> Result<
	(
		Handle,
		Handle,
		Option<Arc<TcpStream>>,
		Option<Arc<TcpStream>>,
	),
	Error,
> {
	let (_tcp_stream, _tcp_listener);
	let mut rethandles = [0u64; 2];
	let handles: *mut c_int = &mut rethandles as *mut _ as *mut c_int;
	let (listener, stream) = socket_pipe(handles)?;
	let listener_socket = listener.as_raw_socket();
	let stream_socket = stream.as_raw_socket();
	_tcp_stream = Some(Arc::new(stream));
	_tcp_listener = Some(Arc::new(listener));
	Ok((
		listener_socket.try_into()?,
		stream_socket.try_into()?,
		_tcp_listener,
		_tcp_stream,
	))
}

pub(crate) fn close_impl(
	ctx: &mut EventHandlerContext,
	handle: Handle,
	partial: bool,
) -> Result<(), Error> {
	let handle_as_usize: usize = handle.try_into()?;
	if handle_as_usize >= ctx.filter_set.len().try_into()? {
		debug!(
			"filter set resize prev size = {}, new = {}",
			ctx.filter_set.len(),
			handle_as_usize + 100
		)?;
		ctx.filter_set
			.resize((handle_as_usize + 100).try_into()?, false);
	}

	ctx.filter_set.replace(handle_as_usize, false);
	debug!("closesocket={},tid={}", handle_as_usize, ctx.tid)?;
	if !partial {
		let data = epoll_data_t {
			fd: handle.try_into()?,
		};
		let mut event = epoll_event { events: 0, data };

		let res = unsafe {
			epoll_ctl(
				ctx.selector as *mut c_void,
				EPOLL_CTL_DEL as i32,
				handle_as_usize,
				&mut event,
			)
		};
		if res < 0 {
			let e = errno();
			warn!(
				"Error epoll_ctl del: {}, fd={}, op={:?}, tid={}",
				e, handle, EPOLL_CTL_DEL, ctx.tid
			)?
		}
	}

	unsafe {
		closesocket(handle);
	}

	Ok(())
}

pub(crate) fn close_handle_impl(handle: Handle) -> Result<(), Error> {
	debug!("close socket {}", handle)?;
	unsafe {
		closesocket(handle);
	}
	Ok(())
}

pub(crate) fn accept_impl(handle: Handle) -> Result<Handle, Error> {
	let handle = unsafe {
		accept(
			handle,
			&mut SOCKADDR { ..zeroed() },
			&mut (size_of::<SOCKADDR>() as u32).try_into()?,
		)
	};
	if handle != usize::MAX {
		set_windows_socket_options(handle)?;
	}
	Ok(handle)
}

pub(crate) fn epoll_ctl_impl(
	interest: u32,
	fd: Handle,
	filter_set: &mut BitVec,
	selector: *mut c_void,
	tid: usize,
) -> Result<(), Error> {
	let handle_as_usize: usize = fd.try_into()?;
	if handle_as_usize >= filter_set.len().try_into()? {
		debug!(
			"filter set resize prev size = {}, new = {}",
			filter_set.len(),
			handle_as_usize + 100
		)?;
		filter_set.resize((handle_as_usize + 100).try_into()?, false);
	}

	let op = match filter_set.get(handle_as_usize) {
		Some(bitref) => {
			if *bitref {
				EPOLL_CTL_MOD
			} else {
				EPOLL_CTL_ADD
			}
		}
		None => EPOLL_CTL_ADD,
	};
	debug!("filter_set true {}, tid={}", handle_as_usize, tid)?;
	filter_set.replace(handle_as_usize, true);
	let data = epoll_data_t { fd: fd.try_into()? };
	let mut event = epoll_event {
		events: interest,
		data,
	};

	debug!(
		"epoll_ctl {} read,op={},add={},mod={}",
		handle_as_usize, op, EPOLL_CTL_ADD, EPOLL_CTL_MOD
	)?;
	set_errno(Errno(0));
	let res = unsafe { epoll_ctl(selector as *mut c_void, op as i32, usize!(fd), &mut event) };
	if res < 0 {
		let e = errno();
		warn!(
			"Error epoll_ctl: {}, fd={}, op={:?}, tid={}",
			e, fd, op, tid
		)?
	}
	Ok(())
}

pub(crate) fn get_events_impl(
	config: &EventHandlerConfig,
	ctx: &mut EventHandlerContext,
	wakeup_requested: bool,
	_debug_err: bool,
) -> Result<usize, Error> {
	debug!(
		"in get_events_impl in_count={}, tid={}",
		ctx.events_in.len(),
		ctx.tid
	)?;
	for evt in &ctx.events_in {
		debug!("event={:?}", evt)?;
		if evt.etype == EventTypeIn::Read
			|| evt.etype == EventTypeIn::Accept
			|| evt.etype == EventTypeIn::Resume
		{
			let fd = evt.handle;
			epoll_ctl_impl(
				EPOLLIN | EPOLLONESHOT | EPOLLRDHUP,
				fd,
				&mut ctx.filter_set,
				ctx.selector as *mut c_void,
				ctx.tid,
			)?;
		} else if evt.etype == EventTypeIn::Write {
			let fd = evt.handle;
			epoll_ctl_impl(
				EPOLLIN | EPOLLOUT | EPOLLONESHOT | EPOLLRDHUP,
				fd,
				&mut ctx.filter_set,
				ctx.selector as *mut c_void,
				ctx.tid,
			)?;
		} else if evt.etype == EventTypeIn::Suspend {
			let fd = evt.handle;
			let data = epoll_data_t { fd: fd.try_into()? };
			let mut event = epoll_event { events: 0, data };

			set_errno(Errno(0));

			let res = unsafe {
				epoll_ctl(
					ctx.selector as *mut c_void,
					EPOLL_CTL_DEL as i32,
					usize!(fd),
					&mut event,
				)
			};
			if res < 0 {
				let e = errno();
				warn!(
					"Error epoll_ctl del: {}, fd={}, op={:?}, tid={}",
					e, fd, EPOLL_CTL_DEL, ctx.tid
				)?
			}
			let handle_as_usize: usize = fd.try_into()?;
			if handle_as_usize >= ctx.filter_set.len().try_into()? {
				debug!(
					"filter set resize prev size = {}, new = {}",
					ctx.filter_set.len(),
					handle_as_usize + 100
				)?;
				ctx.filter_set
					.resize((handle_as_usize + 100).try_into()?, false);
			}
			ctx.filter_set.replace(handle_as_usize, false);
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
	}
	.try_into()?;

	let mut epoll_events: [epoll_event; MAX_EVENTS as usize] = [epoll_event {
		events: 0,
		data: epoll_data_t { fd: 0 },
	}; MAX_EVENTS as usize];

	debug!("wakeup req = {}", wakeup_requested)?;
	set_errno(Errno(0));
	let results = unsafe {
		epoll_wait(
			ctx.selector as *mut c_void,
			epoll_events.as_mut_ptr(),
			MAX_EVENTS.try_into()?,
			sleep,
		)
	};

	ctx.now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

	let mut res_count = 0;
	if results < 0 {
		warn!("epoll wait generated error: {}", errno())?;
	} else {
		for i in 0..results as usize {
			if epoll_events[i].events & EPOLLOUT != 0 {
				ctx.events[res_count] = Event {
					handle: unsafe { epoll_events[i].data.fd } as Handle,
					etype: EventType::Write,
				};
				res_count += 1;
			}
			if epoll_events[i].events & EPOLLIN != 0 {
				ctx.events[res_count] = Event {
					handle: unsafe { epoll_events[i].data.fd } as Handle,
					etype: EventType::Read,
				};
				res_count += 1;
			}
		}
	}

	Ok(res_count)
}

pub(crate) fn create_listeners_impl(
	size: usize,
	addr: &str,
	_listen_size: usize,
	_reuse_port: bool,
) -> Result<Array<Handle>, Error> {
	let mut ret = array!(size, &0)?;
	let handle = TcpListener::bind(addr)?.into_raw_socket().try_into()?;
	let fionbio = 0x8004667eu32;
	let ioctl_res = unsafe { ioctlsocket(handle, fionbio as c_int, &mut 1) };

	if ioctl_res != 0 {
		return Err(err!(
			ErrKind::IO,
			format!("complete fion with error: {}", errno().to_string())
		));
	}

	for i in 0..size {
		ret[i] = handle;
	}

	Ok(ret)
}
