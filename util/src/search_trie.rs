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

use crate::types::{Dictionary, Node, SearchTrieImpl};
use crate::{Match, Pattern, SearchTrie, Stack, UtilBuilder};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{Config, ConfigBuilder, ConfigOption};
use bmw_err::{err, Error};
use bmw_log::*;
use bmw_ser::{Reader, Serializable, Writer};

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

impl SearchTrie for SearchTrieImpl {
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

impl SearchTrieImpl {
	pub(crate) fn new(
		patterns: Vec<Pattern>,
		termination_length: usize,
		max_wildcard_length: usize,
	) -> Result<Self, Error> {
		if patterns.len() == 0 {
			let text = "search trie must have at least one pattern";
			let e = err!(ErrKind::Configuration, text);
			return Err(e);
		}

		let mut dictionary_case_insensitive = Dictionary::new()?;
		let mut dictionary_case_sensitive = Dictionary::new()?;

		let branch_stack = UtilBuilder::build_stack_sync_box(patterns.len(), &(0, 0))?;

		for pattern in patterns.iter() {
			if pattern.is_case_sensitive {
				dictionary_case_sensitive.add(pattern.clone())?;
			} else {
				dictionary_case_insensitive.add(pattern.clone())?;
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
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(vec![CN::Start, CN::End, CN::MatchId], vec![])?;
		let start = config.get_or_usize(&CN::Start, 0);
		let end = config.get_or_usize(&CN::End, 0);
		let id = config.get_or_usize(&CN::MatchId, 0);
		Ok(Self { start, end, id })
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
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(
			vec![
				CN::Regex,
				CN::IsTerminationPattern,
				CN::IsCaseSensitive,
				CN::IsMultiLine,
				CN::PatternId,
			],
			vec![CN::Regex, CN::PatternId],
		)?;

		let regex = config.get_or_string(&CN::Regex, "".to_string());
		let id = config.get_or_usize(&CN::PatternId, 0);
		let is_termination_pattern = config.get_or_bool(&CN::IsTerminationPattern, false);
		let is_case_sensitive = config.get_or_bool(&CN::IsCaseSensitive, false);
		let is_multi_line = config.get_or_bool(&CN::IsMultiLine, true);

		if is_termination_pattern && is_case_sensitive {
			let tx = "Patterns may not be both a termination pattern and case sensitive";
			return Err(err!(ErrKind::IllegalArgument, tx));
		}
		Ok(Self {
			regex,
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
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		let regex = String::read(reader)?;
		let is_case_sensitive = if reader.read_u8()? == 0 { false } else { true };
		let is_termination_pattern = if reader.read_u8()? == 0 { false } else { true };
		let is_multi_line = if reader.read_u8()? == 0 { false } else { true };
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
