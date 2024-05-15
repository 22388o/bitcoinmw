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

use crate::slabio::{slab_reader_box, slab_writer_box, SlabIOClassBuilder, SlabReader, SlabWriter};
use crate::slabs::{SlabAllocator, THREAD_LOCAL_SLAB_ALLOCATOR};
use bmw_core::*;
use bmw_log::*;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use ArrayErrors::*;

debug!();

#[ErrorKind]
pub enum ArrayErrors {
	IllegalArgument,
	TryReserve,
}

#[class {
    no_send;
    var_in len: usize;
    var phantom_data: PhantomData<&'a T>;
    var slab_reader: Box<dyn SlabReader>;
    var slab_writer: Box<dyn SlabWriter>;
    var root: u64;
    required bytes_per_entry: usize = 0;

    [array]
    fn len(&self) -> usize;

    [array]
    fn get(&self, index: usize) -> &T;

    [array]
    fn get_mut(&mut self, index: usize) -> &mut T;

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
		let mut len = 0;
		let bytes_per_entry = constants.bytes_per_entry;
		for passthrough in &constants.passthroughs {
			if passthrough.name == "len" {
				debug!("found size")?;
				match passthrough.value.downcast_ref::<usize>() {
					Ok(l) => {
						len = *l;
					}
					_ => {}
				}
			}
		}

		if len == 0 {
			err!(IllegalArgument, "Len must be specified and non-zero")
		} else {
			let (id, _ptr_size, ids) =
				THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<(u64, usize, Vec<u64>), Error> {
					let mut sa = f.borrow_mut();
					let id = sa.allocate(512)?;
					let (ptr_size, ids) = Self::allocate(&mut *sa, len, bytes_per_entry, 512, id)?;
					Ok((id, ptr_size, ids))
				})?;
			debug!("id={}", id)?;
			let mut slab_reader = slab_reader_box!()?;
			let mut slab_writer = slab_writer_box!()?;
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

	fn get(&self, _index: usize) -> &T {
		todo!()
	}

	fn get_mut(&mut self, _index: usize) -> &mut T {
		todo!()
	}

	fn set_value(&mut self, index: usize, value: &T) -> Result<(), Error> {
		let root = *self.vars().get_root();
		let slab_reader = self.vars_mut().get_mut_slab_reader();
		slab_reader.seek(root, 0)?;
		let mut ptr: usize = 0;
		for _ in 0..(index + 1) {
			ptr = usize::read(&mut *slab_reader)?;
		}
		debug!("ptr={}", ptr)?;

		let slab_writer = self.vars_mut().get_mut_slab_writer();
		value.write(&mut *slab_writer)?;
		Ok(())
	}

	fn get_value(&mut self, _index: usize, value: &mut T) -> Result<(), Error> {
		let slab_reader = self.vars_mut().get_mut_slab_reader();
		*value = T::read(&mut *slab_reader)?;
		Ok(())
	}
}

impl<'a, T> Debug for dyn Array<'a, T> {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		Ok(())
	}
}

impl<'a, T> IndexMut<usize> for dyn Array<'a, T>
where
	T: Clone + Serializable + 'a,
{
	fn index_mut(&mut self, index: usize) -> &mut <Self as Index<usize>>::Output {
		self.get_mut(index)
	}
}

impl<'a, T> Index<usize> for dyn Array<'a, T>
where
	T: Clone + Serializable + 'a,
{
	type Output = T;
	fn index(&self, index: usize) -> &<Self as Index<usize>>::Output {
		self.get(index)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_array() -> Result<(), Error> {
		let mut array = array_box!(Len(100), BytesPerEntry(8))?;
		array.set_value(0, &135u64)?;
		debug!("size={}", array.len())?;

		assert_eq!(array.len(), 100);

		let mut v: u64 = 0;
		array.get_value(0, &mut v)?;
		//assert_eq!(v, 135);

		Ok(())
	}
}
