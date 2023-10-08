// Copyright (c) 2022, 37 Miners, LLC
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
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
use bmw_err::{err, map_err, ErrKind, Error};
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
	#[cfg(not(tarpaulin_include))]
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
		self.do_rlock()
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
		self.do_rlock()
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

	fn do_rlock(&self) -> Result<RwLockReadGuardWrapper<'_, T>, Error> {
		let contains = LOCKS.with(|f| -> Result<bool, Error> {
			let ret = (*f.borrow()).contains(&self.id);
			(*f.borrow_mut()).insert(self.id);

			Ok(ret)
		})?;
		if contains {
			Err(err!(ErrKind::Poison, "would deadlock"))
		} else {
			let guard = map_err!(self.t.read(), ErrKind::Poison)?;
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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::lock::Lock;
	use crate::lock::LockBox;
	use crate::Builder;
	use crate::{lock, lock_box, RwLockReadGuardWrapper, RwLockWriteGuardWrapper};
	use bmw_err::Error;
	use bmw_log::*;
	use std::sync::{Arc, RwLock};
	use std::thread::{sleep, spawn};
	use std::time::Duration;

	info!();

	#[test]
	fn test_locks() -> Result<(), Error> {
		let mut lock = Builder::build_lock(1)?;
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
		let mut lock = Builder::build_lock(1)?;
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
		Ok(())
	}

	#[test]
	fn test_lock_threads() -> Result<(), Error> {
		let mut lock = Builder::build_lock(1)?;
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
			let tlb = tlb.lock_box.rlock()?;
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

		Ok(())
	}
}
