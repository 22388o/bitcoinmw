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

use crate::lock::*;
use crate::slabio::{slab_reader_box, slab_writer_box, SlabIOClassBuilder, SlabReader, SlabWriter};
use crate::slabs::*;
use bmw_core::*;
use bmw_log::*;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use ArrayErrors::*;

debug!();

#[ErrorKind]
pub enum ArrayErrors {
	IllegalArgument,
	TryReserve,
}

#[class {
    no_send;
    var phantom_data: PhantomData<&'a T>;
    var slab_reader: Box<dyn SlabReader>;
    var slab_writer: Box<dyn SlabWriter>;
    var_in slab_allocator_in: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>;
    var root: u64;
    var len: usize;
    required bytes_per_entry: usize = 0;
    required len: usize = 0;

    [array]
    fn len(&self) -> usize;

    [array]
    fn set_value(&mut self, index: usize, value: &T) -> Result<(), Error>;

    [array]
    fn get_value(&mut self, index: usize, value: &mut T) -> Result<(), Error>;
}]
impl<'a, T> ArrayClass<'a, T> where T: Clone + Serializable + 'a {}

impl<'a, T> ArrayClassVarBuilder for ArrayClassVar<'a, T>
where
	T: Clone + Serializable + 'a,
{
	fn builder(constants: &ArrayClassConst) -> Result<Self, Error> {
		let len = constants.len;
		let bytes_per_entry = constants.bytes_per_entry;
		let mut slab_allocator_in = None;

		for passthrough in &constants.passthroughs {
			if passthrough.name == "slab_allocator_in" {
				debug!("found slab allocator")?;
				match passthrough
					.value
					.downcast_ref::<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>>(
					) {
					Ok(slab_allocator) => {
						slab_allocator_in = slab_allocator.clone();
					}
					_ => {}
				}
			}
		}

		if len == 0 {
			err!(IllegalArgument, "Len must not be zero")
		} else {
			let mut slab_reader = slab_reader_box!()?;
			let mut slab_writer = slab_writer_box!()?;

			let (id, _ptr_size, ids) = match slab_allocator_in.as_mut() {
				Some(sa) => {
					slab_reader.set_slab_allocator(sa.clone())?;
					slab_writer.set_slab_allocator(sa.clone())?;
					let mut sa = sa.wlock()?;
					let id = sa.allocate(512)?;
					let (ptr_size, ids) = Self::allocate(&mut *sa, len, bytes_per_entry, 512, id)?;
					(id, ptr_size, ids)
				}
				None => THREAD_LOCAL_SLAB_ALLOCATOR.with(
					|f| -> Result<(u64, usize, Vec<u64>), Error> {
						let mut sa = f.borrow_mut();
						let id = sa.allocate(512)?;
						let (ptr_size, ids) =
							Self::allocate(&mut *sa, len, bytes_per_entry, 512, id)?;
						Ok((id, ptr_size, ids))
					},
				)?,
			};

			debug!("id={}", id)?;
			slab_reader.seek(id, 0)?;
			slab_writer.seek(id, 0)?;

			for id in ids {
				id.write(&mut slab_writer)?;
			}

			Ok(Self {
				len,
				phantom_data: PhantomData,
				slab_reader,
				slab_writer,
				root: id,
				slab_allocator_in,
			})
		}
	}
}

impl<'a, T> ArrayClassVar<'a, T>
where
	T: Clone + Serializable + 'a,
{
	fn allocate(
		slab_allocator: &mut Box<dyn SlabAllocator + Send + Sync>,
		len: usize,
		bytes_per_entry: usize,
		slab_size: usize,
		_id: u64,
	) -> Result<(usize, Vec<u64>), Error> {
		let data_slab_count = 1 + (bytes_per_entry * len / slab_size);
		debug!(
			"data_slabs={},bytes_per_entry={},slab_size={},len={}",
			data_slab_count, bytes_per_entry, slab_size, len
		)?;
		let mut data_slabs = vec![];
		let mut max_data_id = 0;
		for _ in 0..data_slab_count {
			let id = slab_allocator.allocate(slab_size)?;
			data_slabs.push(id);
			if id > max_data_id {
				max_data_id = id;
			}
		}

		let mut ptr_size = 0;
		let mut x = max_data_id;
		loop {
			if x == 0 {
				break;
			}
			x >>= 8;
			ptr_size += 1;
		}

		Ok((ptr_size, data_slabs))
	}
}

impl<'a, T> ArrayClass<'a, T>
where
	T: Clone + Serializable + 'a,
{
	fn len(&self) -> usize {
		*self.vars().get_len()
	}

	fn get_slab_offset(&mut self, index: usize) -> Result<(u64, usize), Error> {
		let root = *self.vars().get_root();
		let bytes_per_entry = *self.constants().get_bytes_per_entry();
		let slab_reader = self.vars_mut().get_mut_slab_reader();

		slab_reader.seek(root, 0)?;
		let slots = (index * bytes_per_entry) / 512;
		let offset = (index * bytes_per_entry) % 512;
		slab_reader.skip(8 * slots)?;
		let ptr = u64::read(&mut *slab_reader)?;
		Ok((ptr, offset))
	}

	fn set_value(&mut self, index: usize, value: &T) -> Result<(), Error> {
		let (ptr, offset) = self.get_slab_offset(index)?;
		debug!("ptr={},offset={}", ptr, offset)?;

		let slab_writer = self.vars_mut().get_mut_slab_writer();
		slab_writer.seek(ptr, offset)?;
		value.write(&mut *slab_writer)?;
		Ok(())
	}

	fn get_value(&mut self, index: usize, value: &mut T) -> Result<(), Error> {
		let (ptr, offset) = self.get_slab_offset(index)?;
		let slab_reader = self.vars_mut().get_mut_slab_reader();
		slab_reader.seek(ptr, offset)?;
		*value = T::read(&mut *slab_reader)?;
		Ok(())
	}
}

impl<'a, T> Debug for dyn Array<'a, T> {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_array() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(200))?),
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(100),
		)?;
		let sa = Some(lock_box!(sa));
		let mut array = array_box!(Len(100), BytesPerEntry(8), SlabAllocatorIn(sa))?;
		array.set_value(0, &135u64)?;
		debug!("size={}", array.len())?;

		assert_eq!(array.len(), 100);

		array.set_value(90, &111)?;

		let mut v: u64 = 0;
		array.get_value(0, &mut v)?;
		assert_eq!(v, 135);

		array.set_value(1, &136u64)?;

		let mut v: u64 = 0;
		array.get_value(1, &mut v)?;
		assert_eq!(v, 136);

		let mut v: u64 = 0;
		array.get_value(90, &mut v)?;
		assert_eq!(v, 111);

		Ok(())
	}
}
