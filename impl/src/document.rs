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

use crate::constants::*;
use crate::types::DocMacroState as MacroState;
use crate::types::{DocItem, Input, TokenType};
use bmw_deps::litrs;
use bmw_deps::substring::Substring;
use bmw_err::*;
use proc_macro::TokenTree::*;
use proc_macro::{Group, Ident, Literal, Punct, TokenStream, TokenTree};
use std::collections::HashMap;
use std::str::FromStr;

const DEBUG: bool = false;

// use a makeshift log because we want to use this as a dependency in the logging crate
macro_rules! debug {
        ($line:expr) => {{
                if DEBUG {
                        println!($line);
                }
                if true {
                        Ok(())
                } else {
                        Err(err!(ErrKind::Log, "impossible logging error"))
                }
        }};
        ($line:expr, $($values:tt)*) => {{
                if DEBUG {
                        println!($line, $($values)*);
                }
                if true {
                        Ok(())
                } else {
                        Err(err!(ErrKind::Log, "impossible logging error"))
                }
        }};
}

#[cfg(not(tarpaulin_include))]
impl MacroState {
	fn new() -> Self {
		Self {
			counter: 0,
			ret: "".to_string(),
			in_hash: false,
			in_trait: false,
			expect_add_doc: false,
			docs: DocItem::new(),
			expect_return_type: false,
			last_dash: false,
			doc_point: 0,
			expect_fn_name: false,
			expect_parameters: false,
			expect_doc_equal: false,
			expect_doc_line: false,
			expect_name: false,
			name: "".to_string(),
			in_macro_rules: false,
		}
	}
}

#[cfg(not(tarpaulin_include))]
impl DocItem {
	fn new() -> Self {
		Self {
			input_hash: HashMap::new(),
			error_str: "".to_string(),
			see_str: "".to_string(),
			return_str: "".to_string(),
			return_type_str: "".to_string(),
		}
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_document(attr: TokenStream, item: TokenStream) -> TokenStream {
	match do_derive_document_impl(attr, &item) {
		Ok(stream) => stream,
		Err(e) => {
			println!(
				"WARNING: do_derive_document generated error, cannot produce documentation: {}",
				e
			);
			item
		}
	}
}

#[cfg(not(tarpaulin_include))]
pub fn do_derive_document_impl(
	_attr: TokenStream,
	item: &TokenStream,
) -> Result<TokenStream, Error> {
	debug!("in do_derive_document")?;
	let mut state = MacroState::new();
	// add a add_doc in to avoid the warning since we strip out all of them in our return value

	state.ret = "#[add_doc(empty)]\n".to_string();

	for tree in item.clone() {
		match tree {
			Ident(ident) => {
				process_ident(ident.clone(), &mut state)?;
				process_token(ident.to_string(), TokenType::Ident, &mut state)?;
			}
			Group(group) => {
				process_group(group.clone(), &mut state)?;
				for group_item in group.stream() {
					process_token(group_item.to_string(), TokenType::GroupItem, &mut state)?;
				}
			}
			Literal(literal) => {
				process_literal(literal.clone(), &mut state)?;
				process_token(literal.to_string(), TokenType::Literal, &mut state)?;
			}
			Punct(punct) => {
				process_punct(punct.clone(), &mut state)?;
				process_token(punct.to_string(), TokenType::Punct, &mut state)?;
			}
		}
	}

	debug!("ret='{}'", state.ret)?;

	map_err!(state.ret.parse(), ErrKind::Parse)
}

#[cfg(not(tarpaulin_include))]
fn process_token(
	_token: String,
	_token_type: TokenType,
	_state: &mut MacroState,
) -> Result<bool, Error> {
	Ok(false)
}

#[cfg(not(tarpaulin_include))]
fn process_punct(punct: Punct, state: &mut MacroState) -> Result<(), Error> {
	debug!("punct[{}]='{}'", state.counter, punct)?;
	if punct == '#' {
		debug!("setting in hash = true")?;
		state.in_hash = true;
	} else if punct == '!' {
		// handle the macro_rules! case
		state.ret = format!("{}{}", state.ret, punct.to_string(),);
	} else {
		state.ret = format!("{}\n{}", state.ret, punct.to_string(),);
	}
	state.counter += 1;
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_literal(literal: Literal, state: &mut MacroState) -> Result<(), Error> {
	debug!("literal[{}]='{}'", state.counter, literal)?;
	state.ret = format!("{}\n{}", state.ret, literal.to_string(),);
	state.counter += 1;
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_group(group: Group, state: &mut MacroState) -> Result<(), Error> {
	let is_trait = state.in_trait;
	let is_macro_rules = state.in_macro_rules;

	if is_trait || is_macro_rules {
		state.ret = format!("{}{}", state.ret, "\n{");
	}

	state.in_macro_rules = false;
	state.in_trait = false;
	debug!("begin group {}", state.counter)?;
	for item in group.stream() {
		process_group_item(item, state)?;
	}

	debug!("==============================end group {}", state.counter)?;

	if is_trait || is_macro_rules {
		state.ret = format!("{}{}", state.ret, "\n}");
	}
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_group_item(item: TokenTree, state: &mut MacroState) -> Result<(), Error> {
	debug!("group_item[{}]='{}'", state.counter, item)?;
	let item_str = item.to_string();

	if item_str == "doc" {
		state.expect_doc_equal = true;
	} else if state.expect_doc_equal {
		if item_str != "=" {
			return Err(err!(ErrKind::Parse, "expected an equal symbol here"));
		}
		state.expect_doc_equal = false;
		state.expect_doc_line = true;
	} else if state.expect_doc_line {
		state.expect_doc_line = false;
		let value =
			map_err!(litrs::StringLit::parse(item_str.clone()), ErrKind::Parse)?.to_string();
		process_doc(value, state)?;
	} else if item_str == "fn" {
		state.expect_fn_name = true;
		mark_doc(state)?;
		debug!("--------------doc point opportunity-----------------")?;
	} else if state.expect_fn_name {
		state.expect_fn_name = false;
		state.expect_parameters = true;
	} else if state.expect_parameters {
		// in parameter list
		debug!("=================the parameters are {}", item_str)?;
		process_parameters(item_str.clone(), state)?;
		state.expect_parameters = false;
	}

	if item_str != ";" && state.expect_return_type {
		state.docs.return_type_str = format!("{} {} ", state.docs.return_type_str, item_str);
	} else if item_str == ";" {
		print_doc(state)?;
	}

	if state.expect_add_doc {
		let start = 1;
		let end = item_str.len().saturating_sub(1);
		if start >= end {
			return Err(err!(
				ErrKind::Parse,
				"unexpected syntax in add_doc '{}'",
				item_str
			));
		}
		process_add_doc(item_str.substring(start, end).to_string(), state)?;
		state.expect_add_doc = false;
	} else if item_str == "#" {
		debug!("setting in hash = true")?;
		state.in_hash = true;
	} else if state.in_hash {
		if item_str != "=" && item_str != "doc" && item_str != "add_doc" {
			debug!("======================setting out of hash")?;
			state.in_hash = false;
			if item_str.find("[add_doc(") == Some(0) {
				let start = 9;
				let end = item_str.len().saturating_sub(2);
				if start >= end {
					return Err(err!(
						ErrKind::Parse,
						"invalid syntax in add_doc '{}'",
						item_str
					));
				}
				process_add_doc(item_str.substring(start, end).to_string(), state)?;
			} else if item_str.find("[doc =") == Some(0) {
				debug!("============---------------========Found a doc")?;
				let start = 7;
				let end = item_str.len().saturating_sub(1);
				if start >= end {
					return Err(err!(ErrKind::Parse, "invalid syntax in doc '{}'", item_str));
				}
				let v = item_str.substring(start, end).to_string();
				let value =
					map_err!(litrs::StringLit::parse(v.clone()), ErrKind::Parse)?.to_string();
				process_doc(value, state)?;
			} else if item_str == "macro_export" {
				mark_doc(state)?;
				state.ret = format!("{}\n#[macro_export]", state.ret);
			}
		} else if item_str == "add_doc" {
			state.expect_add_doc = true;
		}
	} else {
		debug!("out of hash counter = {}", state.counter)?;
		if item_str == "-" || item_str == "=" {
			// special case handling for ->
			state.ret = format!("{}{}", state.ret, item);
			state.last_dash = true;
		} else if item_str == ">" {
			if state.last_dash {
				state.expect_return_type = true;
				state.docs.return_type_str = "".to_string();
			}
			// special case can't prepend newline for ->
			state.ret = format!("{}{} ", state.ret, item);
		} else {
			state.ret = format!("{}\n{} ", state.ret, item);
		}
	}

	if item_str != "-" {
		state.last_dash = false;
	}
	if item_str == ";" {
		state.expect_return_type = false;
	}
	state.counter += 1;
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_doc(mut value: String, state: &mut MacroState) -> Result<(), Error> {
	debug!("value='{}'", value)?;
	if value.len() > 2 {
		value = value.substring(1, value.len() - 1).to_string();
	} else {
		value = "".to_string();
	}
	debug!("value='{}'", value)?;
	let value = value.replace("\\\"", "\"");
	let value = value.replace("\\\'", "\'");
	state.ret = format!("{}\n///{}", state.ret, value);
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn mark_doc(state: &mut MacroState) -> Result<(), Error> {
	if state.doc_point == 0 {
		state.doc_point = state.ret.len();
	}
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn print_doc(state: &mut MacroState) -> Result<(), Error> {
	let mut return_type_str = state.docs.return_type_str.clone();
	debug!("return_type_str = '{}'", return_type_str)?;
	for rep_str in DOC_REPLACEMENTS {
		return_type_str = return_type_str.replace(rep_str.0, rep_str.1);
	}

	if state.docs.see_str.len() > 0 {
		state.ret = format!(
			"{}\n///\n/// # Also See\n///{}{}\n",
			state.ret.substring(0, state.doc_point),
			state.docs.see_str,
			state.ret.substring(state.doc_point, state.ret.len())
		);

		state.docs.see_str = "".to_string();
	}
	if state.docs.error_str.len() > 0 {
		state.ret = format!(
			"{}\n///\n/// # Errors\n///{}{}",
			state.ret.substring(0, state.doc_point),
			state.docs.error_str,
			state.ret.substring(state.doc_point, state.ret.len())
		);

		state.docs.error_str = "".to_string();
	}
	if state.docs.return_str.len() > 0 || return_type_str.len() > 0 {
		state.ret = format!(
			"{}\n///\n/// # Return\n/// `{}` {}{}{}",
			state.ret.substring(0, state.doc_point),
			return_type_str,
			if state.docs.return_str.len() > 0 {
				" - "
			} else {
				""
			},
			state.docs.return_str,
			state.ret.substring(state.doc_point, state.ret.len())
		);
		state.docs.return_str = "".to_string();
		state.docs.return_type_str = "".to_string();
	}
	if state.docs.input_hash.len() > 0 {
		let mut input_parameter_str = "".to_string();
		let mut vec = vec![];
		for (_, v) in &state.docs.input_hash {
			vec.push(v);
		}
		vec.sort();

		for v in &vec {
			input_parameter_str = format!(
				"{}\n/// * `{}` - [{}{}[`{}`]] {} {}",
				input_parameter_str,
				v.name,
				if v.is_ref { "&" } else { "" },
				if v.is_mut { "mut " } else { "" },
				v.type_str,
				if v.text.len() > 0 { "-" } else { "" },
				v.text
			);
		}

		state.ret = format!(
			"{}\n/// # Input Parameters\n///{}{}",
			state.ret.substring(0, state.doc_point),
			input_parameter_str,
			state.ret.substring(state.doc_point, state.ret.len())
		);

		state.docs.input_hash.clear();
	}

	state.doc_point = 0;

	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_parameters(line: String, state: &mut MacroState) -> Result<(), Error> {
	if line.len() < 3 {
		return Ok(());
	}

	let line = line.substring(1, line.len() - 1);
	let strm = map_err!(TokenStream::from_str(&line), ErrKind::Parse)?;

	let mut elems = vec![];
	for elem in strm {
		debug!("fn param={}", elem)?;
		elems.push(elem.to_string());
	}

	let mut itt = 0;
	let mut is_mut = false;
	let mut seqno = 0;
	if elems[0] == "&" {
		itt += 1;
		if elems.len() <= 1 {
			return Err(err!(
				ErrKind::Parse,
				"expected at least 2 elements in fn params",
			));
		}

		loop {
			if itt >= elems.len() {
				break;
			}

			if elems[itt] == "," {
				break;
			} else if elems[itt] == "mut" {
				is_mut = true;
			} else if elems[itt] == "self" {
				add_name_value(
					"self".to_string(),
					state.name.clone(),
					state,
					true,
					is_mut,
					seqno,
				)?;
				seqno += 1;
			}
			itt += 1;
		}
	}

	let mut name: Option<String> = None;
	let mut is_mut = false;
	let mut is_ref = false;
	let mut type_str: String = "".to_string();
	// we should be at either the end (no params) or at the first comma
	loop {
		if itt >= elems.len() {
			break;
		}

		if elems[itt] != "," && name.is_none() {
			name = Some(elems[itt].clone());
		} else if elems[itt] == "," {
			if name.is_some() {
				add_name_value(
					name.as_ref().unwrap().clone(),
					type_str.clone(),
					state,
					is_ref,
					is_mut,
					seqno,
				)?;
				seqno += 1;
				debug!("name='{:?}',value='{}'", name, type_str)?;
			}
			name = None;
			type_str = "".to_string();
			is_mut = false;
			is_ref = false;
		} else if name.is_some() && type_str == "" && elems[itt] == "&" {
			debug!("found an is_ref")?;
			is_ref = true;
		} else if name.is_some() && type_str == "" && elems[itt] == "mut" {
			is_mut = true;
		} else if name.is_some() && elems[itt] != ":" {
			type_str = format!("{} {}", type_str, elems[itt]);
		}

		itt += 1;
	}

	if name.is_some() {
		add_name_value(
			name.as_ref().unwrap().clone(),
			type_str.clone(),
			state,
			is_ref,
			is_mut,
			seqno,
		)?;
		debug!("name='{:?}',value='{}'", name, type_str)?;
	}

	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn add_name_value(
	name: String,
	value: String,
	state: &mut MacroState,
	is_ref: bool,
	is_mut: bool,
	seqno: usize,
) -> Result<(), Error> {
	let mut found = true;
	match state.docs.input_hash.get_mut(&name) {
		Some(input) => {
			input.type_str = value.clone();
			input.is_ref = is_ref;
			input.is_mut = is_mut;
			input.seqno = seqno;
		}
		None => {
			found = false;
		}
	}

	if !found {
		state.docs.input_hash.insert(
			name.clone(),
			Input {
				text: "".to_string(),
				type_str: value,
				is_mut,
				is_ref,
				seqno,
				name,
			},
		);
	}

	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_add_doc(line: String, state: &mut MacroState) -> Result<(), Error> {
	let strm = map_err!(TokenStream::from_str(&line), ErrKind::Parse)?;

	let mut elems = vec![];
	for elem in strm {
		debug!("elem={}", elem)?;
		elems.push(elem.to_string());
	}

	if elems.len() > 0 && elems[0] == "doc_point" {
		mark_doc(state)?;
		return Ok(());
	}

	if elems.len() < 3 {
		return Err(err!(
			ErrKind::Parse,
			"expected at least 3 elements in add_doc: {}",
			line
		));
	}

	if elems[0] == "input" {
		let name = elems[2].clone();
		if elems.len() < 5 {
			return Err(err!(
				ErrKind::Parse,
				"Illegal error line (inputs must have 5 tokens): '{}'",
				line
			));
		}

		let type_str = if elems.len() >= 7 {
			// the type is specified (i.e. for macros)
			let value =
				map_err!(litrs::StringLit::parse(elems[6].clone()), ErrKind::Parse)?.to_string();
			if value.len() < 3 {
				return Err(err!(
					ErrKind::Parse,
					"Illegal error line (length of string must be 3 or greater): '{}'",
					line
				));
			}
			let start = 1;
			let end = value.len() - 1;
			value.substring(start, end).to_string()
		} else {
			"".to_string()
		};
		let value =
			map_err!(litrs::StringLit::parse(elems[4].clone()), ErrKind::Parse)?.to_string();
		if value.len() < 3 {
			return Err(err!(
				ErrKind::Parse,
				"Illegal error line (length of string must be 3 or greater): '{}'",
				line
			));
		}

		let start = 1;
		let end = value.len() - 1;
		let value = value.substring(start, end);
		let mut found = true;

		match state.docs.input_hash.get_mut(&name) {
			Some(input) => input.text = format!("{} {}", input.text, value),
			None => {
				found = false;
			}
		}

		if !found {
			state.docs.input_hash.insert(
				name.clone(),
				Input {
					text: value.to_string(),
					type_str,
					is_mut: false,
					is_ref: false,
					seqno: 0,
					name,
				},
			);
		}
	} else if elems[0] == "error" {
		let error_val =
			map_err!(litrs::StringLit::parse(elems[2].clone()), ErrKind::Parse)?.to_string();
		let start = 1;
		let end = error_val.len().saturating_sub(1);
		if start >= end {
			return Err(err!(ErrKind::Parse, "Illegal error line: '{}'", line));
		}
		state.docs.error_str = format!(
			"{}\n///\n/// * [`{}`] {}",
			state.docs.error_str,
			error_val.substring(start, end),
			if elems.len() >= 5 {
				let mut value =
					map_err!(litrs::StringLit::parse(elems[4].clone()), ErrKind::Parse)?
						.to_string();
				if value.len() > 3 {
					value = value.substring(1, value.len() - 1).to_string();
				}
				format!(" - {}", value)
			} else {
				"".to_string()
			}
		);
	} else if elems[0] == "return" {
		let return_val =
			map_err!(litrs::StringLit::parse(elems[2].clone()), ErrKind::Parse)?.to_string();
		let start = 1;
		let end = return_val.len().saturating_sub(1);
		if start >= end {
			return Err(err!(ErrKind::Parse, "Illegal return line: '{}'", line));
		}
		if elems.len() >= 5 {
			let return_type_str =
				map_err!(litrs::StringLit::parse(elems[4].clone()), ErrKind::Parse)?.to_string();
			let start = 1;
			let end = return_type_str.len().saturating_sub(1);
			if start >= end {
				return Err(err!(ErrKind::Parse, "Illegal return line: '{}'", line));
			}
			state.docs.return_type_str = return_type_str.substring(start, end).to_string();
		}
		state.docs.return_str = format!(
			"{}{} ",
			state.docs.return_str,
			return_val.substring(start, end)
		);
	} else if elems[0] == "see" {
		let see_val =
			map_err!(litrs::StringLit::parse(elems[2].clone()), ErrKind::Parse)?.to_string();
		let start = 1;
		let end = see_val.len().saturating_sub(1);
		if start >= end {
			return Err(err!(ErrKind::Parse, "Illegal see line: '{}'", line));
		}
		state.docs.see_str = format!(
			"{}\n///\n/// * [`{}`] ",
			state.docs.see_str,
			see_val.substring(start, end)
		);
	} else {
		return Err(err!(ErrKind::Parse, "Unknown add_doc command: {}", line));
	}

	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_ident(ident: Ident, state: &mut MacroState) -> Result<(), Error> {
	debug!("ident[{}]='{}'", state.counter, ident)?;
	let ident_str = ident.to_string();

	if ident_str == "pub" {
		mark_doc(state)?;
		print_doc(state)?;
		debug!("--------------doc point opportunity-----------------")?;
	}

	if ident_str == "macro_rules" {
		// special handling for macro_rules
		state.ret = format!("{}\n{}", state.ret, ident);
		state.in_macro_rules = true;
	} else {
		state.ret = format!("{}\n{} ", state.ret, ident);
	}
	if ident_str == "trait" {
		state.in_trait = true;
		state.expect_name = true;
	} else if state.expect_name {
		state.expect_name = false;
		state.name = ident_str;
	}
	state.counter += 1;
	Ok(())
}

#[cfg(not(tarpaulin_include))]
impl PartialEq for Input {
	fn eq(&self, cmp: &Input) -> bool {
		cmp.seqno == self.seqno
	}
}

#[cfg(not(tarpaulin_include))]
impl PartialOrd for Input {
	fn partial_cmp(&self, cmp: &Input) -> Option<std::cmp::Ordering> {
		if self.seqno < cmp.seqno {
			Some(std::cmp::Ordering::Less)
		} else if self.seqno > cmp.seqno {
			Some(std::cmp::Ordering::Greater)
		} else {
			Some(std::cmp::Ordering::Equal)
		}
	}
}

#[cfg(not(tarpaulin_include))]
impl Ord for Input {
	fn cmp(&self, cmp: &Self) -> std::cmp::Ordering {
		if self.seqno < cmp.seqno {
			std::cmp::Ordering::Less
		} else if self.seqno > cmp.seqno {
			std::cmp::Ordering::Greater
		} else {
			std::cmp::Ordering::Equal
		}
	}
}
