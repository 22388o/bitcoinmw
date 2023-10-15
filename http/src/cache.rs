// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::types::HttpCacheImpl;
use crate::HttpCache;
use bmw_deps::chrono::Utc;
use bmw_err::*;
use bmw_evh::ConnData;
use bmw_evh::ConnectionData;
use bmw_log::*;
use bmw_util::*;
use std::fs::File;
use std::io::{BufReader, Read};

info!();

impl HttpCacheImpl {
	pub(crate) fn new() -> Result<Box<dyn HttpCache + Send + Sync>, Error> {
		let hashtable = hashtable_sync_box!()?;
		Ok(Box::new(HttpCacheImpl { hashtable }))
	}
}

impl HttpCache for HttpCacheImpl {
	fn stream_file(
		&self,
		fpath: &String,
		len: u64,
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
	) -> Result<bool, Error> {
		/*
						let file = File::open(fpath)?;
						let mut buf_reader = BufReader::new(file);

						let dt = Utc::now();
						let res = dt
								.format(
										&format!(
												"HTTP/1.1 {} {}\r\n\
		Date: %a, %d %h %C%y %H:%M:%S GMT\r\n\
		Content-Length: ",
												code, message
										)
										.to_string(),
								)
								.to_string();

						let res = format!("{}{}\r\n\r\n", res, len);

						debug!("writing {}", res)?;
		*/

		/*
						conn_data.write_handle().write(&res.as_bytes()[..])?;

						loop {
								let mut buf = vec![0u8; 100];
								let len = buf_reader.read(&mut buf)?;
								conn_data.write_handle().write(&buf[0..len])?;
								if len == 0 {
										break;
								}
						}
		*/
		Ok(false)
	}

	fn write_block(&mut self, path: &String, offset: u64, data: &[u8]) -> Result<(), Error> {
		todo!()
	}

	fn bring_to_front(&mut self, path: &String) -> Result<(), Error> {
		todo!()
	}
}
