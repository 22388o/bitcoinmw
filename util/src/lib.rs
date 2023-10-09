// Copyright (c) 2023, The BitcoinMW Developers // Some code and concepts from: // * Grin: https://github.com/mimblewimble/grin // * Arti: https://gitlab.torproject.org/tpo/core/arti // * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw //
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

//! The util crate provides the data structures that are used in bmw. The data structures
//! implemented include [`crate::Hashtable`], [`crate::Hashset`], [`crate::List`], [`crate::Array`],
//! [`crate::ArrayList`], [`crate::Stack`], [`crate::Queue`], and [`crate::SuffixTree`]. Additionally
//! a [`crate::ThreadPool`] implementation is provided. Each data structure can be instantiated in
//! a impl and Box form using the [`crate::Builder`] or through macros. The impls completely stack
//! based and the box forms are Box<dyn ..>'s that can be stored in other structs and Enums. While
//! the boxed versions do store data on the heap, it is only pointers and the vast majority of the
//! data, in either form, is stored in pre-allocated slabs. The array based structures do use
//! the heap, they use [`std::vec::Vec`] as the underlying storage mechanism, but they only allocate
//! heap memory when they are created and none afterwords.
//!
//! # Motivation
//!
//! The advantage of these implementations is that they do not allocate memory on the
//! heap after initialization of the data structure. So, we can create a [`crate::hashtable`],
//! [`crate::List`] or a [`crate::hashset`] and once created, do millions of operations and
//! no heap memory will be allocated or deallocated. Dynamic heap allocations that are long-lived can cause
//! substantial problems like slowness and memory fragmentation and even system crashes and these data structures
//! are intended to alleviate those problems. The [`core::ops::Drop`] trait is also implemented so all
//! slabs used by the data structure are freed when the data structure goes out of scope.
//!
//! # Performance
//!
//! The hashtable/set are not as fast as the native Rust data structures because they
//! require serialization and deserialization of the entries on each operation. However, the
//! performance is at least in the ballpark of the standard data structures. The array, arraylist,
//! queue, and stack are faster for insert, slower for initialization and about the same for
//! iteration and drop. A performance tool is included in the project in the etc directory
//! [ds_perf](https://github.com/cgilliard/bitcoinmw/tree/main/etc/ds_perf).
//!
//! Below is the output of some runs on a linux machine.
//!
//!```text
//! $ ./target/release/perf  --help
//! ds_perf 1.0
//! The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//!
//! USAGE:
//!    perf [FLAGS]
//!
//! FLAGS:
//!     --array           run tests for array
//!     --array_string    run tests for array with strings
//!     --arraylist       run tests for array list
//!     --hashmap         run tests for standard rust library hashmap
//!     --hashtable       run tests for hashtable
//! -h, --help            Prints help information
//! -V, --version         Prints version information
//!     --vec             run tests for standard rust library vec
//!     --vec_string      run tests for vec with strings
//!
//! $ ./target/release/perf  --array_string
//! [2022-09-01 13:02:41.974]: Starting ds_perf
//! [2022-09-01 13:02:41.974]: Testing array string
//! [2022-09-01 13:02:41.974]: array init: alloc: 240,000, dealloc: 0, alloc_qty: 1, dealloc_qty: 0, delta: 240,000, elapsed: 180.336µs
//! [2022-09-01 13:02:41.975]: array insert: alloc: 110,000, dealloc: 0, alloc_qty: 10,000, dealloc_qty: 0, delta: 110,000, elapsed: 332.891µs
//! [2022-09-01 13:02:41.975]: array iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 27ns
//! [2022-09-01 13:02:41.975]: array drop: alloc: 0, dealloc: 350,000, alloc_qty: 0, dealloc_qty: 10,001, delta: -350,000, elapsed: 121.148µs
//!
//! $ ./target/release/perf  --vec_string
//! [2022-09-01 13:02:38.151]: Starting ds_perf
//! [2022-09-01 13:02:38.151]: Testing vec string
//! [2022-09-01 13:02:38.151]: vec init: alloc: 240,000, dealloc: 0, alloc_qty: 1, dealloc_qty: 0, delta: 240,000, elapsed: 177.351µs
//! [2022-09-01 13:02:38.151]: vec insert: alloc: 110,000, dealloc: 0, alloc_qty: 10,000, dealloc_qty: 0, delta: 110,000, elapsed: 324.671µs
//! [2022-09-01 13:02:38.151]: vec iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 40ns
//! [2022-09-01 13:02:38.152]: vec drop: alloc: 0, dealloc: 350,000, alloc_qty: 0, dealloc_qty: 10,001, delta: -350,000, elapsed: 121.868µs
//!
//! $ ./target/release/perf  --array
//! [2022-09-01 13:00:22.190]: Starting ds_perf
//! [2022-09-01 13:00:22.190]: Testing array
//! [2022-09-01 13:00:22.190]: array init: alloc: 40,000, dealloc: 0, alloc_qty: 1, dealloc_qty: 0, delta: 40,000, elapsed: 27.454µs
//! [2022-09-01 13:00:22.190]: array insert: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 999ns
//! [2022-09-01 13:00:22.190]: array iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 29ns
//! [2022-09-01 13:00:22.190]: array drop: alloc: 0, dealloc: 40,000, alloc_qty: 0, dealloc_qty: 1, delta: -40,000, elapsed: 121ns
//!
//! $ ./target/release/perf  --vec
//! [2022-09-01 13:00:27.820]: Starting ds_perf
//! [2022-09-01 13:00:27.820]: testing vec
//! [2022-09-01 13:00:27.820]: vec init: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 120ns
//! [2022-09-01 13:00:27.820]: vec insert: alloc: 131,056, dealloc: 65,520, alloc_qty: 13, dealloc_qty: 12, delta: 65,536, elapsed: 48.554µs
//! [2022-09-01 13:00:27.820]: vec iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 29ns
//! [2022-09-01 13:00:27.820]: vec drop: alloc: 0, dealloc: 65,536, alloc_qty: 0, dealloc_qty: 1, delta: -65,536, elapsed: 5.096µs
//!
//! $ ./target/release/perf  --hashtable
//! [2022-09-01 13:07:30.861]: Starting ds_perf
//! [2022-09-01 13:07:30.861]: Testing hashtable
//! [2022-09-01 13:07:30.861]: hashtable init: alloc: 100,096, dealloc: 96, alloc_qty: 2, dealloc_qty: 1, delta: 100,000, elapsed: 55.208µs
//! [2022-09-01 13:07:30.863]: hashtable insert: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 2.296961ms
//! [2022-09-01 13:07:30.865]: hashtable get: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 1.462402ms
//! [2022-09-01 13:07:30.865]: hashtable drop: alloc: 100,000, dealloc: 200,000, alloc_qty: 1, dealloc_qty: 2, delta: -100,000, elapsed: 512.423µs
//!
//! $ ./target/release/perf  --hashmap
//! [2022-09-01 13:07:39.269]: Starting ds_perf
//! [2022-09-01 13:07:39.269]: Testing hashmap
//! [2022-09-01 13:07:39.269]: hashmap init: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 69ns
//! [2022-09-01 13:07:39.270]: hashmap insert: alloc: 688,252, dealloc: 344,172, alloc_qty: 26, dealloc_qty: 24, delta: 344,080, elapsed: 611.785µs
//! [2022-09-01 13:07:39.270]: hashmap get: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 199.428µs
//! [2022-09-01 13:07:39.270]: hashmap drop: alloc: 0, dealloc: 344,080, alloc_qty: 0, dealloc_qty: 2, delta: -344,080, elapsed: 25.252µs
//!
//! $ ./target/release/perf  --arraylist
//! [2022-09-01 13:08:08.434]: Starting ds_perf
//! [2022-09-01 13:08:08.435]: testing arraylist
//! [2022-09-01 13:08:08.435]: arraylist init: alloc: 40,000, dealloc: 0, alloc_qty: 1, dealloc_qty: 0, delta: 40,000, elapsed: 27.827µs
//! [2022-09-01 13:08:08.435]: arraylist insert: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 18.963µs
//! [2022-09-01 13:08:08.435]: arraylist iter: alloc: 24, dealloc: 24, alloc_qty: 1, dealloc_qty: 1, delta: 0, elapsed: 20.341µs
//! [2022-09-01 13:08:08.435]: arraylist drop: alloc: 1,856, dealloc: 41,856, alloc_qty: 27, dealloc_qty: 28, delta: -40,000, elapsed: 110ns
//!
//!
//! ```
//!
//! So, as seen, the performance is somewhat comparable and keep in mind, that these operations are
//! being done in the sub millisecond time frame, so it's very unlikely that this will ultimately
//! impact the performance of the application unless it's an already very optimized application
//! that needs to be very scalable.
//!
//! # Using bmw_util (and other crates) in your project
//!
//! To use the crates in bmw in your project, add the following to your Cargo.toml:
//!
//!```text
//! bmw_util   = { git = "https://github.com/cgilliard/bitcoinmw"  }
//!```
//!
//! Optionally, you may wish to use the other associated crates:
//!
//!```text
//! bmw_err    = { git = "https://github.com/cgilliard/bitcoinmw"  }
//! bmw_log    = { git = "https://github.com/cgilliard/bitcoinmw"  }
//! bmw_derive = { git = "https://github.com/cgilliard/bitcoinmw"  }
//!```
//!
//! The linux dependencies can be installed with the following commands on ubuntu:
//!
//!```text
//! $ sudo apt-get update -yqq
//! $ sudo apt-get install -yqq --no-install-recommends libncursesw5-dev libssl-dev
//!```
//!
//! The macos dependencies can be installed with the following commands
//! ```text
//! $ brew install llvm
//! ```
//!
//! The windows dependencies can be installed with the following commands
//!
//! ```text
//! $ choco install -y llvm
//! ```
//!
//! BitcoinMW is tested with the latest version of rust. Please ensure to update it.
//!
//! # Use cases
//!
//! The main use case for these data structures is in server applications to avoid making dynamic
//! heap allocations at runtime, but they also offer some other interesting properties. For instance, with
//! the standard rust collections, the entries in the hashmap are just references so they must
//! stay in scope while they are in the hashmap. With this implementation, that is not required.
//! The inserted items can be dropped and they will remain in the hashtable/hashset. Also,
//! [`crate::Hashtable`] and [`crate::Hashset`] both implement the
//! [`bmw_ser::Serializable`] trait so they can be sent from one part of an app to another or even
//! sent over the network.
//!
//! # Examples
//!
//! Use of the slab allocator, hashtable, and hashset via macros.
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! info!();
//!
//! fn main() -> Result<(), Error> {
//!     // create a slab allocator to be used by our data structures
//!     let slabs = slab_allocator!(SlabSize(128), SlabCount(15_000))?;
//!
//!     // instantiate a hashtable with specified MaxEntries and MaxLoadFactor
//!     let mut hashtable = hashtable!(MaxEntries(10_000), MaxLoadFactor(0.85), Slabs(&slabs))?;
//!
//!     // insert a key/value pair (rust figures out the data types)
//!     hashtable.insert(&1, &2)?;
//!
//!     // get the value for our key and assert it is correct
//!     let v = hashtable.get(&1)?;
//!     assert_eq!(v, Some(2));
//!     info!("value = {:?}", v)?;
//!
//!     // instantiate a hashset that uses the same slab allocator. Since we don't specify
//!     // the MaxLoadFactor, the default value of 0.8 will be used for this hashset.
//!     let mut hashset = hashset!(MaxEntries(5_000), Slabs(&slabs))?;
//!
//!     // insert a key
//!     hashset.insert(&"test".to_string())?;
//!
//!     // assert that our key is found in the hashset
//!     assert!(hashset.contains(&"test".to_string())?);
//!
//!     // do a negative assertion
//!     assert!(!hashset.contains(&"test2".to_string())?);
//!
//!     info!("complete!")?;
//!
//!     Ok(())
//! }
//!
//!```
//!
//! Use of Sortable lists
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // for this example we will use the global slab allocator which is a
//!     // thread local slab allocator which we can configure via macro
//!     global_slab_allocator!(SlabSize(64), SlabCount(100_000))?;
//!
//!     // create two lists (one linked and one array list).
//!     // Note that all lists created via macro are interoperable.
//!     let mut list1 = list![1u32, 2u32, 4u32, 5u32];
//!     let mut list2 = array_list!(10, &0u32)?;
//!     list2.push(5)?;
//!     list2.push(2)?;
//!     list2.push(4)?;
//!     list2.push(1)?;
//!
//!     // sort the array list and assert it's equal to the other list
//!     list2.sort()?;
//!     assert!(list_eq!(list1, list2));
//!
//!     // append list2 to list1 and assert the value
//!     list_append!(list1, list2);
//!     assert!(list_eq!(list1, list![1, 2, 4, 5, 1, 2, 4, 5]));
//!
//!     Ok(())
//! }
//!```
//!
//! Arrays
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // instantiate an array of size 100 with the default values of 0usize
//!     let mut arr = array!(100, &0usize)?;
//!
//!     // set the 10th element to 1
//!     arr[10] = 1;
//!
//!     // set each item to the value of the iterator
//!     for i in 0..100 {
//!         arr[i] = i;
//!     }
//!
//!     // assert the values
//!     for i in 0..100 {
//!         assert_eq!(arr[i], i);
//!     }
//!
//!     Ok(())
//! }
//!```
//!
//! Thread pool and Locks
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // create a thread pool with the default settings
//!     let mut tp = thread_pool!()?;
//!     tp.set_on_panic(move |_id,_e| -> Result<(), Error> { Ok(()) })?;
//!
//!     // create a lock initializing it's value to 0
//!     let x = lock!(0)?;
//!     // clone the lock (one for the thread pool, one for the local thread)
//!     let mut x_clone = x.clone();
//!
//!     // execute in the thread pool
//!     let handle = execute!(tp, {
//!         // obtain the write lock for x
//!         let mut x = x_clone.wlock()?;
//!         // set the value to 1
//!         (**x.guard()) = 1;
//!
//!         // return the value of 100
//!         Ok(100)
//!     })?;
//!
//!     // block on the thread and assert the value returned is 100
//!     assert_eq!(block_on!(handle), PoolResult::Ok(100));
//!
//!     // obtain the read lock for x
//!     let x = x.rlock()?;
//!     // assert the value of x is now 1
//!     assert_eq!(**x.guard(), 1);
//!
//!     Ok(())
//! }
//!```
//!
//! Boxed and Sync versions
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! struct MyStruct {
//!     hashtable: Box<dyn Hashtable<u32, String>>,
//! }
//!
//! fn main() -> Result<(), Error> {
//!     // create a slab allocator to be used by our data structures
//!     let slabs = slab_allocator!(SlabSize(128), SlabCount(15_000))?;
//!
//!     {
//!     
//!         // instantiate a hashtable with specified MaxEntries and MaxLoadFactor
//!         let hashtable = hashtable_box!(
//!             MaxEntries(10_000),
//!             MaxLoadFactor(0.85),
//!             Slabs(&slabs)
//!         )?;
//!
//!         let mut s = MyStruct {
//!             hashtable,
//!         };
//!
//!         s.hashtable.insert(&1, &"something".to_string())?;
//!         s.hashtable.insert(&2, &"something else".to_string())?;
//!
//!         assert_eq!(s.hashtable.get(&1)?, Some("something".to_string()));
//!
//!     }
//!
//!
//!     // sync hashtable with default configuration
//!     let mut hashtable2 = hashtable_sync!()?;
//!     hashtable2.insert(&1, &10)?;
//!     hashtable2.insert(&2, &20)?;
//!
//!     // create a thread pool with the default settings
//!     let mut tp = thread_pool!()?;
//!     tp.set_on_panic(move |_id,_e| -> Result<(), Error> { Ok(()) })?;
//!
//!     // put the hashtable into a lock and clone it
//!     let mut lock = lock!(hashtable2)?;
//!     let lock_clone = lock.clone();
//!
//!     // execute a task in the thread pool
//!     let handle = execute!(tp, {
//!         // obtain the write lock to the hashtable and insert an additional value
//!         let mut hashtable2 = lock.wlock()?;
//!         (**hashtable2.guard()).insert(&3, &30)?;
//!         Ok(())
//!     })?;
//!
//!     // block on the task that is in the thread pool
//!     block_on!(handle);
//!
//!     // obtain the read lock and assert all three values are in the hashtable
//!     let hashtable2 = lock_clone.rlock()?;
//!     assert_eq!((**hashtable2.guard()).get(&1)?, Some(10));
//!     assert_eq!((**hashtable2.guard()).get(&2)?, Some(20));
//!     assert_eq!((**hashtable2.guard()).get(&3)?, Some(30));
//!
//!
//!     Ok(())
//! }
//!```
//!
//! Suffix Tree
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // create an array of matches for the suffix tree to populate in tmatch
//!     let mut matches = [Builder::build_match_default(); 10];
//!
//!
//!     // create a suffix tree with two patterns with distinct Regexes and Ids
//!     // Setting a termination length (length to stop processing the text) of 100
//!     // and a max wild card length (the maximum length of any wildcards) of 50
//!     let mut suffix_tree = suffix_tree!(
//!         list![
//!             pattern!(Regex("abc"), Id(0))?,
//!             pattern!(Regex("def.*ok"), Id(1))?
//!         ],
//!         TerminationLength(100),
//!         MaxWildcardLength(50)
//!     )?;
//!
//!     // run the matches and return the number of matches assert that it's
//!     // one for the abc match
//!     let match_count = suffix_tree.tmatch(b"abc", &mut matches)?;
//!     assert_eq!(match_count, 1);
//!
//!     Ok(())
//! }
//!```
//!
//! Queues and Stacks
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//!
//! fn main() -> Result<(), Error> {
//!     // create a stack and a queue both with capacity of 1_000 items and &0
//!     // is the default value used to initialize the array
//!     let mut queue = queue!(1_000, &0)?;
//!     let mut stack = stack!(1_000, &0)?;
//!
//!     // add three items to the queue
//!     queue.enqueue(1)?;
//!     queue.enqueue(2)?;
//!     queue.enqueue(3)?;
//!
//!     // add the same three items to the stack
//!     stack.push(1)?;
//!     stack.push(2)?;
//!     stack.push(3)?;
//!
//!     // dequeue/pop and assert that the queue/stack are appropriately returning the items
//!     assert_eq!(queue.dequeue(), Some(&1));
//!     assert_eq!(stack.pop(), Some(&3));
//!
//!     // dequeue/pop and assert that the queue/stack are appropriately returning the items
//!     assert_eq!(queue.dequeue(), Some(&2));
//!     assert_eq!(stack.pop(), Some(&2));
//!
//!     // peek at both the stack and queue (view value, but do not remove it from the queue)
//!     assert_eq!(queue.peek(), Some(&3));
//!     assert_eq!(stack.peek(), Some(&1));
//!
//!     // dequeue/pop the last item
//!     assert_eq!(queue.dequeue(), Some(&3));
//!     assert_eq!(stack.pop(), Some(&1));
//!
//!     // assert that the queue/stack are empty
//!     assert_eq!(queue.dequeue(), None);
//!     assert_eq!(stack.pop(), None);
//!
//!     Ok(())
//! }
//!```
//!
//! Derive Serializable trait
//!
//!```
//! // import the util/log/err libraries
//! use bmw_util::*;
//! use bmw_log::*;
//! use bmw_err::*;
//! use bmw_derive::*;
//!
//! info!();
//!
//! #[derive(Serializable, Clone, Debug, PartialEq)]
//! struct MyStruct {
//!     id: u128,
//!     name: String,
//!     phone: Option<String>,
//!     age: u8,
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let s = MyStruct {
//!         id: 1234,
//!         name: "Hagrid".to_string(),
//!         phone: None,
//!         age: 54,
//!     };
//!
//!     debug!("my struct = {:?}", s)?;
//!
//!     let mut hashtable = hashtable!()?;
//!
//!     hashtable.insert(&1, &s)?;
//!
//!     let v = hashtable.get(&1)?;
//!     assert_eq!(v, Some(s));
//!
//!     info!("value of record #1 is {:?}", v)?;
//!     
//!     Ok(())
//! }
//!
//!```

mod array;
mod builder;
mod hash;
mod lock;
mod macros;
mod misc;
mod ser;
mod slabs;
mod suffix_tree;
mod threadpool;
mod types;

pub use crate::ser::{deserialize, serialize};
pub use crate::slabs::GLOBAL_SLAB_ALLOCATOR;
pub use crate::types::ConfigOption::*;
pub use crate::types::PatternParam::*;
pub use crate::types::SuffixParam::*;
pub use crate::types::{
	Array, ArrayIterator, ArrayList, BinReader, BinWriter, Builder, ConfigOption, Hashset,
	HashsetConfig, HashsetIterator, Hashtable, HashtableConfig, HashtableIterator, List,
	ListConfig, ListIterator, Lock, LockBox, Match, Pattern, PatternParam, PoolResult, Queue,
	RwLockReadGuardWrapper, RwLockWriteGuardWrapper, Slab, SlabAllocator, SlabAllocatorConfig,
	SlabMut, SlabReader, SlabWriter, SortableList, Stack, SuffixParam, SuffixTree, ThreadPool,
	ThreadPoolConfig, ThreadPoolExecutor, ThreadPoolStopper,
};

pub use crate::lock::lock_box_from_usize;
pub use bmw_ser::{Reader, Serializable, Writer};
