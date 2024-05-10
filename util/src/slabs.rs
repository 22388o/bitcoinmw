// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::misc::{set_max, slice_to_u64, slice_to_usize, usize_to_slice};
use bmw_core::*;
use bmw_log::*;
use std::cmp::Ordering;
use SlabAllocatorErrors::*;

info!();

#[ErrorKind]
enum SlabAllocatorErrors {
	Configuration,
	ArrayIndexOutOfBounds,
	TryReserveError,
	IllegalArgument,
	OutOfSlabs,
}

#[class {
    clone slab_data;
    pub slab_data_sync_box;
    var data: Vec<u8>;

    [slab_data]
    fn data(&self, offset: usize, len: usize) -> Result<&[u8], Error>;

    [slab_data]
    fn update(&mut self, v: &[u8], offset: usize) -> Result<(), Error>;

    [slab_data]
    fn resize(&mut self, reserved: usize) -> Result<(), Error>;

}]
impl SlabDataClass {
	fn builder(constants: &SlabDataClassConst) -> Result<Self, Error> {
		let data = vec![];
		Ok(Self { data })
	}
}

impl SlabDataClass {
	fn data(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
		let data = self.vars().get_data();
		let dlen = data.len();
		let needed = offset + len;
		if needed > dlen {
			err!(
				ArrayIndexOutOfBounds,
				"needed={},available={}",
				needed,
				dlen
			)
		} else {
			Ok(&data[offset..offset + len])
		}
	}

	fn update(&mut self, v: &[u8], offset: usize) -> Result<(), Error> {
		let vlen = v.len();
		let data = self.vars_mut().get_mut_data();
		let dlen = data.len();
		let needed = vlen + offset;
		if needed > dlen {
			err!(
				ArrayIndexOutOfBounds,
				"needed={},available={}",
				needed,
				dlen
			)
		} else {
			data[offset..offset + vlen].clone_from_slice(v);
			Ok(())
		}
	}

	fn resize(&mut self, reserved: usize) -> Result<(), Error> {
		let data = self.vars_mut().get_mut_data();
		map_err!(data.try_reserve_exact(reserved), TryReserveError)?;
		data.truncate(reserved);
		data.resize(reserved, 0u8);
		Ok(())
	}
}

#[derive(Configurable, Clone, Debug)]
pub struct SlabAllocatorConfig {
	slab_size: usize,
	slab_count: usize,
}

impl Default for SlabAllocatorConfig {
	fn default() -> Self {
		Self {
			slab_size: 512,
			slab_count: usize::MAX,
		}
	}
}

#[macro_export]
macro_rules! slab_config {
        ($($params:tt)*) => {{
            configure_box!(SlabAllocatorConfig, SlabAllocatorConfigOptions, vec![$($params)*])
        }};
}

#[derive(Clone, Debug)]
struct SlabStats {
	cur_slabs: usize,
	cur_capacity: usize,
	slabs_per_resize: usize,
	max_slabs: usize,
}

impl SlabStats {
	fn new(slabs_per_resize: usize, max_slabs: usize) -> Self {
		Self {
			cur_capacity: slabs_per_resize,
			cur_slabs: 0,
			slabs_per_resize,
			max_slabs,
		}
	}
}

#[derive(Debug, Eq, PartialEq)]
struct SlabDataParams {
	slab_size: usize,
	slab_count: usize,
	ptr_size: usize,
	free_list_head: u64,
	max_value: usize,
	invalid_ptr: [u8; 8],
	free_list_end: [u8; 8],
}

impl SlabDataParams {
	fn new(slab_size: usize, slab_count: usize) -> Result<Self, Error> {
		let mut ptr_size = 0;
		// add 2 (1 termination pointer and one for free status)
		let mut x = slab_count.saturating_add(2);
		loop {
			if x == 0 {
				break;
			}
			x >>= 8;
			ptr_size += 1;
		}
		let free_list_head = 0;

		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..ptr_size]);
		let max_value = slice_to_usize(&ptr[0..ptr_size])?;
		let mut invalid_ptr = [0u8; 8];
		let mut free_list_end = [0u8; 8];
		usize_to_slice(max_value - 1, &mut invalid_ptr[0..ptr_size])?;
		usize_to_slice(max_value, &mut free_list_end[0..ptr_size])?;

		Ok(Self {
			slab_size,
			slab_count,
			ptr_size,
			free_list_head,
			max_value,
			invalid_ptr,
			free_list_end,
		})
	}
}

struct SlabDataHolder {
	sdp: SlabDataParams,
	sd: Box<dyn SlabData + Send + Sync>,
	stats: SlabStats,
}

impl Eq for SlabDataHolder {}

impl PartialEq for SlabDataHolder {
	fn eq(&self, other: &SlabDataHolder) -> bool {
		self.sdp.slab_size == other.sdp.slab_size
	}
}

impl PartialOrd for SlabDataHolder {
	fn partial_cmp(&self, other: &SlabDataHolder) -> Option<Ordering> {
		Some(if self.sdp.slab_size < other.sdp.slab_size {
			Ordering::Less
		} else if self.sdp.slab_size > other.sdp.slab_size {
			Ordering::Greater
		} else {
			Ordering::Equal
		})
	}
}
impl Ord for SlabDataHolder {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.sdp.slab_size < other.sdp.slab_size {
			Ordering::Less
		} else if self.sdp.slab_size > other.sdp.slab_size {
			Ordering::Greater
		} else {
			Ordering::Equal
		}
	}
}

impl SlabDataHolder {
	fn new(
		sdp: SlabDataParams,
		sd: Box<dyn SlabData + Send + Sync>,
		slabs_per_resize: usize,
	) -> Self {
		let stats = SlabStats::new(slabs_per_resize, sdp.slab_count);
		Self { sdp, sd, stats }
	}
}

#[class{
    pub slab_allocator;
    const slab_config: Vec<SlabAllocatorConfig> = vec![];
    const slabs_per_resize: usize = 1_000;
    const zeroed: bool = false;
    var slab_data: Vec<SlabDataHolder>;

    [slab_allocator]
    fn allocate(&mut self, size: usize) -> Result<u64, Error>;

    [slab_allocator]
    fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error>;

    [slab_allocator]
    fn read(&self, id: u64) -> Result<&[u8], Error>;

    [slab_allocator]
    fn free(&mut self, id: u64) -> Result<(), Error>;

    [slab_allocator]
    fn stats(&self) -> Result<Vec<SlabStats>, Error>;
}]
impl SlabAllocatorClass {
	fn builder(constants: &SlabAllocatorClassConst) -> Result<Self, Error> {
		if constants.slab_config.len() > u8::MAX as usize {
			err!(Configuration, "no more than {} slab_configs", u8::MAX)
		} else {
			let mut slab_data = vec![];

			let mut ret = Self { slab_data };
			for config in &constants.slab_config {
				let mut sd = slab_data_sync_box!()?;
				let sdp = SlabDataParams::new(config.slab_size, config.slab_count)?;
				sd.resize((config.slab_size + sdp.ptr_size) * constants.slabs_per_resize)?;
				SlabAllocatorClass::init_free_list(&mut sd, &sdp, constants.slabs_per_resize, 0)?;
				ret.slab_data
					.push(SlabDataHolder::new(sdp, sd, constants.slabs_per_resize));
			}

			ret.slab_data.sort();

			Ok(ret)
		}
	}
}

impl SlabAllocatorClass {
	fn stats(&self) -> Result<Vec<SlabStats>, Error> {
		let mut ret = vec![];
		let slab_data = self.vars().get_slab_data();
		for data in slab_data {
			ret.push(data.stats.clone());
		}
		Ok(ret)
	}

	fn free(&mut self, id: u64) -> Result<(), Error> {
		debug!("free {}", id)?;

		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;

		debug!("free index = {}", index)?;
		let sdh = &mut self.vars_mut().get_mut_slab_data()[index];
		let id_relative: usize = try_into!(id_relative)?;

		let mut first_free_slice = [0u8; 8];
		usize_to_slice(
			try_into!(sdh.sdp.free_list_head)?,
			&mut first_free_slice[0..sdh.sdp.ptr_size],
		)?;

		sdh.sd.update(
			&first_free_slice[0..sdh.sdp.ptr_size],
			id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size),
		)?;

		sdh.sdp.free_list_head = try_into!(id_relative)?;
		debug!("update firstfree to {}", sdh.sdp.free_list_head)?;
		sdh.stats.cur_slabs -= 1;

		Ok(())
	}

	fn allocate(&mut self, size: usize) -> Result<u64, Error> {
		debug!("allocate {}", size)?;

		let zeroed = *self.constants().get_zeroed();
		let slab_data = &mut self.vars_mut().get_mut_slab_data();
		let len = slab_data.len();

		let mut mid = len / 2;
		let mut max = len.saturating_sub(1);
		let mut min = 0;
		loop {
			info!("min={},mid={},max={}", min, mid, max)?;
			info!("try mid = {}", slab_data[mid].sdp.slab_size)?;
			if slab_data[mid].sdp.slab_size == size {
				debug!("index={}", mid)?;
				let mut sdh = &mut slab_data[mid];
				debug!("found: {:?}", sdh.sdp)?;
				let index_u64: u64 = try_into!(mid)?;
				let mut ret = index_u64 << 56;
				debug!("index_u64={},ret={}", index_u64, ret)?;
				match Self::get_next_free(&mut sdh)? {
					Some(v) => {
						debug!("v={}", v)?;
						ret |= v;
					}
					None => {
						if sdh.stats.slabs_per_resize + sdh.stats.cur_slabs <= sdh.stats.max_slabs {
							debug!(
								"do realloc: cur_slabs={},max={}",
								sdh.stats.cur_slabs, sdh.stats.max_slabs
							)?;

							info!(
								"resize to {}",
								(sdh.stats.slabs_per_resize + sdh.stats.cur_slabs)
									* sdh.sdp.slab_size
							)?;
							sdh.sd.resize(
								(sdh.stats.slabs_per_resize + sdh.stats.cur_slabs)
									* (sdh.sdp.slab_size + sdh.sdp.ptr_size),
							)?;
							sdh.stats.cur_capacity += sdh.stats.slabs_per_resize;
							Self::init_free_list(
								&mut sdh.sd,
								&sdh.sdp,
								sdh.stats.slabs_per_resize,
								sdh.stats.cur_slabs,
							)?;
							sdh.sdp.free_list_head = try_into!(sdh.stats.cur_slabs)?;
						} else {
							debug!(
								"can't realloc sdh.stats.cur_slabs={}, sdh.stats.max_slabs={}",
								sdh.stats.cur_slabs, sdh.stats.max_slabs
							)?;
						}

						debug!("try get next free")?;
						match Self::get_next_free(&mut sdh)? {
							Some(v) => {
								debug!("v={}", v)?;
								ret |= v;
							}
							None => {
								debug!("ret err")?;
								return err!(OutOfSlabs, "no more slabs");
							}
						}
					}
				}
				sdh.stats.cur_slabs += 1;
				debug!("ret={},index={}", ret, mid)?;
				if zeroed {
					self.zero(ret, size)?;
				}
				return Ok(ret);
			} else if slab_data[mid].sdp.slab_size > size {
				if max <= min {
					debug!("break max = {} min = {}", max, min)?;
					break;
				}
				max = mid.saturating_sub(1);
			} else {
				if max <= min {
					debug!("break max = {} min = {}", max, min)?;
					break;
				}
				min = mid.saturating_add(1);
			}
			mid = min + (max.saturating_sub(min) / 2);
		}

		err!(
			IllegalArgument,
			"SlabSize({}) not found in this slab allocator",
			size
		)
	}

	fn zero(&mut self, id: u64, slab_size: usize) -> Result<(), Error> {
		let mut empty = vec![];
		empty.resize(slab_size, 0u8);
		self.write(id, &empty, 0)?;
		Ok(())
	}

	fn read(&self, id: u64) -> Result<&[u8], Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;

		let sdh = &self.vars().get_slab_data()[index];
		let id_relative: usize = try_into!(id_relative)?;
		sdh.sd.data(
			sdh.sdp.ptr_size + (id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size)),
			sdh.sdp.slab_size,
		)
	}

	fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;

		let sdh = &mut self.vars_mut().get_mut_slab_data()[index];
		let id_relative: usize = try_into!(id_relative)?;

		sdh.sd.update(
			data,
			sdh.sdp.ptr_size + (id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size)) + offset,
		)?;
		Ok(())
	}

	fn get_next_free(sdh: &mut SlabDataHolder) -> Result<Option<u64>, Error> {
		let id = sdh.sdp.free_list_head;
		let id_usize: usize = try_into!(id)?;

		if id_usize == sdh.sdp.max_value {
			Ok(None)
		} else {
			debug!("ret={}", id)?;
			let offset = (sdh.sdp.ptr_size + sdh.sdp.slab_size) * id_usize;
			let ptr_size = sdh.sdp.ptr_size;
			sdh.sdp.free_list_head = slice_to_u64(&sdh.sd.data(offset, ptr_size)?)?;
			debug!("set free list head to {}", sdh.sdp.free_list_head)?;
			sdh.sd.update(&sdh.sdp.invalid_ptr[0..ptr_size], offset)?;

			Ok(Some(id))
		}
	}

	fn init_free_list(
		slab_data: &mut Box<dyn SlabData + Send + Sync>,
		sdp: &SlabDataParams,
		size: usize,
		offset: usize,
	) -> Result<(), Error> {
		debug!("init free list {:?}", sdp)?;
		let max_value = sdp.max_value;
		let ptr_size = sdp.ptr_size;
		let slab_size: usize = try_into!(sdp.slab_size)?;
		for i in offset..(offset + size) {
			debug!("init i = {}", i)?;
			let mut next_bytes = [0u8; 8];
			if i < (offset + size) - 1 {
				debug!("SET A REGULAR VALUE================================")?;
				usize_to_slice(i + 1, &mut next_bytes[0..ptr_size])?;
			} else {
				debug!("SET A MAX VALUE================================")?;
				usize_to_slice(max_value, &mut next_bytes[0..ptr_size])?;
			}

			let offset_next = i * (ptr_size + slab_size);
			slab_data.update(&next_bytes[0..ptr_size], offset_next)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	debug!();

	#[test]
	fn test_slab_allocator() -> Result<(), Error> {
		let mut sa = slab_allocator!(SlabConfig(slab_config!(SlabSize(100), SlabCount(300))?))?;
		sa.allocate(100)?;
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(200))?),
			SlabConfig(slab_config!(SlabSize(100), SlabCount(300))?),
			SlabsPerResize(100),
		)?;
		let id1 = sa.allocate(100)?;
		info!("id1={}", id1)?;
		assert_eq!(&sa.read(id1)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id1, b"test1", 0)?;
		assert_eq!(&sa.read(id1)?[0..5], b"test1");

		let id2 = sa.allocate(100)?;
		assert_eq!(&sa.read(id2)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id2, b"test2", 0)?;
		assert_eq!(&sa.read(id2)?[0..5], b"test2");

		let id3 = sa.allocate(100)?;
		assert_eq!(&sa.read(id3)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id3, b"test3", 0)?;
		assert_eq!(&sa.read(id3)?[0..5], b"test3");
		assert_eq!(&sa.read(id2)?[0..5], b"test2");
		assert_eq!(&sa.read(id1)?[0..5], b"test1");

		let id4 = sa.allocate(200)?;
		assert_eq!(&sa.read(id4)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id4, b"test4", 0)?;
		assert_eq!(&sa.read(id4)?[0..5], b"test4");

		let id5 = sa.allocate(200)?;
		assert_eq!(&sa.read(id5)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id5, b"test5", 0)?;
		assert_eq!(&sa.read(id5)?[0..5], b"test5");

		let id6 = sa.allocate(200)?;
		assert_eq!(&sa.read(id6)?[0..5], &[0, 0, 0, 0, 0]);
		sa.write(id6, b"test6", 0)?;

		assert_eq!(&sa.read(id6)?[0..5], b"test6");
		assert_eq!(&sa.read(id5)?[0..5], b"test5");
		assert_eq!(&sa.read(id4)?[0..5], b"test4");
		assert_eq!(&sa.read(id3)?[0..5], b"test3");
		assert_eq!(&sa.read(id2)?[0..5], b"test2");
		assert_eq!(&sa.read(id1)?[0..5], b"test1");

		info!("id1={}", id1)?;
		sa.free(id1)?;
		let nid = sa.allocate(100)?;
		info!("nid={}", nid)?;

		let nid = sa.allocate(100)?;
		info!("nid={}", nid)?;

		let nid = sa.allocate(200)?;
		info!("nid={}", nid)?;

		let nid = sa.allocate(200)?;
		info!("nid={}", nid)?;

		Ok(())
	}

	#[test]
	fn test_resize() -> Result<(), Error> {
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(106), SlabCount(10))?),
			SlabsPerResize(2),
		)?;

		for _ in 0..10 {
			let ret = sa.allocate(106)?;
			let data = [1u8; 106];
			sa.write(ret, &data, 0)?;
			info!("re={}", ret)?;
		}

		info!("last alloc")?;
		assert_eq!(sa.allocate(106).unwrap_err().kind(), kind!(OutOfSlabs));

		Ok(())
	}

	#[test]
	fn test_slab_data() -> Result<(), Error> {
		let mut slab_data = slab_data!()?;
		slab_data.resize(100)?;
		slab_data.update(&[0, 1, 2, 3], 10)?;
		assert_eq!(slab_data.data(10, 4)?, [0, 1, 2, 3]);
		assert_eq!(slab_data.data(0, 4)?, [0, 0, 0, 0]);
		assert!(slab_data.data(0, 100).is_ok());
		assert!(slab_data.data(0, 101).is_err());
		assert!(slab_data.data(1, 99).is_ok());
		assert!(slab_data.data(1, 100).is_err());

		slab_data.resize(90)?;
		assert!(slab_data.data(1, 89).is_ok());
		assert!(slab_data.data(1, 90).is_err());

		Ok(())
	}

	#[test]
	fn test_stats() -> Result<(), Error> {
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(10))?),
			SlabsPerResize(2),
		)?;
		let stats = sa.stats()?;
		assert_eq!(stats[0].cur_slabs, 0);
		assert_eq!(stats[0].cur_capacity, 2);
		info!("stats={:?}", stats)?;
		let id1 = sa.allocate(100)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;

		let id2 = sa.allocate(100)?;
		let stats = sa.stats()?;
		assert_eq!(stats[0].cur_slabs, 2);
		assert_eq!(stats[0].cur_capacity, 2);
		info!("stats={:?}", stats)?;

		let id3 = sa.allocate(100)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 3);
		assert_eq!(stats[0].cur_capacity, 4);

		sa.free(id2)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 2);
		assert_eq!(stats[0].cur_capacity, 4);

		sa.free(id3)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 1);
		assert_eq!(stats[0].cur_capacity, 4);

		let id4 = sa.allocate(100)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 2);
		assert_eq!(stats[0].cur_capacity, 4);

		sa.free(id1)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 1);
		assert_eq!(stats[0].cur_capacity, 4);

		sa.free(id4)?;
		let stats = sa.stats()?;
		info!("stats={:?}", stats)?;
		assert_eq!(stats[0].cur_slabs, 0);
		assert_eq!(stats[0].cur_capacity, 4);
		Ok(())
	}

	#[test]
	fn test_zeroed() -> Result<(), Error> {
		let mut expected = vec![];
		expected.resize(100, 1u8);

		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(10))?),
			SlabsPerResize(2),
		)?;
		let mut id_vec = vec![];
		for _ in 0..10 {
			let ret = sa.allocate(100)?;
			let data = [1u8; 100];
			sa.write(ret, &data, 0)?;
			info!("re={}", ret)?;
			id_vec.push(ret);
		}

		for id in id_vec {
			sa.free(id)?;
		}

		let ret = sa.allocate(100)?;
		assert_eq!(&sa.read(ret)?[0..100], &expected);

		let mut expected = vec![];
		expected.resize(100, 0u8);

		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(10))?),
			SlabsPerResize(2),
			Zeroed(true),
		)?;
		let mut id_vec = vec![];
		for _ in 0..10 {
			let ret = sa.allocate(100)?;
			let data = [1u8; 100];
			sa.write(ret, &data, 0)?;
			info!("re={}", ret)?;
			id_vec.push(ret);
		}

		for id in id_vec {
			sa.free(id)?;
		}

		let ret = sa.allocate(100)?;
		assert_eq!(&sa.read(ret)?[0..100], &expected);

		Ok(())
	}
}
