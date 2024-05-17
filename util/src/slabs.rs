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

//! The [`crate::slabs`] module defines and implements the [`SlabAllocator`] trait, which handles
//! the allocation of memory in `slabs` or chunks for data structures that require memory
//! allocation.
//!
//! # Examples
//!```
//! use bmw_core::*;
//! use bmw_log::*;
//! use bmw_util::{slab_config, slab_allocator};
//! use bmw_util::slabs::*;
//!
//! debug!();
//!
//! fn main() -> Result<(), Error> {
//!     let mut sa = slab_allocator!(
//!         SlabConfig(slab_config!(SlabSize(200))?),
//!         SlabConfig(slab_config!(SlabSize(100), SlabCount(300))?),
//!         SlabsPerResize(100),
//!     )?;
//!
//!     let id1 = sa.allocate(100)?;
//!     info!("id1={}", id1)?;
//!     assert_eq!(&sa.read(id1)?[0..5], &[0, 0, 0, 0, 0]);
//!     sa.write(id1, b"test1", 0)?;
//!     assert_eq!(&sa.read(id1)?[0..5], b"test1");
//!
//!     let id2 = sa.allocate(200)?;
//!     assert_eq!(&sa.read(id2)?[0..5], &[0, 0, 0, 0, 0]);
//!     sa.write(id2, b"test2", 0)?;
//!     assert_eq!(&sa.read(id2)?[0..5], b"test2");
//!
//!     Ok(())
//! }
//!```
use crate::misc::{set_max, slice_to_u64, slice_to_usize, usize_to_slice};
use bmw_core::rand::random;
use bmw_core::*;
use bmw_log::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use SlabAllocatorErrors::*;

info!();

thread_local! {
	pub static THREAD_LOCAL_SLAB_ALLOCATOR: RefCell<Box<dyn SlabAllocator + Send + Sync>> = RefCell::new(
		SlabAllocatorClassBuilder::build_slab_allocator_sync_box(
			vec![
				SlabConfig(
					Box::new(
						SlabAllocatorConfig {
							slab_size: 512,
							slab_count: usize::MAX,
						}
					)
				),
				SlabConfig(
					Box::new(
						SlabAllocatorConfig {
							slab_size: 51200,
							slab_count: usize::MAX,
						}
					)
				),
			]
		).unwrap()
	);
}

pub struct ThreadLocalSlabAllocator {}

impl ThreadLocalSlabAllocator {
	pub fn slab_allocator<F, R>(expect_id: u128, f: F) -> Result<R, Error>
	where
		F: FnOnce(&RefCell<Box<dyn SlabAllocator + Send + Sync>>) -> R,
	{
		let slab_allocator_id = THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<u128, Error> {
			let sa = f.borrow();
			Ok(sa.id())
		})?;
		if slab_allocator_id != expect_id {
			err!(
				WrongSlabAllocatorId,
				"perhaps you are trying to use a thread local slab allocator in another thread?"
			)
		} else {
			Ok(THREAD_LOCAL_SLAB_ALLOCATOR.with(f))
		}
	}
}

/// Kinds of errors that can occur in a [`crate::slabs::SlabAllocator`].
#[ErrorKind]
pub enum SlabAllocatorErrors {
	Configuration,
	ArrayIndexOutOfBounds,
	TryReserveError,
	IllegalArgument,
	OutOfSlabs,
	InvalidSlabId,
	DoubleFree,
	WrongSlabAllocatorId,
}

#[class {
    module "bmw_util::slabs";
    clone slab_data;
    /// This class is used by the slab allocator class to allocate, grow, read, and write data.
    pub slab_data_sync_box;
    var data: Vec<u8>;

    /// Retreive raw data from the [`SlabData`].
    /// @param offset the offset, in bytes, to read data from.
    /// @param len the length, in bytes, of data to read.
    /// @param self an immutable reference to the [`SlabData`] to access.
    /// @return an immutable reference to the bytes requested.
    /// @error ArrayIndexOutOfBounds if the requested data is out the bound which are currently
    /// allocated to this [`SlabData`].
    /// @see crate::slabs::SlabData::write
    [slab_data]
    fn read(&self, offset: usize, len: usize) -> Result<&[u8], Error>;

    /// Update the raw data with the specified value.
    /// @param offset the offset, in bytes, to write data to.
    /// @param value an immutable reference to an array of [`u8`] to write to this [`SlabData`].
    /// @param self a mutable reference to the [`SlabData`] to write.
    /// @return n/a
    /// @error ArrayIndexOutOfBounds if the requested data is out the bound which are currently
    /// allocated to this [`SlabData`].
    /// @see crate::slabs::SlabData::data
    [slab_data]
    fn write(&mut self, value: &[u8], offset: usize) -> Result<(), Error>;

    /// grows or truncates the [`SlabData`] to the specified size.
    /// @param self a mutable reference to the [`SlabData`] to resize.
    /// @param size the size, in bytes, to reserve.
    /// @error TryReserveError if the resize fails due to an underlying TryReserve error by the
    /// [`std::vec::Vec`].
    /// @return n/a
    /// @see crate::slabs::SlabData
    [slab_data]
    fn resize(&mut self, size: usize) -> Result<(), Error>;

}]
impl SlabDataClass {}

impl SlabDataClassVarBuilder for SlabDataClassVar {
	fn builder(_constants: &SlabDataClassConst) -> Result<Self, Error> {
		let data = vec![];
		Ok(Self { data })
	}
}

impl SlabDataClass {
	fn read(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
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

	fn write(&mut self, v: &[u8], offset: usize) -> Result<(), Error> {
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

/// A configuration used by the [`crate::slabs::SlabAllocator`]. See [`crate::slab_config`].
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

/// Builds an instance of the  [`SlabAllocatorConfig`] struct using the specified input parameters.
/// # Input Parameters
/// * SlabSize([`usize`]) - the size, in bytes, of slabs for this slab allocator config. The default
/// value is 512.<br/>
/// * SlabCount([`usize`]) - the maximum number of slabs available in this slab allocator. The
/// default value is usize::MAX. <br/>
///
/// # Examples
///```
/// use bmw_core::*;
/// use bmw_util::*;
/// use bmw_util::slabs::*;
///
/// fn main() -> Result<(), Error> {
///     let mut sa = slab_allocator!(
///         SlabConfig(slab_config!(SlabSize(100), SlabCount(10))?),
///         SlabConfig(slab_config!(SlabSize(200), SlabCount(20))?),
///         SlabsPerResize(5),
///     )?;
///
///     let id = sa.allocate(100)?;
///     sa.write(id, b"test", 0)?;
///
///     assert_eq!(&sa.read(id)?[0..4], b"test");
///
///     Ok(())
/// }
///```
/// # Also see
/// [`crate::slab_allocator`]<br/>
/// [`SlabAllocator`]
///
#[macro_export]
macro_rules! slab_config {
        ($($params:tt)*) => {{
            configure_box!(SlabAllocatorConfig, SlabAllocatorConfigOptions, vec![$($params)*])
        }};
}
#[cfg(test)]
pub(crate) use slab_config;

/// A statistical snapshot that represents the current state of this [`crate::slabs::SlabAllocator`]
/// which is returned by the [`crate::slabs::SlabAllocator::stats`] function.
#[derive(Clone, Debug)]
pub struct SlabStats {
	/// The current number of slabs that have been allocated by this
	/// [`crate::slabs::SlabAllocator::stats`].
	pub cur_slabs: usize,
	/// The current capacity of this [`crate::slabs::SlabAllocator`].
	pub cur_capacity: usize,
	/// The number of additional slabs allocated when the [`crate::slabs::SlabAllocator`] runs
	/// out of slabs.
	pub slabs_per_resize: usize,
	/// The maximum number of slabs that this [`crate::slabs::SlabAllocator`] can allocate.
	pub max_slabs: usize,
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

#[class {
    module "bmw_util::slabs";
    /// A slab allocator allocates byte arrays, or `slabs`, to be used in data structures.
    pub slab_allocator, slab_allocator_sync_box;
    /// Specifies a [`SlabAllocatorConfig`]. See [`crate::slab_config`].
    const slab_config: Vec<SlabAllocatorConfig> = vec![];
    /// The number of slabs allocated internally, the [`SlabAllocator`] when it has no slabs.
    const slabs_per_resize: usize = 10;
    /// If set to true, all bytes in a slab that is returned by the [`crate::slabs::SlabAllocator::allocate`]
    /// function are set to `0u8`.
    const zeroed: bool = false;
    var slab_data: Vec<SlabDataHolder>;
    var id: u128;

    /// Allocate a slab of the specified size.
    /// @param size the size, in bytes, of the slab.
    /// @param self a mutable reference to the [`SlabAllocator`] to allocate from.
    /// @return the id of the slab that has been allocated.
    /// @error crate::slabs::TryReserveError if no more memory can be reserved
    /// @error crate::slabs::OutOfSlabs if no more slabs can be created due to a limitation of the
    /// configuration for this [`SlabAllocator`].
    /// @error crate::slabs::IllegalArgument if the size specified does not exist in this slab
    /// allocator.
    /// @see SlabAllocator::free
    [slab_allocator]
    fn allocate(&mut self, size: usize) -> Result<u64, Error>;

    /// Write data to the specified slab.
    /// @param self a mutable reference to the [`SlabAllocator`] to write data to.
    /// @param id the identifier of the slab to write data to.
    /// @param data the data to write.
    /// @param offset the offset within the slab to write data at.
    /// @return n/a
    /// @error InvalidSlabId if the slab id is out of range for this [`SlabAllocator`].
    /// @see SlabAllocator::read
    [slab_allocator]
    fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error>;

    /// Read data from the specified slab.
    /// @param self an immutable reference to the [`SlabAllocator`] to read data from.
    /// @param id the identifier of the slab to read data from.
    /// @return an immutable reference to an array of bytes which references the data in
    /// the specified slab.
    /// @error InvalidSlabId if the slab id is out of range for this [`SlabAllocator`].
    /// @see SlabAllocator::write
    [slab_allocator]
    fn read(&self, id: u64) -> Result<&[u8], Error>;

    /// Free the specified slab so that the [`SlabAllocator`] can use it again.
    /// @param self a mutable reference to the [`SlabAllocator`] to free from.
    /// @param id the identifier of the slab to free.
    /// @return n/a
    /// @error InvalidSlabId if the slab id is out of range for this [`SlabAllocator`].
    /// @error DoubleFree if a slab that has already been freed is freed once again.
    /// @see SlabAllocator::write
    /// @see SlabAllocator::read
    [slab_allocator]
    fn free(&mut self, id: u64) -> Result<(), Error>;

    /// Returns a [`std::vec::Vec`] of [`SlabStats`]. One Stats is returned per slab_configs
    /// specified at startup.
    /// @param self an immutable reference to the [`SlabAllocator`] to retreive stats from.
    /// @return a [`std::vec::Vec`] of [`SlabStats`].
    /// @error - this function currently doesn't return errors.
    /// @see SlabStats
    [slab_allocator]
    fn stats(&self) -> Result<Vec<SlabStats>, Error>;

    /// Returns the id for this slab allocator.
    [slab_allocator]
    fn id(&self) -> u128;
}]
impl SlabAllocatorClass {}

#[cfg(test)]
pub(crate) use slab_allocator_sync_box;

impl SlabAllocatorClassVarBuilder for SlabAllocatorClassVar {
	fn builder(constants: &SlabAllocatorClassConst) -> Result<Self, Error> {
		if constants.slab_config.len() > u8::MAX as usize {
			err!(Configuration, "no more than {} slab_configs", u8::MAX)
		} else if constants.slabs_per_resize == 0 {
			err!(Configuration, "SlabsPerResize must be greater than 0")
		} else {
			let slab_data = vec![];

			let id = random();
			let mut ret = Self { slab_data, id };
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
	fn id(&self) -> u128 {
		*self.vars().get_id()
	}

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
		let slab_data = &mut self.vars_mut().get_mut_slab_data();

		if index >= slab_data.len() {
			err!(InvalidSlabId, "invalid slab id")
		} else {
			debug!("free index = {}", index)?;
			let sdh = &mut slab_data[index];
			let id_relative: usize = try_into!(id_relative)?;

			// check if it's already free
			let cur_ptr = sdh.sd.read(
				id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size),
				sdh.sdp.ptr_size,
			)?;
			if cur_ptr != &sdh.sdp.invalid_ptr[0..sdh.sdp.ptr_size] {
				ret_err!(DoubleFree, "a slab that was already free was freed again")
			}

			debug!(
				"cur_ptr={:?},invalid={:?}",
				cur_ptr,
				&sdh.sdp.invalid_ptr[0..sdh.sdp.ptr_size]
			)?;

			let mut first_free_slice = [0u8; 8];
			usize_to_slice(
				try_into!(sdh.sdp.free_list_head)?,
				&mut first_free_slice[0..sdh.sdp.ptr_size],
			)?;

			sdh.sd.write(
				&first_free_slice[0..sdh.sdp.ptr_size],
				id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size),
			)?;

			sdh.sdp.free_list_head = try_into!(id_relative)?;
			debug!("update firstfree to {}", sdh.sdp.free_list_head)?;
			sdh.stats.cur_slabs -= 1;

			Ok(())
		}
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
			debug!("min={},mid={},max={}", min, mid, max)?;
			debug!("try mid = {}", slab_data[mid].sdp.slab_size)?;
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

							debug!(
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
		let slab_data = &self.vars().get_slab_data();

		if index >= slab_data.len() {
			err!(InvalidSlabId, "invalid slab id")
		} else {
			let sdh = &slab_data[index];
			let id_relative: usize = try_into!(id_relative)?;
			sdh.sd.read(
				sdh.sdp.ptr_size + (id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size)),
				sdh.sdp.slab_size,
			)
		}
	}

	fn write(&mut self, id: u64, data: &[u8], offset: usize) -> Result<(), Error> {
		let id_relative = id & !0xFF00000000000000;
		let index = id >> 56;
		let index: usize = try_into!(index)?;
		let slab_data = &mut self.vars_mut().get_mut_slab_data();

		if index >= slab_data.len() {
			err!(InvalidSlabId, "invalid slab id")
		} else {
			let sdh = &mut slab_data[index];
			let id_relative: usize = try_into!(id_relative)?;

			sdh.sd.write(
				data,
				sdh.sdp.ptr_size + (id_relative * (sdh.sdp.ptr_size + sdh.sdp.slab_size)) + offset,
			)?;
			Ok(())
		}
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
			sdh.sdp.free_list_head = slice_to_u64(&sdh.sd.read(offset, ptr_size)?)?;
			debug!("set free list head to {}", sdh.sdp.free_list_head)?;
			sdh.sd.write(&sdh.sdp.invalid_ptr[0..ptr_size], offset)?;

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
			slab_data.write(&next_bytes[0..ptr_size], offset_next)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bmw_core::rand::random;
	use std::collections::HashMap;

	debug!();

	#[test]
	fn test_sizes() -> Result<(), Error> {
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(200), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(300), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(400), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(500), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(600), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(700), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(800), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(900), SlabCount(30))?),
			SlabConfig(slab_config!(SlabSize(1_000), SlabCount(30))?),
			SlabsPerResize(10),
		)?;

		for i in 1..1003 {
			if i % 100 == 0 {
				assert!(sa.allocate(i).is_ok());
			} else {
				assert!(sa.allocate(i).is_err());
			}
		}
		Ok(())
	}

	#[test]
	fn test_allocate_free_read_write_stat() -> Result<(), Error> {
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(1_000))?),
			SlabsPerResize(10)
		)?;

		let rrr: usize = random();
		let mut r = rrr % 100;
		let mut hash = HashMap::new();
		let mut capacity = 10;
		for i in 0..r {
			let x: usize = random();
			let mut x_bytes = [0u8; 8];
			usize_to_slice(x, &mut x_bytes)?;
			let id = sa.allocate(100)?;
			info!("id={},random={}", id, x)?;
			sa.write(id, &x_bytes, 0)?;
			hash.insert(id, x);
			let stats = sa.stats()?;
			info!("stats={:?}", stats)?;
			assert_eq!(stats[0].cur_slabs, i + 1);
			assert_eq!(stats[0].cur_capacity, capacity);
			if i != r - 1 && stats[0].cur_slabs % 10 == 0 {
				capacity += 10;
			}
		}

		for (k, v) in &hash {
			let bytes = sa.read(*k)?;
			let read_v = slice_to_usize(&bytes[0..8])?;
			assert_eq!(read_v, *v);
			sa.free(*k)?;
			let stats = sa.stats()?;
			info!("stats={:?}", stats)?;
			r -= 1;
			assert_eq!(stats[0].cur_slabs, r);
			assert_eq!(stats[0].cur_capacity, capacity);
		}

		assert_eq!(r, 0);

		let mut r = rrr % 100;
		for i in 0..r {
			let x: usize = random();
			let mut x_bytes = [0u8; 8];
			usize_to_slice(x, &mut x_bytes)?;
			let id = sa.allocate(100)?;
			info!("id={},random={}", id, x)?;
			sa.write(id, &x_bytes, 0)?;
			hash.insert(id, x);
			let stats = sa.stats()?;
			info!("stats={:?}", stats)?;
			assert_eq!(stats[0].cur_slabs, i + 1);
			assert_eq!(stats[0].cur_capacity, capacity);
		}

		for (k, v) in hash {
			let bytes = sa.read(k)?;
			let read_v = slice_to_usize(&bytes[0..8])?;
			assert_eq!(read_v, v);
			sa.free(k)?;
			let stats = sa.stats()?;
			info!("stats={:?}", stats)?;
			r -= 1;
			assert_eq!(stats[0].cur_slabs, r);
			assert_eq!(stats[0].cur_capacity, capacity);
		}

		assert_eq!(r, 0);

		Ok(())
	}

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
		slab_data.write(&[0, 1, 2, 3], 10)?;
		assert_eq!(slab_data.read(10, 4)?, [0, 1, 2, 3]);
		assert_eq!(slab_data.read(0, 4)?, [0, 0, 0, 0]);
		assert!(slab_data.read(0, 100).is_ok());
		assert!(slab_data.read(0, 101).is_err());
		assert!(slab_data.read(1, 99).is_ok());
		assert!(slab_data.read(1, 100).is_err());

		slab_data.resize(90)?;
		assert!(slab_data.read(1, 89).is_ok());
		assert!(slab_data.read(1, 90).is_err());

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

	#[test]
	fn test_slabs_per_resize_1() -> Result<(), Error> {
		let mut sa = slab_allocator!(
			SlabConfig(slab_config!(SlabSize(100), SlabCount(10))?),
			SlabsPerResize(1),
		)?;

		for _ in 0..10 {
			let ret = sa.allocate(100)?;
			let data = [1u8; 100];
			sa.write(ret, &data, 0)?;
			info!("re={}", ret)?;
		}

		info!("last alloc")?;
		assert_eq!(sa.allocate(100).unwrap_err().kind(), kind!(OutOfSlabs));

		Ok(())
	}

	#[test]
	fn test_double_free() -> Result<(), Error> {
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

		for id in &id_vec {
			sa.free(*id)?;
		}

		assert_eq!(sa.free(id_vec[0]).unwrap_err().kind(), kind!(DoubleFree));
		Ok(())
	}
}
