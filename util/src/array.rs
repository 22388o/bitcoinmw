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

use crate::types::{
	Array, ArrayIterator, ArrayList, ArrayListIterator, Direction, List, Queue, SortableList, Stack,
};
use bmw_err::*;
use bmw_ser::Serializable;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

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
