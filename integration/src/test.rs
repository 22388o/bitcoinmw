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
	use bmw_evh::{
		close_handle, create_listeners, ClientConnection, ConnData, EventHandler,
		EventHandlerConfig, ServerConnection, TlsClientConfig, TlsServerConfig,
		READ_SLAB_NEXT_OFFSET, READ_SLAB_SIZE,
	};

	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::port::pick_free_port;
	use bmw_util::*;
	use std::io::Read;
	use std::io::Write;
	use std::net::TcpStream;
	#[cfg(unix)]
	use std::os::unix::io::IntoRawFd;
	#[cfg(windows)]
	use std::os::windows::io::IntoRawSocket;
	use std::thread::sleep;
	use std::time::Duration;

	#[cfg(unix)]
	use std::os::unix::io::AsRawFd;
	#[cfg(windows)]
	use std::os::windows::io::AsRawSocket;

	info!();

	#[test]
	fn test_evh_basic() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("basic Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

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
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
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

		let port = pick_free_port()?;
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

		let port = pick_free_port()?;
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
		info!("write complete")?;
		let len = connection.read(&mut buf)?;
		info!("read bcak len = {}", len)?;
		assert_eq!(&buf[0..len], b"test2");
		evh.stop()?;
		info!("stop complete")?;

		Ok(())
	}

	#[test]
	fn test_evh_tls_basic() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_basic Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 10,
				..Default::default()
			};
			let mut evh = bmw_evh::Builder::build_evh(config)?;

			let mut client_handle = lock_box!(0)?;
			let client_handle_clone = client_handle.clone();

			let mut client_received_test1 = lock_box!(false)?;
			let mut server_received_test1 = lock_box!(false)?;
			let mut server_received_abc = lock_box!(false)?;
			let client_received_test1_clone = client_received_test1.clone();
			let server_received_test1_clone = server_received_test1.clone();
			let server_received_abc_clone = server_received_abc.clone();

			evh.set_on_read(move |conn_data, _thread_context, _attachment| {
				info!(
					"on read handle={},id={}",
					conn_data.get_handle(),
					conn_data.get_connection_id()
				)?;
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
				let client_handle = client_handle_clone.rlock()?;
				let guard = client_handle.guard();
				if conn_data.get_handle() != **guard {
					info!("client res = {:?}", res)?;
					if res[0] == 't' as u8 {
						conn_data.write_handle().write(&res)?;
						if res == b"test1".to_vec() {
							let mut server_received_test1 = server_received_test1.wlock()?;
							(**server_received_test1.guard()) = true;
						}
					}
					if res == b"abc".to_vec() {
						let mut server_received_abc = server_received_abc.wlock()?;
						(**server_received_abc.guard()) = true;
					}
				} else {
					info!("server res = {:?})", res)?;
					let mut x = vec![];
					x.extend(b"abc");
					conn_data.write_handle().write(&x)?;
					if res == b"test1".to_vec() {
						let mut client_received_test1 = client_received_test1.wlock()?;
						(**client_received_test1.guard()) = true;
					}
				}
				info!("res={:?}", res)?;
				Ok(())
			})?;
			evh.set_on_accept(move |conn_data, _thread_context| {
				info!(
					"accept a connection handle = {},id={}",
					conn_data.get_handle(),
					conn_data.get_connection_id()
				)?;
				Ok(())
			})?;
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

			wh.write(b"test1")?;
			let mut count = 0;
			loop {
				sleep(Duration::from_millis(1));
				if !(**(client_received_test1_clone.rlock()?.guard())
					&& **(server_received_test1_clone.rlock()?.guard())
					&& **(server_received_abc_clone.rlock()?.guard()))
				{
					count += 1;
					if count < 2_000 {
						continue;
					}
				}
				assert!(**(client_received_test1_clone.rlock()?.guard()));
				assert!(**(server_received_test1_clone.rlock()?.guard()));
				assert!(**(server_received_abc_clone.rlock()?.guard()));
				break;
			}

			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_tls_client_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_client_error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evh = bmw_evh::Builder::build_evh(config)?;

			evh.set_on_read(move |conn_data, _thread_context, _attachment| {
				debug!("on read slab_offset = {}", conn_data.slab_offset())?;
				let first_slab = conn_data.first_slab();
				let last_slab = conn_data.last_slab();
				let slab_offset = conn_data.slab_offset();
				debug!("first_slab={}", first_slab)?;
				conn_data.borrow_slab_allocator(move |sa| {
					let slab = sa.get(first_slab.try_into()?)?;
					assert_eq!(first_slab, last_slab);
					info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				})?;
				conn_data.clear_through(first_slab)?;
				for _ in 0..3 {
					conn_data.write_handle().write(b"test")?;
					sleep(Duration::from_millis(1_000));
				}
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
			info!("client handle = {}", connection_handle)?;
			let mut wh = evh.add_client(client, Box::new(""))?;

			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(1_000));
			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(1_000));
			let _ = wh.write(b"test1");
			sleep(Duration::from_millis(10_000));
			evh.stop()?;
		}

		Ok(())
	}

	#[test]
	fn test_evh_tls_error() -> Result<(), Error> {
		{
			let port = pick_free_port()?;
			info!("eventhandler tls_error Using port: {}", port)?;
			let addr = &format!("127.0.0.1:{}", port)[..];
			let threads = 2;
			let config = EventHandlerConfig {
				threads,
				housekeeping_frequency_millis: 100_000,
				read_slab_count: 100,
				max_handles_per_thread: 3,
				..Default::default()
			};
			let mut evh = bmw_evh::Builder::build_evh(config)?;

			evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
			evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
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
			sleep(Duration::from_millis(5_000));

			// connect and send clear text. Internally an error should occur and
			// warning printed. Processing continues though.
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test")?;
			sleep(Duration::from_millis(1000));
			connection.write(b"test")?;
			sleep(Duration::from_millis(1000));
			let _ = connection.write(b"test");
			evh.stop()?;
		}

		sleep(Duration::from_millis(2000));

		Ok(())
	}

	#[test]
	fn test_evh_close1() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("close1 Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000_000,
			read_slab_count: 30,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

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
			close_handle(handle)?;
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
	fn test_evh_server_close() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("server_close Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 10,
			max_handles_per_thread: 20,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

		let mut close_count = lock_box!(0)?;
		let close_count_clone = close_count.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				let slab = sa.get(first_slab.try_into()?)?;
				info!("read bytes = {:?}", &slab.get()[0..slab_offset as usize])?;
				if slab.get()[0] == 'x' as u8 {
					Ok(vec![])
				} else {
					let mut ret: Vec<u8> = vec![];
					ret.extend(&slab.get()[0..slab_offset as usize]);
					Ok(ret)
				}
			})?;
			conn_data.clear_through(first_slab)?;
			if res.len() > 0 {
				info!("write back data")?;
				conn_data.write_handle().write(&res)?;
			} else {
				info!("do the close")?;
				conn_data.write_handle().close()?;
				assert!(conn_data.write_handle().write(b"test").is_err());
				assert!(conn_data.write_handle().suspend().is_ok());
				assert!(conn_data.write_handle().resume().is_ok());
				assert!(conn_data.write_handle().close().is_ok());
			}
			info!("res={:?}", res)?;
			Ok(())
		})?;
		evh.set_on_accept(move |conn_data, _thread_context| {
			info!("accept a connection handle = {}", conn_data.get_handle())?;
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
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
			connection.write(b"xabc")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(len, 0);

			let mut count = 0;
			loop {
				count += 1;
				sleep(Duration::from_millis(1));
				if **((close_count_clone.rlock()?).guard()) == 0 && count < 2_000 {
					continue;
				}
				assert_eq!(**((close_count_clone.rlock()?).guard()), 1);
				break;
			}
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_multi_slab_message() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("multi_slab_message Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 1_000,
			read_slab_count: 3,
			max_handles_per_thread: 2,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read fs = {}", conn_data.slab_offset())?;
			let first_slab = conn_data.first_slab();
			let last_slab = conn_data.last_slab();
			let slab_offset = conn_data.slab_offset();
			let res = conn_data.borrow_slab_allocator(move |sa| {
				assert_ne!(first_slab, last_slab);

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
			conn_data.clear_through(first_slab)?;
			conn_data.write_handle().write(&res)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_conn_data, _thread_context| {
			// test returning an error on accept. It doesn't affect processing
			Err(err!(ErrKind::Test, "test on acc err"))
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		evh.start()?;
		let handles = create_listeners(threads, addr, 10, false)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;
		sleep(Duration::from_millis(5_000));

		let mut connection = TcpStream::connect(addr)?;
		let mut message = ['a' as u8; 1024];
		for i in 0..1024 {
			message[i] = 'a' as u8 + (i % 26) as u8;
		}
		connection.write(&message)?;
		let mut buf = vec![];
		buf.resize(2000, 0u8);
		let len = connection.read(&mut buf)?;
		assert_eq!(len, 1024);
		for i in 0..len {
			assert_eq!(buf[i], 'a' as u8 + (i % 26) as u8);
		}

		sleep(Duration::from_millis(5_000));
		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_client() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("eventhandler client Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 100_000,
			read_slab_count: 1,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

		let mut client_handle = lock_box!(0)?;
		let client_handle_clone = client_handle.clone();

		let mut client_received_test1 = lock_box!(false)?;
		let mut server_received_test1 = lock_box!(false)?;
		let mut server_received_abc = lock_box!(false)?;
		let client_received_test1_clone = client_received_test1.clone();
		let server_received_test1_clone = server_received_test1.clone();
		let server_received_abc_clone = server_received_abc.clone();

		evh.set_on_read(move |conn_data, _thread_context, _attachment| {
			info!("on read handle={}", conn_data.get_handle())?;
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
			let client_handle = client_handle_clone.rlock()?;
			let guard = client_handle.guard();
			if conn_data.get_handle() != **guard {
				if res[0] == 't' as u8 {
					conn_data.write_handle().write(&res)?;
					if res == b"test1" {
						let mut server_received_test1 = server_received_test1.wlock()?;
						(**server_received_test1.guard()) = true;
					}
				}
				if res == b"abc".to_vec() {
					let mut server_received_abc = server_received_abc.wlock()?;
					(**server_received_abc.guard()) = true;
				}
			} else {
				let mut x = vec![];
				x.extend(b"abc");
				conn_data.write_handle().write(&x)?;
				if res == b"test1".to_vec() {
					let mut client_received_test1 = client_received_test1.wlock()?;
					(**client_received_test1.guard()) = true;
				}
			}
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
		evh.start()?;

		let handles = create_listeners(threads, addr, 10, false)?;
		info!("handles.size={},handles={:?}", handles.size(), handles)?;
		let sc = ServerConnection {
			tls_config: None,
			handles,
			is_reuse_port: false,
		};
		evh.add_server(sc, Box::new(""))?;

		let connection = TcpStream::connect(addr)?;
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
			tls_config: None,
		};
		let mut wh = evh.add_client(client, Box::new(""))?;

		wh.write(b"test1")?;
		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			if !(**(client_received_test1_clone.rlock()?.guard())
				&& **(server_received_test1_clone.rlock()?.guard())
				&& **(server_received_abc_clone.rlock()?.guard()))
			{
				count += 1;
				if count < 25_000 {
					continue;
				}
			}
			assert!(**(client_received_test1_clone.rlock()?.guard()));
			assert!(**(server_received_test1_clone.rlock()?.guard()));
			assert!(**(server_received_abc_clone.rlock()?.guard()));
			break;
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_is_reuse_port() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("is_ reuse port Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

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

		let mut tid0count = lock_box!(0)?;
		let mut tid1count = lock_box!(0)?;
		let tid0count_clone = tid0count.clone();
		let tid1count_clone = tid1count.clone();
		evh.set_on_accept(move |conn_data, _thread_context| {
			if conn_data.tid() == 0 {
				let mut tid0count = tid0count.wlock()?;
				let guard = tid0count.guard();
				(**guard) += 1;
			} else if conn_data.tid() == 1 {
				let mut tid1count = tid1count.wlock()?;
				let guard = tid1count.guard();
				(**guard) += 1;
			}
			Ok(())
		})?;
		evh.set_on_close(move |_conn_data, _thread_context| {
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
		sleep(Duration::from_millis(5_000));

		{
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}

		let total = 100;
		for _ in 0..total {
			let mut connection = TcpStream::connect(addr)?;
			connection.write(b"test1")?;
			let mut buf = vec![];
			buf.resize(100, 0u8);
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test1");
			connection.write(b"test2")?;
			let len = connection.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test2");
		}

		let mut count_count = 0;
		loop {
			count_count += 1;
			sleep(Duration::from_millis(1));
			let count = **((close_count_clone.rlock()?).guard());
			if count != total + 1 && count_count < 1_000 {
				continue;
			}
			assert_eq!((**((close_count_clone.rlock()?).guard())), total + 1);
			break;
		}

		let tid0count = **(tid0count_clone.rlock()?.guard());
		let tid1count = **(tid1count_clone.rlock()?.guard());
		info!("tid0count={},tid1count={}", tid0count, tid1count)?;
		#[cfg(target_os = "linux")]
		{
			assert_ne!(tid0count, 0);
			assert_ne!(tid1count, 0);
		}

		evh.stop()?;

		Ok(())
	}

	#[test]
	fn test_evh_stop() -> Result<(), Error> {
		let port = pick_free_port()?;
		info!("stop Using port: {}", port)?;
		let addr = &format!("127.0.0.1:{}", port)[..];
		let threads = 2;
		let config = EventHandlerConfig {
			threads,
			housekeeping_frequency_millis: 10_000,
			read_slab_count: 2,
			max_handles_per_thread: 3,
			..Default::default()
		};
		let mut evh = bmw_evh::Builder::build_evh(config)?;

		evh.set_on_read(move |_conn_data, _thread_context, _attachment| Ok(()))?;
		evh.set_on_accept(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_close(move |_conn_data, _thread_context| Ok(()))?;
		evh.set_on_panic(move |_thread_context, _e| Ok(()))?;
		evh.set_housekeeper(move |_thread_context| Ok(()))?;

		assert!(evh.stop().is_err());

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

		let mut connection = TcpStream::connect(addr)?;
		sleep(Duration::from_millis(1000));
		evh.stop()?;
		sleep(Duration::from_millis(1000));

		let mut buf = vec![];
		buf.resize(100, 0u8);
		let res = connection.read(&mut buf);

		assert!(res.is_err() || res.unwrap() == 0);

		Ok(())
	}
}
