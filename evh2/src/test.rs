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
	use crate as bmw_evh2;
	use crate::types::{ConnectionType, DebugInfo, Wakeup, WriteHandle, WriteState};
	use crate::{evh, evh_oro, Connection, EvhBuilder};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
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
		let (x, y) = wakeup_impl()?;
		write_impl(y, b"test")?;
		let mut buf = [0u8; 100];
		let len = read_impl(x, &mut buf)?;
		assert_eq!(len, Some(4));
		Ok(())
	}

	#[test]
	fn test_evh_basic() -> Result<(), Error> {
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
				let mut buf = [0u8; 1024];
				let mut data: Vec<u8> = vec![];
				loop {
					let len = ctx.clone_next_chunk(connection, &mut buf)?;

					if len == 0 {
						break;
					}

					data.extend(&buf[0..len]);
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			assert_ne!(ctx.cur_slab_id(), usize::MAX);

			let mut buf = [0u8; 10];
			assert!(ctx.clone_next_chunk(connection, &mut buf).is_err());
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
			let mut buf = [0u8; 1024];
			let mut data: Vec<u8> = vec![];
			info!("chunk starting")?;
			loop {
				let len = ctx.clone_next_chunk(connection, &mut buf)?;
				info!("len={}", len)?;

				if len == 0 {
					break;
				}

				data.extend(&buf[0..len]);
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
			assert_ne!(ctx.cur_slab_id(), usize::MAX);

			let mut buf = [0u8; 10];
			assert!(ctx.clone_next_chunk(connection, &mut buf).is_err());
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
				let mut buf = [0u8; 1024];
				let mut data: Vec<u8> = vec![];
				loop {
					let len = ctx.clone_next_chunk(connection, &mut buf)?;

					if len == 0 {
						break;
					}

					data.extend(&buf[0..len]);
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
}
