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

use crate::constants::*;
use crate::misc::{set_max, slice_to_usize, usize_to_slice};
use crate::types::{Direction, HashImpl, HashImplSync};
use crate::{
	Hashset, HashsetConfig, HashsetIterator, Hashtable, HashtableConfig, HashtableIterator, List,
	ListConfig, ListIterator, LockBox, SlabAllocator, SlabAllocatorConfig, SlabReader, SlabWriter,
	SortableList, UtilBuilder, GLOBAL_SLAB_ALLOCATOR,
};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_err::*;
use bmw_log::*;
use bmw_ser::{Reader, Serializable, Writer};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
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

	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		let list = self.linked_list_ref;
		if list.size == 0 {
			return None;
		}
		let slot = self.cur;
		match list.get_next_slot(&mut self.cur, self.direction, &mut self.slab_reader) {
			Ok(ret) => {
				if ret {
					// seek the location in the list for this
					// slot
					self.slab_reader.seek(slot, list.ptr_size * 2);
					match V::read(&mut self.slab_reader) {
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
	fn new(linked_list_ref: &'a HashImpl<V>, cur: usize, direction: Direction) -> Self {
		let _ = debug!("new list iter");
		Self {
			linked_list_ref,
			cur,
			direction,
			_phantom_data: PhantomData,
			slab_reader: linked_list_ref.slab_reader.clone(),
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
			let mut list = UtilBuilder::build_array_list::<V>(self.size, &first)?;
			for l in self.iter() {
				list.push(l)?;
			}
			list.sort()?;
			self.clear()?;
			for l in list.iter() {
				self.push(l)?;
			}
		}
		Ok(())
	}
	fn sort_unstable(&mut self) -> Result<(), Error>
	where
		V: Ord,
	{
		if self.size > 0 {
			let first = self.iter().next().unwrap();
			let mut list = UtilBuilder::build_array_list::<V>(self.size, &first)?;
			for l in self.iter() {
				list.push(l)?;
			}
			list.sort_unstable()?;
			self.clear()?;
			for l in list.iter() {
				self.push(l)?;
			}
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
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let static_impl = HashImpl::new(configs)?;
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
		data: &mut [u8; BUFFER_SIZE],
	) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.static_impl.raw_read_impl(key, hash, offset, data)
	}
	fn raw_write(
		&mut self,
		key: &K,
		off: usize,
		data: &[u8; BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error>
	where
		V: Clone,
	{
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let h = hasher.finish() as usize;
		self.static_impl.raw_write_impl::<V>(key, h, off, data, len)
	}
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error> {
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
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error> {
		self.static_impl.slabs_impl()
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
		let d = Direction::Forward;
		let x = self.static_impl.head;
		Box::new(ListIterator::new(&self.static_impl, x, d))
	}
	fn iter_rev<'b>(&'b self) -> Box<dyn Iterator<Item = V> + 'b> {
		let d = Direction::Backward;
		let x = self.static_impl.tail;
		Box::new(ListIterator::new(&self.static_impl, x, d))
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

struct SaInfo {
	slab_size: usize,
	slab_count: usize,
	slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
}

impl SaInfo {
	fn new(
		slab_size: usize,
		slab_count: usize,
		slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
	) -> Self {
		Self {
			slab_size,
			slab_count,
			slabs,
		}
	}
}

impl<K> HashImpl<K>
where
	K: Serializable + Clone,
{
	// several lines reported as not covered, but they are
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(
			vec![
				CN::IsHashtable,
				CN::IsHashset,
				CN::IsList,
				CN::SlabSize,
				CN::SlabCount,
				CN::MaxEntries,
				CN::MaxLoadFactor,
				CN::DebugLargeSlabCount,
				CN::GlobalSlabAllocator,
			],
			vec![],
		)?;

		let is_hashtable = config.get_or_bool(&CN::IsHashtable, false);
		let is_hashset = config.get_or_bool(&CN::IsHashset, false);
		let is_list = config.get_or_bool(&CN::IsList, false);
		let is_global_slab_allocator = config.get_or_bool(&CN::GlobalSlabAllocator, true);
		let debug_large_slab_count = config.get_or_bool(&CN::DebugLargeSlabCount, false);

		let max_entries = config.get_or_usize(&CN::MaxEntries, HASH_DEFAULT_MAX_ENTRIES);
		let max_load_factor = config.get_or_f64(&CN::MaxLoadFactor, HASH_DEFAULT_MAX_LOAD_FACTOR);

		let slab_size = config.get_or_usize(&CN::SlabSize, HASH_DEFAULT_SLAB_SIZE);
		let slab_count = config.get_or_usize(&CN::SlabCount, HASH_DEFAULT_SLAB_COUNT);

		let max_entries_specified = config.get(&CN::MaxEntries).is_some();
		let max_load_factor_specified = config.get(&CN::MaxLoadFactor).is_some();
		let slab_size_specified = config.get(&CN::SlabSize).is_some();
		let slab_count_specified = config.get(&CN::SlabCount).is_some();

		if is_list && max_entries_specified {
			let text = "MaxEntries not valid for a list";
			return Err(err!(ErrKind::Configuration, text));
		}

		if is_list && max_load_factor_specified {
			let text = "MaxLoadFactor not valid for a list";
			return Err(err!(ErrKind::Configuration, text));
		}

		if !is_hashtable && !is_hashset && !is_list {
			let text = "exactly one of IsHashtable, IsHashset, and IsList must be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		if is_hashtable && (is_hashset || is_list) {
			let text = "exactly one of IsHashtable, IsHashset, and IsList must be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		if is_hashset && (is_hashtable || is_list) {
			let text = "exactly one of IsHashtable, IsHashset, and IsList must be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		if (slab_count_specified && !slab_size_specified)
			|| (slab_size_specified && !slab_count_specified)
		{
			let text = "Either both or neither SlabSize/SlabCount must be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		if is_global_slab_allocator && slab_size_specified {
			let text = "If GlobalSlabAllocator is true, SlabSize/SlabCount must not be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		if !is_global_slab_allocator && !slab_size_specified {
			let text = "If GlobalSlabAllocator is false, SlabSize/SlabCount must be specified";
			return Err(err!(ErrKind::Configuration, text));
		}

		let sa = if slab_size_specified {
			let mut slabs = UtilBuilder::build_sync_slabs();
			let sc = SlabAllocatorConfig {
				slab_size,
				slab_count,
			};
			slabs.init(sc)?;
			let slabs_ret = UtilBuilder::build_lock_box(slabs)?;
			SaInfo::new(slab_size, slab_count, Some(slabs_ret))
		} else {
			GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SaInfo, Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				let slab_size = if slabs.is_init() {
					slabs.slab_size()?
				} else {
					let th = thread::current();
					let n = th.name().unwrap_or("unknown");
					let m1 = "Slab allocator was not initialized for thread";
					let m2 = "Initializing with default values.";
					warn!("WARN: {} '{}'. {}", m1, n, m2)?;
					slabs.init(SlabAllocatorConfig::default())?;
					slabs.slab_size()?
				};
				let slab_count = slabs.slab_count()?;
				Ok(SaInfo::new(slab_size, slab_count, None))
			})?
		};

		let slab_size = sa.slab_size;
		let slab_count = sa.slab_count;
		let slabs = sa.slabs;

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

		if !is_list && max_entries == 0 {
			let fmt = "MaxEntries must be greater than 0";
			let e = err!(ErrKind::Configuration, fmt);
			return Err(e);
		}
		if !is_list && (max_load_factor <= 0.0 || max_load_factor > 1.0) {
			let fmt = "MaxLoadFactor must be greater than 0 and less than or equal to 1.0";
			let e = err!(ErrKind::Configuration, fmt);
			return Err(e);
		}

		let (entry_array, ptr_size) = if is_list {
			let mut x = slab_count;
			let mut ptr_size = 0;
			loop {
				cbreak!(x == 0);
				x >>= 8;
				ptr_size += 1;
			}
			debug!("ptr_size={}", ptr_size)?;
			(None, ptr_size)
		} else {
			let size: usize = (max_entries as f64 / max_load_factor).ceil() as usize;
			let entry_array = UtilBuilder::build_array(size, &SLOT_EMPTY)?;
			debug!("entry array init to size = {}", size)?;
			let mut x = entry_array.size() + 2; // two more, one for deleted and one for empty
			let mut ptr_size = 0;
			loop {
				cbreak!(x == 0);
				x >>= 8;
				ptr_size += 1;
			}
			debug!("ptr_size={}", ptr_size)?;
			(Some(entry_array), ptr_size)
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
			Some(slabs) => (
				SlabReader::new(Some(slabs.clone()), 0, Some(ptr_size))?,
				SlabWriter::new(Some(slabs.clone()), 0, Some(ptr_size))?,
				Some(slabs.clone()),
			),
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
			is_hashtable,
			debug_get_next_slot_error: false,
			debug_entry_array_len: false,
		};
		Ok(ret)
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

		*cur = if direction == Direction::Backward {
			reader.seek(slot, ptr_size);
			reader.read_fixed_bytes(&mut ptrs[0..ptr_size])?;
			slice_to_usize(&ptrs[0..ptr_size])?
		} else {
			reader.seek(slot, 0);
			reader.read_fixed_bytes(&mut ptrs[0..ptr_size])?;
			slice_to_usize(&ptrs[0..ptr_size])?
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

	#[allow(while_true)]
	fn clear_impl(&mut self) -> Result<(), Error> {
		let mut cur = self.tail;

		while true {
			cbreak!(cur == SLOT_EMPTY || cur == SLOT_DELETED);
			cbreak!(self.entry_array.is_none() && cur >= self.max_value);
			debug!("clear impl cur={}", cur)?;

			if cur < self.max_value {
				let entry = self.lookup_entry(cur);
				debug!("free chain = {}", entry)?;
				self.free_chain(entry)?;
			}

			cbreak!(!(cur < self.max_value));

			let last_cur = cur;
			let dir = Direction::Backward;
			self.get_next_slot(&mut cur, dir, &mut self.slab_reader.clone())?;
			if self.entry_array.is_some() {
				let entry_array = self.entry_array.as_mut().unwrap();
				debug!("setting entry_array[{}]={}", last_cur, SLOT_EMPTY)?;
				entry_array[last_cur] = SLOT_EMPTY
			}
		}
		debug!("set size to 0")?;
		self.size = 0;
		self.tail = SLOT_EMPTY;
		self.head = SLOT_EMPTY;

		// clear the entry array to get rid of SLOT_DELETED
		if self.entry_array.is_some() {
			let size = self.entry_array.as_ref().unwrap().size();
			let entry_array = UtilBuilder::build_array(size, &SLOT_EMPTY)?;
			self.entry_array = Some(entry_array);
		}

		Ok(())
	}

	fn remove_oldest_impl(&mut self) -> Result<(), Error> {
		debug!("self.head={}, Slot_empty={}", self.head, SLOT_EMPTY)?;
		if self.head != SLOT_EMPTY {
			self.remove_impl(self.head)?;
		}
		Ok(())
	}

	fn bring_to_front_impl<V>(&mut self, key: &K, hash: usize) -> Result<(), Error>
	where
		K: PartialEq,
		V: Serializable + Clone,
	{
		let found = self.get_impl(key, hash)?;

		if found.is_some() {
			// unwrap is ok because is_some is a condition
			let (entry, _reader) = found.unwrap();
			debug!("e={},tail={},self.head={}", entry, self.tail, self.head)?;
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

		Ok(())
	}

	fn raw_read_impl(
		&self,
		key: &K,
		hash: usize,
		offset: usize,
		data: &mut [u8; BUFFER_SIZE],
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
		data: &[u8; BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error>
	where
		K: PartialEq,
		V: Clone + Serializable,
	{
		self.insert_hash_impl::<V>(Some(key), None, Some((offset, data, len)), hash)?;

		Ok(())
	}

	pub(crate) fn get_impl(
		&self,
		key: &K,
		hash: usize,
	) -> Result<Option<(usize, SlabReader)>, Error>
	where
		K: Serializable + PartialEq + Clone,
	{
		let entry_array_len = if self.entry_array.is_some() {
			self.entry_array.as_ref().unwrap().size()
		} else {
			let fmt = "get_impl called with no entry array";
			let e = err!(ErrKind::IllegalState, fmt);
			return Err(e);
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

	fn insert_hash_impl<V>(
		&mut self,
		key: Option<&K>,
		value: Option<&V>,
		raw_chunk: Option<(usize, &[u8; BUFFER_SIZE], usize)>,
		hash: usize,
	) -> Result<(), Error>
	where
		K: Serializable + PartialEq + Clone,
		V: Serializable + Clone,
	{
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
			cbreak!(entry_value == SLOT_EMPTY || entry_value == SLOT_DELETED);

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
					cbreak!(true);
				}
			}

			entry = (entry + 1) % entry_array_len;
			i += 1;
		}

		debug!("insert_impl")?;
		let slab_id = if slab_id != self.max_value {
			Some(slab_id)
		} else {
			None
		};
		self.insert_impl(key, value, raw_chunk, Some(entry), slab_id, raw_exists)?;

		Ok(())
	}

	fn insert_impl<V>(
		&mut self,
		key: Option<&K>,
		value: Option<&V>,
		raw_value: Option<(usize, &[u8; BUFFER_SIZE], usize)>,
		entry: Option<usize>,
		slab_id_allocated: Option<usize>,
		raw_exists: bool,
	) -> Result<(), Error>
	where
		V: Serializable + Clone,
	{
		debug!("insimpl raw_value = {:?}", raw_value)?;
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
			let wval = key.as_ref().unwrap().write(&mut self.slab_writer);
			if wval.is_err() {
				let e = wval.unwrap_err();
				warn!("writing key generated error: {}", e)?;
				self.free_chain(slab_id)?;
				let fmt = format!("writing key generated error: {}", e);
				let e = err!(ErrKind::CapacityExceeded, fmt);
				return Err(e);
			}
		}
		debug!("value write")?;
		if value.is_some() {
			let wval = value.as_ref().unwrap().write(&mut self.slab_writer);
			if wval.is_err() {
				let e = wval.unwrap_err();
				warn!("writing value generated error: {}", e)?;
				self.free_chain(slab_id)?;
				let fmt = format!("writing value generated error: {}", e);
				let e = err!(ErrKind::CapacityExceeded, fmt);
				return Err(e);
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
			if self.entry_array.is_some() {
				let entry_array = self.entry_array.as_mut().unwrap();
				// for hash based structures we use the entry index
				if self.tail < max_value {
					if entry_array[self.tail] < max_value {
						let entry_value = self.lookup_entry(self.tail);
						self.slab_writer.seek(entry_value, 0);
						usize_to_slice(entry, &mut ptrs[0..ptr_size])?;
						self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;
					}
				}
			} else {
				// for list based structures we use the slab_id directly
				if self.tail < max_value {
					self.slab_writer.seek(self.tail, 0);
					usize_to_slice(entry, &mut ptrs[0..ptr_size])?;
					self.slab_writer.write_fixed_bytes(&ptrs[0..ptr_size])?;
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
				let mut slabs = slabs.wlock()?;
				let guard = slabs.guard()?;
				let mut slab = (**guard).allocate()?;
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
				let mut slabs = slabs.wlock()?;
				let guard = slabs.guard()?;
				(**guard).free(slab_id)
			}
			None => GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				slabs.free(slab_id)
			}),
		}
	}

	fn free_chain(&mut self, slab_id: usize) -> Result<(), Error> {
		debug!("free chain {}", slab_id)?;
		let bytes_per_slab = self.bytes_per_slab;
		let slab_size = self.slab_size;
		let mut next_bytes = slab_id;
		loop {
			let id = next_bytes;
			let n = match &self.slabs {
				Some(slabs) => {
					let slabs = slabs.rlock()?;
					let guard = slabs.guard()?;
					let slab = (**guard).get(next_bytes)?;
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

			cbreak!(next_bytes >= self.max_value);
		}
		Ok(())
	}
	fn lookup_entry(&self, entry: usize) -> usize {
		match self.entry_array.as_ref() {
			Some(entry_array) => entry_array[entry],
			None => entry,
		}
	}

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

	fn slabs_impl(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error> {
		Ok(self.slabs.clone())
	}

	#[cfg(test)]
	pub(crate) fn set_debug_get_next_slot_error(&mut self, v: bool) {
		self.debug_get_next_slot_error = v;
	}

	#[cfg(test)]
	pub(crate) fn set_debug_entry_array_len(&mut self, v: bool) {
		self.debug_entry_array_len = v;
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
	fn raw_read(&self, key: &K, chunk: usize, data: &mut [u8; BUFFER_SIZE]) -> Result<bool, Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.raw_read_impl(key, hash, chunk, data)
	}
	fn raw_write(
		&mut self,
		key: &K,
		chunk: usize,
		data: &[u8; BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error> {
		let mut hasher = DefaultHasher::new();
		key.hash(&mut hasher);
		let hash = hasher.finish() as usize;
		self.raw_write_impl::<V>(key, hash, chunk, data, len)
	}
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error> {
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
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error> {
		self.slabs_impl()
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
