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

use crate::types::DocMacroState as MacroState;
use crate::types::{DeriveErrorKind, DocItem, Input, TokenType};
use bmw_base::*;
use bmw_deps::substring::Substring;
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;
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
                        err!(DeriveErrorKind::Log, "impossible logging error")
                }
        }};
        ($line:expr, $($values:tt)*) => {{
                if DEBUG {
                        println!($line, $($values)*);
                }
                if true {
                        Ok(())
                } else {
                        err!(DeriveErrorKind::Log, "impossible logging error")
                }
        }};
}

#[cfg(not(tarpaulin_include))]
impl MacroState {
	fn new() -> Self {
		Self {
			found_doc_point: false,
			ret: TokenStream::new(),
			ret_pre: TokenStream::new(),
			ret_post: TokenStream::new(),
			in_punct: false,
			insert: false,
			in_add_doc: false,
			in_fn_signature: false,
			prev_single_tick: false,
			prev_token: "".to_string(),
			trait_name: None,
			add_docs: vec![],
			fn_str: "".to_string(),
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
			trait_name: None,
		}
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_document(item: TokenStream) -> TokenStream {
	match do_derive_document_impl(&item) {
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
pub fn do_derive_document_impl(item: &TokenStream) -> Result<TokenStream, Error> {
	debug!("in do_derive_document")?;
	let mut state = MacroState::new();

	for tree in item.clone() {
		match tree {
			Ident(ident) => {
				process_token(ident.to_string(), TokenType::Ident, &mut state)?;
				state.in_punct = false;
			}
			Group(group) => {
				if state.in_fn_signature {
					state.fn_str = format!("{} {}", state.fn_str, "(");
				}
				if state.found_doc_point {
					state
						.ret_post
						.extend(group.to_string().parse::<TokenStream>());
				} else {
					state
						.ret_pre
						.extend(group.to_string().parse::<TokenStream>());
				}

				for group_item in group.stream() {
					process_token(group_item.to_string(), TokenType::GroupItem, &mut state)?;
				}

				if state.in_fn_signature {
					state.fn_str = format!("{} {}", state.fn_str, ")");
				}

				if state.in_add_doc {
					let group_str = group.to_string();
					if group_str == "[add_doc(doc_point)]" {
						// found our doc point
						state.found_doc_point = true;
					} else {
						debug!("Push {}", group_str)?;
						state.add_docs.push(group_str);
					}

					state.in_add_doc = false;
				}
			}
			Literal(literal) => {
				process_token(literal.to_string(), TokenType::Literal, &mut state)?;
				state.in_punct = false;
			}
			Punct(punct) => {
				process_token(punct.to_string(), TokenType::Punct, &mut state)?;
			}
		}
	}

	state.ret.extend(state.ret_pre);

	match state.trait_name {
		Some(name) => {
			let updated = state.ret.to_string().replace(
				"#[document]",
				format!("#[document] #[add_doc(trait_name: \"{}\")]", name).as_str(),
			);
			debug!("updated='{}'", updated)?;
			state.ret = map_err!(updated.parse::<TokenStream>(), BaseErrorKind::Parse)?;
		}
		None => {}
	}
	debug!("ret='{}'", state.ret)?;

	Ok(state.ret)
}

fn insert_docs(state: &mut MacroState) -> Result<(), Error> {
	let mut docs = match process_parameters(state.fn_str.clone())? {
		Some(docs) => docs,

		None => DocItem::new(),
	};

	for doc in &state.add_docs {
		process_doc(&doc, &mut docs)?;
	}

	let mut vec = vec![];
	for (_, v) in docs.input_hash {
		vec.push(v);
	}
	vec.sort();

	let mut input_str = if vec.len() > 0 {
		"#[doc = \"# Input Parameters\\n\"]\n".to_string()
	} else {
		"".to_string()
	};

	for v in vec {
		input_str = format!(
			"{}#[doc = \"\n* `{}` - [{}{}[`{}`]] - {}\"]\n",
			input_str,
			v.name,
			if v.is_ref { "&" } else { "" },
			if v.is_mut { "mut" } else { "" },
			if v.name == "self" && docs.trait_name.is_some() {
				docs.trait_name.clone().unwrap()
			} else {
				v.type_str
			},
			v.text
		);
	}
	debug!("input_str='{}'", input_str)?;

	if docs.return_str.len() > 0 {
		docs.return_str = format!("#[doc = \"# Return\n\"] {}", docs.return_str);
	}

	if docs.error_str.len() > 0 {
		docs.error_str = format!("#[doc = \"# Errors\n\"] {}", docs.error_str);
	}

	for doc in &state.add_docs {
		debug!("inserting the following add doc: {}", doc)?;
	}
	state.ret.extend(input_str.parse::<TokenStream>());
	state.ret.extend(docs.return_str.parse::<TokenStream>());
	state.ret.extend(docs.error_str.parse::<TokenStream>());
	state.ret.extend(docs.see_str.parse::<TokenStream>());
	state
		.ret
		.extend("#[doc = \"<br/>\"]".parse::<TokenStream>());
	Ok(())
}

fn process_doc(line: &String, docs: &mut DocItem) -> Result<(), Error> {
	if line.find("[add_doc") != Some(0) {
		return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
	}
	let line = line.substring(1, line.len() - 1);
	let strm = map_err!(TokenStream::from_str(line), BaseErrorKind::Parse)?;
	let mut elems = vec![];
	for elem in strm {
		debug!("elem={}", elem)?;
		elems.push(elem.to_string());
	}

	if elems.len() < 2 {
		return err!(
			BaseErrorKind::Parse,
			"Unexpected empty add_doc: {}, len: {}",
			line,
			elems.len()
		);
	}

	if elems[1].len() < 3 {
		return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
	}
	let line = elems[1].substring(1, elems[1].len() - 1);
	let strm = map_err!(TokenStream::from_str(&line), BaseErrorKind::Parse)?;
	let mut elems = vec![];
	for elem in strm {
		debug!("elem={}", elem)?;
		elems.push(elem.to_string());
	}

	if elems.len() < 1 {
		return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
	}

	if elems[0] == "input" {
		if elems.len() < 5 || elems[2].len() < 3 || elems[4].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}

		let elem = elems[2].substring(1, elems[2].len() - 1).to_string();
		let mut insert = false;
		match docs.input_hash.get_mut(&elem) {
			Some(ref mut input) => {
				input.text = format!(
					"{} {}",
					input.text,
					elems[4]
						.substring(1, elems[4].len() - 1)
						.to_string()
						.clone()
				);
			}
			None => {
				// allow this for macros only
				match docs.trait_name {
					Some(_) => {
						return err!(
							BaseErrorKind::IllegalArgument,
							"unknown input '{}',line='{}'",
							elems[2],
							line
						);
					}
					None => {
						insert = true;
					}
				}
			}
		}

		if insert {
			let mut type_str = "".to_string();
			let mut text = "".to_string();
			if elems.len() >= 7 && elems[5] == "-" && elems[6].len() >= 3 {
				type_str = elems[6].substring(1, elems[6].len() - 1).to_string();
			}
			if elems[4].len() >= 3 {
				text = elems[4].substring(1, elems[4].len() - 1).to_string();
			}
			let input = Input {
				is_mut: false,
				is_ref: false,
				name: elem.clone(),
				text,
				seqno: 0,
				type_str,
			};
			docs.input_hash.insert(elem, input);
		}
	} else if elems[0] == "return" {
		if elems.len() < 3 || elems[1] != ":" || elems[2].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}

		if elems.len() == 5 && elems[4].len() >= 3 {
			docs.return_type_str = elems[4].substring(1, elems[4].len() - 1).to_string();
		}

		if docs.return_str.len() == 0 {
			docs.return_str = format!(
				"{} #[doc = \"{} {}\"]\n",
				docs.return_str,
				if docs.return_type_str.len() > 0 {
					format!("{} - ", docs.return_type_str)
				} else {
					"".to_string()
				},
				elems[2].substring(1, elems[2].len() - 1)
			);
		} else {
			docs.return_str = format!(
				"{} #[doc = \"{}\"]\n",
				docs.return_str,
				elems[2].substring(1, elems[2].len() - 1)
			);
		}
	} else if elems[0] == "see" {
		if elems.len() < 3 || elems[1] != ":" || elems[2].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}
		if docs.see_str.len() == 0 {
			docs.see_str = format!(
				"#[doc = \"# Also see\n * [`{}`]\"]\n",
				elems[2].substring(1, elems[2].len() - 1)
			);
		} else {
			docs.see_str = format!(
				"{} #[doc = \" * [`{}`]\"]\n",
				docs.see_str,
				elems[2].substring(1, elems[2].len() - 1)
			);
		}
	} else if elems[0] == "error" {
		if elems.len() < 3 || elems[1] != ":" || elems[2].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}

		if elems.len() != 5 || (elems.len() == 5 && elems[3] != "-") || elems[4].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}

		docs.error_str = format!(
			"{} #[doc = \" * [`{}`] - {}\n\"]",
			docs.error_str,
			elems[2].substring(1, elems[2].len() - 1),
			elems[4].substring(1, elems[4].len() - 1),
		);
	} else if elems[0] == "trait_name" {
		if elems.len() < 3 || elems[1] != ":" || elems[2].len() < 3 {
			return err!(BaseErrorKind::Parse, "Unexpected invalid add_doc: {}", line);
		}

		docs.trait_name = Some(elems[2].substring(1, elems[2].len() - 1).to_string());
	} else {
		return err!(
			BaseErrorKind::Parse,
			"Unexpected command in add_doc tag: '{}', command: {}",
			line,
			elems[0]
		);
	}

	Ok(())
}

fn add_name_value(
	name: String,
	value: String,
	is_ref: bool,
	is_mut: bool,
	seqno: usize,
	docs: &mut DocItem,
) -> Result<(), Error> {
	debug!("add param name = {}", name)?;
	let mut found = true;
	match docs.input_hash.get_mut(&name) {
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
		docs.input_hash.insert(
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

fn process_parameters(line: String) -> Result<Option<DocItem>, Error> {
	if line.len() < 3 {
		return Ok(None);
	}
	let mut docs = DocItem::new();

	let line = line.substring(1, line.len() - 1);
	let strm = map_err!(TokenStream::from_str(&line), BaseErrorKind::Parse)?;
	let mut elems = vec![];
	for elem in strm {
		let ident = match elem {
			Ident(_) => true,
			_ => false,
		};
		debug!("fn param={},is_ident={}", elem, ident)?;
		if ident {
			elems.push(format!("[`{}`]", elem));
		} else {
			elems.push(elem.to_string());
		}
	}

	if elems.len() < 2 {
		return err!(
			BaseErrorKind::Parse,
			"unexpected function line (1): '{}'. Cannot parse.",
			line
		);
	}

	if elems[1] == "()" {
		// empty function
		for i in 2..elems.len() {
			debug!("() elems[{}]='{}'", i, elems[i])?;
		}
	} else if elems[1].len() < 3 {
		return err!(
			BaseErrorKind::Parse,
			"unexpected function line (2): '{}'. Cannot parse. elems[1] = '{}'",
			line,
			elems[1]
		);
	}

	// everything after 1 is the return string
	if elems.len() >= 3 && elems[2] == ";" {
		docs.return_type_str = "[`unit`]".to_string();
	} else if elems.len() >= 5 && elems[2] == "-" && elems[3] == ">" {
		docs.return_type_str = "".to_string();
		for i in 4..elems.len() {
			docs.return_type_str = format!("{} {}", docs.return_type_str, elems[i]);
		}
	} else {
		// empty with no return
	}
	debug!("return_type_str='{}'", docs.return_type_str)?;

	if elems[1] == "()" {
		// return because there are no params
		return Ok(Some(docs));
	}
	// parameter line
	let strm = map_err!(
		TokenStream::from_str(&elems[1].substring(1, elems[1].len() - 1)),
		BaseErrorKind::Parse
	)?;
	let mut elems = vec![];
	for elem in strm {
		debug!("parameter param={}", elem)?;
		elems.push(elem.to_string());
	}
	if elems.len() == 0 {
		return err!(
			BaseErrorKind::Parse,
			"unexpected function line (3): '{}'. Cannot parse.",
			line
		);
	}

	let mut itt = 0;
	let mut is_mut = false;
	let mut seqno = 0;
	if elems[0] == "&" {
		itt += 1;
		if elems.len() <= 1 {
			return err!(
				BaseErrorKind::Parse,
				"expected at least 2 elements in fn params",
			);
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
					"".to_string(),
					true,
					is_mut,
					seqno,
					&mut docs,
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
					is_ref,
					is_mut,
					seqno,
					&mut docs,
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
			is_ref,
			is_mut,
			seqno,
			&mut docs,
		)?;
		debug!("name='{:?}',value='{}'", name, type_str)?;
	}

	Ok(Some(docs))
}

fn update_state(state: &mut MacroState) -> Result<(), Error> {
	state.ret.extend(state.ret_pre.clone());
	insert_docs(state)?;
	state.ret.extend(state.ret_post.clone());

	state.ret_pre = TokenStream::new();
	state.ret_post = TokenStream::new();
	state.found_doc_point = false;
	state.in_punct = false;
	state.in_add_doc = false;
	state.insert = true;
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_token(
	token: String,
	token_type: TokenType,
	state: &mut MacroState,
) -> Result<bool, Error> {
	if state.prev_token == "trait" {
		state.trait_name = Some(token.clone());
		debug!("trait_name={:?}", state.trait_name)?;
	}
	if state.in_fn_signature {
		if token == ">" || state.prev_single_tick {
			state.fn_str = format!("{}{}", state.fn_str, token);
		} else {
			state.fn_str = format!("{} {}", state.fn_str, token);
		}
		debug!("infnsig,token='{}',type='{:?}'", token, token_type)?;
		if token == ";" && token_type == TokenType::Punct {
			debug!("fn str = '{}',", state.fn_str)?;
			update_state(state)?;
			state.in_fn_signature = false;
		}
	}

	if token == "\'" {
		state.prev_single_tick = true;
	} else {
		state.prev_single_tick = false;
	}

	if token_type == TokenType::Punct {
		state.in_punct = true;
	}

	if (token == "trait" || token == "pub" || token == "fn" || token == "macro_rules")
		&& !state.insert
	{
		state.found_doc_point = true;
		if token != "fn" {
			update_state(state)?;
		} else {
			state.in_fn_signature = true;
		}
	}

	if token_type != TokenType::GroupItem {
		let x = token.parse();
		let x: TokenStream = map_err!(x, BaseErrorKind::Parse)?;

		if state.found_doc_point {
			if token == ">" {
				state.ret_post = extend(&state.ret_post, &token)?;
			} else {
				state.ret_post.extend(x);
			}
		} else {
			if token == ">" {
				state.ret_pre = extend(&state.ret_pre, &token)?;
			} else {
				state.ret_pre.extend(x);
			}
		}
	}
	if state.in_punct && token == "add_doc" {
		state.in_add_doc = true;
	}

	state.prev_token = token.clone();

	Ok(false)
}

fn extend(strm: &TokenStream, token: &String) -> Result<TokenStream, Error> {
	let pre = strm.to_string();

	map_err!(format!("{}{}", pre, token).parse(), BaseErrorKind::Parse)
}

impl PartialEq for Input {
	fn eq(&self, cmp: &Input) -> bool {
		cmp.seqno == self.seqno
	}
}

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
