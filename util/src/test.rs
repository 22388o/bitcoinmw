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

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use bmw_err::*;
	use bmw_util::*;

	#[test]
	fn test_suffix_tree_macro() -> Result<(), Error> {
		// build a suffix tree with a wild card
		let mut search_trie = search_trie!(
			vec![
				pattern!(Regex("p1".to_string()), PatternId(0))?,
				pattern!(Regex("^p2".to_string()), PatternId(2))?
			],
			TerminationLength(1_000),
			MaxWildcardLength(100)
		)?;

		// create a matches array for the suffix tree to return matches in
		let mut matches = [tmatch!()?; 10];

		// run the match for the input text b"p1p2". Only "p1" matches this time
		// because p2 is not at the start
		let count = search_trie.tmatch(b"p1p2", &mut matches)?;
		assert_eq!(count, 1);

		// since p2 is at the beginning, both match
		let count = search_trie.tmatch(b"p2p1", &mut matches)?;
		assert_eq!(count, 2);
		Ok(())
	}

	#[test]
	fn test_slab_allocator_macro() -> Result<(), Error> {
		let _slabs = slab_allocator!(SlabSize(128), SlabCount(5000))?;
		Ok(())
	}

	#[test]
	fn test_hashtable_macro() -> Result<(), Error> {
		// create a hashtable with the specified parameters
		let mut hashtable = hashtable!(
			MaxEntries(1_000),
			MaxLoadFactor(0.9),
			GlobalSlabAllocator(false),
			SlabCount(100),
			SlabSize(100)
		)?;

		// do an insert, rust will figure out what type is being inserted
		hashtable.insert(&1, &2)?;

		// assert that the entry was inserted
		assert_eq!(hashtable.get(&1)?, Some(2));

		// create another hashtable with defaults, this time the global slab allocator will be
		// used. Since we did not initialize it default values will be used.
		let mut hashtable = hashtable!()?;

		// do an insert, rust will figure out what type is being inserted
		hashtable.insert(&1, &3)?;

		// assert that the entry was inserted
		assert_eq!(hashtable.get(&1)?, Some(3));

		Ok(())
	}
}
