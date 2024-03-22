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

use bmw_log::*;

info!();

/// Macro to get a [`crate::Lock`]. Internally, the parameter passed in is wrapped in
/// an `Arc<Rwlock<T>>` wrapper that can be used to obtain read/write locks around any
/// data structure.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_util::*;
/// use std::time::Duration;
/// use std::thread::{sleep, spawn};
///
/// #[derive(Debug, PartialEq)]
/// struct MyStruct {
///     id: u128,
///     name: String,
/// }
///
/// impl MyStruct {
///     fn new(id: u128, name: String) -> Self {
///         Self { id, name }
///     }
/// }
///
/// fn main() -> Result<(), Error> {
///     let v = MyStruct::new(1234, "joe".to_string());
///     let mut vlock = lock!(v)?;
///     let vlock_clone = vlock.clone();
///
///     spawn(move || -> Result<(), Error> {
///         let mut x = vlock.wlock()?;
///         assert_eq!((**(x.guard()?)).id, 1234);
///         sleep(Duration::from_millis(3000));
///         (**(x.guard()?)).id = 4321;
///         Ok(())
///     });
///
///     sleep(Duration::from_millis(1000));
///     let x = vlock_clone.rlock()?;
///     assert_eq!((**(x.guard()?)).id, 4321);
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! lock {
	($value:expr) => {{
		bmw_util::UtilBuilder::build_lock($value)
	}};
}

/// The same as lock except that the value returned is in a `Box<dyn LockBox<T>>` structure.
/// See [`crate::LockBox`] for a working example.
#[macro_export]
macro_rules! lock_box {
	($value:expr) => {{
		bmw_util::UtilBuilder::build_lock_box($value)
	}};
}

/// macro to call wlock and guard function on a [`crate::LockBox`] at the same time. Note that this only allows
/// a single access to the variable. If more than one operation needs to be done, this macro
/// should not be used.
#[macro_export]
macro_rules! wlock {
	($value:expr) => {
		**($value.wlock()?.guard()?)
	};
}

/// macro to call rlock and guard function on a [`crate::LockBox`] at the same time. Note that this only allows
/// a single access to the variable. If more than one operation needs to be done, this macro
/// should not be used.
#[macro_export]
macro_rules! rlock {
	($value:expr) => {
		**($value.rlock()?.guard()?)
	};
}

/// The `global_slab_allocator` macro initializes the global thread local slab allocator
/// for the thread that it is executed in. It takes the following parameters:
///
/// * SlabSize(usize) (optional) - the size in bytes of the slabs for this slab allocator.
///                                if not specified, the default value of 256 is used.
///
/// * SlabCount(usize) (optional) - the number of slabs to allocate to the global slab
///                                 allocator. If not specified, the default value of
///                                 40,960 is used.
///
/// # Return
/// Return Ok(()) on success or [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrorKind::Configuration`] - Is returned if a
///                                           ConfigOption other than
///                                           ConfigOption::SlabSize or
///                                           ConfigOption::SlabCount is
///                                           specified.
///
/// * [`bmw_err::ErrorKind::IllegalState`] - Is returned if the global thread local
///                                          slab allocator has already been initialized
///                                          for the thread that executes the macro. This
///                                          can happen if the macro is called more than once
///                                          or if a data structure that uses the global
///                                          slab allocator is initialized and in turn initializes
///                                          the global slab allocator with default values.
///
/// * [`bmw_err::ErrorKind::IllegalArgument`] - Is returned if the SlabSize is 0 or the SlabCount
///                                             is 0.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_err::Error;
///
/// fn main() -> Result<(), Error> {
///     global_slab_allocator!(SlabSize(128), SlabCount(1_000))?;
///
///     // this will use the global slab allocator since we don't specify one
///     let hashtable: Box<dyn Hashtable<u32, u32>> = hashtable_box!()?;
///
///     // ...
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! global_slab_allocator {
( $( $config:expr ),* ) => {{
            #[allow(unused_imports)]
            use bmw_conf::ConfigOption::*;
            let mut config = bmw_util::SlabAllocatorConfig::default();
            let mut error: Option<String> = None;
            let mut slab_size_specified = false;
            let mut slab_count_specified = false;

            // compiler sees macro as not used if it's not used in one part of the code
            // these lines make the warnings go away
            if config.slab_size == 0 { config.slab_size = 0; }
            if slab_count_specified { slab_count_specified = false; }
            if slab_size_specified { slab_size_specified = false; }
            if slab_count_specified {}
            if slab_size_specified {}
            if error.is_some() { error = None; }

            $(
                match $config {
                    bmw_conf::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_conf::ConfigOption::SlabCount(slab_count) => {
                        config.slab_count = slab_count;

                        if slab_count_specified {
                            error = Some("SlabCount was specified more than once!".to_string());
                        }

                        slab_count_specified = true;
                        if slab_count_specified {}
                    },
                    _ => {
                        error = Some(format!("'{:?}' is not allowed for hashset", $config));
                    }
                }
            )*

            match error {
                Some(error) => Err(bmw_err::err!(ErrKind::Configuration, error)),
                None => {
                        bmw_util::GLOBAL_SLAB_ALLOCATOR.with(|f| -> Result<(), Error> {
                        unsafe {
                                f.get().as_mut().unwrap().init(config)?;
                                Ok(())
                        }
                        })
                }
            }
        }
    }
}

/// The `slab_allocator` macro initializes a slab allocator with the specified parameters.
/// It takes the following parameters:
///
/// * SlabSize(usize) (optional) - the size in bytes of the slabs for this slab allocator.
///                                if not specified, the default value of 256 is used.
///
/// * SlabCount(usize) (optional) - the number of slabs to allocate to this slab
///                                 allocator. If not specified, the default value of
///                                 40,960 is used.
///
/// # Return
/// Return `Ok(Rc<RefCell<dyn SlabAllocator>>)` on success or [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrorKind::Configuration`] - Is returned if a
///                                           ConfigOption other than
///                                           ConfigOption::SlabSize or
///                                           ConfigOption::SlabCount is
///                                           specified.
///
/// * [`bmw_err::ErrorKind::IllegalArgument`] - Is returned if the SlabSize is 0 or the SlabCount
///                                             is 0.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_err::Error;
///
/// fn main() -> Result<(), Error> {
///     let slabs = slab_allocator!(SlabSize(128), SlabCount(5000))?;
///
///     // this will use the specified slab allocator
///     //let hashtable: Box<dyn Hashtable<u32, u32>> = hashtable_box!(Slabs(&slabs))?;
///
///     // this will also use the specified slab allocator
///     // (they may be shared within the thread)
///     //let hashtable2: Box<dyn Hashtable<u32, u32>> = hashtable_box!(
///     //        Slabs(&slabs),
///     //        MaxEntries(1_000),
///     //        MaxLoadFactor(0.9)
///     //)?;
///
///     // ...
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! slab_allocator {
	($($config:tt)*) => {{
		use bmw_conf::config;
		#[allow(unused_imports)]
		use bmw_conf::ConfigOptionName as CN;
		use bmw_err::*;
		use bmw_util::{SlabAllocatorConfig, UtilBuilder};
		let mut slab_config = SlabAllocatorConfig::default();
		let config = config!($($config)*);
	        match config.check_config(vec![CN::SlabSize, CN::SlabCount], vec![]) {
                        Ok(_) => {

		                slab_config.slab_size = config.get_or_usize(&CN::SlabSize, slab_config.slab_size);
		                slab_config.slab_count = config.get_or_usize(&CN::SlabCount, slab_config.slab_count);

		                let mut slabs = UtilBuilder::build_sync_slabs();
		                match slabs.init(slab_config) {
			                Ok(_) => Ok(slabs),
			                Err(e) => {
				                        let text = format!("Could not init slabs due to: {}", e.to_string());
					                Err(err!(ErrKind::IllegalState, text))
			                }
		                }
                        }
                        Err(e) => {
                                let text = format!("Could not configure slabs due to: {}", e.to_string());
                                Err(err!(ErrKind::Configuration, text))
                        }
                }
	}};
}

/// The pattern macro builds a [`crate::Pattern`] which is used by the [`crate::SearchTrie`].
/// The pattern macro takes the following parameters:
///
/// * Regex(String)         (required) - The regular expression to use for matching (note this is not a
///                                      full regular expression. Only some parts of regular expressions
///                                      are implemented like wildcards and carets). See [`crate::Pattern`]
///                                      for full details.
/// * Id(usize)             (required) - The id for this pattern. This id is returned in the
///                                      [`crate::Match`] array if this match occurs when the
///                                      [`crate::SearchTrie::tmatch`] function is called.
/// * IsMulti(bool)         (optional) - If true is specified this pattern is a multi-line pattern meaning
///                                      that wildcards can cross newlines. Otherwise newlines are not
///                                      allowed in wildcard matches.
/// * IsTerm(bool)          (optional) - If true, this is a termination pattern meaning that if it is
///                                      found, when the [`crate::SearchTrie::tmatch`] function is called,
///                                      matching will terminate and the matches found up to that point in
///                                      the text will be returned.
/// * IsCaseSensitive(bool) (optional) - If true only case sensitive matches are returned for this
///                                      pattern. Otherwise, case-insensitive matches are also returned.
///
/// # Return
/// Returns `Ok(Pattern)` on success and on error a [`bmw_err::Error`] is returned.
///
/// # Errors
/// * [`bmw_err::ErrorKind::Configuration`] - If a Regex or Id is not specified.
///
/// # Examples
///
/// See [`crate::search_trie!`] for examples.
#[macro_export]
macro_rules! pattern {
	( $( $pattern_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($pattern_items)*];
                bmw_util::UtilBuilder::build_pattern(v)
	}};
}

#[macro_export]
macro_rules! tmatch {
        ( $( $match_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($match_items)*];
                bmw_util::UtilBuilder::build_match(v)
        }};
}

/// The `search_trie` macro builds a [`crate::SearchTrie`] which can be used to match multiple
/// patterns for a given text in a performant way.
/// The search_trie macro takes the following parameters:
///
/// * `List<Pattern>`            (required) - The list of [`crate::Pattern`]s that this [`crate::SearchTrie`]
///                                         will use to match.
/// * TerminationLength(usize) (optional) - The length in bytes at which matching will terminate.
/// * MaxWildCardLength(usize) (optional) - The maximum length in bytes of a wild card match.
///
/// # Return
/// Returns `Ok(SuffixTre)` on success and on error a [`bmw_err::Error`] is returned.
///
/// # Errors
/// * [`bmw_err::ErrorKind::IllegalArgument`] - If one of the regular expressions is invalid.
///                                             or the length of the patterns list is 0.
///
/// # Examples
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with three patterns
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2".
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///
///         // assert that two matches were returned "p1" and "p2"
///         // and that their start/end/id is correct.
///         info!("count={}", count)?;
///         assert_eq!(count, 2);
///         assert_eq!(matches[0].id(), 0);
///         assert_eq!(matches[0].start(), 0);
///         assert_eq!(matches[0].end(), 2);
///         assert_eq!(matches[1].id(), 1);
///         assert_eq!(matches[1].start(), 2);
///         assert_eq!(matches[1].end(), 4);
///
///         Ok(())
/// }
///```
///
/// Wild card match
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2.*test".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [UtilBuilder::build_match(vec![])?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match succeeds to two matches are returned.
///         let count = search_trie.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
///
/// Single character wild card
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("p2.test".to_string()), PatternId(1))?,
///                         pattern!(Regex("p3".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match doesn't succeed because it's a single char match. One match is returned.
///         let count = search_trie.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run it with a single char and see that it matches pattern two.
///         let count = search_trie.tmatch(b"p1p2xtestx", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
///
/// Match at the beginning of the text
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {      
///         // build a suffix tree with a wild card
///         let mut search_trie = search_trie!(
///                 vec![
///                         pattern!(Regex("p1".to_string()), PatternId(0))?,
///                         pattern!(Regex("^p2".to_string()), PatternId(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildCardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [tmatch!()?; 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time
///         // because p2 is not at the start
///         let count = search_trie.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // since p2 is at the beginning, both match
///         let count = search_trie.tmatch(b"p2p1", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! search_trie {
	( $patterns:expr, $($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_search_trie($patterns, v)
        }};
}

/// The [`crate::hashtable`] macro builds a [`crate::Hashtable`] with the specified configuration and
/// optionally the specified [`crate::SlabAllocator`]. The macro accepts the following parameters:
///
/// * MaxEntries(usize) (optional) - The maximum number of entries that can be in this hashtable
///                                  at any given time. If not specified, the default value of
///                                  100_000 will be used.
/// * MaxLoadFactor(usize) (optional) - The maximum load factor of the hashtable. The hashtable is
///                                     array based hashtable and it has a fixed size. Once the
///                                     load factor is reach, insertions will return an error. The
///                                     hashtable uses linear probing to handle collisions. The
///                                     max_load_factor makes sure no additional insertions occur
///                                     at a given ratio of entries to capacity in the array. Note
///                                     that MaxEntries can always be inserted, it's the capacity
///                                     of the array that becomes larger as this ratio goes down.
///                                     If not specified, the default value is 0.8.
/// * Slabs(`Option<&Rc<RefCell<dyn SlabAllocator>>>`) (optional) - An optional reference to a slab
///                                     allocator to use with this [`crate::Hashtable`]. If not
///                                     specified, the global slab allocator is used.
///
/// # Returns
///
/// A Ok(`impl Hashtable<K, V>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrorKind::Configuration`] if anything other than ConfigOption::Slabs,
///                                     ConfigOption::MaxEntries or
///                                     ConfigOption::MaxLoadFactor is specified,
///                                     if the slab_allocator's slab_size is greater than 65,536,
///                                     or slab_count is greater than 281_474_976_710_655,
///                                     max_entries is 0 or max_load_factor is not greater than 0
///                                     and less than or equal to 1.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1, &2)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1)?, Some(2));
///
///         // create another hashtable with defaults, this time the global slab allocator will be
///         // used. Since we did not initialize it default values will be used.
///         let mut hashtable = hashtable!()?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashtable.insert(&1, &3)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashtable.get(&1)?, Some(3));
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashtable {
	($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable(v)
        }};
}

/// The [`crate::hashtable_box`] macro builds a [`crate::Hashtable`] with the specified configuration and
/// optionally the specified [`crate::SlabAllocator`]. The only difference between this macro and
/// the [`crate::hashtable`] macro is that the returned hashtable is inserted into a Box.
/// Specifically, the return type is a `Box<dyn Hashtable>`. See [`crate::hashtable`] for further
/// details.
#[macro_export]
macro_rules! hashtable_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_box(v)
        }};
}

/// The difference between this macro and the [`crate::hashtable`] macro is that the returned
/// [`crate::Hashtable`] implements the Send and Sync traits and is thread safe. With this
/// hashtable you cannot specify a [`crate::SlabAllocator`] because they use [`std::cell::RefCell`]
/// which is not thread safe. That is also why this macro returns an error if
/// ConfigOption::Slabs is specified. The parameters for this macro are:
///
/// * MaxEntries(usize) (optional) - The maximum number of entries that can be in this hashtable
///                                  at any given time. If not specified, the default value of
///                                  100_000 will be used.
/// * MaxLoadFactor(usize) (optional) - The maximum load factor of the hashtable. The hashtable is
///                                     array based hashtable and it has a fixed size. Once the
///                                     load factor is reach, insertions will return an error. The
///                                     hashtable uses linear probing to handle collisions. The
///                                     max_load_factor makes sure no additional insertions occur
///                                     at a given ratio of entries to capacity in the array. Note
///                                     that MaxEntries can always be inserted, it's the capacity
///                                     of the array that becomes larger as this ratio goes down.
///                                     If not specified, the default value is 0.8.
/// * SlabSize(usize) (optional) - the size in bytes of the slabs for this slab allocator.
///                                if not specified, the default value of 256 is used.
///
/// * SlabCount(usize) (optional) - the number of slabs to allocate to this slab
///                                 allocator. If not specified, the default value of
///                                 40,960 is used.
///
/// See the [`crate`] for examples.
#[macro_export]
macro_rules! hashtable_sync {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_sync(v)
        }};
}

/// This macro is the same as [`hashtable_sync`] except that the returned hashtable is in a Box.
/// This macro can be used if the sync hashtable needs to be placed in a struct or an enum.
/// See [`crate::hashtable`] and [`crate::hashtable_sync`] for further details.
#[macro_export]
macro_rules! hashtable_sync_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashtable_sync_box(v)
        }};
}

/// The [`crate::hashset`] macro builds a [`crate::Hashset`] with the specified configuration and
/// optionally the specified [`crate::SlabAllocator`]. The macro accepts the following parameters:
///
/// * MaxEntries(usize) (optional) - The maximum number of entries that can be in this hashset
///                                  at any given time. If not specified, the default value of
///                                  100_000 will be used.
/// * MaxLoadFactor(usize) (optional) - The maximum load factor of the hashset. The hashset is
///                                     array based hashset and it has a fixed size. Once the
///                                     load factor is reach, insertions will return an error. The
///                                     hashset uses linear probing to handle collisions. The
///                                     max_load_factor makes sure no additional insertions occur
///                                     at a given ratio of entries to capacity in the array. Note
///                                     that MaxEntries can always be inserted, it's the capacity
///                                     of the array that becomes larger as this ratio goes down.
///                                     If not specified, the default value is 0.8.
/// * Slabs(`Option<&Rc<RefCell<dyn SlabAllocator>>>`) (optional) - An optional reference to a slab
///                                     allocator to use with this [`crate::Hashset`]. If not
///                                     specified, the global slab allocator is used.
///
/// # Returns
///
/// A Ok(`impl Hashset<K>`) on success or a [`bmw_err::Error`] on failure.
///
/// # Errors
///
/// * [`bmw_err::ErrorKind::Configuration`] if anything other than ConfigOption::Slabs,
///                                     ConfigOption::MaxEntries or
///                                     ConfigOption::MaxLoadFactor is specified,
///                                     if the slab_allocator's slab_size is greater than 65,536,
///                                     or slab_count is greater than 281_474_976_710_655,
///                                     max_entries is 0 or max_load_factor is not greater than 0
///                                     and less than or equal to 1.
///
/// # Examples
///```
/// use bmw_util::*;
/// use bmw_log::*;
/// use bmw_err::*;
///
/// fn main() -> Result<(), Error> {
///         // create a hashset with the specified parameters
///         let mut hashset = hashset!(
///                 MaxEntries(1_000),
///                 MaxLoadFactor(0.9),
///                 GlobalSlabAllocator(false),
///                 SlabSize(100),
///                 SlabCount(100)
///         )?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1)?, true);
///
///         // create another hashset with defaults, this time the global slab allocator will be
///         // used. Since we did not initialize it default values will be used.
///         let mut hashset = hashset!()?;
///
///         // do an insert, rust will figure out what type is being inserted
///         hashset.insert(&1)?;
///
///         // assert that the entry was inserted
///         assert_eq!(hashset.contains(&1)?, true);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! hashset {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset(v)
        }};
}

/// The [`crate::hashset_box`] macro is the same as the [`crate::hashset`] macro except that the
/// hashset is returned in a box. See [`crate::hashset`].
#[macro_export]
macro_rules! hashset_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_box(v)
        }};
}

/// The hashset_sync macro is the same as [`crate::hashset`] except that the returned Hashset
/// implements Send and Sync and can be safely passed through threads. See
/// [`crate::hashtable_sync`] for further details.
#[macro_export]
macro_rules! hashset_sync {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_sync(v)
        }};
}

/// The hashset_sync_box macro is the boxed version of the [`crate::hashset_sync`] macro. It is the
/// same except that the returned [`crate::Hashset`] is in a Box so it can be added to structs and
/// enums.
#[macro_export]
macro_rules! hashset_sync_box {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                bmw_util::UtilBuilder::build_hashset_sync_box(v)
        }};
}

/// The list macro is used to create lists. This macro uses the global slab allocator. To use a
/// specified slab allocator, see [`crate::UtilBuilder::build_list`]. It has the same syntax as the
/// [`std::vec!`] macro. Note that this macro and the builder function both
/// return an implementation of the [`crate::SortableList`] trait.
///
/// # Examples
///
///```
/// use bmw_util::*;
/// use bmw_err::*;
/// use bmw_log::*;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     let list = list![1, 2, 3, 4];
///
///     info!("list={:?}", list)?;
///
///     assert!(list_eq!(list, list![1, 2, 3, 4]));
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! list {
    ( $( $x:expr ),* ) => {
        {
            use bmw_util::List;
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// This is the boxed version of list. The returned value is `Box<dyn SortableList>`. Otherwise,
/// this macro is identical to [`crate::list`].
#[macro_export]
macro_rules! list_box {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_box(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// Like [`crate::hashtable_sync`] and [`crate::hashset_sync`] list has a 'sync' version. See those
/// macros for more details and see the [`crate`] for an example of the sync version of a hashtable.
/// Just as in that example the list can be put into a [`crate::lock!`] or [`crate::lock_box`]
/// and passed between threads.
#[macro_export]
macro_rules! list_sync {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_sync(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// Box version of the [`crate::list_sync`] macro.
#[macro_export]
macro_rules! list_sync_box {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_list = bmw_util::UtilBuilder::build_list_sync_box(vec![])?;
            $(
                temp_list.push($x)?;
            )*
            temp_list
        }
    };
}

/// The [`crate::array!`] macro builds an [`crate::Array`]. The macro takes the following
/// parameters:
/// * size (required) - the size of the array
/// * default (required) - a reference to the value to initialize the array with
/// # Return
/// Returns `Ok(impl Array<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrorKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let arr = array!(10, &0)?;
///
///         for x in arr.iter() {
///                 assert_eq!(x, &0);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! array {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array($size, $default)
	}};
}

/// The [`crate::array_list`] macro builds an [`crate::ArrayList`] in the form of a impl
/// SortableList. The macro takes the following parameters:
/// * size (required) - the size of the array
/// * default (required) - a reference to the value to initialize the array with
/// # Return
/// Returns `Ok(impl SortableList<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrorKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut arr = array_list!(10, &0)?;
///         for _ in 0..10 {
///                 arr.push(0)?;
///         }
///
///         for x in arr.iter() {
///                 assert_eq!(x, 0);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! array_list {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list($size, $default)
	}};
}

/// This macro is identical to [`crate::array_list`] except that the value is returned in a box.
/// To be exact, the return value is `Box<dyn SortableList>`. The boxed version can then be used to
/// store in structs and enums. See [`crate::array_list`] for more details and an example.
#[macro_export]
macro_rules! array_list_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_box($size, $default)
	}};
}

/// sync version of [`crate::array_list`].
#[macro_export]
macro_rules! array_list_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_sync($size, $default)
	}};
}

/// sync box version of [`crate::array_list`].
#[macro_export]
macro_rules! array_list_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_array_list_sync_box($size, $default)
	}};
}

/// This macro creates a [`crate::Queue`]. The parameters are
/// * size (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the queue, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Queue<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrorKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut queue = queue!(10, &0)?;
///
///         for i in 0..10 {
///                 queue.enqueue(i)?;
///         }
///
///         for i in 0..10 {
///                 let v = queue.dequeue().unwrap();
///                 assert_eq!(v, &i);
///         }
///         
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! queue {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue($size, $default)
	}};
}

/// This is the box version of [`crate::queue`]. It is identical other than the returned value is
/// in a box `(Box<dyn Queue>)`.
#[macro_export]
macro_rules! queue_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_box($size, $default)
	}};
}

/// This is the sync version of [`crate::queue`]. It is identical other than the returned value is
/// with Sync/Send traits implemented.
#[macro_export]
macro_rules! queue_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_sync($size, $default)
	}};
}

/// This is the box version of [`crate::queue`]. It is identical other than the returned value is
/// in a box `(Box<dyn Queue>)` and Send/Sync traits implemented.
#[macro_export]
macro_rules! queue_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_queue_sync_box($size, $default)
	}};
}

/// This macro creates a [`crate::Stack`]. The parameters are
/// * size (required) - the size of the underlying array
/// * default (required) - a reference to the value to initialize the array with
/// for the stack, these values are never used, but a default is needed to initialize the
/// underlying array.
/// # Return
/// Returns `Ok(impl Stack<T>)` on success and a [`bmw_err::Error`] on failure.
///
/// # Errors
/// * [`bmw_err::ErrorKind::IllegalArgument`] - if the size is 0.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_util::*;
///
/// fn main() -> Result<(), Error> {
///         let mut stack = stack!(10, &0)?;
///
///         for i in 0..10 {
///                 stack.push(i)?;
///         }
///
///         for i in (0..10).rev() {
///                 let v = stack.pop().unwrap();
///                 assert_eq!(v, &i);
///         }
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! stack {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack($size, $default)
	}};
}

/// This is the box version of [`crate::stack`]. It is identical other than the returned value is
/// in a box `(Box<dyn Stack>)`.
#[macro_export]
macro_rules! stack_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_box($size, $default)
	}};
}

/// sync version of [`crate::stack`].
#[macro_export]
macro_rules! stack_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_sync($size, $default)
	}};
}

/// box version of [`crate::stack`].
#[macro_export]
macro_rules! stack_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::UtilBuilder::build_stack_sync_box($size, $default)
	}};
}

/// Append list2 to list1.
#[macro_export]
macro_rules! list_append {
	($list1:expr, $list2:expr) => {{
		for x in $list2.iter() {
			$list1.push(x)?;
		}
	}};
}

/// Compares equality of list1 and list2.
#[macro_export]
macro_rules! list_eq {
	($list1:expr, $list2:expr) => {{
		let list1 = &$list1;
		let list2 = &$list2;
		let list1_size = list1.size();
		if list1_size != list2.size() {
			false
		} else {
			let mut ret = true;
			{
				let mut itt1 = list1.iter();
				let mut itt2 = list2.iter();
				for _ in 0..list1_size {
					if itt1.next() != itt2.next() {
						ret = false;
					}
				}
			}
			ret
		}
	}};
}

/// Macro used to configure/build a thread pool. See [`crate::ThreadPool`] for working examples.
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_log::*;
/// use bmw_test::*;
/// use bmw_util::*;
///
/// info!();
///
/// fn test_thread_pool_macro() -> Result<(), Error> {
///     info!("testing thread_pool macro")?;
///
///     let mut tp = thread_pool!(MinSize(4))?;
///     tp.set_on_panic(move |id, e| -> Result<(), Error> {
///         info!("PANIC: id={},e={:?}", id, e)?;
///         Ok(())
///     })?;
///
///     tp.start()?;
///
///     execute!(tp, {
///          info!("executing a thread")?;
///          Ok(())
///     })?;
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! thread_pool {
        ( $( $match_items:tt)* ) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($match_items)*];
                bmw_util::UtilBuilder::build_thread_pool(v)
        }};
}

/// Macro used to execute tasks in a thread pool. See [`crate::ThreadPool`] for working examples.
#[macro_export]
macro_rules! execute {
	($thread_pool:expr, $program:expr) => {{
		$thread_pool.execute(async move { $program }, bmw_deps::rand::random())
	}};
	($thread_pool:expr, $id:expr, $program:expr) => {{
		$thread_pool.execute(async move { $program }, $id)
	}};
}

/// Macro used to block until a thread pool has completed the task. See [`crate::ThreadPool`] for working examples.
#[macro_export]
macro_rules! block_on {
	($res:expr) => {{
		match $res.recv() {
			Ok(res) => res,
			Err(e) => bmw_util::PoolResult::Err(bmw_err::err!(
				bmw_err::ErrKind::ThreadPanic,
				format!("thread pool panic: {}", e)
			)),
		}
	}};
}
