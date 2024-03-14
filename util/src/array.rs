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

use crate::types::ArrayListIterator;
use crate::types::Direction;
use crate::{Array, ArrayIterator, ArrayList, List, Queue, Serializable, SortableList, Stack};
use bmw_err::{err, Error};
use bmw_log::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

info!();

impl<T: Clone> Array<T> {
	pub(crate) fn new(size: usize, d: &T) -> Result<Self, Error> {
		if size == 0 {
			return Err(err!(ErrKind::IllegalArgument, "size must not be 0"));
		}
		let mut data = Vec::with_capacity(size);
		data.resize(size, d.clone());

		let ret = Self { data };
		Ok(ret)
	}

	pub fn size(&self) -> usize {
		self.data.len()
	}

	pub fn as_slice<'a>(&'a self) -> &'a [T] {
		&self.data
	}

	pub fn as_mut<'a>(&'a mut self) -> &'a mut [T] {
		&mut self.data
	}
	pub fn iter<'a>(&'a self) -> ArrayIterator<'a, T> {
		ArrayIterator {
			cur: 0,
			array_ref: self,
		}
	}
}

unsafe impl<T> Send for Array<T> where T: Send {}

unsafe impl<T> Sync for Array<T> where T: Sync {}

impl<T> Debug for Array<T>
where
	T: Debug,
{
	fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		for i in 0..self.data.len() {
			if i == 0 {
				write!(fmt, "[{:?}", self[i])?;
			} else {
				write!(fmt, ", {:?}", self[i])?;
			}
		}
		write!(fmt, "]")?;
		Ok(())
	}
}

impl<T> PartialEq for Array<T>
where
	T: PartialEq,
{
	fn eq(&self, rhs: &Self) -> bool {
		let data_len = self.data.len();
		if data_len != rhs.data.len() {
			false
		} else {
			for i in 0..data_len {
				if self.data[i] != rhs.data[i] {
					return false;
				}
			}
			true
		}
	}
}

impl<T> Clone for Array<T>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		Self {
			data: self.data.clone(),
		}
	}
}

impl<T> IndexMut<usize> for Array<T> {
	fn index_mut(&mut self, index: usize) -> &mut <Self as Index<usize>>::Output {
		if index >= self.data.len() {
			panic!("ArrayIndexOutOfBounds: {} >= {}", index, self.data.len());
		}
		&mut self.data[index]
	}
}

impl<T> Index<usize> for Array<T> {
	type Output = T;
	fn index(&self, index: usize) -> &<Self as Index<usize>>::Output {
		if index >= self.data.len() {
			panic!("ArrayIndexOutOfBounds: {} >= {}", index, self.data.len());
		}
		&self.data[index]
	}
}

unsafe impl<T> Send for ArrayList<T> where T: Send {}

unsafe impl<T> Sync for ArrayList<T> where T: Sync {}

impl<T> ArrayList<T>
where
	T: Clone,
{
	pub(crate) fn new(size: usize, default: &T) -> Result<Self, Error> {
		if size == 0 {
			return Err(err!(ErrKind::IllegalArgument, "size must not be 0"));
		}
		let inner = Array::new(size, default)?;
		let ret = Self {
			inner,
			size: 0,
			head: 0,
			tail: 0,
		};
		Ok(ret)
	}
}

impl<T> SortableList<T> for ArrayList<T>
where
	T: Clone + Debug + Serializable + PartialEq,
{
	fn sort(&mut self) -> Result<(), Error>
	where
		T: Ord,
	{
		let size = self.size();
		self.inner.as_mut()[0..size].sort();
		Ok(())
	}
	fn sort_unstable(&mut self) -> Result<(), Error>
	where
		T: Ord,
	{
		let size = self.size();
		self.inner.as_mut()[0..size].sort_unstable();
		Ok(())
	}
}

impl<T> Debug for ArrayList<T>
where
	T: Debug + Clone,
{
	fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		let size = self.size;
		write!(fmt, "{:?}", &self.inner.as_slice()[0..size])
	}
}

impl<T> PartialEq for ArrayList<T>
where
	T: PartialEq,
{
	fn eq(&self, rhs: &ArrayList<T>) -> bool {
		for i in 0..self.size {
			if self.inner[i] != rhs.inner[i] {
				return false;
			}
		}

		true
	}
}

impl<T> List<T> for ArrayList<T>
where
	T: Clone + Debug + PartialEq,
{
	fn push(&mut self, value: T) -> Result<(), Error> {
		if self.size() >= self.inner.size() {
			let fmt = format!("ArrayList capacity exceeded: {}", self.inner.size());
			return Err(err!(ErrKind::CapacityExceeded, fmt));
		}
		self.inner[self.size] = value;
		self.size += 1;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a>
	where
		T: Serializable,
	{
		let ret = ArrayListIterator {
			array_list_ref: &self,
			cur: 0,
			direction: Direction::Forward,
		};
		Box::new(ret)
	}

	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn iter_rev<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a>
	where
		T: Serializable,
	{
		let ret = ArrayListIterator {
			array_list_ref: &self,
			cur: self.size.saturating_sub(1),
			direction: Direction::Backward,
		};
		Box::new(ret)
	}
	fn delete_head(&mut self) -> Result<(), Error> {
		let fmt = "arraylist doesn't support delete_head";
		return Err(err!(ErrKind::OperationNotSupported, fmt));
	}
	fn size(&self) -> usize {
		self.size
	}
	fn clear(&mut self) -> Result<(), Error> {
		self.size = 0;
		Ok(())
	}
}

impl<T> Queue<T> for ArrayList<T>
where
	T: Clone,
{
	fn enqueue(&mut self, value: T) -> Result<(), Error> {
		if self.size == self.inner.size() {
			let fmt = format!("capacity ({}) exceeded.", self.inner.size());
			Err(err!(ErrKind::CapacityExceeded, fmt))
		} else {
			self.inner[self.tail] = value;
			self.tail = (self.tail + 1) % self.inner.size();
			self.size += 1;
			Ok(())
		}
	}
	fn dequeue(&mut self) -> Option<&T> {
		if self.size == 0 {
			None
		} else {
			let ret = &self.inner[self.head];
			self.head = (self.head + 1) % self.inner.size();
			self.size = self.size.saturating_sub(1);
			Some(ret)
		}
	}
	fn peek(&self) -> Option<&T> {
		if self.size == 0 {
			None
		} else {
			Some(&self.inner[self.head])
		}
	}
	fn length(&self) -> usize {
		self.size
	}
}

impl<T> Stack<T> for ArrayList<T>
where
	T: Clone,
{
	fn push(&mut self, value: T) -> Result<(), Error> {
		if self.size == self.inner.size() {
			let fmt = format!("capacity ({}) exceeded.", self.inner.size());
			Err(err!(ErrKind::CapacityExceeded, fmt))
		} else {
			self.inner[self.tail] = value;
			self.tail = (self.tail + 1) % self.inner.size();
			self.size += 1;
			Ok(())
		}
	}
	fn pop(&mut self) -> Option<&T> {
		if self.size == 0 {
			None
		} else {
			if self.tail == 0 {
				self.tail = self.inner.size().saturating_sub(1);
			} else {
				self.tail = self.tail - 1;
			}
			let ret = &self.inner[self.tail];
			self.size = self.size.saturating_sub(1);
			Some(ret)
		}
	}
	fn peek(&self) -> Option<&T> {
		if self.size == 0 {
			None
		} else {
			Some(&self.inner[self.tail.saturating_sub(1)])
		}
	}
	fn length(&self) -> usize {
		self.size
	}
}

impl<'a, T> Iterator for ArrayListIterator<'a, T>
where
	T: Clone,
{
	type Item = T;
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		if self.array_list_ref.size == 0 {
			None
		} else if self.direction == Direction::Forward && self.cur >= self.array_list_ref.size {
			None
		} else if self.direction == Direction::Backward && self.cur <= 0 {
			None
		} else {
			let ret = Some(self.array_list_ref.inner[self.cur].clone());
			if self.direction == Direction::Forward {
				self.cur += 1;
			} else {
				self.cur = self.cur.saturating_sub(1);
			}
			ret
		}
	}
}

impl<'a, T> Iterator for ArrayIterator<'a, T>
where
	T: Clone,
{
	type Item = &'a T;
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		if self.cur >= self.array_ref.size() {
			None
		} else {
			self.cur += 1;
			Some(&self.array_ref[self.cur - 1])
		}
	}
}

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::{
		array, block_on, execute, list, list_eq, lock, queue_box, thread_pool, Array, ArrayList,
		Builder, List, Lock, PoolResult, Queue, SortableList, Stack, ThreadPool,
	};
	use bmw_deps::dyn_clone::clone_box;
	use bmw_deps::rand::random;
	use bmw_deps::random_string;
	use bmw_err::*;
	use bmw_log::*;

	info!();

	struct TestStruct {
		arr: Array<u32>,
	}

	#[test]
	fn test_array_simple() -> Result<(), Error> {
		let mut arr = Builder::build_array(10, &0)?;
		for i in 0..10 {
			arr[i] = i as u64;
		}
		for i in 0..10 {
			info!("arr[{}]={}", i, arr[i])?;
			assert_eq!(arr[i], i as u64);
		}

		let mut test = TestStruct {
			arr: Builder::build_array(40, &0)?,
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

		assert!(Builder::build_array::<u8>(0, &0).is_err());

		Ok(())
	}

	#[test]
	fn test_array_iterator() -> Result<(), Error> {
		let mut arr = Builder::build_array(10, &0)?;
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

		let handle = execute!(tp, {
			let mut x = Builder::build_array(10, &0)?;
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
			let mut x = Builder::build_array(10, &0)?;
			for i in 0..10 {
				x[i] = i;
			}
			Ok(())
		})?;

		assert_eq!(block_on!(handle), PoolResult::Ok(()));

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

		let handle = execute!(tp, {
			let mut x = Builder::build_array(10, &0)?;
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
		let mut arr1 = Builder::build_array(10, &0)?;
		let mut arr2 = Builder::build_array(11, &0)?;

		for i in 0..10 {
			arr1[i] = 7;
		}

		for i in 0..11 {
			arr2[i] = 7;
		}

		assert_ne!(arr1, arr2);

		let mut arr3 = Builder::build_array(10, &0)?;
		for i in 0..10 {
			arr3[i] = 8;
		}

		assert_ne!(arr3, arr1);

		let mut arr4 = Builder::build_array(10, &0)?;
		for i in 0..10 {
			arr4[i] = 7;
		}

		assert_eq!(arr4, arr1);

		let mut arr5 = Builder::build_array(20, &0)?;
		let mut arr6 = Builder::build_array(20, &0)?;

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
		let mut list1 = Builder::build_array_list(10, &0)?;
		let mut list2 = Builder::build_array_list(10, &0)?;

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

		let mut list3 = Builder::build_array_list(10, &0)?;

		for i in 0..5 {
			list3.push(i)?;
		}

		let mut list = Builder::build_array_list(50, &0)?;

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

		let mut list = Builder::build_array_list(5, &0)?;
		for _ in 0..5 {
			list.push(1)?;
		}
		assert!(list.push(1).is_err());
		assert!(list.delete_head().is_err());

		assert!(Builder::build_array_list::<u8>(0, &0).is_err());

		Ok(())
	}

	#[test]
	fn test_as_slice_mut() -> Result<(), Error> {
		let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let mut array = Builder::build_array(data.len(), &0)?;
		array.as_mut().clone_from_slice(&data);

		assert_eq!(array[3], 3u8);
		assert_eq!(array.as_slice()[4], 4u8);
		Ok(())
	}

	#[test]
	fn test_queue() -> Result<(), Error> {
		let mut queue = Builder::build_queue(10, &0)?;

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
		let mut stack = Builder::build_stack(10, &0)?;

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
		let mut array = Builder::build_array(10, &0)?;
		array[0] = 1;

		let mut lock = lock!(array)?;
		let lock_clone = lock.clone();

		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

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
		let queue = Builder::build_queue_box(10, &0)?;
		let mut test = TestBoxedQueue { queue };
		test.queue.enqueue(1)?;
		Ok(())
	}

	#[test]
	fn test_queue_clone() -> Result<(), Error> {
		let queue = Builder::build_queue_box(10, &0)?;
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
		let mut list = Builder::build_array_list(10, &0)?;

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
