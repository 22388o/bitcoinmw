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
use std::fmt::Debug;
use std::marker::PhantomData;

debug!();

/*
pub struct Iterator<'a, K>
where
	K: Serializable,
{
	hashtable: Option<&'a dyn Hashtable<'a, K>>,
	list: Option<&'a dyn List<'a, K>>,
	hashset: Option<&'a dyn Hashset<'a, K>>,
	cur: usize,
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

/// 1.) generics statement to define generics for the trait of a view which is totally independant
///   of the class, but if not specified, the class is used. This also includes the where clause
/// 2.) 'as' other_function_name;
#[class {
	clone hashtable, list, hashset;
	no_send;
	var phantom_data: PhantomData<&'a (K, V)>;
	generics hashtable: <K, V> where K: Serializable, V: Serializable;
	generics hashset: <K> where K: Serializable;
	generics list: <V> where V: Serializable;

	var v: Option<V>;

	[hashtable]
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error> as hashtable_insert;

	[hashset]
	fn insert(&mut self, key: &K) -> Result<(), Error> as hashset_insert;

	[hashtable]
	fn get<V>(&self, key: &K) -> Result<Option<V>, Error> where V: Serializable;

	[hashtable]
	fn remove<V>(&mut self, key: &K) -> Result<Option<V>, Error> where V: Serializable;

	[hashtable, hashset]
	fn clear(&mut self) -> Result<(), Error>;

	[hashset]
	fn contains(&self) -> Result<bool, Error>;

	[hashset, list]
	fn push(&mut self, key: &K) -> Result<(), Error>;

	[hashset]
	fn delete(&mut self, key: &K) -> Result<(), Error>;

	[hashtable, hashset, list]
	fn iterator(&self) -> Result<Iterator<K>, Error>;
}]
impl<'a, K> HashListClass<'a, K> where K: Serializable + Clone + 'a {}

impl<'a, K> HashListClassVarBuilder for HashListClassVar<'a, K>
where
	K: Serializable + Clone + 'a,
{
	fn builder(constants: &HashListClassConst) -> Result<Self, Error> {
		Ok(Self {
			phantom_data: PhantomData,
		})
	}
}

impl<'a, K> HashListClass<'a, K>
where
	K: Serializable + Clone + 'a,
{
		fn hashtable_insert<V>(&mut self, key: &K, value: &V) -> Result<(), Error> where V: Serializable + 'a {
			todo!()
		}

		fn hashset_insert(&mut self, key: &K) -> Result<(), Error> {
			todo!()
		}

	fn insert<V>(&mut self, key: &K, value: &V) -> Result<(), Error>
	where
		V: Serializable,
	{
		Ok(())
	}

	fn remove<V>(&mut self, key: &K) -> Result<Option<V>, Error>
	where
		V: Serializable,
	{
		Ok(None)
	}

	fn clear(&mut self) -> Result<(), Error> {
		Ok(())
	}

	fn push(&mut self, key: &K) -> Result<(), Error> {
		Ok(())
	}

	fn iterator(&self) -> Result<Iterator<K>, Error> {
		let ret = Iterator {
			hashtable: Some(self),
			hashset: Some(self),
			list: Some(self),
			cur: 0,
			_phantom_data: PhantomData,
		};
		Ok(ret)
	}
}
*/

/*

#[class{
	no_send;
	var phantom_data: PhantomData<&'a (K, V)>;
	var v: Option<V>;

	[hashtable]
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error>;

	[hashset, list]
	fn push(&mut self, key: &K) -> Result<(), Error>;
}]
impl<'a, K, V> TestDD<'a, K, V>
where
	K: Serializable + 'a,
	V: Serializable + 'a,
{
}
impl<'a, K, V> TestDDVarBuilder for TestDDVar<'a, K, V>
where
	K: Serializable + 'a,
	V: Serializable + 'a,
{
	fn builder(constants: &TestDDConst) -> Result<Self, Error> {
		Ok(Self {
			phantom_data: PhantomData,
			v: None,
		})
	}
}
impl<'a, K, V> TestDD<'a, K, V>
where
	K: Serializable + 'a,
	V: Serializable + 'a,
{
	fn push(&mut self, key: &K) -> Result<(), Error> {
		self.handle_annotation(key);
		Ok(())
	}

	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error> {
		Ok(())
	}

	fn handle_annotation<X>(&self, value: &X)
	where
		X: Serializable + 'a,
	{
	}
}
*/

/*
trait Hashset<'a, K>
where
	K: Serializable + 'a,
{
	fn insert(&mut self, key: &K) -> Result<(), Error>;
}

trait Hashtable<'a, K, V>
where
	K: Serializable + 'a,
	V: Serializable,
{
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error>;
}

struct Hash<'a, K>
where
	K: Serializable,
{
	phantom_data: PhantomData<&'a K>,
}

impl<'a, K> Hashset<'a, K> for Hash<'a, K>
where
	K: Serializable + 'a,
{
	fn insert(&mut self, key: &K) -> Result<(), Error> {
		Self::do_insert_hashset(self, key)
	}
}

impl<'a, K, V> Hashtable<'a, K, V> for Hash<'a, K>
where
	K: Serializable + 'a,
	V: Serializable,
{
	fn insert(&mut self, key: &K, value: &V) -> Result<(), Error> {
		Self::do_insert_hashtable(self, key, value)
	}
}

impl<'a, K> Hash<'a, K>
where
	K: Serializable + 'a,
{
	fn new() -> Self {
		Self {
			phantom_data: PhantomData,
		}
	}

	fn do_insert_hashtable<V>(&mut self, key: &K, value: &V) -> Result<(), Error>
	where
		V: Serializable,
	{
		self.do_insert(key, Some(value))?;
		Ok(())
	}

	fn do_insert_hashset(&mut self, key: &K) -> Result<(), Error> {
		let value: Option<usize> = None;
		self.do_insert(key, value.as_ref())?;
		Ok(())
	}

	fn do_insert<V>(&mut self, _key: &K, value: Option<&V>) -> Result<(), Error>
	where
		V: Serializable,
	{
		debug!("do insert value.is_some={}", value.is_some())?;
		Ok(())
	}
}
*/

pub struct Iterator<'a, K>
where
	K: Serializable,
{
	hash_class: &'a HashClass<'a, K>,
	cur: usize,
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
        no_send;
        var phantom_data: PhantomData<&'a K>;
        generic hashtable: <'a, K, V> where K: Serializable + 'a, V: Serializable;
        pub list as list_impl;

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
impl<'a, K> HashClass<'a, K> where K: Serializable + 'a {}

impl<'a, K> HashClassVarBuilder for HashClassVar<'a, K>
where
	K: Serializable + 'a,
{
	fn builder(_constants: &HashClassConst) -> Result<Self, Error> {
		Ok(Self {
			phantom_data: PhantomData,
		})
	}
}

impl<'a, K> HashClass<'a, K>
where
	K: Serializable + 'a,
{
	fn hashtable_insert<V>(&mut self, _key: K, _value: V) -> Result<(), Error>
	where
		V: Serializable,
	{
		println!("insert hashtable!");
		Ok(())
	}

	fn hashset_insert(&mut self, key: K) -> Result<(), Error> {
		println!("hashset insert!");
		Ok(())
	}

	fn push(&mut self, _value: K) -> Result<(), Error> {
		println!("push list!");
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
		let mut x_clone = x.clone();

		std::thread::spawn(move || -> Result<(), Error> {
			println!("ok");
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
