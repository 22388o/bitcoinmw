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
use crate::Serializable;
use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_derive::Serializable;
use bmw_err::*;
use bmw_log::*;
use std::any::Any;
use std::cell::RefCell;
use std::fmt::Debug;
use std::future::Future;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

info!();

/// Wrapper around the lock functionalities used by bmw in [`std::sync`] rust libraries.
/// The main benefits are the simplified interface and the fact that if a thread attempts
/// to obtain a lock twice, an error will be thrown instead of a thread panic. This is implemented
/// through a thread local Hashset which keeps track of the guards used by the lock removes an
/// entry for them when the guard is dropped.
///
/// # Examples
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use std::time::Duration;
/// use std::thread::{sleep,spawn};
///
/// fn test() -> Result<(), Error> {
///     let mut lock = lock!(1)?;
///     let lock_clone = lock.clone();
///
///     spawn(move || -> Result<(), Error> {
///         let mut x = lock.wlock()?;
///         assert_eq!(**(x.guard()), 1);
///         sleep(Duration::from_millis(3000));
///         **(x.guard()) = 2;
///         Ok(())
///     });
///
///     sleep(Duration::from_millis(1000));
///     let x = lock_clone.rlock()?;
///     assert_eq!(**(x.guard()), 2);
///     Ok(())
/// }
///```
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

/// [`crate::LockBox`] is the same as [`crate::Lock`] except that it is possible to build
/// The LockBox into a Box<dyn LockBox<T>> structure so that it is object safe. It can then
/// be cloned using DynClone.
///
/// # Examples
///
///```
///
/// use bmw_err::*;
/// use bmw_util::*;
///
/// struct TestLockBox<T> {
///     lock_box: Box<dyn LockBox<T>>,
/// }
///
/// fn test_lock_box() -> Result<(), Error> {
///     let lock_box = lock_box!(1u32)?;
///     let mut lock_box2 = lock_box.clone();
///     let mut tlb = TestLockBox { lock_box };
///
///     {
///         let mut tlb = tlb.lock_box.wlock()?;
///         (**tlb.guard()) = 2u32;
///     }
///
///     {
///         let tlb = tlb.lock_box.rlock()?;
///         assert_eq!((**tlb.guard()), 2u32);
///     }
///
///     Ok(())
/// }
///
///```
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

/// The enum used to define patterns. See [`crate::pattern`] for more info.
pub enum PatternParam<'a> {
	/// The regular expression for this pattern.
	Regex(&'a str),
	/// Whether or not this is a termination pattern.
	IsTerm(bool),
	/// Whether or not this is a multi-line pattern. (allowing multi-line wildcard matches).
	IsMulti(bool),
	/// Whether or not this pattern is case sensitive.
	IsCaseSensitive(bool),
	/// The id of this pattern.
	Id(usize),
}

/// The enum used to define a suffix_tree. See [`crate::suffix_tree!`] for more info.
pub enum SuffixParam {
	/// The termination length of this suffix tree (length at which matching stops).
	TerminationLength(usize),
	/// The maximum length to look for wildcards.
	MaxWildcardLength(usize),
}

/// Configuration options used throughout this crate via macro.
#[derive(Clone, Debug)]
pub enum ConfigOption<'a> {
	/// The maximum number of entries for a data structure. See [`crate::Hashtable`] and
	/// [`crate::Hashset`].
	MaxEntries(usize),
	/// The maximum load factor for a data structure. See [`crate::Hashtable`] and
	/// [`crate::Hashset`].
	MaxLoadFactor(f64),
	/// The slab size for a slab allocator. See [`crate::SlabAllocator`].
	SlabSize(usize),
	/// The slab count for a slab allocator. See [`crate::SlabAllocator`].
	SlabCount(usize),
	/// The minimum number of threads for a thread pool. See [`crate::ThreadPool`].
	MinSize(usize),
	/// The maximum number of threads for a thread pool. See [`crate::ThreadPool`].
	MaxSize(usize),
	/// The size of the sync channel for a thread pool. See [`crate::ThreadPool`].
	SyncChannelSize(usize),
	/// Slab allocator to be used by this data structure.
	Slabs(&'a Rc<RefCell<dyn SlabAllocator>>),
}

/// Utility to write to slabs using the [`bmw_ser::Writer`] trait.
#[derive(Clone)]
pub struct SlabWriter {
	pub(crate) slabs: Option<Rc<RefCell<dyn SlabAllocator>>>,
	pub(crate) slab_id: usize,
	pub(crate) offset: usize,
	pub(crate) slab_size: usize,
	pub(crate) bytes_per_slab: usize,
}

/// Utility to read from slabs using the [`bmw_ser::Reader`] trait.
#[derive(Clone)]
pub struct SlabReader {
	pub(crate) slabs: Option<Rc<RefCell<dyn SlabAllocator>>>,
	pub(crate) slab_id: usize,
	pub(crate) offset: usize,
	pub(crate) slab_size: usize,
	pub(crate) bytes_per_slab: usize,
	pub(crate) max_value: usize,
}

/// Utility wrapper for an underlying byte Writer. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc.
pub struct BinWriter<'a> {
	pub(crate) sink: &'a mut dyn Write,
}

/// Utility wrapper for an underlying byte Reader. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc.
pub struct BinReader<'a, R: Read> {
	pub(crate) source: &'a mut R,
}

/// The configuration struct for a [`crate::ThreadPool`]. This struct is passed into the
/// [`crate::Builder::build_thread_pool`] function or the [`crate::thread_pool`] macro. The
/// [`std::default::Default`] trait is implemented for this trait. Also see [`crate::ConfigOption`]
/// for details on configuring via macro.
#[derive(Debug, Clone, Serializable)]
pub struct ThreadPoolConfig {
	/// The minimum number of threads that this thread_pool will use. The default value is 3.
	pub min_size: usize,
	/// The maximum number of threads that this thread_pool will use. The default value is 7.
	pub max_size: usize,
	/// The size of the sync_channel buffer. See [`std::sync::mpsc::sync_channel`] for more
	/// information. The default value is 7.
	pub sync_channel_size: usize,
}

/// The configuration struct for a [`Hashtable`]. This struct is passed
/// into the [`crate::Builder::build_hashtable`] function. The [`std::default::Default`]
/// trait is implemented for this trait.
#[derive(Debug, Clone, Copy, PartialEq, Serializable)]
pub struct HashtableConfig {
	/// The maximum number of entries that can exist in this [`Hashtable`].
	/// The default is 100_000. Note that the overhead for this value is 8 bytes
	/// per entry. The [`crate::HashtableConfig::max_load_factor`] setting will
	/// also affect how much memory is used by the entry array.
	pub max_entries: usize,
	/// The maximum load factor for this [`crate::Hashtable`]. This number
	/// indicates how full the hashtable can be. This is an array based hashtable
	/// and it is not possible to resize it after it is instantiated. The default value
	/// is 0.75.
	pub max_load_factor: f64,
}

/// The configuration struct for a [`Hashset`]. This struct is passed
/// into the [`crate::Builder::build_hashset`] function. The [`std::default::Default`]
/// trait is implemented for this trait.
#[derive(Debug, Clone, Copy, PartialEq, Serializable)]
pub struct HashsetConfig {
	/// The maximum number of entries that can exist in this [`Hashset`].
	/// The default is 100_000. Note that the overhead for this value is 8 bytes
	/// per entry. So, by default 8 mb are allocated with this configuration.
	pub max_entries: usize,
	/// The maximum load factor for this [`crate::Hashset`]. This number
	/// indicates how full the hashset can be. This is an array based hashset
	/// and it is not possible to resize it after it is instantiated. The default value
	/// is 0.75.
	pub max_load_factor: f64,
}

/// Slab Allocator configuration struct. This struct is the input to the
/// [`crate::SlabAllocator::init`] function. The two parameters are `slab_size`
/// which is the size of the slabs in bytes allocated by this
/// [`crate::SlabAllocator`] and `slab_count` which is the number of slabs
/// that can be allocated by this [`crate::SlabAllocator`].
#[derive(Debug, Clone, Serializable)]
pub struct SlabAllocatorConfig {
	/// The size, in bytes, of a slab
	pub slab_size: usize,
	/// The number of slabs that this slab allocator can allocate
	pub slab_count: usize,
}

/// [`crate::ArrayList`] is an array based implementation of the [`crate::List`] and
/// [`crate::SortableList`] trait. It uses the [`crate::Array`] as it's underlying
/// storage mechanism. In most cases it is more performant than the LinkedList implementation,
/// but it does do a heap allocation when created. See the module level documentation for
/// an example.
#[derive(Clone)]
pub struct ArrayList<T> {
	pub(crate) inner: Array<T>,
	pub(crate) size: usize,
	pub(crate) head: usize,
	pub(crate) tail: usize,
}

/// The [`crate::Array`] is essentially a wrapper around [`std::vec::Vec`]. It is mainly used
/// to prevent post initialization heap allocations. In many cases resizing/growing of the vec
/// is not needed and adding this wrapper assures that they do not happen. There are use cases
/// where growing is useful and in fact in the library we use Vec for the suffix tree, but in most
/// cases where contiguous memory is used, the primary purpose is for indexing. That can be done
/// with an array. So we add this wrapper.
///
/// # Examples
///```
/// use bmw_err::Error;
/// use bmw_util::array;
///
/// fn main() -> Result<(), Error> {
///      let mut arr = array!(10, &0)?;
///      for i in 0..10 {
///          arr[i] = i;
///      }
///
///      for i in 0..10 {
///          assert_eq!(arr[i], i);
///      }
///      Ok(())
/// }
///```
pub struct Array<T> {
	pub(crate) data: Vec<T>,
}

/// Configuration for Lists currently there are no parameters, but it is still used
/// to stay consistent with [`crate::Hashtable`] and [`crate::Hashset`].
#[derive(Debug, Clone, Serializable)]
pub struct ListConfig {}

/// A slab allocated hashtable. Data is stored in a [`crate::SlabAllocator`] defined
/// by the user or using a global thread local slab allocator. All keys and values
/// must implement the [`bmw_ser::Serializable`] trait which can be implemented with
/// the [`bmw_derive::Serializable`] proc_macro.
///
/// # Sizing
///
/// It is important to configure this hashtable correctly and to do so, the user must
/// understand how the data are stored. Each hashtable is configured with an associated
/// [`crate::SlabAllocator`] and that slab allocator has a specific size for each slab.
/// The hashtable will only store a single entry in a slab. No slab contains data from
/// multiple entries. So, if you have a large slab size and small entries, you will be
/// wasting space. If your data is fixed in size, you can calculate an optimal slab size,
/// but if it is variable, it is probably better to use smaller slabs so that less space
/// is left empty. The slab layout looks like this:
///
/// [ ptr_size bytes for next iterator list]
/// [ ptr_size bytes for prev iterator list]
/// [ hashtable key ]
/// [ hashtable value ]
/// ... (empty space if the slab is not filled up)
/// [ ptr_size bytes for key/value list ]
///
/// where ptr_size is determined by the size of the entry array. The minimum number of
/// bytes will be used. So, if you have an entry array that is length less than 256
/// ptr_size will be 1 byte. If you have an entry array that is length less than
/// 65,536 but greater than or equal to 256, ptr_size will be 2 and so on. If the
/// data for the entry only takes up one slab, the key/value list is not used, but
/// if the data takes up more than one slab, the key/value list will point to the next
/// slab in the list. So, if your key is always 8 bytes and your value is always 8 bytes
/// and you have an entry array size of 100,000 (ptr_size = 3 bytes), you can will need
/// a total of 9 bytes overhead for the three pointers and you will need 16 bytes for your
/// data. So you can size your hashtable at 25 bytes.
///
/// Note: your entry array size is ceil(the max_entries / max_load_factor).
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_util::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         let slabs = slab_allocator!(SlabSize(128), SlabCount(10_000))?;
///         let mut hashtable = hashtable!(Slabs(&slabs))?;
///
///         hashtable.insert(&1, &2)?;
///         let v = hashtable.get(&1)?.unwrap();
///         info!("v={}", v)?;
///         assert_eq!(v, 2);
///
///         Ok(())
/// }
///```
///
///```
/// use bmw_err::*;
/// use bmw_util::*;
/// use bmw_log::*;
/// use std::collections::HashMap;
/// use bmw_deps::rand::random;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let mut keys = vec![];
///     let mut values = vec![];
///     for _ in 0..1_000 {
///         keys.push(random::<u32>());
///         values.push(random::<u32>());
///     }
///     let mut hashtable = Builder::build_hashtable(HashtableConfig::default(), &None)?;
///     let mut hashmap = HashMap::new();
///     for i in 0..1_000 {
///         hashtable.insert(&keys[i], &values[i])?;
///         hashmap.insert(&keys[i], &values[i]);
///     }
///
///     for _ in 0..100 {
///         let index: usize = random::<usize>() % 1_000;
///         hashtable.remove(&keys[index])?;
///         hashmap.remove(&keys[index]);
///     }
///
///     let mut i = 0;
///     for (k, vm) in &hashmap {
///         let vt = hashtable.get(&k)?;
///         assert_eq!(&vt.unwrap(), *vm);
///         i += 1;
///     }
///
///     assert_eq!(i, hashtable.size());
///     assert_eq!(i, hashmap.len());
///
///     Ok(())
/// }
///```
///
///```
/// // import the util/log/err libraries
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
/// use bmw_derive::*;
///
/// info!();
///
/// #[derive(Serializable, Clone, Debug, PartialEq)]
/// struct MyStruct {
///     id: u128,
///     name: String,
///     phone: Option<String>,
///     age: u8,
/// }
///
/// fn main() -> Result<(), Error> {
///     let s = MyStruct {
///         id: 1234,
///         name: "Hagrid".to_string(),
///         phone: None,
///         age: 54,
///     };
///
///     debug!("my struct = {:?}", s)?;
///
///     let mut hashtable = hashtable!()?;
///
///     hashtable.insert(&1, &s)?;
///
///     let v = hashtable.get(&1)?;
///     assert_eq!(v, Some(s));
///
///     info!("value of record #1 is {:?}", v)?;
///
///     Ok(())
/// }
///
///```
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
	fn raw_read(
		&self,
		key: &K,
		offset: usize,
		data: &mut [u8; CACHE_BUFFER_SIZE],
	) -> Result<bool, Error>;
	/// Write raw data from `data` with given offset.
	fn raw_write(
		&mut self,
		key: &K,
		offset: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
	) -> Result<(), Error>;
	/// Gets the slab allocator associated with this Hashtable or None if the global slab
	/// allocator is used.
	fn slabs(&self) -> Result<Option<Rc<RefCell<dyn SlabAllocator>>>, Error>;
}

/// A slab allocated Hashset. Most of the implementation is shared with [`crate::Hashtable`].
/// See [`crate::Hashtable`] for a discussion of the slab layout. The difference is that as is
/// the case with hashsets, there is no value.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///      let slabs = slab_allocator!()?;
///      let mut hashset = hashset!(MaxEntries(1_000), Slabs(&slabs))?;
///
///      hashset.insert(&1)?;
///      assert_eq!(hashset.size(), 1);
///      assert!(hashset.contains(&1)?);
///      assert!(!hashset.contains(&2)?);
///      assert_eq!(hashset.remove(&1)?, true);
///      assert_eq!(hashset.remove(&1)?, false);
///      assert_eq!(hashset.size(), 0);
///
///      hashset.insert(&1)?;
///      hashset.clear()?;
///      assert_eq!(hashset.size(), 0);
///
///      Ok(())
/// }
///```
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
}

/// This trait defines a queue. The implementation is a bounded queue.
/// The queue uses an [`crate::Array`] as the underlying storage mechanism.
///
/// Examples:
///
///```
/// // import the util/log/err libraries
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///     // create a queue with capacity of 1_000 items and &0
///     //is the default value used to initialize the array
///     let mut queue = queue!(1_000, &0)?;
///
///     // add three items to the queue
///     queue.enqueue(1)?;
///     queue.enqueue(2)?;
///     queue.enqueue(3)?;
///
///     // dequeue the first item
///     assert_eq!(queue.dequeue(), Some(&1));
///
///     // dequeue the second item
///     assert_eq!(queue.dequeue(), Some(&2));
///
///     // peek at the last item
///     assert_eq!(queue.peek(), Some(&3));
///
///     // dequeue the last item
///     assert_eq!(queue.dequeue(), Some(&3));
///
///     // now that the queue is empty, dequeue returns None
///     assert_eq!(queue.dequeue(), None);
///
///     Ok(())
/// }
///```
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

/// This trait defines a stack. The implementation is a bounded stack.
/// The stack uses an [`crate::Array`] as the underlying storage mechanism.
///
/// Examples:
///
///```
/// // import the util/log/err libraries
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///     // create a stack with capacity of 1_000 items and &0
///     //is the default value used to initialize the array
///     let mut stack = stack!(1_000, &0)?;
///
///     // add three items to the stack
///     stack.push(1)?;
///     stack.push(2)?;
///     stack.push(3)?;
///
///     // pop the first item
///     assert_eq!(stack.pop(), Some(&3));
///
///     // pop the second item
///     assert_eq!(stack.pop(), Some(&2));
///
///     // peek at the last item
///     assert_eq!(stack.peek(), Some(&1));
///
///     // pop the last item
///     assert_eq!(stack.pop(), Some(&1));
///
///     // now that the stack is empty, pop returns None
///     assert_eq!(stack.pop(), None);
///
///     Ok(())
/// }
///```
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

/// A trait that defines a list. Both an array list and a linked list
/// are implemented the default [`crate::list`] macro returns the linked list.
/// The [`crate::array_list`] macro returns the array list. Both implement
/// this trait.
///
/// # Examples
///
///```
/// // import the util/log/err libraries
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///     // for this example we will use the global slab allocator which is a
///     // thread local slab allocator which we can configure via macro
///     global_slab_allocator!(SlabSize(64), SlabCount(100_000))?;
///
///     // create two lists (one linked and one array list).
///     // Note that all lists created via macro are interoperable.
///     let mut list1 = list![1u32, 2u32, 4u32, 5u32];
///     let mut list2 = array_list!(10, &0u32)?;
///     list2.push(5)?;
///     list2.push(2)?;
///     list2.push(4)?;
///     list2.push(1)?;
///
///     // sort the array list and assert it's equal to the other list
///     list2.sort()?;
///     assert!(list_eq!(list1, list2));
///
///     // append list2 to list1 and assert the value
///     list_append!(list1, list2);
///     assert!(list_eq!(list1, list![1, 2, 4, 5, 1, 2, 4, 5]));
///
///     Ok(())
/// }
///```
pub trait List<V>: DynClone + Debug {
	/// push a value onto the end of the list
	fn push(&mut self, value: V) -> Result<(), Error>;
	/// iterate through the list
	fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = V> + 'a>
	where
		V: Serializable + Clone;
	/// iterate through the list in reverse order
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

/// A trait that defines a sortable list. Both implementations
/// implement this trait, although the linked list merely copies
/// the data into an array list to sort. The array_list can natively sort
/// with rust's sort implementations using the functions associated
/// with slice.
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

clone_trait_object!(<V>Queue<V>);
clone_trait_object!(<V>Stack<V>);
clone_trait_object!(<V>Hashset<V>);
clone_trait_object!(<K, V>Hashtable<K, V>);
clone_trait_object!(<V>List<V>);
clone_trait_object!(<V>SortableList<V>);
clone_trait_object!(SuffixTree);

/// The result returned from a call to [`crate::ThreadPool::execute`]. This is
/// similar to [`std::result::Result`] except that it implements [`std::marker::Send`]
/// and [`std::marker::Sync`] so that it can be passed through threads. Also a type
/// [`crate::PoolResult::Panic`] is returned if a thread panic occurs in the thread pool.
#[derive(Debug, PartialEq)]
pub enum PoolResult<T, E> {
	Ok(T),
	Err(E),
	Panic,
}

/// This trait defines the public interface to the ThreadPool. A pool can be configured
/// via the [`crate::ThreadPoolConfig`] struct. The thread pool should be accessed through the
/// macros under normal circumstances. See [`crate::thread_pool`], [`crate::execute`] and
/// [`crate::block_on`] for additional details. The thread pool can be passed through threads via a
/// [`crate::Lock`] or [`crate::LockBox`] so a single thread pool can service multiple
/// worker threads. See examples below.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
/// use bmw_util::ConfigOption::{MaxSize, MinSize};
///
/// info!();
///
/// fn thread_pool() -> Result<(), Error> {
///     let mut tp = thread_pool!()?; // create a thread pool using default settings
///     tp.set_on_panic(move |_id,_e| -> Result<(), Error> { Ok(()) })?;
///
///     // create a shared variable protected by the [`crate::lock`] macro.
///     let mut shared = lock!(0)?; // we use an integer 0, but any struct can be used.
///     let shared_clone = shared.clone();
///
///     // execute a task
///     let handle = execute!(tp, {
///         // obtain the lock in write mode
///         let mut shared = shared.wlock()?;
///         // increment the counter
///         (**shared.guard()) += 1;
///         // return the counter value (now it's 1)
///         Ok((**shared.guard()))
///     })?;
///
///     // block on the returned handle until it's task is complete and assert that the
///     // return value is 1.
///     assert_eq!(block_on!(handle), PoolResult::Ok(1));
///
///     // check our shared value which was updated in the thread pool.
///     let shared_clone = shared_clone.rlock()?;
///     assert_eq!((**shared_clone.guard()), 1);
///
///     Ok(())
/// }
///
/// fn thread_pool2() -> Result<(), Error> {
///     // create a thread pool with the specified max/min size. See [`crate::ThreadPoolConfig`]
///     // for further details.
///     let mut tp = thread_pool!(MaxSize(10), MinSize(5))?;
///     tp.set_on_panic(move |_id,_e| -> Result<(), Error> { Ok(()) })?;
///
///     // put the thread pool in a [`crate::Lock`].
///     let tp = lock!(tp)?;
///
///     // spawn 6 worker threads
///     for _ in 0..6 {
///         // create a clone of our locked thread pool for each worker thread
///         let tp = tp.clone();
///         std::thread::spawn(move || -> Result<(), Error> {
///             // do some work here and pass other work to the thread pool.
///             // ...
///
///             // obtain a read lock on the thread pool
///             let tp = tp.rlock()?;
///
///             // execute the task in the thread pool, the returned handle can be used
///             // if desired or ignored
///             execute!((**tp.guard()), {
///                 info!("executing in thread pool")?;
///                 Ok(1)
///             })?;
///             Ok(())
///         });
///     }
///     Ok(())
/// }
///
///```
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
	/// on the first available thread in the pool. The return value is a receiver
	/// which will be sent a [`crate::PoolResult`] on completion of the task. If
	/// an error occurs, [`bmw_err::Error`] will be returned.
	fn execute<F>(&self, f: F, id: u128) -> Result<Receiver<PoolResult<T, Error>>, Error>
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
	/// [`crate::ThreadPoolConfig::min_size`] and [`crate::ThreadPoolConfig::max_size`].
	fn size(&self) -> Result<usize, Error>;

	/// Get the [`crate::ThreadPoolStopper`] for this thread pool.
	fn stopper(&self) -> Result<ThreadPoolStopper, Error>;

	/// Get the [`crate::ThreadPoolExecutor`] for this thread pool.
	fn executor(&self) -> Result<ThreadPoolExecutor<T>, Error>
	where
		T: Send + Sync;

	/// Set an on panic handler for this thread pool
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
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

/// This trait defines the public interface to the [`crate::SlabAllocator`]. The slab
/// allocator is used by the other data structures in this crate to avoid dynamic heap
/// allocations. By itself, the slab allocator is fairly simple. It only allocates and frees
/// slabs. [`crate::SlabAllocator::get`] and [`crate::SlabAllocator::get_mut`] are also
/// provided to obtain immutable and mutable references to a slab respectively. They only
/// contain references to the data and not copies.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_util::slab_allocator;
/// use std::cell::{RefMut,Ref};
///
/// fn main() -> Result<(), Error> {
///     // build a slab allocator, in this case with defaults
///     let slabs = slab_allocator!()?;
///
///     let id = {
///         // slab allocator is stored in an Rc<RefCell>. This allows for it to be used by
///         // multiple data structures at the same time.
///         let mut slabs: RefMut<_> = slabs.borrow_mut();
///         // allocate a slab. [`crate::SlabAllocator::allocate`] returns [`crate::SlabMut`]
///         // which contains a mutable reference to the underlying data in the slab.
///         let mut slab = slabs.allocate()?;
///
///         // get the id for this slab
///         let id = slab.id();
///         // get_mut returns a mutable reference to the data in owned by the
///         // [`crate::SlabAllocator`]
///         slab.get_mut()[0] = 101;
///         id
///     };
///
///     // borrow, this time with a Ref instead of refmut since it's an immutable call.
///     let slabs: Ref<_> = slabs.borrow();
///     // now we can get an immutable reference to this slab
///     let slab = slabs.get(id)?;
///     assert_eq!(slab.get()[0], 101);
///
///     Ok(())
/// }
///```
pub trait SlabAllocator: DynClone + Debug {
	/// If the slab allocator has been initialized, return true, otherwise, false.
	fn is_init(&self) -> bool;

	/// Allocate a slab and return a [`crate::SlabMut`] on success.
	/// On failure, return an [`bmw_err::Error`].
	///
	/// * [`bmw_err::ErrorKind::IllegalState`] if the [`crate::SlabAllocator::init`]
	/// function has not been called.
	///
	/// * [`bmw_err::ErrorKind::CapacityExceeded`] if the capacity of this
	/// [`crate::SlabAllocator`] has been exceeded.
	fn allocate<'a>(&'a mut self) -> Result<SlabMut<'a>, Error>;

	/// Free a slab that has previously been allocated by this slab allocator.
	/// `id` is the id of the slab to free. It can be obtained through the
	/// [`crate::SlabMut::id`] or [`crate::Slab::id`] function. Return a
	/// [`bmw_err::Error`] on failure.
	///
	/// *  [`bmw_err::ErrorKind::ArrayIndexOutOfBounds`] if this slab entry is
	/// too big for this instance.
	///
	/// * [`bmw_err::ErrorKind::IllegalState`] if the [`crate::SlabAllocator::init`]
	/// function has not been called or this slab was not allocated.
	///
	/// # Examples
	///
	///```
	/// use bmw_err::*;
	/// use bmw_util::slab_allocator;
	/// use bmw_util::ConfigOption::{SlabSize, SlabCount};
	///
	/// fn main() -> Result<(), Error> {
	///     // instantiate a slab allocator with a slab count of 1,000.
	///     let mut slabs = slab_allocator!(SlabSize(1_000), SlabCount(1_000))?;
	///
	///     // borrow a mutable reference
	///     let mut slabs = slabs.borrow_mut();
	///
	///     // assert that there are 1,000 free slabs.
	///     assert_eq!(slabs.free_count()?, 1_000);
	///
	///     let slab_id = {
	///         // allocate a slab.
	///         let slab = slabs.allocate()?;
	///         slab.id()
	///     };
	///
	///     // assert that the free count has decreased by 1.
	///     assert_eq!(slabs.free_count()?, 999);
	///
	///
	///     // free the slab that was allocated
	///     slabs.free(slab_id)?;
	///
	///     // assert that the free count has returned to the initial value of 1,000.
	///     assert_eq!(slabs.free_count()?, 1_000);
	///
	///     Ok(())
	/// }
	///```
	fn free(&mut self, id: usize) -> Result<(), Error>;

	/// Get an immutable reference to a slab that has previously been allocated by the
	/// [`crate::SlabAllocator`]. On success a [`crate::Slab`] is returned. On failure,
	/// a [`bmw_err::Error`] is returned.
	///
	/// *  [`bmw_err::ErrorKind::ArrayIndexOutOfBounds`] if this slab entry is
	/// too big for this instance.
	///
	/// * [`bmw_err::ErrorKind::IllegalState`] if the [`crate::SlabAllocator::init`]
	/// function has not been called or this slab was not allocated.
	///
	/// # Examples
	///
	///```
	/// use bmw_err::*;
	/// use bmw_log::*;
	/// use bmw_util::slab_allocator;
	/// use bmw_util::ConfigOption::{SlabSize,SlabCount};
	///
	/// info!();
	///
	/// fn main() -> Result<(), Error> {
	///     // instantiate a slab allocator with a slab count of 1,000.
	///     let mut slabs = slab_allocator!(SlabCount(1_000), SlabSize(1_000))?;
	///     
	///     // borrow a mutable reference
	///     let mut slabs = slabs.borrow_mut();
	///
	///     // assert that there are 1,000 free slabs.
	///     assert_eq!(slabs.free_count()?, 1_000);
	///
	///     let slab_id = {
	///         // allocate a slab.
	///         let slab = slabs.allocate()?;
	///         slab.id()
	///     };
	///
	///     // assert that the free count has decreased by 1.
	///     assert_eq!(slabs.free_count()?, 999);
	///
	///
	///     // get the slab that was allocated
	///     let slab = slabs.get(slab_id)?;
	///
	///     info!("slab data = {:?}", slab.get())?;
	///
	///     Ok(())
	/// }
	///```
	fn get<'a>(&'a self, id: usize) -> Result<Slab<'a>, Error>;

	/// Get an mutable reference to a slab that has previously been allocated by the
	/// [`crate::SlabAllocator`]. On success a [`crate::SlabMut`] is returned. On failure,
	/// a [`bmw_err::Error`] is returned.
	///
	/// *  [`bmw_err::ErrorKind::ArrayIndexOutOfBounds`] if this slab entry is
	/// too big for this instance.
	///
	/// * [`bmw_err::ErrorKind::IllegalState`] if the [`crate::SlabAllocator::init`]
	/// function has not been called or this slab was not allocated.
	///
	/// # Examples
	///
	///```
	/// use bmw_err::*;
	/// use bmw_log::*;
	/// use bmw_util::slab_allocator;
	/// use bmw_util::ConfigOption::{SlabSize, SlabCount};
	///
	/// info!();
	///
	/// fn main() -> Result<(), Error> {
	///     // instantiate a slab allocator with a slab count of 1,000.
	///     let mut slabs = slab_allocator!(SlabSize(1_000), SlabCount(1_000))?;
	///
	///     // borrow a mutable reference
	///     let mut slabs = slabs.borrow_mut();
	///
	///     // assert that there are 1,000 free slabs.
	///     assert_eq!(slabs.free_count()?, 1_000);
	///
	///     let slab_id = {
	///         // allocate a slab.
	///         let slab = slabs.allocate()?;
	///         slab.id()
	///     };
	///
	///     // assert that the free count has decreased by 1.
	///     assert_eq!(slabs.free_count()?, 999);
	///
	///
	///     // get the slab that was allocated
	///     let mut slab = slabs.get_mut(slab_id)?;
	///
	///     info!("slab data = {:?}", slab.get_mut())?;
	///
	///     Ok(())
	/// }
	///```
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

clone_trait_object!(SlabAllocator);

/// A pattern which is used with the suffix tree. See [`crate::suffix_tree!`].
#[derive(Debug, PartialEq, Clone)]
pub struct Pattern {
	pub(crate) regex: String,
	pub(crate) is_case_sensitive: bool,
	pub(crate) is_termination_pattern: bool,
	pub(crate) is_multi_line: bool,
	pub(crate) id: usize,
}

/// A match which is returned by the suffix tree. See [`crate::suffix_tree!`].
#[derive(Clone, Copy, Debug)]
pub struct Match {
	pub(crate) start: usize,
	pub(crate) end: usize,
	pub(crate) id: usize,
}

/// The suffix tree data structure. See [`crate::suffix_tree!`].
pub trait SuffixTree: DynClone {
	/// return matches associated with the supplied `text` for this
	/// [`crate::SuffixTree`]. Matches are returned in the `matches`
	/// array supplied by the caller. The result is the number of
	/// matches found or a [`bmw_err::Error`] if an error occurs.
	fn tmatch(&mut self, text: &[u8], matches: &mut [Match]) -> Result<usize, Error>;
}

/// The builder struct for the util library. All data structures are built through this
/// interface. This is used by the macros as well.
pub struct Builder {}

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

/// An iterator for the [`crate::List`].
pub struct ListIterator<'a, V>
where
	V: Serializable + Clone,
{
	pub(crate) linked_list_ref: Option<&'a HashImpl<V>>,
	pub(crate) array_list_ref: Option<&'a ArrayList<V>>,
	pub(crate) cur: usize,
	pub(crate) direction: Direction,
	pub(crate) _phantom_data: PhantomData<V>,
	pub(crate) slab_reader: Option<SlabReader>,
}

/// An iterator for the [`crate::ArrayList`].
pub struct ArrayListIterator<'a, T> {
	pub(crate) array_list_ref: &'a ArrayList<T>,
	pub(crate) direction: Direction,
	pub(crate) cur: usize,
}

/// An iterator for the [`crate::Array`].
pub struct ArrayIterator<'a, T> {
	pub(crate) array_ref: &'a Array<T>,
	pub(crate) cur: usize,
}

/// Internally used struct that stores the state of the thread pool.
#[derive(Debug, Clone, Serializable)]
pub struct ThreadPoolState {
	pub(crate) waiting: usize,
	pub(crate) cur_size: usize,
	pub(crate) config: ThreadPoolConfig,
	pub(crate) stop: bool,
}

/// Struct that can be used to execute tasks in the thread pool. Mainly needed
/// for passing the execution functionality to structs/threads.
#[derive(Debug, Clone)]
pub struct ThreadPoolExecutor<T>
where
	T: 'static + Send + Sync,
{
	pub(crate) config: ThreadPoolConfig,
	pub(crate) tx: Option<SyncSender<FutureWrapper<T>>>,
}

/// Struct that can be used to stop the thread pool. Note the limitations
/// in [`crate::ThreadPoolStopper::stop`].
#[derive(Debug, Clone)]
pub struct ThreadPoolStopper {
	pub(crate) state: Box<dyn LockBox<ThreadPoolState>>,
}

// crate local structs/enums

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
pub(crate) struct SuffixTreeImpl {
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
	pub(crate) slabs: Option<Rc<RefCell<dyn SlabAllocator>>>,
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

#[derive(Clone)]
pub(crate) struct LockImpl<T> {
	pub(crate) t: Arc<RwLock<T>>,
	pub(crate) id: u128,
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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::types::ConfigOption;
	use crate::{deserialize, serialize, slab_allocator};
	use bmw_err::*;

	#[test]
	fn test_config_option_ser() -> Result<(), Error> {
		let config_option = ConfigOption::MinSize(10);
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &config_option)?;
		let ser_in: ConfigOption = deserialize(&mut &v[..])?;
		assert!(matches!(ser_in, ConfigOption::MinSize(_)));

		let config_option = ConfigOption::MaxSize(10);
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &config_option)?;
		let ser_in: ConfigOption = deserialize(&mut &v[..])?;
		assert!(matches!(ser_in, ConfigOption::MaxSize(_)));

		let config_option = ConfigOption::SyncChannelSize(10);
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &config_option)?;
		let ser_in: ConfigOption = deserialize(&mut &v[..])?;
		assert!(matches!(ser_in, ConfigOption::SyncChannelSize(_)));

		let v = vec![7u8]; // 7 is illegal
		let err: Result<ConfigOption, Error> = deserialize(&mut &v[..]);
		assert!(err.is_err());

		let slabs = slab_allocator!()?;
		let config_option = ConfigOption::Slabs(&slabs);
		let mut v: Vec<u8> = vec![];
		assert!(serialize(&mut v, &config_option).is_err());

		Ok(())
	}
}
