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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::constants::*;
	use crate::misc::DEBUG_INVALID_PATH;
	use crate::types::{HashImpl, HashImplSync, ThreadPoolImpl};
	use bmw_conf::ConfigOption;
	use bmw_deps::dyn_clone::clone_box;
	use bmw_deps::rand;
	use bmw_deps::rand::random;
	use bmw_deps::random_string;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_ser::{deserialize, serialize, Reader, Serializable, Writer};
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
	use std::fmt::Debug;
	use std::fs::{create_dir_all, File};
	use std::io::Write;
	use std::path::PathBuf;
	use std::sync::mpsc::Receiver;
	use std::sync::{Arc, RwLock};

	info!();

	#[test]
	fn test_search_trie_macro() -> Result<(), Error> {
		// build a suffix tree with a wild card
		let mut search_trie = search_trie!(
			vec![
				pattern!(Regex("p1".to_string()), PatternId(0))?,
				pattern!(Regex("^p2".to_string()), PatternId(2))?
			],
			TerminationLength(1_000),
			MaxWildcardLength(100)
		)?;

		// create a matches array for the suffix tree to return matches in
		let mut matches = [tmatch!()?; 10];

		// run the match for the input text b"p1p2". Only "p1" matches this time
		// because p2 is not at the start
		let count = search_trie.tmatch(b"p1p2", &mut matches)?;
		assert_eq!(count, 1);

		// since p2 is at the beginning, both match
		let count = search_trie.tmatch(b"p2p1", &mut matches)?;
		assert_eq!(count, 2);
		Ok(())
	}

	struct TestStruct {
		arr: Array<u32>,
	}

	#[test]
	fn test_array_simple() -> Result<(), Error> {
		let mut arr = UtilBuilder::build_array(10, &0)?;
		for i in 0..10 {
			arr[i] = i as u64;
		}
		for i in 0..10 {
			info!("arr[{}]={}", i, arr[i])?;
			assert_eq!(arr[i], i as u64);
		}

		let mut test = TestStruct {
			arr: UtilBuilder::build_array(40, &0)?,
		};

		for i in 0..40 {
			test.arr[i] = i as u32;
		}

		let test2 = test.arr.clone();

		for i in 0..40 {
			info!("i={}", i)?;
			assert_eq!(test.arr[i], i as u32);
			assert_eq!(test2[i], i as u32);
		}

		assert!(UtilBuilder::build_array::<u8>(0, &0).is_err());

		Ok(())
	}

	#[test]
	fn test_array_iterator() -> Result<(), Error> {
		let mut arr = UtilBuilder::build_array(10, &0)?;
		for i in 0..10 {
			arr[i] = i as u64;
		}

		let mut i = 0;
		for x in arr.iter() {
			assert_eq!(x, &(i as u64));
			i += 1;
		}
		Ok(())
	}

	#[test]
	fn test_array_index_out_of_bounds() -> Result<(), Error> {
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let handle = execute!(tp, {
			let mut x = UtilBuilder::build_array(10, &0)?;
			for i in 0..10 {
				x[i] = i;
			}
			Ok(x[10] = 10)
		})?;

		assert_eq!(
			block_on!(handle),
			PoolResult::Err(err!(
				ErrKind::ThreadPanic,
				"thread pool panic: receiving on a closed channel"
			))
		);

		let handle = execute!(tp, {
			let mut x = UtilBuilder::build_array(10, &0)?;
			for i in 0..10 {
				x[i] = i;
			}
			Ok(())
		})?;

		assert_eq!(block_on!(handle), PoolResult::Ok(()));

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let handle = execute!(tp, {
			let mut x = UtilBuilder::build_array(10, &0)?;
			x[1] = 1;
			Ok(x[10])
		})?;

		assert_eq!(
			block_on!(handle),
			PoolResult::Err(err!(
				ErrKind::ThreadPanic,
				"thread pool panic: receiving on a closed channel"
			))
		);

		Ok(())
	}

	#[test]
	fn test_array_partial_eq() -> Result<(), Error> {
		let mut arr1 = UtilBuilder::build_array(10, &0)?;
		let mut arr2 = UtilBuilder::build_array(11, &0)?;

		for i in 0..10 {
			arr1[i] = 7;
		}

		for i in 0..11 {
			arr2[i] = 7;
		}

		assert_ne!(arr1, arr2);

		let mut arr3 = UtilBuilder::build_array(10, &0)?;
		for i in 0..10 {
			arr3[i] = 8;
		}

		assert_ne!(arr3, arr1);

		let mut arr4 = UtilBuilder::build_array(10, &0)?;
		for i in 0..10 {
			arr4[i] = 7;
		}

		assert_eq!(arr4, arr1);

		let mut arr5 = UtilBuilder::build_array(20, &0)?;
		let mut arr6 = UtilBuilder::build_array(20, &0)?;

		info!("test 128")?;
		for i in 0..20 {
			arr5[i] = 71u128;
		}
		for i in 0..20 {
			arr6[i] = 71u128;
		}

		assert_eq!(arr5, arr6);

		arr5[3] = 100;

		assert_ne!(arr5, arr6);

		Ok(())
	}

	#[test]
	fn test_raw_array_list() -> Result<(), Error> {
		let mut list1 = ArrayList::new(10, &0)?;
		let mut list2 = ArrayList::new(10, &0)?;

		{
			let mut iter = list1.iter();
			assert!(iter.next().is_none());
		}

		assert!(list1 == list2);

		List::push(&mut list1, 1)?;
		List::push(&mut list2, 1)?;

		List::push(&mut list1, 2)?;
		assert!(list1 != list2);

		List::push(&mut list2, 2)?;
		assert!(list1 == list2);

		List::push(&mut list1, 1)?;
		List::push(&mut list2, 3)?;

		assert!(list1 != list2);

		Ok(())
	}

	#[test]
	fn test_array_list() -> Result<(), Error> {
		let mut list1 = UtilBuilder::build_array_list(10, &0)?;
		let mut list2 = UtilBuilder::build_array_list(10, &0)?;

		list1.push(1usize)?;
		list2.push(1usize)?;

		assert!(list_eq!(&list1, &list2));

		list1.push(2)?;
		assert!(!list_eq!(&list1, &list2));

		list2.push(2)?;
		assert!(list_eq!(&list1, &list2));

		list1.push(1)?;
		list2.push(3)?;
		assert!(!list_eq!(&list1, &list2));

		list1.clear()?;
		list2.clear()?;
		assert!(list_eq!(&list1, &list2));

		list1.push(10)?;
		list2.push(10)?;
		assert!(list_eq!(&list1, &list2));

		let mut list3 = UtilBuilder::build_array_list(10, &0)?;

		for i in 0..5 {
			list3.push(i)?;
		}

		let mut list = UtilBuilder::build_array_list(50, &0)?;

		for i in 0..5 {
			list.push(i as u64)?;
		}

		let mut i = 0;
		for x in list.iter() {
			assert_eq!(x, i);
			i += 1;
		}

		assert_eq!(i, 5);
		for x in list.iter_rev() {
			i -= 1;
			assert_eq!(x, i);
		}

		let mut list = UtilBuilder::build_array_list(5, &0)?;
		for _ in 0..5 {
			list.push(1)?;
		}
		assert!(list.push(1).is_err());
		assert!(list.delete_head().is_err());

		assert!(UtilBuilder::build_array_list::<u8>(0, &0).is_err());

		Ok(())
	}

	#[test]
	fn test_as_slice_mut() -> Result<(), Error> {
		let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let mut array = UtilBuilder::build_array(data.len(), &0)?;
		array.as_mut().clone_from_slice(&data);

		assert_eq!(array[3], 3u8);
		assert_eq!(array.as_slice()[4], 4u8);
		Ok(())
	}

	#[test]
	fn test_queue() -> Result<(), Error> {
		let mut queue = UtilBuilder::build_queue(10, &0)?;

		assert_eq!(queue.length(), 0);
		queue.enqueue(1)?;
		queue.enqueue(2)?;
		queue.enqueue(3)?;
		assert_eq!(queue.length(), 3);

		assert_eq!(queue.dequeue(), Some(&1));
		assert_eq!(queue.peek(), Some(&2));
		assert_eq!(queue.peek(), Some(&2));
		assert_eq!(queue.dequeue(), Some(&2));
		assert_eq!(queue.dequeue(), Some(&3));
		assert_eq!(queue.dequeue(), None);
		assert_eq!(queue.peek(), None);

		for i in 0..9 {
			queue.enqueue(i)?;
		}

		for i in 0..9 {
			assert_eq!(queue.dequeue(), Some(&i));
		}

		for i in 0..10 {
			queue.enqueue(i)?;
		}

		for i in 0..10 {
			assert_eq!(queue.dequeue(), Some(&i));
		}

		for i in 0..10 {
			queue.enqueue(i)?;
		}

		assert!(queue.enqueue(1).is_err());

		Ok(())
	}

	#[test]
	fn test_stack() -> Result<(), Error> {
		let mut stack = UtilBuilder::build_stack(10, &0)?;

		assert_eq!(stack.length(), 0);
		stack.push(1)?;
		stack.push(2)?;
		stack.push(3)?;
		assert_eq!(stack.length(), 3);

		assert_eq!(stack.pop(), Some(&3));
		assert_eq!(stack.peek(), Some(&2));
		assert_eq!(stack.peek(), Some(&2));
		assert_eq!(stack.pop(), Some(&2));
		assert_eq!(stack.pop(), Some(&1));
		assert_eq!(stack.pop(), None);
		assert_eq!(stack.peek(), None);

		for i in 0..9 {
			stack.push(i)?;
		}

		for i in (0..9).rev() {
			assert_eq!(stack.pop(), Some(&i));
		}

		for i in 0..10 {
			stack.push(i)?;
		}

		for i in (0..10).rev() {
			assert_eq!(stack.pop(), Some(&i));
		}

		for i in 0..10 {
			stack.push(i)?;
		}

		assert!(stack.push(1).is_err());
		assert_eq!(stack.pop(), Some(&9));

		Ok(())
	}

	#[test]
	fn test_sync_array() -> Result<(), Error> {
		let mut array = UtilBuilder::build_array(10, &0)?;
		array[0] = 1;

		let mut lock = lock!(array)?;
		let lock_clone = lock.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let handle = execute!(tp, {
			let mut array = lock.wlock()?;
			assert_eq!((**array.guard())[0], 1);
			(**array.guard())[0] = 2;
			(**array.guard())[1] = 20;

			Ok(())
		})?;

		block_on!(handle);

		let array_processed = lock_clone.rlock()?;
		assert_eq!((**array_processed.guard())[0], 2);
		assert_eq!((**array_processed.guard())[1], 20);

		Ok(())
	}

	struct TestBoxedQueue {
		queue: Box<dyn Queue<u32>>,
	}

	#[test]
	fn test_queue_boxed() -> Result<(), Error> {
		let queue = UtilBuilder::build_queue_box(10, &0)?;
		let mut test = TestBoxedQueue { queue };
		test.queue.enqueue(1)?;
		Ok(())
	}

	#[test]
	fn test_queue_clone() -> Result<(), Error> {
		let queue = UtilBuilder::build_queue_box(10, &0)?;
		let mut test = TestBoxedQueue { queue };
		test.queue.enqueue(1)?;
		let mut test2 = clone_box(&*test.queue);

		assert_eq!(test.queue.dequeue(), Some(&1));
		assert_eq!(test.queue.dequeue(), None);
		assert_eq!(test2.dequeue(), Some(&1));
		assert_eq!(test2.dequeue(), None);

		Ok(())
	}

	#[test]
	fn test_sort() -> Result<(), Error> {
		let mut list = UtilBuilder::build_array_list(10, &0)?;

		list.push(1)?;
		list.push(3)?;
		list.push(2)?;

		let other_list = list![1, 3, 2];
		info!("list={:?}", list)?;
		assert!(list_eq!(list, other_list));

		list.sort()?;

		let other_list = list![1, 2, 3];
		info!("list={:?}", list)?;
		assert!(list_eq!(list, other_list));

		Ok(())
	}

	#[test]
	fn test_array_of_queues() -> Result<(), Error> {
		let mut queues = array!(10, &queue_box!(10, &0)?)?;

		for i in 0..10 {
			queues[i].enqueue(i)?;
		}

		for i in 0..10 {
			assert_eq!(queues[i].dequeue(), Some(&i));
		}

		for i in 0..10 {
			assert_eq!(queues[i].dequeue(), None);
		}

		Ok(())
	}

	#[test]
	fn test_string_array() -> Result<(), Error> {
		let mut arr: Array<String> = Array::new(100, &"".to_string())?;
		for i in 0..100 {
			arr[i] = "".to_string();
		}
		info!("array = {:?}", arr)?;

		let mut vec: Vec<String> = vec![];
		vec.resize(100, "".to_string());

		let charset = "0123456789abcdefghijklmopqrstuvwxyz";
		for _ in 0..10_000 {
			let rand: usize = random();
			let rstring = random_string::generate(2_000, charset);
			vec[rand % 100] = rstring.clone();
			arr[rand % 100] = rstring.clone();
		}

		for i in 0..100 {
			assert_eq!(vec[i], arr[i]);
		}

		Ok(())
	}

	#[test]
	fn test_builder() -> Result<(), Error> {
		let mut arrlist = UtilBuilder::build_array_list_box(10, &0)?;
		arrlist.push(0)?;
		let mut i = 0;
		for x in arrlist.iter() {
			assert_eq!(x, 0);
			i += 1;
		}
		assert_eq!(i, 1);

		let mut list = UtilBuilder::build_list_sync_box(vec![])?;
		list.push(0)?;
		assert_eq!(list.size(), 1);

		let nmatch = UtilBuilder::build_match(vec![Start(0), End(1), MatchId(2)])?;
		assert_eq!(nmatch.start(), 0);
		assert_eq!(nmatch.end(), 1);
		assert_eq!(nmatch.id(), 2);

		assert!(!UtilBuilder::build_sync_slabs().is_init());

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
		search_trie: Box<dyn SearchTrie + Send + Sync>,
	}

	#[test]
	fn test_builder_sync() -> Result<(), Error> {
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_, _| Ok(()))?;
		tp.start()?;

		let test_obj = TestObj {
			array: UtilBuilder::build_array(10, &0)?,
			array_list: UtilBuilder::build_array_list_sync_box(10, &0)?,
			queue: UtilBuilder::build_queue_sync_box(10, &0)?,
			stack: UtilBuilder::build_stack_sync_box(10, &0)?,
			hashtable: UtilBuilder::build_hashtable_sync_box(vec![
				GlobalSlabAllocator(false),
				SlabSize(1_000),
				SlabCount(300),
			])?,
			hashset: UtilBuilder::build_hashset_sync_box(vec![
				GlobalSlabAllocator(false),
				SlabSize(1_000),
				SlabCount(300),
			])?,
			list: UtilBuilder::build_list_sync_box(vec![
				GlobalSlabAllocator(false),
				SlabSize(1_000),
				SlabCount(300),
			])?,
			search_trie: UtilBuilder::build_search_trie_box(
				vec![pattern!(Regex("abc".to_string()), PatternId(0))?],
				100,
				50,
			)?,
		};
		let array_list_sync = UtilBuilder::build_array_list_sync(10, &0)?;
		let mut array_list_sync = lock_box!(array_list_sync)?;
		let queue_sync = UtilBuilder::build_queue_sync(10, &0)?;
		let mut queue_sync = lock_box!(queue_sync)?;
		let stack_sync = UtilBuilder::build_stack_sync(10, &0)?;
		let mut stack_sync = lock_box!(stack_sync)?;

		let mut stack_box = UtilBuilder::build_stack_box(10, &0)?;
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
				let mut matches = [tmatch!()?; 10];
				(**guard).search_trie.tmatch(b"test", &mut matches)?;
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

	#[test]
	fn test_static_hashtable() -> Result<(), Error> {
		let free_count1;

		{
			let mut hashtable = UtilBuilder::build_hashtable(vec![MaxEntries(100)])?;
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
		let mut hashtable = UtilBuilder::build_hashtable(vec![])?;
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
		let mut hashtable = UtilBuilder::build_hashtable(vec![])?;
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
		let mut hashtable = UtilBuilder::build_hashtable(vec![])?;
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
		let mut hashtable = UtilBuilder::build_hashtable(vec![])?;
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
			let mut hashtable = UtilBuilder::build_hashtable(vec![])?;
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
		let mut hashset = UtilBuilder::build_hashset::<i32>(vec![])?;
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
		let mut list = UtilBuilder::build_list(vec![])?;
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

		let mut list = UtilBuilder::build_list(vec![])?;
		if false {
			list.push(0u8)?;
		}
		let mut count = 0;
		for _x in list.iter() {
			count += 1;
		}

		assert_eq!(count, 0);

		Ok(())
	}

	#[test]
	fn test_small_slabs() -> Result<(), Error> {
		let mut table = UtilBuilder::build_hashtable(vec![
			GlobalSlabAllocator(false),
			SlabSize(100),
			SlabCount(100),
			MaxEntries(100),
		])?;

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
		let mut h = UtilBuilder::build_hashtable(vec![
			MaxEntries(1),
			SlabCount(1),
			SlabSize(12),
			GlobalSlabAllocator(false),
		])?;

		info!("insert 1")?;
		assert!(h.insert(&2u64, &6u64).is_err());
		info!("insert 2")?;
		let mut h = UtilBuilder::build_hashtable(vec![
			MaxEntries(1),
			SlabCount(1),
			SlabSize(12),
			GlobalSlabAllocator(false),
		])?;
		h.insert(&2000u32, &1000u32)?;
		Ok(())
	}

	#[test]
	fn test_sync_hashtable() -> Result<(), Error> {
		let h = UtilBuilder::build_hashtable_sync(vec![
			MaxEntries(1024),
			SlabSize(1024),
			SlabCount(1024),
			GlobalSlabAllocator(false),
		])?;
		assert!(h.slabs().is_ok());
		let mut h = lock!(h)?;
		let mut h_clone = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		{
			let h2 = h_clone.rlock()?;
			assert_eq!((**h2.guard()).get(&2u64)?, None);
			assert_eq!((**h2.guard()).size(), 0);
			assert_eq!(
				(**h2.guard()).max_load_factor(),
				HASH_DEFAULT_MAX_LOAD_FACTOR
			);
			assert_eq!((**h2.guard()).max_entries(), 1024);
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
		let h = UtilBuilder::build_hashset_sync(vec![
			MaxEntries(1024),
			SlabSize(1024),
			SlabCount(1024),
			GlobalSlabAllocator(false),
		])?;

		let mut h = lock!(h)?;
		let h_clone = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

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
		let h = UtilBuilder::build_list_sync(vec![
			SlabSize(1024),
			SlabCount(1024),
			GlobalSlabAllocator(false),
		])?;

		let mut h = lock!(h)?;
		let h_clone = h.clone();
		let mut h_clone2 = h.clone();
		let mut h_clone3 = h.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

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

		let h2 = UtilBuilder::build_list_sync(vec![
			SlabSize(1024),
			SlabCount(1024),
			GlobalSlabAllocator(false),
		])?;
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

		let x: HashImplSync<u32> = HashImplSync::new(vec![
			IsList(true),
			GlobalSlabAllocator(false),
			SlabSize(128),
			SlabCount(128),
		])?;

		let mut x2: HashImplSync<u32> = HashImplSync::new(vec![
			IsList(true),
			GlobalSlabAllocator(false),
			SlabSize(128),
			SlabCount(128),
		])?;

		x2.push(1)?;
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
		let mut hashset = UtilBuilder::build_hashset_sync(vec![
			GlobalSlabAllocator(false),
			SlabSize(128),
			SlabCount(128),
		])?;

		hashset.insert(&1)?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&1)?);
		assert!(!hashset.contains(&2)?);
		assert_eq!(hashset.remove(&1)?, true);
		assert_eq!(hashset.remove(&1)?, false);
		assert_eq!(hashset.size(), 0);
		assert_eq!(hashset.max_load_factor(), HASH_DEFAULT_MAX_LOAD_FACTOR);
		assert_eq!(hashset.max_entries(), HASH_DEFAULT_MAX_ENTRIES);

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
		let h = UtilBuilder::build_hashtable_box(vec![])?;
		let mut thtb = TestHashtableBox { h };

		let x = 1;
		thtb.h.insert(&x, &2)?;
		assert_eq!(thtb.h.get(&x)?, Some(2));

		Ok(())
	}

	#[test]
	fn test_list_boxed() -> Result<(), Error> {
		let mut list1 = UtilBuilder::build_list_box(vec![])?;
		list1.push(1)?;
		list1.push(2)?;

		let mut list2 = UtilBuilder::build_list(vec![])?;
		list2.push(1)?;
		list2.push(2)?;

		assert!(list_eq!(list1, list2));

		let list3 = list![1, 2, 1, 2];
		list_append!(list1, list2);
		assert!(list_eq!(list1, list3));

		let mut list4 = UtilBuilder::build_array_list(100, &0)?;
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
		hashtable.insert(&1, &10)?;
		hashtable.insert(&2, &20)?;
		hashtable.insert(&1, &10)?;
		info!("hashtable={:?}", hashtable)?;
		assert!(hashtable.slabs().is_ok());
		Ok(())
	}

	#[test]
	fn test_hash_impl_internal_errors() -> Result<(), Error> {
		let mut hash_impl: HashImpl<u32> = HashImpl::new(vec![IsHashtable(true)])?;
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
		let mut hash_impl: HashImpl<u32> = HashImpl::new(vec![IsList(true)])?;
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
		let mut hash_impl: HashImpl<u32> = HashImpl::new(vec![IsHashtable(true)])?;
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
		let mut hash_impl: HashImpl<SerErr> = HashImpl::new(vec![IsHashtable(true)])?;
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
		let mut hash_impl: HashImpl<SerErr> = HashImpl::new(vec![IsList(true)])?;
		hash_impl.push(SerErr { exp: 100, empty: 0 })?;
		let mut iter: Box<dyn Iterator<Item = SerErr>> = List::iter(&hash_impl);
		assert_eq!(iter.next(), None);

		let mut hash_impl: HashImpl<SerErr> = HashImpl::new(vec![IsList(true)])?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;
		{
			let mut iter: Box<dyn Iterator<Item = SerErr>> = List::iter(&hash_impl);
			assert_eq!(iter.next(), Some(SerErr { exp: 99, empty: 0 }));
		}

		Ok(())
	}

	#[test]
	fn test_hash_impl_error_conditions() -> Result<(), Error> {
		let hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
			SlabSize(100_000),
			SlabCount(1),
			GlobalSlabAllocator(false),
		]);
		assert!(hashtable.is_err());

		let hash_impl: Result<HashImpl<u32>, Error> =
			HashImpl::new(vec![IsList(true), DebugLargeSlabCount(true)]);
		assert!(hash_impl.is_err());

		let hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![MaxEntries(0)]);
		assert!(hashtable.is_err());

		let hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![MaxLoadFactor(2.0)]);
		assert!(hashtable.is_err());

		let hashset = UtilBuilder::build_hashset::<u32>(vec![MaxEntries(0)]);
		assert!(hashset.is_err());

		let hashset = UtilBuilder::build_hashset::<u32>(vec![MaxLoadFactor(2.0)]);
		assert!(hashset.is_err());

		let hashset = UtilBuilder::build_hashset::<u32>(vec![
			GlobalSlabAllocator(false),
			SlabSize(8),
			SlabCount(1),
			MaxEntries(10_000_000),
		]);
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
		let mut hashset = UtilBuilder::build_hashset::<u128>(vec![
			SlabSize(12),
			SlabCount(1),
			GlobalSlabAllocator(false),
		])?;
		let e = hashset.insert(&1).unwrap_err().kind();
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = hashset.slabs()?;
		assert_eq!(rlock!(slabs.unwrap()).free_count()?, 1);

		let hashset = UtilBuilder::build_hashset_sync::<u8>(vec![
			SlabSize(12),
			SlabCount(1),
			GlobalSlabAllocator(false),
		])?;

		assert!(hashset.slabs()?.is_some());

		Ok(())
	}

	#[test]
	fn test_hashtable_value_write_error() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u128, u128>(vec![
			SlabSize(30),
			SlabCount(1),
			GlobalSlabAllocator(false),
		])?;
		let e = hashtable.insert(&1, &2).unwrap_err().kind();
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = hashtable.slabs()?;
		assert_eq!(rlock!(slabs.unwrap()).free_count()?, 1);
		Ok(())
	}

	#[test]
	fn test_hashtable_value_write_error_multi_slab() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u128, u128>(vec![
			SlabSize(16),
			SlabCount(2),
			GlobalSlabAllocator(false),
		])?;
		let e = hashtable.insert(&1, &2).unwrap_err().kind();
		info!("e={}", e)?;
		let m = matches!(e, ErrorKind::CapacityExceeded(_));
		assert!(m);
		let slabs = hashtable.slabs()?;
		assert_eq!(rlock!(slabs.unwrap()).free_count()?, 2);
		Ok(())
	}

	#[test]
	fn test_hashtable_writer_full_error() -> Result<(), Error> {
		let slabs_ext;
		{
			let mut hashset = UtilBuilder::build_hashset::<u128>(vec![
				SlabSize(25),
				SlabCount(1),
				GlobalSlabAllocator(false),
			])?;
			hashset.insert(&2)?;
			let e = hashset.insert(&1).unwrap_err().kind();
			let m = matches!(e, ErrorKind::CapacityExceeded(_));
			assert!(m);
			let slabs = hashset.slabs()?;
			slabs_ext = Some(slabs.clone());
			assert_eq!(rlock!(slabs.unwrap()).free_count()?, 0);
		}

		assert_eq!(rlock!(slabs_ext.unwrap().unwrap()).free_count()?, 1);
		Ok(())
	}

	#[test]
	fn test_hashset_load_factor() -> Result<(), Error> {
		let mut hashset = UtilBuilder::build_hashset::<u128>(vec![
			SlabSize(128),
			SlabCount(100),
			GlobalSlabAllocator(false),
			MaxEntries(10),
			MaxLoadFactor(1.0),
		])?;

		for i in 0..10 {
			hashset.insert(&(i as u128))?;
		}

		assert!(hashset.insert(&10u128).is_err());

		Ok(())
	}

	#[test]
	fn test_remove_oldest() -> Result<(), Error> {
		{
			let mut hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
				GlobalSlabAllocator(false),
				SlabSize(25),
				SlabCount(10),
			])?;
			hashtable.insert(&1, &4)?;
			hashtable.insert(&2, &5)?;
			hashtable.insert(&3, &6)?;
			{
				let slabs = hashtable.slabs().unwrap().unwrap();
				assert_eq!(rlock!(slabs).free_count()?, 7);
			}

			assert_eq!(hashtable.get(&1), Ok(Some(4)));
			assert_eq!(hashtable.get(&2), Ok(Some(5)));
			assert_eq!(hashtable.get(&3), Ok(Some(6)));
			hashtable.remove_oldest()?;
			{
				let slabs = hashtable.slabs().unwrap().unwrap();
				assert_eq!(rlock!(slabs).free_count()?, 8);
			}
			assert_eq!(hashtable.get(&1), Ok(None));
			assert_eq!(hashtable.get(&2), Ok(Some(5)));
			assert_eq!(hashtable.get(&3), Ok(Some(6)));
		}
		Ok(())
	}

	#[test]
	fn test_bring_to_front_mid_tail() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
			SlabSize(25),
			SlabCount(10),
			GlobalSlabAllocator(false),
		])?;
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
		let mut hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
			SlabSize(25),
			SlabCount(10),
			GlobalSlabAllocator(false),
		])?;

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
		let mut hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
			SlabSize(25),
			SlabCount(1_000),
			GlobalSlabAllocator(false),
		])?;

		let mut data2 = [0u8; BUFFER_SIZE];
		let data = [8u8; BUFFER_SIZE];
		hashtable.raw_write(&0, 0, &data, BUFFER_SIZE)?;
		hashtable.raw_read(&0, 0, &mut data2)?;
		assert_eq!(data, data2);

		let data = [10u8; BUFFER_SIZE];
		hashtable.raw_write(&7, 383, &data, BUFFER_SIZE)?;
		hashtable.raw_read(&7, 383, &mut data2)?;
		assert_eq!(data, data2);
		assert!(!hashtable.raw_read(&9, 384, &mut data2)?);

		Ok(())
	}

	#[test]
	fn test_hashtable_sync_raw() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable_sync::<u32, u32>(vec![])?;

		let mut data2 = [0u8; BUFFER_SIZE];
		let data = [8u8; BUFFER_SIZE];
		hashtable.raw_write(&0, 0, &data, BUFFER_SIZE)?;
		assert!(hashtable.raw_read(&0, 0, &mut data2)?);
		assert_eq!(data, data2);

		let data = [10u8; BUFFER_SIZE];
		hashtable.raw_write(&7, 383, &data, BUFFER_SIZE)?;
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
		let mut hashtable = UtilBuilder::build_hashtable::<u32, u32>(vec![
			SlabSize(25),
			SlabCount(1_000),
			GlobalSlabAllocator(false),
		])?;
		let mut data2 = [0u8; BUFFER_SIZE];
		let empty = [4u8; BUFFER_SIZE];
		let data = [10u8; BUFFER_SIZE];

		info!("raw_write at 1383")?;
		hashtable.raw_write(&7, 1383, &data, BUFFER_SIZE)?;
		info!("raw write at 0")?;
		hashtable.raw_write(&7, 0, &empty, BUFFER_SIZE)?;
		info!("raw read at 1383")?;
		hashtable.raw_read(&7, 1383, &mut data2)?;
		assert_eq!(data, data2);
		Ok(())
	}

	#[test]
	fn test_multi_remove_oldest() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u32, String>(vec![
			SlabSize(25),
			SlabCount(3),
			GlobalSlabAllocator(false),
		])?;
		hashtable.insert(&1, &"4".to_string())?;
		hashtable.insert(&2, &"5".to_string())?;
		hashtable.insert(&3, &"6".to_string())?;
		{
			let free_count = rlock!(hashtable.slabs().unwrap().unwrap()).free_count()?;
			assert_eq!(free_count, 0);
		}

		hashtable.remove_oldest()?;
		{
			let free_count = rlock!(hashtable.slabs().unwrap().unwrap()).free_count()?;
			assert_eq!(free_count, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 2);
		}
		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.insert(&1, &"0123456789012345678901234".to_string())?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.insert(&1, &"0123456789012345678901234567890".to_string())?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.insert(&1, &"4".to_string())?;
		hashtable.insert(&2, &"0123456789012345678901234".to_string())?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 1);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.insert(&2, &"0123456789012345678901234".to_string())?;
		hashtable.insert(&1, &"4".to_string())?;

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 2);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		hashtable.insert(&1, &"0123456789012345678901234567890".to_string())?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 3);
		}

		Ok(())
	}

	#[test]
	fn test_insert_with_big_small_big_rem_all() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u32, String>(vec![
			SlabSize(25),
			SlabCount(64),
			GlobalSlabAllocator(false),
		])?;

		let mut big_string = "".to_string();
		let string10 = "0123456789".to_string();
		for _ in 0..144 {
			big_string = format!("{}{}", big_string, string10);
		}
		hashtable.insert(&1, &big_string)?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 64);
		}

		hashtable.insert(&2, &"ok".to_string())?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 63);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 64);
		}

		hashtable.insert(&1, &big_string)?;

		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 0);
		}

		hashtable.remove_oldest()?;
		{
			let slabs = hashtable.slabs().unwrap().unwrap();
			assert_eq!(rlock!(slabs).free_count()?, 64);
		}
		Ok(())
	}

	#[test]
	fn test_raw() -> Result<(), Error> {
		let mut hashtable = UtilBuilder::build_hashtable::<u32, String>(vec![
			SlabSize(250),
			SlabCount(3),
			GlobalSlabAllocator(false),
		])?;

		let bytes = [3u8; BUFFER_SIZE];
		hashtable.raw_write(&1, 0, &bytes, BUFFER_SIZE)?;
		hashtable.raw_write(&1, 4, &bytes, BUFFER_SIZE)?;
		hashtable.remove_oldest()?;

		Ok(())
	}

	#[test]
	fn test_locks() -> Result<(), Error> {
		let mut lock = UtilBuilder::build_lock(1)?;
		let mut lock2 = lock.clone();
		{
			let x = lock.rlock()?;
			println!("x={}", *x.guard());
		}
		{
			let mut y = lock.wlock()?;
			**(y.guard()) = 2;

			assert!(lock2.wlock().is_err());
		}

		{
			let mut z = lock.wlock()?;
			assert_eq!(**(z.guard()), 2);
		}

		Ok(())
	}

	#[test]
	fn test_read_deadlock() -> Result<(), Error> {
		let mut lock = UtilBuilder::build_lock(1)?;
		let lock2 = lock.clone();
		{
			let x = lock.rlock()?;
			println!("x={}", *x.guard());
		}
		{
			let mut y = lock.wlock()?;
			**(y.guard()) = 2;

			assert!(lock2.rlock().is_err());
		}

		{
			let mut z = lock.wlock()?;
			assert_eq!(**(z.guard()), 2);
		}

		let mut lock = UtilBuilder::build_lock_box(1)?;
		let lock2 = lock.clone();
		{
			let x = lock.rlock_ignore_poison()?;
			println!("x={}", *x.guard());
			assert!(lock.rlock_ignore_poison().is_err());
		}
		{
			let mut y = lock.wlock()?;
			**(y.guard()) = 2;

			assert!(lock2.rlock_ignore_poison().is_err());
			assert!(lock2.rlock_ignore_poison().is_err());
		}

		{
			let mut z = lock.wlock()?;
			assert_eq!(**(z.guard()), 2);
		}

		Ok(())
	}

	#[test]
	fn test_lock_threads() -> Result<(), Error> {
		let mut lock = UtilBuilder::build_lock(1)?;
		let mut lock_clone = lock.clone();

		spawn(move || -> Result<(), Error> {
			let mut x = lock.wlock()?;
			sleep(Duration::from_millis(3000));
			**(x.guard()) = 2;
			Ok(())
		});

		sleep(Duration::from_millis(1000));
		let mut x = lock_clone.wlock()?;
		assert_eq!(**(x.guard()), 2);

		Ok(())
	}

	#[test]
	fn test_lock_macro() -> Result<(), Error> {
		let mut lock = lock!(1)?;
		let lock_clone = lock.clone();
		println!("lock={:?}", lock);

		spawn(move || -> Result<(), Error> {
			let mut x = lock.wlock()?;
			assert_eq!(**(x.guard()), 1);
			sleep(Duration::from_millis(3000));
			**(x.guard()) = 2;
			Ok(())
		});

		sleep(Duration::from_millis(1000));
		let x = lock_clone.rlock()?;
		assert_eq!(**(x.guard()), 2);

		Ok(())
	}

	struct TestLockBox<T> {
		lock_box: Box<dyn LockBox<T>>,
	}

	#[test]
	fn test_lock_box() -> Result<(), Error> {
		let lock_box = lock_box!(1u32)?;
		let mut lock_box2 = lock_box.clone();
		let mut tlb = TestLockBox { lock_box };
		{
			let mut tlb = tlb.lock_box.wlock()?;
			(**tlb.guard()) = 2u32;
		}

		{
			let tlb = tlb.lock_box.rlock()?;
			assert_eq!((**tlb.guard()), 2u32);
		}

		{
			let mut tlb = lock_box2.wlock()?;
			assert_eq!((**tlb.guard()), 2u32);
			(**tlb.guard()) = 3u32;
		}

		{
			let clone = tlb.lock_box.clone();
			let tlb2 = tlb.lock_box.rlock_ignore_poison()?;
			assert_eq!((**tlb2.guard()), 3u32);
			assert!(clone.rlock_ignore_poison().is_err());
		}

		{
			let mut tlb = lock_box2.wlock_ignore_poison()?;
			assert_eq!((**tlb.guard()), 3u32);
		}

		Ok(())
	}

	#[test]
	fn test_rw_guards() -> Result<(), Error> {
		{
			let lock = Arc::new(RwLock::new(1));
			let guard = lock.read().unwrap();
			let x = RwLockReadGuardWrapper {
				guard,
				id: 0,
				debug_err: true,
			};
			let guard = x.guard();
			assert_eq!(**guard, 1);
		}
		{
			let lock = Arc::new(RwLock::new(1));
			let guard = lock.write().unwrap();
			let mut x = RwLockWriteGuardWrapper {
				guard,
				id: 0,
				debug_err: true,
			};
			let guard = x.guard();
			assert_eq!(**guard, 1);
		}
		Ok(())
	}

	#[test]
	fn test_ignore_poison_scenarios() -> Result<(), Error> {
		let mut x = crate::types::LockImpl::new(1u32);
		let x_clone = crate::types::Lock::clone(&x);

		spawn(move || -> Result<(), Error> {
			let _v = x_clone.t.write();
			let p: Option<usize> = None;
			p.unwrap();
			Ok(())
		});

		sleep(Duration::from_millis(5_000));
		x.rlock_ignore_poison()?;
		x.wlock_ignore_poison()?;
		Ok(())
	}

	#[test]
	fn test_to_usize() -> Result<(), Error> {
		let v = {
			let x: Box<dyn LockBox<u32>> = lock_box!(100u32)?;
			let v = x.danger_to_usize();
			v
		};

		let arc = Arc::new(unsafe { Arc::from_raw(v as *mut RwLock<u32>) });
		let v = arc.read().unwrap();
		info!("ptr_ret = {}", *v)?;
		assert_eq!(*v, 100);

		let mut lbox = lock_box!(1_100)?;
		let v = lbox.danger_to_usize();
		let lbox_new: Box<dyn LockBox<u32>> = crate::lock_box_from_usize(v);
		(**(lbox.wlock()?.guard())) = 1_200;
		assert_eq!((**(lbox_new.rlock()?.guard())), 1_200);

		Ok(())
	}

	struct TestHashsetHolder {
		h1: Option<Box<dyn Hashset<u32>>>,
		h2: Option<Box<dyn LockBox<Box<dyn Hashset<u32> + Send + Sync>>>>,
	}

	#[test]
	fn test_hashset_macros() -> Result<(), Error> {
		{
			let mut hashset = hashset!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());
		}

		{
			let mut hashset =
				hashset_box!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());

			let mut thh = TestHashsetHolder {
				h2: None,
				h1: Some(hashset),
			};

			{
				let hashset = thh.h1.as_mut().unwrap();
				assert_eq!(hashset.size(), 1);
			}
		}

		{
			let mut hashset =
				hashset_sync!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());
		}

		{
			let hashset =
				hashset_sync_box!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
			let mut hashset = lock_box!(hashset)?;

			{
				let mut hashset = hashset.wlock()?;
				(**hashset.guard()).insert(&1)?;
				assert!((**hashset.guard()).contains(&1)?);
				assert!(!(**hashset.guard()).contains(&2)?);
				assert!((**hashset.guard()).insert(&2).is_err());
			}

			let mut thh = TestHashsetHolder {
				h1: None,
				h2: Some(hashset),
			};

			{
				let mut hashset = thh.h2.as_mut().unwrap().wlock()?;
				assert_eq!((**hashset.guard()).size(), 1);
			}
		}

		Ok(())
	}

	#[test]
	fn test_slabs_in_hashtable_macro() -> Result<(), Error> {
		let mut hashtable = hashtable!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
		hashtable.insert(&1, &2)?;

		assert_eq!(hashtable.get(&1).unwrap(), Some(2));

		assert!(hashtable.insert(&2, &3).is_err());

		Ok(())
	}

	#[test]
	fn test_hashtable_box_macro() -> Result<(), Error> {
		let mut hashtable =
			hashtable_box!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
		hashtable.insert(&1, &2)?;

		assert_eq!(hashtable.get(&1).unwrap(), Some(2));

		assert!(hashtable.insert(&2, &3).is_err());

		Ok(())
	}

	struct TestHashtableSyncBox {
		h: Box<dyn LockBox<Box<dyn Hashtable<u32, u32> + Send + Sync>>>,
	}

	#[test]
	fn test_hashtable_sync_box_macro() -> Result<(), Error> {
		let hashtable =
			hashtable_sync_box!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
		let mut hashtable = lock_box!(hashtable)?;

		{
			let mut hashtable = hashtable.wlock()?;
			(**hashtable.guard()).insert(&1, &2)?;
			assert_eq!((**hashtable.guard()).get(&1).unwrap(), Some(2));
			assert!((**hashtable.guard()).insert(&2, &3).is_err());
		}

		let thsb = TestHashtableSyncBox { h: hashtable };

		{
			let h = thsb.h.rlock()?;
			assert!((**h.guard()).get(&1)?.is_some());
			assert!((**h.guard()).get(&2)?.is_none());
		}

		Ok(())
	}

	#[test]
	fn test_hashtable_sync_macro() -> Result<(), Error> {
		let hashtable =
			hashtable_sync_box!(SlabSize(128), SlabCount(1), GlobalSlabAllocator(false))?;
		let mut hashtable = lock!(hashtable)?;

		{
			let mut hashtable = hashtable.wlock()?;
			(**hashtable.guard()).insert(&1, &2)?;
			assert_eq!((**hashtable.guard()).get(&1).unwrap(), Some(2));
			assert!((**hashtable.guard()).insert(&2, &3).is_err());
		}

		Ok(())
	}

	#[test]
	fn test_slab_allocator_macro() -> Result<(), bmw_err::Error> {
		let mut slabs = slab_allocator!()?;
		let mut slabs2 = slab_allocator!(SlabSize(128), SlabCount(1))?;
		let slab = slabs.allocate()?;
		assert_eq!(
			slab.get().len(),
			bmw_util::SlabAllocatorConfig::default().slab_size
		);
		let slab = slabs2.allocate()?;
		assert_eq!(slab.get().len(), 128);
		assert!(slabs2.allocate().is_err());
		assert!(slabs.allocate().is_ok());

		assert!(slab_allocator!(SlabSize(128), SlabSize(64)).is_err());
		assert!(slab_allocator!(SlabCount(128), SlabCount(64)).is_err());
		assert!(slab_allocator!(MaxEntries(128)).is_err());
		assert!(slab_allocator!(MaxLoadFactor(128.0)).is_err());
		Ok(())
	}

	#[test]
	fn test_hashtable_macro() -> Result<(), bmw_err::Error> {
		let mut hashtable = hashtable!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_sync!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_sync!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100),
			GlobalSlabAllocator(false),
		)?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_box!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_box!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_sync_box!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_sync_box!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100),
			GlobalSlabAllocator(false)
		)?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		Ok(())
	}

	#[test]
	fn test_hashset_macro() -> Result<(), bmw_err::Error> {
		let mut hashset = hashset!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_sync!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_sync!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100),
			GlobalSlabAllocator(false)
		)?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_box!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_box!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_sync_box!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_sync_box!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100),
			GlobalSlabAllocator(false),
		)?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		Ok(())
	}

	#[test]
	fn test_list_macro() -> Result<(), bmw_err::Error> {
		let mut list1 = list!['1', '2', '3'];
		list_append!(list1, list!['a', 'b', 'c']);
		let list2 = list!['1', '2', '3', 'a', 'b', 'c'];
		assert!(list_eq!(list1, list2));
		let list2 = list!['a', 'b', 'c', '1', '2'];
		assert!(!list_eq!(list1, list2));

		let list3 = list![1, 2, 3, 4, 5];
		info!("list={:?}", list3)?;

		let list4 = list_box![1, 2, 3, 4, 5];
		let mut list5 = list_sync!();
		let mut list6 = list_sync_box!();
		list_append!(list5, list4);
		list_append!(list6, list4);
		assert!(list_eq!(list4, list3));
		assert!(list_eq!(list4, list5));
		assert!(list_eq!(list4, list6));

		Ok(())
	}

	#[test]
	fn test_thread_pool_macro() -> Result<(), bmw_err::Error> {
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let resp = execute!(tp, {
			info!("in thread pool")?;
			Ok(123)
		})?;
		assert_eq!(block_on!(resp), PoolResult::Ok(123));

		let mut tp = thread_pool!(MinSize(3))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;
		let resp: Receiver<PoolResult<u32, Error>> = execute!(tp, {
			info!("thread pool2")?;
			Err(err!(ErrKind::Test, "test err"))
		})?;
		assert_eq!(
			block_on!(resp),
			PoolResult::Err(err!(ErrKind::Test, "test err"))
		);

		let mut tp = thread_pool!(MinSize(3))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let resp: Receiver<PoolResult<u32, Error>> = execute!(tp, {
			info!("thread pool panic")?;
			let x: Option<u32> = None;
			Ok(x.unwrap())
		})?;
		assert_eq!(
			block_on!(resp),
			PoolResult::Err(err!(
				ErrKind::ThreadPanic,
				"thread pool panic: receiving on a closed channel"
			))
		);
		Ok(())
	}

	#[test]
	fn test_thread_pool_options() -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(4), MaxSize(5), SyncChannelSize(10))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		assert_eq!(tp.size()?, 4);
		sleep(Duration::from_millis(2_000));
		let resp = execute!(tp, {
			info!("thread pool")?;
			Ok(0)
		})?;
		assert_eq!(block_on!(resp), PoolResult::Ok(0));
		assert_eq!(tp.size()?, 4);

		for _ in 0..10 {
			execute!(tp, {
				info!("thread pool")?;
				sleep(Duration::from_millis(5_000));
				Ok(0)
			})?;
		}
		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 5);
		Ok(())
	}

	#[test]
	fn test_list_eq() -> Result<(), Error> {
		let list1 = list![1, 2, 3];
		let eq = list_eq!(list1, list![1, 2, 3]);
		let mut list2 = list![4, 5, 6];
		list_append!(list2, list![5, 5, 5]);
		assert!(eq);
		assert!(list_eq!(list2, list![4, 5, 6, 5, 5, 5]));
		list2.sort_unstable()?;
		assert!(list_eq!(list2, list![4, 5, 5, 5, 5, 6]));
		assert!(!list_eq!(list2, list![1, 2, 3]));
		Ok(())
	}

	#[test]
	fn test_array_macro() -> Result<(), Error> {
		let mut array = array!(10, &0)?;
		array[1] = 2;
		assert_eq!(array[1], 2);

		let mut a = array_list_box!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut a = array_list_sync!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut a = array_list_sync!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut q = queue!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_sync!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_box!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_sync_box!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut s = stack!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_box!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_sync!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_sync_box!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		Ok(())
	}

	#[test]
	fn test_pattern_suffix_macros() -> Result<(), Error> {
		// create matches array
		let mut matches = [tmatch!()?; 10];

		// test pattern
		let pattern = pattern!(
			Regex("abc".to_string()),
			PatternId(0),
			IsCaseSensitive(true)
		)?;
		assert_eq!(
			pattern,
			UtilBuilder::build_pattern(vec![
				Regex("abc".to_string()),
				IsCaseSensitive(true),
				IsTerminationPattern(false),
				PatternId(0),
				IsMultiLine(true)
			])?,
		);

		// test suffix tree
		let mut search_trie = search_trie!(
			vec![
				pattern!(Regex("abc".to_string()), PatternId(0))?,
				pattern!(Regex("def".to_string()), PatternId(1))?
			],
			TerminationLength(100),
			MaxWildcardLength(50)
		)?;
		let match_count = search_trie.tmatch(b"abc", &mut matches)?;
		assert_eq!(match_count, 1);
		Ok(())
	}

	#[test]
	fn test_simple_search_trie() -> Result<(), Error> {
		// create matches array
		let mut matches = [tmatch!()?; 10];

		// create a suffix tree
		let mut search_trie = search_trie!(vec![
			pattern!(Regex("aaa".to_string()), PatternId(0))?,
			pattern!(Regex("bbb".to_string()), PatternId(1))?
		],)?;

		// match
		let match_count = search_trie.tmatch(b"aaa", &mut matches)?;
		assert_eq!(match_count, 1);
		Ok(())
	}

	#[test]
	fn test_list_sync() -> Result<(), Error> {
		let mut list = list_sync!();
		list.push(1)?;
		assert!(list_eq!(list, list![1]));

		let mut list2: Box<dyn SortableList<_>> = list_sync_box!();
		list2.push(1)?;
		assert!(list_eq!(list2, list![1]));
		Ok(())
	}

	#[test]
	fn test_u128_to_slice() -> Result<(), Error> {
		// test 1 byte
		for i in 0..u8::MAX {
			let mut b = [0u8; 1];
			u128_to_slice(i as u128, &mut b)?;
			assert_eq!(slice_to_u128(&b)?, i as u128);
		}

		// test 2 bytes
		for i in 0..u16::MAX {
			let mut b = [0u8; 2];
			u128_to_slice(i as u128, &mut b)?;
			assert_eq!(slice_to_u128(&b)?, i as u128);
		}

		// test 3 bytes
		for i in 0..16777216 {
			let mut b = [0u8; 3];
			u128_to_slice(i as u128, &mut b)?;
			assert_eq!(slice_to_u128(&b)?, i as u128);
		}

		// one bigger is an error
		let mut b = [0u8; 3];
		u128_to_slice(16777216, &mut b)?;
		assert_eq!(b, [0xFF, 0xFF, 0xFF]);

		// 4 bytes is too big to test whole range,
		// try some bigger ones with a partial range
		for i in 1099511620000usize..1099511627776usize {
			let mut b = [0u8; 6];
			u128_to_slice(i as u128, &mut b)?;
			assert_eq!(slice_to_u128(&b)?, i as u128);
		}

		for i in 11099511620000usize..11099511627776usize {
			let mut b = [0u8; 7];
			u128_to_slice(i as u128, &mut b)?;
			assert_eq!(slice_to_u128(&b)?, i as u128);
		}

		assert!(u128_to_slice(1, &mut [0u8; 17]).is_err());

		assert!(slice_to_u128(&mut [0u8; 17]).is_err());

		Ok(())
	}

	#[test]
	fn test_usize_to_slice() -> Result<(), Error> {
		// test 1 byte
		for i in 0..u8::MAX {
			let mut b = [0u8; 1];
			usize_to_slice(i as usize, &mut b)?;
			assert_eq!(slice_to_usize(&b)?, i as usize);
		}

		// test 2 bytes
		for i in 0..u16::MAX {
			let mut b = [0u8; 2];
			usize_to_slice(i as usize, &mut b)?;
			assert_eq!(slice_to_usize(&b)?, i as usize);
		}

		// test 3 bytes
		for i in 0..16777216 {
			let mut b = [0u8; 3];
			usize_to_slice(i as usize, &mut b)?;
			assert_eq!(slice_to_usize(&b)?, i as usize);
		}

		// one bigger is an error
		let mut b = [0u8; 3];
		usize_to_slice(16777216, &mut b)?;
		assert_eq!(b, [0xFF, 0xFF, 0xFF]);

		// 4 bytes is too big to test whole range,
		// try some bigger ones with a partial range
		for i in 1099511620000usize..1099511627776usize {
			let mut b = [0u8; 6];
			usize_to_slice(i as usize, &mut b)?;
			assert_eq!(slice_to_usize(&b)?, i as usize);
		}

		for i in 11099511620000usize..11099511627776usize {
			let mut b = [0u8; 7];
			usize_to_slice(i as usize, &mut b)?;
			assert_eq!(slice_to_usize(&b)?, i as usize);
		}

		assert!(usize_to_slice(1, &mut [0u8; 9]).is_err());

		assert!(slice_to_usize(&mut [0u8; 9]).is_err());

		Ok(())
	}

	#[test]
	fn test_u32_to_slice() -> Result<(), Error> {
		// test 1 byte
		for i in 0..u8::MAX {
			let mut b = [0u8; 1];
			u32_to_slice(i as u32, &mut b)?;
			assert_eq!(slice_to_u32(&b)?, i as u32);
		}

		// test 2 bytes
		for i in 0..u16::MAX {
			let mut b = [0u8; 2];
			u32_to_slice(i as u32, &mut b)?;
			assert_eq!(slice_to_u32(&b)?, i as u32);
		}

		// test 3 bytes
		for i in 0..16777216 {
			let mut b = [0u8; 3];
			u32_to_slice(i as u32, &mut b)?;
			assert_eq!(slice_to_u32(&b)?, i as u32);
		}

		// one bigger is an error
		let mut b = [0u8; 3];
		u32_to_slice(16777216, &mut b)?;
		assert_eq!(b, [0xFF, 0xFF, 0xFF]);

		// 4 bytes is too big to test whole range,
		// try some bigger ones with a partial range
		for i in 1099511620000usize..1099511627776usize {
			let mut b = [0u8; 4];
			u32_to_slice(i as u32, &mut b)?;
			assert_eq!(slice_to_u32(&b)?, i as u32);
		}

		for i in 11099511620000usize..11099511627776usize {
			let mut b = [0u8; 4];
			u32_to_slice(i as u32, &mut b)?;
			assert_eq!(slice_to_u32(&b)?, i as u32);
		}

		assert!(u32_to_slice(1, &mut [0u8; 5]).is_err());

		assert!(slice_to_u32(&mut [0u8; 5]).is_err());
		Ok(())
	}

	#[test]
	fn test_u64_to_slice() -> Result<(), Error> {
		// test 1 byte
		for i in 0..u8::MAX {
			let mut b = [0u8; 1];
			u64_to_slice(i as u64, &mut b)?;
			assert_eq!(slice_to_u64(&b)?, i as u64);
		}
		Ok(())
	}

	#[test]
	fn test_random_u32() -> Result<(), Error> {
		let r1 = random_u32();
		let r2 = random_u32();
		let r3 = random_u32();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_u64() -> Result<(), Error> {
		let r1 = random_u64();
		let r2 = random_u64();
		let r3 = random_u64();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_u128() -> Result<(), Error> {
		let r1 = random_u128();
		let r2 = random_u128();
		let r3 = random_u128();
		debug!("r1={},r2={},r3={}", r1, r2, r3)?;
		assert!(r1 != r2 || r1 != r3); // while it's possible very unlikely.
		Ok(())
	}

	#[test]
	fn test_random_bytes() -> Result<(), Error> {
		let mut buffer1 = [0u8; 10];
		let mut buffer2 = [0u8; 10];
		let mut buffer3 = [0u8; 10];

		random_bytes(&mut buffer1);
		random_bytes(&mut buffer2);
		random_bytes(&mut buffer3);
		debug!("r1={:?},r2={:?},r3={:?}", buffer1, buffer2, buffer3)?;
		assert!(buffer1 != buffer2 || buffer2 != buffer3);
		Ok(())
	}

	#[derive(Debug, PartialEq)]
	struct SerAll {
		a: u8,
		b: i8,
		c: u16,
		d: i16,
		e: u32,
		f: i32,
		g: u64,
		h: i64,
		i: u128,
		j: i128,
		k: usize,
	}

	impl Serializable for SerAll {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			let a = reader.read_u8()?;
			let b = reader.read_i8()?;
			let c = reader.read_u16()?;
			let d = reader.read_i16()?;
			let e = reader.read_u32()?;
			let f = reader.read_i32()?;
			let g = reader.read_u64()?;
			let h = reader.read_i64()?;
			let i = reader.read_u128()?;
			let j = reader.read_i128()?;
			let k = reader.read_usize()?;
			reader.expect_u8(100)?;
			assert_eq!(reader.read_u64()?, 4);
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_empty_bytes(10)?;

			let ret = Self {
				a,
				b,
				c,
				d,
				e,
				f,
				g,
				h,
				i,
				j,
				k,
			};

			Ok(ret)
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			writer.write_u8(self.a)?;
			writer.write_i8(self.b)?;
			writer.write_u16(self.c)?;
			writer.write_i16(self.d)?;
			writer.write_u32(self.e)?;
			writer.write_i32(self.f)?;
			writer.write_u64(self.g)?;
			writer.write_i64(self.h)?;
			writer.write_u128(self.i)?;
			writer.write_i128(self.j)?;
			writer.write_usize(self.k)?;
			writer.write_u8(100)?;
			writer.write_bytes([1, 2, 3, 4])?;
			writer.write_empty_bytes(10)?;
			Ok(())
		}
	}

	fn ser_helper<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: S = deserialize(&mut &v[..])?;
		assert_eq!(ser_in, ser_out);
		Ok(())
	}

	fn ser_helper_slabs<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut slab_writer = SlabWriter::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_writer.seek(slab.id(), 0);
		ser_out.write(&mut slab_writer)?;
		let mut slab_reader = SlabReader::new(None, slab.id(), None)?;
		slab_reader.seek(slab.id(), 0);
		let ser_in = S::read(&mut slab_reader)?;
		assert_eq!(ser_in, ser_out);

		Ok(())
	}

	#[test]
	fn test_skip_bytes() -> Result<(), Error> {
		let mut slabs = slab_allocator(1024, 10_240)?;

		let slab_id = {
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			let slab = (**guard).allocate()?;
			slab.id()
		};

		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		slab_writer.skip_bytes(800)?;
		slab_writer.write_u128(123)?;

		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		slab_reader.skip_bytes(800)?;
		assert_eq!(slab_reader.read_u128()?, 123);

		slab_writer.slabs = None;
		slab_writer.skip_bytes(1)?;

		Ok(())
	}

	#[test]
	fn test_serialization_slab_rw() -> Result<(), Error> {
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
		};

		ser_helper_slabs(ser_out)?;

		let ser_err = SerErr { exp: 100, empty: 0 };

		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		let mut slab_writer = SlabWriter::new(None, slab.id(), None)?;
		slab_writer.seek(slab.id(), 0);
		ser_err.write(&mut slab_writer)?;
		let mut slab_reader = SlabReader::new(None, slab.id(), None)?;
		slab_reader.seek(slab.id(), 0);
		assert!(SerErr::read(&mut slab_reader).is_err());

		Ok(())
	}

	#[test]
	fn test_serialization() -> Result<(), Error> {
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
		};
		ser_helper(ser_out)?;
		ser_helper(())?;
		ser_helper((rand::random::<u32>(), rand::random::<i128>()))?;
		ser_helper(("hi there".to_string(), 123))?;
		let x = [3u8; 8];
		ser_helper(x)?;

		let ser_out = SerErr { exp: 100, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let ser_out = SerErr { exp: 99, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_ok());

		let ser_out = SerErr { exp: 99, empty: 1 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let v = vec!["test1".to_string(), "a".to_string(), "okokok".to_string()];
		ser_helper(v)?;

		let mut hashtable = hashtable_box!(MaxEntries(123), MaxLoadFactor(0.5))?;
		hashtable.insert(&1, &2)?;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &hashtable)?;
		let ser_in: Box<dyn Hashtable<u32, u32>> = deserialize(&mut &v[..])?;
		assert_eq!(ser_in.max_entries(), hashtable.max_entries());
		assert_eq!(ser_in.max_load_factor(), hashtable.max_load_factor());
		assert_eq!(ser_in.get(&1)?, Some(2));

		let mut hashset = hashset_box!(MaxEntries(23), MaxLoadFactor(0.54))?;
		hashset.insert(&1)?;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &hashset)?;
		let ser_in: Box<dyn Hashset<u32>> = deserialize(&mut &v[..])?;
		assert_eq!(ser_in.max_entries(), hashset.max_entries());
		assert_eq!(ser_in.max_load_factor(), hashset.max_load_factor());
		assert!(ser_in.contains(&1)?);

		Ok(())
	}

	fn slab_allocator(
		slab_size: usize,
		slab_count: usize,
	) -> Result<Box<dyn LockBox<Box<dyn SlabAllocator + Send + Sync>>>, Error> {
		let config = bmw_util::SlabAllocatorConfig {
			slab_count,
			slab_size,
			..Default::default()
		};
		let mut slabs = lock_box!(UtilBuilder::build_sync_slabs())?;

		{
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			(**guard).init(config)?;
		}

		Ok(slabs)
	}

	#[test]
	fn test_slab_rw() -> Result<(), Error> {
		let mut slabs = slab_allocator(1024, 10_240)?;

		let slab_id = {
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			let slab = (**guard).allocate()?;
			slab.id()
		};

		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		slab_writer.write_u64(123)?;
		slab_writer.write_u128(123)?;

		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		assert_eq!(slab_reader.read_u64()?, 123);
		assert_eq!(slab_reader.read_u128()?, 123);

		Ok(())
	}

	#[test]
	fn test_multi_slabs() -> Result<(), Error> {
		let mut slabs = slab_allocator(1024, 10_240)?;
		let slab_id = {
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			let mut slab = (**guard).allocate()?;
			let slab_mut = slab.get_mut();
			for j in 0..1024 {
				slab_mut[j] = 0xFF;
			}

			slab.id()
		};
		let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;
		let r = 10_100;
		for i in 0..r {
			slab_writer.write_u128(i)?;
		}
		let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
		for i in 0..r {
			assert_eq!(slab_reader.read_u128()?, i);
		}

		let mut v = vec![];
		v.resize(1024 * 2, 0u8);
		// we can't read anymore
		assert!(slab_reader.read_fixed_bytes(&mut v).is_err());

		Ok(())
	}

	#[test]
	fn test_global_multi_slabs() -> Result<(), Error> {
		global_slab_allocator!()?;
		let slab_id = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			let slabs = unsafe { f.get().as_mut().unwrap() };
			let mut slab = slabs.allocate()?;
			let slab_mut = slab.get_mut();
			for j in 0..slab_mut.len() {
				slab_mut[j] = 0xFF;
			}
			Ok(slab.id())
		})?;
		let mut slab_writer = SlabWriter::new(None, slab_id, None)?;
		let r = 10_100;
		for i in 0..r {
			slab_writer.write_u128(i)?;
		}
		let mut slab_reader = SlabReader::new(None, slab_id, None)?;
		for i in 0..r {
			assert_eq!(slab_reader.read_u128()?, i);
		}

		let mut v = vec![];
		v.resize(1024 * 2, 0u8);
		// we can't read anymore
		assert!(slab_reader.read_fixed_bytes(&mut v).is_err());

		Ok(())
	}

	#[test]
	fn test_alternate_sized_slabs() -> Result<(), Error> {
		for i in 0..1000 {
			let mut slabs = slab_allocator(48 + i, 10)?;

			let slab_id = {
				let mut slabs = slabs.wlock()?;
				let guard = slabs.guard();
				let mut slab = (**guard).allocate()?;
				let slab_mut = slab.get_mut();
				for j in 0..slab_mut.len() {
					slab_mut[j] = 0xFF;
				}

				slab.id()
			};
			let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;

			let mut v = [0u8; 256];
			for i in 0..v.len() {
				v[i] = (i % 256) as u8;
			}
			slab_writer.write_fixed_bytes(v)?;

			let mut slab_reader = SlabReader::new(Some(slabs), slab_id, None)?;
			let mut v_back = [1u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);
		}
		// test capacity exceeded

		// 470 is ok because only 1 byte overhead per slab.
		let mut slabs = slab_allocator(48, 10)?;
		let slab_id = {
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			let slab = (**guard).allocate()?;
			slab.id()
		};
		let mut slab_writer = SlabWriter::new(Some(slabs), slab_id, None)?;
		let mut v = [0u8; 470];
		for i in 0..v.len() {
			v[i] = (i % 256) as u8;
		}
		assert!(slab_writer.write_fixed_bytes(v).is_ok());

		// 471 is one too many and returns error (note: user responsible for cleanup)
		let mut slabs = slab_allocator(48, 10)?;
		let _slab_id = {
			let mut slabs = slabs.wlock()?;
			let guard = slabs.guard();
			let slab = (**guard).allocate()?;
			slab.id()
		};
		//let mut slab_writer = SlabWriter::new(Some(slabs), slab_id, None)?;
		let mut v = [0u8; 471];
		for i in 0..v.len() {
			v[i] = (i % 256) as u8;
		}
		// since raw_write this is not an error it just adds on.
		//assert!(slab_writer.write_fixed_bytes(v).is_err());

		Ok(())
	}

	#[test]
	fn slab_writer_out_of_slabs() -> Result<(), Error> {
		global_slab_allocator!(SlabSize(100), SlabCount(1))?;
		let free_count1 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count1)?;

		{
			let slabid = {
				let mut slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
					let slabs = unsafe { f.get().as_mut().unwrap() };
					slabs.allocate()
				})?;
				let slab_mut = slab.get_mut();
				slab_mut[99] = 0xFF; // set next to 0xFF
				slab.id()
			};
			let mut writer = SlabWriter::new(None, slabid, None)?;
			let mut v = vec![];
			for _ in 0..200 {
				v.push(1);
			}
			assert!(writer.write_fixed_bytes(v).is_err());

			// user responsible for freeing the chain
			GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
				let slabs = unsafe { f.get().as_mut().unwrap() };
				slabs.free(slabid)?;
				Ok(())
			})?;
		}
		let free_count2 = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<usize, Error> {
			Ok(unsafe { f.get().as_ref().unwrap().free_count()? })
		})?;
		info!("free_count={}", free_count2)?;
		assert_eq!(free_count1, free_count2);

		Ok(())
	}

	#[test]
	fn test_seek() -> Result<(), Error> {
		for i in 0..1000 {
			let mut slabs = slab_allocator(48 + i, 10)?;

			let slab_id = {
				let mut slabs = slabs.wlock()?;
				let guard = slabs.guard();
				let mut slab = (**guard).allocate()?;
				let slab_mut = slab.get_mut();
				for j in 0..48 + i {
					slab_mut[j] = 0xFF;
				}
				slab.id()
			};
			let mut slab_writer = SlabWriter::new(Some(slabs.clone()), slab_id, None)?;

			let mut v = [0u8; 256];
			for i in 0..v.len() {
				v[i] = (i % 256) as u8;
			}
			slab_writer.write_fixed_bytes(v)?;

			let mut slab_reader = SlabReader::new(Some(slabs.clone()), slab_id, None)?;
			let mut v_back = [1u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);

			slab_reader.seek(slab_id, 0);
			let mut v_back = [3u8; 256];
			slab_reader.read_fixed_bytes(&mut v_back)?;
			assert_eq!(v, v_back);
		}

		Ok(())
	}

	#[test]
	fn test_global_slab_writer_unallocated() -> Result<(), Error> {
		let mut slab_writer = SlabWriter::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_writer.seek(slab.id(), 0);

		Ok(())
	}

	#[test]
	fn test_global_slab_reader_unallocated() -> Result<(), Error> {
		let mut slab_reader = SlabReader::new(None, 0, None)?;
		let slab = GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		slab_reader.seek(slab.id(), 0);

		Ok(())
	}

	#[test]
	fn test_ser_array_and_array_list() -> Result<(), Error> {
		let mut arr = Array::new(10, &0)?;
		for i in 0..arr.size() {
			arr[i] = i;
		}
		ser_helper(arr)?;

		let mut v: Vec<u8> = vec![];
		v.push(0);
		v.push(0);
		v.push(0);
		v.push(0);

		v.push(0);
		v.push(0);
		v.push(0);
		v.push(0);
		let ser_in: Result<Array<u8>, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		let mut arrlist: ArrayList<usize> = ArrayList::new(20, &0)?;
		for i in 0..20 {
			List::push(&mut arrlist, i)?;
		}
		ser_helper(arrlist)?;
		Ok(())
	}

	#[test]
	fn test_sortable_list() -> Result<(), Error> {
		let ser_out = list_box![1, 2, 3, 4];
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Box<dyn SortableList<u32>> = deserialize(&mut &v[..])?;
		assert!(list_eq!(ser_in, ser_out));
		Ok(())
	}

	#[test]
	fn test_ser_option() -> Result<(), Error> {
		let mut x: Option<bool> = None;
		ser_helper(x)?;
		x = Some(false);
		ser_helper(x)?;
		x = Some(true);
		ser_helper(x)?;

		Ok(())
	}

	#[test]
	fn test_read_ref() -> Result<(), Error> {
		let r = 1u32;
		let ser_out = &r;
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<&u32, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());
		Ok(())
	}

	#[test]
	fn test_simple() -> Result<(), Error> {
		let mut slabs = UtilBuilder::build_slabs();

		assert!(slabs.slab_count().is_err());
		assert!(slabs.slab_size().is_err());

		slabs.init(SlabAllocatorConfig::default())?;

		let (id1, id2);
		{
			let mut slab = slabs.allocate()?;
			id1 = slab.id();
			slab.get_mut()[0] = 111;
		}

		{
			let mut slab = slabs.allocate()?;
			id2 = slab.id();
			slab.get_mut()[0] = 112;
		}

		assert_eq!(slabs.get(id1)?.get()[0], 111);
		assert_eq!(slabs.get(id2)?.get()[0], 112);

		Ok(())
	}

	#[test]
	fn test_static_slaballoc() -> Result<(), Error> {
		crate::slabs::GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
			unsafe {
				f.get()
					.as_mut()
					.unwrap()
					.init(SlabAllocatorConfig::default())?;
				Ok(())
			}
		})?;
		let slab = crate::slabs::GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<SlabMut<'_>, Error> {
			Ok(unsafe { f.get().as_mut().unwrap().allocate()? })
		})?;
		info!("slab={:?}", slab.get())?;
		Ok(())
	}

	#[test]
	fn test_capacity() -> Result<(), Error> {
		let mut slabs = UtilBuilder::build_slabs();
		slabs.init(SlabAllocatorConfig {
			slab_count: 10,
			..SlabAllocatorConfig::default()
		})?;
		for _ in 0..10 {
			slabs.allocate()?;
		}
		assert!(slabs.allocate().is_err());

		slabs.free(0)?;
		assert!(slabs.allocate().is_ok());
		assert!(slabs.allocate().is_err());
		Ok(())
	}

	#[test]
	fn test_error_conditions() -> Result<(), Error> {
		let mut slabs = UtilBuilder::build_slabs();
		assert!(slabs.allocate().is_err());
		assert!(slabs.free(0).is_err());
		assert!(slabs.get(0).is_err());
		assert!(slabs.get_mut(0).is_err());
		assert!(slabs.free_count().is_err());
		slabs.init(SlabAllocatorConfig::default())?;
		assert!(slabs.allocate().is_ok());
		assert!(slabs.free_count().is_ok());
		assert!(slabs.free(usize::MAX).is_err());
		assert!(slabs.get(usize::MAX).is_err());
		assert!(slabs.get_mut(usize::MAX).is_err());
		assert!(slabs.init(SlabAllocatorConfig::default()).is_err());
		Ok(())
	}

	#[test]
	fn test_double_free() -> Result<(), Error> {
		let mut slabs = UtilBuilder::build_slabs();
		slabs.init(SlabAllocatorConfig::default())?;
		let id = {
			let slab = slabs.allocate()?;
			slab.id()
		};
		let slab = slabs.get(id)?;
		assert_eq!(slab.id(), id);
		slabs.free(id)?;
		assert!(slabs.free(id).is_err());
		let id2 = {
			let slab = slabs.allocate()?;
			slab.id()
		};
		slabs.free(id2)?;
		assert!(slabs.free(id2).is_err());
		// we know id and id2 are equal because when you free a slab it's added to the
		// front of the list
		assert_eq!(id, id2);
		Ok(())
	}

	#[test]
	fn test_other_slabs_configs() -> Result<(), Error> {
		assert!(UtilBuilder::build_slabs()
			.init(SlabAllocatorConfig::default())
			.is_ok());

		assert!(UtilBuilder::build_slabs()
			.init(SlabAllocatorConfig {
				slab_size: 100,
				..SlabAllocatorConfig::default()
			})
			.is_ok());

		assert!(UtilBuilder::build_slabs()
			.init(SlabAllocatorConfig {
				slab_size: 48,
				..SlabAllocatorConfig::default()
			})
			.is_ok());

		assert!(UtilBuilder::build_slabs()
			.init(SlabAllocatorConfig {
				slab_size: 7,
				..SlabAllocatorConfig::default()
			})
			.is_err());
		assert!(UtilBuilder::build_slabs()
			.init(SlabAllocatorConfig {
				slab_count: 0,
				..SlabAllocatorConfig::default()
			})
			.is_err());

		let mut sh = UtilBuilder::build_slabs();
		sh.init(SlabAllocatorConfig {
			slab_count: 1,
			..SlabAllocatorConfig::default()
		})?;

		let slab = sh.allocate();
		assert!(slab.is_ok());
		let id = slab.unwrap().id();
		assert!(sh.allocate().is_err());
		sh.free(id)?;
		assert!(sh.allocate().is_ok());

		Ok(())
	}

	#[test]
	fn test_pattern() -> Result<(), Error> {
		let pattern = Pattern::new(vec![
			Regex("abc".to_string()),
			IsCaseSensitive(true),
			IsTerminationPattern(true),
			IsMultiLine(true),
			PatternId(0),
		]);
		assert!(pattern.is_err());
		let pattern = Pattern::new(vec![
			Regex("abc".to_string()),
			IsCaseSensitive(false),
			IsTerminationPattern(true),
			IsMultiLine(true),
			PatternId(0),
		])?;
		assert_eq!(pattern.regex(), "abc");
		assert_eq!(pattern.is_case_sensitive(), false);
		assert_eq!(pattern.is_termination_pattern(), true);
		assert_eq!(pattern.id(), 0);

		Ok(())
	}

	#[test]
	fn test_search_trie1() -> Result<(), Error> {
		let mut search_trie = UtilBuilder::build_search_trie(
			vec![
				UtilBuilder::build_pattern(vec![
					Regex("p1".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(0),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p2".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(1),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p3".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(2),
				])?,
			],
			1_000,
			100,
		)?;

		let mut matches = [tmatch!()?; 10];
		let count = search_trie.tmatch(b"p1p2", &mut matches)?;
		info!("count={}", count)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[0].end(), 2);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 2);
		assert_eq!(matches[1].end(), 4);

		Ok(())
	}

	#[test]
	fn test_search_trie_wildcard() -> Result<(), Error> {
		let mut search_trie = UtilBuilder::build_search_trie(
			vec![
				UtilBuilder::build_pattern(vec![
					Regex("p1.*abc".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(0),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p2".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(1),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p3".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(2),
				])?,
			],
			37,
			10,
		)?;

		let mut matches = [tmatch!()?; 10];
		let count = search_trie.tmatch(b"p1xyz123abcp2", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[0].end(), 11);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 11);
		assert_eq!(matches[1].end(), 13);
		for i in 0..count {
			info!("match[{}]={:?}", i, matches[i])?;
		}

		// try a wildcard that is too long
		let count = search_trie.tmatch(b"p1xyzxxxxxxxxxxxxxxxxxxxxxxxx123abcp2", &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 35);
		assert_eq!(matches[0].end(), 37);
		for i in 0..count {
			info!("match[{}]={:?}", i, matches[i])?;
		}

		// test termination
		let count =
			search_trie.tmatch(b"p1xyzxxxxxxxxxxxxxxxxxxxxxxxxxxx123abcp2", &mut matches)?;
		assert_eq!(count, 0);

		// non-repeating wildcard
		let mut search_trie = UtilBuilder::build_search_trie(
			vec![
				UtilBuilder::build_pattern(vec![
					Regex("p1.abc".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(0),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p2".to_string()),
					IsCaseSensitive(false),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(1),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p3".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(2),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p4.".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(3),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p5\\\\x".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(4),
				])?,
				UtilBuilder::build_pattern(vec![
					Regex("p6\\.x".to_string()),
					IsCaseSensitive(true),
					IsTerminationPattern(false),
					IsMultiLine(false),
					PatternId(5),
				])?,
			],
			37,
			10,
		)?;
		// 2 wildcard chars so no match
		let count = search_trie.tmatch(b"p1xxabc", &mut matches)?;
		assert_eq!(count, 0);

		// 1 wildcard char so it's a match
		let count = search_trie.tmatch(b"p1xabc", &mut matches)?;
		assert_eq!(count, 1);

		// no char after p4 so no match
		let count = search_trie.tmatch(b"p4", &mut matches)?;
		assert_eq!(count, 0);

		// char after p4 so match
		let count = search_trie.tmatch(b"p4a", &mut matches)?;
		assert_eq!(count, 1);

		// char after p4 so match
		let count = search_trie.tmatch(b"p4aaa", &mut matches)?;
		assert_eq!(count, 1);

		// '\' matches
		let count = search_trie.tmatch(b"p5\\x", &mut matches)?;
		assert_eq!(count, 1);

		// escaped dot match
		let count = search_trie.tmatch(b"p6.x", &mut matches)?;
		assert_eq!(count, 1);

		// escaped dot is not a wildcard
		let count = search_trie.tmatch(b"p6ax", &mut matches)?;
		assert_eq!(count, 0);

		Ok(())
	}

	#[test]
	fn test_case_sensitivity() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let pattern1 = UtilBuilder::build_pattern(vec![
			Regex("AaAaA".to_string()),
			IsCaseSensitive(true),
			IsTerminationPattern(false),
			IsMultiLine(false),
			PatternId(0),
		])?;
		let pattern2 = UtilBuilder::build_pattern(vec![
			Regex("AaAaA".to_string()),
			IsCaseSensitive(false),
			IsTerminationPattern(false),
			IsMultiLine(false),
			PatternId(0),
		])?;

		let mut search_trie = UtilBuilder::build_search_trie(vec![pattern1], 100, 100)?;

		assert_eq!(search_trie.tmatch(b"AAAAA", &mut matches)?, 0);

		let mut search_trie = UtilBuilder::build_search_trie(vec![pattern2], 100, 100)?;

		assert_eq!(search_trie.tmatch(b"AAAAA", &mut matches)?, 1);

		Ok(())
	}

	#[test]
	fn test_multi_line() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(
				Regex("abc.*123".to_string()),
				PatternId(0),
				IsMultiLine(false)
			)?,
			pattern!(
				Regex("def.*123".to_string()),
				PatternId(1),
				IsMultiLine(true)
			)?
		],)?;

		// this will not match because of the newline
		let count = search_trie.tmatch(b"abcxxx\n123", &mut matches)?;
		assert_eq!(count, 0);

		// this will match because IsMulti is true
		let count = search_trie.tmatch(b"defxxx\n123", &mut matches)?;
		assert_eq!(count, 1);
		Ok(())
	}

	#[test]
	fn test_termination_pattern() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(
				Regex("abc".to_string()),
				IsCaseSensitive(false),
				PatternId(0),
				IsTerminationPattern(false)
			)?,
			pattern!(
				Regex("def".to_string()),
				IsCaseSensitive(false),
				PatternId(1),
				IsTerminationPattern(true)
			)?
		],)?;

		// both matches will be found
		let count = search_trie.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 2);

		// only the first match will be found because it is a termination pattern
		let count = search_trie.tmatch(b"defabc", &mut matches)?;
		assert_eq!(count, 1);
		Ok(())
	}

	#[test]
	fn test_match_list_too_big() -> Result<(), Error> {
		let mut matches1 = [tmatch!()?; 1];
		let mut matches10 = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(
				Regex("abc".to_string()),
				IsCaseSensitive(false),
				PatternId(0),
				IsTerminationPattern(false)
			)?,
			pattern!(
				Regex("def".to_string()),
				IsCaseSensitive(false),
				PatternId(1),
				IsTerminationPattern(true)
			)?
		],)?;

		// only one match returned because match list length is 1
		let count = search_trie.tmatch(b"abcdef", &mut matches1)?;
		assert_eq!(count, 1);

		// both matches returned with long enough list
		let count = search_trie.tmatch(b"abcdef", &mut matches10)?;
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_search_trie_overlap() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(Regex("abc".to_string()), PatternId(0))?,
			pattern!(Regex("abcdef".to_string()), PatternId(1))?
		],)?;

		let count = search_trie.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[1].start(), 0);

		let mut count = 0;
		for i in 0..2 {
			if matches[i].id() == 1 {
				assert_eq!(matches[i].end(), 6);
				count += 1;
			} else if matches[i].id() == 0 {
				assert_eq!(matches[i].end(), 3);
				count += 1;
			}
		}
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_search_trie_caret() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(Regex("abc".to_string()), PatternId(0))?,
			pattern!(Regex("^def".to_string()), PatternId(1))?
		],)?;

		// only abc is found because def is not at the start
		let count = search_trie.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 1);

		// both found
		let count = search_trie.tmatch(b"defabc", &mut matches)?;
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_search_trie_error_conditions() -> Result<(), Error> {
		assert!(UtilBuilder::build_search_trie(
			vec![UtilBuilder::build_pattern(vec![
				Regex("".to_string()),
				IsCaseSensitive(false),
				IsTerminationPattern(false),
				IsMultiLine(false),
				PatternId(0)
			])?],
			36,
			36
		)
		.is_err());

		assert!(UtilBuilder::build_search_trie(
			vec![UtilBuilder::build_pattern(vec![
				Regex("^".to_string()),
				IsCaseSensitive(false),
				IsTerminationPattern(false),
				IsMultiLine(false),
				PatternId(0)
			])?],
			100,
			100
		)
		.is_err());

		assert!(UtilBuilder::build_search_trie(
			vec![UtilBuilder::build_pattern(vec![
				Regex("x\\y".to_string()),
				IsCaseSensitive(false),
				IsTerminationPattern(false),
				IsMultiLine(false),
				PatternId(0)
			])?],
			100,
			100
		)
		.is_err());

		assert!(UtilBuilder::build_search_trie(
			vec![UtilBuilder::build_pattern(vec![
				Regex("x\\".to_string()),
				IsCaseSensitive(false),
				IsTerminationPattern(false),
				IsMultiLine(false),
				PatternId(0)
			])?],
			100,
			100
		)
		.is_err());

		assert!(UtilBuilder::build_search_trie(vec![], 100, 100).is_err());

		Ok(())
	}

	#[test]
	fn test_search_trie_branches() -> Result<(), Error> {
		let mut matches = [tmatch!()?; 10];
		let mut search_trie = search_trie!(vec![
			pattern!(Regex("abc".to_string()), PatternId(0))?,
			pattern!(Regex("ab.*x".to_string()), PatternId(1))?
		],)?;

		let count = search_trie.tmatch(b"abc", &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);

		let count = search_trie.tmatch(b"abcx", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 0);

		let text = b"abxx";
		let count = search_trie.tmatch(text, &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(&text[matches[0].start()..matches[0].end()], text);

		let mut search_trie = search_trie!(vec![
			pattern!(Regex("header: 1234\r\n".to_string()), PatternId(1))?,
			pattern!(Regex("header: .*\r\n".to_string()), PatternId(0))?
		],)?;

		let text = b"yyyheader: 1299\r\n";
		let count = search_trie.tmatch(text, &mut matches)?;
		info!("count1={}", count)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 3);
		assert_eq!(matches[0].end(), text.len());

		let text = b"yyyheader: 1234\r\n";
		let count = search_trie.tmatch(text, &mut matches)?;
		info!("count2={}", count)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 3);
		assert_eq!(matches[0].end(), text.len());
		assert_eq!(matches[1].id(), 0);
		assert_eq!(matches[1].start(), 3);
		assert_eq!(matches[1].end(), text.len());

		Ok(())
	}

	#[test]
	fn test_threadpool1() -> Result<(), Error> {
		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(10), MaxSize(10)])?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		// simple execution, return value
		let res = tp.execute(async move { Ok(1) }, 0)?;
		assert_eq!(res.recv()?, PoolResult::Ok(1));

		// increment value using locks
		let mut x = lock!(1)?;
		let x_clone = x.clone();
		tp.execute(
			async move {
				**x.wlock()?.guard() = 2;
				Ok(1)
			},
			0,
		)?;

		let mut count = 0;
		loop {
			count += 1;
			assert!(count < 500);
			sleep(Duration::from_millis(10));
			if **x_clone.rlock()?.guard() == 2 {
				break;
			}
		}

		// return an error
		let res = tp.execute(async move { Err(err!(ErrKind::Test, "test")) }, 0)?;

		assert_eq!(res.recv()?, PoolResult::Err(err!(ErrKind::Test, "test")));

		// handle panic
		let res = tp.execute(
			async move {
				let x: Option<u32> = None;
				Ok(x.unwrap())
			},
			0,
		)?;

		assert!(res.recv().is_err());

		// 10 more panics to ensure pool keeps running
		for _ in 0..10 {
			let res = tp.execute(
				async move {
					let x: Option<u32> = None;
					Ok(x.unwrap())
				},
				0,
			)?;

			assert!(res.recv().is_err());
		}

		// now do a regular request
		let res = tp.execute(async move { Ok(5) }, 0)?;
		assert_eq!(res.recv()?, PoolResult::Ok(5));

		sleep(Duration::from_millis(1000));
		info!("test sending errors")?;

		// send an error and ignore the response
		{
			let res = tp.execute(async move { Err(err!(ErrKind::Test, "")) }, 0)?;
			drop(res);
			sleep(Duration::from_millis(1000));
		}
		{
			let res = tp.execute(async move { Err(err!(ErrKind::Test, "test")) }, 0)?;
			assert_eq!(res.recv()?, PoolResult::Err(err!(ErrKind::Test, "test")));
		}
		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_sizing() -> Result<(), Error> {
		let mut tp = ThreadPoolImpl::new(vec![MinSize(2), MaxSize(4)])?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;
		let mut v = vec![];

		let x = lock!(0)?;

		loop {
			sleep(Duration::from_millis(10));
			{
				let state = tp.state.rlock()?;
				if (**state.guard()).waiting == 2 {
					break;
				}
			}
		}
		assert_eq!(tp.size()?, 2);
		// first use up all the min_size threads
		let y = lock!(0)?;
		for _ in 0..2 {
			let mut y_clone = y.clone();
			let x_clone = x.clone();
			let res = tp.execute(
				async move {
					**(y_clone.wlock()?.guard()) += 1;
					loop {
						if **(x_clone.rlock()?.guard()) != 0 {
							break;
						}
						sleep(Duration::from_millis(50));
					}
					Ok(1)
				},
				0,
			)?;
			v.push(res);
		}
		loop {
			sleep(Duration::from_millis(100));
			{
				let y = y.rlock()?;
				if (**y.guard()) == 2 {
					break;
				}
			}
		}
		assert_eq!(tp.size()?, 3);

		// confirm we can still process
		for _ in 0..10 {
			let mut x_clone = x.clone();
			let res = tp.execute(
				async move {
					**(x_clone.wlock()?.guard()) = 1;

					Ok(2)
				},
				0,
			)?;
			assert_eq!(res.recv()?, PoolResult::Ok(2));
		}

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 2);

		let mut i = 0;
		for res in v {
			assert_eq!(res.recv()?, PoolResult::Ok(1));
			info!("res complete {}", i)?;
			i += 1;
		}

		sleep(Duration::from_millis(1000));

		let mut x2 = lock!(0)?;

		// confirm that the maximum is in place

		// block all 4 threads waiting on x2
		for _ in 0..4 {
			let mut x2_clone = x2.clone();
			tp.execute(
				async move {
					info!("x0a")?;
					loop {
						if **(x2_clone.rlock()?.guard()) != 0 {
							break;
						}
						sleep(Duration::from_millis(50));
					}
					info!("x2")?;
					**(x2_clone.wlock()?.guard()) += 1;

					Ok(0)
				},
				0,
			)?;
		}

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 4);

		// confirm that the next thread cannot be processed
		let mut x2_clone = x2.clone();
		tp.execute(
			async move {
				info!("x0")?;
				**(x2_clone.wlock()?.guard()) += 1;
				info!("x1")?;
				Ok(0)
			},
			0,
		)?;

		// wait
		sleep(Duration::from_millis(2_000));

		// confirm situation hasn't changed
		assert_eq!(**(x2.rlock()?.guard()), 0);

		// unlock the threads by setting x2 to 1
		**(x2.wlock()?.guard()) = 1;

		// wait
		sleep(Duration::from_millis(4_000));
		assert_eq!(**(x2.rlock()?.guard()), 6);
		info!("exit all")?;

		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 2);

		tp.stop()?;
		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 0);

		Ok(())
	}

	#[test]
	fn test_stop() -> Result<(), Error> {
		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(2), MaxSize(4)])?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		sleep(Duration::from_millis(1000));
		assert_eq!(tp.size()?, 2);
		tp.execute(async move { Ok(()) }, 0)?;

		sleep(Duration::from_millis(1000));
		info!("stopping pool")?;
		assert_eq!(tp.size()?, 2);
		tp.stop()?;

		sleep(Duration::from_millis(1000));
		assert_eq!(tp.size()?, 0);
		Ok(())
	}

	#[test]
	fn pass_to_threads() -> Result<(), Error> {
		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(2), MaxSize(4)])?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		let tp = lock!(tp)?;
		for _ in 0..6 {
			let tp = tp.clone();
			std::thread::spawn(move || -> Result<(), Error> {
				let tp = tp.rlock()?;
				execute!((**tp.guard()), {
					info!("executing in thread pool")?;
					Ok(1)
				})?;
				Ok(())
			});
		}
		Ok(())
	}

	#[test]
	fn test_bad_configs() -> Result<(), Error> {
		/*
		assert!(load_config(6, 5).is_err());
		assert!(load_config(0, 4).is_err());
			*/

		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(5), MaxSize(6)])?;
		tp.set_on_panic(move |_, _| -> Result<(), Error> { Ok(()) })?;

		assert_eq!(
			tp.execute(async move { Ok(()) }, 0).unwrap_err(),
			err!(
				ErrKind::IllegalState,
				"Thread pool has not been initialized"
			)
		);
		Ok(())
	}

	#[test]
	fn test_no_on_panic_handler() -> Result<(), Error> {
		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(1), MaxSize(1)])?;

		tp.set_on_panic(move |_, _| -> Result<(), Error> { Ok(()) })?;
		tp.set_on_panic_none()?;
		tp.start()?;
		tp.execute(
			async move {
				if true {
					panic!("2");
				}
				Ok(())
			},
			0,
		)?;

		sleep(Duration::from_millis(1_000));

		Ok(())
	}

	#[test]
	fn test_on_panic_error() -> Result<(), Error> {
		let mut tp = UtilBuilder::build_thread_pool(vec![MinSize(1), MaxSize(1)])?;

		let mut count = lock_box!(0)?;
		let count_clone = count.clone();
		tp.set_on_panic(move |_, _| -> Result<(), Error> {
			let mut count = count.wlock()?;
			**count.guard() += 1;
			return Err(err!(ErrKind::Test, "panic errored"));
		})?;

		// test that unstarted pool returns err
		let executor = tp.executor()?;
		assert!(executor.execute(async move { Ok(0) }, 0,).is_err());

		tp.start()?;

		tp.execute(
			async move {
				panic!("err");
			},
			0,
		)?;

		let mut count = 0;
		loop {
			count += 1;
			sleep(Duration::from_millis(1));
			if **(count_clone.rlock()?.guard()) != 1 && count < 5_000 {
				continue;
			}
			assert_eq!(**(count_clone.rlock()?.guard()), 1);
			break;
		}

		// ensure processing can still occur (1 thread means that thread recovered after
		// panic)
		let res = tp.execute(
			async move {
				info!("execute")?;
				Ok(1)
			},
			0,
		)?;

		assert_eq!(res.recv()?, PoolResult::Ok(1));

		Ok(())
	}

	#[test]
	fn test_thread_pool_macro_panic() -> Result<(), Error> {
		let (tx, rx) = sync_channel(1);
		let mut v = lock_box!(0)?;
		let v_clone = v.clone();
		info!("testing thread_pool macro")?;

		let mut tp = thread_pool!(MinSize(4))?;
		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			match e.downcast_ref::<&str>() {
				Some(as_str) => {
					info!("Error: {:?}", as_str)?;
					assert_eq!(as_str, &"test88");
					wlock!(v) += 1;
					tx.send(())?;
				}
				None => {
					info!("Unknown panic type")?;
				}
			}

			info!("PANIC: id={},e={:?}", id, e)?;
			Ok(())
		})?;

		tp.start()?;

		execute!(tp, {
			info!("executing a thread")?;
			if true {
				// avoid compiler warning
				panic!("test88");
			}

			Ok(())
		})?;

		rx.recv()?;
		assert_eq!(rlock!(v_clone), 1);

		Ok(())
	}

	#[test]
	fn test_thread_pool_resources() -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(2), MaxSize(3))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;
		assert_eq!(tp.size()?, 2);

		let count = lock_box!(0)?;
		let complete_count = lock_box!(0)?;

		let (tx, rx) = sync_channel(1);
		let mut v = vec![];
		let (comptx, comprx) = sync_channel(1);

		for _ in 0..5 {
			let mut count = count.clone();
			let mut complete_count = complete_count.clone();
			let comptx = comptx.clone();
			let tx = tx.clone();
			let (tx2, rx2) = sync_channel(1);
			v.push(tx2);
			execute!(tp, {
				let localcount;
				{
					let mut count = count.wlock()?;
					let guard = count.guard();
					localcount = **guard;
					(**guard) += 1;
				}

				info!("i={}", localcount)?;

				if localcount >= 2 {
					info!("sending")?;
					tx.send(())?;
				}

				rx2.recv()?;
				info!("ending thread {}", localcount)?;

				{
					let mut count = complete_count.wlock()?;
					let guard = count.guard();
					(**guard) += 1;

					if **guard == 5 {
						comptx.send(())?;
					}
				}
				Ok(())
			})?;
		}

		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[0].send(())?;
		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[1].send(())?;
		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[2].send(())?;
		v[3].send(())?;
		v[4].send(())?;
		comprx.recv()?;
		assert_eq!(rlock!(complete_count), 5);
		sleep(Duration::from_millis(1_000));
		assert_eq!(tp.size()?, 2);

		Ok(())
	}

	#[test]
	fn test_thread_pool_resources_w_panic() -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(2), MaxSize(3))?;
		let mut panic_count = lock_box!(0)?;
		let panic_count_clone = panic_count.clone();
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> {
			wlock!(panic_count) += 1;
			Ok(())
		})?;
		tp.start()?;
		assert_eq!(tp.size()?, 2);

		let count = lock_box!(0)?;
		let complete_count = lock_box!(0)?;

		let (tx, rx) = sync_channel(1);
		let mut v = vec![];
		let (comptx, comprx) = sync_channel(1);

		for _ in 0..5 {
			let mut count = count.clone();
			let mut complete_count = complete_count.clone();
			let comptx = comptx.clone();
			let tx = tx.clone();
			let (tx2, rx2) = sync_channel(1);
			v.push(tx2);
			execute!(tp, {
				let localcount;
				{
					let mut count = count.wlock()?;
					let guard = count.guard();
					localcount = **guard;
					(**guard) += 1;
				}

				info!("i={}", localcount)?;

				if localcount >= 2 {
					info!("sending")?;
					tx.send(())?;
				}

				rx2.recv()?;
				info!("ending thread {}", localcount)?;

				{
					let mut count = complete_count.wlock()?;
					let guard = count.guard();
					(**guard) += 1;

					if **guard == 5 {
						comptx.send(())?;
					}
				}

				if true {
					// to avoid compiler warning
					panic!("panic");
				}
				Ok(())
			})?;
		}

		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[0].send(())?;
		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[1].send(())?;
		rx.recv()?;
		assert_eq!(tp.size()?, 3);
		v[2].send(())?;
		v[3].send(())?;
		v[4].send(())?;
		comprx.recv()?;
		assert_eq!(rlock!(complete_count), 5);
		sleep(Duration::from_millis(1_000));
		assert_eq!(rlock!(panic_count_clone), 5);
		assert_eq!(tp.size()?, 2);

		tp.executor()?.execute(async move { Ok(()) }, 0)?;

		Ok(())
	}

	#[test]
	fn test_panic_id() -> Result<(), Error> {
		let (tx, rx) = sync_channel(1);
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			let e_as_str = e.downcast_ref::<&str>().unwrap();
			assert_eq!(e_as_str, &"67890");
			assert_eq!(id, 12345);
			info!("id={},e={}", id, e_as_str)?;

			tx.send(())?;

			Ok(())
		})?;
		tp.start()?;

		tp.executor()?.execute(
			async move {
				if true {
					panic!("67890");
				}
				Ok(())
			},
			12345,
		)?;

		rx.recv()?;
		Ok(())
	}

	fn build_tp(min: usize, max: usize, sync: usize) -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(min), MaxSize(max), SyncChannelSize(sync))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		tp.start()?;

		execute!(tp, {
			info!("in execute")?;
			Ok(())
		})?;

		assert!(tp.stopper().is_ok());
		assert!(tp.stopper()?.stop().is_ok());

		Ok(())
	}

	#[test]
	fn test_threadpool_bad_configs() -> Result<(), Error> {
		assert!(build_tp(1, 1, 1).is_ok());
		assert!(build_tp(1, 1, 0).is_err());
		assert!(build_tp(0, 1, 1).is_err());
		assert!(build_tp(1, 0, 1).is_err());
		assert!(build_tp(2, 1, 1).is_err());
		Ok(())
	}

	#[test]
	fn test_lock_guard() -> Result<(), Error> {
		let y = Arc::new(RwLock::new(0));
		let y_guard = y.read()?.clone();
		let guard = y.read()?;
		let x = RwLockReadGuardWrapper {
			guard,
			id: 0,
			debug_err: false,
		};
		assert_eq!(**(x.guard()), (y_guard));
		Ok(())
	}

	#[test]
	fn test_canonicalize_path() -> Result<(), Error> {
		let test_info = test_info!()?;
		let directory = test_info.directory();
		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("test.txt");
		let mut file = File::create(path_buf)?;
		file.write_all(b"Hello, world!")?;
		assert!(canonicalize_base_path(&"".to_string(), &"".to_string()).is_err());
		assert!(canonicalize_base_path(&directory, &"/test.txt".to_string()).is_ok());

		let mut path_buf = PathBuf::new();
		path_buf.push(directory);
		path_buf.push("test");
		create_dir_all(path_buf.clone())?;

		assert!(canonicalize_base_path(
			&path_buf.to_str().unwrap().to_string(),
			&"/../test.txt".to_string(),
		)
		.is_err());

		DEBUG_INVALID_PATH.with(|f| -> Result<(), Error> {
			(*f.borrow_mut()) = true;
			Ok(())
		})?;

		assert!(canonicalize_base_path(&directory, &"/test.txt".to_string()).is_err());

		DEBUG_INVALID_PATH.with(|f| -> Result<(), Error> {
			(*f.borrow_mut()) = false;
			Ok(())
		})?;
		Ok(())
	}

	#[test]
	fn test_patterns() -> Result<(), Error> {
		assert!(Pattern::new(vec![
			Regex("test".to_string()),
			PatternId(0),
			IsCaseSensitive(true),
			IsTerminationPattern(true)
		])
		.is_err());

		let pattern1 = pattern!(
			Regex("test1".to_string()),
			PatternId(random()),
			IsCaseSensitive(false),
			IsTerminationPattern(false),
			IsMultiLine(false)
		)?;
		let pattern2 = pattern!(
			Regex("test2".to_string()),
			PatternId(random()),
			IsCaseSensitive(true)
		)?;
		let pattern3 = pattern!(
			Regex("test3".to_string()),
			PatternId(random()),
			IsMultiLine(true)
		)?;
		let pattern4 = pattern!(
			Regex("test4".to_string()),
			PatternId(random()),
			IsTerminationPattern(true)
		)?;

		ser_helper(pattern1)?;
		ser_helper(pattern2)?;
		ser_helper(pattern3)?;
		ser_helper(pattern4)?;

		Ok(())
	}

	#[test]
	fn test_hashtable_capacity() -> Result<(), Error> {
		// 6 bytes overhead
		let mut hashtable = hashtable!(GlobalSlabAllocator(false), SlabSize(14), SlabCount(1))?;
		hashtable.insert(&1u32, &1u32)?;
		assert!(hashtable.insert(&2u32, &2u32).is_err());
		hashtable.remove(&1u32)?;
		hashtable.insert(&2u32, &2u32)?;

		// test max entries
		let mut hashtable = hashtable!(
			GlobalSlabAllocator(false),
			SlabSize(1_000),
			SlabCount(1_000),
			MaxEntries(10)
		)?;

		for i in 0..10 {
			let i_u32 = i as u32;
			hashtable.insert(&i_u32, &i_u32)?;
		}

		assert!(hashtable.insert(&100u32, &200u32).is_err());

		Ok(())
	}

	fn build_hash(vec: Vec<ConfigOption>) -> Result<(), Error> {
		let mut h = HashImpl::new(vec)?;
		h.push(0)?;
		Ok(())
	}

	#[test]
	fn test_invalid_hash_impl_confgs() -> Result<(), Error> {
		assert!(UtilBuilder::build_list::<u8>(vec![ConfigOption::MaxEntries(10)]).is_err());
		assert!(UtilBuilder::build_list::<u8>(vec![ConfigOption::MaxLoadFactor(0.9)]).is_err());

		assert!(build_hash(vec![]).is_err());
		assert!(build_hash(vec![IsHashtable(true), IsHashset(true)]).is_err());
		assert!(build_hash(vec![IsList(true), IsHashset(true)]).is_err());
		assert!(build_hash(vec![IsHashtable(true), SlabSize(100)]).is_err());
		assert!(build_hash(vec![IsHashtable(true), SlabSize(100), SlabCount(10)]).is_err());
		assert!(build_hash(vec![IsHashtable(true), GlobalSlabAllocator(false)]).is_err());
		Ok(())
	}
}
