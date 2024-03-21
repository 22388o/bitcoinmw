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

use crate::types::LockImpl;
use crate::{Lock, LockBox, RwLockReadGuardWrapper, RwLockWriteGuardWrapper};
use bmw_deps::rand::random;
use bmw_err::{err, map_err, Error};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

impl<T> Clone for Box<dyn LockBox<T>>
where
	T: Send + Sync + 'static,
{
	fn clone(&self) -> Self {
		Box::new(LockImpl {
			id: self.id(),
			t: self.inner().clone(),
		})
	}
}

#[cfg(not(tarpaulin_include))]
thread_local! {
	pub static LOCKS: RefCell<HashSet<u128>> = RefCell::new(HashSet::new());
}

/// Rebuild a [`crate::LockBox`] from te usize which is returned from the
/// [`crate::LockBox::danger_to_usize`] function.
pub fn lock_box_from_usize<T>(value: usize) -> Box<dyn LockBox<T> + Send + Sync>
where
	T: Send + Sync + 'static,
{
	let t = unsafe { Arc::from_raw(value as *mut RwLock<T>) };
	Box::new(LockImpl { id: random(), t })
}

impl<'a, T> RwLockReadGuardWrapper<'a, T>
where
	T: Send + Sync,
{
	/// Return the RwLockReadGuard associated with this lock.
	#[cfg(not(tarpaulin_include))]
	pub fn guard(&self) -> &RwLockReadGuard<'a, T> {
		&self.guard
	}
}

impl<T> Drop for RwLockReadGuardWrapper<'_, T> {
	fn drop(&mut self) {
		let id = self.id;

		let res = LOCKS.with(|f| -> Result<(), Error> {
			(*f.borrow_mut()).remove(&id);
			Ok(())
		});
		if res.is_err() || self.debug_err {
			println!("error dropping read lock: {:?}", res);
		}
	}
}

impl<'a, T> RwLockWriteGuardWrapper<'a, T> {
	/// Return the RwLockWriteGuard associated with this lock.
	pub fn guard(&mut self) -> &mut RwLockWriteGuard<'a, T> {
		&mut self.guard
	}
}

impl<T> Drop for RwLockWriteGuardWrapper<'_, T> {
	fn drop(&mut self) {
		let id = self.id;

		let res = LOCKS.with(|f| -> Result<(), Error> {
			(*f.borrow_mut()).remove(&id);
			Ok(())
		});

		if res.is_err() || self.debug_err {
			println!("error dropping write lock: {:?}", res);
		}
	}
}

impl<T> Debug for LockImpl<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "LockImpl<{}>", self.id)
	}
}

impl<T> Lock<T> for LockImpl<T>
where
	T: Send + Sync,
{
	fn wlock(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error> {
		self.do_wlock(false)
	}

	fn rlock(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error> {
		self.do_rlock(false)
	}

	fn clone(&self) -> Self {
		Self {
			t: self.t.clone(),
			id: self.id,
		}
	}
}

impl<T> LockBox<T> for LockImpl<T>
where
	T: Send + Sync,
{
	fn wlock(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error> {
		self.do_wlock(false)
	}

	fn rlock(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error> {
		self.do_rlock(false)
	}

	fn rlock_ignore_poison(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error> {
		self.do_rlock(true)
	}

	fn wlock_ignore_poison(&mut self) -> Result<RwLockWriteGuardWrapper<'_, T>, Error> {
		self.do_wlock(true)
	}

	fn danger_to_usize(&self) -> usize {
		Arc::into_raw(self.t.clone()) as usize
	}

	fn inner(&self) -> Arc<RwLock<T>> {
		self.t.clone()
	}

	fn id(&self) -> u128 {
		self.id
	}
}

impl<T> LockImpl<T> {
	pub(crate) fn new(t: T) -> Self {
		Self {
			t: Arc::new(RwLock::new(t)),
			id: random(),
		}
	}

	fn do_wlock(&mut self, ignore_poison: bool) -> Result<RwLockWriteGuardWrapper<'_, T>, Error> {
		let contains = LOCKS.with(|f| -> Result<bool, Error> {
			let ret = (*f.borrow()).contains(&self.id);
			(*f.borrow_mut()).insert(self.id);

			Ok(ret)
		})?;
		if contains {
			Err(err!(ErrKind::Poison, "would deadlock"))
		} else {
			let guard = if ignore_poison {
				match self.t.write() {
					Ok(guard) => guard,
					Err(e) => e.into_inner(),
				}
			} else {
				map_err!(self.t.write(), ErrKind::Poison)?
			};
			let id = self.id;
			let debug_err = false;
			let ret = RwLockWriteGuardWrapper {
				guard,
				id,
				debug_err,
			};
			Ok(ret)
		}
	}

	fn do_rlock(&self, ignore_poison: bool) -> Result<RwLockReadGuardWrapper<'_, T>, Error> {
		let contains = LOCKS.with(|f| -> Result<bool, Error> {
			let ret = (*f.borrow()).contains(&self.id);
			(*f.borrow_mut()).insert(self.id);
			Ok(ret)
		})?;
		if contains {
			Err(err!(ErrKind::Poison, "would deadlock"))
		} else {
			let guard = if ignore_poison {
				match self.t.read() {
					Ok(guard) => guard,
					Err(e) => e.into_inner(),
				}
			} else {
				map_err!(self.t.read(), ErrKind::Poison)?
			};
			let id = self.id;
			let debug_err = false;
			let ret = RwLockReadGuardWrapper {
				guard,
				id,
				debug_err,
			};
			Ok(ret)
		}
	}
}
