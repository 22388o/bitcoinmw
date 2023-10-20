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

use crate::misc::set_max;
use crate::misc::{slice_to_usize, usize_to_slice};
use crate::ConfigOption::*;
use crate::{
	Array, ArrayList, BinReader, BinWriter, Builder, ConfigOption, Hashset, HashsetConfig,
	Hashtable, HashtableConfig, List, ListConfig, Reader, Serializable, SlabAllocator,
	SlabAllocatorConfig, SlabMut, SlabReader, SlabWriter, SortableList, Writer,
	GLOBAL_SLAB_ALLOCATOR,
};
use bmw_err::{err, ErrKind, Error};
use bmw_log::*;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;
use std::hash::Hash;
use std::io::{Read, Write};
use std::rc::Rc;
use std::thread;

info!();

impl Serializable for ConfigOption<'_> {
	// fully covered, but the wildcard match not counted by tarpaulin
	#[cfg(not(tarpaulin_include))]
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		match reader.read_u8()? {
			0 => Ok(MaxEntries(reader.read_usize()?)),
			1 => Ok(MaxLoadFactor(f64::read(reader)?)),
			2 => Ok(SlabSize(reader.read_usize()?)),
			3 => Ok(SlabCount(reader.read_usize()?)),
			4 => Ok(MinSize(reader.read_usize()?)),
			5 => Ok(MaxSize(reader.read_usize()?)),
			6 => Ok(SyncChannelSize(reader.read_usize()?)),
			_ => {
				let fmt = "invalid type for config option!";
				let e = err!(ErrKind::CorruptedData, fmt);
				Err(e)
			}
		}
	}

	// fully covered but the Slabs(_) line not counted by tarpaulin
	#[cfg(not(tarpaulin_include))]
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		match self {
			MaxEntries(size) => {
				writer.write_u8(0)?;
				writer.write_usize(*size)?;
			}
			MaxLoadFactor(lf) => {
				writer.write_u8(1)?;
				f64::write(lf, writer)?;
			}
			SlabSize(ss) => {
				writer.write_u8(2)?;
				writer.write_usize(*ss)?;
			}
			SlabCount(sc) => {
				writer.write_u8(3)?;
				writer.write_usize(*sc)?;
			}
			MinSize(mins) => {
				writer.write_u8(4)?;
				writer.write_usize(*mins)?;
			}
			MaxSize(maxs) => {
				writer.write_u8(5)?;
				writer.write_usize(*maxs)?;
			}
			SyncChannelSize(scs) => {
				writer.write_u8(6)?;
				writer.write_usize(*scs)?;
			}
			Slabs(_) => {
				let fmt = "can't serialize slab allocator";
				let e = err!(ErrKind::OperationNotSupported, fmt);
				return Err(e);
			}
		}
		Ok(())
	}
}

impl<S: Serializable + Clone> Serializable for Array<S> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.size();
		writer.write_usize(len)?;
		for i in 0..len {
			Serializable::write(&self[i], writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Array<S>, Error> {
		let len = reader.read_usize()?;
		let mut a: Option<Array<S>> = None;
		for i in 0..len {
			let s = Serializable::read(reader)?;
			if i == 0 {
				a = Some(Builder::build_array(len, &s)?);
			}
			a.as_mut().unwrap()[i] = s;
		}

		if a.is_none() {
			let e = err!(ErrKind::CorruptedData, "size of array cannot be 0");
			return Err(e);
		}

		Ok(a.unwrap())
	}
}

impl<S: Serializable + PartialEq + Debug + Clone + 'static> Serializable
	for Box<dyn SortableList<S>>
{
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.size();
		writer.write_usize(len)?;
		for x in self.iter() {
			Serializable::write(&x, writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let len = reader.read_usize()?;
		let mut list = Builder::build_list_box(ListConfig::default(), &None)?;
		for _ in 0..len {
			list.push(Serializable::read(reader)?)?;
		}
		Ok(list)
	}
}

impl<K, V> Serializable for Box<dyn Hashtable<K, V>>
where
	K: Serializable + Clone + Debug + PartialEq + Hash + 'static,
	V: Serializable + Clone,
{
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_usize(self.max_entries())?;
		self.max_load_factor().write(writer)?;
		let len = self.size();
		writer.write_usize(len)?;
		for (k, v) in self.iter() {
			Serializable::write(&k, writer)?;
			Serializable::write(&v, writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let max_entries = reader.read_usize()?;
		let max_load_factor = f64::read(reader)?;
		let len = reader.read_usize()?;
		let config = HashtableConfig {
			max_entries,
			max_load_factor,
			..Default::default()
		};
		let mut hashtable = Builder::build_hashtable_box(config, &None)?;
		for _ in 0..len {
			let k: K = Serializable::read(reader)?;
			let v: V = Serializable::read(reader)?;
			hashtable.insert(&k, &v)?;
		}
		Ok(hashtable)
	}
}

impl<K> Serializable for Box<dyn Hashset<K>>
where
	K: Serializable + Clone + Debug + PartialEq + Hash + 'static,
{
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_usize(self.max_entries())?;
		self.max_load_factor().write(writer)?;
		let len = self.size();
		writer.write_usize(len)?;
		for k in self.iter() {
			Serializable::write(&k, writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let max_entries = reader.read_usize()?;
		let max_load_factor = f64::read(reader)?;
		let len = reader.read_usize()?;
		let config = HashsetConfig {
			max_entries,
			max_load_factor,
			..Default::default()
		};
		let mut hashset = Builder::build_hashset_box(config, &None)?;
		for _ in 0..len {
			let k: K = Serializable::read(reader)?;
			hashset.insert(&k)?;
		}
		Ok(hashset)
	}
}

impl<S: Serializable + Clone + Debug + PartialEq> Serializable for ArrayList<S> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.inner.size();
		writer.write_usize(len)?;
		for x in self.inner.iter() {
			Serializable::write(&x, writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<ArrayList<S>, Error> {
		let len = reader.read_usize()?;
		let mut a: Option<ArrayList<S>> = None;
		for i in 0..len {
			let s = Serializable::read(reader)?;
			if i == 0 {
				a = Some(ArrayList::new(len, &s)?);
			}
			a.as_mut().unwrap().push(s)?;
		}
		Ok(a.unwrap())
	}
}

impl SlabWriter {
	pub fn new(
		slabs: Option<Rc<RefCell<dyn SlabAllocator>>>,
		slab_id: usize,
		slab_ptr_size: Option<usize>,
	) -> Result<Self, Error> {
		debug!("new with slab_id = {}", slab_id)?;
		let (slab_size, slab_count) = match slabs {
			Some(ref slabs) => {
				let slabs: Ref<_> = slabs.borrow();
				(slabs.slab_size()?, slabs.slab_count()?)
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(usize, usize), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let slab_size = match slabs.is_init() {
					true => slabs.slab_size()?,
					false => {
						let th = thread::current();
						let n = th.name().unwrap_or("unknown");
						warn!(
							"Slab allocator was not initialized for thread '{}'. {}",
							n, "Initializing with default values.",
						)?;
						slabs.init(SlabAllocatorConfig::default())?;
						slabs.slab_size()?
					}
				};
				let slab_count = slabs.slab_count()?;
				Ok((slab_size, slab_count))
			})?,
		};

		let slab_ptr_size = match slab_ptr_size {
			Some(s) => s,
			None => {
				let mut x = slab_count;
				let mut ptr_size = 0;
				loop {
					if x == 0 {
						break;
					}
					x >>= 8;
					ptr_size += 1;
				}
				ptr_size
			}
		};
		debug!("slab_ptr_size={}", slab_ptr_size)?;
		let bytes_per_slab = slab_size.saturating_sub(slab_ptr_size);

		let ret = Self {
			slabs,
			slab_id,
			offset: 0,
			slab_size,
			bytes_per_slab,
		};

		Ok(ret)
	}

	/// go to a particular slab_id/offset within the [`crate::SlabAllocator`] associated with
	/// this [`crate::SlabWriter`].
	pub fn seek(&mut self, slab_id: usize, offset: usize) {
		self.slab_id = slab_id;
		self.offset = offset;
	}

	pub fn skip_bytes(&mut self, count: usize) -> Result<(), Error> {
		match self.slabs.clone() {
			Some(slabs) => {
				let slabs = slabs.borrow_mut();
				self.do_write_fixed_bytes_impl(&[0u8; 0], Some(count), Some(slabs))
			}
			None => self.do_write_fixed_bytes_impl(&[0u8; 0], Some(count), None),
		}
	}

	// appears to be 100% covered. Tarpaulin reprots a few lines.
	#[cfg(not(tarpaulin_include))]
	pub(crate) fn write_fixed_bytes_impl<T: AsRef<[u8]>>(
		&mut self,
		bytes: T,
		slabs: Option<RefMut<dyn SlabAllocator>>,
	) -> Result<(), Error> {
		self.do_write_fixed_bytes_impl(bytes, None, slabs)
	}

	fn process_write(
		&mut self,
		bytes: &[u8],
		bytes_len: usize,
		skip: bool,
		mut slabs: &mut Option<RefMut<dyn SlabAllocator>>,
	) -> Result<usize, Error> {
		let mut wlen = bytes_len;
		let space_left_in_slab = self.bytes_per_slab.saturating_sub(self.offset);

		if wlen > space_left_in_slab {
			wlen = space_left_in_slab;
		}

		if !skip {
			match &mut slabs {
				Some(slabs) => {
					let mut slab_mut = slabs.get_mut(self.slab_id)?;

					debug!(
						"bytes.len={},self.offset={},wlen={},slab_size={}",
						bytes.len(),
						self.offset,
						wlen,
						self.slab_size,
					)?;
					slab_mut.get_mut()[self.offset..self.offset + wlen]
						.clone_from_slice(&bytes[0..wlen]);
				}
				None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					let mut slab_mut = slabs.get_mut(self.slab_id)?;
					slab_mut.get_mut()[self.offset..self.offset + wlen]
						.clone_from_slice(&bytes[0..wlen]);
					Ok(())
				})?,
			}
		}

		Ok(wlen)
	}

	fn do_write_fixed_bytes_impl<T: AsRef<[u8]>>(
		&mut self,
		bytes: T,
		count: Option<usize>,
		mut slabs: Option<RefMut<dyn SlabAllocator>>,
	) -> Result<(), Error> {
		let bytes = bytes.as_ref();
		let bytes_len = match count {
			Some(count) => count,
			None => bytes.len(),
		};
		let skip = count.is_some();

		debug!(
			"+++++++++++++++++ do_write_fixed_bytes_impl bytes={:?},count={:?},slab_id={},offset={}",
			bytes, count, self.slab_id, self.offset
		)?;

		if bytes_len == 0 {
			return Ok(());
		}

		let mut bytes_offset = 0;

		let mut ptr = [0u8; 8];
		let slab_ptr_size = self.slab_size.saturating_sub(self.bytes_per_slab);
		set_max(&mut ptr[0..slab_ptr_size]);
		let max_value = slice_to_usize(&ptr[0..slab_ptr_size])?;

		loop {
			debug!(
				"loop with bytes_offset={},bytes_len={},self.offset={},slab_id={},self.bytes_per_slab={},slab_size={}",
				bytes_offset, bytes_len, self.offset, self.slab_id, self.bytes_per_slab, self.slab_size,
			)?;
			if bytes_offset >= bytes_len {
				break;
			}

			if self.offset >= self.bytes_per_slab {
				debug!(
					"need to allocate a new slab bytes_offset={}, bytes_len={}",
					bytes_offset, bytes_len
				)?;
				// we need to allocate another slab
				self.next_slab(&mut slabs, max_value)?;
			}

			let index = if skip { 0 } else { bytes_offset };

			let wlen = self.process_write(
				&bytes[index..],
				bytes_len.saturating_sub(bytes_offset),
				skip,
				&mut slabs,
			)?;
			debug!("wlen = {}", wlen)?;
			bytes_offset += wlen;
			self.offset += wlen;
		}

		Ok(())
	}

	fn read_next(&mut self, cur_slab: SlabMut) -> Result<usize, Error> {
		let next = slice_to_usize(&cur_slab.get()[self.bytes_per_slab..self.slab_size])?;
		Ok(next)
	}

	fn next_slab(
		&mut self,
		mut slabs: &mut Option<RefMut<dyn SlabAllocator>>,
		max_value: usize,
	) -> Result<(), Error> {
		self.offset = 0;

		let next = match &mut slabs {
			Some(slabs) => self.read_next(slabs.get_mut(self.slab_id)?)?,
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				self.read_next(slabs.get_mut(self.slab_id)?)
			})?,
		};

		if next == max_value {
			let new_id = match &mut slabs {
				Some(slabs) => {
					let mut nslab = slabs.allocate()?;
					let nslab_mut = nslab.get_mut();
					for i in self.bytes_per_slab..self.slab_size {
						nslab_mut[i] = 0xFF;
					}
					nslab.id()
				}
				None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					let mut nslab = slabs.allocate()?;
					let nslab_mut = nslab.get_mut();
					for i in self.bytes_per_slab..self.slab_size {
						nslab_mut[i] = 0xFF;
					}

					Ok(nslab.id())
				})?,
			};

			match &mut slabs {
				Some(slabs) => {
					let mut slab = slabs.get_mut(self.slab_id)?;
					let prev = &mut slab.get_mut()[self.bytes_per_slab..self.slab_size];
					debug!("writing pointer to {} -> {}", self.slab_id, new_id)?;
					usize_to_slice(new_id, prev)?;
				}
				None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					let mut slab = slabs.get_mut(self.slab_id)?;
					let prev = &mut slab.get_mut()[self.bytes_per_slab..self.slab_size];
					debug!("writing pointer to {} -> {}", self.slab_id, new_id)?;
					usize_to_slice(new_id, prev)?;
					Ok(())
				})?,
			}

			self.slab_id = new_id;
		} else {
			self.slab_id = next;
		}

		Ok(())
	}
}

impl Writer for SlabWriter {
	fn write_fixed_bytes<'a, T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		match &self.slabs {
			Some(slabs) => {
				let slabs = slabs.clone();
				let slabs: RefMut<_> = slabs.borrow_mut();
				self.write_fixed_bytes_impl(bytes, Some(slabs))
			}
			None => self.write_fixed_bytes_impl(bytes, None),
		}
	}
}

impl<'a> SlabReader {
	// only the break line is reported as uncovered, but it is covered
	#[cfg(not(tarpaulin_include))]
	pub fn new(
		slabs: Option<Rc<RefCell<dyn SlabAllocator>>>,
		slab_id: usize,
		slab_ptr_size: Option<usize>,
	) -> Result<Self, Error> {
		let (slab_size, slab_count) = match slabs.as_ref() {
			Some(slabs) => {
				let slabs: Ref<_> = slabs.borrow();
				(slabs.slab_size()?, slabs.slab_count()?)
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(usize, usize), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let slab_size = match slabs.is_init() {
					true => slabs.slab_size()?,
					false => {
						let th = thread::current();
						let n = th.name().unwrap_or("unknown");
						let m = "Initializing with default values.";
						warn!("Allocator was not initialized for thread '{}'. {}", n, m)?;
						slabs.init(SlabAllocatorConfig::default())?;
						slabs.slab_size()?
					}
				};
				let slab_count = slabs.slab_count()?;
				Ok((slab_size, slab_count))
			})?,
		};

		let slab_ptr_size = match slab_ptr_size {
			Some(s) => s,
			None => {
				let mut x = slab_count;
				let mut ptr_size = 0;
				loop {
					if x == 0 {
						break;
					}
					x >>= 8;
					ptr_size += 1;
				}
				ptr_size
			}
		};
		let bytes_per_slab = slab_size.saturating_sub(slab_ptr_size);

		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..slab_ptr_size]);
		let max_value = slice_to_usize(&ptr[0..slab_ptr_size])?;

		let ret = Self {
			slabs,
			slab_id,
			offset: 0,
			slab_size,
			bytes_per_slab,
			max_value,
		};
		Ok(ret)
	}

	/// go to a particular slab_id/offset within the [`crate::SlabAllocator`] associated with
	/// this [`crate::SlabReader`].
	pub fn seek(&mut self, slab_id: usize, offset: usize) {
		self.slab_id = slab_id;
		self.offset = offset;
	}

	fn get_next_id(&self, id: usize) -> Result<usize, Error> {
		let bytes_per_slab = self.bytes_per_slab;
		let slab_size = self.slab_size;
		match &self.slabs {
			Some(slabs) => {
				let slabs: Ref<_> = slabs.borrow();
				let slab = slabs.get(id)?;
				Ok(slice_to_usize(&slab.get()[bytes_per_slab..slab_size])?)
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let slab = slabs.get(id)?;
				Ok(slice_to_usize(&slab.get()[bytes_per_slab..slab_size])?)
			}),
		}
	}

	fn read_bytes(
		&self,
		id: usize,
		offset: usize,
		rlen: usize,
		buf: &mut [u8],
		skip: bool,
	) -> Result<(), Error> {
		match &self.slabs {
			Some(slabs) => {
				let slabs: Ref<_> = slabs.borrow();
				let slab = slabs.get(id)?;
				if !skip {
					buf.clone_from_slice(&slab.get()[offset..(offset + rlen)]);
				}
				Ok(())
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
				let slabs = unsafe { f.get().as_ref().unwrap() };
				let slab = slabs.get(id)?;
				if !skip {
					buf.clone_from_slice(&slab.get()[offset..(offset + rlen)]);
				}
				Ok(())
			}),
		}
	}

	pub fn skip_bytes(&mut self, count: usize) -> Result<(), Error> {
		self.do_read_exact(&mut [0u8; 0], Some(count))
	}

	// only the break line is reported as uncovered, but it is covered
	#[cfg(not(tarpaulin_include))]
	pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.do_read_exact(buf, None)
	}

	pub fn do_read_exact(&mut self, buf: &mut [u8], count: Option<usize>) -> Result<(), Error> {
		debug!("do read exact {:?},buf.len={}", count, buf.len())?;
		let mut buf_offset = 0;
		let buf_len = match count {
			Some(count) => count,
			None => buf.len(),
		};
		debug!("buflen={}", buf_len)?;
		loop {
			if buf_offset >= buf_len {
				break;
			}
			let buf_rem = buf_len - buf_offset;

			if self.offset >= self.bytes_per_slab {
				self.offset = 0;
				let next = self.get_next_id(self.slab_id)?;
				if next >= self.max_value {
					return Err(err!(
						ErrKind::IO,
						format!(
							"failed to fill whole buffer next={}, self.max_value={}",
							next, self.max_value
						)
					));
				}
				self.slab_id = next;
			}

			let mut rlen = self.bytes_per_slab - self.offset;
			if rlen > buf_rem {
				rlen = buf_rem;
			}

			debug!("read exact rln={}", rlen)?;

			let b = if count.is_some() {
				&mut [0; 0usize]
			} else {
				&mut buf[buf_offset..buf_offset + rlen]
			};

			self.read_bytes(self.slab_id, self.offset, rlen, b, count.is_some())?;
			debug!("buf[first]={}", buf[buf_offset])?;
			buf_offset += rlen;
			debug!("buf_offset={}", buf_offset)?;
			self.offset += rlen;
		}

		Ok(())
	}
}

impl Reader for SlabReader {
	fn read_u8(&mut self) -> Result<u8, Error> {
		let mut b = [0u8; 1];
		self.read_exact(&mut b)?;
		Ok(b[0])
	}
	fn read_i8(&mut self) -> Result<i8, Error> {
		let mut b = [0u8; 1];
		self.read_exact(&mut b)?;
		Ok(b[0] as i8)
	}
	fn read_i16(&mut self) -> Result<i16, Error> {
		let mut b = [0u8; 2];
		self.read_exact(&mut b)?;
		Ok(i16::from_be_bytes(b))
	}
	fn read_u16(&mut self) -> Result<u16, Error> {
		let mut b = [0u8; 2];
		self.read_exact(&mut b)?;
		Ok(u16::from_be_bytes(b))
	}
	fn read_u32(&mut self) -> Result<u32, Error> {
		let mut b = [0u8; 4];
		self.read_exact(&mut b)?;
		Ok(u32::from_be_bytes(b))
	}
	fn read_i32(&mut self) -> Result<i32, Error> {
		let mut b = [0u8; 4];
		self.read_exact(&mut b)?;
		Ok(i32::from_be_bytes(b))
	}
	fn read_u64(&mut self) -> Result<u64, Error> {
		let mut b = [0u8; 8];
		self.read_exact(&mut b)?;
		Ok(u64::from_be_bytes(b))
	}
	fn read_i128(&mut self) -> Result<i128, Error> {
		let mut b = [0u8; 16];
		self.read_exact(&mut b)?;
		Ok(i128::from_be_bytes(b))
	}
	fn read_usize(&mut self) -> Result<usize, Error> {
		let mut b = [0u8; 8];
		self.read_exact(&mut b)?;
		Ok(usize::from_be_bytes(b))
	}

	fn read_u128(&mut self) -> Result<u128, Error> {
		let mut b = [0u8; 16];
		self.read_exact(&mut b)?;
		Ok(u128::from_be_bytes(b))
	}
	fn read_i64(&mut self) -> Result<i64, Error> {
		let mut b = [0u8; 8];
		self.read_exact(&mut b)?;
		Ok(i64::from_be_bytes(b))
	}

	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.read_exact(buf)?;
		Ok(())
	}

	fn expect_u8(&mut self, val: u8) -> Result<u8, Error> {
		let b = self.read_u8()?;
		if b == val {
			Ok(b)
		} else {
			let fmt = format!("expected: {:?}, received: {:?}", val, b);
			Err(err!(ErrKind::CorruptedData, fmt))
		}
	}
}

impl<'a> BinWriter<'a> {
	/// Wraps a standard Write in a new BinWriter
	pub fn new(sink: &'a mut dyn Write) -> BinWriter<'a> {
		BinWriter { sink }
	}
}

impl<'a> Writer for BinWriter<'a> {
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.sink.write_all(bytes.as_ref())?;
		Ok(())
	}
}

impl<'a, R: Read> BinReader<'a, R> {
	/// Constructor for a new BinReader for the provided source
	pub fn new(source: &'a mut R) -> Self {
		BinReader { source }
	}
}

impl<'a, R: Read> Reader for BinReader<'a, R> {
	fn read_u8(&mut self) -> Result<u8, Error> {
		let mut b = [0u8; 1];
		self.source.read_exact(&mut b)?;
		Ok(b[0])
	}
	fn read_i8(&mut self) -> Result<i8, Error> {
		let mut b = [0u8; 1];
		self.source.read_exact(&mut b)?;
		Ok(b[0] as i8)
	}
	fn read_i16(&mut self) -> Result<i16, Error> {
		let mut b = [0u8; 2];
		self.source.read_exact(&mut b)?;
		Ok(i16::from_be_bytes(b))
	}
	fn read_u16(&mut self) -> Result<u16, Error> {
		let mut b = [0u8; 2];
		self.source.read_exact(&mut b)?;
		Ok(u16::from_be_bytes(b))
	}
	fn read_u32(&mut self) -> Result<u32, Error> {
		let mut b = [0u8; 4];
		self.source.read_exact(&mut b)?;
		Ok(u32::from_be_bytes(b))
	}
	fn read_i32(&mut self) -> Result<i32, Error> {
		let mut b = [0u8; 4];
		self.source.read_exact(&mut b)?;
		Ok(i32::from_be_bytes(b))
	}
	fn read_u64(&mut self) -> Result<u64, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(u64::from_be_bytes(b))
	}
	fn read_i128(&mut self) -> Result<i128, Error> {
		let mut b = [0u8; 16];
		self.source.read_exact(&mut b)?;
		Ok(i128::from_be_bytes(b))
	}
	fn read_usize(&mut self) -> Result<usize, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(usize::from_be_bytes(b))
	}

	fn read_u128(&mut self) -> Result<u128, Error> {
		let mut b = [0u8; 16];
		self.source.read_exact(&mut b)?;
		Ok(u128::from_be_bytes(b))
	}
	fn read_i64(&mut self) -> Result<i64, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(i64::from_be_bytes(b))
	}

	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.source.read_exact(buf)?;
		Ok(())
	}

	fn expect_u8(&mut self, val: u8) -> Result<u8, Error> {
		let b = self.read_u8()?;
		if b == val {
			Ok(b)
		} else {
			let fmt = format!("expected: {:?}, received: {:?}", val, b);
			Err(err!(ErrKind::CorruptedData, fmt))
		}
	}
}

/// Serializes a Serializable into any std::io::Write implementation.
pub fn serialize<W: Serializable>(sink: &mut dyn Write, thing: &W) -> Result<(), Error> {
	let mut writer = BinWriter::new(sink);
	thing.write(&mut writer)
}

/// Deserializes a Serializable from any std::io::Read implementation.
pub fn deserialize<T: Serializable, R: Read>(source: &mut R) -> Result<T, Error> {
	let mut reader = BinReader::new(source);
	T::read(&mut reader)
}

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::ser::{SlabReader, SlabWriter};
	use crate::Builder;
	use crate::*;
	use bmw_deps::rand;
	use bmw_err::*;
	use bmw_log::*;
	use std::cell::{RefCell, RefMut};
	use std::fmt::Debug;
	use std::rc::Rc;

	info!();

	#[derive(Debug, PartialEq)]
	struct SerErr {
		exp: u8,
		empty: u8,
	}

	impl Serializable for SerErr {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			reader.expect_u8(99)?;
			reader.read_empty_bytes(1)?;
			Ok(Self { exp: 99, empty: 0 })
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			writer.write_u8(self.exp)?;
			writer.write_u8(self.empty)?;
			Ok(())
		}
	}

	#[derive(Debug, PartialEq)]
	struct SerAll {
		a: u8,
		b: i8,
		c: u16,
		d: i16,
		e: u32,
		f: i32,
		g: u64,
		h: i64,
		i: u128,
		j: i128,
		k: usize,
	}

	impl Serializable for SerAll {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			let a = reader.read_u8()?;
			let b = reader.read_i8()?;
			let c = reader.read_u16()?;
			let d = reader.read_i16()?;
			let e = reader.read_u32()?;
			let f = reader.read_i32()?;
			let g = reader.read_u64()?;
			let h = reader.read_i64()?;
			let i = reader.read_u128()?;
			let j = reader.read_i128()?;
			let k = reader.read_usize()?;
			reader.expect_u8(100)?;
			assert_eq!(reader.read_u64()?, 4);
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_empty_bytes(10)?;

			let ret = Self {
				a,
				b,
				c,
				d,
				e,
				f,
				g,
				h,
				i,
				j,
				k,
			};

			Ok(ret)
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			writer.write_u8(self.a)?;
			writer.write_i8(self.b)?;
			writer.write_u16(self.c)?;
			writer.write_i16(self.d)?;
			writer.write_u32(self.e)?;
			writer.write_i32(self.f)?;
			writer.write_u64(self.g)?;
			writer.write_i64(self.h)?;
			writer.write_u128(self.i)?;
			writer.write_i128(self.j)?;
			writer.write_usize(self.k)?;
			writer.write_u8(100)?;
			writer.write_bytes([1, 2, 3, 4])?;
			writer.write_empty_bytes(10)?;
			Ok(())
		}
	}

	fn ser_helper<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: S = deserialize(&mut &v[..])?;
		assert_eq!(ser_in, ser_out);
		Ok(())
	}

	fn ser_helper_slabs<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut slab_writer = SlabWriter::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_writer.seek(slab.id(), 0);
		ser_out.write(&mut slab_writer)?;
		let mut slab_reader = SlabReader::new(None, slab.id(), None)?;
		slab_reader.seek(slab.id(), 0);
		let ser_in = S::read(&mut slab_reader)?;
		assert_eq!(ser_in, ser_out);

		Ok(())
	}

	#[test]
	fn test_skip_bytes() -> Result<(), Error> {
		let slabs = slab_allocator(1024, 10_240)?;

		let slab_id = {
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			let slab = slabs.allocate()?;
			slab.id()
		};

		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		slab_writer.skip_bytes(800)?;
		slab_writer.write_u128(123)?;

		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		slab_reader.skip_bytes(800)?;
		assert_eq!(slab_reader.read_u128()?, 123);
		Ok(())
	}

	#[test]
	fn test_serialization_slab_rw() -> Result<(), Error> {
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
		};

		ser_helper_slabs(ser_out)?;

		let ser_err = SerErr { exp: 100, empty: 0 };

		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		let mut slab_writer = SlabWriter::new(None, slab.id(), None)?;
		slab_writer.seek(slab.id(), 0);
		ser_err.write(&mut slab_writer)?;
		let mut slab_reader = SlabReader::new(None, slab.id(), None)?;
		slab_reader.seek(slab.id(), 0);
		assert!(SerErr::read(&mut slab_reader).is_err());

		Ok(())
	}

	#[test]
	fn test_serialization() -> Result<(), Error> {
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
		};
		ser_helper(ser_out)?;
		ser_helper(())?;
		ser_helper((rand::random::<u32>(), rand::random::<i128>()))?;
		ser_helper(("hi there".to_string(), 123))?;
		let x = [3u8; 8];
		ser_helper(x)?;

		let ser_out = SerErr { exp: 100, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let ser_out = SerErr { exp: 99, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_ok());

		let ser_out = SerErr { exp: 99, empty: 1 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let v = vec!["test1".to_string(), "a".to_string(), "okokok".to_string()];
		ser_helper(v)?;

		let mut hashtable = hashtable_box!(MaxEntries(123), MaxLoadFactor(0.5))?;
		hashtable.insert(&1, &2)?;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &hashtable)?;
		let ser_in: Box<dyn Hashtable<u32, u32>> = deserialize(&mut &v[..])?;
		assert_eq!(ser_in.max_entries(), hashtable.max_entries());
		assert_eq!(ser_in.max_load_factor(), hashtable.max_load_factor());
		assert_eq!(ser_in.get(&1)?, Some(2));

		let mut hashset = hashset_box!(MaxEntries(23), MaxLoadFactor(0.54))?;
		hashset.insert(&1)?;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &hashset)?;
		let ser_in: Box<dyn Hashset<u32>> = deserialize(&mut &v[..])?;
		assert_eq!(ser_in.max_entries(), hashset.max_entries());
		assert_eq!(ser_in.max_load_factor(), hashset.max_load_factor());
		assert!(ser_in.contains(&1)?);

		Ok(())
	}

	fn slab_allocator(
		slab_size: usize,
		slab_count: usize,
	) -> Result<Rc<RefCell<dyn SlabAllocator>>, Error> {
		let config = bmw_util::SlabAllocatorConfig {
			slab_count,
			slab_size,
			..Default::default()
		};
		let slabs = Builder::build_slabs_ref();

		{
			let mut slabs_refmut: RefMut<_> = slabs.borrow_mut();

			slabs_refmut.init(config).unwrap();
		}

		Ok(slabs)
	}

	#[test]
	fn test_slab_rw() -> Result<(), Error> {
		let slabs = slab_allocator(1024, 10_240)?;

		let slab_id = {
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			let slab = slabs.allocate()?;
			slab.id()
		};

		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		slab_writer.write_u64(123)?;
		slab_writer.write_u128(123)?;

		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		assert_eq!(slab_reader.read_u64()?, 123);
		assert_eq!(slab_reader.read_u128()?, 123);

		Ok(())
	}

	#[test]
	fn test_multi_slabs() -> Result<(), Error> {
		let slabs = slab_allocator(1024, 10_240)?;
		let slab_id = {
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			let mut slab = slabs.allocate()?;
			let slab_mut = slab.get_mut();
			for j in 0..1024 {
				slab_mut[j] = 0xFF;
			}

			slab.id()
		};
		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		let r = 10_100;
		for i in 0..r {
			slab_writer.write_u128(i)?;
		}
		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		for i in 0..r {
			assert_eq!(slab_reader.read_u128()?, i);
		}

		let mut v = vec![];
		v.resize(1024 * 2, 0u8);
		// we can't read anymore
		assert!(slab_reader.read_fixed_bytes(&mut v).is_err());

		Ok(())
	}

	#[test]
	fn test_global_multi_slabs() -> Result<(), Error> {
		global_slab_allocator!()?;
		let slab_id = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			let slabs = unsafe { f.get().as_mut().unwrap() };
			let mut slab = slabs.allocate()?;
			let slab_mut = slab.get_mut();
			for j in 0..slab_mut.len() {
				slab_mut[j] = 0xFF;
			}
			Ok(slab.id())
		})?;
		let mut slab_writer = SlabWriter::new(None, slab_id, None)?;
		let r = 10_100;
		for i in 0..r {
			slab_writer.write_u128(i)?;
		}
		let mut slab_reader = SlabReader::new(None, slab_id, None)?;
		for i in 0..r {
			assert_eq!(slab_reader.read_u128()?, i);
		}

		let mut v = vec![];
		v.resize(1024 * 2, 0u8);
		// we can't read anymore
		assert!(slab_reader.read_fixed_bytes(&mut v).is_err());

		Ok(())
	}

	#[test]
	fn test_alternate_sized_slabs() -> Result<(), Error> {
		for i in 0..1000 {
			let slabs = slab_allocator(48 + i, 10)?;

			let slab_id = {
				let mut slabs: RefMut<_> = slabs.borrow_mut();
				let mut slab = slabs.allocate()?;
				let slab_mut = slab.get_mut();
				for j in 0..slab_mut.len() {
					slab_mut[j] = 0xFF;
				}

				slab.id()
			};
			let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;

			let mut v = [0u8; 256];
			for i in 0..v.len() {
				v[i] = (i % 256) as u8;
			}
			slab_writer.write_fixed_bytes(v)?;

			let mut slab_reader = SlabReader::new(Some(slabs), slab_id, None)?;
			let mut v_back = [1u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);
		}
		// test capacity exceeded

		// 470 is ok because only 1 byte overhead per slab.
		let slabs = slab_allocator(48, 10)?;
		let slab_id = {
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			let slab = slabs.allocate()?;
			slab.id()
		};
		let mut slab_writer = SlabWriter::new(Some(slabs), slab_id, None)?;
		let mut v = [0u8; 470];
		for i in 0..v.len() {
			v[i] = (i % 256) as u8;
		}
		assert!(slab_writer.write_fixed_bytes(v).is_ok());

		// 471 is one too many and returns error (note: user responsible for cleanup)
		let slabs = slab_allocator(48, 10)?;
		let _slab_id = {
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			let slab = slabs.allocate()?;
			slab.id()
		};
		//let mut slab_writer = SlabWriter::new(Some(slabs), slab_id, None)?;
		let mut v = [0u8; 471];
		for i in 0..v.len() {
			v[i] = (i % 256) as u8;
		}
		// since raw_write this is not an error it just adds on.
		//assert!(slab_writer.write_fixed_bytes(v).is_err());

		Ok(())
	}

	#[test]
	fn slab_writer_out_of_slabs() -> Result<(), Error> {
		global_slab_allocator!(SlabSize(100), SlabCount(1))?;
		let free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count1)?;

		{
			let slabid = {
				let mut slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					slabs.allocate()
				})?;
				let slab_mut = slab.get_mut();
				slab_mut[99] = 0xFF; // set next to 0xFF
				slab.id()
			};
			let mut writer = SlabWriter::new(None, slabid, None)?;
			let mut v = vec![];
			for _ in 0..200 {
				v.push(1);
			}
			assert!(writer.write_fixed_bytes(v).is_err());

			// user responsible for freeing the chain
			GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				slabs.free(slabid)?;
				Ok(())
			})?;
		}
		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count2)?;
		assert_eq!(free_count1, free_count2);

		Ok(())
	}

	#[test]
	fn test_seek() -> Result<(), Error> {
		for i in 0..1000 {
			let slabs = slab_allocator(48 + i, 10)?;

			let slab_id = {
				let mut slabs: RefMut<_> = slabs.borrow_mut();
				let mut slab = slabs.allocate()?;
				let slab_mut = slab.get_mut();
				for j in 0..48 + i {
					slab_mut[j] = 0xFF;
				}
				slab.id()
			};
			let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;

			let mut v = [0u8; 256];
			for i in 0..v.len() {
				v[i] = (i % 256) as u8;
			}
			slab_writer.write_fixed_bytes(v)?;

			let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
			let mut v_back = [1u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);

			slab_reader.seek(slab_id, 0);
			let mut v_back = [3u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);
		}

		Ok(())
	}

	#[test]
	fn test_global_slab_writer_unallocated() -> Result<(), Error> {
		let mut slab_writer = SlabWriter::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_writer.seek(slab.id(), 0);

		Ok(())
	}

	#[test]
	fn test_global_slab_reader_unallocated() -> Result<(), Error> {
		let mut slab_reader = SlabReader::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_reader.seek(slab.id(), 0);

		Ok(())
	}

	#[test]
	fn test_ser_array_and_array_list() -> Result<(), Error> {
		let mut arr = Array::new(10, &0)?;
		for i in 0..arr.size() {
			arr[i] = i;
		}
		ser_helper(arr)?;

		let mut v: Vec<u8> = vec![];
		v.push(0);
		v.push(0);
		v.push(0);
		v.push(0);

		v.push(0);
		v.push(0);
		v.push(0);
		v.push(0);
		let ser_in: Result<Array<u8>, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let mut arrlist: ArrayList<usize> = ArrayList::new(20, &0)?;
		for i in 0..20 {
			List::push(&mut arrlist, i)?;
		}
		ser_helper(arrlist)?;
		Ok(())
	}

	#[test]
	fn test_sortable_list() -> Result<(), Error> {
		let ser_out = list_box![1, 2, 3, 4];
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Box<dyn SortableList<u32>> = deserialize(&mut &v[..])?;
		assert!(list_eq!(ser_in, ser_out));
		Ok(())
	}

	#[test]
	fn test_ser_option() -> Result<(), Error> {
		let mut x: Option<bool> = None;
		ser_helper(x)?;
		x = Some(false);
		ser_helper(x)?;
		x = Some(true);
		ser_helper(x)?;

		Ok(())
	}

	#[test]
	fn test_read_ref() -> Result<(), Error> {
		let r = 1u32;
		let ser_out = &r;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<&u32, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());
		Ok(())
	}
}
