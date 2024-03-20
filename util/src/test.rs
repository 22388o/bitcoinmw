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
	use crate::types::{HashImpl, HashImplSync};
	use bmw_deps::dyn_clone::clone_box;
	use bmw_deps::rand::random;
	use bmw_deps::random_string;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_ser::{Reader, Serializable, Writer};
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
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

		assert_eq!(x, x2);
		x2.push(1)?;
		assert_ne!(x, x2);
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

		let mut hash_impl2: HashImpl<SerErr> = HashImpl::new(vec![IsList(true)])?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl.push(SerErr { exp: 99, empty: 0 })?;

		// lengths unequal
		assert_ne!(hash_impl, hash_impl2);

		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;
		hash_impl2.push(SerErr { exp: 99, empty: 0 })?;

		// now contents are equal
		assert_eq!(hash_impl, hash_impl2);

		let mut hash_impl: HashImpl<u32> = HashImpl::new(vec![IsList(true)])?;
		let mut hash_impl2: HashImpl<u32> = HashImpl::new(vec![IsList(true)])?;
		hash_impl2.push(1)?;
		hash_impl.push(8)?;

		// the value is not equal
		assert_ne!(hash_impl, hash_impl2);

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
		let pattern = pattern!(Regex("abc".to_string()), PatternId(0))?;
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
}
