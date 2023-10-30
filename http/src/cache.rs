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
use crate::types::{CacheStreamResult, HttpCache, HttpCacheImpl, HttpContext, HttpServerImpl};
use crate::{HttpConfig, HttpHeaders, HttpRequestType};
use bmw_deps::chrono::{DateTime, TimeZone, Utc};
use bmw_err::*;
use bmw_evh::ConnData;
use bmw_evh::ConnectionData;
use bmw_log::*;
use bmw_util::*;
use std::fs::metadata;
use std::time::UNIX_EPOCH;

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
		mut code: u16,
		mut message: &str,
		ctx: &HttpContext,
		config: &HttpConfig,
		headers: &HttpHeaders,
	) -> Result<CacheStreamResult, Error> {
		let mut data = [0u8; CACHE_BUFFER_SIZE];
		debug!("try cache {}", fpath)?;
		let found = self.hashtable.raw_read(fpath, 0, &mut data)?;
		debug!("raw read complete")?;
		let text_plain = TEXT_PLAIN.to_string();
		let mut ret = CacheStreamResult::Miss;
		if found {
			ret = CacheStreamResult::Hit;
			let len = slice_to_usize(&data[0..8])?;
			let last_modified = slice_to_u64(&data[8..16])?;
			let mime_code = slice_to_u32(&data[16..20])?;
			let last_check = slice_to_u64(&data[20..28])?;
			let now_u64: u64 = try_into!(ctx.now)?;
			let diff = now_u64.saturating_sub(last_check);
			if diff > config.restat_file_frequency_in_millis {
				match metadata(&fpath) {
					Ok(md) => {
						let last_modified_metadata: u64 =
							try_into!(md.modified()?.duration_since(UNIX_EPOCH)?.as_millis())?;

						if last_modified_metadata != last_modified {
							// file has been updated or has changed
							// in some way. Return false so that
							// the file can be re-read.
							return Ok(CacheStreamResult::Modified);
						} else {
							ret = CacheStreamResult::NotModified;
						}
					}
					Err(_) => {
						// presumably something's differt on the file
						// system. Just say it's modified, it will be
						// re-read and any error reported as the file is
						// streamed.
						return Ok(CacheStreamResult::Modified);
					}
				}
			}
			let mime_type = ctx.mime_lookup.get(&mime_code).unwrap_or(&text_plain);
			debug!(
				"cache found len = {}, data = {:?}, found={}",
				len,
				&data[0..8],
				found
			)?;

			let range_start = headers.range_start()?;
			let range_end = headers.range_end()?;
			let range_end_content = if range_end > len { len } else { range_end };
			let content_len = range_end_content.saturating_sub(range_start);

			let etag = format!("{}-{:01x}", last_modified, content_len);
			let modified_since = DateTime::parse_from_rfc2822(headers.if_modified_since()?)
				.unwrap_or(Utc.with_ymd_and_hms(1970, 1, 1, 0, 1, 1).unwrap().into());
			let modified_since = modified_since.timestamp_millis();

			if &etag == headers.if_none_match()? || last_modified < try_into!(modified_since)? {
				code = 304;
				message = "Not Modified";
			}

			let (keep_alive, res) = HttpServerImpl::build_response_headers(
				config,
				match headers.has_range()? {
					true => 206,
					false => code,
				},
				match headers.has_range()? {
					true => "Partial Content",
					false => message,
				},
				content_len,
				len,
				None,
				Some(mime_type.clone()),
				ctx,
				headers,
				false,
				try_into!(last_modified)?,
				etag,
				false,
			)?;

			debug!("writing {}", res)?;

			let mut write_handle = conn_data.write_handle();
			write_handle.write(&res.as_bytes()[..])?;

			if code != 304 {
				let mut rem = len;
				let mut i = 0;
				let mut len_sum = 0;
				let http_request_type = headers.http_request_type()?;
				loop {
					self.hashtable
						.raw_read(fpath, 28 + i * CACHE_BUFFER_SIZE, &mut data)?;
					let blen = if rem > CACHE_BUFFER_SIZE {
						CACHE_BUFFER_SIZE
					} else {
						rem
					};
					debug!("read blen={},rem={},data={:?}", blen, rem, data)?;

					if http_request_type != &HttpRequestType::HEAD {
						HttpServerImpl::range_write(
							range_start,
							range_end,
							&data.to_vec(),
							len_sum,
							blen,
							&mut write_handle,
							headers.accept_gzip()?,
							headers.has_range()?,
						)?;
					}
					len_sum += blen;

					rem = rem.saturating_sub(blen);
					if rem == 0 {
						break;
					}
					i += 1;
				}

				if !headers.has_range()? {
					debug!("write term bytes")?;
					// write termination bytes
					let term = ['0' as u8, '\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];
					write_handle.write(&term)?;
				}
			}

			if !keep_alive {
				write_handle.close()?;
			}
		}
		Ok(ret)
	}

	fn remove(&mut self, fpath: &String) -> Result<(), Error> {
		self.hashtable.remove(fpath)?;
		Ok(())
	}

	fn update_last_checked_if_needed(
		&mut self,
		fpath: &String,
		ctx: &HttpContext,
		config: &HttpConfig,
	) -> Result<(), Error> {
		let mut data = [0u8; CACHE_BUFFER_SIZE];
		let found = self.hashtable.raw_read(fpath, 0, &mut data)?;
		if found {
			let last_check = slice_to_u64(&data[20..28])?;
			let now_u64: u64 = try_into!(ctx.now)?;
			let diff = now_u64.saturating_sub(last_check);
			if diff > config.restat_file_frequency_in_millis {
				u64_to_slice(now_u64, &mut data[20..28])?;
				self.hashtable.raw_write(fpath, 0, &data)?;
			}
		}

		Ok(())
	}

	fn write_metadata(
		&mut self,
		path: &String,
		len: usize,
		last_modified: u64,
		mime_type: u32,
		now: u64,
	) -> Result<bool, Error> {
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
			u64_to_slice(last_modified, &mut data[8..16])?;
			u32_to_slice(mime_type, &mut data[16..20])?;
			u64_to_slice(now, &mut data[20..28])?;
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
			.raw_write(path, 28 + block_num * CACHE_BUFFER_SIZE, data);
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
