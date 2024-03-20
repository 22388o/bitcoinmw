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

use crate::misc::set_max;
use crate::misc::{slice_to_usize, usize_to_slice};
use crate::{
	Array, ArrayList, List, LockBox, SlabAllocator, SlabAllocatorConfig, SlabReader, SlabWriter,
	UtilBuilder, GLOBAL_SLAB_ALLOCATOR,
};
use bmw_err::{err, Error};
use bmw_log::*;
use bmw_ser::{Reader, Serializable, Writer};
use std::fmt::Debug;
use std::thread;

info!();

/*
impl Serializable for ConfigOption<'_> {
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
*/

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
				a = Some(UtilBuilder::build_array(len, &s)?);
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
		slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
		slab_id: usize,
		slab_ptr_size: Option<usize>,
	) -> Result<Self, Error> {
		debug!("new with slab_id = {}", slab_id)?;
		let (slab_size, slab_count) = match slabs {
			Some(ref slabs) => {
				let slabs = slabs.rlock()?;
				let guard = slabs.guard();
				((**guard).slab_size()?, (**guard).slab_count()?)
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
		self.do_write_fixed_bytes_impl(&[0u8; 0], Some(count))
	}

	fn process_write(
		&mut self,
		bytes: &[u8],
		bytes_len: usize,
		skip: bool,
	) -> Result<usize, Error> {
		let mut wlen = bytes_len;
		let space_left_in_slab = self.bytes_per_slab.saturating_sub(self.offset);

		if wlen > space_left_in_slab {
			wlen = space_left_in_slab;
		}

		if !skip {
			match self.slabs.as_mut() {
				Some(slabs) => {
					let mut slabs = slabs.wlock()?;
					let guard = slabs.guard();
					let mut slab_mut = (**guard).get_mut(self.slab_id)?;

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
	) -> Result<(), Error> {
		let bytes = bytes.as_ref();
		let bytes_len = match count {
			Some(count) => count,
			None => bytes.len(),
		};
		let skip = count.is_some();

		let b = bytes;
		let c = count;
		let s = self.slab_id;
		let o = self.offset;
		debug!("dwfb_impl b={:?},c={:?},s={},o={}", b, c, s, o)?;

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
				debug!("alloc slab b_offset={}, b_len={}", bytes_offset, bytes_len)?;
				// we need to allocate another slab
				self.next_slab(max_value)?;
			}

			let index = if skip { 0 } else { bytes_offset };

			let b = &bytes[index..];
			let l = bytes_len.saturating_sub(bytes_offset);
			let wlen = self.process_write(b, l, skip)?;
			debug!("wlen = {}", wlen)?;
			bytes_offset += wlen;
			self.offset += wlen;
		}

		Ok(())
	}

	fn read_next(&mut self) -> Result<usize, Error> {
		let next = match self.slabs.as_mut() {
			Some(slabs) => {
				let mut slabs = slabs.wlock()?;
				let guard = slabs.guard();
				let cur_slab = (**guard).get_mut(self.slab_id)?;
				slice_to_usize(&cur_slab.get()[self.bytes_per_slab..self.slab_size])?
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let cur_slab = slabs.get_mut(self.slab_id)?;
				Ok(slice_to_usize(
					&cur_slab.get()[self.bytes_per_slab..self.slab_size],
				)?)
			})?,
		};
		Ok(next)
	}

	fn next_slab(&mut self, max_value: usize) -> Result<(), Error> {
		self.offset = 0;
		let next = self.read_next()?;

		if next == max_value {
			let new_id = match self.slabs.as_mut() {
				Some(slabs) => {
					let mut slabs = slabs.wlock()?;
					let guard = slabs.guard();
					let mut nslab = (**guard).allocate()?;
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

			match self.slabs.as_mut() {
				Some(slabs) => {
					let mut slabs = slabs.wlock()?;
					let guard = slabs.guard();
					let mut slab = (**guard).get_mut(self.slab_id)?;
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
		self.do_write_fixed_bytes_impl(bytes, None)
	}
}

impl<'a> SlabReader {
	pub fn new(
		slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
		slab_id: usize,
		slab_ptr_size: Option<usize>,
	) -> Result<Self, Error> {
		let (slab_size, slab_count) = match slabs.as_ref() {
			Some(slabs) => {
				let slabs = slabs.rlock()?;
				let guard = slabs.guard();
				(guard.slab_size()?, guard.slab_count()?)
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
				let slabs = slabs.rlock()?;
				let guard = slabs.guard();
				let slab = (**guard).get(id)?;
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
				let slabs = slabs.rlock()?;
				let guard = slabs.guard();
				let slab = (**guard).get(id)?;
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
					let t = format!("overflow: next={}, self.max_value={}", next, self.max_value);
					let e = err!(ErrKind::IO, t);
					return Err(e);
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
