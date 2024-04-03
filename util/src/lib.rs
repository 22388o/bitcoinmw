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

//! # The BMW Utility crate
//!
//! <table style="border: 0px"><tr><td><img style="width: 1100px; height: 200px; background: white;"
//! src="https://raw.githubusercontent.com/cgilliard/bitcoinmw/main/.github/images/elephant-5553135_1280.png"></td><td>
//! The BMW Utility crate defines and implements numerous utilties used in BMW. Generally, these
//! utilities are designed to allocate little to no memory after initialization. In addition to
//! these data structures, there are byte utils, random number generation tools a slab allocator
//! and a thread pool. A locking utility is also included in this library. Like the other
//! libraries, macros are provided and should be used in favor of using the Builder struct. All
//! options that are available for these macros are documented in the Rustdoc. A performance tool
//! is also available for this crate and it's README can be seen on <a
//! href="https://github.com/cgilliard/bitcoinmw/blob/main/etc/util_perf/README.md">Github</a>.
//! </td></tr></table>
//!
//! # Motivation
//!
//! The data structures included in this library are: [`crate::Hashtable`], [`crate::Hashset`],
//! [`crate::List`], [`crate::Array`], [`crate::ArrayList`], [`crate::Stack`], [`crate::Queue`],
//! and [`crate::SearchTrie`].  The advantage of these implementations is that they do not allocate memory
//! on the heap after initialization of the data structure.
//!
//! So, we can create a [`crate::hashtable`],
//! [`crate::List`] or a [`crate::hashset`] and once created, do many operations and
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
//! [util_perf](https://github.com/cgilliard/bitcoinmw/tree/main/etc/util_perf).
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

mod array;
mod builder;
mod constants;
mod hash;
mod lock;
mod macros;
mod misc;
mod rand;
mod search_trie;
mod ser;
mod slabs;
mod test;
mod test_serializable_derive;
mod threadpool;
mod types;

pub use crate::lock::lock_box_from_usize;
pub use crate::misc::*;
pub use crate::rand::*;

#[doc(hidden)]
pub use crate::slabs::GLOBAL_SLAB_ALLOCATOR;

pub use crate::types::{
	Array, ArrayList, Hashset, HashsetIterator, Hashtable, HashtableIterator, List, ListIterator,
	Lock, LockBox, Match, Pattern, PoolResult, Queue, RwLockReadGuardWrapper,
	RwLockWriteGuardWrapper, SearchTrie, Slab, SlabAllocator, SlabAllocatorConfig, SlabMut,
	SlabReader, SlabWriter, SortableList, Stack, ThreadPool, ThreadPoolExecutor, ThreadPoolHandle,
	ThreadPoolStopper, UtilBuilder,
};

#[doc(hidden)]
pub use bmw_conf::ConfigOption::*;
