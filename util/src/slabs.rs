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
use std::fmt::Debug;
use SlabAllocatorErrors::*;

debug!();

#[ErrorKind]
enum SlabAllocatorErrors {
	Configuration,
	ArrayIndexOutOfBounds,
	TryReserveError,
	IllegalArgument,
	OutOfSlabs,
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
		let mut x = slab_count + 2;
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

#[class {
    var data: Vec<u8>;

    fn builder(&c) -> Result<Self, Error> {
        let data = vec![];
        Ok(Self { data })
    }

    [slab_data]
    fn data(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
        let data = self.get_data();
        let dlen = data.len();
        let needed = offset + len;
        if needed > dlen {
            err!(ArrayIndexOutOfBounds, "needed={},available={}", needed, dlen)
        } else {
            Ok(&data[offset..offset+len])
        }
    }

    [slab_data]
    fn update(&mut self, v: &[u8], offset: usize) -> Result<(), Error> {
        let vlen = v.len();
        let data = self.get_mut_data();
        let dlen = data.len();
        let needed = vlen + offset;
        if needed > dlen {
            err!(ArrayIndexOutOfBounds, "needed={},available={}", needed, dlen)
        } else {
            data[offset..offset+vlen].clone_from_slice(v);
            Ok(())
        }
    }

    [slab_data]
    fn resize(&mut self, reserved: usize) -> Result<(), Error> {
        let data = self.get_mut_data();
        map_err!(data.try_reserve_exact(reserved), TryReserveError)?;
        data.truncate(reserved);
        data.resize(reserved, 0u8);
        Ok(())
    }

}]
impl SlabDataClass {}

#[class {
    public slab_allocator;
    const slab_size: Vec<usize> = vec![];
    const slab_count: Vec<usize> = vec![];
    const slabs_per_resize: usize = 100;

    var slab_data: HashMap<usize, (SlabDataParams, Box<dyn SlabData + Send + Sync>)>;
    var slab_data_index: HashMap<u8, usize>;

    fn builder(&c) -> Result<Self, Error> {
        let slab_sizes = c.get_slab_size();
        let slab_counts = c.get_slab_count();
        let slab_sizes_len = slab_sizes.len();
        let slab_counts_len = slab_counts.len();
        let slabs_per_resize = *c.get_slabs_per_resize();

        if slab_sizes_len != slab_counts_len {
            err!(Configuration, "configured SlabSizes must be same is SlabCounts")
        } else if slab_sizes_len > u8::MAX.into() {
            err!(Configuration, "more than {} SlabSizes cannot be specified", slab_sizes_len)
        } else {
            let mut slab_data = HashMap::new();
            let mut slab_data_index = HashMap::new();

            let mut i = 0;
            for slab_size in slab_sizes {
                let mut nslab_data = slab_data_sync_box!()?;
                let sdp = SlabDataParams::new(try_into!(i)?, *slab_size, slab_counts[i])?;
                nslab_data.resize((slab_size + sdp.ptr_size) * slabs_per_resize)?;
                SlabAllocatorClass::init_free_list(&mut nslab_data, &sdp, slabs_per_resize)?;
                slab_data_index.insert(sdp.index, *slab_size);
                slab_data.insert(*slab_size, (sdp, nslab_data));
                i += 1;
            }


            Ok(Self {
                slab_data,
                slab_data_index,
            })
        }
    }


    [slab_allocator]
    fn allocate(&mut self, size: usize) -> Result<u64, Error> {
        self.allocate_impl(size)
    }

    [slab_allocator]
    fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error> {
        self.write_impl(id, data, offset)
    }

    [slab_allocator]
    fn read(&self, id: u64) -> Result<&[u8], Error> {
        self.read_impl(id)
    }
/*
    [slab_allocator]
    fn free(&self, id: u64) -> Result<(), Error> {
        Ok(())
    }
    */
}]
impl SlabAllocatorClass {}

impl SlabAllocatorClass {
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

	fn allocate_impl(&mut self, size: usize) -> Result<u64, Error> {
		match self.get_mut_slab_data().get_mut(&size) {
			Some((sdp, slab_data)) => {
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

	fn read_impl(&self, id: u64) -> Result<&[u8], Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: u8 = try_into!(index)?;

		debug!("id={},index={},id_rel={}", id, index, id_relative)?;
		let slab_size = match self.get_slab_data_index().get(&index) {
			Some(slab_size) => *slab_size,
			None => return err!(IllegalArgument, "invalid id"),
		};

		match self.get_slab_data().get(&slab_size) {
			Some((sdp, slab_data)) => {
				let id_relative: usize = try_into!(id_relative)?;
				slab_data.data(id_relative * (sdp.ptr_size + sdp.slab_size), slab_size)
			}
			None => {
				err!(IllegalArgument, "invalid id")
			}
		}
	}

	fn write_impl(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: u8 = try_into!(index)?;
		debug!("id={},index={},id_rel={}", id, index, id_relative)?;
		let slab_size = match self.get_slab_data_index().get(&index) {
			Some(slab_size) => *slab_size,
			None => return err!(IllegalArgument, "invalid id"),
		};

		match self.get_mut_slab_data().get_mut(&slab_size) {
			Some((sdp, slab_data)) => {
				if offset + data.len() > sdp.slab_size {
					err!(IllegalArgument, "data did not fit into slab")
				} else {
					let id_relative: usize = try_into!(id_relative)?;
					slab_data
						.update(data, id_relative * (sdp.ptr_size + sdp.slab_size) + offset)?;
					Ok(())
				}
			}
			None => {
				err!(IllegalArgument, "invalid id")
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	debug!();

	#[test]
	fn test_slab_allocator() -> Result<(), Error> {
		let mut slab_allocator = slab_allocator!(
			SlabSize(100),
			SlabCount(100),
			SlabSize(200),
			SlabCount(200),
			SlabSize(300),
			SlabCount(300)
		)?;
		let mut ids = vec![];
		let mut lens = vec![];
		let id = slab_allocator.allocate(100)?;
		debug!("id100={}", id)?;
		slab_allocator.write(id, &[1, 1, 1, 1], 0)?;
		ids.push(id);
		lens.push(100);
		let id = slab_allocator.allocate(200)?;
		debug!("id200={}", id)?;
		slab_allocator.write(id, &[2, 2, 2, 2], 0)?;
		ids.push(id);
		lens.push(200);
		let id = slab_allocator.allocate(300)?;
		debug!("id300={}", id)?;
		slab_allocator.write(id, &[3, 3, 3, 3], 0)?;
		ids.push(id);
		lens.push(300);
		let id = slab_allocator.allocate(300)?;
		debug!("id300={}", id)?;
		slab_allocator.write(id, &[4, 4, 4, 4], 0)?;
		ids.push(id);
		lens.push(300);
		let id = slab_allocator.allocate(300)?;
		debug!("id300={}", id)?;
		slab_allocator.write(id, &[5, 5, 5, 5], 0)?;
		ids.push(id);
		lens.push(300);
		let id = slab_allocator.allocate(100)?;
		debug!("id100={}", id)?;
		slab_allocator.write(id, &[6, 6, 6, 6], 0)?;
		ids.push(id);
		lens.push(100);

		let mut counter = 1;
		for id in ids {
			let b = slab_allocator.read(id)?;
			assert_eq!(&b[0..5], &[counter, counter, counter, counter, 0]);
			assert_eq!(b.len(), lens[counter as usize - 1]);
			counter += 1;
		}

		/*
		slab_allocator.write(id, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0)?;
		let data = slab_allocator.read(id)?;
		assert_eq!(data, &[5, 6, 7, 8, 9, 10, 0, 0, 0, 0]);
		slab_allocator.free(id)?;
			*/

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
