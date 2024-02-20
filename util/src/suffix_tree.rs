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

use crate::types::{Dictionary, Node, SuffixTreeImpl};
use crate::{Builder, List, Match, Pattern, Reader, Serializable, Stack, SuffixTree, Writer};
use bmw_err::{err, ErrKind, Error};
use bmw_log::*;

info!();

impl Default for Node {
	fn default() -> Self {
		Self {
			next: [u32::MAX; 257],
			pattern_id: usize::MAX,
			is_multi: false,
			is_term: false,
			is_start_only: false,
			is_multi_line: true,
		}
	}
}

impl Dictionary {
	fn new() -> Result<Self, Error> {
		Ok(Self {
			nodes: vec![Node::default()],
			next: 0,
		})
	}

	fn add(&mut self, pattern: Pattern) -> Result<(), Error> {
		if pattern.regex.len() == 0 {
			let text = "regex length must be greater than 0";
			let e = err!(ErrKind::IllegalArgument, text);
			return Err(e);
		}

		let lower;
		let mut regex = if pattern.is_case_sensitive {
			pattern.regex.as_str().bytes().peekable()
		} else {
			lower = pattern.regex.to_lowercase();
			lower.as_str().bytes().peekable()
		};
		let mut cur_byte = regex.next().unwrap();
		let mut cur_node = &mut self.nodes[0];
		let mut is_start_only = false;

		if cur_byte == '^' as u8 {
			cur_byte = match regex.next() {
				Some(cur_byte) => {
					is_start_only = true;
					cur_byte
				}
				None => {
					let text = "Regex must be at least one byte long not including the ^ character";
					let e = err!(ErrKind::IllegalArgument, text);
					return Err(e);
				}
			}
		}

		loop {
			let (check_index, is_multi) = if cur_byte == '.' as u8 {
				let peek = regex.peek();
				let is_multi = match peek {
					Some(peek) => {
						if *peek == '*' as u8 {
							regex.next();
							true
						} else {
							false
						}
					}
					_ => false,
				};
				(256usize, is_multi) // wild card is 256
			} else if cur_byte == '\\' as u8 {
				let next = regex.next();
				match next {
					Some(next) => {
						if next == '\\' as u8 {
							(cur_byte as usize, false)
						} else if next == '.' as u8 {
							(next as usize, false)
						} else {
							let fmt = format!("Illegal escape character '{}'", next as char);
							let e = err!(ErrKind::IllegalArgument, fmt);
							return Err(e);
						}
					}
					None => {
						let fmt = "Illegal escape character at termination of string";
						let e = err!(ErrKind::IllegalArgument, fmt);
						return Err(e);
					}
				}
			} else {
				(cur_byte as usize, false)
			};
			let index = match cur_node.next[check_index] {
				u32::MAX => {
					cur_node.next[check_index] = self.next + 1;
					self.next += 1;
					self.next
				}
				_ => cur_node.next[check_index],
			};

			if index >= self.nodes.len().try_into()? {
				self.nodes.push(Node::default());
			}
			cur_node = &mut self.nodes[index as usize];
			cur_node.is_multi = is_multi;
			cur_byte = match regex.next() {
				Some(cur_byte) => cur_byte,
				None => {
					cur_node.pattern_id = pattern.id;
					cur_node.is_term = pattern.is_termination_pattern;
					cur_node.is_start_only = is_start_only;
					cur_node.is_multi_line = pattern.is_multi_line;
					break;
				}
			};
		}

		Ok(())
	}
}

impl SuffixTree for SuffixTreeImpl {
	fn tmatch(&mut self, text: &[u8], matches: &mut [Match]) -> Result<usize, Error> {
		let match_count = 0;
		let max_wildcard_length = self.max_wildcard_length;
		let termination_length = self.termination_length;
		let dictionary = &self.dictionary_case_insensitive;
		loop {
			if self.branch_stack.pop().is_none() {
				break;
			}
		}
		let (match_count, term_pos) = Self::tmatch_impl(
			text,
			matches,
			match_count,
			dictionary,
			false,
			max_wildcard_length,
			&mut self.branch_stack,
			termination_length,
			usize::MAX,
		)?;
		let dictionary = &self.dictionary_case_sensitive;
		loop {
			if self.branch_stack.pop().is_none() {
				break;
			}
		}
		let (match_count, _term_pos) = Self::tmatch_impl(
			text,
			matches,
			match_count,
			dictionary,
			true,
			max_wildcard_length,
			&mut self.branch_stack,
			termination_length,
			term_pos,
		)?;

		Ok(match_count)
	}
}

impl SuffixTreeImpl {
	pub(crate) fn new(
		patterns: impl List<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<Self, Error> {
		if patterns.size() == 0 {
			let text = "suffix tree must have at least one pattern";
			let e = err!(ErrKind::Configuration, text);
			return Err(e);
		}

		let mut dictionary_case_insensitive = Dictionary::new()?;
		let mut dictionary_case_sensitive = Dictionary::new()?;

		let branch_stack = Builder::build_stack_sync_box(patterns.size(), &(0, 0))?;

		for pattern in patterns.iter() {
			if pattern.is_case_sensitive {
				dictionary_case_sensitive.add(pattern)?;
			} else {
				dictionary_case_insensitive.add(pattern)?;
			}
		}
		// no additional memory is needed. Shrink to the maximum possible
		dictionary_case_insensitive.nodes.shrink_to(0);
		dictionary_case_sensitive.nodes.shrink_to(0);

		Ok(Self {
			dictionary_case_insensitive,
			dictionary_case_sensitive,
			termination_length,
			max_wildcard_length,
			branch_stack,
		})
	}

	fn tmatch_impl(
		text: &[u8],
		matches: &mut [Match],
		mut match_count: usize,
		dictionary: &Dictionary,
		case_sensitive: bool,
		max_wildcard_length: usize,
		branch_stack: &mut Box<dyn Stack<(usize, usize)> + Send + Sync>,
		termination_length: usize,
		term_pos: usize,
	) -> Result<(usize, usize), Error> {
		let mut itt = 0;
		let len = text.len();
		let mut cur_node = &dictionary.nodes[0];
		let mut start = 0;
		let mut multi_counter = 0;
		let mut is_branch = false;
		let mut has_newline = false;

		loop {
			if start >= len || start >= termination_length {
				break;
			}
			if is_branch {
				is_branch = false;
			} else {
				has_newline = false;
				itt = start;
			}

			loop {
				if itt >= len {
					break;
				}

				let byte = if case_sensitive {
					text[itt]
				} else {
					if text[itt] >= 'A' as u8 && text[itt] <= 'Z' as u8 {
						text[itt] + 32
					} else {
						text[itt]
					}
				};

				if byte == '\r' as u8 || byte == '\n' as u8 {
					has_newline = true;
				}

				if !cur_node.is_multi {
					multi_counter = 0;
				}

				match cur_node.next[byte as usize] {
					u32::MAX => {
						if cur_node.is_multi {
							multi_counter += 1;
							if multi_counter >= max_wildcard_length {
								// wild card max length. break as no
								// match and continue
								break;
							}
							itt += 1;
							continue;
						}
						// check wildcard
						match cur_node.next[256] {
							u32::MAX => {
								break;
							}
							_ => cur_node = &dictionary.nodes[cur_node.next[256] as usize],
						}
					}
					_ => {
						match cur_node.next[256] {
							u32::MAX => {}
							_ => {
								// we have a branch here. Add it to the stack.
								branch_stack.push((itt, cur_node.next[256] as usize))?;
							}
						}
						cur_node = &dictionary.nodes[cur_node.next[byte as usize] as usize]
					}
				}

				match cur_node.pattern_id {
					usize::MAX => {}
					_ => {
						if !(cur_node.is_start_only && start != 0) {
							if match_count >= matches.len() {
								// too many matches return with the
								// first set of matches
								return Ok((match_count, usize::MAX));
							}

							if (!has_newline || cur_node.is_multi_line) && itt + 1 < term_pos {
								matches[match_count].set_id(cur_node.pattern_id);
								matches[match_count].set_end(itt + 1);
								matches[match_count].set_start(start);
								match_count += 1;
								if cur_node.is_term {
									return Ok((match_count, itt));
								}
							}
						}
					}
				}

				itt += 1;
			}

			match branch_stack.pop() {
				Some(br) => {
					cur_node = &dictionary.nodes[br.1];
					itt = br.0;
					is_branch = true;
				}
				None => {
					start += 1;
					cur_node = &dictionary.nodes[0];
				}
			}
		}
		Ok((match_count, usize::MAX))
	}
}

impl Match {
	pub(crate) fn new(start: usize, end: usize, id: usize) -> Self {
		Self { start, end, id }
	}
	pub fn start(&self) -> usize {
		self.start
	}
	pub fn end(&self) -> usize {
		self.end
	}
	pub fn id(&self) -> usize {
		self.id
	}
	pub(crate) fn set_start(&mut self, start: usize) {
		self.start = start;
	}
	pub(crate) fn set_end(&mut self, end: usize) {
		self.end = end;
	}
	pub(crate) fn set_id(&mut self, id: usize) {
		self.id = id;
	}
}

impl Pattern {
	pub(crate) fn new(
		regex: &str,
		is_case_sensitive: bool,
		is_termination_pattern: bool,
		is_multi_line: bool,
		id: usize,
	) -> Result<Self, Error> {
		if is_termination_pattern && is_case_sensitive {
			return Err(err!(
				ErrKind::IllegalArgument,
				"Patterns may not be both a termination pattern and case sensitive"
			));
		}
		Ok(Self {
			regex: regex.to_string(),
			is_termination_pattern,
			is_case_sensitive,
			is_multi_line,
			id,
		})
	}
	pub fn regex(&self) -> &String {
		&self.regex
	}
	pub fn is_case_sensitive(&self) -> bool {
		self.is_case_sensitive
	}
	pub fn is_termination_pattern(&self) -> bool {
		self.is_termination_pattern
	}
	pub fn id(&self) -> usize {
		self.id
	}
}

impl Serializable for Pattern {
	#[cfg(not(tarpaulin_include))] // assert full coverage for this function
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let regex = String::read(reader)?;
		let is_case_sensitive = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let is_termination_pattern = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let is_multi_line = match reader.read_u8()? {
			0 => false,
			_ => true,
		};
		let id = reader.read_usize()?;

		let ret = Self {
			regex,
			is_case_sensitive,
			is_termination_pattern,
			is_multi_line,
			id,
		};
		Ok(ret)
	}
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		String::write(&self.regex, writer)?;
		match self.is_case_sensitive {
			false => writer.write_u8(0)?,
			true => writer.write_u8(1)?,
		}
		match self.is_termination_pattern {
			false => writer.write_u8(0)?,
			true => writer.write_u8(1)?,
		}
		match self.is_multi_line {
			false => writer.write_u8(0)?,
			true => writer.write_u8(1)?,
		}
		writer.write_usize(self.id)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate as bmw_util;
	use crate::PatternParam::*;
	use crate::{list, pattern, suffix_tree, Builder, ListConfig, Pattern, SuffixTree};
	use bmw_err::*;
	use bmw_log::*;

	info!();

	#[test]
	fn test_pattern() -> Result<(), Error> {
		let pattern = Pattern::new("abc", true, true, true, 0);
		assert!(pattern.is_err());
		let pattern = Pattern::new("abc", false, true, true, 0)?;
		assert_eq!(pattern.regex(), "abc");
		assert_eq!(pattern.is_case_sensitive(), false);
		assert_eq!(pattern.is_termination_pattern(), true);
		assert_eq!(pattern.id(), 0);

		Ok(())
	}

	#[test]
	fn test_suffix_tree1() -> Result<(), Error> {
		let mut suffix_tree = Builder::build_suffix_tree(
			list![
				Builder::build_pattern("p1", false, false, false, 0)?,
				Builder::build_pattern("p2", false, false, false, 1)?,
				Builder::build_pattern("p3", true, false, false, 2)?
			],
			1_000,
			100,
		)?;

		let mut matches = [Builder::build_match_default(); 10];
		let count = suffix_tree.tmatch(b"p1p2", &mut matches)?;
		info!("count={}", count)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[0].end(), 2);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 2);
		assert_eq!(matches[1].end(), 4);

		Ok(())
	}

	#[test]
	fn test_suffix_tree_wildcard() -> Result<(), Error> {
		let mut suffix_tree = Builder::build_suffix_tree(
			list![
				Builder::build_pattern("p1.*abc", false, false, false, 0)?,
				Builder::build_pattern("p2", false, false, false, 1)?,
				Builder::build_pattern("p3", true, false, false, 2)?
			],
			37,
			10,
		)?;

		let mut matches = [Builder::build_match_default(); 10];
		let count = suffix_tree.tmatch(b"p1xyz123abcp2", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[0].end(), 11);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 11);
		assert_eq!(matches[1].end(), 13);
		for i in 0..count {
			info!("match[{}]={:?}", i, matches[i])?;
		}

		// try a wildcard that is too long
		let count = suffix_tree.tmatch(b"p1xyzxxxxxxxxxxxxxxxxxxxxxxxx123abcp2", &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 35);
		assert_eq!(matches[0].end(), 37);
		for i in 0..count {
			info!("match[{}]={:?}", i, matches[i])?;
		}

		// test termination
		let count =
			suffix_tree.tmatch(b"p1xyzxxxxxxxxxxxxxxxxxxxxxxxxxxx123abcp2", &mut matches)?;
		assert_eq!(count, 0);

		// non-repeating wildcard
		let mut suffix_tree = Builder::build_suffix_tree(
			list![
				Builder::build_pattern("p1.abc", false, false, false, 0)?,
				Builder::build_pattern("p2", false, false, false, 1)?,
				Builder::build_pattern("p3", true, false, false, 2)?,
				Builder::build_pattern("p4.", true, false, false, 3)?,
				Builder::build_pattern("p5\\\\x", true, false, false, 4)?,
				Builder::build_pattern("p6\\.x", true, false, false, 5)?
			],
			37,
			10,
		)?;
		// 2 wildcard chars so no match
		let count = suffix_tree.tmatch(b"p1xxabc", &mut matches)?;
		assert_eq!(count, 0);

		// 1 wildcard char so it's a match
		let count = suffix_tree.tmatch(b"p1xabc", &mut matches)?;
		assert_eq!(count, 1);

		// no char after p4 so no match
		let count = suffix_tree.tmatch(b"p4", &mut matches)?;
		assert_eq!(count, 0);

		// char after p4 so match
		let count = suffix_tree.tmatch(b"p4a", &mut matches)?;
		assert_eq!(count, 1);

		// char after p4 so match
		let count = suffix_tree.tmatch(b"p4aaa", &mut matches)?;
		assert_eq!(count, 1);

		// '\' matches
		let count = suffix_tree.tmatch(b"p5\\x", &mut matches)?;
		assert_eq!(count, 1);

		// escaped dot match
		let count = suffix_tree.tmatch(b"p6.x", &mut matches)?;
		assert_eq!(count, 1);

		// escaped dot is not a wildcard
		let count = suffix_tree.tmatch(b"p6ax", &mut matches)?;
		assert_eq!(count, 0);

		Ok(())
	}

	#[test]
	fn test_case_sensitivity() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let pattern1 = Builder::build_pattern("AaAaA", true, false, false, 0)?;
		let pattern2 = Builder::build_pattern("AaAaA", false, false, false, 0)?;

		let mut suffix_tree = Builder::build_suffix_tree(list![pattern1], 100, 100)?;

		assert_eq!(suffix_tree.tmatch(b"AAAAA", &mut matches)?, 0);

		let mut suffix_tree = Builder::build_suffix_tree(list![pattern2], 100, 100)?;

		assert_eq!(suffix_tree.tmatch(b"AAAAA", &mut matches)?, 1);

		Ok(())
	}

	#[test]
	fn test_multi_line() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc.*123"), Id(0), IsMulti(false))?,
			pattern!(Regex("def.*123"), Id(1), IsMulti(true))?
		],)?;

		// this will not match because of the newline
		let count = suffix_tree.tmatch(b"abcxxx\n123", &mut matches)?;
		assert_eq!(count, 0);

		// this will match because IsMulti is true
		let count = suffix_tree.tmatch(b"defxxx\n123", &mut matches)?;
		assert_eq!(count, 1);
		Ok(())
	}

	#[test]
	fn test_termination_pattern() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc"), Id(0), IsTerm(false))?,
			pattern!(Regex("def"), Id(1), IsTerm(true))?
		],)?;

		// both matches will be found
		let count = suffix_tree.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 2);

		// only the first match will be found because it is a termination pattern
		let count = suffix_tree.tmatch(b"defabc", &mut matches)?;
		assert_eq!(count, 1);
		Ok(())
	}

	#[test]
	fn test_match_list_too_big() -> Result<(), Error> {
		let mut matches1 = [Builder::build_match_default(); 1];
		let mut matches10 = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc"), Id(0), IsTerm(false))?,
			pattern!(Regex("def"), Id(1), IsTerm(true))?
		],)?;

		// only one match returned because match list length is 1
		let count = suffix_tree.tmatch(b"abcdef", &mut matches1)?;
		assert_eq!(count, 1);

		// both matches returned with long enough list
		let count = suffix_tree.tmatch(b"abcdef", &mut matches10)?;
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_suffix_tree_overlap() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc"), Id(0))?,
			pattern!(Regex("abcdef"), Id(1))?
		],)?;

		let count = suffix_tree.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[1].start(), 0);

		let mut count = 0;
		for i in 0..2 {
			if matches[i].id() == 1 {
				assert_eq!(matches[i].end(), 6);
				count += 1;
			} else if matches[i].id() == 0 {
				assert_eq!(matches[i].end(), 3);
				count += 1;
			}
		}
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_suffix_tree_caret() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc"), Id(0))?,
			pattern!(Regex("^def"), Id(1))?
		],)?;

		// only abc is found because def is not at the start
		let count = suffix_tree.tmatch(b"abcdef", &mut matches)?;
		assert_eq!(count, 1);

		// both found
		let count = suffix_tree.tmatch(b"defabc", &mut matches)?;
		assert_eq!(count, 2);

		Ok(())
	}

	#[test]
	fn test_suffix_tree_error_conditions() -> Result<(), Error> {
		assert!(Builder::build_suffix_tree(
			list![Builder::build_pattern("", false, false, false, 0)?],
			36,
			36
		)
		.is_err());

		assert!(Builder::build_suffix_tree(
			list![Builder::build_pattern("^", false, false, false, 0)?],
			100,
			100
		)
		.is_err());

		assert!(Builder::build_suffix_tree(
			list![Builder::build_pattern("x\\y", false, false, false, 0)?],
			100,
			100
		)
		.is_err());

		assert!(Builder::build_suffix_tree(
			list![Builder::build_pattern("x\\", false, false, false, 0)?],
			100,
			100
		)
		.is_err());

		assert!(Builder::build_suffix_tree(
			Builder::build_list(ListConfig::default(), &None)?,
			100,
			100
		)
		.is_err());

		Ok(())
	}

	#[test]
	fn test_suffix_tree_branches() -> Result<(), Error> {
		let mut matches = [Builder::build_match_default(); 10];
		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("abc"), Id(0))?,
			pattern!(Regex("ab.*x"), Id(1))?
		],)?;

		let count = suffix_tree.tmatch(b"abc", &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);

		let count = suffix_tree.tmatch(b"abcx", &mut matches)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(matches[1].id(), 1);
		assert_eq!(matches[1].start(), 0);

		let text = b"abxx";
		let count = suffix_tree.tmatch(text, &mut matches)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 0);
		assert_eq!(&text[matches[0].start()..matches[0].end()], text);

		let mut suffix_tree = suffix_tree!(list![
			pattern!(Regex("header: 1234\r\n"), Id(1))?,
			pattern!(Regex("header: .*\r\n"), Id(0))?
		],)?;

		let text = b"yyyheader: 1299\r\n";
		let count = suffix_tree.tmatch(text, &mut matches)?;
		info!("count1={}", count)?;
		assert_eq!(count, 1);
		assert_eq!(matches[0].id(), 0);
		assert_eq!(matches[0].start(), 3);
		assert_eq!(matches[0].end(), text.len());

		let text = b"yyyheader: 1234\r\n";
		let count = suffix_tree.tmatch(text, &mut matches)?;
		info!("count2={}", count)?;
		assert_eq!(count, 2);
		assert_eq!(matches[0].id(), 1);
		assert_eq!(matches[0].start(), 3);
		assert_eq!(matches[0].end(), text.len());
		assert_eq!(matches[1].id(), 0);
		assert_eq!(matches[1].start(), 3);
		assert_eq!(matches[1].end(), text.len());

		Ok(())
	}
}
