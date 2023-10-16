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
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
	) -> Result<bool, Error> {
		let mut data = [0u8; 512];
		info!("try cache {}", fpath);
		let found = self.hashtable.raw_read(fpath, 0, &mut data)?;
		if found {
			let len = slice_to_usize(&data[0..8])?;
			info!(
				"cache found len = {}, data = {:?}, found={}",
				len,
				&data[0..8],
				found
			);

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

			conn_data.write_handle().write(&res.as_bytes()[..])?;
			let mut rem = len;
			let mut i = 0;
			loop {
				let mut buf = vec![0u8; 512];
				let found = self.hashtable.raw_read(fpath, 8 + i * 512, &mut data)?;
				let wlen = if rem > 512 { 512 } else { rem };
				conn_data.write_handle().write(&data[0..wlen])?;

				rem = rem.saturating_sub(wlen);
				if rem == 0 {
					break;
				}
				i += 1;
			}
		}
		Ok(found)
	}

	fn write_len(&mut self, path: &String, len: usize) -> Result<(), Error> {
		info!("write_len {} = {}", path, len);
		let mut data = [0u8; 512];
		usize_to_slice(len, &mut data[0..8])?;
		info!("writing data = {:?}", &data[0..8]);
		self.hashtable.raw_write(path, 0, &data)?;
		Ok(())
	}

	fn write_block(
		&mut self,
		path: &String,
		block_num: usize,
		data: &[u8; 512],
	) -> Result<(), Error> {
		info!("write block num = {}, path = {}", block_num, path);
		self.hashtable.raw_write(path, 8 + block_num * 512, data);
		Ok(())
	}

	fn bring_to_front(&mut self, path: &String) -> Result<(), Error> {
		self.hashtable.bring_to_front(path)
	}
}

#[cfg(test)]
mod test {
	use bmw_err::*;
	use bmw_log::*;

	debug!();

	#[test]
	fn test_cache_basic() -> Result<(), Error> {
		Ok(())
	}
}
