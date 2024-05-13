// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_core::*;
use bmw_log::*;
use std::sync::{Arc, RwLock, RwLockReadGuard};

debug!();

#[class{
        var lock: Arc<RwLock<Option<T>>>;

        [lock]
        fn init(&mut self, t: T) -> Result<(), Error>;

        [lock]
        fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error>;
}]
impl<T> LockClass<T> where T: Send + Sync + 'static {}

impl<T> LockClassVarBuilder for LockClassVar<T>
where
	T: Send + Sync + 'static,
{
	fn builder(_constants: &LockClassConst) -> Result<Self, Error> {
		let lock = Arc::new(RwLock::new(None));
		Ok(Self { lock })
	}
}

impl<T> LockClass<T>
where
	T: Send + Sync + 'static,
{
	fn init(&mut self, t: T) -> Result<(), Error> {
		let v = self.vars_mut().get_mut_lock();
		let mut guard = v.write()?;
		*guard = Some(t);
		Ok(())
	}
	fn rlock(&self) -> Result<RwLockReadGuard<'_, Option<T>>, Error> {
		let v = self.vars().get_lock();
		let guard = v.read()?;
		Ok(guard)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_lock() -> Result<(), Error> {
		let mut lock = lock!()?;
		lock.init(0usize)?;
		Ok(())
	}
}
