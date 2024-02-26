// Copyright (c) 2023, The BitcoinMW Developers
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
///         assert_eq!((**(x.guard())).id, 1234);
///         sleep(Duration::from_millis(3000));
///         (**(x.guard())).id = 4321;
///         Ok(())
///     });
///
///     sleep(Duration::from_millis(1000));
///     let x = vlock_clone.rlock()?;
///     assert_eq!((**(x.guard())).id, 4321);
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! lock {
	($value:expr) => {{
		bmw_util::Builder::build_lock($value)
	}};
}

/// The same as lock except that the value returned is in a `Box<dyn LockBox<T>>` structure.
/// See [`crate::LockBox`] for a working example.
#[macro_export]
macro_rules! lock_box {
	($value:expr) => {{
		bmw_util::Builder::build_lock_box($value)
	}};
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
///                                           [`crate::ConfigOption`] other than
///                                           [`crate::ConfigOption::SlabSize`] or
///                                           [`crate::ConfigOption::SlabCount`] is
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
                    bmw_util::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_util::ConfigOption::SlabCount(slab_count) => {
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
                Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
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
///                                           [`crate::ConfigOption`] other than
///                                           [`crate::ConfigOption::SlabSize`] or
///                                           [`crate::ConfigOption::SlabCount`] is
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
///     let hashtable: Box<dyn Hashtable<u32, u32>> = hashtable_box!(Slabs(&slabs))?;
///
///     // this will also use the specified slab allocator
///     // (they may be shared within the thread)
///     let hashtable2: Box<dyn Hashtable<u32, u32>> = hashtable_box!(
///             Slabs(&slabs),
///             MaxEntries(1_000),
///             MaxLoadFactor(0.9)
///     )?;
///
///     // ...
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! slab_allocator {
( $( $config:expr ),* ) => {{
            let slabs = bmw_util::Builder::build_slabs_ref();
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
                    bmw_util::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_util::ConfigOption::SlabCount(slab_count) => {
                        config.slab_count = slab_count;

                        if slab_count_specified {
                            error = Some("SlabCount was specified more than once!".to_string());
                        }

                        slab_count_specified = true;
                        if slab_count_specified {}
                    },
                    _ => {
                        error = Some(format!("'{:?}' is not allowed for slab_allocator", $config));
                    }
                }
            )*
            match error {
                Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                None => {
                        {
                                let mut slabs: std::cell::RefMut<_> = slabs.borrow_mut();
                                slabs.init(config)?;
                        }
                        Ok(slabs)
                },
            }
     }};
}

/// The pattern macro builds a [`crate::Pattern`] which is used by the [`crate::SuffixTree`].
/// The pattern macro takes the following parameters:
///
/// * Regex(String)         (required) - The regular expression to use for matching (note this is not a
///                                      full regular expression. Only some parts of regular expressions
///                                      are implemented like wildcards and carets). See [`crate::Pattern`]
///                                      for full details.
/// * Id(usize)             (required) - The id for this pattern. This id is returned in the
///                                      [`crate::Match`] array if this match occurs when the
///                                      [`crate::SuffixTree::tmatch`] function is called.
/// * IsMulti(bool)         (optional) - If true is specified this pattern is a multi-line pattern meaning
///                                      that wildcards can cross newlines. Otherwise newlines are not
///                                      allowed in wildcard matches.
/// * IsTerm(bool)          (optional) - If true, this is a termination pattern meaning that if it is
///                                      found, when the [`crate::SuffixTree::tmatch`] function is called,
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
/// See [`crate::suffix_tree!`] for examples.
#[macro_export]
macro_rules! pattern {
	( $( $pattern_items:expr ),* ) => {{
                let mut regex: Option<&str> = None;
                let mut is_term = false;
                let mut is_multi = true;
                let mut is_case_sensitive = false;
                let mut id: Option<usize> = None;
                $(
                        match $pattern_items {
                                bmw_util::PatternParam::Regex(r) => { regex = Some(r); },
                                bmw_util::PatternParam::IsTerm(is_t) => { is_term = is_t; },
                                bmw_util::PatternParam::IsMulti(is_m) => { is_multi = is_m; },
                                bmw_util::PatternParam::IsCaseSensitive(is_cs) => { is_case_sensitive = is_cs; },
                                bmw_util::PatternParam::Id(i) => { id = Some(i) },
                        }
                )*

                match id {
                        Some(id) => match regex {
                                Some(regex) => bmw_util::Builder::build_pattern(regex, is_case_sensitive, is_term, is_multi, id),
                                None => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, "Regex must be specified")),
                        }
                        None => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, "Id must be specified")),
                }
	}};
}

/// The `suffix_tree` macro builds a [`crate::SuffixTree`] which can be used to match multiple
/// patterns for a given text in a performant way.
/// The suffix_tree macro takes the following parameters:
///
/// * `List<Pattern>`            (required) - The list of [`crate::Pattern`]s that this [`crate::SuffixTree`]
///                                         will use to match.
/// * TerminationLength(usize) (optional) - The length in bytes at which matching will terminate.
/// * MaxWildcardLength(usize) (optional) - The maximum length in bytes of a wild card match.
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
///         let mut suffix_tree = suffix_tree!(
///                 list![
///                         pattern!(Regex("p1"), Id(0))?,
///                         pattern!(Regex("p2"), Id(1))?,
///                         pattern!(Regex("p3"), Id(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildcardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [Builder::build_match_default(); 10];
///
///         // run the match for the input text b"p1p2".
///         let count = suffix_tree.tmatch(b"p1p2", &mut matches)?;
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
///         let mut suffix_tree = suffix_tree!(
///                 list![
///                         pattern!(Regex("p1"), Id(0))?,
///                         pattern!(Regex("p2.*test"), Id(1))?,
///                         pattern!(Regex("p3"), Id(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildcardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [Builder::build_match_default(); 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = suffix_tree.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match succeeds to two matches are returned.
///         let count = suffix_tree.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
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
///         let mut suffix_tree = suffix_tree!(
///                 list![
///                         pattern!(Regex("p1"), Id(0))?,
///                         pattern!(Regex("p2.test"), Id(1))?,
///                         pattern!(Regex("p3"), Id(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildcardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [Builder::build_match_default(); 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time.
///         let count = suffix_tree.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run the match for the input text b"p1p2xxxxxxtest1". Now the wildcard
///         // match doesn't succeed because it's a single char match. One match is returned.
///         let count = suffix_tree.tmatch(b"p1p2xxxxxxtest1", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // run it with a single char and see that it matches pattern two.
///         let count = suffix_tree.tmatch(b"p1p2xtestx", &mut matches)?;
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
///         let mut suffix_tree = suffix_tree!(
///                 list![
///                         pattern!(Regex("p1"), Id(0))?,
///                         pattern!(Regex("^p2"), Id(2))?
///                 ],
///                 TerminationLength(1_000),
///                 MaxWildcardLength(100)
///         )?;
///
///         // create a matches array for the suffix tree to return matches in
///         let mut matches = [Builder::build_match_default(); 10];
///
///         // run the match for the input text b"p1p2". Only "p1" matches this time
///         // because p2 is not at the start
///         let count = suffix_tree.tmatch(b"p1p2", &mut matches)?;
///         assert_eq!(count, 1);
///
///         // since p2 is at the beginning, both match
///         let count = suffix_tree.tmatch(b"p2p1", &mut matches)?;
///         assert_eq!(count, 2);
///
///         Ok(())
/// }
///```
#[macro_export]
macro_rules! suffix_tree {
	( $patterns:expr, $( $suffix_items:expr ),* ) => {{
                let mut termination_length = usize::MAX;
                let mut max_wildcard_length = usize::MAX;

                // compiler reports these as not needing to be mut so
                // add these lines to satisfy the comipler
                if termination_length != usize::MAX {
                        termination_length = usize::MAX;
                }
                if max_wildcard_length != usize::MAX {
                        max_wildcard_length = usize::MAX;
                }

                $(
                        match $suffix_items {
                                bmw_util::SuffixParam::TerminationLength(t) => { termination_length = t; },
                                bmw_util::SuffixParam::MaxWildcardLength(m) => { max_wildcard_length = m; },
                        }
                )*

                bmw_util::Builder::build_suffix_tree($patterns, termination_length, max_wildcard_length)
        }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! hashtable_config {
	( $config_list:expr ) => {{
		let mut config = bmw_util::HashtableConfig::default();
		let mut error: Option<String> = None;
		let mut max_entries_specified = false;
		let mut max_load_factor_specified = false;

		// compiler sees macro as not used if it's not used in one part of the code
		// these lines make the warnings go away
		if config.max_entries == 0 {
			config.max_entries = 0;
		}
		if max_entries_specified {
			max_entries_specified = false;
		}
		if max_load_factor_specified {
			max_load_factor_specified = false;
		}
		if max_load_factor_specified {}
		if max_entries_specified {}
		if error.is_some() {
			error = None;
		}

		for config_item in $config_list.iter() {
			match config_item {
				bmw_util::ConfigOption::MaxEntries(max_entries) => {
					config.max_entries = max_entries;

					if max_entries_specified {
						error = Some("MaxEntries was specified more than once!".to_string());
					}

					max_entries_specified = true;
					if max_entries_specified {}
				}
				bmw_util::ConfigOption::MaxLoadFactor(max_load_factor) => {
					config.max_load_factor = max_load_factor;

					if max_load_factor_specified {
						error = Some("MaxLoadFactor was specified more than once!".to_string());
					}

					max_load_factor_specified = true;
					if max_load_factor_specified {}
				}
				_ => {
					error = Some(format!("'{:?}' is not allowed for hashtable", config_item));
				}
			}
		}

		match error {
			Some(error) => (Some(error), None),
			None => (None, Some(config)),
		}
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! hashtable_sync_config {
	( $config_list:expr ) => {{
		let mut slab_config = bmw_util::SlabAllocatorConfig::default();
		let mut config = bmw_util::HashtableConfig::default();
		let mut error: Option<String> = None;
		let mut max_entries_specified = false;
		let mut max_load_factor_specified = false;
		let mut slab_size_specified = false;
		let mut slab_count_specified = false;

		// compiler sees macro as not used if it's not used in one part of the code
		// these lines make the warnings go away
		if config.max_entries == 0 {
			config.max_entries = 0;
		}
		if max_entries_specified {
			max_entries_specified = false;
		}
		if max_load_factor_specified {
			max_load_factor_specified = false;
		}
		if max_load_factor_specified {}
		if max_entries_specified {}
		if error.is_some() {
			error = None;
		}

		for config_item in $config_list.iter() {
			match config_item {
				bmw_util::ConfigOption::MaxEntries(max_entries) => {
					config.max_entries = max_entries;

					if max_entries_specified {
						error = Some("MaxEntries was specified more than once!".to_string());
					}

					max_entries_specified = true;
					if max_entries_specified {}
				}
				bmw_util::ConfigOption::MaxLoadFactor(max_load_factor) => {
					config.max_load_factor = max_load_factor;

					if max_load_factor_specified {
						error = Some("MaxLoadFactor was specified more than once!".to_string());
					}

					max_load_factor_specified = true;
					if max_load_factor_specified {}
				}
				bmw_util::ConfigOption::SlabSize(slab_size) => {
					slab_config.slab_size = slab_size;

					if slab_size_specified {
						error = Some("SlabSize was specified more than once!".to_string());
					}
					slab_size_specified = true;
					if slab_size_specified {}
				}
				bmw_util::ConfigOption::SlabCount(slab_count) => {
					slab_config.slab_count = slab_count;

					if slab_count_specified {
						error = Some("SlabCount was specified more than once!".to_string());
					}

					slab_count_specified = true;
					if slab_count_specified {}
				}
				_ => {
					error = Some(format!("'{:?}' is not allowed for hashtable", config_item));
				}
			}
		}

		match error {
			Some(error) => (Some(error), None, None),
			None => (None, Some(config), Some(slab_config)),
		}
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! hashset_config {
	( $config_list:expr ) => {{
		let mut config = bmw_util::HashsetConfig::default();
		let mut error: Option<String> = None;
		let mut max_entries_specified = false;
		let mut max_load_factor_specified = false;

		// compiler sees macro as not used if it's not used in one part of the code
		// these lines make the warnings go away
		if config.max_entries == 0 {
			config.max_entries = 0;
		}
		if max_entries_specified {
			max_entries_specified = false;
		}
		if max_load_factor_specified {
			max_load_factor_specified = false;
		}
		if max_load_factor_specified {}
		if max_entries_specified {}
		if error.is_some() {
			error = None;
		}

		for config_item in $config_list.iter() {
			match config_item {
				bmw_util::ConfigOption::MaxEntries(max_entries) => {
					config.max_entries = max_entries;

					if max_entries_specified {
						error = Some("MaxEntries was specified more than once!".to_string());
					}

					max_entries_specified = true;
					if max_entries_specified {}
				}
				bmw_util::ConfigOption::MaxLoadFactor(max_load_factor) => {
					config.max_load_factor = max_load_factor;

					if max_load_factor_specified {
						error = Some("MaxLoadFactor was specified more than once!".to_string());
					}

					max_load_factor_specified = true;
					if max_load_factor_specified {}
				}
				_ => {
					error = Some(format!("'{:?}' is not allowed for hashset", config_item));
				}
			}
		}

		match error {
			Some(error) => (Some(error), None),
			None => (None, Some(config)),
		}
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! hashset_sync_config {
	( $config_list:expr ) => {{
		let mut slab_config = bmw_util::SlabAllocatorConfig::default();
		let mut config = bmw_util::HashsetConfig::default();
		let mut error: Option<String> = None;
		let mut max_entries_specified = false;
		let mut max_load_factor_specified = false;
		let mut slab_size_specified = false;
		let mut slab_count_specified = false;

		// compiler sees macro as not used if it's not used in one part of the code
		// these lines make the warnings go away
		if config.max_entries == 0 {
			config.max_entries = 0;
		}
		if max_entries_specified {
			max_entries_specified = false;
		}
		if max_load_factor_specified {
			max_load_factor_specified = false;
		}
		if max_load_factor_specified {}
		if max_entries_specified {}
		if error.is_some() {
			error = None;
		}

		for config_item in $config_list.iter() {
			match config_item {
				bmw_util::ConfigOption::MaxEntries(max_entries) => {
					config.max_entries = max_entries;

					if max_entries_specified {
						error = Some("MaxEntries was specified more than once!".to_string());
					}

					max_entries_specified = true;
					if max_entries_specified {}
				}
				bmw_util::ConfigOption::MaxLoadFactor(max_load_factor) => {
					config.max_load_factor = max_load_factor;

					if max_load_factor_specified {
						error = Some("MaxLoadFactor was specified more than once!".to_string());
					}

					max_load_factor_specified = true;
					if max_load_factor_specified {}
				}
				bmw_util::ConfigOption::SlabSize(slab_size) => {
					slab_config.slab_size = slab_size;

					if slab_size_specified {
						error = Some("SlabSize was specified more than once!".to_string());
					}
					slab_size_specified = true;
					if slab_size_specified {}
				}
				bmw_util::ConfigOption::SlabCount(slab_count) => {
					slab_config.slab_count = slab_count;

					if slab_count_specified {
						error = Some("SlabCount was specified more than once!".to_string());
					}

					slab_count_specified = true;
					if slab_count_specified {}
				}
				_ => {
					error = Some(format!("'{:?}' is not allowed for hashset", config_item));
				}
			}
		}

		match error {
			Some(error) => (Some(error), None, None),
			None => (None, Some(config), Some(slab_config)),
		}
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
/// * [`bmw_err::ErrorKind::Configuration`] if anything other than [`crate::ConfigOption::Slabs`],
///                                     [`crate::ConfigOption::MaxEntries`] or
///                                     [`crate::ConfigOption::MaxLoadFactor`] is specified,
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
///         // create a default slab allocator
///         let slabs = slab_allocator!()?;
///
///         // create a hashtable with the specified parameters
///         let mut hashtable = hashtable!(MaxEntries(1_000), MaxLoadFactor(0.9), Slabs(&slabs))?;
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
	( $( $config:expr ),* ) => {{
                use bmw_util::List;

                let mut slabs: Option<&std::rc::Rc<std::cell::RefCell<dyn bmw_util::SlabAllocator>>> = None;
                if slabs.is_some() {
                    slabs = None;
                }
                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(s) => {
                            slabs = Some(&s);
                        }
                        _ => {
                        }
                    }
                )*

                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &slabs
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;


                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

		let (error, config) = bmw_util::hashtable_config!(config_list);
		match error {
			Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
			None => bmw_util::Builder::build_hashtable(config.unwrap(), &slabs),
		}
	}};
}

/// The [`crate::hashtable_box`] macro builds a [`crate::Hashtable`] with the specified configuration and
/// optionally the specified [`crate::SlabAllocator`]. The only difference between this macro and
/// the [`crate::hashtable`] macro is that the returned hashtable is inserted into a Box.
/// Specifically, the return type is a `Box<dyn Hashtable>`. See [`crate::hashtable`] for further
/// details.
#[macro_export]
macro_rules! hashtable_box {
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                let mut slabs: Option<&std::rc::Rc<std::cell::RefCell<dyn bmw_util::SlabAllocator>>> = None;
                if slabs.is_some() {
                    slabs = None;
                }
                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(s) => {
                            slabs = Some(&s);
                        }
                        _ => {
                        }
                    }
                )*

                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &slabs
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;


                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config) = bmw_util::hashtable_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashtable_box(config.unwrap(), &slabs),
                }
        }};
}

/// The difference between this macro and the [`crate::hashtable`] macro is that the returned
/// [`crate::Hashtable`] implements the Send and Sync traits and is thread safe. With this
/// hashtable you cannot specify a [`crate::SlabAllocator`] because they use [`std::cell::RefCell`]
/// which is not thread safe. That is also why this macro returns an error if
/// [`crate::ConfigOption::Slabs`] is specified. The parameters for this macro are:
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
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                            return Err(bmw_err::err!(
                                    bmw_err::ErrKind::Configuration,
                                    "Slabs cannot be specified with hashtable_sync"
                            ));
                        }
                        _ => {
                        }
                    }
                )*

                let slabs = slab_allocator!(SlabSize(256),SlabCount(256))?;
                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &Some(&slabs)
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config, slab_config) = bmw_util::hashtable_sync_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashtable_sync(config.unwrap(), slab_config.unwrap()),
                }
        }};
}

/// This macro is the same as [`hashtable_sync`] except that the returned hashtable is in a Box.
/// This macro can be used if the sync hashtable needs to be placed in a struct or an enum.
/// See [`crate::hashtable`] and [`crate::hashtable_sync`] for further details.
#[macro_export]
macro_rules! hashtable_sync_box {
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                            return Err(bmw_err::err!(
                                    bmw_err::ErrKind::Configuration,
                                    "Slabs cannot be specified with hashtable_sync"
                            ));
                        }
                        _ => {
                        }
                    }
                )*

                let slabs = slab_allocator!(SlabSize(256),SlabCount(256))?;
                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &Some(&slabs)
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config, slab_config) = bmw_util::hashtable_sync_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashtable_sync_box(config.unwrap(), slab_config.unwrap()),
                }
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
/// * [`bmw_err::ErrorKind::Configuration`] if anything other than [`crate::ConfigOption::Slabs`],
///                                     [`crate::ConfigOption::MaxEntries`] or
///                                     [`crate::ConfigOption::MaxLoadFactor`] is specified,
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
///         // create a default slab allocator
///         let slabs = slab_allocator!()?;
///
///         // create a hashset with the specified parameters
///         let mut hashset = hashset!(MaxEntries(1_000), MaxLoadFactor(0.9), Slabs(&slabs))?;
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
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                let mut slabs: Option<&std::rc::Rc<std::cell::RefCell<dyn bmw_util::SlabAllocator>>> = None;
                if slabs.is_some() {
                    slabs = None;
                }
                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(s) => {
                            slabs = Some(&s);
                        }
                        _ => {
                        }
                    }
                )*

                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &slabs
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;


                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config) = bmw_util::hashset_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashset(config.unwrap(), &slabs),
                }
        }};
}

/// The [`crate::hashset_box`] macro is the same as the [`crate::hashset`] macro except that the
/// hashset is returned in a box. See [`crate::hashset`].
#[macro_export]
macro_rules! hashset_box {
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                let mut slabs: Option<&std::rc::Rc<std::cell::RefCell<dyn bmw_util::SlabAllocator>>> = None;
                if slabs.is_some() {
                    slabs = None;
                }
                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(s) => {
                            slabs = Some(&s);
                        }
                        _ => {
                        }
                    }
                )*

                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &slabs
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;


                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config) = bmw_util::hashset_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashset_box(config.unwrap(), &slabs),
                }
        }};
}

/// The hashset_sync macro is the same as [`crate::hashset`] except that the returned Hashset
/// implements Send and Sync and can be safely passed through threads. See
/// [`crate::hashtable_sync`] for further details.
#[macro_export]
macro_rules! hashset_sync {
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                            return Err(bmw_err::err!(
                                    bmw_err::ErrKind::Configuration,
                                    "Slabs cannot be specified with hashset_sync"
                            ));
                        }
                        _ => {
                        }
                    }
                )*

                let slabs = slab_allocator!(SlabSize(256),SlabCount(256))?;
                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &Some(&slabs)
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config, slab_config) = bmw_util::hashset_sync_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashset_sync(config.unwrap(), slab_config.unwrap()),
                }
        }};
}

/// The hashset_sync_box macro is the boxed version of the [`crate::hashset_sync`] macro. It is the
/// same except that the returned [`crate::Hashset`] is in a Box so it can be added to structs and
/// enums.
#[macro_export]
macro_rules! hashset_sync_box {
        ( $( $config:expr ),* ) => {{
                use bmw_util::List;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                            return Err(bmw_err::err!(
                                    bmw_err::ErrKind::Configuration,
                                    "Slabs cannot be specified with hashset_sync"
                            ));
                        }
                        _ => {
                        }
                    }
                )*

                let slabs = slab_allocator!(SlabSize(256),SlabCount(256))?;
                let mut config_list = bmw_util::Builder::build_list(
                        bmw_util::ListConfig::default(),
                        &Some(&slabs)
                )?;

                // delete_head is called to remove compiler warning about not needing to be mutable.
                // It does need to be mutable.
                config_list.delete_head()?;

                $(
                    match $config {
                        bmw_util::ConfigOption::Slabs(_) => {
                        }
                        _ => {
                            config_list.push($config)?;
                        }
                    }
                )*

                let (error, config, slab_config) = bmw_util::hashset_sync_config!(config_list);
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => bmw_util::Builder::build_hashset_sync_box(config.unwrap(), slab_config.unwrap()),
                }
        }};
}

/// The list macro is used to create lists. This macro uses the global slab allocator. To use a
/// specified slab allocator, see [`crate::Builder::build_list`]. It has the same syntax as the
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
            let mut temp_list = bmw_util::Builder::build_list(bmw_util::ListConfig::default(), &None)?;
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
            let mut temp_list = bmw_util::Builder::build_list_box(bmw_util::ListConfig::default(), &None)?;
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
                match $x {
                    bmw_util::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_util::ConfigOption::SlabCount(slab_count) => {
                        config.slab_count = slab_count;

                        if slab_count_specified {
                            error = Some("SlabCount was specified more than once!".to_string());
                        }

                        slab_count_specified = true;
                        if slab_count_specified {}
                    },
                    _ => {
                        error = Some(format!("'{:?}' is not allowed for sync_list", $x));
                    }
                }
            )*
            match error {
                Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                None => {
                    let mut temp_list = bmw_util::Builder::build_list_sync(bmw_util::ListConfig::default(), config)?;
                    temp_list.delete_head()?;
                    $(
                        temp_list.push($x)?;
                    )*
                    Ok(temp_list)
                },
            }
        }
    };
}

/// Box version of the [`crate::list_sync`] macro.
#[macro_export]
macro_rules! list_sync_box {
    ( $( $x:expr ),* ) => {
        {
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
                match $x {
                    bmw_util::ConfigOption::SlabSize(slab_size) => {
                        config.slab_size = slab_size;

                        if slab_size_specified {
                            error = Some("SlabSize was specified more than once!".to_string());
                        }
                        slab_size_specified = true;
                        if slab_size_specified {}

                    },
                    bmw_util::ConfigOption::SlabCount(slab_count) => {
                        config.slab_count = slab_count;

                        if slab_count_specified {
                            error = Some("SlabCount was specified more than once!".to_string());
                        }

                        slab_count_specified = true;
                        if slab_count_specified {}
                    },
                    _ => {
                        error = Some(format!("'{:?}' is not allowed for sync_list", $x));
                    }
                }
            )*
            match error {
                Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                None => {
                    let mut temp_list = bmw_util::Builder::build_list_sync(bmw_util::ListConfig::default(), config)?;
                    temp_list.delete_head()?;
                    $(
                        temp_list.push($x)?;
                    )*
                    Ok(Box::new(temp_list))
                },
            }
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
		bmw_util::Builder::build_array($size, $default)
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
		bmw_util::Builder::build_array_list($size, $default)
	}};
}

/// This macro is identical to [`crate::array_list`] except that the value is returned in a box.
/// To be exact, the return value is `Box<dyn SortableList>`. The boxed version can then be used to
/// store in structs and enums. See [`crate::array_list`] for more details and an example.
#[macro_export]
macro_rules! array_list_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_array_list_box($size, $default)
	}};
}

/// sync version of [`crate::array_list`].
#[macro_export]
macro_rules! array_list_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_array_list_sync($size, $default)
	}};
}

/// sync box version of [`crate::array_list`].
#[macro_export]
macro_rules! array_list_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_array_list_sync_box($size, $default)
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
		bmw_util::Builder::build_queue($size, $default)
	}};
}

/// This is the box version of [`crate::queue`]. It is identical other than the returned value is
/// in a box `(Box<dyn Queue>)`.
#[macro_export]
macro_rules! queue_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_queue_box($size, $default)
	}};
}

/// This is the sync version of [`crate::queue`]. It is identical other than the returned value is
/// with Sync/Send traits implemented.
#[macro_export]
macro_rules! queue_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_queue_sync($size, $default)
	}};
}

/// This is the box version of [`crate::queue`]. It is identical other than the returned value is
/// in a box `(Box<dyn Queue>)` and Send/Sync traits implemented.
#[macro_export]
macro_rules! queue_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_queue_sync_box($size, $default)
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
		bmw_util::Builder::build_stack($size, $default)
	}};
}

/// This is the box version of [`crate::stack`]. It is identical other than the returned value is
/// in a box `(Box<dyn Stack>)`.
#[macro_export]
macro_rules! stack_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_stack_box($size, $default)
	}};
}

/// sync version of [`crate::stack`].
#[macro_export]
macro_rules! stack_sync {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_stack_sync($size, $default)
	}};
}

/// box version of [`crate::stack`].
#[macro_export]
macro_rules! stack_sync_box {
	( $size:expr, $default:expr ) => {{
		bmw_util::Builder::build_stack_sync_box($size, $default)
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
#[macro_export]
macro_rules! thread_pool {
	() => {{
                let config = bmw_util::ThreadPoolConfig::default();
                match bmw_util::Builder::build_thread_pool(config) {
                        Ok(mut ret) => {
                                ret.start()?;
                                Ok(ret)
                        }
                        Err(e) => {
                            Err(
                                    bmw_err::err!(
                                            bmw_err::ErrKind::Misc,
                                            format!("threadpoolbuilder buld error: {}", e)
                                    )
                            )
                        }
                }
        }};
	( $( $config:expr ),* ) => {{
                let mut config = bmw_util::ThreadPoolConfig::default();
		let mut error: Option<String> = None;
		let mut min_size_specified = false;
                let mut max_size_specified = false;
                let mut sync_channel_size_specified = false;

                $(
                match $config {
                    bmw_util::ConfigOption::MinSize(min_size) => {
                        config.min_size = min_size;
                         if min_size_specified {
                            error = Some("MinSize was specified more than once!".to_string());
                        }

                        min_size_specified = true;
                        if min_size_specified {}
                    },
                    bmw_util::ConfigOption::MaxSize(max_size) => {
                        config.max_size = max_size;
                        if max_size_specified {
                            error = Some("MaxSize was specified more than once!".to_string());
                        }

                        max_size_specified = true;
                        if max_size_specified {}
                    },
                    bmw_util::ConfigOption::SyncChannelSize(sync_channel_size) => {
                        config.sync_channel_size = sync_channel_size;
                        if sync_channel_size_specified {
                             error = Some("SyncChannelSize was specified more than once!".to_string());
                        }

                        sync_channel_size_specified = true;
                        if sync_channel_size_specified {}
                    }
                    _ => {
                        error = Some(
                            format!(
                                "Invalid configuration {:?} is not allowed for thread_pool!",
                                $config
                            )
                        );
                    }
                }
                )*

                match error {
                    Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                    None => {
                            let mut ret = bmw_util::Builder::build_thread_pool(config)?;
                            ret.start()?;
                            Ok(ret)
                    },
                }
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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::PatternParam::*;
	use crate::{
		lock, lock_box, thread_pool, Builder, Hashset, Hashtable, List, Lock, LockBox, PoolResult,
		Queue, SortableList, Stack, SuffixTree, ThreadPool,
	};
	use bmw_err::{err, ErrKind, Error};
	use bmw_log::*;
	use bmw_util::ConfigOption::*;
	use bmw_util::SuffixParam::*;
	use std::cell::RefMut;
	use std::sync::mpsc::Receiver;
	use std::thread::sleep;
	use std::time::Duration;

	info!();

	struct TestHashsetHolder {
		h1: Option<Box<dyn Hashset<u32>>>,
		h2: Option<Box<dyn LockBox<Box<dyn Hashset<u32> + Send + Sync>>>>,
	}

	#[test]
	fn test_hashset_macros() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(128), SlabCount(1))?;
		{
			let mut hashset = hashset!(Slabs(&slabs))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());
		}

		{
			let mut hashset = hashset_box!(Slabs(&slabs))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());

			let mut thh = TestHashsetHolder {
				h2: None,
				h1: Some(hashset),
			};

			{
				let hashset = thh.h1.as_mut().unwrap();
				assert_eq!(hashset.size(), 1);
			}
		}

		{
			let mut hashset = hashset_sync!(SlabSize(128), SlabCount(1))?;
			hashset.insert(&1)?;
			assert!(hashset.contains(&1)?);
			assert!(!hashset.contains(&2)?);
			assert!(hashset.insert(&2).is_err());
		}

		{
			let hashset = hashset_sync_box!(SlabSize(128), SlabCount(1))?;
			let mut hashset = lock_box!(hashset)?;

			{
				let mut hashset = hashset.wlock()?;
				(**hashset.guard()).insert(&1)?;
				assert!((**hashset.guard()).contains(&1)?);
				assert!(!(**hashset.guard()).contains(&2)?);
				assert!((**hashset.guard()).insert(&2).is_err());
			}

			let mut thh = TestHashsetHolder {
				h1: None,
				h2: Some(hashset),
			};

			{
				let mut hashset = thh.h2.as_mut().unwrap().wlock()?;
				assert_eq!((**hashset.guard()).size(), 1);
			}
		}

		Ok(())
	}

	#[test]
	fn test_slabs_in_hashtable_macro() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(128), SlabCount(1))?;
		let mut hashtable = hashtable!(Slabs(&slabs))?;
		hashtable.insert(&1, &2)?;

		assert_eq!(hashtable.get(&1).unwrap(), Some(2));

		assert!(hashtable.insert(&2, &3).is_err());

		Ok(())
	}

	#[test]
	fn test_hashtable_box_macro() -> Result<(), Error> {
		let slabs = slab_allocator!(SlabSize(128), SlabCount(1))?;
		let mut hashtable = hashtable_box!(Slabs(&slabs))?;
		hashtable.insert(&1, &2)?;

		assert_eq!(hashtable.get(&1).unwrap(), Some(2));

		assert!(hashtable.insert(&2, &3).is_err());

		Ok(())
	}

	struct TestHashtableSyncBox {
		h: Box<dyn LockBox<Box<dyn Hashtable<u32, u32> + Send + Sync>>>,
	}

	#[test]
	fn test_hashtable_sync_box_macro() -> Result<(), Error> {
		let hashtable = hashtable_sync_box!(SlabSize(128), SlabCount(1), MaxEntries(10))?;
		let mut hashtable = lock_box!(hashtable)?;

		{
			let mut hashtable = hashtable.wlock()?;
			(**hashtable.guard()).insert(&1, &2)?;
			assert_eq!((**hashtable.guard()).get(&1).unwrap(), Some(2));
			assert!((**hashtable.guard()).insert(&2, &3).is_err());
		}

		let thsb = TestHashtableSyncBox { h: hashtable };

		{
			let h = thsb.h.rlock()?;
			assert!((**h.guard()).get(&1)?.is_some());
			assert!((**h.guard()).get(&2)?.is_none());
		}

		Ok(())
	}

	#[test]
	fn test_hashtable_sync_macro() -> Result<(), Error> {
		let hashtable = hashtable_sync_box!(SlabSize(128), SlabCount(1), MaxEntries(10))?;
		let mut hashtable = lock!(hashtable)?;

		{
			let mut hashtable = hashtable.wlock()?;
			(**hashtable.guard()).insert(&1, &2)?;
			assert_eq!((**hashtable.guard()).get(&1).unwrap(), Some(2));
			assert!((**hashtable.guard()).insert(&2, &3).is_err());
		}

		Ok(())
	}

	#[test]
	fn test_slab_allocator_macro() -> Result<(), bmw_err::Error> {
		let slabs = slab_allocator!()?;
		let slabs2 = slab_allocator!(SlabSize(128), SlabCount(1))?;

		let mut slabs: RefMut<_> = slabs.borrow_mut();
		let mut slabs2: RefMut<_> = slabs2.borrow_mut();

		let slab = slabs.allocate()?;
		assert_eq!(
			slab.get().len(),
			bmw_util::SlabAllocatorConfig::default().slab_size
		);
		let slab = slabs2.allocate()?;
		assert_eq!(slab.get().len(), 128);
		assert!(slabs2.allocate().is_err());
		assert!(slabs.allocate().is_ok());

		assert!(slab_allocator!(SlabSize(128), SlabSize(64)).is_err());
		assert!(slab_allocator!(SlabCount(128), SlabCount(64)).is_err());
		assert!(slab_allocator!(MaxEntries(128)).is_err());
		assert!(slab_allocator!(MaxLoadFactor(128.0)).is_err());
		Ok(())
	}

	#[test]
	fn test_hashtable_macro() -> Result<(), bmw_err::Error> {
		let mut hashtable = hashtable!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_sync!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_sync!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100)
		)?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_box!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_box!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		let mut hashtable = hashtable_sync_box!()?;
		hashtable.insert(&1, &2)?;
		assert_eq!(hashtable.get(&1).unwrap().unwrap(), 2);
		let mut hashtable = hashtable_sync_box!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100)
		)?;
		hashtable.insert(&"test".to_string(), &1)?;
		assert_eq!(hashtable.size(), 1);
		hashtable.insert(&"something".to_string(), &2)?;
		info!("hashtable={:?}", hashtable)?;

		Ok(())
	}

	#[test]
	fn test_hashset_macro() -> Result<(), bmw_err::Error> {
		let mut hashset = hashset!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_sync!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_sync!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100)
		)?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_box!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_box!(MaxEntries(100), MaxLoadFactor(0.9))?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		let mut hashset = hashset_sync_box!()?;
		hashset.insert(&1)?;
		assert_eq!(hashset.contains(&1).unwrap(), true);
		assert_eq!(hashset.contains(&2).unwrap(), false);
		let mut hashset = hashset_sync_box!(
			MaxEntries(100),
			MaxLoadFactor(0.9),
			SlabSize(100),
			SlabCount(100)
		)?;
		hashset.insert(&"test".to_string())?;
		assert_eq!(hashset.size(), 1);
		assert!(hashset.contains(&"test".to_string())?);
		info!("hashset={:?}", hashset)?;
		hashset.insert(&"another item".to_string())?;
		hashset.insert(&"third item".to_string())?;
		info!("hashset={:?}", hashset)?;

		Ok(())
	}

	#[test]
	fn test_list_macro() -> Result<(), bmw_err::Error> {
		let mut list1 = list!['1', '2', '3'];
		list_append!(list1, list!['a', 'b', 'c']);
		let list2 = list!['1', '2', '3', 'a', 'b', 'c'];
		assert!(list_eq!(list1, list2));
		let list2 = list!['a', 'b', 'c', '1', '2'];
		assert!(!list_eq!(list1, list2));

		let list3 = list![1, 2, 3, 4, 5];
		info!("list={:?}", list3)?;

		let list4 = list_box![1, 2, 3, 4, 5];
		let mut list5 = list_sync!()?;
		let mut list6 = list_sync_box!()?;
		list_append!(list5, list4);
		list_append!(list6, list4);
		assert!(list_eq!(list4, list3));
		assert!(list_eq!(list4, list5));
		assert!(list_eq!(list4, list6));

		Ok(())
	}

	#[test]
	fn test_thread_pool_macro() -> Result<(), bmw_err::Error> {
		let mut tp = thread_pool!()?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		let resp = execute!(tp, {
			info!("in thread pool")?;
			Ok(123)
		})?;
		assert_eq!(block_on!(resp), PoolResult::Ok(123));

		let mut tp = thread_pool!(MinSize(3))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		let resp: Receiver<PoolResult<u32, Error>> = execute!(tp, {
			info!("thread pool2")?;
			Err(err!(ErrKind::Test, "test err"))
		})?;
		assert_eq!(
			block_on!(resp),
			PoolResult::Err(err!(ErrKind::Test, "test err"))
		);

		let mut tp = thread_pool!(MinSize(3))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;
		let resp: Receiver<PoolResult<u32, Error>> = execute!(tp, {
			info!("thread pool panic")?;
			let x: Option<u32> = None;
			Ok(x.unwrap())
		})?;
		assert_eq!(
			block_on!(resp),
			PoolResult::Err(err!(
				ErrKind::ThreadPanic,
				"thread pool panic: receiving on a closed channel"
			))
		);
		Ok(())
	}

	#[test]
	fn test_thread_pool_options() -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(4), MaxSize(5), SyncChannelSize(10))?;
		tp.set_on_panic(move |_id, _e| -> Result<(), Error> { Ok(()) })?;

		assert_eq!(tp.size()?, 4);
		sleep(Duration::from_millis(2_000));
		let resp = execute!(tp, {
			info!("thread pool")?;
			Ok(0)
		})?;
		assert_eq!(block_on!(resp), PoolResult::Ok(0));
		assert_eq!(tp.size()?, 4);

		for _ in 0..10 {
			execute!(tp, {
				info!("thread pool")?;
				sleep(Duration::from_millis(5_000));
				Ok(0)
			})?;
		}
		sleep(Duration::from_millis(2_000));
		assert_eq!(tp.size()?, 5);
		Ok(())
	}

	#[test]
	fn test_list_eq() -> Result<(), Error> {
		let list1 = list![1, 2, 3];
		let eq = list_eq!(list1, list![1, 2, 3]);
		let mut list2 = list![4, 5, 6];
		list_append!(list2, list![5, 5, 5]);
		assert!(eq);
		assert!(list_eq!(list2, list![4, 5, 6, 5, 5, 5]));
		list2.sort_unstable()?;
		assert!(list_eq!(list2, list![4, 5, 5, 5, 5, 6]));
		assert!(!list_eq!(list2, list![1, 2, 3]));
		Ok(())
	}

	#[test]
	fn test_array_macro() -> Result<(), Error> {
		let mut array = array!(10, &0)?;
		array[1] = 2;
		assert_eq!(array[1], 2);

		let mut a = array_list_box!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut a = array_list_sync!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut a = array_list_sync!(10, &0)?;
		a.push(1)?;
		assert_eq!(a.size(), 1);

		let mut q = queue!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_sync!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_box!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut q = queue_sync_box!(10, &0)?;
		q.enqueue(1)?;
		assert_eq!(q.peek(), Some(&1));

		let mut s = stack!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_box!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_sync!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		let mut s = stack_sync_box!(10, &0)?;
		s.push(1)?;
		assert_eq!(s.peek(), Some(&1));

		Ok(())
	}

	#[test]
	fn test_pattern_suffix_macros() -> Result<(), Error> {
		// create matches array
		let mut matches = [Builder::build_match_default(); 10];

		// test pattern
		let pattern = pattern!(Regex("abc"), Id(0))?;
		assert_eq!(
			pattern,
			Builder::build_pattern("abc", false, false, true, 0)?
		);

		// test suffix tree
		let mut suffix_tree = suffix_tree!(
			list![
				pattern!(Regex("abc"), Id(0))?,
				pattern!(Regex("def"), Id(1))?
			],
			TerminationLength(100),
			MaxWildcardLength(50)
		)?;
		let match_count = suffix_tree.tmatch(b"abc", &mut matches)?;
		assert_eq!(match_count, 1);
		Ok(())
	}

	#[test]
	fn test_simple_suffix_tree() -> Result<(), Error> {
		// create matches array
		let mut matches = [Builder::build_match_default(); 10];

		// create a suffix tree
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("aaa"), Id(0))?,
			pattern!(Regex("bbb"), Id(1))?
		],)?;

		// match
		let match_count = suffix_tree.tmatch(b"aaa", &mut matches)?;
		assert_eq!(match_count, 1);
		Ok(())
	}

	#[test]
	fn test_list_sync() -> Result<(), Error> {
		let mut list = list_sync!()?;
		//let mut hash = hashset_sync_box!()?;
		list.push(1)?;
		assert!(list_eq!(list, list![1]));

		let mut list2: Box<dyn SortableList<_>> = list_sync_box!()?;
		list2.push(1)?;
		assert!(list_eq!(list2, list![1]));
		Ok(())
	}
}
