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
use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_derive::Serializable;
use bmw_err::*;
use bmw_ser::Serializable;
use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Arrays for use with other functions in this library. An array can be contructed with the macro
/// [`crate::array!`].
pub struct Array<T> {
	pub(crate) data: Vec<T>,
}

/// ArrayList data structure. Arraylist implements List and Sortable list. A [`crate::ArrayList`]
/// can be constructed with the macro [`crate::array_list`].
#[derive(Clone)]
pub struct ArrayList<T> {
	pub(crate) inner: Array<T>,
	pub(crate) size: usize,
	pub(crate) head: usize,
	pub(crate) tail: usize,
}

/// An iterator for the [`crate::ArrayList`]. See [`crate::array_list`] for examples.
pub struct ArrayListIterator<'a, T> {
	pub(crate) arr: &'a ArrayList<T>,
	pub(crate) dir: Direction,
	pub(crate) c: usize,
}

/// An iterator for the [`crate::Array`]. See [`crate::array`] for examples.
pub struct ArrayIterator<'a, T> {
	pub(crate) array_ref: &'a Array<T>,
	pub(crate) cur: usize,
}

/// The list trait is implemented by both [`crate::list`] and [`crate::array_list`]. See these
/// macros for detailed examples.
pub trait List<V>: DynClone + Debug {
	/// push a value onto the end of the list
	fn push(&mut self, value: V) -> Result<(), Error>;
	/// return an iterator that can iterate through the list
	fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a>
	where
		V: Serializable + Clone;
	/// return an iterator that can iterate through the list in reverse order
	fn iter_rev<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a>
	where
		V: Serializable + Clone;
	/// delete the head of the list
	fn delete_head(&mut self) -> Result<(), Error>;
	/// return the size of the list
	fn size(&self) -> usize;
	/// clear all items from the list
	fn clear(&mut self) -> Result<(), Error>;
}

/// An iterator for [`crate::list`].
pub struct ListIterator<'a, V>
where
	V: Serializable + Clone,
{
	pub(crate) linked_list_ref: &'a HashImpl<V>,
	pub(crate) cur: usize,
	pub(crate) direction: Direction,
	pub(crate) _phantom_data: PhantomData<V>,
	pub(crate) slab_reader: SlabReader,
}

/// The sortable list allows for efficient sorting of both the [`crate::array_list`] and
/// [`crate::list`].
pub trait SortableList<V>: List<V> + DynClone {
	/// sort with a stable sorting algorithm
	fn sort(&mut self) -> Result<(), Error>
	where
		V: Ord;

	/// sort with an unstable sorting algorithm.
	/// unstable sort is significantly faster and should be
	/// used when stable sorting (ording of equal values consistent)
	/// is not required.
	fn sort_unstable(&mut self) -> Result<(), Error>
	where
		V: Ord;
}

pub trait Queue<V>: DynClone {
	/// Enqueue a value
	fn enqueue(&mut self, value: V) -> Result<(), Error>;
	/// Dequeue a value. If the queue is Empty return None
	fn dequeue(&mut self) -> Option<&V>;
	/// peek at the next value in the queue. If the queue is Empty return None
	fn peek(&self) -> Option<&V>;
	/// return the number of items currently in the queue
	fn length(&self) -> usize;
}

pub trait Stack<V>: DynClone {
	/// push a `value` onto the stack
	fn push(&mut self, value: V) -> Result<(), Error>;
	/// pop a value off the top of the stack
	fn pop(&mut self) -> Option<&V>;
	/// peek at the top of the stack
	fn peek(&self) -> Option<&V>;
	/// return the number of items currently in the stack
	fn length(&self) -> usize;
}

pub trait Hashtable<K, V>: Debug + DynClone
where
	K: Serializable + Clone,
	V: Serializable,
{
	/// Returns the maximum load factor as configured for this [`crate::Hashtable`].
	fn max_load_factor(&self) -> f64;
	/// Returns the maximum entries as configured for this [`crate::Hashtable`].
	fn max_entries(&self) -> usize;
	/// Insert a key/value pair into the hashtable.
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error>;
	/// Get the value associated with the specified `key`.
	fn get(&self, key: &K) -> Result<Option<V>, Error>;
	/// Remove the specified `key` from the hashtable.
	fn remove(&mut self, key: &K) -> Result<Option<V>, Error>;
	/// Return the size of the hashtable.
	fn size(&self) -> usize;
	/// Clear all items, reinitialized the entry array, and free the slabs
	/// associated with this hashtable.
	fn clear(&mut self) -> Result<(), Error>;
	/// Returns an [`std::iter::Iterator`] to iterate through this hashtable.
	fn iter<'a>(&'a self) -> HashtableIterator<'a, K, V>;
	/// Bring the entry to the front of the list for deletion purposes in a cache.
	fn bring_to_front(&mut self, key: &K) -> Result<(), Error>;
	/// Remove the oldest entry in the hashtable.
	fn remove_oldest(&mut self) -> Result<(), Error>;
	/// Get raw data and store it in `data` with given offset.
	fn raw_read(&self, key: &K, offset: usize, data: &mut [u8; BUFFER_SIZE])
		-> Result<bool, Error>;
	/// Write raw data from `data` with given offset.
	fn raw_write(
		&mut self,
		key: &K,
		offset: usize,
		data: &[u8; BUFFER_SIZE],
		len: usize,
	) -> Result<(), Error>;
	/// Gets the slab allocator associated with this Hashtable or None if the global slab
	/// allocator is used.
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error>;
}

pub trait Hashset<K>: Debug + DynClone
where
	K: Serializable + Clone,
{
	/// Returns the maximum load factor as configured for this [`crate::Hashset`].
	fn max_load_factor(&self) -> f64;
	/// Returns the maximum entries as configured for this [`crate::Hashset`].
	fn max_entries(&self) -> usize;
	/// Insert a key into this hashset.
	fn insert(&mut self, key: &K) -> Result<(), Error>;
	/// If `key` is present this function returns true, otherwise false.
	fn contains(&self, key: &K) -> Result<bool, Error>;
	/// Remove the specified `key` from this hashset.
	fn remove(&mut self, key: &K) -> Result<bool, Error>;
	/// Returns the size of this hashset.
	fn size(&self) -> usize;
	/// Clear all items, reinitialized the entry array, and free the slabs
	/// associated with this hashset.
	fn clear(&mut self) -> Result<(), Error>;
	/// Returns an [`std::iter::Iterator`] to iterate through this hashset.
	fn iter<'a>(&'a self) -> HashsetIterator<'a, K>;
	fn slabs(
		&self,
	) -> Result<Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>, Error>;
}

/// An iterator for the [`crate::Hashtable`].
pub struct HashtableIterator<'a, K, V>
where
	K: Serializable + Clone,
{
	pub(crate) hashtable: &'a HashImpl<K>,
	pub(crate) cur: usize,
	pub(crate) _phantom_data: PhantomData<(K, V)>,
}

/// An iterator for the [`crate::Hashset`].
pub struct HashsetIterator<'a, K>
where
	K: Serializable + Clone,
{
	pub(crate) hashset: &'a HashImpl<K>,
	pub(crate) cur: usize,
	pub(crate) _phantom_data: PhantomData<K>,
	pub(crate) slab_reader: SlabReader,
}

#[derive(Debug, Clone, Serializable)]
pub struct SlabAllocatorConfig {
	/// The size, in bytes, of a slab
	pub slab_size: usize,
	/// The number of slabs that this slab allocator can allocate
	pub slab_count: usize,
}

/// Struct that is used as a mutable reference to data in a slab. See [`crate::SlabAllocator`] for
/// further details.
pub struct SlabMut<'a> {
	pub(crate) data: &'a mut [u8],
	pub(crate) id: usize,
}

/// Struct that is used as a immutable reference to data in a slab. See [`crate::SlabAllocator`] for
/// further details.
pub struct Slab<'a> {
	pub(crate) data: &'a [u8],
	pub(crate) id: usize,
}

pub trait SlabAllocator: DynClone + Debug {
	/// If the slab allocator has been initialized, return true, otherwise, false.
	fn is_init(&self) -> bool;

	fn allocate<'a>(&'a mut self) -> Result<SlabMut<'a>, Error>;

	fn free(&mut self, id: usize) -> Result<(), Error>;

	fn get<'a>(&'a self, id: usize) -> Result<Slab<'a>, Error>;

	fn get_mut<'a>(&'a mut self, id: usize) -> Result<SlabMut<'a>, Error>;

	/// Returns the number of free slabs this [`crate::SlabAllocator`] has remaining.
	fn free_count(&self) -> Result<usize, Error>;

	/// Returns the configured `slab_size` for this [`crate::SlabAllocator`].
	fn slab_size(&self) -> Result<usize, Error>;

	/// Returns the configured `slab_count` for this [`crate::SlabAllocator`].
	fn slab_count(&self) -> Result<usize, Error>;

	/// Initializes the [`crate::SlabAllocator`] with the given `config`. See
	/// [`crate::SlabAllocatorConfig`] for further details.
	fn init(&mut self, config: SlabAllocatorConfig) -> Result<(), Error>;
}

pub trait Lock<T>: Send + Sync + Debug
where
	T: Send + Sync,
{
	/// obtain a write lock and corresponding [`std::sync::RwLockWriteGuard`] for this
	/// [`crate::Lock`].
	fn wlock(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error>;
	/// obtain a read lock and corresponding [`std::sync::RwLockReadGuard`] for this
	/// [`crate::Lock`].
	fn rlock(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error>;
	/// Clone this [`crate::Lock`].
	fn clone(&self) -> Self;
}

pub trait LockBox<T>: Send + Sync + Debug
where
	T: Send + Sync,
{
	/// obtain a write lock and corresponding [`std::sync::RwLockWriteGuard`] for this
	/// [`crate::LockBox`].
	fn wlock(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error>;
	/// obtain a read lock and corresponding [`std::sync::RwLockReadGuard`] for this
	/// [`crate::LockBox`].
	fn rlock(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error>;
	/// Same as [`crate::LockBox::wlock`] except that any poison errors are ignored
	/// by calling the underlying into_inner() fn.
	fn wlock_ignore_poison(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error>;
	/// Same as [`crate::LockBox::rlock`] except that any poison errors are ignored
	/// by calling the underlying into_inner() fn.
	fn rlock_ignore_poison(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error>;
	/// consume the inner Arc and return a usize value. This function is dangerous
	/// because it potentially leaks memory. The usize must be rebuilt into a lockbox
	/// that can then be dropped via the [`crate::lock_box_from_usize`] function.
	fn danger_to_usize(&self) -> usize;
	/// return the inner data holder.
	fn inner(&self) -> Arc<RwLock<T>>;
	/// return the id for this lockbox.
	fn id(&self) -> u128;
}

/// Wrapper around the [`std::sync::RwLockReadGuard`].
pub struct RwLockReadGuardWrapper<'a, T> {
	pub(crate) guard: RwLockReadGuard<'a, T>,
	pub(crate) id: u128,
	pub(crate) debug_err: bool,
}

/// Wrapper around the [`std::sync::RwLockWriteGuard`].
pub struct RwLockWriteGuardWrapper<'a, T> {
	pub(crate) guard: RwLockWriteGuard<'a, T>,
	pub(crate) id: u128,
	pub(crate) debug_err: bool,
}

/// Utility to write to slabs using the [`bmw_ser::Writer`] trait.
#[derive(Clone)]
pub struct SlabWriter {
	pub(crate) slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
	pub(crate) slab_id: usize,
	pub(crate) offset: usize,
	pub(crate) slab_size: usize,
	pub(crate) bytes_per_slab: usize,
}

/// Utility to read from slabs using the [`bmw_ser::Reader`] trait.
#[derive(Clone)]
pub struct SlabReader {
	pub(crate) slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
	pub(crate) slab_id: usize,
	pub(crate) offset: usize,
	pub(crate) slab_size: usize,
	pub(crate) bytes_per_slab: usize,
	pub(crate) max_value: usize,
}

#[derive(Debug, PartialEq)]
pub enum PoolResult<T, E> {
	Ok(T),
	Err(E),
	Panic,
}

pub trait ThreadPool<T, OnPanic>
where
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	/// Execute a task in the thread pool. This task will run to completion
	/// on the first available thread in the pool. The return value [`crate::ThreadPoolHandle`]
	/// which can be used to get the id of the task sent to the thread pool or to block on.
	fn execute<F>(&self, f: F, id: u128) -> Result<ThreadPoolHandle<T>, Error>
	where
		F: Future<Output = Result<T, Error>> + Send + 'static;

	/// Start the pool. If macros are used, this call is unnecessary.
	fn start(&mut self) -> Result<(), Error>;

	/// Stop the thread pool. This function will ensure no new
	/// tasks are processed in the ThreadPool and that the threads will be stopped once they
	/// become idle again. It however, does not ensure that any tasks currently running in the thread pool are stopped
	/// immediately. That is the responsibility of the user.
	fn stop(&mut self) -> Result<(), Error>;

	/// Returns the current size of the thread pool which will be between
	/// the configured maximum and minimum size.
	fn size(&self) -> Result<usize, Error>;

	/// Get the [`crate::ThreadPoolStopper`] for this thread pool.
	fn stopper(&self) -> Result<ThreadPoolStopper, Error>;

	/// Get the [`crate::ThreadPoolExecutor`] for this thread pool.
	fn executor(&self) -> Result<ThreadPoolExecutor<T>, Error>
	where
		T: Send + Sync;

	/// Set an on panic handler for this thread pool
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;

	#[cfg(test)]
	fn set_on_panic_none(&mut self) -> Result<(), Error>;
}

/// This handle is returned by [`crate::ThreadPool::execute`]. It can be used to retrieve the task
/// id or to block on the task.
#[derive(Debug)]
pub struct ThreadPoolHandle<T> {
	pub(crate) id: u128,
	pub(crate) recv_handle: Receiver<PoolResult<T, Error>>,
}

/// Struct that can be used to execute tasks in the thread pool. Mainly needed
/// for passing the execution functionality to structs/threads.
#[derive(Debug, Clone)]
pub struct ThreadPoolExecutor<T>
where
	T: 'static + Send + Sync,
{
	pub(crate) tx: Option<SyncSender<FutureWrapper<T>>>,
}

/// Struct that can be used to stop the thread pool. Note the limitations
/// in [`crate::ThreadPoolStopper::stop`].
#[derive(Debug, Clone)]
pub struct ThreadPoolStopper {
	pub(crate) state: Box<dyn LockBox<ThreadPoolState>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pattern {
	pub(crate) regex: String,
	pub(crate) is_case_sensitive: bool,
	pub(crate) is_termination_pattern: bool,
	pub(crate) is_multi_line: bool,
	pub(crate) id: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Match {
	pub(crate) start: usize,
	pub(crate) end: usize,
	pub(crate) id: usize,
}

pub trait SearchTrie: DynClone {
	/// return matches associated with the supplied `text` for this
	/// [`crate::SearchTrie`]. Matches are returned in the `matches`
	/// array supplied by the caller. The result is the number of
	/// matches found or a [`bmw_err::Error`] if an error occurs.
	fn tmatch(&mut self, text: &[u8], matches: &mut [Match]) -> Result<usize, Error>;
}

clone_trait_object!(SlabAllocator);
clone_trait_object!(<V>Queue<V>);
clone_trait_object!(<V>Stack<V>);
clone_trait_object!(<V>List<V>);
clone_trait_object!(<V>SortableList<V>);
clone_trait_object!(SearchTrie);
clone_trait_object!(<K,V>Hashtable<K,V>);
clone_trait_object!(<K>Hashset<K>);

pub struct UtilBuilder {}

// pub(crate) structures

#[derive(Clone)]
pub(crate) struct LockImpl<T> {
	pub(crate) t: Arc<RwLock<T>>,
	pub(crate) id: u128,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Direction {
	Forward,
	Backward,
}

#[derive(Clone, Debug)]
pub(crate) struct SlabAllocatorImpl {
	pub(crate) config: Option<SlabAllocatorConfig>,
	pub(crate) data: Array<u8>,
	pub(crate) first_free: usize,
	pub(crate) free_count: usize,
	pub(crate) ptr_size: usize,
	pub(crate) max_value: usize,
}

pub(crate) struct FutureWrapper<T> {
	pub(crate) f: Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'static>>,
	pub(crate) tx: SyncSender<PoolResult<T, Error>>,
	pub(crate) id: u128,
}

pub(crate) struct ThreadPoolImpl<T, OnPanic>
where
	T: 'static + Send + Sync,
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	pub(crate) config: ThreadPoolConfig,
	pub(crate) rx: Option<Arc<Mutex<Receiver<FutureWrapper<T>>>>>,
	pub(crate) tx: Option<SyncSender<FutureWrapper<T>>>,
	pub(crate) state: Box<dyn LockBox<ThreadPoolState>>,
	pub(crate) on_panic: Option<Pin<Box<OnPanic>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
	pub(crate) next: [u32; 257],
	pub(crate) pattern_id: usize,
	pub(crate) is_multi: bool,
	pub(crate) is_term: bool,
	pub(crate) is_start_only: bool,
	pub(crate) is_multi_line: bool,
}

#[derive(Clone)]
pub(crate) struct Dictionary {
	pub(crate) nodes: Vec<Node>,
	pub(crate) next: u32,
}

#[derive(Clone)]
pub(crate) struct SearchTrieImpl {
	pub(crate) dictionary_case_insensitive: Dictionary,
	pub(crate) dictionary_case_sensitive: Dictionary,
	pub(crate) termination_length: usize,
	pub(crate) max_wildcard_length: usize,
	pub(crate) branch_stack: Box<dyn Stack<(usize, usize)> + Send + Sync>,
}

#[derive(Clone)]
pub(crate) struct HashImplSync<K>
where
	K: Serializable + Clone,
{
	pub(crate) static_impl: HashImpl<K>,
}

#[derive(Clone)]
pub(crate) struct HashImpl<K>
where
	K: Serializable + Clone,
{
	pub(crate) slabs: Option<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>>,
	pub(crate) slab_reader: SlabReader,
	pub(crate) slab_writer: SlabWriter,
	pub(crate) max_value: usize,
	pub(crate) bytes_per_slab: usize,
	pub(crate) slab_size: usize,
	pub(crate) ptr_size: usize,
	pub(crate) entry_array: Option<Array<usize>>,
	pub(crate) size: usize,
	pub(crate) head: usize,
	pub(crate) tail: usize,
	pub(crate) max_load_factor: f64,
	pub(crate) max_entries: usize,
	pub(crate) is_hashtable: bool,
	pub(crate) _phantom_data: PhantomData<K>,
	pub(crate) debug_get_next_slot_error: bool,
	pub(crate) debug_entry_array_len: bool,
}

#[derive(Debug, Clone, Serializable)]
pub(crate) struct ThreadPoolConfig {
	pub min_size: usize,
	pub max_size: usize,
	pub sync_channel_size: usize,
}

#[derive(Debug, Clone, Serializable)]
pub(crate) struct ThreadPoolState {
	pub(crate) waiting: usize,
	pub(crate) cur_size: usize,
	pub(crate) config: ThreadPoolConfig,
	pub(crate) stop: bool,
}
