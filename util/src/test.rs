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
	use bmw_deps::dyn_clone::clone_box;
	use bmw_deps::rand::random;
	use bmw_deps::random_string;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_util::*;

	info!();

	#[test]
	fn test_suffix_tree_macro() -> Result<(), Error> {
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

	#[test]
	fn test_slab_allocator_macro() -> Result<(), Error> {
		let _slabs = slab_allocator!(SlabSize(128), SlabCount(5000))?;
		Ok(())
	}

	#[test]
	fn test_hashtable_macro() -> Result<(), Error> {
		// create a hashtable with the specified parameters
		let mut hashtable = hashtable!(
			MaxEntries(1_000),
			MaxLoadFactor(0.9),
			GlobalSlabAllocator(false),
			SlabCount(100),
			SlabSize(100)
		)?;

		// do an insert, rust will figure out what type is being inserted
		hashtable.insert(&1, &2)?;

		// assert that the entry was inserted
		assert_eq!(hashtable.get(&1)?, Some(2));

		// create another hashtable with defaults, this time the global slab allocator will be
		// used. Since we did not initialize it default values will be used.
		let mut hashtable = hashtable!()?;

		// do an insert, rust will figure out what type is being inserted
		hashtable.insert(&1, &3)?;

		// assert that the entry was inserted
		assert_eq!(hashtable.get(&1)?, Some(3));

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
}
