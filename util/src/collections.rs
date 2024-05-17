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
use crate::slabio::*;
use crate::slabs::{SlabAllocator, ThreadLocalSlabAllocator, THREAD_LOCAL_SLAB_ALLOCATOR};
use bmw_core::*;
use bmw_log::*;
use std::hash::Hash;
use std::marker::PhantomData;

debug!();

#[ErrorKind]
pub enum CollectionErrors {
	WrongSlabAllocatorId,
}

pub struct IteratorHashtable<'a, K, V>
where
	K: Serializable,
	V: Serializable,
{
	_collection: &'a dyn Hashtable<K, V>,
	_cur: u64,
}

impl<'a, K, V> IteratorHashtable<'a, K, V>
where
	K: Serializable,
	V: Serializable,
{
	fn new(_collection: &'a dyn Hashtable<K, V>, _cur: u64) -> Self {
		Self { _collection, _cur }
	}
}

impl<K, V> std::iter::Iterator for IteratorHashtable<'_, K, V>
where
	K: Serializable,
	V: Serializable,
{
	type Item = (K, V);
	fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
		None
	}
}

pub struct Iterator<'a, K>
where
	K: Serializable + 'static,
{
	_phantom_data: &'a PhantomData<K>,
	slab_reader: Box<dyn SlabReader + Send + Sync>,
	cur: u64,
}

impl<'a, K> Iterator<'a, K>
where
	K: Serializable + Clone,
{
	fn new(collection: &'a Collection<K>, cur: u64) -> Self {
		Self {
			slab_reader: collection.slab_reader(),
			cur,
			_phantom_data: &PhantomData,
		}
	}
}

impl<K> std::iter::Iterator for Iterator<'_, K>
where
	K: Serializable,
{
	type Item = K;
	fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
		if self.cur == u64::MAX {
			None
		} else {
			match self.next_impl() {
				Ok(ret) => ret,
				Err(e) => {
					let _ = error!("iterator next generated error: {}", e);
					None
				}
			}
		}
	}
}
impl<K> Iterator<'_, K>
where
	K: Serializable,
{
	fn next_impl(&mut self) -> Result<Option<<Self as std::iter::Iterator>::Item>, Error> {
		self.slab_reader.seek(self.cur, 8)?;
		self.cur = u64::read(&mut self.slab_reader)?;
		let ret = K::read(&mut self.slab_reader)?;
		Ok(Some(ret))
	}
}

#[derive(PartialEq, Clone, Debug, Serializable)]
struct IdOffsetPair {
	id: u64,
	offset: u16,
}

impl IdOffsetPair {
	fn new(id: u64, offset: u16) -> Self {
		Self { id, offset }
	}

	#[allow(dead_code)]
	const MAX: Self = IdOffsetPair {
		id: u64::MAX,
		offset: u16::MAX,
	};
}

impl From<u64> for IdOffsetPair {
	fn from(id: u64) -> Self {
		Self { id, offset: 0 }
	}
}

impl From<usize> for IdOffsetPair {
	fn from(id: usize) -> Self {
		Self {
			id: try_into!(id).unwrap(),
			offset: 0,
		}
	}
}

impl From<i32> for IdOffsetPair {
	fn from(id: i32) -> Self {
		Self {
			id: try_into!(id).unwrap(),
			offset: 0,
		}
	}
}

#[class {
		var phantom_data: PhantomData<K>;
		generic hashtable: <K, V> where K: Serializable + Clone + Hash + 'static, V: Serializable;
                generic hashset: <K> where K: Serializable + Clone + Hash + 'static;
                clone list;
		pub list as list_impl;

                var_in slab_allocator_in: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>;
                var entry_array: Vec<usize>;
                var head: u64;
                var tail: u64;
                var slab_reader: Box<dyn SlabReader + Send + Sync>;
                var slab_writer: Box<dyn SlabWriter + Send + Sync>;
                var slab_allocator_id: u128;
                const entry_array_len: usize = 50 * 1024;
                const slab_size: usize = 512;

		[hashtable]
		fn insert(&mut self, key: K, value: V) -> Result<(), Error> as hashtable_insert;

		[hashset]
		fn insert(&mut self, key: K) -> Result<(), Error> as hashset_insert;

		[hashtable]
		fn get(&self, key: K) -> Result<Option<V>, Error>;

		[hashset]
		fn contains(&self, key: K) -> Result<bool, Error>;

		[hashtable]
		fn delete(&mut self, key: K) -> Result<Option<V>, Error> as hashtable_delete;

		[hashset]
		fn delete(&mut self, key: K) -> Result<bool, Error> as hashset_delete;

		[list]
		fn push(&mut self, value: K) -> Result<(), Error>;

		[hashset, list]
		fn iter(&self) -> Iterator<K>;

                [hashtable]
                fn iter(&self) -> IteratorHashtable<K, V> as iter_hashtable;

		[hashtable, hashset, list]
		fn clear(&mut self) -> Result<(), Error>;
}]
impl<K> Collection<K> where K: Serializable + Clone + 'static {}

impl<K> Drop for Collection<K>
where
	K: Serializable + Clone + 'static,
{
	fn drop(&mut self) {
		match self.clear() {
			Ok(_) => {}
			Err(e) => {
				let _ = error!("drop generated error: {}", e);
			}
		}
	}
}

impl<K> CollectionVarBuilder for CollectionVar<K>
where
	K: Serializable + Clone + 'static,
{
	fn builder(constants: &CollectionConst) -> Result<Self, Error> {
		let name = constants.get_name();
		let entry_array = if name == "hashtable" || name == "hashset" {
			let mut ret = vec![];
			ret.resize(constants.entry_array_len, usize::MAX);
			ret
		} else {
			vec![]
		};

		let head = u64::MAX;
		let tail = u64::MAX;

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

		let slab_size = constants.slab_size;
		let mut slab_reader = slab_reader_sync_box!(SlabIOClassConstOptions::SlabSize(slab_size))?;
		let mut slab_writer = slab_writer_sync_box!(SlabIOClassConstOptions::SlabSize(slab_size))?;

		let slab_allocator_id = match slab_allocator_in {
			Some(ref mut sa) => {
				slab_writer.set_slab_allocator(sa.clone())?;
				slab_reader.set_slab_allocator(sa.clone())?;
				sa.id()
			}
			None => THREAD_LOCAL_SLAB_ALLOCATOR.with(|f| -> Result<u128, Error> {
				let sa = f.borrow();
				Ok(sa.id())
			})?,
		};
		debug!(
			"name={},is_send={},is_sync={},is_box={}",
			constants.get_name(),
			constants.is_send,
			constants.is_sync,
			constants.is_box,
		)?;
		Ok(Self {
			phantom_data: PhantomData,
			entry_array,
			head,
			tail,
			slab_allocator_in,
			slab_reader,
			slab_writer,
			slab_allocator_id,
		})
	}
}

impl<K> Collection<K>
where
	K: Serializable + Clone + Hash + 'static,
{
	fn hashtable_insert<V>(&mut self, key: K, value: V) -> Result<(), Error>
	where
		V: Serializable,
	{
		let pair = self.insert_key(key)?;
		self.insert_value(value, pair)?;
		Ok(())
	}

	fn hashset_insert(&mut self, key: K) -> Result<(), Error> {
		self.insert_key(key)?;
		Ok(())
	}
	fn insert_key(&mut self, key: K) -> Result<IdOffsetPair, Error> {
		self.insert_time_list(key)
	}

	fn insert_value<V>(&mut self, value: V, pair: IdOffsetPair) -> Result<(), Error>
	where
		V: Serializable,
	{
		let slab_writer = self.vars_mut().get_mut_slab_writer();
		slab_writer.seek(pair.id, pair.offset.into())?;

		// write the value which will be after the key
		value.write(slab_writer)?;

		Ok(())
	}

	fn hashtable_delete<V>(&mut self, _key: K) -> Result<Option<V>, Error>
	where
		V: Serializable,
	{
		todo!()
	}

	fn hashset_delete(&mut self, _key: K) -> Result<bool, Error> {
		Ok(false)
	}

	fn contains(&self, _key: K) -> Result<bool, Error> {
		todo!()
	}

	fn get<V>(&self, _key: K) -> Result<Option<V>, Error>
	where
		V: Serializable,
	{
		todo!()
	}

	fn iter_hashtable<V>(&self) -> IteratorHashtable<K, V>
	where
		V: Serializable,
	{
		let head = *self.vars().get_head();
		IteratorHashtable::new(self, head)
	}
}

impl<K> Collection<K>
where
	K: Serializable + Clone + 'static,
{
	fn slab_reader(&self) -> Box<dyn SlabReader + Send + Sync> {
		self.vars().get_slab_reader().clone()
	}
	fn push<V>(&mut self, value: V) -> Result<(), Error>
	where
		V: Serializable,
	{
		self.insert_time_list(value)?;
		Ok(())
	}

	fn iter(&self) -> Iterator<K> {
		let head = *self.vars().get_head();
		Iterator::new(self, head)
	}

	fn clear(&mut self) -> Result<(), Error> {
		let mut cur = *self.vars().get_head();
		let slab_reader = self.vars_mut().get_mut_slab_reader();
		loop {
			cbreak!(cur == u64::MAX);

			debug!("clear cur = {}", cur)?;
			slab_reader.free_tail(cur)?;
			debug!("clear cur complete")?;
			slab_reader.seek(cur, 8)?;
			cur = u64::read(slab_reader)?;
		}
		*self.vars_mut().get_mut_head() = u64::MAX;
		*self.vars_mut().get_mut_tail() = u64::MAX;
		Ok(())
	}

	fn allocate(&mut self) -> Result<u64, Error> {
		let mut invalid_ptr = [0u8; 8];
		let ptr_size = 8;
		let mut ptr = [0u8; 8];
		set_max(&mut ptr[0..ptr_size]);
		let max_value = slice_to_usize(&ptr[0..ptr_size])?;
		usize_to_slice(max_value - 1, &mut invalid_ptr[0..ptr_size])?;

		let slab_size = *self.constants().get_slab_size();
		let slab_data_size = slab_size - 8;
		match self.vars_mut().get_mut_slab_allocator_in() {
			Some(sa) => {
				let mut sa = sa.wlock()?;
				let ret = sa.allocate(slab_size)?;
				sa.write(ret, &invalid_ptr, slab_data_size)?;
				Ok(ret)
			}
			None => ThreadLocalSlabAllocator::slab_allocator(
				*self.vars().get_slab_allocator_id(),
				|f| -> Result<u64, Error> {
					let mut sa = f.borrow_mut();
					let id = sa.allocate(slab_size)?;
					sa.write(id, &invalid_ptr, slab_data_size)?;
					Ok(id)
				},
			)?,
		}
	}

	fn insert_time_list<V>(&mut self, value: V) -> Result<IdOffsetPair, Error>
	where
		V: Serializable,
	{
		let append = self.allocate()?;
		if self.vars().get_head() == &u64::MAX {
			*self.vars_mut().get_mut_head() = append;
		}

		let tail = (*self.vars().get_tail()).clone();
		let slab_writer = self.vars_mut().get_mut_slab_writer();
		slab_writer.seek(append, 0)?;

		// write the entry
		tail.write(slab_writer)?; // prev is current tail
		u64::MAX.write(slab_writer)?; // next is MAX (null)
		value.write(slab_writer)?;
		let ret = Self::cur_id_offset(slab_writer);

		// update prev tail to point to us if this is not the only entry
		if tail != u64::MAX {
			debug!("update prev at tail = {}", tail)?;
			slab_writer.seek(tail, 8)?;
			append.write(slab_writer)?;
		}

		debug!("Setting tail to {:?}", append)?;
		*self.vars_mut().get_mut_tail() = append;

		Ok(ret)
	}

	fn cur_id_offset(slab_writer: &mut Box<dyn SlabWriter + Send + Sync>) -> IdOffsetPair {
		let id = slab_writer.get_id();
		let offset = slab_writer.get_offset();
		IdOffsetPair::new(id, try_into!(offset).unwrap_or(u16::MAX.into()))
	}
}

#[macro_export]
macro_rules! list {
	($( $x:expr ),*) => {{
                let mut ret = list_impl!()?;
                $(
                    ret.push($x)?;
                )*

                ret
	}};
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::slabs::slab_allocator_sync_box;
	use crate::slabs::SlabAllocatorClassConstOptions::*;
	use crate::slabs::*;
	use bmw_test::*;
	use std::sync::{Arc, RwLock};

	#[test]
	fn test_list_iter() -> Result<(), Error> {
		let test_info = test_info!()?;
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(100),
		)?;

		let mut list = list_impl!()?;
		list.push("1".to_string())?;
		list.push("2".to_string())?;
		list.push("3".to_string())?;
		list.push("last one".to_string())?;

		for v in list.iter() {
			info!("v={}", v)?;
		}

		let mut success = lock_box!(false);
		let success_clone = success.clone();
		let (tx, rx) = test_info.sync_channel();

		let mut list2 = list_sync_box!(SlabAllocatorIn(Some(lock_box!(sa))))?;
		list2.push(0)?;
		let list2_clone = list2.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			debug!("pre0")?;
			for v in list.iter() {
				info!("v={}", v)?;
			}

			list2.push(1)?;
			list2.push(2)?;

			debug!("Pre")?;
			assert!(list.push("ok".to_string()).is_err());
			debug!("post")?;
			*success.wlock()? = true;
			tx.send(())?;

			Ok(())
		});

		rx.recv()?;
		assert!(*success_clone.rlock()?);

		let compare = vec![0, 1, 2];
		let mut counter = 0;
		for v in list2_clone.iter() {
			assert_eq!(v, compare[counter]);
			debug!("v={}", v)?;
			counter += 1;
		}
		assert_eq!(counter, 3);

		Ok(())
	}

	#[test]
	fn test_collections() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(200))?),
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(100),
		)?;
		let sa = Some(lock_box!(sa));
		let mut hashtable = hashtable_box!(SlabAllocatorIn(sa.clone()))?;
		let mut hashset = hashset!(SlabAllocatorIn(sa))?;
		let mut list = list!["dd".to_string(), "ee".to_string()];
		let mut hashtable2 = hashtable!()?;

		hashtable2.insert("test".to_string(), 1usize)?;

		hashtable.insert(&0usize, &1usize)?;
		hashset.insert(&0usize)?;
		list.push("ok".to_string())?;

		let x = Arc::new(RwLock::new(hashset));
		let x_clone = x.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			let mut v = x.write()?;
			(*v).insert(&3usize)?;
			Ok(())
		});

		std::thread::sleep(std::time::Duration::from_millis(1000));

		let mut t = x_clone.write()?;
		(*t).insert(&4usize)?;

		Ok(())
	}

	#[test]
	fn test_collection_clear() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(10),
		)?;
		let lock_box = lock_box!(sa);
		let lock_box_clone = lock_box.clone();

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);

		let mut list = list_impl!(SlabAllocatorIn(Some(lock_box)))?;
		list.push(1)?;
		list.push(2)?;
		list.push(3)?;
		list.push(4)?;
		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 4);
		let mut count = 0;
		let expect = vec![1, 2, 3, 4];
		for v in list.iter() {
			assert_eq!(v, expect[count]);
			count += 1;
			debug!("v={}", v)?;
		}
		assert_eq!(count, 4);

		list.clear()?;
		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);
		let mut count = 0;
		for v in list.iter() {
			count += 1;
			debug!("v={}", v)?;
		}

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);
		assert_eq!(count, 0);

		list.clear()?;
		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);

		Ok(())
	}

	#[test]
	fn test_collection_drop() -> Result<(), Error> {
		let sa = slab_allocator_sync_box!(
			SlabConfig(slab_config!(SlabSize(512), SlabCount(300))?),
			SlabsPerResize(10),
		)?;
		let lock_box = lock_box!(sa);
		let lock_box_clone = lock_box.clone();

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);

		{
			let mut list = list_impl!(SlabAllocatorIn(Some(lock_box)))?;
			list.push(1)?;
			list.push(2)?;
			list.push(3)?;
			list.push(4)?;
			list.push(9)?;
			assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 5);
		}

		assert_eq!(lock_box_clone.rlock()?.stats()?[0].cur_slabs, 0);
		Ok(())
	}
}
