// Copyright (c) 2023-2024, The BitcoinMW Developers
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

use crate::lock::LockBox;
use crate::slabs::{SlabAllocator, THREAD_LOCAL_SLAB_ALLOCATOR};
use crate::{set_max, slice_to_u64, slice_to_usize, usize_to_slice};
use bmw_core::*;
use bmw_log::*;

info!();

#[class {
        module "bmw_util::slabio";
        pub slab_reader, slab_writer, slab_reader_box, slab_writer_box;
	var slab_allocator: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>;
	var id: u64;
	var offset: usize;
        var ptr_size: usize;
        var invalid_ptr: [u8; 8];
        var max_value: usize;
        var null_ptr: [u8; 8];
        const max_slabs: usize = (u32::MAX as usize) - 2;
        const slab_size: usize = 512;

	[slab_reader, slab_writer]
	fn set_slab_allocator(&mut self, slab_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>) -> Result<(), Error>;

	[slab_reader, slab_writer]
	fn seek(&mut self, id: u64, offset: usize) -> Result<(), Error>;

	[slab_reader, slab_writer]
	fn skip(&mut self, bytes: usize) -> Result<(), Error>;

        [slab_reader]
        fn read_fixed_bytes_impl(&mut self, bytes: &mut [u8]) -> Result<(), Error>;

        [slab_writer]
        fn write_fixed_bytes_impl(&mut self, bytes: &[u8]) -> Result<(), Error>;

        [slab_reader, slab_writer]
        fn get_id(&self) -> u64;

        [slab_reader, slab_writer]
        fn get_offset(&self) -> usize;

        [slab_writer]
        fn free_tail(&mut self, id: u64) -> Result<(), Error>;

}]
impl SlabIOClass {}

pub(crate) use slab_reader_box;
pub(crate) use slab_writer_box;

impl SlabIOClassVarBuilder for SlabIOClassVar {
	fn builder(constants: &SlabIOClassConst) -> Result<Self, Error> {
		let slab_allocator = None;
		let mut invalid_ptr = [0u8; 8];
		let mut null_ptr = [0u8; 8];

		let mut ptr_size = 0;
		// add 2 (1 termination pointer and one for free status)
		let mut x = constants.max_slabs.saturating_add(2);
		loop {
			if x == 0 {
				break;
			}
			x >>= 8;
			ptr_size += 1;
		}

		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..ptr_size]);
		let max_value = slice_to_usize(&ptr[0..ptr_size])?;
		usize_to_slice(max_value - 1, &mut invalid_ptr[0..ptr_size])?;
		usize_to_slice(max_value, &mut null_ptr[0..ptr_size])?;

		Ok(Self {
			slab_allocator,
			id: try_into!(max_value)?,
			offset: 0,
			ptr_size,
			invalid_ptr,
			null_ptr,
			max_value,
		})
	}
}

impl SlabIOClass {
	fn free_tail(&mut self, _id: u64) -> Result<(), Error> {
		todo!()
	}

	fn get_id(&self) -> u64 {
		*self.vars().get_id()
	}

	fn get_offset(&self) -> usize {
		*self.vars().get_offset()
	}

	fn set_slab_allocator(
		&mut self,
		slab_allocator: Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>,
	) -> Result<(), Error> {
		*self.vars_mut().get_mut_slab_allocator() = Some(slab_allocator);
		Ok(())
	}

	fn seek(&mut self, id: u64, offset: usize) -> Result<(), Error> {
		*self.vars_mut().get_mut_id() = id;
		*self.vars_mut().get_mut_offset() = offset;
		Ok(())
	}

	fn skip(&mut self, bytes: usize) -> Result<(), Error> {
		match self.slab_allocator() {
			Some(mut slab_allocator) => {
				let mut slab_allocator = slab_allocator.wlock()?;
				self.do_skip(bytes, &mut *slab_allocator)?;
			}
			None => {
				THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
					let mut sa = f.borrow_mut();
					self.do_skip(bytes, &mut *sa)?;
					Ok(())
				})?;
			}
		}

		Ok(())
	}

	fn do_skip(
		&mut self,
		bytes: usize,
		slab_allocator: &mut Box<dyn SlabAllocator + Send + Sync>,
	) -> Result<(), Error> {
		let mut cur_id = *self.vars().get_id();
		let cur_offset = *self.vars().get_offset();
		let ptr_size = *self.vars().get_ptr_size();
		let slab_size = *self.constants().get_slab_size();
		let data_per_slab = slab_size - ptr_size;

		*self.vars_mut().get_mut_offset() = (cur_offset + bytes) % data_per_slab;

		let nblocks = (cur_offset + bytes) / data_per_slab;

		for _ in 0..nblocks {
			let cur_block = slab_allocator.read(cur_id)?;
			cur_id = slice_to_u64(&cur_block[data_per_slab..data_per_slab + ptr_size])?;
		}
		*self.vars_mut().get_mut_id() = cur_id;

		Ok(())
	}

	fn cur_id(&mut self) -> &mut u64 {
		self.vars_mut().get_mut_id()
	}

	fn cur_offset(&mut self) -> &mut usize {
		self.vars_mut().get_mut_offset()
	}

	fn slab_allocator(&mut self) -> Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>> {
		(*self.vars_mut().get_mut_slab_allocator()).clone()
	}

	fn read_fixed_bytes_impl(&mut self, ret: &mut [u8]) -> Result<(), Error> {
		match self.slab_allocator() {
			Some(slab_allocator) => {
				let slab_allocator = slab_allocator.rlock()?;
				self.do_read_fixed_bytes_impl(ret, &*slab_allocator)?;
			}
			None => {
				THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
					let sa = f.borrow();
					self.do_read_fixed_bytes_impl(ret, &*sa)?;
					Ok(())
				})?;
			}
		}

		Ok(())
	}

	fn write_fixed_bytes_impl(&mut self, bytes: &[u8]) -> Result<(), Error> {
		match self.slab_allocator() {
			Some(mut slab_allocator) => {
				let mut slab_allocator = slab_allocator.wlock()?;
				self.do_write_fixed_bytes_impl(bytes, &mut *slab_allocator)?;
			}
			None => {
				THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
					let mut sa = f.borrow_mut();
					self.do_write_fixed_bytes_impl(bytes, &mut *sa)?;
					Ok(())
				})?;
			}
		}
		Ok(())
	}

	fn do_read_fixed_bytes_impl(
		&mut self,
		ret: &mut [u8],
		slab_allocator: &Box<dyn SlabAllocator + Send + Sync>,
	) -> Result<(), Error> {
		let mut cur_slab = *self.cur_id();
		let mut cur_offset = *self.cur_offset();
		let ptr_size = *self.vars().get_ptr_size();
		let slab_size = *self.constants().get_slab_size();
		let slab_data_size = slab_size - ptr_size;
		let mut rem = slab_data_size.saturating_sub(cur_offset);
		let mut needed = ret.len();
		let mut itt = 0;

		loop {
			let to_read = if needed < rem { needed } else { rem };
			let slab_bytes = slab_allocator.read(cur_slab)?;

			debug!("loop with cur_slab={},cur_offset={}", cur_slab, cur_offset)?;

			ret[itt..itt + to_read].clone_from_slice(&slab_bytes[cur_offset..cur_offset + to_read]);

			itt += to_read;
			needed -= to_read;

			if needed == 0 {
				cur_offset += to_read;
			}

			cbreak!(needed == 0);

			let next_slab = slice_to_usize(&slab_bytes[slab_data_size..slab_data_size + ptr_size])?;
			cur_slab = try_into!(next_slab)?;
			cur_offset = 0;
			rem = slab_data_size;
		}

		*self.cur_id() = cur_slab;
		*self.cur_offset() = cur_offset;
		Ok(())
	}

	fn do_write_fixed_bytes_impl(
		&mut self,
		bytes: &[u8],
		slab_allocator: &mut Box<dyn SlabAllocator + Send + Sync>,
	) -> Result<(), Error> {
		let slab_size = *self.constants().get_slab_size();
		let ptr_size = *self.vars().get_ptr_size();
		let slab_data_size = slab_size - ptr_size;

		let mut cur_slab = *self.cur_id();
		let mut cur_offset = *self.cur_offset();
		let mut needed = bytes.len();
		let mut itt = 0;

		debug!("pre write loop")?;
		loop {
			let rem = slab_data_size - cur_offset;
			if needed <= rem {
				debug!(
					"needed is enough write slab_id={},offset={}",
					cur_slab, cur_offset
				)?;
				(*slab_allocator).write(cur_slab, &bytes[itt..itt + needed], cur_offset)?;
				cur_offset += needed;
				needed = 0;
			} else {
				(*slab_allocator).write(cur_slab, &bytes[itt..itt + rem], cur_offset)?;
				needed -= rem;
				itt += rem;
				cur_offset = 0;
				let nslab_id = (*slab_allocator).allocate(slab_size)?;
				let mut ptr_bytes = [0u8; 8];
				usize_to_slice(try_into!(nslab_id)?, &mut ptr_bytes[0..ptr_size])?;
				debug!("Allocated new slab: {}", nslab_id)?;
				(*slab_allocator).write(cur_slab, &ptr_bytes[0..ptr_size], slab_data_size)?;

				cur_slab = nslab_id;
			}

			cbreak!(needed == 0);
		}

		*self.cur_id() = cur_slab;
		*self.cur_offset() = cur_offset;
		Ok(())
	}
}

impl Reader for Box<dyn SlabReader> {
	fn read_fixed_bytes(&mut self, ret: &mut [u8]) -> Result<(), Error> {
		self.read_fixed_bytes_impl(ret)
	}
}

impl Writer for Box<dyn SlabWriter> {
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.write_fixed_bytes_impl(bytes.as_ref())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::lock::{build_lock_box, lock_box};
	use crate::slabs::*;

	debug!();

	#[test]
	fn test_slab_reader() -> Result<(), Error> {
		let mut slab_allocator = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(1_000))?),
			SlabsPerResize(100),
		)?;
		let id = slab_allocator.allocate(512)?;
		debug!("id={}", id)?;
		let mut slab_reader = slab_reader_box!()?;
		let mut slab_writer = slab_writer_box!()?;
		let lb1 = lock_box!(slab_allocator);
		let lb2 = lb1.clone();
		slab_reader.set_slab_allocator(lb1)?;
		slab_writer.set_slab_allocator(lb2)?;
		slab_reader.seek(id, 0)?;
		slab_writer.seek(id, 0)?;

		let mut v = 100;
		for _ in 0..100 {
			slab_writer.write_usize(v)?;
			v += 100;
		}

		let mut v = 100;
		for _ in 0..100 {
			let x = slab_reader.read_usize()?;
			assert_eq!(x, v);
			v += 100;
			debug!("x={:?}", x)?;
		}

		Ok(())
	}

	#[test]
	fn test_thread_local_slabs() -> Result<(), Error> {
		let id = THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<u64, Error> {
			let mut sa = f.borrow_mut();
			sa.allocate(512)
		})?;
		debug!("id={}", id)?;
		let mut slab_reader = slab_reader_box!()?;
		let mut slab_writer = slab_writer_box!()?;
		slab_reader.seek(id, 0)?;
		slab_writer.seek(id, 0)?;

		let mut v = 100;
		for _ in 0..100 {
			slab_writer.write_usize(v)?;
			v += 100;
		}

		let mut v = 100;
		for _ in 0..100 {
			let x = slab_reader.read_usize()?;
			assert_eq!(x, v);
			v += 100;
			debug!("x={:?}", x)?;
		}

		Ok(())
	}

	#[test]
	fn test_slab_reader_fixed_bytes() -> Result<(), Error> {
		let mut slab_allocator = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(1_000))?),
			SlabsPerResize(100),
		)?;
		let id = slab_allocator.allocate(512)?;
		debug!("id={}", id)?;
		let mut slab_reader = slab_reader_box!()?;
		let mut slab_writer = slab_writer_box!()?;
		let lb1 = lock_box!(slab_allocator);
		let lb2 = lb1.clone();
		slab_reader.set_slab_allocator(lb1)?;
		slab_writer.set_slab_allocator(lb2)?;
		slab_reader.seek(id, 0)?;
		slab_writer.seek(id, 0)?;

		let mut bytes = [0u8; 1000];
		for i in 0..1000 {
			bytes[i] = (i % u8::MAX as usize) as u8;
		}
		for i in 0..100 {
			slab_writer.write_fixed_bytes(&bytes[0..(i + 600)])?;
		}

		for i in 0..100 {
			let mut bytes = [0u8; 1000];
			slab_reader.read_fixed_bytes(&mut bytes[0..(i + 600)])?;

			for j in 0..i + 600 {
				assert_eq!(bytes[j], (j % (u8::MAX as usize)) as u8);
			}
		}

		Ok(())
	}

	#[derive(Serializable, PartialEq, Debug)]
	struct SerTest {
		x: u8,
		y: u32,
		z: u128,
		s: String,
	}

	#[test]
	fn test_ser() -> Result<(), Error> {
		let ser_in = SerTest {
			x: 1,
			y: 2,
			z: 3,
			s: "mystr".to_string(),
		};
		let mut slab_allocator = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(1_000))?),
			SlabsPerResize(100),
		)?;
		let id = slab_allocator.allocate(512)?;
		debug!("id={}", id)?;
		let mut slab_reader = slab_reader_box!()?;
		let mut slab_writer = slab_writer_box!()?;
		let lb1 = lock_box!(slab_allocator);
		let lb2 = lb1.clone();
		slab_reader.set_slab_allocator(lb1)?;
		slab_writer.set_slab_allocator(lb2)?;
		slab_reader.seek(id, 0)?;
		slab_writer.seek(id, 0)?;

		ser_in.write(&mut slab_writer)?;
		slab_writer.write_u64(1234)?;
		let ser_out = SerTest::read(&mut slab_reader)?;
		assert_eq!(ser_in, ser_out);
		let v_u64 = slab_reader.read_u64()?;
		assert_eq!(v_u64, 1234);

		Ok(())
	}

	#[test]
	fn test_skip() -> Result<(), Error> {
		let mut slab_allocator = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(1_000))?),
			SlabsPerResize(100),
		)?;
		let id = slab_allocator.allocate(512)?;
		debug!("id={}", id)?;
		let mut slab_reader = slab_reader_box!()?;
		let mut slab_writer = slab_writer_box!()?;
		let lb1 = lock_box!(slab_allocator);
		let lb2 = lb1.clone();
		slab_reader.set_slab_allocator(lb1)?;
		slab_writer.set_slab_allocator(lb2)?;
		slab_reader.seek(id, 0)?;
		slab_writer.seek(id, 0)?;

		for i in 0..100 {
			(i as usize).write(&mut slab_writer)?;
		}

		let v = usize::read(&mut slab_reader)?;
		assert_eq!(v, 0);

		let v = usize::read(&mut slab_reader)?;
		assert_eq!(v, 1);

		slab_reader.skip(8)?;

		let v = usize::read(&mut slab_reader)?;
		assert_eq!(v, 3);

		slab_reader.skip(600)?;

		let v = usize::read(&mut slab_reader)?;
		assert_eq!(v, 79);

		Ok(())
	}
}
