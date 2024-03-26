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
	use std::str::from_utf8;

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
			EvhTimeout(1_000),
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
}
