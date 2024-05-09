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
use std::collections::HashMap;
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

#[derive(Debug)]
struct SlabDataParams {
	index: u8,
	slab_size: usize,
	slab_count: usize,
	ptr_size: usize,
	free_list_head: u64,
	max_value: usize,
	invalid_ptr: [u8; 8],
	free_list_end: [u8; 8],
}

impl SlabDataParams {
	fn new(index: u8, slab_size: usize, slab_count: usize) -> Result<Self, Error> {
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
			index,
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

#[class{
    pub slab_allocator;
    const slab_config: Vec<SlabAllocatorConfig> = vec![];
    const slabs_per_resize: usize = 1_000;
    var slab_data: Vec<(SlabDataParams, Box<dyn SlabData + Send + Sync>)>;
    var slab_data_index: HashMap<usize, usize>;

    [slab_allocator]
    fn allocate(&mut self, size: usize) -> Result<u64, Error>;

    [slab_allocator]
    fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error>;

    [slab_allocator]
    fn read(&self, id: u64) -> Result<&[u8], Error>;

    [slab_allocator]
    fn free(&self, id: u64) -> Result<(), Error>;
}]
impl SlabAllocatorClass {
	fn builder(constants: &SlabAllocatorClassConst) -> Result<Self, Error> {
		let mut slab_data = vec![];
		let mut slab_data_index = HashMap::new();

		let mut index = 0u8;

		let mut ret = Self {
			slab_data,
			slab_data_index,
		};

		if constants.slab_config.len() > u8::MAX as usize {
			err!(Configuration, "no more than {} slab_configs", u8::MAX)
		} else {
			for config in &constants.slab_config {
				let mut sdsb = slab_data_sync_box!()?;
				let sdp = SlabDataParams::new(index, config.slab_size, config.slab_count)?;
				sdsb.resize((config.slab_size + sdp.ptr_size) * constants.slabs_per_resize)?;
				SlabAllocatorClass::init_free_list(&mut sdsb, &sdp, constants.slabs_per_resize)?;
				ret.slab_data_index.insert(config.slab_size, index as usize);
				ret.slab_data.push((sdp, sdsb));
				index += 1;
			}

			Ok(ret)
		}
	}
}

impl SlabAllocatorClass {
	fn allocate(&mut self, size: usize) -> Result<u64, Error> {
		debug!("allocate {}", size)?;
		let slab_data_index = self.vars_mut().get_mut_slab_data_index();
		match slab_data_index.get_mut(&size) {
			Some(index) => {
				let index = *index;
				let (sdp, slab_data) = &mut self.vars_mut().get_mut_slab_data()[index];
				debug!("found: {:?}", sdp)?;
				let index_u64: u64 = sdp.index.into();
				let mut ret = index_u64 << 56;
				debug!("index_u64={},ret={}", index_u64, ret)?;
				match Self::get_next_free(slab_data, sdp)? {
					Some(v) => {
						ret |= v;
					}
					None => {
						return err!(OutOfSlabs, "no more slabs");
					}
				}
				Ok(ret)
			}
			None => {
				err!(IllegalArgument, "SlabSize({}) not supported", size)
			}
		}
	}

	fn read(&self, id: u64) -> Result<&[u8], Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;

		let (sdp, slab_data) = &self.vars().get_slab_data()[index];
		let id_relative: usize = try_into!(id_relative)?;
		slab_data.data(
			sdp.ptr_size + (id_relative * (sdp.ptr_size + sdp.slab_size)),
			sdp.slab_size,
		)
	}

	fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;

		let (sdp, slab_data) = &mut self.vars_mut().get_mut_slab_data()[index];
		let id_relative: usize = try_into!(id_relative)?;

		slab_data.update(
			data,
			sdp.ptr_size + (id_relative * (sdp.ptr_size + sdp.slab_size)) + offset,
		)?;
		Ok(())
	}

	fn get_next_free(
		slab_data: &mut Box<dyn SlabData + Send + Sync>,
		sdp: &mut SlabDataParams,
	) -> Result<Option<u64>, Error> {
		let id = sdp.free_list_head;
		let id_usize: usize = try_into!(id)?;
		debug!("ret={}", id)?;
		let offset = (sdp.ptr_size + sdp.slab_size) * id_usize;
		sdp.free_list_head = slice_to_u64(&slab_data.data(offset, sdp.ptr_size)?)?;
		slab_data.update(&sdp.invalid_ptr[0..sdp.ptr_size], offset)?;

		Ok(Some(id))
	}

	fn init_free_list(
		slab_data: &mut Box<dyn SlabData + Send + Sync>,
		sdp: &SlabDataParams,
		size: usize,
	) -> Result<(), Error> {
		debug!("init free list {:?}", sdp)?;
		let max_value = sdp.max_value;
		let ptr_size = sdp.ptr_size;
		let slab_size: usize = try_into!(sdp.slab_size)?;
		let slab_count = sdp.slab_count;
		for i in 0..size {
			let mut next_bytes = [0u8; 8];
			if i < slab_count - 1 {
				usize_to_slice(i + 1, &mut next_bytes[0..ptr_size])?;
			} else {
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
			SlabConfig(slab_config!(SlabSize(200), SlabCount(100))?),
			SlabConfig(slab_config!(SlabSize(100), SlabCount(300))?),
			SlabsPerResize(100),
		)?;
		let id1 = sa.allocate(100)?;
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
}
