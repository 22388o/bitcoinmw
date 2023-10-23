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

use crate::constants::*;
use crate::types::{HttpCacheImpl, HttpContext, HttpServerImpl};
use crate::{HttpCache, HttpConfig, HttpHeaders};
use bmw_err::*;
use bmw_evh::ConnData;
use bmw_evh::ConnectionData;
use bmw_log::*;
use bmw_util::*;

info!();

impl HttpCacheImpl {
	pub(crate) fn new(config: &HttpConfig) -> Result<Box<dyn HttpCache + Send + Sync>, Error> {
		let hashtable = hashtable_sync_box!(
			SlabSize(CACHE_SLAB_SIZE),
			SlabCount(config.cache_slab_count)
		)?;
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
		ctx: &HttpContext,
		config: &HttpConfig,
		headers: &HttpHeaders,
	) -> Result<bool, Error> {
		let mut data = [0u8; CACHE_BUFFER_SIZE];
		debug!("try cache {}", fpath)?;
		let found = self.hashtable.raw_read(fpath, 0, &mut data)?;
		debug!("raw read complete")?;
		if found {
			let len = slice_to_usize(&data[0..8])?;
			debug!(
				"cache found len = {}, data = {:?}, found={}",
				len,
				&data[0..8],
				found
			)?;

			let (keep_alive, res) = HttpServerImpl::build_response_headers(
				config,
				code,
				message,
				len,
				None,
				match ctx.mime_map.get(&headers.extension()?) {
					Some(mime_type) => Some(mime_type.clone()),
					None => Some("text/html".to_string()),
				},
				ctx,
				headers,
			)?;

			debug!("writing {}", res)?;

			let mut write_handle = conn_data.write_handle();
			write_handle.write(&res.as_bytes()[..])?;
			let mut rem = len;
			let mut i = 0;
			loop {
				self.hashtable
					.raw_read(fpath, 8 + i * CACHE_BUFFER_SIZE, &mut data)?;
				let wlen = if rem > CACHE_BUFFER_SIZE {
					CACHE_BUFFER_SIZE
				} else {
					rem
				};
				debug!("read wlen={},rem={},data={:?}", wlen, rem, data)?;
				write_handle.write(&data[0..wlen])?;

				rem = rem.saturating_sub(wlen);
				if rem == 0 {
					break;
				}
				i += 1;
			}

			if !keep_alive {
				write_handle.close()?;
			}
		}
		Ok(found)
	}

	fn write_len(&mut self, path: &String, len: usize) -> Result<bool, Error> {
		let mut free_count;
		let slab_count;
		(free_count, slab_count) = {
			let slabs = self.hashtable.slabs()?.unwrap();
			let slabs = slabs.borrow();
			(slabs.free_count()?, slabs.slab_count()?)
		};
		let bytes_needed = len + path.len() + CACHE_OVERHEAD_BYTES;
		let blocks_needed = 1 + (bytes_needed / CACHE_BYTES_PER_SLAB);
		debug!("free_count={},blocks_needed={}", free_count, blocks_needed)?;

		if blocks_needed > slab_count {
			Ok(false)
		} else {
			loop {
				if free_count >= blocks_needed {
					break;
				}

				debug!("removing oldest")?;
				self.hashtable.remove_oldest()?;

				free_count = self.hashtable.slabs()?.unwrap().borrow().free_count()?;
				debug!(
					"loop free_count={},blocks_needed={}",
					free_count, blocks_needed
				)?;
			}
			debug!("write_len {} = {}", path, len)?;
			let mut data = [0u8; CACHE_BUFFER_SIZE];
			usize_to_slice(len, &mut data[0..8])?;
			debug!("write_len {:?}", &data[0..8])?;
			self.hashtable.raw_write(path, 0, &data)?;
			debug!("====================================write_len complete")?;
			Ok(true)
		}
	}

	fn write_block(
		&mut self,
		path: &String,
		block_num: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
	) -> Result<(), Error> {
		debug!(
			"write block num = {}, path = {}, data={:?}",
			block_num, path, data
		)?;
		let ret = self
			.hashtable
			.raw_write(path, 8 + block_num * CACHE_BUFFER_SIZE, data);
		debug!(
			"=====================================write block complete: {:?}",
			ret
		)?;
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
