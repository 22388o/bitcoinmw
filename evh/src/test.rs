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
	use crate as bmw_evh;
	use crate::types::{
		ConnectionType, ConnectionVariant, DebugInfo, EventHandlerCallbacks, EventHandlerConfig,
		EventHandlerContext, EventHandlerImpl, EventHandlerState, EvhStats, GlobalStats,
		UserContextImpl, Wakeup, WriteHandle, WriteState,
	};
	use crate::{evh, evh_oro, Connection, EvhBuilder, UserContext};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::{HashMap, VecDeque};
	use std::io::{Read, Write};
	use std::net::TcpStream;
	use std::str::from_utf8;
	use std::thread;

	#[cfg(target_os = "linux")]
	use crate::linux::*;
	#[cfg(target_os = "macos")]
	use crate::mac::*;
	#[cfg(target_os = "windows")]
	use crate::win::*;

	info!();

	#[test]
	fn test_wakeup_impl() -> Result<(), Error> {
		let debug_info = DebugInfo::default();
		let (x, y) = wakeup_impl()?;
		write_impl(y, b"test")?;
		let mut buf = [0u8; 100];
		let len = read_impl(x, &mut buf, &debug_info)?;
		assert_eq!(len, Some(4));
		Ok(())
	}

	#[test]
	fn test_evh_os() -> Result<(), Error> {
		let debug_info = DebugInfo {
			os_error: lock_box!(true)?,
			..Default::default()
		};
		let port = pick_free_port()?;
		let addr = format!("127.0.0.1:{}", port);
		assert!(create_listener(&addr, 1, &debug_info).is_err());

		let port = pick_free_port()?;
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 5)?;
		assert!(accept_impl(conn.handle(), &debug_info).is_err());

		let mut buf = [0u8; 100];
		assert!(read_impl(conn.handle(), &mut buf, &debug_info).is_err());
		Ok(())
	}

	#[test]
	fn test_evh_basic1() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(5000)
		)?;

		let mut client_id = lock_box!(0)?;

		let mut client_recv = lock_box!(false)?;
		let mut server_recv = lock_box!(false)?;
		let client_recv_clone = client_recv.clone();
		let server_recv_clone = server_recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let client_id_clone = client_id.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			assert_eq!(dstring, "hi");

			let mut wh = connection.write_handle()?;

			assert_eq!(wh.id(), connection.id());

			// echo
			if rlock!(client_id_clone) != connection.id() {
				wh.write(dstring.as_bytes())?;
				wlock!(server_recv) = true;
			} else {
				wlock!(client_recv) = true;
			}

			if rlock!(client_recv) && rlock!(server_recv) {
				tx.send(())?;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		let (tx2, rx2) = test_info.sync_channel();
		let mut counter = lock_box!(0)?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			info!("onClose")?;
			if rlock!(counter) == 1 {
				tx2.send(())?;
			}
			wlock!(counter) += 1;
			Err(err!(ErrKind::Test, "simulated error"))
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;
		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		wlock!(client_id) = conn2.id();

		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"hi")?;

		rx.recv()?;
		assert!(rlock!(client_recv_clone));
		assert!(rlock!(server_recv_clone));

		wh.close()?;

		rx2.recv()?;
		Ok(())
	}

	#[test]
	fn test_evh_handler_errors() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(1),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(5000),
			EvhHouseKeeperFrequencyMillis(2)
		)?;

		let mut on_read_count = lock_box!(0)?;
		let (tx, rx) = test_info.sync_channel();
		let (tx2, rx2) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			wlock!(on_read_count) += 1;

			if rlock!(on_read_count) == 3 {
				panic!("test panic");
			}

			ctx.clear_all(connection)?;

			if rlock!(on_read_count) == 1 {
				tx.send(())?;
				Err(err!(ErrKind::Test, "on read err"))
			} else {
				info!("write a byte")?;
				connection.write_handle()?.write(b"0")?;
				Ok(())
			}
		})?;

		evh.set_on_accept(move |_connection, _ctx| -> Result<(), Error> {
			Err(err!(ErrKind::Test, "on accept err"))
		})?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			Err(err!(ErrKind::Test, "on close err"))
		})?;

		let mut tx2_sent = lock_box!(false)?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			if !rlock!(tx2_sent) {
				tx2.send(())?;
			}

			wlock!(tx2_sent) = true;
			Err(err!(ErrKind::Test, "on housekeeper err"))
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			Err(err!(ErrKind::Test, "on_panic err"))
		})?;

		evh.start()?;

		// wait for house keeper to execute
		info!("wait for a housekeeper")?;
		rx2.recv()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		evh.add_server_connection(conn)?;

		{
			let mut strm = TcpStream::connect(addr.clone())?;

			strm.write(b"x")?;
			rx.recv()?;
			strm.write(b"y")?;
			let mut buf = [8u8; 100];
			let len = strm.read(&mut buf)?;
			assert_eq!(len, 1);
			assert_eq!(buf[0], b'0');
		}
		// stream will close. Ensure we can still connect
		{
			let mut strm = TcpStream::connect(addr.clone())?;

			// this causes a thread panic (counter == 3)
			strm.write(b"x")?;
			let mut buf = [8u8; 100];
			let len = strm.read(&mut buf)?;
			// closed
			assert_eq!(len, 0);
		}
		// stream will close. Ensure we can still connect
		{
			// now everything is normal
			let mut strm = TcpStream::connect(addr)?;
			strm.write(b"x")?;
			let mut buf = [8u8; 100];
			let len = strm.read(&mut buf)?;
			assert_eq!(len, 1);
			assert_eq!(buf[0], b'0');
		}

		Ok(())
	}

	#[test]
	fn test_evh_basic_pending() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(5000)
		)?;

		let debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info)?;

		let mut client_id = lock_box!(0)?;

		let mut client_recv = lock_box!(false)?;
		let mut server_recv = lock_box!(false)?;
		let client_recv_clone = client_recv.clone();
		let server_recv_clone = server_recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let client_id_clone = client_id.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			assert_eq!(dstring, "hi");

			let mut wh = connection.write_handle()?;

			// echo
			if rlock!(client_id_clone) != connection.id() {
				wh.write(dstring.as_bytes())?;
				wlock!(server_recv) = true;
			} else {
				wlock!(client_recv) = true;
			}

			if rlock!(client_recv) && rlock!(server_recv) {
				tx.send(())?;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			info!("onClose")?;
			Ok(())
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;
		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		wlock!(client_id) = conn2.id();

		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"hi")?;

		rx.recv()?;
		assert!(rlock!(client_recv_clone));
		assert!(rlock!(server_recv_clone));

		Ok(())
	}

	#[test]
	fn test_evh_wrong_add() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);

		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(5000)
		)?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			ctx.clear_all(connection)?;
			Ok(())
		})?;
		evh.start()?;

		// show adding a server as a client and vice-versa is an error
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		assert!(evh.add_client_connection(conn).is_err());
		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		assert!(evh.add_server_connection(conn2).is_err());
		Ok(())
	}

	#[test]
	fn test_evh_oro() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(true),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();
		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			assert_eq!(dstring, "hi");
			wlock!(recv_msg) = true;
			tx.send(())?;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"hi")?;

		rx.recv()?;
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_stop() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);

		let mut strm;
		{
			let mut evh = evh_oro!(EvhThreads(2), EvhTimeout(u16::MAX))?;
			evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
				let mut data: Vec<u8> = vec![];

				loop {
					let next_chunk = ctx.next_chunk(connection)?;
					cbreak!(next_chunk.is_none());
					let next_chunk = next_chunk.unwrap();
					data.extend(next_chunk.data());
				}

				let dstring = from_utf8(&data)?;
				info!("data[{}]='{}'", connection.id(), dstring,)?;
				connection.write_handle()?.write(b"ok")?;
				ctx.clear_all(connection)?;
				Ok(())
			})?;
			evh.start()?;
			let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
			info!("conn.handle = {}", conn.handle())?;
			evh.add_server_connection(conn)?;

			strm = TcpStream::connect(addr.clone())?;
			strm.write(b"test")?;
			let mut buf = [0u8; 100];
			let res = strm.read(&mut buf)?;
			assert_eq!(res, 2);
		}

		let mut buf = [0u8; 100];
		let res = strm.read(&mut buf)?;
		assert!(res == 0); // closed

		Ok(())
	}

	#[test]
	fn test_evh_housekeeping() -> Result<(), Error> {
		let threads = 10;
		let mut evh = evh!(
			EvhThreads(threads),
			EvhTimeout(1),
			EvhHouseKeeperFrequencyMillis(2)
		)?;
		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			info!("onClose")?;
			Ok(())
		})?;

		let mut thread_hash = lock_box!(HashMap::new())?;
		let mut complete_count = lock_box!(0)?;
		let mut complete = lock_box!(false)?;
		let complete_clone = complete.clone();
		let test_info = test_info!()?;

		let (tx, rx) = test_info.sync_channel();

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			let id = thread::current().id();
			let mut thread_hash = thread_hash.wlock()?;
			let guard = thread_hash.guard()?;
			let count = match (**guard).get(&id) {
				Some(count) => count + 1,
				None => 0,
			};

			(**guard).insert(id, count);
			info!("count = {}", count)?;
			if count == 5 {
				wlock!(complete_count) += 1;

				if rlock!(complete_count) == threads {
					wlock!(complete) = true;
					tx.send(())?;
				}
			}

			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;

		rx.recv()?;
		assert!(rlock!(complete_clone));

		Ok(())
	}

	#[test]
	fn test_evh_panic1() -> Result<(), Error> {
		let test_info = test_info!()?;
		let threads = 1;

		let mut evh = evh_oro!(
			EvhThreads(threads),
			EvhTimeout(u16::MAX),
			EvhHouseKeeperFrequencyMillis(usize::MAX)
		)?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;
			if dstring == "crash\r\n" {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			}
			let mut wh = connection.write_handle()?;
			wh.write(&data)?;

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("Host on {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;

		let mut strm1 = TcpStream::connect(addr.clone())?;
		let mut strm2 = TcpStream::connect(addr)?;

		strm1.write(b"x")?;
		let mut buf = [0u8; 100];
		let len = strm1.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(buf[0], b'x');
		strm1.write(b"crash\r\n")?;

		assert!(strm1.read(&mut buf)? == 0);
		strm2.write(b"y")?;
		let len = strm2.read(&mut buf)?;
		assert_eq!(len, 1);
		assert_eq!(buf[0], b'y');
		info!("got y response back")?;

		Ok(())
	}

	#[test]
	fn test_evh_panic_advanced() -> Result<(), Error> {
		let test_info = test_info!()?;
		let threads = 1;

		let mut evh = evh_oro!(
			EvhThreads(threads),
			EvhTimeout(u16::MAX),
			EvhHouseKeeperFrequencyMillis(usize::MAX)
		)?;

		let spin_lock1 = lock_box!(false)?;
		let mut spin_lock1_clone = spin_lock1.clone();

		let spin_lock2 = lock_box!(false)?;
		let mut spin_lock2_clone = spin_lock2.clone();

		let spin_lock3 = lock_box!(false)?;
		let mut spin_lock3_clone = spin_lock3.clone();

		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;
			if dstring == "crash\r\n" {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			} else if dstring == "pause1\r\n" {
				tx.send(())?;
				info!("pause1")?;
				loop {
					if rlock!(spin_lock1) {
						break;
					}
					sleep(Duration::from_millis(10));
				}
				info!("pause1 complete")?;
				let mut wh = connection.write_handle()?;
				wh.write(b"p1complete")?;

				// introduce a small pause to ensure all i/o data is sent into a
				// single set of events. The test still works fine without this,
				// but it's better to excersise it all at once
				sleep(Duration::from_millis(10));
			} else if dstring == "pause2\r\n" {
				info!("pause2")?;
				loop {
					if rlock!(spin_lock2) {
						break;
					}
					sleep(Duration::from_millis(10));
				}
				info!("pause2 complete")?;
				let mut wh = connection.write_handle()?;
				wh.write(b"p2complete")?;
			} else if dstring == "pause3\r\n" {
				info!("pause3")?;
				loop {
					if rlock!(spin_lock3) {
						break;
					}
					sleep(Duration::from_millis(10));
				}
				info!("pause3 complete")?;
				let mut wh = connection.write_handle()?;
				wh.write(b"p3complete")?;
			} else {
				let mut wh = connection.write_handle()?;
				wh.write(&data)?;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("Host on {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;

		// create 4 streams
		let mut strm1 = TcpStream::connect(addr.clone())?;
		let mut strm2 = TcpStream::connect(addr.clone())?;
		let mut strm3 = TcpStream::connect(addr.clone())?;
		let mut strm4 = TcpStream::connect(addr.clone())?;

		// pause the thread so nothing else can be processed
		strm1.write(b"pause1\r\n")?;

		// wait to make sure that's the only event and it's in progress
		rx.recv()?;

		// now send crash
		strm4.write(b"crash\r\n")?;

		// now send two other pauses (these will all just queue up until we unlock the
		// pauses
		strm2.write(b"pause2\r\n")?;
		strm3.write(b"pause3\r\n")?;

		// unlock thread and let the test proceed
		info!("unlocking")?;
		wlock!(spin_lock3_clone) = true;
		wlock!(spin_lock2_clone) = true;
		wlock!(spin_lock1_clone) = true;

		// now try to read from each stream and ensure expected result
		let mut buf = [0u8; 1000];

		let len = strm1.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"p1complete");

		let len = strm2.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"p2complete");

		let len = strm3.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"p3complete");

		// strm 4 closed due to crash
		assert_eq!(strm4.read(&mut buf)?, 0);

		Ok(())
	}

	#[test]
	fn test_evh_panic_trigger_on_read() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let mut wh = connection.write_handle()?;

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			if dstring == "hi\r\n" {
				wh.write(b"1111")?;
				spawn(move || -> Result<(), Error> {
					info!("trigger on read")?;
					wh.trigger_on_read()?;
					info!("trigger complete")?;
					Ok(())
				});
			} else if dstring == "" {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		info!("connecting to {}", addr)?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"hi\r\n")?;

		let mut buf = [0u8; 100];

		let len = strm.read(&mut buf)?;
		assert_eq!(len, 4);

		info!("data={:?}", &buf[0..len])?;
		assert_eq!(&buf[0..len], [49, 49, 49, 49]);

		assert_eq!(strm.read(&mut buf)?, 0);

		Ok(())
	}

	#[test]
	fn test_evh_trigger_on_read() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let mut wh = connection.write_handle()?;

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			if dstring == "hi" {
				wh.write(b"1111")?;
				spawn(move || -> Result<(), Error> {
					wh.trigger_on_read()?;
					Ok(())
				});
			} else if dstring == "" {
				wh.write(b"2222")?;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"hi")?;

		let mut buf = [0u8; 100];
		let mut len_sum = 0;

		loop {
			let len = strm.read(&mut buf[len_sum..])?;
			len_sum += len;
			if len_sum == 8 {
				break;
			}
		}

		info!("data={:?}", &buf[0..len_sum])?;
		assert_eq!(&buf[0..len_sum], [49, 49, 49, 49, 50, 50, 50, 50]);

		Ok(())
	}

	#[test]
	fn test_evh_invalid_configs() -> Result<(), Error> {
		// timeout == 0
		let error;
		match evh_oro!(
			Debug(false),
			EvhTimeout(0),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(3_000)
		) {
			Ok(mut evh) => {
				evh.set_on_read(move |_, _| -> Result<(), Error> { Ok(()) })?;
				error = false;
			}
			Err(_) => {
				error = true;
			}
		}
		assert!(error);

		// read_slab_size < 25
		let error;
		match evh_oro!(
			Debug(false),
			EvhTimeout(1),
			EvhThreads(1),
			EvhReadSlabSize(10),
			EvhStatsUpdateMillis(3_000)
		) {
			Ok(mut evh) => {
				evh.set_on_read(move |_, _| -> Result<(), Error> { Ok(()) })?;
				error = false;
			}
			Err(_) => {
				error = true;
			}
		}
		assert!(error);

		// read_slab_count == 0
		let error;
		match evh_oro!(
			Debug(false),
			EvhTimeout(1),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhReadSlabCount(0),
			EvhStatsUpdateMillis(3_000)
		) {
			Ok(mut evh) => {
				evh.set_on_read(move |_, _| -> Result<(), Error> { Ok(()) })?;
				error = false;
			}
			Err(_) => {
				error = true;
			}
		}
		assert!(error);

		// housekeeping_frequency_millis == 0
		let error;
		match evh_oro!(
			Debug(false),
			EvhTimeout(1),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhHouseKeeperFrequencyMillis(0),
			EvhStatsUpdateMillis(3_000)
		) {
			Ok(mut evh) => {
				evh.set_on_read(move |_, _| -> Result<(), Error> { Ok(()) })?;
				error = false;
			}
			Err(_) => {
				error = true;
			}
		}
		assert!(error);

		Ok(())
	}

	#[test]
	fn test_evh_stats() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(10),
			EvhThreads(1),
			EvhReadSlabSize(100),
			EvhStatsUpdateMillis(3_000)
		)?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("addr={}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		{
			let _strm = TcpStream::connect(addr.clone())?;
			let _strm = TcpStream::connect(addr.clone())?;
			let _strm = TcpStream::connect(addr.clone())?;
			let _strm = TcpStream::connect(addr.clone())?;
			let _strm = TcpStream::connect(addr.clone())?;
		}
		let mut strm = TcpStream::connect(addr.clone())?;
		strm.write(b"test")?;

		let stats = evh.wait_for_stats()?;
		info!("stats={:?}", stats)?;

		// 1 left in scope has not disconnecte yet
		assert_eq!(stats.accepts, 6);
		assert_eq!(stats.closes, 5);
		assert_eq!(stats.reads, 1);
		assert!(stats.event_loops != 0);

		Ok(())
	}

	#[test]
	fn test_evh_trigger_empty() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();
		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			ctx.clear_all(connection)?;
			wlock!(recv_msg) = true;
			tx.send(())?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn2)?;
		wh.trigger_on_read()?;

		rx.recv()?;
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_short_buf() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();
		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let next_chunk = ctx.next_chunk(connection)?.unwrap();
			assert_ne!(next_chunk.slab_id(), usize::MAX);

			wlock!(recv_msg) = true;
			tx.send(())?;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"01234567890123456789")?;

		rx.recv()?;
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_multi_chunk() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(10),
			EvhThreads(1),
			EvhReadSlabSize(25),
		)?;

		let (tx, rx) = test_info.sync_channel();
		let b = b"0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
		let mut found_full = lock_box!(false)?;
		let found_full_read = found_full.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let mut wh = connection.write_handle()?;

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;

			if dstring == "hi" {
				wh.write(b"1111")?;
				spawn(move || -> Result<(), Error> {
					wh.trigger_on_read()?;
					Ok(())
				});
			} else if dstring == "" {
				wh.write(b"2222")?;
			}

			if data == b {
				wlock!(found_full) = true;
				tx.send(())?;
			}

			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("addr={}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		strm.write(b"0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789")?;

		rx.recv()?;
		assert!(rlock!(found_full_read));

		Ok(())
	}

	#[test]
	fn test_evh_user_data() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let mut recv_msg2 = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();
		let recv_msg2_clone = recv_msg2.clone();
		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let x = ctx.get_user_data();
			if !rlock!(recv_msg) {
				info!("none")?;
				assert!(x.is_none());
			} else {
				info!("some")?;
				assert_eq!(
					x.as_ref().unwrap().downcast_ref::<usize>().unwrap(),
					&1usize
				);
				wlock!(recv_msg2) = true;
				tx.send(())?;
			}
			ctx.set_user_data(Box::new(1usize));
			info!("onRead")?;

			let next_chunk = ctx.next_chunk(connection)?.unwrap();
			assert_ne!(next_chunk.slab_id(), usize::MAX);

			wlock!(recv_msg) = true;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"01234567890123456789")?;

		let conn3 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn3)?;
		wh.write(b"01234567890123456789")?;

		rx.recv()?;
		assert!(rlock!(recv_msg_clone));
		assert!(rlock!(recv_msg2_clone));

		Ok(())
	}

	#[test]
	fn test_evh_pending1() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info.clone())?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut wh = connection.write_handle()?;
			wh.write(b"test")?;
			wh.close()?;
			assert!(wh.write(b"test").is_err());
			assert!(wh.close().is_err());
			wlock!(recv_msg) = true;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 100)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"test");

		// closed connection
		assert_eq!(strm.read(&mut buf)?, 0);
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_read_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let debug_info = DebugInfo {
			read_err: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info.clone())?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut wh = connection.write_handle()?;
			wlock!(recv_msg) = true;
			wh.write(b"test")?;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 100)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		// this will be an error because the connection is closed when there's a read error
		assert!(strm.read(&mut buf).is_err());
		assert!(!rlock!(recv_msg_clone));

		evh.set_debug_info(DebugInfo::default())?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		// this will be an error because the connection is closed when there's a read error

		let len = strm.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..len], b"test");
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_write_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let debug_info = DebugInfo {
			write_err: lock_box!(true)?,
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info.clone())?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut wh = connection.write_handle()?;
			wh.write(b"test")?;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 100)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;
		// the len will be 0 because it will be closed by the server due to the write error
		assert_eq!(len, 0);

		// now update debug info to avoid the error

		evh.set_debug_info(DebugInfo::default())?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;
		// now it's corrected
		assert_eq!(len, 4);

		Ok(())
	}

	#[test]
	fn test_evh_write_handle_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let debug_info = DebugInfo {
			write_handle_err: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info.clone())?;

		let (tx, rx) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut wh = connection.write_handle()?;
			match wh.write(b"test") {
				Ok(_) => {
					info!("write ok")?;
				}
				Err(e) => {
					info!("write err: {}", e)?;
					tx.send(())?;
				}
			}
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 100)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		// trigger write handle error
		strm.write(b"01234567890123456789")?;

		rx.recv()?;

		// now fix it
		evh.set_debug_info(DebugInfo::default())?;

		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];

		let len = strm.read(&mut buf)?;
		// the message goes through now
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"test");

		Ok(())
	}

	#[test]
	fn test_evh_connection_close() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let recv_msg_clone = recv_msg.clone();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut wh = connection.write_handle()?;
			wh.write(b"test")?;
			wh.close()?;
			assert!(wh.write(b"test").is_err());
			assert!(wh.trigger_on_read().is_err());
			assert!(wh.close().is_err());
			wlock!(recv_msg) = true;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 100)?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr)?;
		strm.write(b"01234567890123456789")?;

		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;
		assert_eq!(len, 4);
		assert_eq!(&buf[0..4], b"test");

		// closed connection
		assert_eq!(strm.read(&mut buf)?, 0);
		assert!(rlock!(recv_msg_clone));

		Ok(())
	}

	#[test]
	fn test_evh_normal_fatal_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let threads = 1;

		let mut evh = evh!(
			EvhThreads(threads),
			EvhTimeout(u16::MAX),
			EvhHouseKeeperFrequencyMillis(usize::MAX)
		)?;
		evh.set_debug_info(DebugInfo {
			normal_fatal_error: lock_box!(true)?,
			..Default::default()
		})?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;
			let mut wh = connection.write_handle()?;
			wh.write(&data)?;

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_panic(move |_id, e| -> Result<(), Error> {
			let e = e.downcast_ref::<&str>().unwrap_or(&"unknown error");
			error!("on panic: {}", e)?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_close(move |_, _| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_housekeeper(move |_| -> Result<(), Error> { Ok(()) })?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("Host on {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		spawn(move || {
			// this will never complete due to the fatal error that occurs in the server
			// thread
			let _ = evh.add_server_connection(conn);
		});

		// sleep to ensure code is exercised
		sleep(Duration::from_millis(QA_SLEEP));

		Ok(())
	}

	#[test]
	fn test_evh_panic_fatal_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let threads = 1;

		let mut evh = evh!(
			EvhThreads(threads),
			EvhTimeout(u16::MAX),
			EvhHouseKeeperFrequencyMillis(usize::MAX)
		)?;
		evh.set_debug_info(DebugInfo {
			panic_fatal_error: lock_box!(true)?,
			..Default::default()
		})?;

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!("data[{}]='{}'", connection.id(), dstring,)?;
			if dstring == "crash\r\n" {
				let x: Option<u32> = None;
				let _y = x.unwrap();
			}
			let mut wh = connection.write_handle()?;
			wh.write(&data)?;

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		let (tx, rx) = test_info.sync_channel();
		evh.set_on_panic(move |_id, e| -> Result<(), Error> {
			let e = e.downcast_ref::<&str>().unwrap_or(&"unknown error");
			error!("on panic: {}", e)?;
			tx.send(())?;
			Ok(())
		})?;

		evh.set_on_accept(move |_, _| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_close(move |_, _| -> Result<(), Error> { Ok(()) })?;
		evh.set_on_housekeeper(move |_| -> Result<(), Error> { Ok(()) })?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("Host on {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr)?;
		// trigger panic
		strm.write(b"crash\r\n")?;
		rx.recv()?;
		sleep(Duration::from_millis(QA_SLEEP));

		Ok(())
	}

	#[test]
	fn test_invalid_write_handle() -> Result<(), Error> {
		let connection = Connection {
			handle: 0,
			id: 0,
			slab_offset: 0,
			last_slab: 0,
			first_slab: 0,
			write_state: lock_box!(WriteState {
				flags: 0,
				write_buffer: vec![]
			})?,
			wakeup: None,
			state: None,
			tx: None,
			ctype: ConnectionType::Connection,
			debug_info: DebugInfo::default(),
			origin_id: 0,
			write_final: false,
			disable_write_final: false,
		};
		assert!(WriteHandle::new(&connection, DebugInfo::default()).is_err());

		let connection = Connection {
			handle: 0,
			id: 0,
			slab_offset: 0,
			last_slab: 0,
			first_slab: 0,
			write_state: lock_box!(WriteState {
				flags: 0,
				write_buffer: vec![]
			})?,
			wakeup: Some(Wakeup::new()?),
			state: None,
			tx: None,
			ctype: ConnectionType::Connection,
			debug_info: DebugInfo::default(),
			origin_id: 0,
			write_final: false,
			disable_write_final: false,
		};
		assert!(WriteHandle::new(&connection, DebugInfo::default()).is_err());
		Ok(())
	}

	#[test]
	fn test_evh_stop_error() -> Result<(), Error> {
		let test_info = test_info!()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		{
			let mut evh = evh_oro!(
				Debug(false),
				EvhTimeout(u16::MAX),
				EvhThreads(5),
				EvhReadSlabSize(100)
			)?;
			evh.set_on_read(move |_connection, _ctx| -> Result<(), Error> { Ok(()) })?;
			evh.set_debug_info(DebugInfo {
				stop_error: lock_box!(true).unwrap(),
				..Default::default()
			})?;

			evh.start()?;

			let conn = EvhBuilder::build_server_connection(&addr, 100)?;
			evh.add_server_connection(conn)?;
		}

		// we should still be able to connect because stop failed.
		assert!(TcpStream::connect(addr).is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_get_events_error() -> Result<(), Error> {
		let config = EventHandlerConfig {
			debug: false,
			housekeeping_frequency_millis: 100,
			stats_update_frequency_millis: 100,
			threads: 1,
			timeout: 1_000,
			read_slab_count: 1,
			read_slab_size: 100,
			out_of_slabs_message: "".to_string(),
		};
		let debug_info = DebugInfo {
			get_events_error: lock_box!(true)?,
			..Default::default()
		};

		let read_slabs = slab_allocator!(
			SlabSize(config.read_slab_size),
			SlabCount(config.read_slab_count)
		)?;
		let user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};
		let user_context_arr = array!(1, &lock_box!(user_context)?)?;
		let state = array!(config.threads, &lock_box!(EventHandlerState::new()?)?)?;
		let w = Wakeup::new()?;
		let wakeups = array!(1, &w)?;

		let global_stats = GlobalStats {
			stats: EvhStats::new(),
			update_counter: 0,
			tx: None,
		};
		let stats = lock_box!(global_stats)?;

		let sample = EventHandlerContext::new(wakeups, 0, stats)?;
		let sample = lock_box!(sample)?;
		let ctx_arr = array!(1, &sample)?;
		let callbacks = EventHandlerCallbacks {
			on_read: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_accept: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_close: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_panic: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>, _| -> Result<(), Error> { Ok(()) },
			)),
			on_housekeeper: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>| -> Result<(), Error> { Ok(()) },
			)),
		};

		spawn(move || {
			let _ = EventHandlerImpl::execute_thread(
				config,
				callbacks,
				state,
				ctx_arr,
				user_context_arr,
				0,
				true,
				&debug_info,
			);
		});

		sleep(Duration::from_millis(QA_SLEEP));
		Ok(())
	}

	#[test]
	fn test_evh_functions() -> Result<(), Error> {
		let test_info = test_info!()?;
		let w = Wakeup::new()?;
		let wakeups = array!(1, &w)?;

		let global_stats = GlobalStats {
			stats: EvhStats::new(),
			update_counter: 0,
			tx: None,
		};
		let stats = lock_box!(global_stats)?;
		let mut ehc = EventHandlerContext::new(wakeups, 0, stats)?;
		let mut callbacks = EventHandlerCallbacks {
			on_read: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_accept: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_close: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_panic: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>, _| -> Result<(), Error> { Ok(()) },
			)),
			on_housekeeper: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>| -> Result<(), Error> { Ok(()) },
			)),
		};

		let mut v = VecDeque::new();
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let server = EvhBuilder::build_server_connection(&addr, 5)?;
		v.push_back(ConnectionVariant::ServerConnection(server));
		let client = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		v.push_back(ConnectionVariant::ClientConnection(client));
		let client2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		v.push_back(ConnectionVariant::Connection(client2));
		v.push_back(ConnectionVariant::Wakeup(Wakeup::new()?));
		assert!(EventHandlerImpl::close_handles(&mut ehc, &v, &mut callbacks).is_ok());

		let read_slabs = slab_allocator!(SlabSize(100), SlabCount(1))?;
		let mut user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};

		let port = pick_free_port()?;
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 5)?;
		ehc.id_hash
			.insert(0, ConnectionVariant::ServerConnection(conn));
		assert!(
			EventHandlerImpl::process_write_id(&mut ehc, 0, &mut callbacks, &mut user_context)
				.is_ok()
		);

		let mut conn = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		conn.wakeup = Some(Wakeup::new()?);
		conn.state = Some(lock_box!(EventHandlerState::new()?)?);
		conn.write_handle()?.trigger_on_read()?;
		conn.write_handle()?.close()?;
		ehc.id_hash.insert(0, ConnectionVariant::Connection(conn));
		assert!(
			EventHandlerImpl::process_write_id(&mut ehc, 0, &mut callbacks, &mut user_context)
				.is_ok()
		);

		let mut conn = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		conn.wakeup = Some(Wakeup::new()?);
		conn.state = Some(lock_box!(EventHandlerState::new()?)?);
		conn.write_handle()?.trigger_on_read()?;
		conn.write_handle()?.close()?;
		ehc.id_hash
			.insert(0, ConnectionVariant::ClientConnection(conn));
		assert!(
			EventHandlerImpl::process_write_id(&mut ehc, 0, &mut callbacks, &mut user_context)
				.is_ok()
		);

		ehc.id_hash
			.insert(0, ConnectionVariant::Wakeup(Wakeup::new()?));
		assert!(
			EventHandlerImpl::process_write_id(&mut ehc, 0, &mut callbacks, &mut user_context)
				.is_ok()
		);

		// try on a not found. just prints a warning
		assert!(
			EventHandlerImpl::process_write_id(&mut ehc, 1, &mut callbacks, &mut user_context)
				.is_ok()
		);

		let config = EventHandlerConfig {
			debug: false,
			housekeeping_frequency_millis: 100,
			stats_update_frequency_millis: 100,
			threads: 1,
			timeout: 1_000,
			read_slab_count: 1,
			read_slab_size: 100,
			out_of_slabs_message: "".to_string(),
		};
		let mut state = array!(config.threads, &lock_box!(EventHandlerState::new()?)?)?;
		let debug_info = DebugInfo::default();

		let port = pick_free_port()?;
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 5)?;
		ehc.handle_hash.insert(conn.handle(), conn.id());
		ehc.trigger_on_read_list.push(conn.handle());
		ehc.id_hash
			.insert(conn.id(), ConnectionVariant::ServerConnection(conn));

		let conn = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		ehc.handle_hash.insert(conn.handle(), conn.id());
		ehc.trigger_on_read_list.push(conn.handle());

		assert!(EventHandlerImpl::process_events(
			&config,
			&mut ehc,
			&mut callbacks,
			&mut state,
			&mut user_context,
			&debug_info,
		)
		.is_ok());

		let debug_info = DebugInfo {
			wakeup_read_err: lock_box!(true)?,
			..Default::default()
		};
		let wakeup = Wakeup::new()?;
		ehc.handle_hash.insert(0, 0);
		ehc.id_hash.insert(0, ConnectionVariant::Wakeup(wakeup));

		assert!(EventHandlerImpl::process_read_event(
			&config,
			&mut ehc,
			&mut callbacks,
			0,
			&mut state,
			&mut user_context,
			&debug_info,
		)
		.is_ok());

		ehc.handle_hash.insert(0, 100);

		assert!(EventHandlerImpl::process_read_event(
			&config,
			&mut ehc,
			&mut callbacks,
			0,
			&mut state,
			&mut user_context,
			&debug_info,
		)
		.is_ok());

		assert!(EventHandlerImpl::process_read_event(
			&config,
			&mut ehc,
			&mut callbacks,
			1230,
			&mut state,
			&mut user_context,
			&debug_info,
		)
		.is_ok());

		let port = pick_free_port()?;
		assert!(create_listener(&format!("[::1]:{}", port), 1, &DebugInfo::default()).is_ok());

		ehc.handle_hash.clear();
		ehc.id_hash.clear();

		ehc.handle_hash.insert(0, 0);
		ehc.id_hash
			.insert(0, ConnectionVariant::Wakeup(Wakeup::new()?));
		assert!(
			EventHandlerImpl::call_on_close(&mut user_context, 0, &mut callbacks, &mut ehc).is_ok()
		);

		ehc.handle_hash.clear();
		ehc.id_hash.clear();

		ehc.handle_hash.insert(0, 0);
		assert!(
			EventHandlerImpl::call_on_close(&mut user_context, 0, &mut callbacks, &mut ehc).is_ok()
		);

		ehc.handle_hash.clear();
		ehc.id_hash.clear();

		assert!(
			EventHandlerImpl::call_on_close(&mut user_context, 0, &mut callbacks, &mut ehc).is_ok()
		);

		callbacks.on_close = None;
		assert!(
			EventHandlerImpl::call_on_close(&mut user_context, 0, &mut callbacks, &mut ehc).is_ok()
		);

		ehc.handle_hash.insert(0, 0);
		ehc.id_hash
			.insert(0, ConnectionVariant::Wakeup(Wakeup::new()?));
		assert!(
			EventHandlerImpl::process_close(0, &mut ehc, &mut callbacks, &mut user_context).is_ok()
		);

		let port = pick_free_port()?;
		let _server = EvhBuilder::build_server_connection(&format!("127.0.0.1:{}", port), 1)?;
		let mut client = EvhBuilder::build_client_connection("127.0.0.1", port)?;

		// try to accept on the client (internal error printed out, but it returns ok)
		assert!(EventHandlerImpl::process_accept(
			&client,
			&mut vec![],
			&DebugInfo::default(),
			&mut callbacks,
		)
		.is_ok());

		assert!(EventHandlerImpl::process_write_event(
			&config,
			&mut ehc,
			&mut callbacks,
			0,
			&mut user_context
		)
		.is_ok());

		ehc.handle_hash.clear();
		ehc.id_hash.clear();

		ehc.handle_hash.insert(0, 0);

		assert!(EventHandlerImpl::process_write_event(
			&config,
			&mut ehc,
			&mut callbacks,
			0,
			&mut user_context
		)
		.is_ok());

		ehc.handle_hash.clear();
		ehc.id_hash.clear();

		ehc.handle_hash.insert(0, 0);
		ehc.id_hash
			.insert(0, ConnectionVariant::Wakeup(Wakeup::new()?));

		assert!(EventHandlerImpl::process_write_event(
			&config,
			&mut ehc,
			&mut callbacks,
			0,
			&mut user_context
		)
		.is_ok());

		client.wakeup = Some(Wakeup::new()?);
		client.state = Some(lock_box!(EventHandlerState::new()?)?);
		client.debug_info = DebugInfo {
			write_err2: lock_box!(true)?,
			..Default::default()
		};
		assert!(EventHandlerImpl::write_loop(&mut client, &mut callbacks).is_ok());

		Ok(())
	}

	#[test]
	fn test_evh_execute_thread_other_situations() -> Result<(), Error> {
		let config = EventHandlerConfig {
			debug: false,
			housekeeping_frequency_millis: 100,
			stats_update_frequency_millis: 100,
			threads: 1,
			timeout: 1_000,
			read_slab_count: 1,
			read_slab_size: 100,
			out_of_slabs_message: "".to_string(),
		};
		let debug_info = DebugInfo {
			internal_panic: lock_box!(true)?,
			..Default::default()
		};

		let read_slabs = slab_allocator!(
			SlabSize(config.read_slab_size),
			SlabCount(config.read_slab_count)
		)?;
		let user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};
		let user_context_arr = array!(1, &lock_box!(user_context)?)?;
		let state = array!(config.threads, &lock_box!(EventHandlerState::new()?)?)?;
		let w = Wakeup::new()?;
		let wakeups = array!(1, &w)?;

		let global_stats = GlobalStats {
			stats: EvhStats::new(),
			update_counter: 0,
			tx: None,
		};
		let stats = lock_box!(global_stats)?;

		let sample = EventHandlerContext::new(wakeups, 0, stats)?;
		let sample = lock_box!(sample)?;
		let ctx_arr = array!(1, &sample)?;
		let callbacks = EventHandlerCallbacks {
			on_read: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_accept: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_close: Some(Box::pin(
				move |_: &mut Connection, _: &mut Box<dyn UserContext + '_>| -> Result<(), Error> {
					Ok(())
				},
			)),
			on_panic: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>, _| -> Result<(), Error> { Ok(()) },
			)),
			on_housekeeper: Some(Box::pin(
				move |_: &mut Box<dyn UserContext + '_>| -> Result<(), Error> { Ok(()) },
			)),
		};

		spawn(move || {
			let _ = EventHandlerImpl::execute_thread(
				config,
				callbacks,
				state,
				ctx_arr,
				user_context_arr,
				0,
				true,
				&debug_info,
			);
		});

		sleep(Duration::from_millis(QA_SLEEP));
		Ok(())
	}

	#[test]
	fn test_evh_out_of_slabs() -> Result<(), Error> {
		let test_info = test_info!()?;

		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(25),
			EvhReadSlabCount(1)
		)?;

		let mut counter = lock_box!(0)?;
		let counter_clone = counter.clone();

		evh.set_on_read(move |conn, _| -> Result<(), Error> {
			info!("on read")?;
			let mut counter = counter.wlock()?;
			let guard = counter.guard()?;
			**guard += 1;

			if **guard == 2 {
				conn.write_handle()?.write(b"response")?;
			}

			Ok(())
		})?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);

		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		// write over 25 bytes
		strm.write(b"012345678901234567890123456789")?;

		let mut buf = [0u8; 100];

		// the stream gets closed by the server because there's no more slabs
		assert!(strm.read(&mut buf).is_err());

		// we should have only gotten a single request (second one causes the allocation
		// error)
		assert_eq!(rlock!(counter_clone), 1);

		// now that the connection is closed, we can continue processing because the slabs
		// is freed up.

		// open two connections
		let mut strm1 = TcpStream::connect(addr.clone())?;
		let mut strm2 = TcpStream::connect(addr)?;

		// write less than the single slab
		strm1.write(b"test")?;
		strm2.write(b"test")?;

		// stream 2 gets closed, but 1 is ok
		assert!(strm2.read(&mut buf).is_err());
		assert_eq!(strm1.read(&mut buf)?, 8);
		assert_eq!(&buf[0..8], b"response");
		assert_eq!(rlock!(counter_clone), 2);

		Ok(())
	}

	#[test]
	fn test_evh_no_clear() -> Result<(), Error> {
		let test_info = test_info!()?;

		let mut evh = evh_oro!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(25),
			EvhReadSlabCount(10)
		)?;

		evh.set_on_read(move |conn, _| -> Result<(), Error> {
			info!("on read")?;
			conn.write_handle()?.write(b"response")?;

			Ok(())
		})?;

		evh.start()?;

		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);

		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;

		let mut strm = TcpStream::connect(addr.clone())?;
		// write over 25 bytes
		strm.write(b"0123456789012345678")?;

		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;

		assert_eq!(len, 8);
		assert_eq!(&buf[0..8], b"response");

		strm.write(b"hi")?;
		let mut buf = [0u8; 100];
		let len = strm.read(&mut buf)?;

		assert_eq!(len, 8);
		assert_eq!(&buf[0..8], b"response");

		sleep(Duration::from_millis(QA_SLEEP));

		Ok(())
	}

	#[test]
	fn test_evh_controller_stop() -> Result<(), Error> {
		let mut client;
		let mut controller;
		let mut buf = [0u8; 100];
		{
			let test_info = test_info!()?;
			let mut evh = evh_oro!(
				Debug(false),
				EvhTimeout(100),
				EvhStatsUpdateMillis(try_into!(QA_SLEEP)?),
				EvhThreads(1),
				EvhReadSlabSize(100)
			)?;

			evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
				info!("onRead")?;
				let mut data: Vec<u8> = vec![];

				loop {
					let next_chunk = ctx.next_chunk(connection)?;
					cbreak!(next_chunk.is_none());
					let next_chunk = next_chunk.unwrap();
					data.extend(next_chunk.data());
				}

				let dstring = from_utf8(&data)?;
				info!("data[{}]='{}'", connection.id(), dstring,)?;

				assert_eq!(dstring, "hi");
				let mut wh = connection.write_handle()?;
				wh.write(b"test")?;
				ctx.clear_all(connection)?;
				Ok(())
			})?;

			evh.start()?;
			controller = evh.controller()?;
			let port = test_info.port();
			let addr = format!("127.0.0.1:{}", port);
			let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
			evh.add_server_connection(conn)?;

			client = TcpStream::connect(addr)?;
			client.write(b"hi")?;

			let len = client.read(&mut buf)?;
			assert_eq!(&buf[0..len], b"test");
		}

		client.write(b"hi")?;
		let len = client.read(&mut buf)?;
		assert_eq!(&buf[0..len], b"test");

		let stats = controller.wait_for_stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats.accepts, 1);
		assert_eq!(stats.reads, 2);
		assert_eq!(stats.bytes_read, 4);

		controller.stop()?;

		// now it's closed
		let len = client.read(&mut buf)?;
		assert_eq!(len, 0);

		Ok(())
	}

	#[test]
	fn test_evh_resources() -> Result<(), Error> {
		// this test doesn't currently do assertions, but it can be used to monitor resources
		// like file descriptors. Change `target` to a higher value and increase sleep at the
		// end to be able to look at usage
		let test_info = test_info!()?;
		let mut loop_count = 0;
		let target = 1;
		loop {
			info!("create evh {}", loop_count)?;
			let mut evh = evh_oro!(
				Debug(false),
				EvhTimeout(u16::MAX),
				EvhThreads(5),
				EvhReadSlabSize(100)
			)?;

			let (tx, rx) = test_info.sync_channel();
			evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
				let mut data: Vec<u8> = vec![];

				loop {
					let next_chunk = ctx.next_chunk(connection)?;
					cbreak!(next_chunk.is_none());
					let next_chunk = next_chunk.unwrap();
					data.extend(next_chunk.data());
				}

				let _dstring = from_utf8(&data)?;
				tx.send(())?;

				ctx.clear_all(connection)?;
				Ok(())
			})?;
			evh.start()?;
			let port = test_info.port();
			let addr = format!("127.0.0.1:{}", port);
			let conn = EvhBuilder::build_server_connection(&addr, 100)?;
			evh.add_server_connection(conn)?;
			let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
			let mut wh = evh.add_client_connection(conn2)?;
			wh.write(b"01234567890123456789")?;
			rx.recv()?;

			loop_count += 1;

			if loop_count == target {
				break;
			}
			//sleep(Duration::from_millis(100));
		}

		info!("sleep")?;
		//sleep(Duration::from_millis(120_000));
		Ok(())
	}

	#[test]
	fn test_evh_origin_id() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh_oro!(
			Debug(true),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
		)?;

		let mut recv_msg = lock_box!(false)?;
		let mut origin_assertion_ok = lock_box!(false)?;
		let origin_assertion_ok_clone = origin_assertion_ok.clone();
		let recv_msg_clone = recv_msg.clone();
		let (tx, rx) = test_info.sync_channel();
		let (tx2, rx2) = test_info.sync_channel();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;
			info!(
				"data[{}]='{}', origin_id={}",
				connection.id(),
				dstring,
				connection.origin_id()
			)?;

			assert_eq!(dstring, "hi");

			if !rlock!(recv_msg) {
				// origin id should be different from id (accepted connection)
				assert_ne!(connection.id(), connection.origin_id());
				// send back once
				let mut wh = connection.write_handle()?;
				wh.write(b"hi")?;
			} else {
				// this is the client, origin id should be equal to id
				assert_eq!(connection.id(), connection.origin_id());
				wlock!(origin_assertion_ok) = true;
				tx2.send(())?;
			}

			wlock!(recv_msg) = true;
			tx.send(())?;
			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
		evh.add_server_connection(conn)?;

		let conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(b"hi")?;

		rx.recv()?;
		assert!(rlock!(recv_msg_clone));
		rx2.recv()?;
		assert!(rlock!(origin_assertion_ok_clone));

		Ok(())
	}

	#[test]
	fn test_evh_pending_write_on_close() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(250),
			EvhReadSlabCount(100_000),
			EvhStatsUpdateMillis(5000),
			EvhOutOfSlabsMessage("12345".to_string())
		)?;

		let debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info)?;

		let mut client_id = lock_box!(0)?;

		let mut client_recv = lock_box!(false)?;
		let mut server_recv = lock_box!(false)?;
		let client_recv_clone = client_recv.clone();
		let server_recv_clone = server_recv.clone();
		let (tx, rx) = test_info.sync_channel();
		let client_id_clone = client_id.clone();

		let message = "0123456789";
		let ret_message = [b'p'; 100_000];
		let ret_message = from_utf8(&ret_message).unwrap().to_string();

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;

			if dstring != message && rlock!(client_id_clone) != connection.id() {
				return Ok(());
			} else if dstring != ret_message && rlock!(client_id_clone) == connection.id() {
				return Ok(());
			} else {
				info!("equal")?;
			}

			let mut wh = connection.write_handle()?;

			// respond
			if rlock!(client_id_clone) != connection.id() {
				info!("wh write and close")?;
				wh.write_and_close(ret_message.as_bytes())?;
				wlock!(server_recv) = true;
			} else {
				wlock!(client_recv) = true;
			}

			if rlock!(client_recv) && rlock!(server_recv) {
				tx.send(())?;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			info!("onClose")?;
			Ok(())
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;
		let mut conn2 = EvhBuilder::build_client_connection("127.0.0.1", port)?;
		conn2.debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		wlock!(client_id) = conn2.id();

		let mut wh = evh.add_client_connection(conn2)?;
		wh.write(message.as_bytes())?;

		rx.recv()?;
		assert!(rlock!(client_recv_clone));
		assert!(rlock!(server_recv_clone));

		Ok(())
	}

	#[test]
	fn test_evh_out_of_slabs_message() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(25),
			EvhReadSlabCount(1),
			EvhStatsUpdateMillis(5000),
			EvhOutOfSlabsMessage("outofslabs".to_string())
		)?;

		let debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info)?;

		let client_id = lock_box!(0)?;

		let mut client_recv = lock_box!(false)?;
		let mut server_recv = lock_box!(false)?;
		let client_id_clone = client_id.clone();

		let message = "012345678901234567890123456789";
		let ret_message = "0123456789";

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;

			if dstring != message && rlock!(client_id_clone) != connection.id() {
				info!("not equal server = {}", dstring)?;
				return Ok(());
			} else if dstring != "outofslabs" && rlock!(client_id_clone) == connection.id() {
				info!("not equal client = {}", dstring)?;
				return Ok(());
			} else {
				info!(
					"equal {}",
					if rlock!(client_id_clone) == connection.id() {
						"client"
					} else {
						"server"
					}
				)?;
			}

			let mut wh = connection.write_handle()?;

			// respond
			if rlock!(client_id_clone) != connection.id() {
				info!("wh write and close")?;
				wh.write_and_close(ret_message.as_bytes())?;
				wlock!(server_recv) = true;
			} else {
				wlock!(client_recv) = true;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		evh.set_on_close(move |connection, _ctx| -> Result<(), Error> {
			info!("onClose: {}", connection.id())?;
			Ok(())
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;
		let mut strm = TcpStream::connect(addr)?;

		strm.write(message.as_bytes())?;
		let mut buf = [0u8; 100];
		info!("about to read")?;
		let len = strm.read(&mut buf)?;
		info!("read complete")?;

		assert_eq!(&buf[0..len], b"outofslabs");
		Ok(())
	}

	#[test]
	fn test_evh_disabled_out_of_slabs_message() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(100),
			EvhThreads(1),
			EvhReadSlabSize(25),
			EvhReadSlabCount(1),
			EvhStatsUpdateMillis(5000),
			EvhOutOfSlabsMessage("outofslabs".to_string())
		)?;

		let debug_info = DebugInfo {
			pending: lock_box!(true)?,
			..Default::default()
		};
		evh.set_debug_info(debug_info)?;

		let client_id = lock_box!(0)?;

		let mut client_recv = lock_box!(false)?;
		let mut server_recv = lock_box!(false)?;
		let client_id_clone = client_id.clone();

		let message = "012345678901234567890123456789";
		let ret_message = "0123456789";

		evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
			info!("onRead")?;
			let mut data: Vec<u8> = vec![];

			loop {
				let next_chunk = ctx.next_chunk(connection)?;
				cbreak!(next_chunk.is_none());
				let next_chunk = next_chunk.unwrap();
				data.extend(next_chunk.data());
			}

			let dstring = from_utf8(&data)?;

			if dstring != message && rlock!(client_id_clone) != connection.id() {
				info!("not equal server = {}", dstring)?;
				return Ok(());
			} else if dstring != "outofslabs" && rlock!(client_id_clone) == connection.id() {
				info!("not equal client = {}", dstring)?;
				return Ok(());
			} else {
				info!(
					"equal {}",
					if rlock!(client_id_clone) == connection.id() {
						"client"
					} else {
						"server"
					}
				)?;
			}

			let mut wh = connection.write_handle()?;

			// respond
			if rlock!(client_id_clone) != connection.id() {
				info!("wh write and close")?;
				wh.write_and_close(ret_message.as_bytes())?;
				wlock!(server_recv) = true;
			} else {
				wlock!(client_recv) = true;
			}

			ctx.clear_all(connection)?;
			Ok(())
		})?;

		evh.set_on_accept(move |connection, _ctx| -> Result<(), Error> {
			connection.disable_write_final();
			info!(
				"onAccept: handle={},id={}",
				connection.handle(),
				connection.id()
			)?;
			Ok(())
		})?;

		evh.set_on_close(move |connection, _ctx| -> Result<(), Error> {
			info!("onClose: {}", connection.id())?;
			Ok(())
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;
		let port = test_info.port();
		let addr = format!("127.0.0.1:{}", port);
		info!("connecting to addr {}", addr)?;
		let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;

		info!("adding connection now")?;
		evh.add_server_connection(conn)?;
		let mut strm = TcpStream::connect(addr)?;

		strm.write(message.as_bytes())?;
		let mut buf = [0u8; 100];
		info!("about to read")?;
		assert!(strm.read(&mut buf).is_err());

		Ok(())
	}
}
