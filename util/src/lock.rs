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

use bmw_core::rand::random;
use bmw_core::*;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use LockErrors::*;

#[ErrorKind]
pub enum LockErrors {
	Poison,
}

#[macro_export]
macro_rules! lock_box {
	($value:expr) => {{
		build_lock_box($value)
	}};
}
#[cfg(test)]
pub(crate) use lock_box;

pub fn build_lock_box<T>(t: T) -> Box<dyn LockBox<T>>
where
	T: Send + Sync + 'static,
{
	Box::new(LockImpl::new(t))
}

pub trait LockBox<T>: Send + Sync + Debug
where
	T: Send + Sync,
{
	/// obtain a write lock and corresponding [`std::sync::RwLockWriteGuard`] for this
	/// [`crate::LockBox`].
	fn wlock(&mut self) -> Result<RwLockWriteGuard<'_, T>, Error>;
	/// obtain a read lock and corresponding [`std::sync::RwLockReadGuard`] for this
	/// [`crate::LockBox`].
	fn rlock(&self) -> Result<RwLockReadGuard<'_, T>, Error>;
	/// Same as [`crate::LockBox::wlock`] except that any poison errors are ignored
	/// by calling the underlying into_inner() fn.
	fn wlock_ignore_poison(&mut self) -> Result<RwLockWriteGuard<'_, T>, Error>;
	/// Same as [`crate::LockBox::rlock`] except that any poison errors are ignored
	/// by calling the underlying into_inner() fn.
	fn rlock_ignore_poison(&self) -> Result<RwLockReadGuard<'_, T>, Error>;
	/// return the inner data holder.
	fn inner(&self) -> Arc<RwLock<T>>;
	/// return the id for this lockbox.
	fn id(&self) -> u128;
}

#[derive(Clone)]
struct LockImpl<T> {
	t: Arc<RwLock<T>>,
	id: u128,
}

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

impl<T> Debug for LockImpl<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "LockImpl<{}>", self.id)
	}
}

impl<T> LockBox<T> for LockImpl<T>
where
	T: Send + Sync,
{
	fn wlock(&mut self) -> Result<RwLockWriteGuard<'_, T>, Error> {
		self.do_wlock(false)
	}

	fn rlock(&self) -> Result<RwLockReadGuard<'_, T>, Error> {
		self.do_rlock(false)
	}

	fn rlock_ignore_poison(&self) -> Result<RwLockReadGuard<'_, T>, Error> {
		self.do_rlock(true)
	}

	fn wlock_ignore_poison(&mut self) -> Result<RwLockWriteGuard<'_, T>, Error> {
		self.do_wlock(true)
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

	fn do_wlock(&mut self, ignore_poison: bool) -> Result<RwLockWriteGuard<'_, T>, Error> {
		let guard = if ignore_poison {
			match self.t.write() {
				Ok(guard) => guard,
				Err(e) => e.into_inner(),
			}
		} else {
			map_err!(self.t.write(), Poison)?
		};
		Ok(guard)
	}

	fn do_rlock(&self, ignore_poison: bool) -> Result<RwLockReadGuard<'_, T>, Error> {
		let guard = if ignore_poison {
			match self.t.read() {
				Ok(guard) => guard,
				Err(e) => e.into_inner(),
			}
		} else {
			map_err!(self.t.read(), Poison)?
		};
		Ok(guard)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bmw_test::*;

	#[test]
	fn test_lock_box() -> Result<(), Error> {
		let test = test_info!()?;
		let mut lock = lock_box!(5);
		let lock_clone = lock.clone();

		let (tx, rx) = test.sync_channel();

		std::thread::spawn(move || -> Result<(), Error> {
			let mut guard = lock.wlock()?;
			*guard += 1;

			tx.send(())?;

			Ok(())
		});

		rx.recv()?;
		let guard = lock_clone.rlock()?;
		assert_eq!(*guard, 6);

		Ok(())
	}
}
