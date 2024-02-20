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

use crate::types::{
	Builder, HashImpl, HashImplSync, LockImpl, SlabAllocatorImpl, SuffixTreeImpl, ThreadPoolImpl,
};
use crate::{
	Array, ArrayList, Hashset, HashsetConfig, Hashtable, HashtableConfig, ListConfig, Lock,
	LockBox, Match, Pattern, Queue, Serializable, SlabAllocator, SlabAllocatorConfig, SortableList,
	Stack, SuffixTree, ThreadPool, ThreadPoolConfig,
};
use bmw_err::Error;
use bmw_log::*;
use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

info!();

/// The [`crate::Builder`] is used to build the data structures in the [`crate`].
impl Builder {
	/// Build a [`crate::ThreadPool`] based on the specified [`crate::ThreadPoolConfig`].
	/// The [`crate::ThreadPool::start`] function must be called before executing tasks.
	pub fn build_thread_pool<T, OnPanic>(
		config: ThreadPoolConfig,
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

	/// Build a [`crate::Array`]. `size` is the size of the array and `default` is the
	/// value that is used to initialize the array. Because `default` must be cloned,
	/// the [`std::clone::Clone`] trait must be implemented by T. The returned value is
	/// either a [`crate::Array`] or a [`bmw_err::Error`].
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_array<T>(size: usize, default: &T) -> Result<Array<T>, Error>
	where
		T: Clone,
	{
		Array::new(size, default)
	}

	/// Build an [`crate::ArrayList`] based on the specified `size` and `default` value.
	/// The default value is only used to initialize the underlying [`crate::Array`]
	/// and is not included in the list. On success an anonymous impl of
	/// [`crate::SortableList`] is returned.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::IllegalArgument`] is returned if the specified size is 0.
	pub fn build_array_list<T>(size: usize, default: &T) -> Result<impl SortableList<T>, Error>
	where
		T: Clone + Debug + PartialEq + Serializable,
	{
		ArrayList::new(size, default)
	}

	/// boxed version of [`crate::Builder::build_array_list`].
	pub fn build_array_list_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn SortableList<T>>, Error>
	where
		T: Clone + Debug + PartialEq + Serializable + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// sync version of [`crate::Builder::build_array_list`].
	pub fn build_array_list_sync<T>(
		size: usize,
		default: &T,
	) -> Result<impl SortableList<T> + Send + Sync, Error>
	where
		T: Clone + Debug + PartialEq + Serializable + Send + Sync,
	{
		ArrayList::new(size, default)
	}

	/// sync box version of [`crate::Builder::build_array_list`].
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

	/// sync version of [`crate::Builder::build_stack`].
	pub fn build_stack_sync<T>(
		size: usize,
		default: &T,
	) -> Result<impl Stack<T> + Send + Sync, Error>
	where
		T: Send + Sync + Clone,
	{
		ArrayList::new(size, default)
	}

	/// sync box version of [`crate::Builder::build_stack`].
	pub fn build_stack_sync_box<T>(
		size: usize,
		default: &T,
	) -> Result<Box<dyn Stack<T> + Send + Sync>, Error>
	where
		T: Send + Sync + Clone + 'static,
	{
		Ok(Box::new(ArrayList::new(size, default)?))
	}

	/// Build a synchronous [`crate::Hashtable`] based on the specified `config` and
	/// `slab_config`. The returned Hashtable implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// hashtable/hashset builder functions. The returned value is an anonymous impl of
	/// Hashtable<K, V>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashtable_sync<K, V>(
		config: HashtableConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<impl Hashtable<K, V> + Send + Sync, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
		V: Serializable + Clone,
	{
		HashImplSync::new(Some(config), None, None, slab_config)
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
		config: HashtableConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<Box<dyn Hashtable<K, V> + Send + Sync>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
		V: Serializable + Clone,
	{
		let ret = HashImplSync::new(Some(config), None, None, slab_config)?;
		let ret = Box::new(ret);
		Ok(ret)
	}

	/// Build a [`crate::Hashtable`] based on the specified `config` and
	/// `slabs`. The returned Hashtable is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is an anonymous impl of
	/// Hashtable<K, V>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashtable<K, V>(
		config: HashtableConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<impl Hashtable<K, V>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
		V: Serializable + Clone,
	{
		HashImpl::new(Some(config), None, None, slabs, false)
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
		config: HashtableConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<Box<dyn Hashtable<K, V>>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
		V: Serializable + Clone,
	{
		let ret = HashImpl::new(Some(config), None, None, &slabs.clone(), false)?;
		let bx = Box::new(ret);
		Ok(bx)
	}

	/// Build a synchronous [`crate::Hashset`] based on the specified `config` and
	/// `slab_config`. The returned Hashset implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// hashtable/hashset builder functions. The returned value is an anonymous impl of
	/// Hashset<K, V>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashset_sync<K>(
		config: HashsetConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<impl Hashset<K> + Send + Sync, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
	{
		HashImplSync::new(None, Some(config), None, slab_config)
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
		config: HashsetConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<Box<dyn Hashset<K> + Send + Sync>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
	{
		let ret = HashImplSync::new(None, Some(config), None, slab_config)?;
		let ret = Box::new(ret);
		Ok(ret)
	}

	/// Build a [`crate::Hashset`] based on the specified `config` and
	/// `slabs`. The returned Hashset is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is an anonymous impl of
	/// Hashset<K, V>.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_hashset<K>(
		config: HashsetConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<impl Hashset<K>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + Clone,
	{
		HashImpl::new(None, Some(config), None, slabs, false)
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
		config: HashsetConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<Box<dyn Hashset<K>>, Error>
	where
		K: Serializable + Hash + PartialEq + Debug + 'static + Clone,
	{
		let ret = HashImpl::new(None, Some(config), None, slabs, false)?;
		let bx = Box::new(ret);
		Ok(bx)
	}

	/// Build a synchronous [`crate::List`] based on the specified `config` and
	/// `slab_config`. The returned List implements Send and Sync. Since a shared
	/// slab allocator is not thread safe and the global slab allocator is thread local,
	/// a dedicated slab allocator must be used. That is why the slab allocator configuration
	/// is specified and a slab allocator may not be passed in as is the case with the regular
	/// list builder functions. The returned value is an anonymous impl of
	/// `SortableList<V>`. This version of the list is a linked list.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_list_sync<V>(
		config: ListConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<impl SortableList<V>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone,
	{
		HashImplSync::new(None, None, Some(config), slab_config)
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
		config: ListConfig,
		slab_config: SlabAllocatorConfig,
	) -> Result<Box<dyn SortableList<V> + Send + Sync>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone + 'static,
	{
		let ret = HashImplSync::new(None, None, Some(config), slab_config)?;
		let ret = Box::new(ret);
		Ok(ret)
	}

	/// Build a [`crate::List`] based on the specified `config` and
	/// `slabs`. The returned List is not thread safe and does not implement Send
	/// or Sync. The slab allocator may be shared among other data structures, but it must
	/// not be used in other threads. The returned value is an anonymous impl of the
	/// Sortable List trait.
	///
	/// # Errors
	///
	/// [`bmw_err::ErrorKind::Configuration`] is returned if the `slab_size` is greater than
	/// 65_536, the slab count is greater than 281_474_976_710_655, `max_entries` is equal to
	/// 0, `max_load_factor` is 0 or less or greater than 1.0. or the `slab_size` is to small
	/// to fit the pointer values needed.
	pub fn build_list<V>(
		config: ListConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<impl SortableList<V>, Error>
	where
		V: Serializable + Debug + Clone,
	{
		HashImpl::new(None, None, Some(config), slabs, false)
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
		config: ListConfig,
		slabs: &Option<&Rc<RefCell<dyn SlabAllocator>>>,
	) -> Result<Box<dyn SortableList<V>>, Error>
	where
		V: Serializable + Debug + PartialEq + Clone + 'static,
	{
		let ret = HashImpl::new(None, None, Some(config), slabs, false)?;
		let bx = Box::new(ret);
		Ok(bx)
	}

	/// Build a match with the specified `start`, `end` and `id` values.
	pub fn build_match(start: usize, end: usize, id: usize) -> Match {
		Match::new(start, end, id)
	}

	/// Build a default match struct.
	pub fn build_match_default() -> Match {
		Match::new(0, 0, 0)
	}

	/// Build a pattern based on the specified `regex`. If `is_case_sensitive` is true, only
	/// case sensitive matches will be returned. Otherwise all case matches will be returned.
	/// If `termination_pattern` is true, if the suffix tree finds this pattern, it will stop
	/// searching for additional patterns.
	/// If `is_multi_line` is true, wildcard matches will be allowed to contain newlines.
	/// Otherwise a newline will terminate any potential wild card match. The `id` is a value
	/// that is returned in the matches array so indicate that this pattern was matched.
	pub fn build_pattern(
		regex: &str,
		is_case_sensitive: bool,
		is_termination_pattern: bool,
		is_multi_line: bool,
		id: usize,
	) -> Result<Pattern, Error> {
		Pattern::new(
			regex,
			is_case_sensitive,
			is_termination_pattern,
			is_multi_line,
			id,
		)
	}

	/// Builds a suffix tree based on the specified list of patterns. The `termination_length`
	/// is the length at which the matching terminates. The `max_wildcard_length` is the
	/// maximum length of any wild card matches.
	pub fn build_suffix_tree(
		patterns: impl SortableList<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<impl SuffixTree + Send + Sync, Error> {
		SuffixTreeImpl::new(patterns, termination_length, max_wildcard_length)
	}

	/// Same as [`crate::Builder::build_suffix_tree`] except that the tree is returned
	/// as a `Box<dyn SuffixTree>>`.
	pub fn build_suffix_tree_box(
		patterns: impl SortableList<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<Box<dyn SuffixTree + Send + Sync>, Error> {
		Ok(Box::new(SuffixTreeImpl::new(
			patterns,
			termination_length,
			max_wildcard_length,
		)?))
	}

	/// Build a slab allocator on the heap in an [`std::cell::UnsafeCell`].
	/// This function is used by the global thread local slab allocator to allocate
	/// thread local slab allocators. Note that it calls unsafe functions. This
	/// function should generally be called through the [`crate::global_slab_allocator`]
	/// macro.
	pub fn build_slabs_unsafe() -> UnsafeCell<Box<dyn SlabAllocator>> {
		UnsafeCell::new(Box::new(SlabAllocatorImpl::new()))
	}

	/// Build a slab allocator in a Rc/RefCell. This function is used by [`crate::slab_allocator`]
	/// to create slab allocators for use with the other macros.
	pub fn build_slabs_ref() -> Rc<RefCell<dyn SlabAllocator>> {
		Rc::new(RefCell::new(SlabAllocatorImpl::new()))
	}

	/// Build a slab allocator in a Box.
	pub fn build_slabs() -> Box<dyn SlabAllocator> {
		Box::new(SlabAllocatorImpl::new())
	}

	/// sync version of [`crate::Builder::build_slabs`].
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
}

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::*;
	use bmw_err::*;
	use std::thread::sleep;
	use std::time::Duration;

	#[test]
	fn test_builder() -> Result<(), Error> {
		let mut arrlist = Builder::build_array_list_box(10, &0)?;
		arrlist.push(0)?;
		let mut i = 0;
		for x in arrlist.iter() {
			assert_eq!(x, 0);
			i += 1;
		}
		assert_eq!(i, 1);

		let mut list =
			Builder::build_list_sync_box(ListConfig::default(), SlabAllocatorConfig::default())?;
		list.push(0)?;
		assert_eq!(list.size(), 1);

		let nmatch = Builder::build_match(0, 1, 2);
		assert_eq!(nmatch.start(), 0);
		assert_eq!(nmatch.end(), 1);
		assert_eq!(nmatch.id(), 2);

		assert!(!Builder::build_sync_slabs().is_init());

		Ok(())
	}

	#[derive(Clone)]
	struct TestObj {
		array: Array<u32>,
		array_list: Box<dyn SortableList<u32> + Send + Sync>,
		queue: Box<dyn Queue<u32> + Send + Sync>,
		stack: Box<dyn Stack<u32> + Send + Sync>,
		hashtable: Box<dyn Hashtable<u32, u32> + Send + Sync>,
		hashset: Box<dyn Hashset<u32> + Send + Sync>,
		list: Box<dyn SortableList<u32> + Send + Sync>,
		suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	}

	#[test]
	fn test_builder_sync() -> Result<(), Error> {
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_, _| Ok(()))?;

		let test_obj = TestObj {
			array: Builder::build_array(10, &0)?,
			array_list: Builder::build_array_list_sync_box(10, &0)?,
			queue: Builder::build_queue_sync_box(10, &0)?,
			stack: Builder::build_stack_sync_box(10, &0)?,
			hashtable: Builder::build_hashtable_sync_box(
				HashtableConfig::default(),
				SlabAllocatorConfig::default(),
			)?,
			hashset: Builder::build_hashset_sync_box(
				HashsetConfig::default(),
				SlabAllocatorConfig::default(),
			)?,
			list: Builder::build_list_sync_box(
				ListConfig::default(),
				SlabAllocatorConfig::default(),
			)?,
			suffix_tree: Builder::build_suffix_tree_box(
				list![pattern!(Regex("abc"), Id(0))?],
				100,
				50,
			)?,
		};
		let array_list_sync = Builder::build_array_list_sync(10, &0)?;
		let mut array_list_sync = lock_box!(array_list_sync)?;
		let queue_sync = Builder::build_queue_sync(10, &0)?;
		let mut queue_sync = lock_box!(queue_sync)?;
		let stack_sync = Builder::build_stack_sync(10, &0)?;
		let mut stack_sync = lock_box!(stack_sync)?;

		let mut stack_box = Builder::build_stack_box(10, &0)?;
		stack_box.push(50)?;
		assert_eq!(stack_box.pop(), Some(&50));
		assert_eq!(stack_box.pop(), None);

		assert_eq!(test_obj.array[0], 0);
		assert_eq!(test_obj.array_list.iter().next().is_none(), true);
		assert_eq!(test_obj.queue.peek().is_none(), true);
		assert_eq!(test_obj.stack.peek().is_none(), true);
		assert_eq!(test_obj.hashtable.size(), 0);
		assert_eq!(test_obj.hashset.size(), 0);
		assert_eq!(test_obj.list.size(), 0);
		let mut test_obj = lock_box!(test_obj)?;
		let test_obj_clone = test_obj.clone();

		execute!(tp, {
			{
				let mut test_obj = test_obj.wlock()?;
				let guard = test_obj.guard();
				(**guard).array[0] = 1;
				(**guard).array_list.push(1)?;
				(**guard).queue.enqueue(1)?;
				(**guard).stack.push(1)?;
				(**guard).hashtable.insert(&0, &0)?;
				(**guard).hashset.insert(&0)?;
				(**guard).list.push(0)?;
				let mut matches = [Builder::build_match_default(); 10];
				(**guard).suffix_tree.tmatch(b"test", &mut matches)?;
			}
			{
				let mut array_list_sync = array_list_sync.wlock()?;
				let guard = array_list_sync.guard();
				(**guard).push(0)?;
			}

			{
				let mut queue_sync = queue_sync.wlock()?;
				let guard = queue_sync.guard();
				(**guard).enqueue(0)?;
			}

			{
				let mut stack_sync = stack_sync.wlock()?;
				let guard = stack_sync.guard();
				(**guard).push(0)?;
			}

			Ok(())
		})?;

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(1));
			let test_obj = test_obj_clone.rlock()?;
			let guard = test_obj.guard();
			if (**guard).array[0] != 1 && count < 2_000 {
				continue;
			}
			assert_eq!((**guard).array[0], 1);
			assert_eq!((**guard).array_list.iter().next().is_none(), false);
			assert_eq!((**guard).queue.peek().is_some(), true);
			assert_eq!((**guard).stack.peek().is_some(), true);
			assert_eq!((**guard).hashtable.size(), 1);
			assert_eq!((**guard).hashset.size(), 1);
			assert_eq!((**guard).list.size(), 1);
			break;
		}

		Ok(())
	}
}
