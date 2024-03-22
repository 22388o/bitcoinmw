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

use crate::types::{
	HashImpl, HashImplSync, LockImpl, SearchTrieImpl, SlabAllocatorImpl, ThreadPoolImpl,
};
use crate::{
	Array, ArrayList, Hashset, Hashtable, Lock, LockBox, Match, Pattern, Queue, SearchTrie,
	SlabAllocator, SortableList, Stack, ThreadPool, UtilBuilder,
};
use bmw_conf::ConfigOption;
use bmw_err::*;
use bmw_ser::Serializable;
use std::any::Any;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::hash::Hash;

impl UtilBuilder {
	/// Build a [`crate::ThreadPool`] based on the specified [`crate::ThreadPoolConfig`].
	/// The [`crate::ThreadPool::start`] function must be called before executing tasks.
	pub fn build_thread_pool<T, OnPanic>(
		config: Vec<ConfigOption>,
	) -> Result<impl ThreadPool<T, OnPanic>, Error>
	where
		OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
			+ Send
			+ 'static
			+ Clone
			+ Sync
			+ Unpin,
		T: 'static + Send + Sync,
	{
		Ok(ThreadPoolImpl::new(config)?)
	}

	pub fn build_array<T>(size: usize, default: &T) -> Result<Array<T>, Error>
	where
		T: Clone,
	{
		Array::new(size, default)
	}

	pub fn build_array_list<T>(size: usize, default: &T) -> Result<impl SortableList<T>, Error>
	where
		T: Clone + Debug + PartialEq + Serializable,
	{
		ArrayList::new(size, default)
	}

	pub fn build_array_list_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn SortableList<T>>, Error>
	where
		T: Clone + Debug + PartialEq + Serializable + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	pub fn build_array_list_sync<T>(
		size: usize,
		default: &T,
	) -> Result<impl SortableList<T> + Send + Sync, Error>
	where
		T: Clone + Debug + PartialEq + Serializable + Send + Sync,
	{
		ArrayList::new(size, default)
	}

	pub fn build_array_list_sync_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn SortableList<T> + Send + Sync>, Error>
	where
		T: Clone + Debug + PartialEq + Serializable + Send + Sync + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// Build an [`crate::Queue`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the queue. On success an anonymous impl of [`crate::Queue`]
	/// is returned.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_queue<T>(size: usize, default: &T) -> Result<impl Queue<T>, Error>
	where
		T: Clone,
	{
		ArrayList::new(size, default)
	}

	/// Build an [`crate::Queue`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the queue. On success a `Box<dyn Queue<T>>`
	/// is returned. This function may be used if you wish to store the list in a
	/// struct or enum.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_queue_box<T>(size: usize, default: &T) -> Result<Box<dyn Queue<T>>, Error>
	where
		T: Clone + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// Build an [`crate::Queue`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the queue. On success an anonymous impl of [`crate::Queue`]
	/// is returned. This version requires that T be Send and Sync and returns a "Send/Sync"
	/// queue.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_queue_sync<T>(
		size: usize,
		default: &T,
	) -> Result<impl Queue<T> + Send + Sync, Error>
	where
		T: Clone + Send + Sync,
	{
		ArrayList::new(size, default)
	}

	/// Build an [`crate::Queue`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the queue. On success a `Box<dyn Queue<T>>`
	/// is returned. This function may be used if you wish to store the list in a
	/// struct or enum. This version requires that T be Send and Sync and returns a "Send/Sync"
	/// queue.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_queue_sync_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn Queue<T> + Send + Sync>, Error>
	where
		T: Clone + Send + Sync + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// Build an [`crate::Stack`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the stack. On success an anonymous impl of [`crate::Stack`]
	/// is returned.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_stack<T>(size: usize, default: &T) -> Result<impl Stack<T>, Error>
	where
		T: Clone,
	{
		ArrayList::new(size, default)
	}

	/// Build a [`crate::Stack`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the stack. On success a `Box<dyn Stack<T>>`
	/// is returned. This function may be used if you wish to store the list in a
	/// struct or enum.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_stack_box<T>(size: usize, default: &T) -> Result<Box<dyn Stack<T>>, Error>
	where
		T: Clone + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// sync version of [`crate::UtilBuilder::build_stack`].
	pub fn build_stack_sync<T>(
		size: usize,
		default: &T,
	) -> Result<impl Stack<T> + Send + Sync, Error>
	where
		T: Send + Sync + Clone,
	{
		ArrayList::new(size, default)
	}

	/// sync box version of [`crate::UtilBuilder::build_stack`].
	pub fn build_stack_sync_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn Stack<T> + Send + Sync>, Error>
	where
		T: Send + Sync + Clone + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// Build a slab allocator on the heap in an [`std::cell::UnsafeCell`].
	/// This function is used by the global thread local slab allocator to allocate
	/// thread local slab allocators. Note that it calls unsafe functions. This
	/// function should generally be called through the [`crate::global_slab_allocator`]
	/// macro.
	pub fn build_slabs_unsafe() -> UnsafeCell<Box<dyn SlabAllocator>> {
		UnsafeCell::new(Box::new(SlabAllocatorImpl::new()))
	}

	/// Build a slab allocator in a Box.
	pub fn build_slabs() -> Box<dyn SlabAllocator> {
		Box::new(SlabAllocatorImpl::new())
	}

	/// sync version of [`crate::UtilBuilder::build_slabs`].
	pub fn build_sync_slabs() -> Box<dyn SlabAllocator + Send + Sync> {
		Box::new(SlabAllocatorImpl::new())
	}

	/// Build a [`crate::Lock`].
	pub fn build_lock<T>(t: T) -> Result<impl Lock<T>, Error>
	where
		T: Send + Sync,
	{
		Ok(LockImpl::new(t))
	}

	/// Build a [`crate::LockBox`].
	pub fn build_lock_box<T>(t: T) -> Result<Box<dyn LockBox<T>>, Error>
	where
		T: Send + Sync + 'static,
	{
		Ok(Box::new(LockImpl::new(t)))
	}

	/// Build a match struct.
	pub fn build_match(configs: Vec<ConfigOption>) -> Result<Match, Error> {
		Match::new(configs)
	}

	/// Build a pattern based on the specified `regex`. If `is_case_sensitive` is true, only
	/// case sensitive matches will be returned. Otherwise all case matches will be returned.
	/// If `termination_pattern` is true, if the search trie finds this pattern, it will stop
	/// searching for additional patterns.
	/// If `is_multi_line` is true, wildcard matches will be allowed to contain newlines.
	/// Otherwise a newline will terminate any potential wild card match. The `id` is a value
	/// that is returned in the matches array so indicate that this pattern was matched.
	pub fn build_pattern(configs: Vec<ConfigOption>) -> Result<Pattern, Error> {
		Pattern::new(configs)
	}

	/// Builds a search trie based on the specified list of patterns. The `termination_length`
	/// is the length at which the matching terminates. The `max_wildcard_length` is the
	/// maximum length of any wild card matches.
	pub fn build_search_trie(
		patterns: Vec<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<impl SearchTrie + Send + Sync, Error> {
		SearchTrieImpl::new(patterns, termination_length, max_wildcard_length)
	}

	/// Same as [`crate::UtilBuilder::build_search_trie`] except that the tree is returned
	/// as a `Box<dyn SearchTrie>>`.
	pub fn build_search_trie_box(
		patterns: Vec<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<Box<dyn SearchTrie + Send + Sync>, Error> {
		Ok(Box::new(SearchTrieImpl::new(
			patterns,
			termination_length,
			max_wildcard_length,
		)?))
	}

	pub fn build_hashtable_sync<K, V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<impl Hashtable<K, V> + Send + Sync, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
		V: Serializable + Clone,
	{
		configs.push(ConfigOption::IsHashtable(true));
		HashImplSync::new(configs)
	}

	/// Build a synchronous [`crate::Hashtable`] based on the specified `config` and
	/// `slab_config`. The returned Hashtable implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// hashtable/hashset builder functions. The returned value is a Box<dyn Hashtable<K, V>>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashtable_sync_box<K, V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn Hashtable<K, V> + Send + Sync>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
		V: Serializable + Clone,
	{
		configs.push(ConfigOption::IsHashtable(true));
		Ok(Box::new(HashImplSync::new(configs)?))
	}

	pub fn build_hashtable<K, V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<impl Hashtable<K, V>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
		V: Serializable + Clone,
	{
		configs.push(ConfigOption::IsHashtable(true));
		HashImpl::new(configs)
	}

	/// Build a [`crate::Hashtable`] based on the specified `config` and
	/// `slabs`. The returned Hashtable is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is a Box<dyn Hashtable<K, V>>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashtable_box<K, V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn Hashtable<K, V>>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
		V: Serializable + Clone,
	{
		configs.push(ConfigOption::IsHashtable(true));
		let ret = HashImpl::new(configs)?;
		let bx = Box::new(ret);
		Ok(bx)
	}

	pub fn build_hashset_sync<K>(
		mut configs: Vec<ConfigOption>,
	) -> Result<impl Hashset<K> + Send + Sync, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
	{
		configs.push(ConfigOption::IsHashset(true));
		HashImplSync::new(configs)
	}

	/// Build a synchronous [`crate::Hashset`] based on the specified `config` and
	/// `slab_config`. The returned Hashset implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// hashtable/hashset builder functions. The returned value is a `Box<dyn Hashset<K>>`.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashset_sync_box<K>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn Hashset<K> + Send + Sync>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
	{
		configs.push(ConfigOption::IsHashset(true));
		let ret = HashImplSync::new(configs)?;
		let ret = Box::new(ret);
		Ok(ret)
	}

	pub fn build_hashset<K>(mut configs: Vec<ConfigOption>) -> Result<impl Hashset<K>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
	{
		configs.push(ConfigOption::IsHashset(true));
		HashImpl::new(configs)
	}

	/// Build a [`crate::Hashset`] based on the specified `config` and
	/// `slabs`. The returned Hashset is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is a `Box<dyn Hashset<K>>`.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashset_box<K>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn Hashset<K>>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
	{
		configs.push(ConfigOption::IsHashset(true));
		let ret = HashImpl::new(configs)?;
		let bx = Box::new(ret);
		Ok(bx)
	}

	pub fn build_list_sync<V>(mut configs: Vec<ConfigOption>) -> Result<impl SortableList<V>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone,
	{
		configs.push(ConfigOption::IsList(true));
		HashImplSync::new(configs)
	}

	/// Build a synchronous [`crate::List`] based on the specified `config` and
	/// `slab_config`. The returned List implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// list builder functions. The returned value is a `Box<dyn SortableList<V>>`.
	/// This version of the list is a linked list.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_list_sync_box<V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn SortableList<V> + Send + Sync>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone + 'static,
	{
		configs.push(ConfigOption::IsList(true));
		let ret = HashImplSync::new(configs)?;
		let ret = Box::new(ret);
		Ok(ret)
	}

	pub fn build_list<V>(mut configs: Vec<ConfigOption>) -> Result<impl SortableList<V>, Error>
	where
		V: Serializable + Debug + Clone,
	{
		configs.push(ConfigOption::IsList(true));
		HashImpl::new(configs)
	}

	/// Build a [`crate::List`] based on the specified `config` and
	/// `slabs`. The returned List is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is a `Box<dyn SortableList<V>>`.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_list_box<V>(
		mut configs: Vec<ConfigOption>,
	) -> Result<Box<dyn SortableList<V>>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone + 'static,
	{
		configs.push(ConfigOption::IsList(true));
		let ret = HashImpl::new(configs)?;
		let bx = Box::new(ret);
		Ok(bx)
	}
}
