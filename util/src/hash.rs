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

use crate::constants::*;
use crate::list_append;
use crate::misc::{set_max, slice_to_usize, usize_to_slice};
use crate::types::{Direction, HashImpl, HashImplSync};
use crate::{
	Builder, Hashset, HashsetConfig, HashsetIterator, Hashtable, HashtableConfig,
	HashtableIterator, List, ListConfig, ListIterator, Reader, Serializable, SlabAllocator,
	SlabAllocatorConfig, SlabReader, SlabWriter, SortableList, Writer, GLOBAL_SLAB_ALLOCATOR,
};
use bmw_err::*;
use bmw_log::*;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;
use std::thread;

const SLOT_EMPTY: usize = usize::MAX;
const SLOT_DELETED: usize = usize::MAX - 1;

info!();

impl<'a, K, V> Iterator for HashtableIterator<'a, K, V>
where
	K: Serializable + Clone,
	V: Serializable + Clone,
{
	type Item = (K, V);
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		match self.hashtable.get_next(&mut self.cur) {
			Ok(x) => x,
			Err(e) => {
				let _ = error!("get_next generated unexpected error: {}", e);
				None
			}
		}
	}
}

impl<'a, K> Iterator for HashsetIterator<'a, K>
where
	K: Serializable + Clone,
{
	type Item = K;
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		let hashset = &mut self.hashset;
		match hashset.get_next_slot(&mut self.cur, Direction::Backward, &mut self.slab_reader) {
			Ok(ret) => match ret {
				true => match K::read(&mut self.slab_reader) {
					Ok(k) => Some(k),
					Err(e) => {
						let _ = warn!("deserialization generated error: {}", e);
						None
					}
				},
				false => None,
			},
			Err(e) => {
				let _ = warn!("get_next_slot generated error: {}", e);
				None
			}
		}
	}
}

impl<'a, V> Iterator for ListIterator<'a, V>
where
	V: Serializable + Clone,
{
	type Item = V;

	// only the None line is reported as not covered but it is
	#[cfg(not(tarpaulin_include))]
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		match self.linked_list_ref {
			Some(list) => {
				let mut slab_reader = self.slab_reader.as_mut().unwrap();
				// linked list
				if list.size == 0 {
					return None;
				}
				let slot = self.cur;
				match list.get_next_slot(&mut self.cur, self.direction, &mut slab_reader) {
					Ok(ret) => {
						if ret {
							// seek the location in the list for this
							// slot
							slab_reader.seek(slot, list.ptr_size * 2);
							match V::read(slab_reader) {
								Ok(v) => Some(v),
								Err(e) => {
									let _ = warn!("deserialization generated error: {}", e);
									None
								}
							}
						} else {
							None
						}
					}
					Err(e) => {
						let _ = warn!("get_next_slot generated error: {}", e);
						None
					}
				}
			}
			None => {
				// array list
				let array_list_ref = self.array_list_ref.unwrap();
				if array_list_ref.size == 0 {
					None
				} else if self.direction == Direction::Forward && self.cur >= array_list_ref.size {
					None
				} else if self.direction == Direction::Backward && self.cur <= 0 {
					None
				} else {
					let ret = Some(array_list_ref.inner[self.cur].clone());
					if self.direction == Direction::Forward {
						self.cur += 1;
					} else {
						self.cur = self.cur.saturating_sub(1);
					}
					ret
				}
			}
		}
	}
}

impl<'a, K, V> HashtableIterator<'a, K, V>
where
	K: Serializable + Clone,
{
	fn new(hashtable: &'a HashImpl<K>, cur: usize) -> Self {
		Self {
			hashtable,
			cur,
			_phantom_data: PhantomData,
		}
	}
}

impl<'a, K> HashsetIterator<'a, K>
where
	K: Serializable + Clone,
{
	fn new(hashset: &'a HashImpl<K>, cur: usize) -> Self {
		Self {
			hashset,
			cur,
			_phantom_data: PhantomData,
			slab_reader: hashset.slab_reader.clone(),
		}
	}
}

impl<'a, V> ListIterator<'a, V>
where
	V: Serializable + Clone,
{
	fn new(list: &'a HashImpl<V>, cur: usize, direction: Direction) -> Self {
		let _ = debug!("new list iter");
		Self {
			linked_list_ref: Some(list),
			cur,
			direction,
			_phantom_data: PhantomData,
			slab_reader: Some(list.slab_reader.clone()),
			array_list_ref: None,
		}
	}
}

impl Default for HashtableConfig {
	fn default() -> Self {
		Self {
			max_entries: 100_000,
			max_load_factor: 0.8,
		}
	}
}

impl Default for HashsetConfig {
	fn default() -> Self {
		Self {
			max_entries: 100_000,
			max_load_factor: 0.8,
		}
	}
}

impl Default for ListConfig {
	fn default() -> Self {
		Self {}
	}
}

impl<K> PartialEq for HashImpl<K>
where
	K: Serializable + PartialEq + Clone + Debug,
{
	// break and loop line reported as not covered but they are
	#[cfg(not(tarpaulin_include))]
	fn eq(&self, rhs: &Self) -> bool {
		if self.size != rhs.size {
			false
		} else {
			let mut itt1 = ListIterator::new(self, self.head, Direction::Forward);
			let mut itt2 = ListIterator::new(rhs, rhs.head, Direction::Forward);
			loop {
				let next1 = itt1.next();
				let next2 = itt2.next();
				if next1 != next2 {
					return false;
				}
				if next1 == None {
					break;
				}
			}

			true
		}
	}
}

impl<V> SortableList<V> for HashImpl<V>
where
	V: Serializable + Debug + Clone,
{
	fn sort(&mut self) -> Result<(), Error>
	where
		V: Ord,
	{
		if self.size > 0 {
			let first = self.iter().next().unwrap();
			let mut list = Builder::build_array_list::<V>(self.size, &first)?;
			list_append!(list, self);
			list.sort()?;
			self.clear()?;
			list_append!(self, list);
		}
		Ok(())
	}
	fn sort_unstable(&mut self) -> Result<(), Error>
	where
		V: Ord,
	{
		if self.size > 0 {
			let first = self.iter().next().unwrap();
			let mut list = Builder::build_array_list::<V>(self.size, &first)?;
			list_append!(list, self);
			list.sort_unstable()?;
			self.clear()?;
			list_append!(self, list);
		}
		Ok(())
	}
}

impl<K> Debug for HashImpl<K>
where
	K: Serializable + Debug + Clone,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		if self.entry_array.is_some() {
			let itt = HashsetIterator::new(self, self.tail);
			write!(f, "[")?;
			let mut i = 0;
			for x in itt {
				let v = if self.is_hashtable { "=VALUE" } else { "" };
				if i == 0 {
					write!(f, "{:?}{}", x, v)?;
				} else {
					write!(f, ", {:?}{}", x, v)?;
				}
				i += 1;
			}
			write!(f, "]")?;
		} else {
			let itt = ListIterator::new(self, self.head, Direction::Forward);
			write!(f, "[")?;
			let mut i = 0;
			for x in itt {
				if i == 0 {
					write!(f, "{:?}", x)?;
				} else {
					write!(f, ", {:?}", x)?;
				}
				i += 1;
			}
			write!(f, "]")?;
		}
		Ok(())
	}
}

unsafe impl<K> Send for HashImplSync<K> where K: Serializable + Clone {}

unsafe impl<K> Sync for HashImplSync<K> where K: Serializable + Clone {}

impl<V> SortableList<V> for HashImplSync<V>
where
	V: Clone + PartialEq + Debug + Serializable,
{
	fn sort(&mut self) -> Result<(), Error>
	where
		V: Ord,
	{
		self.static_impl.sort()
	}
	fn sort_unstable(&mut self) -> Result<(), Error>
	where
		V: Ord,
	{
		self.static_impl.sort_unstable()
	}
}

impl<K> PartialEq for HashImplSync<K>
where
	K: Serializable + PartialEq + Clone + Debug,
{
	fn eq(&self, rhs: &Self) -> bool {
		self.static_impl == rhs.static_impl
	}
}

impl<K> Debug for HashImplSync<K>
where
	K: Serializable + Debug + Clone,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{:?}", self.static_impl)
	}
}

impl<K> HashImplSync<K>
where
	K: Serializable + Clone,
{
	pub(crate) fn new(
		htc: Option<HashtableConfig>,
		hsc: Option<HashsetConfig>,
		list_config: Option<ListConfig>,
		slab_allocator_config: SlabAllocatorConfig,
	) -> Result<Self, Error> {
		let slabs = Builder::build_slabs_ref();
		{
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			slabs.init(slab_allocator_config)?;
		}

		let static_impl = HashImpl::new(htc, hsc, list_config, &Some(&slabs), false)?;
		Ok(Self { static_impl })
	}
}

impl<K, V> Hashtable<K, V> for HashImplSync<K>
where
	K: Serializable + Hash + PartialEq + Debug + Clone,
	V: Serializable + Clone,
{
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl
			.insert_hash_impl(Some(key), Some(value), None, hash)
	}
	fn get(&self, key: &K) -> Result<Option<V>, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.static_impl.get_impl(key, hash)? {
			Some((_entry, mut reader)) => Ok(Some(V::read(&mut reader)?)),
			None => Ok(None),
		}
	}
	fn remove(&mut self, key: &K) -> Result<Option<V>, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.static_impl.get_impl(key, hash)? {
			Some((entry, mut reader)) => {
				let v = V::read(&mut reader)?;
				self.static_impl.remove_impl(entry)?;
				Ok(Some(v))
			}
			None => Ok(None),
		}
	}
	fn size(&self) -> usize {
		self.static_impl.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.static_impl.clear_impl()
	}

	fn iter<'b>(&'b self) -> HashtableIterator<'b, K, V> {
		HashtableIterator::new(&self.static_impl, self.static_impl.tail)
	}
	fn max_load_factor(&self) -> f64 {
		self.static_impl.max_load_factor
	}
	fn max_entries(&self) -> usize {
		self.static_impl.max_entries
	}
	fn bring_to_front(&mut self, key: &K) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl.bring_to_front_impl::<V>(key, hash)
	}
	fn remove_oldest(&mut self) -> Result<(), Error> {
		self.static_impl.remove_oldest_impl()
	}
	fn raw_read(
		&self,
		key: &K,
		offset: usize,
		data: &mut [u8; CACHE_BUFFER_SIZE],
	) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl.raw_read_impl(key, hash, offset, data)
	}
	fn raw_write(
		&mut self,
		key: &K,
		offset: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error>
	where
		V: Clone,
	{
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl
			.raw_write_impl::<V>(key, hash, offset, data, len)
	}
	fn slabs(&self) -> Result<Option<Rc<RefCell<dyn SlabAllocator>>>, Error> {
		self.static_impl.slabs_impl()
	}
}

impl<K> Hashset<K> for HashImplSync<K>
where
	K: Serializable + Hash + PartialEq + Debug + Clone,
{
	fn insert(&mut self, key: &K) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl
			.insert_hash_impl::<K>(Some(key), None, None, hash)
	}
	fn contains(&self, key: &K) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.static_impl.get_impl(key, hash)? {
			Some(_) => Ok(true),
			None => Ok(false),
		}
	}
	fn remove(&mut self, key: &K) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.static_impl.get_impl(key, hash)? {
			Some((entry, _reader)) => {
				self.static_impl.remove_impl(entry)?;
				Ok(true)
			}
			None => Ok(false),
		}
	}
	fn size(&self) -> usize {
		self.static_impl.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.static_impl.clear_impl()
	}

	fn iter<'b>(&'b self) -> HashsetIterator<'b, K> {
		HashsetIterator::new(&self.static_impl, self.static_impl.tail)
	}
	fn max_load_factor(&self) -> f64 {
		self.static_impl.max_load_factor
	}
	fn max_entries(&self) -> usize {
		self.static_impl.max_entries
	}
}

impl<V> List<V> for HashImplSync<V>
where
	V: Serializable + Debug + PartialEq + Clone,
{
	fn push(&mut self, value: V) -> Result<(), Error> {
		self.static_impl
			.insert_impl::<V>(Some(&value), None, None, None, None, false)
	}

	fn iter<'b>(&'b self) -> Box<dyn Iterator<Item = V> + 'b> {
		Box::new(ListIterator::new(
			&self.static_impl,
			self.static_impl.head,
			Direction::Forward,
		))
	}
	fn iter_rev<'b>(&'b self) -> Box<dyn Iterator<Item = V> + 'b> {
		Box::new(ListIterator::new(
			&self.static_impl,
			self.static_impl.tail,
			Direction::Backward,
		))
	}
	fn delete_head(&mut self) -> Result<(), Error> {
		self.static_impl.delete_head()
	}
	fn size(&self) -> usize {
		self.static_impl.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.static_impl.clear_impl()
	}
}

impl<K> HashImpl<K>
where
	K: Serializable + Clone,
{
	// several lines reported as not covered, but they are
	#[cfg(not(tarpaulin_include))]
	pub(crate) fn new(
		hashtable_config: Option<HashtableConfig>,
		hashset_config: Option<HashsetConfig>,
		list_config: Option<ListConfig>,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
		debug_large_slab_count: bool,
	) -> Result<Self, Error> {
		let (slab_size, slab_count) = match slabs {
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
						let m1 = "Slab allocator was not initialized for thread";
						let m2 = "Initializing with default values.";
						warn!("WARN: {} '{}'. {}", m1, n, m2)?;
						slabs.init(SlabAllocatorConfig::default())?;
						slabs.slab_size()?
					}
				};
				let slab_count = slabs.slab_count()?;
				Ok((slab_size, slab_count))
			})?,
		};

		if slab_size > 256 * 256 {
			let fmt = "slab_size must be equal to or less than 65,536";
			let e = err!(ErrKind::Configuration, fmt);
			return Err(e);
		}

		if slab_count > 281_474_976_710_655 || debug_large_slab_count {
			let fmt = "slab_count must be equal to or less than 281_474_976_710_655";
			let e = err!(ErrKind::Configuration, fmt);
			return Err(e);
		}

		let (max_entries, max_load_factor) = match hashtable_config {
			Some(config) => {
				if config.max_entries == 0 {
					let fmt = "MaxEntries must be greater than 0";
					let e = err!(ErrKind::Configuration, fmt);
					return Err(e);
				}
				if config.max_load_factor <= 0.0 || config.max_load_factor > 1.0 {
					let fmt = "MaxLoadFactor must be greater than 0 and less than or equal to 1.0";
					let e = err!(ErrKind::Configuration, fmt);
					return Err(e);
				}
				(config.max_entries, config.max_load_factor)
			}
			None => {
				match hashset_config {
					Some(config) => {
						if config.max_entries == 0 {
							let fmt = "MaxEntries must be greater than 0";
							let e = err!(ErrKind::Configuration, fmt);
							return Err(e);
						}
						if config.max_load_factor <= 0.0 || config.max_load_factor > 1.0 {
							let fmt = "MaxLoadFactor must be greater than 0 and less than or equal to 1.0";
							let e = err!(ErrKind::Configuration, fmt);
							return Err(e);
						}

						(config.max_entries, config.max_load_factor)
					}
					None => (0, 1.0), // for lists it's ignored
				}
			}
		};

		let (entry_array, ptr_size) = match list_config {
			Some(_) => {
				let mut x = slab_count;
				let mut ptr_size = 0;
				loop {
					if x == 0 {
						break;
					}
					x >>= 8;
					ptr_size += 1;
				}
				debug!("ptr_size={}", ptr_size)?;
				(None, ptr_size)
			}
			None => {
				let size: usize = (max_entries as f64 / max_load_factor).ceil() as usize;
				let entry_array = Builder::build_array(size, &SLOT_EMPTY)?;
				debug!("entry array init to size = {}", size)?;
				let mut x = entry_array.size() + 2; // two more, one for deleted and one for empty
				let mut ptr_size = 0;
				loop {
					if x == 0 {
						break;
					}
					x >>= 8;
					ptr_size += 1;
				}
				debug!("ptr_size={}", ptr_size)?;
				(Some(entry_array), ptr_size)
			}
		};
		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..ptr_size]);
		let max_value = slice_to_usize(&ptr[0..ptr_size])?;

		let bytes_per_slab = slab_size.saturating_sub(ptr_size);
		if slab_size < ptr_size * 4 {
			let fmt = format!("SlabSize is too small. Must be at least {}", ptr_size * 4);
			let e = err!(ErrKind::Configuration, fmt);
			return Err(e);
		}

		let (slab_reader, slab_writer, slabs) = match slabs.clone() {
			Some(slabs) => {
				let slabs2: Rc<RefCell<(dyn SlabAllocator + 'static)>> = slabs.clone();
				(
					SlabReader::new(Some(slabs2.clone()), 0, Some(ptr_size))?,
					SlabWriter::new(Some(slabs2.clone()), 0, Some(ptr_size))?,
					Some(slabs2.clone()),
				)
			}
			None => (
				SlabReader::new(None, 0, Some(ptr_size))?,
				SlabWriter::new(None, 0, Some(ptr_size))?,
				None,
			),
		};

		let ret = Self {
			slabs,
			entry_array,
			bytes_per_slab,
			max_value,
			slab_size,
			ptr_size,
			max_load_factor,
			max_entries,
			size: 0,
			head: max_value,
			tail: max_value,
			slab_reader,
			slab_writer,
			_phantom_data: PhantomData,
			is_hashtable: hashtable_config.is_some(),
			debug_get_next_slot_error: false,
			debug_entry_array_len: false,
		};
		Ok(ret)
	}

	#[cfg(test)]
	fn set_debug_get_next_slot_error(&mut self, v: bool) {
		self.debug_get_next_slot_error = v;
	}

	#[cfg(test)]
	fn set_debug_entry_array_len(&mut self, v: bool) {
		self.debug_entry_array_len = v;
	}

	fn get_next<V>(
		&self,
		cur: &mut usize,
	) -> Result<Option<<HashtableIterator<K, V> as Iterator>::Item>, Error>
	where
		V: Serializable + Clone,
	{
		let mut reader = self.slab_reader.clone();
		match self.get_next_slot(cur, Direction::Backward, &mut reader)? {
			true => Ok(Some((K::read(&mut reader)?, V::read(&mut reader)?))),
			false => Ok(None),
		}
	}

	// a few lines reported as not covered, but they are
	#[cfg(not(tarpaulin_include))]
	fn get_next_slot(
		&self,
		cur: &mut usize,
		direction: Direction,
		reader: &mut SlabReader,
	) -> Result<bool, Error> {
		if self.debug_get_next_slot_error {
			let e = err!(ErrKind::Test, "get_next_slot");
			return Err(e);
		}
		debug!("cur={}", *cur)?;
		if *cur >= self.max_value {
			return Ok(false);
		}
		let slot = match &self.entry_array {
			Some(entry_array) => entry_array[*cur],
			None => *cur,
		};
		debug!("slot={}", slot)?;

		let mut ptrs = [0u8; 8];
		let ptr_size = self.ptr_size;

		*cur = match direction {
			Direction::Backward => {
				reader.seek(slot, ptr_size);
				reader.read_fixed_bytes(&mut ptrs[0..ptr_size])?;
				slice_to_usize(&ptrs[0..ptr_size])?
			}
			Direction::Forward => {
				reader.seek(slot, 0);
				reader.read_fixed_bytes(&mut ptrs[0..ptr_size])?;
				slice_to_usize(&ptrs[0..ptr_size])?
			}
		};
		debug!("read cur = {}", cur)?;
		Ok(true)
	}

	fn delete_head_impl(&mut self) -> Result<(), Error> {
		if self.size != 0 {
			self.remove_impl(self.head)?;
		}
		Ok(())
	}

	// fully tested, but some lines reported as not covered
	#[cfg(not(tarpaulin_include))]
	fn clear_impl(&mut self) -> Result<(), Error> {
		let mut cur = self.tail;
		loop {
			if cur == SLOT_EMPTY || cur == SLOT_DELETED {
				break;
			}

			if self.entry_array.is_none() && cur >= self.max_value {
				break;
			}
			debug!("clear impl cur={}", cur)?;

			if cur < self.max_value {
				let entry = self.lookup_entry(cur);
				debug!("free chain = {}", entry)?;
				self.free_chain(entry)?;
			} else {
				break;
			}

			let last_cur = cur;
			let dir = Direction::Backward;
			self.get_next_slot(&mut cur, dir, &mut self.slab_reader.clone())?;
			match self.entry_array.as_mut() {
				Some(entry_array) => {
					debug!("setting entry_array[{}]={}", last_cur, SLOT_EMPTY)?;
					entry_array[last_cur] = SLOT_EMPTY
				}
				None => {}
			}
		}
		debug!("set size to 0")?;
		self.size = 0;
		self.tail = SLOT_EMPTY;
		self.head = SLOT_EMPTY;

		// clear the entry array to get rid of SLOT_DELETED
		if self.entry_array.is_some() {
			let size = self.entry_array.as_ref().unwrap().size();
			let entry_array = Builder::build_array(size, &SLOT_EMPTY)?;
			self.entry_array = Some(entry_array);
		}

		Ok(())
	}

	fn remove_oldest_impl(&mut self) -> Result<(), Error> {
		debug!(
			"remove_oldest_impl self.head={}, Slot_empty={}",
			self.head, SLOT_EMPTY
		)?;
		if self.head != SLOT_EMPTY {
			self.remove_impl(self.head)?;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn bring_to_front_impl<V>(&mut self, key: &K, hash: usize) -> Result<(), Error>
	where
		K: PartialEq,
		V: Serializable + Clone,
	{
		match self.get_impl(key, hash)? {
			Some((entry, _reader)) => {
				debug!(
					"entry={},self.tail={},self.head={}",
					entry, self.tail, self.head
				)?;
				if entry != self.tail {
					let entry_slab_id = self.lookup_entry(entry);
					let tail_slab_id = self.lookup_entry(self.tail);
					let ptr_size = self.ptr_size;
					self.slab_reader.seek(entry_slab_id, 0);
					let mut ptrs = [0u8; 16];

					self.slab_reader
						.read_fixed_bytes(&mut ptrs[0..ptr_size * 2])?;
					let entry_next = slice_to_usize(&ptrs[0..ptr_size])?;
					let entry_prev = slice_to_usize(&ptrs[ptr_size..ptr_size * 2])?;
					let entry_next_slab_id = self.lookup_entry(entry_next);

					// update entry_prev_next to entry_next
					if entry != self.head {
						let entry_prev_slab_id = self.lookup_entry(entry_prev);
						self.slab_writer.seek(entry_prev_slab_id, 0);
						usize_to_slice(entry_next, &mut ptrs[0..ptr_size])?;
						self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;
					}

					// update entry_next_prev to entry_prev
					self.slab_writer.seek(entry_next_slab_id, ptr_size);
					usize_to_slice(entry_prev, &mut ptrs[0..ptr_size])?;
					self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;

					// write the entry to point to current tail
					self.slab_writer.seek(entry_slab_id, 0);
					usize_to_slice(SLOT_EMPTY, &mut ptrs[0..ptr_size])?;
					usize_to_slice(self.tail, &mut ptrs[ptr_size..ptr_size * 2])?;
					self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size * 2])?;

					// update the tail
					self.slab_writer.seek(tail_slab_id, 0);
					usize_to_slice(entry, &mut ptrs[0..ptr_size])?;
					self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;

					self.tail = entry;
					if entry == self.head {
						debug!("setting head to {}", entry_next)?;
						self.head = entry_next;
					}
				}
			}
			None => {}
		}

		Ok(())
	}

	fn raw_read_impl(
		&self,
		key: &K,
		hash: usize,
		offset: usize,
		data: &mut [u8; CACHE_BUFFER_SIZE],
	) -> Result<bool, Error>
	where
		K: PartialEq,
	{
		match self.get_impl(key, hash)? {
			Some((_entry, mut reader)) => {
				reader.skip_bytes(offset)?;
				reader.read_fixed_bytes(data)?;
				Ok(true)
			}
			None => Ok(false),
		}
	}
	fn raw_write_impl<V>(
		&mut self,
		key: &K,
		hash: usize,
		offset: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error>
	where
		K: PartialEq,
		V: Clone + Serializable,
	{
		self.insert_hash_impl::<V>(Some(key), None, Some((offset, data, len)), hash)?;

		Ok(())
	}

	// None line reported as not covered, but it is
	#[cfg(not(tarpaulin_include))]
	fn get_impl(&self, key: &K, hash: usize) -> Result<Option<(usize, SlabReader)>, Error>
	where
		K: Serializable + PartialEq + Clone,
	{
		let entry_array_len = match self.entry_array.as_ref() {
			Some(e) => e.size(),
			None => {
				let fmt = "get_impl called with no entry array";
				let e = err!(ErrKind::IllegalState, fmt);
				return Err(e);
			}
		};
		let mut entry = hash % entry_array_len;

		let mut i = 0;
		loop {
			if i >= entry_array_len || self.debug_entry_array_len {
				let msg = "HashImpl: Capacity exceeded";
				return Err(err!(ErrKind::CapacityExceeded, msg));
			}
			if self.lookup_entry(entry) == SLOT_EMPTY {
				debug!("slot empty at {}", entry)?;
				return Ok(None);
			}

			// does the current key match ours?
			if self.lookup_entry(entry) != SLOT_DELETED {
				let rkey = self.read_key(self.lookup_entry(entry))?;
				if rkey.is_some() {
					let (k, reader) = rkey.unwrap();
					if &k == key {
						return Ok(Some((entry, reader)));
					}
				}
			}

			entry = (entry + 1) % entry_array_len;
			i += 1;
		}
	}

	// fully covered but tarpaulin reporting a few lines uncovered
	#[cfg(not(tarpaulin_include))]
	fn insert_hash_impl<V>(
		&mut self,
		key: Option<&K>,
		value: Option<&V>,
		raw_chunk: Option<(usize, &[u8; CACHE_BUFFER_SIZE], usize)>,
		hash: usize,
	) -> Result<(), Error>
	where
		K: Serializable + PartialEq + Clone,
		V: Serializable + Clone,
	{
		debug!(
			"insert_hash_impl with cur head = {}, raw_chunk={:?}",
			self.head, raw_chunk
		)?;
		let entry_array_len = self.entry_array.as_ref().unwrap().size();

		let key_val = key.unwrap();
		let mut entry = hash % entry_array_len;

		// check the load factor
		if (self.size + 1) as f64 > self.max_load_factor * entry_array_len as f64 {
			let fmt = format!("load factor ({}) exceeded", self.max_load_factor);
			return Err(err!(ErrKind::CapacityExceeded, fmt));
		}

		let mut i = 0;
		let mut slab_id = self.max_value;
		let mut raw_exists = false;
		loop {
			debug!("loop")?;
			if i >= entry_array_len || self.debug_entry_array_len {
				let msg = "HashImpl: Capacity exceeded";
				debug!("err1")?;
				return Err(err!(ErrKind::CapacityExceeded, msg));
			}
			let entry_value = self.lookup_entry(entry);
			if entry_value == SLOT_EMPTY || entry_value == SLOT_DELETED {
				debug!("break")?;
				break;
			}
			// does the current key match ours?
			let kr = self.read_key(entry_value)?;
			if kr.is_some() {
				let k = kr.unwrap().0;
				if &k == key_val {
					if raw_chunk.is_none() {
						self.remove_impl(entry)?;
					} else {
						raw_exists = true;
						slab_id = entry_value;
					}
					break;
				}
			}

			entry = (entry + 1) % entry_array_len;
			i += 1;
		}

		debug!("insert_impl")?;
		self.insert_impl(
			key,
			value,
			raw_chunk,
			Some(entry),
			match slab_id != self.max_value {
				true => Some(slab_id),
				false => None,
			},
			raw_exists,
		)?;

		debug!(
			"---------------------------------insert_hash_impl complete slab_id={}",
			slab_id
		)?;

		Ok(())
	}

	// fully covered but tarpaulin reporting a few lines uncovered
	#[cfg(not(tarpaulin_include))]
	fn insert_impl<V>(
		&mut self,
		key: Option<&K>,
		value: Option<&V>,
		raw_value: Option<(usize, &[u8; CACHE_BUFFER_SIZE], usize)>,
		entry: Option<usize>,
		slab_id_allocated: Option<usize>,
		raw_exists: bool,
	) -> Result<(), Error>
	where
		V: Serializable + Clone,
	{
		debug!(
			"in insert impl with raw value = {:?}, entry = {:?}, raw_exists = {}",
			raw_value, entry, raw_exists
		)?;
		let ptr_size = self.ptr_size;
		let max_value = self.max_value;
		let tail = self.tail;
		debug!("in insert_impl")?;
		let slab_id = match slab_id_allocated {
			Some(slab_id) => slab_id,
			None => self.allocate()?,
		};
		debug!("alloc={},entry={:?}", slab_id, entry)?;
		self.slab_writer.seek(slab_id, 0);

		// for lists we use the slab_id as the entry
		let entry = match entry {
			Some(entry) => entry,
			None => slab_id,
		};
		let mut ptrs = [0u8; 16];
		debug!("slab_id={}", slab_id)?;
		// update head/tail pointers
		usize_to_slice(SLOT_EMPTY, &mut ptrs[0..ptr_size])?;
		usize_to_slice(tail, &mut ptrs[ptr_size..ptr_size * 2])?;
		debug!("updating slab id {}", slab_id)?;

		if !raw_exists {
			self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size * 2])?;
		} else {
			self.slab_writer.skip_bytes(ptr_size * 2)?;
		}

		if key.is_some() {
			match key.as_ref().unwrap().write(&mut self.slab_writer) {
				Ok(_) => {}
				Err(e) => {
					warn!("writing key generated error: {}", e)?;
					self.free_chain(slab_id)?;
					let fmt = format!("writing key generated error: {}", e);
					let e = err!(ErrKind::CapacityExceeded, fmt);
					return Err(e);
				}
			}
		}
		debug!("value write")?;
		if value.is_some() {
			match value.as_ref().unwrap().write(&mut self.slab_writer) {
				Ok(_) => {}
				Err(e) => {
					warn!("writing value generated error: {}", e)?;
					self.free_chain(slab_id)?;
					let fmt = format!("writing value generated error: {}", e);
					let e = err!(ErrKind::CapacityExceeded, fmt);
					return Err(e);
				}
			}
		}

		if raw_value.is_some() {
			let raw_value = raw_value.unwrap();
			debug!("skip bytes {}", raw_value.0)?;
			self.slab_writer.skip_bytes(raw_value.0)?;
			debug!("write fixed bytes {:?}", raw_value.1)?;
			self.slab_writer
				.write_fixed_bytes(&raw_value.1[..raw_value.2])?;
		}

		debug!("array update")?;

		if !raw_exists {
			match self.entry_array.as_mut() {
				Some(entry_array) => {
					// for hash based structures we use the entry index
					if self.tail < max_value {
						if entry_array[self.tail] < max_value {
							let entry_value = self.lookup_entry(self.tail);
							self.slab_writer.seek(entry_value, 0);
							usize_to_slice(entry, &mut ptrs[0..ptr_size])?;
							self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;
						}
					}
				}
				None => {
					// for list based structures we use the slab_id directly
					if self.tail < max_value {
						self.slab_writer.seek(self.tail, 0);
						usize_to_slice(entry, &mut ptrs[0..ptr_size])?;
						self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;
					}
				}
			}

			self.tail = entry;

			if self.head >= max_value {
				self.head = entry;
			}

			if self.entry_array.is_some() {
				self.entry_array.as_mut().unwrap()[entry] = slab_id;
			}

			self.size += 1;
		}

		Ok(())
	}

	fn read_key(&self, slab_id: usize) -> Result<Option<(K, SlabReader)>, Error> {
		let ptr_size = self.ptr_size;
		// get a reader, we have to clone the rc because we are not mutable
		let mut reader = self.slab_reader.clone();
		// seek past the ptr data
		reader.seek(slab_id, ptr_size * 2);
		// read our serailized struct
		Ok(Some((K::read(&mut reader)?, reader)))
	}

	fn allocate(&mut self) -> Result<usize, Error> {
		match &mut self.slabs {
			Some(slabs) => {
				let mut slabs: RefMut<_> = slabs.borrow_mut();
				let mut slab = slabs.allocate()?;
				let slab_mut = slab.get_mut();
				// set next pointer to none
				for i in self.bytes_per_slab..self.slab_size {
					slab_mut[i] = 0xFF;
				}
				debug!("allocate id = {}", slab.id())?;
				Ok(slab.id())
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let mut slab = slabs.allocate()?;
				let slab_mut = slab.get_mut();
				// set next pointer to none
				for i in self.bytes_per_slab..self.slab_size {
					slab_mut[i] = 0xFF;
				}
				debug!("allocate id = {}", slab.id())?;
				Ok(slab.id())
			}),
		}
	}

	fn free(&mut self, slab_id: usize) -> Result<(), Error> {
		match &mut self.slabs {
			Some(slabs) => {
				let mut slabs: RefMut<_> = slabs.borrow_mut();
				slabs.free(slab_id)
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				slabs.free(slab_id)
			}),
		}
	}

	// loop and break counted as not covered, but are covered
	#[cfg(not(tarpaulin_include))]
	fn free_chain(&mut self, slab_id: usize) -> Result<(), Error> {
		debug!("free chain {}", slab_id)?;
		let bytes_per_slab = self.bytes_per_slab;
		let slab_size = self.slab_size;
		let mut next_bytes = slab_id;
		loop {
			let id = next_bytes;
			let n = match &self.slabs {
				Some(slabs) => {
					let slabs: Ref<_> = slabs.borrow();
					let slab = slabs.get(next_bytes)?;
					slice_to_usize(&slab.get()[bytes_per_slab..slab_size])
				}
				None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					let slab = slabs.get(next_bytes)?;
					slice_to_usize(&slab.get()[bytes_per_slab..slab_size])
				}),
			}?;
			next_bytes = n;
			debug!("free id = {}, next_bytes={}", id, next_bytes)?;
			self.free(id)?;

			if next_bytes >= self.max_value {
				break;
			}
		}
		Ok(())
	}
	fn lookup_entry(&self, entry: usize) -> usize {
		match self.entry_array.as_ref() {
			Some(entry_array) => entry_array[entry],
			None => entry,
		}
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn free_iter_list(&mut self, entry: usize) -> Result<(), Error> {
		let slab_id = self.lookup_entry(entry);
		let mut next = [0u8; 8];
		let mut prev = [0u8; 8];
		let ptr_size = self.ptr_size;
		self.slab_reader.seek(slab_id, 0);
		self.slab_reader.read_fixed_bytes(&mut next[0..ptr_size])?;
		self.slab_reader.read_fixed_bytes(&mut prev[0..ptr_size])?;

		let next_usize_entry = slice_to_usize(&next[0..ptr_size])?;
		let prev_usize_entry = slice_to_usize(&prev[0..ptr_size])?;

		debug!(
			"free_iter_list with self.head = {}, entry = {}, next_usize_entry={}, prev_usize_entry={}",
			self.head, entry, next_usize_entry, prev_usize_entry
		)?;
		if self.head == entry {
			if next_usize_entry >= self.max_value {
				debug!("updating self.head to {}", SLOT_EMPTY)?;
				self.head = SLOT_EMPTY;
			} else {
				debug!("2updating self.head to {}", next_usize_entry)?;
				self.head = next_usize_entry;
			}
		}
		if self.tail == entry {
			if prev_usize_entry >= self.max_value {
				self.tail = SLOT_EMPTY;
			} else {
				self.tail = prev_usize_entry;
			}
		}

		if next_usize_entry < self.max_value {
			let next_usize = self.lookup_entry(next_usize_entry);
			if next_usize < self.max_value {
				let mut ptrs = [0u8; 8];
				self.slab_reader.seek(next_usize, 0);
				self.slab_reader
					.read_fixed_bytes(&mut ptrs[0..ptr_size * 2])?;
				usize_to_slice(prev_usize_entry, &mut ptrs[ptr_size..ptr_size * 2])?;
				self.slab_writer.seek(next_usize, 0);
				self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size * 2])?;
			}
		}

		if prev_usize_entry < self.max_value {
			let prev_usize = self.lookup_entry(prev_usize_entry);
			if prev_usize < self.max_value {
				let mut next = [0u8; 8];
				let mut prev = [0u8; 8];
				self.slab_reader.seek(prev_usize, 0);
				self.slab_reader.read_fixed_bytes(&mut next[0..ptr_size])?;
				self.slab_reader.read_fixed_bytes(&mut prev[0..ptr_size])?;
				usize_to_slice(next_usize_entry, &mut next[0..ptr_size])?;
				self.slab_writer.seek(prev_usize, 0);
				self.slab_writer.write_fixed_bytes(&next[0..ptr_size])?;
				self.slab_writer.write_fixed_bytes(&prev[0..ptr_size])?;
			}
		}

		Ok(())
	}
	fn remove_impl(&mut self, entry: usize) -> Result<(), Error> {
		debug!("remove impl {}", entry)?;

		self.free_iter_list(entry)?;
		self.free_chain(self.lookup_entry(entry))?;
		if self.entry_array.is_some() {
			self.entry_array.as_mut().unwrap()[entry] = SLOT_DELETED;
		}
		self.size = self.size.saturating_sub(1);

		Ok(())
	}

	fn slabs_impl(&self) -> Result<Option<Rc<RefCell<dyn SlabAllocator>>>, Error> {
		Ok(self.slabs.clone())
	}
}

impl<K> Drop for HashImpl<K>
where
	K: Serializable + Clone,
{
	fn drop(&mut self) {
		let res = self.clear_impl();
		if res.is_err() {
			let e = res.unwrap_err();
			let _ = warn!("unexpected error in drop: {}", e);
		}
	}
}

impl<K, V> Hashtable<K, V> for HashImpl<K>
where
	K: Serializable + Hash + PartialEq + Debug + Clone,
	V: Serializable + Clone,
{
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.insert_hash_impl(Some(key), Some(value), None, hash)
	}
	fn get(&self, key: &K) -> Result<Option<V>, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.get_impl(key, hash)? {
			Some((_entry, mut reader)) => Ok(Some(V::read(&mut reader)?)),
			None => Ok(None),
		}
	}
	fn remove(&mut self, key: &K) -> Result<Option<V>, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.get_impl(key, hash)? {
			Some((entry, mut reader)) => {
				let v = V::read(&mut reader)?;
				self.remove_impl(entry)?;
				Ok(Some(v))
			}
			None => Ok(None),
		}
	}
	fn size(&self) -> usize {
		self.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.clear_impl()
	}

	fn iter<'b>(&'b self) -> HashtableIterator<'b, K, V> {
		HashtableIterator::new(self, self.tail)
	}
	fn max_load_factor(&self) -> f64 {
		self.max_load_factor
	}
	fn max_entries(&self) -> usize {
		self.max_entries
	}
	fn bring_to_front(&mut self, key: &K) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.bring_to_front_impl::<V>(key, hash)
	}
	fn remove_oldest(&mut self) -> Result<(), Error> {
		self.remove_oldest_impl()
	}
	fn raw_read(
		&self,
		key: &K,
		chunk: usize,
		data: &mut [u8; CACHE_BUFFER_SIZE],
	) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.raw_read_impl(key, hash, chunk, data)
	}
	fn raw_write(
		&mut self,
		key: &K,
		chunk: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.raw_write_impl::<V>(key, hash, chunk, data, len)
	}
	fn slabs(&self) -> Result<Option<Rc<RefCell<dyn SlabAllocator>>>, Error> {
		self.slabs_impl()
	}
}

impl<K> Hashset<K> for HashImpl<K>
where
	K: Serializable + Hash + PartialEq + Debug + Clone,
{
	fn insert(&mut self, key: &K) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.insert_hash_impl::<K>(Some(key), None, None, hash)
	}
	fn contains(&self, key: &K) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.get_impl(key, hash)? {
			Some(_) => Ok(true),
			None => Ok(false),
		}
	}
	fn remove(&mut self, key: &K) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		match self.get_impl(key, hash)? {
			Some((entry, _reader)) => {
				self.remove_impl(entry)?;
				Ok(true)
			}
			None => Ok(false),
		}
	}
	fn size(&self) -> usize {
		self.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.clear_impl()
	}

	fn iter<'b>(&'b self) -> HashsetIterator<'b, K> {
		HashsetIterator::new(self, self.tail)
	}
	fn max_load_factor(&self) -> f64 {
		self.max_load_factor
	}
	fn max_entries(&self) -> usize {
		self.max_entries
	}
}

impl<V> List<V> for HashImpl<V>
where
	V: Serializable + Debug + Clone,
{
	fn push(&mut self, value: V) -> Result<(), Error> {
		self.insert_impl::<V>(Some(&value), None, None, None, None, false)
	}

	fn iter<'b>(&'b self) -> Box<dyn Iterator<Item = V> + 'b> {
		Box::new(ListIterator::new(self, self.head, Direction::Forward))
	}
	fn iter_rev<'b>(&'b self) -> Box<dyn Iterator<Item = V> + 'b> {
		Box::new(ListIterator::new(self, self.tail, Direction::Backward))
	}
	fn delete_head(&mut self) -> Result<(), Error> {
		self.delete_head_impl()
	}
	fn size(&self) -> usize {
		self.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.clear_impl()
	}
}

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::hash::CACHE_BUFFER_SIZE;
	use crate::types::{HashImpl, HashImplSync, Hashset, List};
	use crate::ConfigOption::{SlabCount, SlabSize};
	use crate::{
		block_on, execute, hashset, hashtable, list, list_append, list_eq, lock, slab_allocator,
		thread_pool, Builder, HashsetConfig, HashsetIterator, Hashtable, HashtableConfig,
		HashtableIterator, ListConfig, Lock, Reader, Serializable, SlabAllocatorConfig,
		SortableList, ThreadPool, Writer, GLOBAL_SLAB_ALLOCATOR,
	};
	use bmw_deps::rand::random;
	use bmw_err::*;
	use bmw_log::*;
	use std::cell::RefMut;
	use std::collections::HashMap;

	info!();

	#[test]
	fn test_static_hashtable() -> Result<(), Error> {
		let free_count1;

		{
			let mut hashtable = Builder::build_hashtable(
				HashtableConfig {
					max_entries: 100,
					..Default::default()
				},
				&None,
			)?;
			free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
			})?;

			hashtable.insert(&1, &2)?;
			let v = hashtable.get(&1)?;
			assert_eq!(v.unwrap(), 2);
			assert_eq!(hashtable.size(), 1);
			assert_eq!(hashtable.get(&2)?, None);
			hashtable.insert(&1, &3)?;
			assert_eq!(hashtable.get(&1)?, Some(3));
			assert_eq!(hashtable.size(), 1);
			let free_count3 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
			})?;
			assert_eq!(free_count3, free_count1 - 1);
		}

		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;

		assert_eq!(free_count1, free_count2);

		Ok(())
	}

	#[test]
	fn test_remove_static_hashtable() -> Result<(), Error> {
		let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
		hashtable.insert(&1, &2)?;
		let v = hashtable.get(&1)?;
		assert_eq!(v.unwrap(), 2);
		assert_eq!(hashtable.size(), 1);
		assert_eq!(hashtable.remove(&2)?, None);
		assert_eq!(hashtable.remove(&1)?, Some(2));
		assert_eq!(hashtable.remove(&1)?, None);
		assert_eq!(hashtable.size(), 0);

		Ok(())
	}

	#[test]
	fn test_compare() -> Result<(), Error> {
		let mut keys = vec![];
		let mut values = vec![];
		for _ in 0..1_000 {
			keys.push(random::<u32>());
			values.push(random::<u32>());
		}
		let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
		let mut hashmap = HashMap::new();
		for i in 0..1_000 {
			hashtable.insert(&keys[i], &values[i])?;
			hashmap.insert(&keys[i], &values[i]);
		}

		for _ in 0..100 {
			let index: usize = random::<usize>() % 1_000;
			hashtable.remove(&keys[index])?;
			hashmap.remove(&keys[index]);
		}

		let mut i = 0;
		for (k, vm) in &hashmap {
			let vt = hashtable.get(&k)?;
			assert_eq!(&vt.unwrap(), *vm);
			i += 1;
		}

		assert_eq!(i, hashtable.size());
		assert_eq!(i, hashmap.len());

		Ok(())
	}

	#[test]
	fn test_iterator() -> Result<(), Error> {
		let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
		hashtable.insert(&1, &10)?;
		hashtable.insert(&2, &20)?;
		hashtable.insert(&3, &30)?;
		hashtable.insert(&4, &40)?;
		let size = hashtable.size();
		let mut i = 0;
		for (k, v) in hashtable.iter() {
			info!("k={},v={}", k, v)?;
			assert_eq!(hashtable.get(&k)?, Some(v));
			i += 1;
		}

		assert_eq!(i, 4);
		assert_eq!(size, i);

		hashtable.remove(&3)?;
		let size = hashtable.size();
		let mut i = 0;
		for (k, v) in hashtable.iter() {
			info!("k={},v={}", k, v)?;
			assert_eq!(hashtable.get(&k)?, Some(v));
			i += 1;
		}
		assert_eq!(i, 3);
		assert_eq!(size, i);

		hashtable.remove(&4)?;
		let size = hashtable.size();
		let mut i = 0;
		for (k, v) in hashtable.iter() {
			info!("k={},v={}", k, v)?;
			assert_eq!(hashtable.get(&k)?, Some(v));
			i += 1;
		}
		assert_eq!(i, 2);
		assert_eq!(size, i);

		Ok(())
	}

	#[test]
	fn test_clear() -> Result<(), Error> {
		let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
		let free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count1)?;

		hashtable.insert(&1, &10)?;
		hashtable.insert(&2, &20)?;
		hashtable.insert(&3, &30)?;
		hashtable.insert(&4, &40)?;
		let size = hashtable.size();
		let mut i = 0;
		for (k, v) in hashtable.iter() {
			info!("k={},v={}", k, v)?;
			assert_eq!(hashtable.get(&k)?, Some(v));
			i += 1;
		}

		assert_eq!(i, 4);
		assert_eq!(size, i);

		hashtable.clear()?;
		assert_eq!(hashtable.size(), 0);

		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count2)?;
		assert_eq!(free_count1, free_count2);

		Ok(())
	}

	#[test]
	fn test_hashtable_drop() -> Result<(), Error> {
		let free_count1;
		{
			let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
			free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
			})?;
			info!("free_count={}", free_count1)?;

			hashtable.insert(&1, &10)?;
			hashtable.insert(&2, &20)?;
			hashtable.insert(&3, &30)?;
			hashtable.insert(&4, &40)?;
			let size = hashtable.size();
			let mut i = 0;
			for (k, v) in hashtable.iter() {
				info!("k={},v={}", k, v)?;
				assert_eq!(hashtable.get(&k)?, Some(v));
				i += 1;
			}

			assert_eq!(i, 4);
			assert_eq!(size, i);
		}

		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count2)?;
		assert_eq!(free_count1, free_count2);

		Ok(())
	}

	#[test]
	fn test_hashset1() -> Result<(), Error> {
		let mut hashset = Builder::build_hashset::<i32>(HashsetConfig::default(), &None)?;
		hashset.insert(&1)?;
		hashset.insert(&2)?;
		hashset.insert(&3)?;
		hashset.insert(&4)?;
		let size = hashset.size();
		let mut i = 0;
		for k in hashset.iter() {
			info!("k={}", k)?;
			assert_eq!(hashset.contains(&k)?, true);
			i += 1;
		}

		assert_eq!(i, 4);
		assert_eq!(size, i);

		hashset.remove(&3)?;
		let size = hashset.size();
		let mut i = 0;
		for k in hashset.iter() {
			info!("k={}", k)?;
			assert_eq!(hashset.contains(&k)?, true);
			i += 1;
		}
		assert_eq!(i, 3);
		assert_eq!(size, i);

		hashset.remove(&4)?;
		let size = hashset.size();
		let mut i = 0;
		for k in hashset.iter() {
			info!("k={}", k)?;
			assert_eq!(hashset.contains(&k)?, true);
			i += 1;
		}
		assert_eq!(i, 2);
		assert_eq!(size, i);
		hashset.clear()?;
		assert_eq!(hashset.size(), 0);

		assert_eq!(hashset.remove(&0)?, false);

		Ok(())
	}

	#[test]
	fn test_list1() -> Result<(), Error> {
		let mut list = Builder::build_list(ListConfig::default(), &None)?;
		list.push(1)?;
		list.push(2)?;
		list.push(3)?;
		list.push(4)?;
		list.push(5)?;
		list.push(6)?;
		let mut i = 0;
		for x in list.iter() {
			info!("valuetest_fwd={}", x)?;
			i += 1;
			if i > 10 {
				break;
			}
		}

		let mut i = 0;
		for x in list.iter_rev() {
			info!("valuetest_rev={}", x)?;
			i += 1;
			if i > 10 {
				break;
			}
		}

		Ok(())
	}

	#[test]
	fn test_small_slabs() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(8))?;
		let mut table = Builder::build_hashtable(
			HashtableConfig {
				max_entries: 100,
				..Default::default()
			},
			&Some(&slabs),
		)?;

		table.insert(&1u8, &1u8)?;
		table.insert(&2u8, &2u8)?;

		let mut count = 0;
		for (k, v) in table.iter() {
			match k {
				1u8 => assert_eq!(v, 1u8),
				_ => assert_eq!(v, 2u8),
			}
			count += 1;
		}

		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_small_config() -> Result<(), Error> {
		let slab_size = 12;
		let slabs = Builder::build_slabs_ref();
		let config = SlabAllocatorConfig {
			slab_size,
			slab_count: 1,
			..Default::default()
		};
		{
			let mut slabs: RefMut<_> = slabs.borrow_mut();
			slabs.init(config)?;
		}

		{
			let config = HashtableConfig {
				max_entries: 1,
				..Default::default()
			};
			let mut h = Builder::build_hashtable(config, &Some(&slabs))?;

			info!("insert 1")?;
			assert!(h.insert(&2u64, &6u64).is_err());
			info!("insert 2")?;
			let mut h = Builder::build_hashtable(config, &Some(&slabs))?;
			h.insert(&2000u32, &1000u32)?;
		}
		Ok(())
	}
	#[test]
	fn test_sync_hashtable() -> Result<(), Error> {
		let slab_config = SlabAllocatorConfig {
			slab_size: 1024,
			slab_count: 1024,
			..Default::default()
		};

		let config = HashtableConfig {
			max_entries: 1024,
			..Default::default()
		};

		let h = Builder::build_hashtable_sync(config, slab_config)?;
		assert!(h.slabs().is_ok());
		let mut h = lock!(h)?;
		let mut h_clone = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

		{
			let h2 = h_clone.rlock()?;
			assert_eq!((**h2.guard()).get(&2u64)?, None);
			assert_eq!((**h2.guard()).size(), 0);
			assert_eq!((**h2.guard()).max_load_factor(), config.max_load_factor);
			assert_eq!((**h2.guard()).max_entries(), config.max_entries);
		}

		let handle = execute!(tp, {
			let mut h = h.wlock()?;
			(**h.guard()).insert(&2u64, &6u64)?;
			(**h.guard()).insert(&3u64, &6u64)?;
			Ok(())
		})?;

		block_on!(handle);

		{
			let h = h_clone.rlock()?;
			assert_eq!((**h.guard()).get(&2u64)?, Some(6u64));
		}

		{
			let mut h = h_clone.wlock()?;
			(**h.guard()).remove(&2u64)?;
			assert_eq!((**h.guard()).get(&2u64)?, None);
			assert_eq!((**h.guard()).remove(&2u64)?, None);
		}

		{
			let mut h = h_clone.wlock()?;
			let mut iter = (**h.guard()).iter();
			assert_eq!(iter.next(), Some((3u64, 6u64)));
			assert_eq!(iter.next(), None);
		}

		{
			let mut h = h_clone.wlock()?;
			assert_eq!((**h.guard()).size(), 1);
			(**h.guard()).clear()?;
			assert_eq!((**h.guard()).size(), 0);
		}

		Ok(())
	}

	#[test]
	fn test_sync_hashset() -> Result<(), Error> {
		let slab_config = SlabAllocatorConfig {
			slab_size: 1024,
			slab_count: 1024,
			..Default::default()
		};

		let config = HashsetConfig {
			max_entries: 1024,
			..Default::default()
		};

		let h = Builder::build_hashset_sync(config, slab_config)?;
		let mut h = lock!(h)?;
		let h_clone = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

		{
			let h2 = h_clone.rlock()?;
			assert_eq!((**h2.guard()).contains(&2u64)?, false);
		}

		let handle = execute!(tp, {
			let mut h = h.wlock()?;
			(**h.guard()).insert(&2u64)?;
			Ok(())
		})?;

		block_on!(handle);

		let h = h_clone.rlock()?;
		assert_eq!((**h.guard()).contains(&2u64)?, true);

		let mut iter = (**h.guard()).iter();
		assert_eq!(iter.next(), Some(2u64));
		assert_eq!(iter.next(), None);

		Ok(())
	}

	#[test]
	fn test_sync_list() -> Result<(), Error> {
		let slab_config = SlabAllocatorConfig {
			slab_size: 1024,
			slab_count: 1024,
			..Default::default()
		};

		let config = ListConfig {};

		let h = Builder::build_list_sync(config.clone(), slab_config.clone())?;
		let mut h = lock!(h)?;
		let h_clone = h.clone();
		let mut h_clone2 = h.clone();
		let mut h_clone3 = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

		{
			let h = h_clone.rlock()?;
			assert_eq!((**h.guard()).size(), 0);
		}

		let handle = execute!(tp, {
			let mut h = h.wlock()?;
			(**h.guard()).push(2u64)?;
			Ok(())
		})?;

		block_on!(handle);

		{
			let h = h_clone.rlock()?;
			assert_eq!((**h.guard()).size(), 1);
		}

		{
			let h = h_clone.rlock()?;
			let mut iter = (**h.guard()).iter();
			assert_eq!(iter.next(), Some(2u64));
			assert_eq!(iter.next(), None);

			let mut iter = (**h.guard()).iter_rev();
			assert_eq!(iter.next(), Some(2u64));
			assert_eq!(iter.next(), None);
		}

		{
			let mut h = h_clone2.wlock()?;
			(**h.guard()).push(3u64)?;
			(**h.guard()).push(1u64)?;
		}

		{
			let mut h = h_clone3.wlock()?;
			assert!(list_eq!((**h.guard()), list![2u64, 3, 1]));
			(**h.guard()).sort()?;
			assert!(list_eq!((**h.guard()), list![1u64, 2, 3]));
			(**h.guard()).push(7u64)?;
			(**h.guard()).push(4u64)?;
		}

		{
			let mut h = h_clone3.wlock()?;
			assert!(list_eq!((**h.guard()), list![1u64, 2, 3, 7, 4]));
			(**h.guard()).sort_unstable()?;
			assert!(list_eq!((**h.guard()), list![1u64, 2, 3, 4, 7]));
		}

		let h2 = Builder::build_list_sync::<u64>(config.clone(), slab_config)?;
		let h2 = lock!(h2)?;
		let mut h2_clone = h2.clone();
		{
			let mut h = h2_clone.wlock()?;
			(**h.guard()).push(1u64)?;
			(**h.guard()).push(2u64)?;
			(**h.guard()).push(3u64)?;
			(**h.guard()).push(4u64)?;
			(**h.guard()).push(7u64)?;
		}

		{
			let h = h_clone3.rlock()?;
			let h2 = h2_clone.rlock()?;
			info!("h={:?},h2={:?}", **h.guard(), **h2.guard())?;
			assert!(list_eq!(**h.guard(), **h2.guard()));
		}

		let x: HashImplSync<u32> = HashImplSync::new(
			None,
			None,
			Some(config.clone()),
			SlabAllocatorConfig::default(),
		)?;

		let mut x2: HashImplSync<u32> = HashImplSync::new(
			None,
			None,
			Some(config.clone()),
			SlabAllocatorConfig::default(),
		)?;

		assert_eq!(x, x2);
		x2.push(1)?;
		assert_ne!(x, x2);
		assert_eq!(List::size(&x), 0);
		assert_eq!(List::size(&x2), 1);

		assert_eq!(Hashset::size(&x), 0);
		assert_eq!(Hashset::size(&x2), 1);

		List::delete_head(&mut x2)?;
		assert_eq!(List::size(&x2), 0);

		x2.push(1)?;
		x2.push(2)?;
		x2.push(3)?;

		assert_eq!(List::size(&x2), 3);
		List::clear(&mut x2)?;
		assert_eq!(List::size(&x2), 0);

		Ok(())
	}

	#[test]
	fn test_sync_hashset2() -> Result<(), Error> {
		let config = HashsetConfig::default();
		let mut hashset =
			Builder::build_hashset_sync::<u32>(config, SlabAllocatorConfig::default())?;

		hashset.insert(&1)?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&1)?);
		assert!(!hashset.contains(&2)?);
		assert_eq!(hashset.remove(&1)?, true);
		assert_eq!(hashset.remove(&1)?, false);
		assert_eq!(hashset.size(), 0);
		assert_eq!(hashset.max_load_factor(), config.max_load_factor);
		assert_eq!(hashset.max_entries(), config.max_entries);

		hashset.insert(&1)?;
		hashset.clear()?;
		assert_eq!(hashset.size(), 0);

		Ok(())
	}

	struct TestHashtableBox {
		h: Box<dyn Hashtable<u32, u32>>,
	}

	#[test]
	fn test_hashtable_box() -> Result<(), Error> {
		let config = HashtableConfig {
			..Default::default()
		};

		let h = Builder::build_hashtable_box(config, &None)?;
		let mut thtb = TestHashtableBox { h };

		let x = 1;
		thtb.h.insert(&x, &2)?;
		assert_eq!(thtb.h.get(&x)?, Some(2));

		Ok(())
	}

	#[test]
	fn test_list_boxed() -> Result<(), Error> {
		let mut list1 = Builder::build_list_box(ListConfig {}, &None)?;
		list1.push(1)?;
		list1.push(2)?;

		let mut list2 = Builder::build_list(ListConfig {}, &None)?;
		list2.push(1)?;
		list2.push(2)?;

		//list_append!(list1, list2);

		assert!(list_eq!(list1, list2));

		let list3 = list![1, 2, 1, 2];
		list_append!(list1, list2);
		assert!(list_eq!(list1, list3));

		let mut list4 = Builder::build_array_list(100, &0)?;
		list4.push(1)?;
		list4.push(2)?;
		list4.push(1)?;
		list4.push(2)?;
		assert!(list_eq!(list1, list4));

		Ok(())
	}

	#[test]
	fn test_delete_head() -> Result<(), Error> {
		let free_count1;
		{
			let mut list = list![1, 2, 3, 4];
			free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
				Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
			})? + 4;

			list.delete_head()?;
		}

		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;

		assert_eq!(free_count1, free_count2);
		Ok(())
	}

	#[test]
	fn test_sort_linked() -> Result<(), Error> {
		let mut list = list![1, 2, 3, 7, 5];
		list.sort()?;
		info!("list={:?}", list)?;

		let other_list = list![1, 2, 3, 5, 7];
		assert!(list_eq!(other_list, list));
		Ok(())
	}

	#[test]
	fn test_debug() -> Result<(), Error> {
		let mut hashset = hashset!()?;
		hashset.insert(&1)?;
		hashset.insert(&2)?;
		hashset.insert(&1)?;
		info!("hashset={:?}", hashset)?;

		let mut hashtable = hashtable!()?;
		assert!(hashtable.slabs().is_ok());
		hashtable.insert(&1, &10)?;
		hashtable.insert(&2, &20)?;
		hashtable.insert(&1, &10)?;
		info!("hashtable={:?}", hashtable)?;
		Ok(())
	}

	#[test]
	fn test_hash_impl_internal_errors() -> Result<(), Error> {
		let mut hash_impl: HashImpl<u32> =
			HashImpl::new(Some(HashtableConfig::default()), None, None, &None, false)?;
		hash_impl.set_debug_get_next_slot_error(true);
		Hashtable::insert(&mut hash_impl, &0, &0u32)?;

		{
			let mut iter: HashtableIterator<'_, u32, u32> = Hashtable::iter(&mut hash_impl);
			// none because error occurs in the get_next_slot fn
			assert!(iter.next().is_none());
		}

		{
			let mut iter: HashsetIterator<'_, u32> = Hashset::iter(&mut hash_impl);
			// same with hashset iterator
			assert!(iter.next().is_none());
		}

		hash_impl.set_debug_get_next_slot_error(false);

		{
			let mut iter: HashtableIterator<'_, u32, u32> = Hashtable::iter(&mut hash_impl);
			// no error occurs this time
			assert!(iter.next().is_some());
		}

		{
			let mut iter: HashsetIterator<'_, u32> = Hashset::iter(&mut hash_impl);
			// also no error
			assert!(iter.next().is_some());
		}

		hash_impl.set_debug_get_next_slot_error(true);

		Ok(())
	}

	#[test]
	fn test_hash_impl_aslist_internal_errors() -> Result<(), Error> {
		let mut hash_impl: HashImpl<u32> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		assert!(hash_impl.get_impl(&0, 0).is_err());
		hash_impl.set_debug_get_next_slot_error(true);
		List::push(&mut hash_impl, 0)?;
		{
			let mut iter: Box<dyn Iterator<Item = u32>> = List::iter(&mut hash_impl);
			// none because error occurs in the get_next_slot fn
			assert!(iter.next().is_none());
		}

		hash_impl.set_debug_get_next_slot_error(false);

		{
			let mut iter: Box<dyn Iterator<Item = u32>> = List::iter(&mut hash_impl);
			// now it's found
			assert!(iter.next().is_some());
		}

		Ok(())
	}

	#[test]
	fn test_debug_entry_array_len() -> Result<(), Error> {
		let mut hash_impl: HashImpl<u32> =
			HashImpl::new(Some(HashtableConfig::default()), None, None, &None, false)?;
		Hashtable::insert(&mut hash_impl, &1, &2)?;
		hash_impl.set_debug_entry_array_len(true);
		assert!(hash_impl.get_impl(&1, 0).is_err());
		assert!(Hashtable::insert(&mut hash_impl, &3, &2).is_err());
		Ok(())
	}

	#[derive(Debug, PartialEq, Clone, Hash)]
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

	#[test]
	fn test_hash_impl_ser_err() -> Result<(), Error> {
		let mut hash_impl: HashImpl<SerErr> =
			HashImpl::new(Some(HashtableConfig::default()), None, None, &None, false)?;
		Hashtable::insert(&mut hash_impl, &SerErr { exp: 100, empty: 0 }, &0)?;
		let res: Result<Option<u32>, Error> =
			Hashtable::get(&mut hash_impl, &SerErr { exp: 100, empty: 0 });
		assert_eq!(
			res,
			Err(err!(ErrKind::CorruptedData, "expected: 99, received: 100"))
		);

		let mut iter: HashtableIterator<SerErr, u32> = Hashtable::iter(&mut hash_impl);
		assert_eq!(iter.next(), None);

		// we can also get the error with the hashset iterator (value is ignored)
		let mut iter: HashsetIterator<SerErr> = Hashset::iter(&mut hash_impl);
		assert_eq!(iter.next(), None);

		// hashtable will work other than this entry
		Hashtable::insert(&mut hash_impl, &SerErr { exp: 99, empty: 0 }, &1)?;
		assert_eq!(
			Hashtable::get(&mut hash_impl, &SerErr { exp: 99, empty: 0 })?,
			Some(1)
		);

		Ok(())
	}

	#[test]
	fn test_hash_impl_aslist_ser_err() -> Result<(), Error> {
		let mut hash_impl: HashImpl<SerErr> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		hash_impl.push(SerErr { exp: 100, empty: 0 })?;
		let mut iter: Box<dyn Iterator<Item = SerErr>> = List::iter(&hash_impl);
		assert_eq!(iter.next(), None);

		let mut hash_impl: HashImpl<SerErr> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;
		{
			let mut iter: Box<dyn Iterator<Item = SerErr>> = List::iter(&hash_impl);
			assert_eq!(iter.next(), Some(SerErr { exp: 99, empty: 0 }));
		}

		let mut hash_impl2: HashImpl<SerErr> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;

		// lengths unequal
		assert_ne!(hash_impl, hash_impl2);

		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;

		// now contents are equal
		assert_eq!(hash_impl, hash_impl2);

		let mut hash_impl: HashImpl<u32> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		let mut hash_impl2: HashImpl<u32> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, false)?;
		hash_impl2.push(1)?;
		hash_impl.push(8)?;

		// the value is not equal
		assert_ne!(hash_impl, hash_impl2);

		Ok(())
	}

	#[test]
	fn test_hash_impl_error_conditions() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(100_000), SlabCount(1))?;
		let hashtable =
			Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs));
		assert!(hashtable.is_err());

		let hash_impl: Result<HashImpl<u32>, Error> =
			HashImpl::new(None, None, Some(ListConfig {}), &None, true);
		assert!(hash_impl.is_err());

		let hashtable = Builder::build_hashtable::<u32, u32>(
			HashtableConfig {
				max_entries: 0,
				..Default::default()
			},
			&None,
		);
		assert!(hashtable.is_err());

		let hashtable = Builder::build_hashtable::<u32, u32>(
			HashtableConfig {
				max_load_factor: 2.0,
				..Default::default()
			},
			&None,
		);
		assert!(hashtable.is_err());

		let hashset = Builder::build_hashset::<u32>(
			HashsetConfig {
				max_entries: 0,
				..Default::default()
			},
			&None,
		);
		assert!(hashset.is_err());

		let hashset = Builder::build_hashset::<u32>(
			HashsetConfig {
				max_load_factor: 2.0,
				..Default::default()
			},
			&None,
		);
		assert!(hashset.is_err());

		let slabs = slab_allocator!(SlabSize(8), SlabCount(1))?;
		let hashset = Builder::build_hashset::<u32>(
			HashsetConfig {
				max_entries: 10_000_000,
				..Default::default()
			},
			&Some(&slabs),
		);
		assert_eq!(
			hashset.unwrap_err(),
			err!(
				ErrKind::Configuration,
				"SlabSize is too small. Must be at least 12"
			)
		);

		Ok(())
	}

	#[test]
	fn test_hashset_key_write_error() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(12), SlabCount(1))?;
		let mut hashset = Builder::build_hashset::<u128>(HashsetConfig::default(), &Some(&slabs))?;
		let e = hashset.insert(&1).unwrap_err().kind();
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = slabs.borrow();
		assert_eq!(slabs.free_count()?, 1);
		Ok(())
	}

	#[test]
	fn test_hashtable_value_write_error() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(30), SlabCount(1))?;
		let mut hashtable =
			Builder::build_hashtable::<u128, u128>(HashtableConfig::default(), &Some(&slabs))?;
		let e = hashtable.insert(&1, &2).unwrap_err().kind();
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = slabs.borrow();
		assert_eq!(slabs.free_count()?, 1);
		Ok(())
	}

	#[test]
	fn test_hashtable_value_write_error_multi_slab() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(16), SlabCount(2))?;
		let mut hashtable =
			Builder::build_hashtable::<u128, u128>(HashtableConfig::default(), &Some(&slabs))?;
		let e = hashtable.insert(&1, &2).unwrap_err().kind();
		info!("e={}", e)?;
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = slabs.borrow();
		assert_eq!(slabs.free_count()?, 2);
		Ok(())
	}

	#[test]
	fn test_hashtable_writer_full_error() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(1))?;
		{
			let mut hashset =
				Builder::build_hashset::<u128>(HashsetConfig::default(), &Some(&slabs))?;
			hashset.insert(&2)?;
			let e = hashset.insert(&1).unwrap_err().kind();
			let m = matches!(e, ErrorKind::CapacityExceeded(_));
			assert!(m);
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}
		let slabs = slabs.borrow();
		assert_eq!(slabs.free_count()?, 1);
		Ok(())
	}

	#[test]
	fn test_hashset_load_factor() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(128), SlabCount(100))?;
		let mut hashset = Builder::build_hashset::<u128>(
			HashsetConfig {
				max_entries: 10,
				max_load_factor: 1.0,
				..Default::default()
			},
			&Some(&slabs),
		)?;

		for i in 0..10 {
			hashset.insert(&(i as u128))?;
		}

		assert!(hashset.insert(&10u128).is_err());

		Ok(())
	}

	#[test]
	fn test_remove_oldest() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(10))?;
		{
			let mut hashtable =
				Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs))?;
			hashtable.insert(&1, &4)?;
			hashtable.insert(&2, &5)?;
			hashtable.insert(&3, &6)?;
			{
				let slabs = slabs.borrow();
				assert_eq!(slabs.free_count()?, 7);
			}

			assert_eq!(hashtable.get(&1), Ok(Some(4)));
			assert_eq!(hashtable.get(&2), Ok(Some(5)));
			assert_eq!(hashtable.get(&3), Ok(Some(6)));
			hashtable.remove_oldest()?;
			{
				let slabs = slabs.borrow();
				assert_eq!(slabs.free_count()?, 8);
			}
			assert_eq!(hashtable.get(&1), Ok(None));
			assert_eq!(hashtable.get(&2), Ok(Some(5)));
			assert_eq!(hashtable.get(&3), Ok(Some(6)));
		}
		Ok(())
	}

	#[test]
	fn test_bring_to_front_mid_tail() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(10))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs))?;
		hashtable.insert(&0, &4)?;
		hashtable.insert(&1, &5)?;
		hashtable.insert(&2, &6)?;

		hashtable.bring_to_front(&1)?;
		info!("bring to front 1")?;

		hashtable.remove_oldest()?;

		hashtable.remove_oldest()?;
		assert_eq!(hashtable.get(&1), Ok(Some(5)));

		hashtable.insert(&3, &7)?;
		hashtable.insert(&4, &8)?;

		hashtable.bring_to_front(&4)?;
		info!("bring to front complete")?;
		assert_eq!(hashtable.get(&3), Ok(Some(7)));
		assert_eq!(hashtable.get(&4), Ok(Some(8)));
		assert_eq!(hashtable.get(&1), Ok(Some(5)));

		info!("remove oldest start")?;
		hashtable.remove_oldest()?;
		info!("remove oldest complete")?;

		assert_eq!(hashtable.get(&3), Ok(Some(7)));
		assert_eq!(hashtable.get(&4), Ok(Some(8)));
		assert_eq!(hashtable.get(&1), Ok(None));
		hashtable.remove_oldest()?;
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(Some(8)));
		assert_eq!(hashtable.get(&1), Ok(None));
		hashtable.remove_oldest()?;
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));

		info!("end assertions")?;
		Ok(())
	}

	#[test]
	fn test_bring_to_front_head() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(10))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs))?;
		hashtable.insert(&0, &4)?;
		hashtable.insert(&1, &5)?;
		hashtable.insert(&2, &6)?;
		info!("bring to front")?;
		hashtable.bring_to_front(&0)?;
		info!("end bring to front")?;

		hashtable.remove_oldest()?;

		info!("end rem")?;

		assert_eq!(hashtable.get(&0), Ok(Some(4)));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(Some(6)));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(Some(4)));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(None));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(None));

		hashtable.insert(&0, &10)?;

		hashtable.bring_to_front(&0)?;

		assert_eq!(hashtable.get(&0), Ok(Some(10)));

		hashtable.insert(&0, &10)?;
		hashtable.insert(&1, &11)?;
		hashtable.insert(&2, &12)?;
		hashtable.insert(&3, &13)?;
		hashtable.insert(&4, &14)?;
		hashtable.insert(&5, &15)?;

		hashtable.bring_to_front(&2)?;

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(Some(11)));
		assert_eq!(hashtable.get(&2), Ok(Some(12)));
		assert_eq!(hashtable.get(&3), Ok(Some(13)));
		assert_eq!(hashtable.get(&4), Ok(Some(14)));
		assert_eq!(hashtable.get(&5), Ok(Some(15)));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(Some(12)));
		assert_eq!(hashtable.get(&3), Ok(Some(13)));
		assert_eq!(hashtable.get(&4), Ok(Some(14)));
		assert_eq!(hashtable.get(&5), Ok(Some(15)));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(Some(12)));
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(Some(14)));
		assert_eq!(hashtable.get(&5), Ok(Some(15)));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(Some(12)));
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(None));
		assert_eq!(hashtable.get(&5), Ok(Some(15)));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(Some(12)));
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(None));
		assert_eq!(hashtable.get(&5), Ok(None));

		hashtable.remove_oldest()?;

		assert_eq!(hashtable.get(&0), Ok(None));
		assert_eq!(hashtable.get(&1), Ok(None));
		assert_eq!(hashtable.get(&2), Ok(None));
		assert_eq!(hashtable.get(&3), Ok(None));
		assert_eq!(hashtable.get(&4), Ok(None));
		assert_eq!(hashtable.get(&5), Ok(None));

		hashtable.bring_to_front(&1)?;
		Ok(())
	}

	#[test]
	fn test_hashtable_raw() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(1_000))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs))?;

		let mut data2 = [0u8; CACHE_BUFFER_SIZE];
		let data = [8u8; CACHE_BUFFER_SIZE];
		hashtable.raw_write(&0, 0, &data, CACHE_BUFFER_SIZE)?;
		hashtable.raw_read(&0, 0, &mut data2)?;
		assert_eq!(data, data2);

		let data = [10u8; CACHE_BUFFER_SIZE];
		hashtable.raw_write(&7, 383, &data, CACHE_BUFFER_SIZE)?;
		hashtable.raw_read(&7, 383, &mut data2)?;
		assert_eq!(data, data2);
		assert!(!hashtable.raw_read(&9, 384, &mut data2)?);

		Ok(())
	}

	#[test]
	fn test_hashtable_sync_raw() -> Result<(), Error> {
		let mut hashtable = Builder::build_hashtable_sync::<u32, u32>(
			HashtableConfig::default(),
			SlabAllocatorConfig::default(),
		)?;

		let mut data2 = [0u8; CACHE_BUFFER_SIZE];
		let data = [8u8; CACHE_BUFFER_SIZE];
		hashtable.raw_write(&0, 0, &data, CACHE_BUFFER_SIZE)?;
		assert!(hashtable.raw_read(&0, 0, &mut data2)?);
		assert_eq!(data, data2);

		let data = [10u8; CACHE_BUFFER_SIZE];
		hashtable.raw_write(&7, 383, &data, CACHE_BUFFER_SIZE)?;
		assert!(hashtable.raw_read(&7, 383, &mut data2)?);
		assert_eq!(data, data2);

		assert!(!hashtable.raw_read(&9, 384, &mut data2)?);
		assert!(hashtable.slabs().is_ok());
		assert!(hashtable.bring_to_front(&7).is_ok());
		assert!(hashtable.remove_oldest().is_ok());

		Ok(())
	}

	#[test]
	fn test_hashtable_raw_overwrite() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(1_000))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, u32>(HashtableConfig::default(), &Some(&slabs))?;

		let mut data2 = [0u8; CACHE_BUFFER_SIZE];
		let empty = [4u8; CACHE_BUFFER_SIZE];
		let data = [10u8; CACHE_BUFFER_SIZE];

		info!("raw_write at 1383")?;
		hashtable.raw_write(&7, 1383, &data, CACHE_BUFFER_SIZE)?;
		info!("raw write at 0")?;
		hashtable.raw_write(&7, 0, &empty, CACHE_BUFFER_SIZE)?;
		info!("raw read at 1383")?;
		hashtable.raw_read(&7, 1383, &mut data2)?;
		assert_eq!(data, data2);
		Ok(())
	}

	#[test]
	fn test_multi_remove_oldest() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(3))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, String>(HashtableConfig::default(), &Some(&slabs))?;
		hashtable.insert(&1, &"4".to_string())?;
		hashtable.insert(&2, &"5".to_string())?;
		hashtable.insert(&3, &"6".to_string())?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 2);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.insert(&1, &"0123456789012345678901234".to_string())?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.insert(&1, &"012345678901234567890123456789".to_string())?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.insert(&1, &"4".to_string())?;
		hashtable.insert(&2, &"0123456789012345678901234".to_string())?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.insert(&2, &"0123456789012345678901234".to_string())?;
		hashtable.insert(&1, &"4".to_string())?;

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 2);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		hashtable.insert(&1, &"012345678901234567890123456789".to_string())?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 3);
		}

		Ok(())
	}

	#[test]
	fn test_insert_with_big_small_big_rem_all() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(25), SlabCount(64))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, String>(HashtableConfig::default(), &Some(&slabs))?;

		let mut big_string = "".to_string();
		let string10 = "0123456789".to_string();
		for _ in 0..137 {
			big_string = format!("{}{}", big_string, string10);
		}
		hashtable.insert(&1, &big_string)?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 64);
		}

		hashtable.insert(&2, &"ok".to_string())?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 63);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 64);
		}

		hashtable.insert(&1, &big_string)?;

		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = slabs.borrow();
			assert_eq!(slabs.free_count()?, 64);
		}
		Ok(())
	}

	#[test]
	fn test_raw() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(250), SlabCount(3))?;
		let mut hashtable =
			Builder::build_hashtable::<u32, String>(HashtableConfig::default(), &Some(&slabs))?;

		let bytes = [3u8; CACHE_BUFFER_SIZE];
		hashtable.raw_write(&1, 0, &bytes, CACHE_BUFFER_SIZE)?;
		hashtable.raw_write(&1, 4, &bytes, CACHE_BUFFER_SIZE)?;
		hashtable.remove_oldest()?;

		Ok(())
	}
}
