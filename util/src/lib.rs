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

#[doc(hidden)]
pub use crate::types::SearchParam::*;
pub use crate::types::{
	Array, ArrayList, Hashset, HashsetIterator, Hashtable, HashtableIterator, List, ListIterator,
	Lock, LockBox, Match, Pattern, PoolResult, Queue, RwLockReadGuardWrapper,
	RwLockWriteGuardWrapper, SearchParam, SearchTrie, Slab, SlabAllocator, SlabAllocatorConfig,
	SlabMut, SlabReader, SlabWriter, SortableList, Stack, ThreadPool, ThreadPoolConfig,
	ThreadPoolExecutor, ThreadPoolStopper, UtilBuilder,
};

#[doc(hidden)]
pub use bmw_conf::ConfigOption::*;
