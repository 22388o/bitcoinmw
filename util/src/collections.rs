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

use crate::array::*;
use bmw_core::*;
use bmw_log::*;
use std::marker::PhantomData;

debug!();

pub struct Iterator<'a, K>
where
	K: Serializable,
{
	_hash_class: &'a Collection<'a, K>,
	_cur: usize,
	_phantom_data: PhantomData<K>,
}

impl<'a, K> std::iter::Iterator for Iterator<'a, K>
where
	K: Serializable,
{
	type Item = K;
	fn next(&mut self) -> Option<<Self as std::iter::Iterator>::Item> {
		/*
		match self.hashtable.get_next(&mut self.cur) {
				Ok(x) => x,
				Err(e) => {
						let _ = error!("get_next generated unexpected error: {}", e);
						None
				}
		}
				*/

		todo!()
	}
}

#[class {
		var phantom_data: PhantomData<&'a K>;
		generic hashtable: <'a, K, V> where K: Serializable + 'a, V: Serializable;
		pub list as list_impl;
		var hash_array: Option<Box<dyn Array<usize> + Send + Sync>>;

		[hashtable]
		fn insert(&mut self, key: K, value: V) -> Result<(), Error> as hashtable_insert;

		[hashset]
		fn insert(&mut self, key: K) -> Result<(), Error> as hashset_insert;

		[hashtable]
		fn get(&self, key: K) -> Result<Option<V>, Error>;

		[hashset]
		fn contains(&self, key: K) -> Result<bool, Error>;

		[hashtable]
		fn delete(&mut self, key: K) -> Result<Option<V>, Error> as hashtable_delete;

		[hashset]
		fn delete(&mut self, key: K) -> Result<bool, Error> as hashset_delete;

		[list]
		fn push(&mut self, value: K) -> Result<(), Error>;

		[hashtable, hashset, list]
		fn iter(&self) -> Iterator<K>;

		[hashtable, hashset, list]
		fn clear(&mut self) -> Result<(), Error>;
}]
impl<'a, K> Collection<'a, K> where K: Serializable + 'a {}

impl<'a, K> CollectionVarBuilder for CollectionVar<'a, K>
where
	K: Serializable + 'a,
{
	fn builder(constants: &CollectionConst) -> Result<Self, Error> {
		let name = constants.get_name();
		let hash_array = if name == "hashtable" || name == "hashset" {
			Some(array_sync_box!(Len(100), BytesPerEntry(30))?)
		} else {
			None
		};

		info!("name={}", constants.get_name())?;
		Ok(Self {
			phantom_data: PhantomData,
			hash_array,
		})
	}
}

impl<'a, K> Collection<'a, K>
where
	K: Serializable + 'a,
{
	fn hashtable_insert<V>(&mut self, key: K, value: V) -> Result<(), Error>
	where
		V: Serializable,
	{
		self.insert_key(key)?;
		self.insert_value(value)?;
		Ok(())
	}

	fn hashset_insert(&mut self, key: K) -> Result<(), Error> {
		self.insert_key(key)?;
		Ok(())
	}

	fn insert_key(&mut self, s: K) -> Result<(), Error> {
		Ok(())
	}

	fn insert_value<V>(&mut self, s: V) -> Result<(), Error>
	where
		V: Serializable,
	{
		Ok(())
	}

	fn push(&mut self, _value: K) -> Result<(), Error> {
		Ok(())
	}

	fn iter(&self) -> Iterator<K> {
		todo!()
	}

	fn clear(&mut self) -> Result<(), Error> {
		todo!()
	}

	fn hashtable_delete<V>(&mut self, _key: K) -> Result<Option<V>, Error>
	where
		V: Serializable,
	{
		todo!()
	}

	fn hashset_delete(&mut self, _key: K) -> Result<bool, Error> {
		todo!()
	}

	fn contains(&self, _key: K) -> Result<bool, Error> {
		todo!()
	}

	fn get<V>(&self, _key: K) -> Result<Option<V>, Error>
	where
		V: Serializable,
	{
		todo!()
	}
}

#[macro_export]
macro_rules! list {
	($( $x:expr ),*) => {{
                let mut ret = list_impl!()?;
                $(
                    ret.push($x)?;
                )*

                ret
	}};
}

#[cfg(test)]
mod test {
	use super::*;
	use std::sync::{Arc, RwLock};

	#[test]
	fn test_hash_list() -> Result<(), Error> {
		let mut hashtable = hashtable_box!()?;
		let mut hashset = hashset!()?;
		let mut list = list!["dd".to_string(), "ee".to_string()];
		let mut hashtable2 = hashtable!()?;

		hashtable2.insert("test".to_string(), 1usize)?;

		hashtable.insert(&0usize, &1usize)?;
		hashset.insert(&0usize)?;
		list.push("ok".to_string())?;

		let x = Arc::new(RwLock::new(hashset));
		let x_clone = x.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			let mut v = x.write()?;
			(*v).insert(&3usize)?;
			Ok(())
		});

		std::thread::sleep(std::time::Duration::from_millis(1000));

		let mut t = x_clone.write()?;
		(*t).insert(&4usize)?;

		/*
		let mut hashtable: Box<dyn Hashtable<String, String>> = Box::new(Hash::new());
		hashtable.insert(&"test".to_string(), &"abc".to_string())?;

		let mut hashset: Box<dyn Hashset<String>> = Box::new(Hash::new());
		hashset.insert(&"aaaa".to_string())?;
			*/
		//let mut hashtable = hashtable!()?;
		//hashtable.insert(&0, &1)?;

		//let mut list = list!()?;
		//list.push(&0)?;
		//let test: Box<dyn Z> = Box::new(y);
		//test.v(10);
		/*
				let b = 'b';
				{
					let mut hashtable = hashtable!()?;
					{
						let a = 'a';
						hashtable.insert(&a, &b)?;
					}
					hashtable.remove(&'a')?;

					let itt = hashtable.iterator()?;
				}

				let mut list = list!()?;
				list.push(&1)?;
		*/
		Ok(())
	}
}
