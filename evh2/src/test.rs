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
	use crate::{evh, evh_oro, EvhBuilder};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
	use std::io::{Read, Write};
	use std::net::TcpStream;
	use std::str::from_utf8;
	use std::thread;

	info!();

	#[test]
	#[cfg(target_os = "linux")]
	fn test_evh_basic() -> Result<(), Error> {
		let test_info = test_info!()?;
		let mut evh = evh!(
			Debug(false),
			EvhTimeout(u16::MAX),
			EvhThreads(1),
			EvhReadSlabSize(100)
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
	#[cfg(target_os = "linux")]
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
	#[cfg(target_os = "linux")]
	fn test_evh_stop() -> Result<(), Error> {
		let test_info = test_info!()?;
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
			let port = test_info.port();
			let addr = format!("127.0.0.1:{}", port);
			let conn = EvhBuilder::build_server_connection(&addr, 10_000)?;
			info!("conn.handle = {}", conn.handle())?;
			evh.add_server_connection(conn)?;

			strm = TcpStream::connect(addr)?;
			strm.write(b"test")?;
			let mut buf = [0u8; 100];
			let res = strm.read(&mut buf);
			assert!(res.is_ok());
		}

		let mut buf = [0u8; 100];
		let res = strm.read(&mut buf)?;
		assert!(res == 0); // closed
		Ok(())
	}

	#[test]
	#[cfg(target_os = "linux")]
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
	#[cfg(target_os = "linux")]
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
	#[cfg(target_os = "linux")]
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
	#[cfg(target_os = "linux")]
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
}
