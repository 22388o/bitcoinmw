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
use crate::misc::{set_max, slice_to_usize, usize_to_slice};
use crate::slabio::{
	slab_reader_sync_box, slab_writer_sync_box, SlabIOClassBuilder, SlabIOClassConstOptions,
	SlabReader, SlabWriter,
};
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
        module "bmw_util::array";
        /// @noexample
	pub array_box, array_sync_box;
	var phantom_data: PhantomData<T>;
	var slab_reader: Box<dyn SlabReader + Send + Sync>;
	var slab_writer: Box<dyn SlabWriter + Send + Sync>;
	var_in slab_allocator_in: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>;
        var slab_allocator_id: u128;
	var root: u64;
	var len: usize;
	var ptr_size: usize;
	required bytes_per_entry: usize = 0;
	required len: usize = 0;
	const slab_size: usize = 512;

	[array]
	fn len(&self) -> usize;

	[array]
	fn set_value(&mut self, index: usize, value: &T) -> Result<(), Error>;

	[array]
	fn get_value(&mut self, index: usize, value: &mut T) -> Result<(), Error>;
}]
impl<T> ArrayClass<T> where T: Clone + Serializable + 'static {}

impl<T> ArrayClassVarBuilder for ArrayClassVar<T>
where
	T: Clone + Serializable,
{
	fn builder(constants: &ArrayClassConst) -> Result<Self, Error> {
		let slab_size = constants.slab_size;
		debug!("slab_size array = {}", slab_size)?;
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
			let mut slab_reader =
				slab_reader_sync_box!(SlabIOClassConstOptions::SlabSize(slab_size))?;
			let mut slab_writer =
				slab_writer_sync_box!(SlabIOClassConstOptions::SlabSize(slab_size))?;

			let (id, ptr_size, ids, slab_allocator_id) = match slab_allocator_in.as_mut() {
				Some(sa) => {
					slab_reader.set_slab_allocator(sa.clone())?;
					slab_writer.set_slab_allocator(sa.clone())?;
					let mut sa = sa.wlock()?;
					let said = sa.id();
					let id = sa.allocate(slab_size)?;
					let (ptr_size, ids) =
						Self::allocate(&mut *sa, len, bytes_per_entry, slab_size, id)?;
					(id, ptr_size, ids, said)
				}
				None => THREAD_LOCAL_SLAB_ALLOCATOR.with(
					|f| -> Result<(u64, usize, Vec<u64>, u128), Error> {
						let mut sa = f.borrow_mut();
						let said = sa.id();
						let id = sa.allocate(slab_size)?;
						let (ptr_size, ids) =
							Self::allocate(&mut *sa, len, bytes_per_entry, slab_size, id)?;
						Ok((id, ptr_size, ids, said))
					},
				)?,
			};

			debug!("root={},ptr_size={}", id, ptr_size)?;
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
				ptr_size,
				slab_allocator_id,
			})
		}
	}
}

impl<T> ArrayClassVar<T>
where
	T: Clone + Serializable,
{
	fn allocate(
		slab_allocator: &mut Box<dyn SlabAllocator + Send + Sync>,
		len: usize,
		bytes_per_entry: usize,
		slab_size: usize,
		id: u64,
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
			debug!("allocated slab id = {}", id)?;
			data_slabs.push(id);
			if id > max_data_id {
				max_data_id = id;
			}
		}

		let ptr_size = 8;

		let mut invalid_ptr = [0u8; 8];
		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..ptr_size]);
		let max_value = slice_to_usize(&ptr[0..ptr_size])?;
		usize_to_slice(max_value - 1, &mut invalid_ptr[0..ptr_size])?;

		slab_allocator.write(id, &invalid_ptr[0..ptr_size], slab_size - ptr_size)?;

		Ok((ptr_size, data_slabs))
	}
}

impl<T> Drop for ArrayClass<T>
where
	T: Clone + Serializable,
{
	fn drop(&mut self) {
		match self.clear() {
			Ok(_) => {}
			Err(e) => {
				let _ = warn!("drop resulted in an error: {}", e);
			}
		}
	}
}

impl<T> ArrayClass<T>
where
	T: Clone + Serializable,
{
	fn clear(&mut self) -> Result<(), Error> {
		debug!("clear")?;
		let root = *self.vars().get_root();
		debug!("root={}", root)?;
		let bytes_per_entry = *self.constants().get_bytes_per_entry();
		let slab_size = *self.constants().get_slab_size();
		let len = *self.constants().get_len();
		let slab_reader = self.vars_mut().get_mut_slab_reader();
		slab_reader.seek(root, 0)?;

		let slots = 1 + ((len * bytes_per_entry) / slab_size);

		let mut ids = vec![];
		for i in 0..slots {
			let id = u64::read(slab_reader)?;
			ids.push(id);
			debug!("clear slot[{}]={}", i, id)?;
		}

		match self.vars_mut().get_slab_allocator_in().clone() {
			Some(mut sa) => {
				let mut sa = sa.wlock()?;
				for id in ids {
					Self::clear_slab(&mut sa, id)?;
				}
			}
			None => ThreadLocalSlabAllocator::slab_allocator(
				*self.vars().get_slab_allocator_id(),
				|f| -> Result<(), Error> {
					let mut sa = f.borrow_mut();
					for id in ids {
						Self::clear_slab(&mut sa, id)?;
					}
					Ok(())
				},
			)??,
		}

		let slab_writer = self.vars_mut().get_mut_slab_writer();
		debug!("free tail {}", root)?;
		(*slab_writer).free_tail(root)?;

		Ok(())
	}

	fn clear_slab(sa: &mut Box<dyn SlabAllocator + Send + Sync>, id: u64) -> Result<(), Error> {
		debug!("clear_slab: {}", id)?;
		sa.free(id)?;
		Ok(())
	}

	fn len(&self) -> usize {
		*self.vars().get_len()
	}

	fn get_slab_offset(&mut self, index: usize) -> Result<(u64, usize), Error> {
		let slab_size = *self.constants().get_slab_size();
		let root = *self.vars().get_root();
		let bytes_per_entry = *self.constants().get_bytes_per_entry();
		let slab_reader = self.vars_mut().get_mut_slab_reader();

		slab_reader.seek(root, 0)?;
		let slots = (index * bytes_per_entry) / slab_size;
		let offset = (index * bytes_per_entry) % slab_size;
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

impl<T> Debug for dyn Array<T> {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_array1() -> Result<(), Error> {
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

	#[test]
	fn test_array_other_slab_size() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(57))?),
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(100),
		)?;
		let sa = Some(lock_box!(sa));
		let mut array = array_box!(
			Len(100),
			BytesPerEntry(8),
			SlabAllocatorIn(sa),
			SlabSize(57)
		)?;
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

	#[test]
	fn test_array_drop() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(57))?),
			SlabsPerResize(100),
		)?;
		let lock_box = lock_box!(sa);
		let lock_box_clone = lock_box.clone();

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);

		{
			let sa = Some(lock_box);
			let mut array = array_box!(
				Len(100),
				BytesPerEntry(8),
				SlabAllocatorIn(sa),
				SlabSize(57)
			)?;

			array.set_value(0, &10u128)?;
			let mut v: u128 = 0;
			array.get_value(0, &mut v)?;
			assert_eq!(v, 10);

			assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 18);
		}

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);

		Ok(())
	}

	#[test]
	fn test_array_type() -> Result<(), Error> {
		let _arr: Box<dyn Array<usize> + Send + Sync> =
			array_sync_box!(Len(100), BytesPerEntry(8))?;
		Ok(())
	}
}
