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
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use ArrayErrors::*;

debug!();

#[ErrorKind]
pub enum ArrayErrors {
	IllegalArgument,
	TryReserve,
}

#[class {
    no_send;
    var_in len: usize;
    var phantom_data: PhantomData<&'a T>;

    [array]
    fn len(&self) -> usize;

    [array]
    fn get(&self, index: usize) -> &T;

    [array]
    fn get_mut(&mut self, index: usize) -> &mut T;
}]
impl<'a, T> ArrayClass<'a, T> where T: Clone + 'a {}

impl<'a, T> ArrayClassVarBuilder for ArrayClassVar<'a, T>
where
	T: Clone + 'a,
{
	fn builder(constants: &ArrayClassConst) -> Result<Self, Error> {
		let mut len = 0;
		let _init_value: Option<T> = None;
		for passthrough in &constants.passthroughs {
			if passthrough.name == "len" {
				debug!("found size")?;
				match passthrough.value.downcast_ref::<usize>() {
					Ok(l) => {
						len = *l;
					}
					_ => {}
				}
			}
		}

		if len == 0 {
			err!(IllegalArgument, "Len must be specified and non-zero")
		} else {
			Ok(Self {
				len,
				phantom_data: PhantomData,
			})
		}
	}
}

impl<'a, T> ArrayClass<'a, T>
where
	T: Clone + 'a,
{
	fn len(&self) -> usize {
		*self.vars().get_len()
	}

	fn get(&self, _index: usize) -> &T {
		todo!()
	}

	fn get_mut(&mut self, _index: usize) -> &mut T {
		todo!()
	}
}

impl<'a, T> Debug for dyn Array<'a, T> {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		Ok(())
	}
}

impl<'a, T> IndexMut<usize> for dyn Array<'a, T>
where
	T: Clone + 'a,
{
	fn index_mut(&mut self, index: usize) -> &mut <Self as Index<usize>>::Output {
		self.get_mut(index)
	}
}

impl<'a, T> Index<usize> for dyn Array<'a, T>
where
	T: Clone + 'a,
{
	type Output = T;
	fn index(&self, index: usize) -> &<Self as Index<usize>>::Output {
		self.get(index)
	}
}

#[cfg(test)]
mod test {
	/*
	#[test]
	fn test_array() -> Result<(), Error> {
		let mut array = array_box!(Len(10))?;
		array[0] = 1usize;
		debug!("size={}", array.len())?;

		assert_eq!(array.len(), 10);

		Ok(())
	}
		*/
}
