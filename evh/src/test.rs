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

#[cfg(test)]
mod test {
	use crate::evh::{create_listeners, make_config, read_bytes};
	use crate::evh::{load_private_key, READ_SLAB_NEXT_OFFSET, READ_SLAB_SIZE};
	use crate::types::{
		ConnectionInfo, Event, EventHandlerContext, EventHandlerImpl, EventType, ListenerInfo,
		StreamInfo, Wakeup, WriteState,
	};
	use crate::{
		ClientConnection, CloseHandle, ConnData, EventHandler, EventHandlerConfig,
		EventHandlerData, ServerConnection, ThreadContext, TlsClientConfig, TlsServerConfig,
		READ_SLAB_DATA_SIZE,
	};
	use bmw_deps::rand::random;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_ser::{deserialize, serialize};
	use bmw_test::*;
	use bmw_util::*;
	use std::io::Read;
	use std::io::Write;
	use std::net::TcpStream;
	#[cfg(unix)]
	use std::os::unix::io::IntoRawFd;
	#[cfg(windows)]
	use std::os::windows::io::IntoRawSocket;
	use std::sync::mpsc::sync_channel;
	use std::sync::Arc;
	use std::thread::{sleep, spawn};
	use std::time::Duration;

	#[cfg(unix)]
	use std::os::unix::io::{AsRawFd, FromRawFd};
	#[cfg(windows)]
	use std::os::windows::io::{AsRawSocket, FromRawSocket};

	#[cfg(target_os = "linux")]
	use crate::linux::*;
	#[cfg(target_os = "macos")]
	use crate::mac::*;
	#[cfg(windows)]
	use crate::win::*;

	info!();

	#[test]
	fn test_wakeup() -> Result<(), Error> {
		let check = lock!(0)?;
		let mut check_clone = check.clone();
		let mut wakeup = Wakeup::new()?;
		let wakeup_clone = wakeup.clone();

		let (tx, rx) = sync_channel(1);

		std::thread::spawn(move || -> Result<(), Error> {
			let mut wakeup = wakeup_clone;
			{
				let wakeup_clone = wakeup.clone();

				let mut count = 0;
				loop {
					let len;

					{
						let (_requested, _lock) = wakeup.pre_block()?;

						if count == 0 {
							tx.send(())?;
						}
						count += 1;
						let mut buffer = [0u8; 1];
						info!("reader = {}", wakeup_clone.reader)?;
						info!("writer = {}", wakeup_clone.writer)?;

						len = read_bytes(wakeup_clone.reader, &mut buffer);
						if len == 1 {
							break;
						}
					}
					sleep(Duration::from_millis(1));
				}
			}

			wakeup.post_block()?;

			let mut check = check_clone.wlock()?;
			**check.guard() = 1;

			Ok(())
		});

		rx.recv()?;
		wakeup.wakeup()?;

		let mut count = 0;
		loop {
			count += 1;
			if count > 10_000 {
				break;
			}
			sleep(Duration::from_millis(1));
			let check = check.rlock()?;
			if **(check).guard() == 1 {
				break;
			}
		}

		assert_eq!(rlock!(check), 1);
		Ok(())
	}

	#[test]
	fn test_evh_tls_basic_read_error() -> Result<(), Error> {
		{
			let port = free_port!()?;
			info!("eventhandler tls_basic read error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |conn_data, _thread_context| {
				info!("on close: {}", conn_data.get_handle())?;
				Ok(())
			})?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.set_debug_tls_read(true);
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);

			// connection will close because of the error
			assert_eq!(connection.read(&mut buf)?, 0);

			let port = free_port!()?;
			let addr2 = &format!("127.0.0.1:{}", port)[..];
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evhserver = EventHandlerImpl::new(config)?;
			evhserver.set_on_read(move |conn_data, _thread_context, _attachment| {
				debug!("on read slab_offset = {}", conn_data.slab_offset())?;
				let first_slab = conn_data.first_slab();
				let last_slab = conn_data.last_slab();
				let slab_offset = conn_data.slab_offset();
				debug!("first_slab={}", first_slab)?;
				let res = conn_data.borrow_slab_allocator(move |sa| {
					let slab = sa.get(first_slab.try_into()?)?;
					assert_eq!(first_slab, last_slab);
					info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				})?;
				conn_data.clear_through(first_slab)?;
				conn_data.write_handle().write(&res)?;
				info!("res={:?}", res)?;
				Ok(())
			})?;
			evhserver.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evhserver.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evhserver.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evhserver.set_housekeeper(move |_thread_context| Ok(()))?;
			evhserver.start()?;
			let handles = create_listeners(threads, addr2, 10, false)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evhserver.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let connection = TcpStream::connect(addr2)?;
			connection.set_nonblocking(true)?;
			#[cfg(unix)]
			let connection_handle = connection.into_raw_fd();
			#[cfg(windows)]
			let connection_handle = connection.into_raw_socket().try_into()?;
			let client = ClientConnection {
				handle: connection_handle,
				tls_config: Some(TlsClientConfig {
					sni_host: "localhost".to_string(),
					trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
				}),
			};
			let mut wh = evh.add_client(client, Box::new(""))?;
			assert!(wh.id() > 0);
			assert!(evh.event_handler_data().is_ok());
			wh.write(b"test")?;
			sleep(Duration::from_millis(2000));

			evh.stop()?;
			evhserver.stop()?;
		}

		Ok(())
	}

	#[test]
	fn test_evh_tls_basic_server_error() -> Result<(), Error> {
		{
			let port = free_port!()?;
			info!("eventhandler tls_basic server error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = EventHandlerImpl::new(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
			evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
			evh.set_housekeeper(move |_thread_context| Ok(()))?;
			evh.set_debug_tls_server_error(true);
			evh.start()?;

			let handles = create_listeners(threads, addr, 10, false)?;
			info!("handles.size={},handles={:?}", handles.size(), handles)?;
			let sc = ServerConnection {
				tls_config: Some(TlsServerConfig {
					certificates_file: "./resources/cert.pem".to_string(),
					private_key_file: "./resources/key.pem".to_string(),
				}),
				handles,
				is_reuse_port: false,
			};
			evh.add_server(sc, Box::new(""))?;
			sleep(Duration::from_millis(5_000));

			let mut connection = TcpStream::connect(addr)?;
			let mut buf = vec![];
			buf.resize(100, 0u8);

			// connection will close because of the error
			assert_eq!(connection.read(&mut buf)?, 0);

			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_close1() -> Result<(), Error> {
		let port = free_port!()?;
		info!("close1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000_000,
			read_slab_count: 30,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!(
				"accept a connection handle = {},tid={}",
				conn_data.get_handle(),
				conn_data.tid()
			)?;
			Ok(())
		})?;
		evh.set_on_close(move |conn_data, _thread_context| {
			info!(
				"on close: {}/{}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			let mut close_count = close_count.wlock()?;
			(**close_count.guard()) += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;

		let mut handle = lock_box!(0)?;
		let handle_clone = handle.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			std::thread::sleep(Duration::from_millis(600_000));
			let handle = rlock!(handle_clone);
			info!("due to timeout closing handle = {}", handle)?;
			close_handle_impl(handle)?;
			Ok(())
		});

		let total = 500;
		for i in 0..total {
			info!("loop {}", i)?;
			let mut connection = TcpStream::connect(addr)?;

			#[cfg(unix)]
			let rhandle = connection.as_raw_fd();
			#[cfg(windows)]
			let rhandle = connection.as_raw_socket();

			{
				wlock!(handle) = try_into!(rhandle)?;
			}

			info!("loop {} connected", i)?;
			connection.write(b"test1")?;
			info!("loop {} write complete", i)?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("loop {} about to read", i)?;
			let len = connection.read(&mut buf)?;
			info!("loop {} about read complete", i)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			info!("loop {} about to read2", i)?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
			info!("loop {} complete", i)?;
		}

		info!("complete")?;

		let mut count_count = 0;
		loop {
			count_count += 1;
			sleep(Duration::from_millis(1));
			let count = **((close_count_clone.rlock()?).guard());
			if count != total && count_count < 60_000 {
				info!(
					"count = {}, total = {}, will try again in a millisecond",
					count, total
				)?;
				continue;
			}
			assert_eq!((**((close_count_clone.rlock()?).guard())), total);
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_partial_clear() -> Result<(), Error> {
		let port = free_port!()?;
		info!("partial clear Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 40,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let mut second_slab = usize::MAX;
			let slab_offset = conn_data.slab_offset();
			let (res, second_slab) = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				info!("on_read ")?;
				loop {
					info!("loop with id={}", slab_id)?;
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					debug!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					if slab_id != last_slab {
						ret.extend(&slab_bytes[0..READ_SLAB_DATA_SIZE as usize]);
					} else {
						ret.extend(&slab_bytes[0..slab_offset as usize]);
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
					if second_slab == usize::MAX {
						info!("set secondslab to {} ", slab_id)?;
						second_slab = slab_id.try_into()?;
					}
					info!("end loop")?;
				}
				Ok((ret, second_slab))
			})?;
			info!("second_slab={}", second_slab)?;
			assert_ne!(second_slab, usize::MAX);
			conn_data.clear_through(second_slab.try_into()?)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};

		sleep(Duration::from_millis(1000));
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		let mut message = ['a' as u8; 1036];
		for i in 0..1036 {
			message[i] = 'a' as u8 + (i % 26) as u8;
		}
		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(2000, 0u8);

		let mut len = 0;

		loop {
			let partial_len = connection.read(&mut buf[len..])?;
			len += partial_len;
			if len >= 1036 {
				break;
			}
		}
		assert_eq!(len, 1036);
		for i in 0..len {
			assert_eq!(buf[i], 'a' as u8 + (i % 26) as u8);
		}

		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(5000, 0u8);

		let mut len = 0;

		loop {
			let partial_len = connection.read(&mut buf[len..])?;
			len += partial_len;
			if len >= 1044 {
				break;
			}
		}

		// there are some remaining bytes left in the last of the three slabs.
		// only 8 bytes so we have 8 + 1036 = 1044.
		assert_eq!(len, 1044);

		assert_eq!(buf[0], 111);
		assert_eq!(buf[1], 112);
		assert_eq!(buf[2], 113);
		assert_eq!(buf[3], 114);
		assert_eq!(buf[4], 115);
		assert_eq!(buf[5], 116);
		assert_eq!(buf[6], 117);
		assert_eq!(buf[7], 118);
		for i in 8..1044 {
			assert_eq!(buf[i], 'a' as u8 + ((i - 8) % 26) as u8);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_different_lengths1() -> Result<(), Error> {
		let port = free_port!()?;
		info!("different len1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 41,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					let offset = if slab_id == last_slab {
						slab_offset as usize
					} else {
						READ_SLAB_DATA_SIZE
					};
					debug!("read bytes = {:?}", &slab.get()[0..offset as usize])?;
					ret.extend(&slab_bytes[0..offset]);

					if slab_id == last_slab {
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
				}
				Ok(ret)
			})?;
			conn_data.clear_through(last_slab)?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;

		sleep(Duration::from_millis(5_000));
		let mut stream = TcpStream::connect(addr)?;

		let mut bytes = [0u8; 10240];
		for i in 0..10240 {
			bytes[i] = 'a' as u8 + i as u8 % 26;
		}

		for i in 1..2000 {
			info!("i={}", i)?;
			stream.write(&bytes[0..i])?;
			let mut buf = vec![];
			buf.resize(i + 2_000, 0u8);

			let mut len = 0;

			loop {
				let partial_len = stream.read(&mut buf[len..])?;
				len += partial_len;
				if len >= i {
					break;
				}
			}
			assert_eq!(len, i);
			assert_eq!(&buf[0..len], &bytes[0..len]);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_different_lengths_client() -> Result<(), Error> {
		let port = free_port!()?;
		info!("different lengths client Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 41,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					debug!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					if slab_id != last_slab {
						ret.extend(&slab_bytes[0..READ_SLAB_DATA_SIZE as usize]);
					} else {
						ret.extend(&slab_bytes[0..slab_offset as usize]);
						break;
					}
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
				}
				Ok(ret)
			})?;
			conn_data.clear_through(last_slab)?;
			conn_data.write_handle().write(&res)?;
			debug!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		debug!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 21,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh2 = crate::Builder::build_evh(config.clone())?;

		let expected = lock_box!("".to_string())?;
		let mut expected_clone = expected.clone();
		let (tx, rx) = sync_channel(1);

		evh2.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let last_slab = conn_data.last_slab();
			let value = conn_data.borrow_slab_allocator(move |sa| {
				let mut slab_id = first_slab;
				let mut full = vec![];
				loop {
					let slab = sa.get(slab_id.try_into()?)?;
					let slab_bytes = slab.get();
					let offset = if slab_id == last_slab {
						slab_offset as usize
					} else {
						READ_SLAB_DATA_SIZE
					};

					full.extend(&slab_bytes[0..offset]);
					if slab_id == last_slab {
						break;
					} else {
						slab_id = u32::from_be_bytes(try_into!(
							slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
						)?);
					}
				}
				Ok(full)
			})?;

			let expected = expected.rlock()?;
			let guard = expected.guard();

			if value.len() == (**guard).len() {
				assert_eq!(std::str::from_utf8(&value)?, (**guard));
				tx.send(())?;
				conn_data.clear_through(last_slab)?;
			}
			Ok(())
		})?;

		evh2.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh2.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh2.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh2.set_housekeeper(move |_thread_context| Ok(()))?;
		evh2.start()?;

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: None,
		};
		let mut wh = evh2.add_client(client, Box::new(""))?;

		let mut bytes = [0u8; 2000];
		for i in 0..2000 {
			bytes[i] = 'a' as u8 + i as u8 % 26;
		}

		for i in 1..1024 {
			let rand: usize = random();
			let rand = rand % 26;
			info!("i={},rand[0]={}", i, bytes[rand])?;
			let s = std::str::from_utf8(&bytes[rand..i + rand])?.to_string();
			{
				let mut expected = expected_clone.wlock()?;
				let guard = expected.guard();
				**guard = s;
				for j in 0..i {
					let mut b = [0u8; 1];
					b[0] = bytes[j + rand];
					wh.write(&b[0..1])?;
				}
			}

			rx.recv()?;
		}

		evh.stop()?;
		evh2.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_out_of_slabs() -> Result<(), Error> {
		let port = free_port!()?;
		info!("out of slabs Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 1,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = slab_offset as usize;
				ret.extend(&slab_bytes[0..offset]);
				Ok(ret)
			})?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		sleep(Duration::from_millis(100));
		// do a request that uses 2 slabs (with capacity of only one)
		let mut buf = [10u8; 600];
		stream.write(&buf)?;
		assert!(stream.read(&mut buf).is_err());

		sleep(Duration::from_millis(100));
		// now we should be able to continue with small requests

		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"posterror")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 9);
		assert_eq!(&buf[0..len], b"posterror");

		// now make a new connection and run out of slabs
		let mut stream2 = TcpStream::connect(addr)?;
		stream2.write(b"posterror")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		assert!(stream2.read(&mut buf).is_err());

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_user_data() -> Result<(), Error> {
		let port = free_port!()?;
		info!("user data Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, thread_context, _attachment| {
			assert_eq!(
				thread_context.user_data.downcast_ref::<String>().unwrap(),
				&"something".to_string()
			);
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab_id = first_slab;
				let mut ret: Vec<u8> = vec![];
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = slab_offset as usize;
				ret.extend(&slab_bytes[0..offset]);
				Ok(ret)
			})?;
			debug!("res.len={}", res.len())?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, thread_context| {
			thread_context.user_data = Box::new("something".to_string());
			Ok(())
		})?;

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read_error() -> Result<(), Error> {
		let port = free_port!()?;
		info!("trigger on_read error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			conn_data.write_handle().write(b"1234")?;
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				info!("new thread")?;
				sleep(Duration::from_millis(1000));
				wh.write(b"5678")?;
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Err(err!(ErrKind::Test, "on_read test err"))
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"5678");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read() -> Result<(), Error> {
		let port = free_port!()?;
		info!("trigger on_read Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			conn_data.write_handle().write(b"1234")?;
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				info!("new thread")?;
				sleep(Duration::from_millis(1000));
				wh.write(b"5679")?;
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"5679");

		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"1234");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_oob_panic() -> Result<(), Error> {
		let port = free_port!()?;
		info!("oob_panic Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;

		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		info!("adding server")?;
		evh.add_server(sc, Box::new(""))?;
		info!("server added - sleep 10 secs")?;
		sleep(Duration::from_millis(10_000));
		info!("5 second sleep complete connect now")?;

		let _connection = TcpStream::connect(addr)?;
		info!("sleeping 10 seconds")?;
		sleep(Duration::from_millis(10_000));
		info!("10 seconds sleep complete")?;
		// connection should be ok because the listener is still open we just close the
		// accepted handle
		assert!(TcpStream::connect(addr).is_ok());

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_thread_panic1() -> Result<(), Error> {
		let port = free_port!()?;
		info!("thread_panic1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config.clone())?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				panic!("test panic");
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		let handles = create_listeners(threads, addr, 10, false)?;
		let server_handle = handles[0];

		evh.set_on_accept(move |conn_data, _thread_context| {
			assert_eq!(server_handle, conn_data.get_accept_handle().unwrap());
			Ok(())
		})?;
		evh.set_on_close(move |_, _| Ok(()))?;

		let mut on_panic_callback = lock_box!(0)?;
		let on_panic_callback_clone = on_panic_callback.clone();
		evh.set_on_panic(move |_thread_context, e| {
			let e = e.downcast_ref::<&str>().unwrap();
			info!("on panic callback: '{}'", e)?;
			let mut on_panic_callback = on_panic_callback.wlock()?;
			**(on_panic_callback.guard()) += 1;
			if **(on_panic_callback.guard()) > 1 {
				return Err(err!(ErrKind::Test, "test on_panic err"));
			}
			Ok(())
		})?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// create a thread panic
		stream.write(b"aaa")?;
		// read and we should get 0 for close
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 0);
		// connect and send another request
		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test");

		// assert that the on_panic callback was called
		assert_eq!(**on_panic_callback_clone.rlock()?.guard(), 1);
		// create a thread panic
		stream.write(b"aaa")?;
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_no_panic_handler() -> Result<(), Error> {
		let port = free_port!()?;
		info!("no panic_handler Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config.clone())?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				panic!("test panic");
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		let handles = create_listeners(threads, addr, 10, false)?;
		let server_handle = handles[0];

		evh.set_on_accept(move |conn_data, _thread_context| {
			assert_eq!(server_handle, conn_data.get_accept_handle().unwrap());
			Ok(())
		})?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_panic_none();
		evh.start()?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// create a thread panic
		stream.write(b"aaa")?;
		sleep(Duration::from_millis(5000));

		// read and we should get 0 for close
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 0);

		// connect and send another request
		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_thread_panic_multi() -> Result<(), Error> {
		let port = free_port!()?;
		info!("thread_panic_multi Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 10,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let x = lock_box!(0)?;
		let mut x_clone = x.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res[0] == 'a' as u8 {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			} else if res[0] == 'b' as u8 {
				loop {
					sleep(Duration::from_millis(10));
					if **(x.rlock()?.guard()) != 0 {
						break;
					}
				}
				wh.write(&res)?;
			} else {
				wh.write(&res)?;
			}
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, e| {
			let e = e.downcast_ref::<&str>().unwrap();
			info!("on panic callback: '{}'", e)?;
			Ok(())
		})?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		// make 4 connections
		let mut stream1 = TcpStream::connect(addr)?;
		let mut stream2 = TcpStream::connect(addr)?;
		let mut stream3 = TcpStream::connect(addr)?;
		let mut stream4 = TcpStream::connect(addr)?;

		// do a normal request on 1
		stream1.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// do a normal request on 2
		stream2.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream2.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// do a normal request on 3
		stream3.write(b"test")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream3.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");

		// pause request
		stream4.write(b"bbbb")?;
		sleep(Duration::from_millis(100));

		// normal request
		stream1.write(b"1")?;
		sleep(Duration::from_millis(100));

		// create panic
		stream2.write(b"a")?;
		sleep(Duration::from_millis(100));

		// normal request
		stream3.write(b"c")?;
		sleep(Duration::from_millis(100));

		// unblock with guard
		**(x_clone.wlock()?.guard()) = 1;

		// read responses

		// normal echo after lock lifted
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream4.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"bbbb");

		//normal echo
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(&buf[0..len], b"1");

		// panic so close len == 0
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream2.read(&mut buf)?;
		assert_eq!(len, 0);

		// normal echo
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream3.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(&buf[0..len], b"c");

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_too_many_connections() -> Result<(), Error> {
		let port = free_port!()?;
		info!("too many connections on_read Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 30,
			max_handles_per_thread: 1,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			let mut wh = conn_data.write_handle();

			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();

			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				let slab_bytes = slab.get();
				let mut ret = vec![];
				ret.extend(&slab_bytes[0..slab_offset as usize]);
				Ok(ret)
			})?;

			wh.write(&res)?;
			conn_data.clear_through(first_slab)?;

			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();
		evh.set_on_close(move |_conn_data, _thread_context| {
			let mut close_count = close_count.wlock()?;
			**close_count.guard() += 1;
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut stream1 = TcpStream::connect(addr)?;

			// do a normal request on 1
			stream1.write(b"test")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = stream1.read(&mut buf)?;
			assert_eq!(len, 4);
			assert_eq!(&buf[0..len], b"test");

			// there are already two connections on the single thread (listener/stream).
			// so another connection will fail

			let mut stream2 = TcpStream::connect(addr)?;
			assert_eq!(stream2.read(&mut buf)?, 0);
		}

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(1));
			if **(close_count_clone.rlock()?.guard()) == 0 && count < 2_000 {
				continue;
			}
			assert_eq!(**(close_count_clone.rlock()?.guard()), 1);
			break;
		}
		info!("sleep complete")?;
		// now that stream1 is dropped we should be able to reconnect

		let mut stream1 = TcpStream::connect(addr)?;

		// do a normal request on 1
		stream1.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream1.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		Ok(())
	}

	#[test]
	fn test_evh_write_error() -> Result<(), Error> {
		let port = free_port!()?;
		info!("write_error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_write_error(true);

		let mut count = lock_box!(0)?;
		let count_clone = count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			if res.len() > 0 && res[0] == '1' as u8 {
				conn_data.write_handle().write(&res)?;
			}
			let mut count = count.wlock()?;
			let g = count.guard();
			**g += 1;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		stream.write(b"12345")?;
		sleep(Duration::from_millis(1_000));
		stream.write(b"0000")?;

		sleep(Duration::from_millis(10_000));
		assert_eq!(**(count_clone.rlock()?.guard()), 1);

		Ok(())
	}

	#[test]
	fn test_evh_debug_suspend() -> Result<(), Error> {
		let port = free_port!()?;
		info!("debug_suspend Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_suspended(true);

		let mut success = lock_box!(false)?;
		let success_clone = success.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			assert!(conn_data.write_handle().write(&res).is_err());
			info!("res={:?}", res)?;
			**(success.wlock()?.guard()) = true;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;
		stream.write(b"12345")?;

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			count += 1;
			if !**(success_clone.rlock()?.guard()) && count < 25_000 {
				continue;
			}
			assert!(**(success_clone.rlock()?.guard()));

			break;
		}

		Ok(())
	}

	#[test]
	fn test_evh_debug_pending() -> Result<(), Error> {
		let port = free_port!()?;
		info!("pending Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 60_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_pending(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;

		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		Ok(())
	}

	#[test]
	fn test_evh_write_queue() -> Result<(), Error> {
		let port = free_port!()?;
		info!("test_evh_write_queue Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_write_queue(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut stream = TcpStream::connect(addr)?;

		// do a normal request
		stream.write(b"12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..len], b"12345");

		// this request uses the write queue
		stream.write(b"a12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(1000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"a12345");

		// this request uses the write queue
		stream.write(b"b12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(5_000));
		let len = stream.read(&mut buf)?;
		info!("read = {:?}", &buf[0..len])?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"b12345");

		// this request uses the write queue
		info!("write second time to write queue")?;
		stream.write(b"c12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(3_000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"c12345");

		// this request uses the write queue
		stream.write(b"d12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		sleep(Duration::from_millis(3_000));
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"d12345");

		// this request uses the write queue
		stream.write(b"e12345")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		let len = stream.read(&mut buf)?;
		assert_eq!(len, 6);
		assert_eq!(&buf[0..len], b"e12345");
		Ok(())
	}

	#[test]
	fn test_evh_housekeeper() -> Result<(), Error> {
		let port = free_port!()?;
		info!("housekeeper Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;

		let mut x = lock_box!(0)?;
		let x_clone = x.clone();
		evh.set_housekeeper(move |thread_context: _| {
			info!("housekeep callback")?;
			match thread_context.user_data.downcast_mut::<u64>() {
				Some(value) => {
					*value += 1;
					let mut x = x.wlock()?;
					(**x.guard()) = *value;
					info!("value={}", *value)?;
				}
				None => {
					thread_context.user_data = Box::new(0u64);
				}
			}
			Ok(())
		})?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let v = **(x_clone.rlock()?.guard());
			info!("v={}", v)?;
		}

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(100));
			{
				let v = **(x_clone.rlock()?.guard());
				info!("v={}", v)?;
				if v < 10 && count < 10_000 {
					continue;
				}
			}

			assert!((**(x_clone.rlock()?.guard())) >= 10);
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_suspend_resume() -> Result<(), Error> {
		let port = free_port!()?;
		info!("suspend/resume Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		let complete = lock_box!(0)?;
		let complete_clone = complete.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;

			let mut wh = conn_data.write_handle();
			let mut complete_clone = complete_clone.clone();

			if res[0] == 't' as u8 {
				spawn(move || -> Result<(), Error> {
					wh.write(b"test")?;
					let handle = wh.handle();
					wh.suspend()?;
					sleep(Duration::from_millis(1_000));

					#[cfg(unix)]
					let mut strm = unsafe { TcpStream::from_raw_fd(handle) };
					#[cfg(windows)]
					let mut strm = unsafe { TcpStream::from_raw_socket(u64!(handle)) };

					let mut count = 0;
					loop {
						sleep(Duration::from_millis(1_000));
						strm.write(b"ok")?;
						if count > 3 {
							break;
						}
						count += 1;
					}

					let mut buf = vec![];
					buf.resize(100, 0u8);
					let len = strm.read(&mut buf)?;
					info!("read = {}", std::str::from_utf8(&buf[0..len]).unwrap())?;
					assert_eq!(std::str::from_utf8(&buf[0..len]).unwrap(), "next");

					wh.resume()?;
					info!("resume complete")?;

					#[cfg(unix)]
					strm.into_raw_fd();
					#[cfg(windows)]
					strm.into_raw_socket();

					let mut complete = complete_clone.wlock()?;
					**complete.guard() = 1;

					Ok(())
				});
			} else {
				wh.write(&res)?;
			}

			let response = std::str::from_utf8(&res)?;
			info!("res={:?}", response)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;
		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;

			sleep(Duration::from_millis(10_000));
			connection.write(b"next")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;

			let response = std::str::from_utf8(&buf[0..len])?;
			info!("buf={:?}", response)?;
			assert_eq!(response, "testokokokokok");

			sleep(Duration::from_millis(1_000));
			connection.write(b"resume")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			let response = std::str::from_utf8(&buf[0..len])?;
			info!("final={:?}", response)?;
			assert_eq!(response, "resume");

			let mut count = 0;
			loop {
				count += 1;
				sleep(Duration::from_millis(1));
				if **(complete.rlock()?.guard()) != 1 && count < 2_000 {
					continue;
				}

				assert_eq!(**(complete.rlock()?.guard()), 1);
				break;
			}
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_arc_to_ptr() -> Result<(), Error> {
		let x = Arc::new(10u32);

		let ptr = Arc::into_raw(x);
		let ptr_as_usize = ptr as usize;
		info!("ptr_as_usize = {}", ptr_as_usize)?;
		let ptr_ret = unsafe { Arc::from_raw(ptr_as_usize as *mut u32) };
		info!("ptr_val={:?}", ptr_ret)?;

		Ok(())
	}

	#[test]
	fn test_debug_listener_info() -> Result<(), Error> {
		let li = ListenerInfo {
			id: 0,
			handle: 0,
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		};
		info!("li={:?}", li)?;
		assert_eq!(li.id, 0);
		Ok(())
	}

	fn compare_ci(ci1: ConnectionInfo, ci2: ConnectionInfo) -> Result<(), Error> {
		match ci1 {
			ConnectionInfo::ListenerInfo(li1) => match ci2 {
				ConnectionInfo::ListenerInfo(li2) => {
					assert_eq!(li1.id, li2.id);
					assert_eq!(li1.handle, li2.handle);
					assert_eq!(li1.is_reuse_port, li2.is_reuse_port);
				}
				ConnectionInfo::StreamInfo(_rwi) => return Err(err!(ErrKind::IllegalArgument, "")),
			},
			ConnectionInfo::StreamInfo(rwi1) => match ci2 {
				ConnectionInfo::ListenerInfo(_li) => {
					return Err(err!(ErrKind::IllegalArgument, ""));
				}
				ConnectionInfo::StreamInfo(rwi2) => {
					assert_eq!(rwi1.id, rwi2.id);
					assert_eq!(rwi1.handle, rwi2.handle);
					assert_eq!(rwi1.accept_handle, rwi2.accept_handle);
					assert_eq!(rwi1.first_slab, rwi2.first_slab);
					assert_eq!(rwi1.last_slab, rwi2.last_slab);
					assert_eq!(rwi1.slab_offset, rwi2.slab_offset);
					assert_eq!(rwi1.is_accepted, rwi2.is_accepted);
				}
			},
		}
		Ok(())
	}

	#[test]
	fn test_connection_info_serialization() -> Result<(), Error> {
		let mut hashtable = hashtable!()?;
		let ci1 = ConnectionInfo::ListenerInfo(ListenerInfo {
			id: 7,
			handle: 8,
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		});
		hashtable.insert(&0, &ci1)?;
		let v = hashtable.get(&0)?.unwrap();
		compare_ci(v, ci1.clone())?;

		let ser_out = ConnectionInfo::ListenerInfo(ListenerInfo {
			id: 10,
			handle: 80,
			is_reuse_port: true,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		});
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		v[0] = 2; // corrupt data
		let ser_in: Result<ConnectionInfo, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let ci2 = ConnectionInfo::StreamInfo(StreamInfo {
			accept_handle: None,
			accept_id: None,
			id: 0,
			handle: 0,
			first_slab: 0,
			last_slab: 0,
			slab_offset: 0,
			is_accepted: true,
			tls_client: None,
			tls_server: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0
			})?,
			tx: None,
		});
		hashtable.insert(&0, &ci2)?;
		let v = hashtable.get(&0)?.unwrap();
		compare_ci(v, ci2.clone())?;

		assert!(compare_ci(ci1.clone(), ci2.clone()).is_err());
		assert!(compare_ci(ci2, ci1).is_err());

		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk_reuse_port_acc_err() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler tls_multi_chunk no reuse port acc err Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Err(err!(ErrKind::Test, "acc err")))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_big_message() -> Result<(), Error> {
		let port = free_port!()?;
		info!("eventhandler tls_big_message Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut data_accumulator: Box<dyn LockBox<Vec<u8>>> = lock_box!(vec![])?;
		let data_accumulator_clone = data_accumulator.clone();

		let mut big_msg = vec![];
		// TODO: breaks with 10_000 * 1024 on windows
		big_msg.resize(1_000 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();

		let (tx, rx) = sync_channel(1);

		let mut client_data_accumulator: Box<dyn LockBox<Vec<u8>>> = lock_box!(vec![])?;
		let client_data_accumulator_clone = client_data_accumulator.clone();

		let (client_tx, client_rx) = sync_channel(1);

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			let attachment = attachment.unwrap();
			let val = attachment.attachment.rlock()?;
			let guard = val.guard();
			let v = (**guard).downcast_ref::<bool>().unwrap();
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;

			if *v {
				// client
				wlock!(client_data_accumulator).extend(&res);
				let len = rlock!(client_data_accumulator).len();
				info!("client_data_accumulated.len={}", len)?;
				if rlock!(client_data_accumulator) == big_msg {
					info!("found the client msg")?;
					client_tx.send(())?;
				}
				conn_data.clear_through(last_slab)?;
			} else {
				// server

				wlock!(data_accumulator).extend(&res);
				let len = rlock!(data_accumulator).len();
				info!("data_accumulated.len={}", len)?;
				conn_data.clear_through(last_slab)?;

				if rlock!(data_accumulator) == big_msg {
					info!("found the message")?;
					conn_data.write_handle().write(&big_msg)?;
					tx.send(())?;
				}
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_accept_none();

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(false))?;

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(true))?;

		info!("about to write message")?;
		wh.write(&big_msg_clone)?;
		info!("message written")?;
		rx.recv()?;
		assert_eq!(rlock!(data_accumulator_clone), big_msg_clone);
		client_rx.recv()?;
		assert_eq!(rlock!(client_data_accumulator_clone), big_msg_clone);

		evh.stop()?;
		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk_no_reuse_port() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler tls_multi_chunk no reuse port Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_accept_none();

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_multi_chunk() -> Result<(), Error> {
		let port = free_port!()?;
		info!("eventhandler tls_multi_chunk Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 100,
			max_handles_per_thread: 10,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(0)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		let mut big_msg = vec![];
		big_msg.resize(10 * 1024, 7u8);
		big_msg[0] = 't' as u8;
		let big_msg_clone = big_msg.clone();
		let mut server_accumulator = lock_box!(vec![])?;
		let mut client_accumulator = lock_box!(vec![])?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read slab offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let mut ret: Vec<u8> = vec![];
				let mut slab_id = first_slab;
				loop {
					if slab_id == last_slab {
						let slab = sa.get(slab_id.try_into()?)?;
						ret.extend(&slab.get()[0..slab_offset as usize]);
						break;
					} else {
						let slab = sa.get(slab_id.try_into()?)?;
						let slab_bytes = slab.get();
						ret.extend(&slab_bytes[0..READ_SLAB_NEXT_OFFSET]);
						slab_id = u32::from_be_bytes(try_into!(
							&slab_bytes[READ_SLAB_NEXT_OFFSET..READ_SLAB_SIZE]
						)?);
					}
				}
				Ok(ret)
			})?;
			info!(
				"on read handle={},id={}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
			conn_data.clear_through(last_slab)?;
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				info!("server res.len= = {}", res.len())?;
				if res == b"abc".to_vec() {
					info!("found abc")?;
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;

					// write a big message to test the server side big messages
					conn_data.write_handle().write(&big_msg)?;
				} else {
					conn_data.write_handle().write(&res)?;
					let mut server_accumulator = server_accumulator.wlock()?;
					let guard = server_accumulator.guard();
					(**guard).extend(res.clone());

					if **guard == big_msg {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
			} else {
				info!("client res.len = {}", res.len())?;

				let mut client_accumulator = client_accumulator.wlock()?;
				let guard = client_accumulator.guard();
				(**guard).extend(res.clone());

				if **guard == big_msg {
					info!("client found a big message")?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					**guard = vec![];
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) += 1;
				}
			}
			info!("res[0]={}, res.len()={}", res[0], res.len())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_accept_none();

		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let connection = TcpStream::connect(addr)?;
		connection.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection.into_raw_socket().try_into()?;
		{
			let mut client_handle = client_handle.wlock()?;
			(**client_handle.guard()) = connection_handle;
		}

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: Some(TlsClientConfig {
				sni_host: "localhost".to_string(),
				trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
			}),
		};

		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(&big_msg_clone)?;
		info!("big write complete")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard()) >= 2
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 20_000 {
					continue;
				}
			}

			let v = **(client_received_test1_clone.rlock()?.guard());
			info!("client recieved = {}", v)?;
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(client_received_test1_clone.rlock()?.guard()) >= 2);
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_bad_configs() -> Result<(), Error> {
		let mut evhs = vec![];
		evhs.push(EventHandlerImpl::new(EventHandlerConfig {
			read_slab_count: u32::MAX as usize,
			..Default::default()
		}));
		evhs.push(EventHandlerImpl::new(EventHandlerConfig {
			read_slab_count: 100 as usize,
			..Default::default()
		}));

		for i in 0..2 {
			if i == 0 {
				assert!(evhs[i].is_err());
			} else {
				let evh = evhs[i].as_mut().unwrap();
				evh.set_on_read(move |_, _, _| Ok(()))?;
				evh.set_on_accept(move |_, _| Ok(()))?;
				evh.set_on_close(move |_, _| Ok(()))?;
				evh.set_housekeeper(move |_| Ok(()))?;
				evh.set_on_panic(move |_, _| Ok(()))?;
			}
		}
		Ok(())
	}

	#[test]
	fn test_bad_keys() -> Result<(), Error> {
		// it's empty so it would be an error
		assert!(load_private_key("./resources/emptykey.pem").is_err());

		// key is ok to load but signing won't work
		assert!(load_private_key("./resources/badkey.pem").is_ok());

		// rsa
		assert!(load_private_key("./resources/rsa.pem").is_ok());

		// eckey
		assert!(load_private_key("./resources/ec256.pem").is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_panic_fatal() -> Result<(), Error> {
		let port = free_port!()?;
		info!("panic_fatal Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res.len() > 0 && res[0] == '1' as u8 {
				panic!("test start with '1'");
			}

			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"1panic")?;
		sleep(Duration::from_millis(1_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test2");

		connection.write(b"0test1")?;
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_panic_backlog() -> Result<(), Error> {
		let port = free_port!()?;
		info!("panic_backlog Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 20,
			max_handles_per_thread: 4,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;

			if res.len() > 0 && res[0] == '1' as u8 {
				panic!("test start with '1'");
			} else if res.len() > 0 && res[0] == '2' as u8 {
				sleep(Duration::from_millis(5_000));
			}

			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut c1 = TcpStream::connect(addr)?;
		let mut c2 = TcpStream::connect(addr)?;
		let mut c3 = TcpStream::connect(addr)?;

		c1.write(b"2222")?;
		sleep(Duration::from_millis(1_000));
		c2.write(b"1111")?;
		sleep(Duration::from_millis(1_000));
		c3.write(b"3333")?;
		let mut buf = vec![];
		buf.resize(10, 0u8);
		let len = c3.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"3333");
		let len = c1.read(&mut buf)?;
		info!("read {} bytes", len)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"2222");
		c1.write(b"5555")?;
		c2.write(b"6666")?;
		c3.write(b"77777")?;

		let len = c1.read(&mut buf)?;
		info!("read {} bytes", len)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"5555");

		// this one closed
		let len = match c2.read(&mut buf) {
			Ok(len) => len,
			Err(_) => 0,
		};
		info!("read {} bytes", len)?;
		assert_eq!(len, 0);

		let len = c3.read(&mut buf)?;
		info!("read {} bytes", len)?;
		assert_eq!(len, 5);
		assert_eq!(&buf[0..5], b"77777");

		Ok(())
	}

	#[test]
	fn test_evh_other_situations() -> Result<(), Error> {
		let port = free_port!()?;
		info!("other_situations Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;

			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test2");

		connection.write(b"0test1")?;
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_other_panics_wo_err() -> Result<(), Error> {
		let port = free_port!()?;
		info!("test_evh_other_panics_wo_err Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;
		let mut close_count = lock_box!(0)?;
		let mut housekeeper_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
			let count = {
				let mut close_count = close_count.wlock()?;
				let count = **close_count.guard();
				**close_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on close panic");
			}
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| {
			let count = {
				let mut housekeeper_count = housekeeper_count.wlock()?;
				let count = **housekeeper_count.guard();
				**housekeeper_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on housekeeper panic");
			}
			Ok(())
		})?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let _connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1_000));
		// listener should be closed so this will fail
		assert!(TcpStream::connect(addr).is_err());

		let port = free_port!()?;
		info!("other_situations2 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let _connection = TcpStream::connect(addr)?;
		}
		sleep(Duration::from_millis(5_000));

		// last connection on close handler panics, but we should be able to still send
		// requests.
		{
			let mut connection = TcpStream::connect(addr)?;
			info!("about to write")?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("about to read")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			info!("read back buf[{}] = {:?}", len, buf)?;
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}
		sleep(Duration::from_millis(1_000));

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_other_panics_w_err() -> Result<(), Error> {
		let port = free_port!()?;
		info!("test_evh_other_panics_w_err Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 20,
			max_handles_per_thread: 30,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_fatal_error(true);

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("on read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;

		let mut acc_count = lock_box!(0)?;
		let mut close_count = lock_box!(0)?;
		let mut housekeeper_count = lock_box!(0)?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			let count = {
				let mut acc_count = acc_count.wlock()?;
				let count = **acc_count.guard();
				**acc_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on acc panic");
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
			let count = {
				let mut close_count = close_count.wlock()?;
				let count = **close_count.guard();
				**close_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on close panic");
			}
			Ok(())
		})?;
		evh.set_on_panic(move |_thread_context, _e| Err(err!(ErrKind::Test, "")))?;
		evh.set_housekeeper(move |_thread_context| {
			let count = {
				let mut housekeeper_count = housekeeper_count.wlock()?;
				let count = **housekeeper_count.guard();
				**housekeeper_count.guard() += 1;
				count
			};
			if count == 0 {
				panic!("on housekeeper panic");
			}
			Ok(())
		})?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let _connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1_000));
		// listener should be closed so this will fail
		assert!(TcpStream::connect(addr).is_err());

		let port = free_port!()?;
		info!("other_situations2 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: true,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		{
			let _connection = TcpStream::connect(addr)?;
		}
		sleep(Duration::from_millis(5_000));

		// last connection on close handler panics, but we should be able to still send
		// requests.
		{
			let mut connection = TcpStream::connect(addr)?;
			info!("about to write")?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			info!("about to read")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			info!("read back buf[{}] = {:?}", len, buf)?;
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}
		sleep(Duration::from_millis(1_000));

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_housekeeper_error() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Err(err!(ErrKind::Test, "")))?;

		evh.start()?;
		sleep(Duration::from_millis(10_000));
		assert!(evh.stop().is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_invalid_write_queue() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		// enqueue an invalid handle. the function should just print the warning and still
		// succeed
		{
			let mut data = evh.data[0].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).write_queue.enqueue(100)?;
		}
		evh.process_write_queue(&mut ctx)?;

		// insert the listener. again an error should be printed but processing continue
		let li = ListenerInfo {
			id: 1_000,
			handle: 0,
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		};
		let ci = ConnectionInfo::ListenerInfo(li.clone());
		ctx.connection_hashtable.insert(&1_000, &ci)?;
		{
			let mut data = evh.data[0].wlock_ignore_poison()?;
			let guard = data.guard();
			(**guard).write_queue.enqueue(1_000)?;
		}
		evh.process_write_queue(&mut ctx)?;

		Ok(())
	}

	#[test]
	fn test_evh_close_no_handler() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		// insert the rwi
		let mut rwi = StreamInfo {
			id: 1_000,
			handle: 0,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
			tx: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.connection_hashtable.insert(&1_000, &ci)?;

		// call on close to trigger the none on close. No error should return.
		evh.process_close(&mut ctx, &mut rwi, &mut ThreadContext::new())?;
		Ok(())
	}

	#[test]
	fn test_evh_on_read_none() -> Result<(), Error> {
		let port = free_port!()?;
		info!("eventhandler on_read_none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			let mut wh = conn_data.write_handle();

			spawn(move || -> Result<(), Error> {
				sleep(Duration::from_millis(1000));
				wh.trigger_on_read()?;
				Ok(())
			});
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_read_none();
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));
		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"test")?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read_none() -> Result<(), Error> {
		let port = free_port!()?;
		info!("eventhandler trigger_on_read_none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_read_none();

		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		sleep(Duration::from_millis(1_000));

		let connection2 = TcpStream::connect(addr)?;
		connection2.set_nonblocking(true)?;
		#[cfg(unix)]
		let connection_handle = connection2.into_raw_fd();
		#[cfg(windows)]
		let connection_handle = connection2.into_raw_socket().try_into()?;

		let client = ClientConnection {
			handle: connection_handle,
			tls_config: None,
		};

		info!("about to add client")?;
		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.trigger_on_read()?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
			tx: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_evh_debug_read_error() -> Result<(), Error> {
		let port = free_port!()?;
		info!("debug debug_read_error Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			debug!("debug read slab_offset = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			debug!("first_slab={}", first_slab)?;
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				assert_eq!(first_slab, last_slab);
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				let mut ret: Vec<u8> = vec![];
				ret.extend(&slab.get()[0..slab_offset as usize]);
				Ok(ret)
			})?;
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_debug_read_error(true);

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test1");
		info!("read back buf[{}] = {:?}", len, buf)?;
		connection.write(b"test2")?;
		info!("write ok")?;
		let res = connection.read(&mut buf);
		assert!(res.is_err() || res.unwrap() == 0);
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_process_events_other_situations() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut wakeup = Wakeup::new()?;
		ctx.counter = 0;
		ctx.count = 1;
		ctx.events[0] = Event {
			handle: 1000,
			etype: EventType::Read,
		};

		// both of these will succeed with warning printed
		// TODO: would be good to do an assertion that verifies these
		evh.process_events(&mut ctx, &mut wakeup, &mut ThreadContext::new())?;
		ctx.handle_hashtable.insert(&1000, &2000)?;
		ctx.counter = 0;
		ctx.count = 1;
		evh.process_events(&mut ctx, &mut wakeup, &mut ThreadContext::new())?;

		Ok(())
	}

	#[test]
	fn test_evh_process_write_errors() -> Result<(), Error> {
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.set_on_close_none();

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
			tx: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read2() -> Result<(), Error> {
		let port = free_port!()?;
		info!("eventhandler trigger_on_read2 none Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut on_read_count = lock_box!(0)?;
		let on_read_count_clone = on_read_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("in on read")?;
			let mut wh = conn_data.write_handle();
			assert!(wh.write_state().is_ok());
			let mut on_read_count = on_read_count.wlock()?;
			let guard = on_read_count.guard();
			**guard += 1;

			// only trigger on on read for the first request
			if **guard == 1 {
				info!("about to spawn thread")?;

				spawn(move || -> Result<(), Error> {
					info!("spawned thread")?;
					sleep(Duration::from_millis(1000));
					info!("trigger on read")?;
					wh.trigger_on_read()?;
					Ok(())
				});
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));
		let mut connection = TcpStream::connect(addr)?;
		connection.write(b"test1")?;

		let mut count = 0;
		loop {
			{
				let on_read_count_clone = on_read_count_clone.rlock()?;
				let guard = on_read_count_clone.guard();
				if **guard == 2 || count > 10_000 {
					break;
				} else {
					info!("sleep {}", count)?;
					count += 1;
				}
			}
			sleep(Duration::from_millis(5));
		}

		sleep(Duration::from_millis(1_000));

		let on_read_count_clone = on_read_count_clone.rlock()?;
		let guard = on_read_count_clone.guard();
		assert_eq!(**guard, 2);

		Ok(())
	}

	#[test]
	fn test_evh_process_accept_no_attachment() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler test_evh_process_accept_no_attachment Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;

		let mut on_read_count = lock_box!(0)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("in on read")?;
			let mut wh = conn_data.write_handle();
			assert!(wh.write_state().is_ok());
			let mut on_read_count = on_read_count.wlock()?;
			let guard = on_read_count.guard();
			**guard += 1;

			// only trigger on on read for the first request
			if **guard == 1 {
				info!("about to spawn thread")?;

				spawn(move || -> Result<(), Error> {
					info!("spawned thread")?;
					sleep(Duration::from_millis(1000));
					info!("trigger on read")?;
					wh.trigger_on_read()?;
					Ok(())
				});
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let mut ctx = EventHandlerContext::new(0, 10, 10, 10, 100)?;
		let mut tc = ThreadContext::new();
		let handles = create_listeners(threads, addr, 10, false)?;
		let li = ListenerInfo {
			handle: handles[0],
			id: random(),
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		};
		let ret = evh.process_accept(&li, &mut ctx, &mut tc)?;

		#[cfg(unix)]
		assert_eq!(ret, -1);
		#[cfg(windows)]
		assert_eq!(ret, usize::MAX);

		info!("about to call create listeners")?;
		// try with an actual socket
		let li = ListenerInfo {
			handle: handles[0],
			id: 1,
			is_reuse_port: false,
			tls_config: None,
			tx: None,
			ready: lock_box!(true)?,
		};
		ctx.debug_bypass_acc_err = true;

		info!("about to call process_accept")?;
		let ret = evh.process_accept(&li, &mut ctx, &mut tc)?;
		info!("ret={}", ret)?;

		#[cfg(unix)]
		assert_eq!(ret, -1);
		#[cfg(windows)]
		assert_eq!(ret, usize::MAX);

		Ok(())
	}

	#[test]
	fn test_evh_make_config() -> Result<(), Error> {
		assert!(make_config(None).is_ok());
		Ok(())
	}

	#[test]
	fn test_evh_debug_rw_accept_id_none() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler test_evh_debug_rw_accept_id_none Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_rw_accept_id_none();

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			info!("in on read: attachment.is_some()={}", attachment.is_some())?;
			let mut wh = conn_data.write_handle();
			if attachment.is_none() {
				wh.write(b"none")?;
			} else {
				wh.write(b"some")?;
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"none");
		info!("read back buf[{}] = {:?}", len, buf)?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
			tx: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_debug_attachment_none() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler test_evh_debug_attachment_none none Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_attachment_none();

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			info!("in on read: attachment.is_some()={}", attachment.is_some())?;
			let mut wh = conn_data.write_handle();
			if attachment.is_none() {
				wh.write(b"none")?;
			} else {
				wh.write(b"some")?;
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let handles = create_listeners(threads + 1, addr2, 10, false)?;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_err());

		let port = free_port!()?;
		info!("basic Using port: {}", port)?;
		let addr2 = &format!("127.0.0.1:{}", port)[..];
		let mut handles = create_listeners(threads, addr2, 10, false)?;
		handles[0] = 0;
		info!("handles={:?}", handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		assert!(evh.add_server(sc, Box::new("")).is_ok());
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		info!("about to write")?;
		connection.write(b"test1")?;
		let mut buf = vec![];
		buf.resize(100, 0u8);
		info!("about to read")?;
		let len = connection.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"none");
		info!("read back buf[{}] = {:?}", len, buf)?;

		let mut ctx = EventHandlerContext::new(0, 100, 100, 100, 100)?;

		let mut rwi = StreamInfo {
			id: 1001,
			handle: 1001,
			accept_handle: None,
			accept_id: None,
			write_state: lock_box!(WriteState {
				write_buffer: vec!['a' as u8],
				flags: 0
			})?,
			first_slab: u32::MAX,
			last_slab: u32::MAX,
			slab_offset: 0,
			is_accepted: false,
			tls_client: None,
			tls_server: None,
			tx: None,
		};
		let ci = ConnectionInfo::StreamInfo(rwi.clone());
		ctx.handle_hashtable.insert(&1001, &1001)?;
		ctx.connection_hashtable.insert(&1001, &ci)?;
		ctx.debug_trigger_on_read = true;
		evh.process_write(&mut rwi, &mut ctx, &mut ThreadContext::new())?;

		evh.stop()?;

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_close_handle() -> Result<(), Error> {
		assert!(CloseHandle::new(
			&mut lock_box!(WriteState {
				write_buffer: vec![],
				flags: 0,
			})?,
			0,
			&mut lock_box!(EventHandlerData::new(
				100,
				100,
				Wakeup::new()?,
				false,
				false,
				false,
				false
			)?)?,
		)
		.close()
		.is_ok());
		Ok(())
	}

	#[test]
	fn test_debug_close_handle() -> Result<(), Error> {
		let port = free_port!()?;
		info!(
			"eventhandler test_evh_debug_attachment_none none Using port: {}",
			port
		)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 1;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 2,
			max_handles_per_thread: 5,
			..Default::default()
		};
		let mut evh = EventHandlerImpl::new(config)?;
		evh.set_debug_close_handle(true);

		evh.set_on_read(move |conn_data, _thread_context, attachment| {
			info!("in on read: attachment.is_some()={}", attachment.is_some())?;
			let mut wh = conn_data.write_handle();
			if attachment.is_none() {
				wh.write(b"none")?;
			} else {
				wh.write(b"some")?;
			}
			Ok(())
		})?;
		evh.set_on_accept(move |_conn_data, _thread_context| {
			info!("on accept")?;
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let _c = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(5_000));
		evh.stop()?;
		sleep(Duration::from_millis(5_000));

		Ok(())
	}
}
