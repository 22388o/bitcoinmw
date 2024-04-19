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
			found_doc_point: false,
			ret: TokenStream::new(),
			ret_pre: TokenStream::new(),
			ret_post: TokenStream::new(),
			in_punct: false,
			insert: false,
			in_add_doc: false,
			add_docs: vec![],
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

	debug!("ret='{}'", state.ret)?;

	Ok(state.ret)
}

fn insert_docs(state: &mut MacroState) -> Result<(), Error> {
	for doc in &state.add_docs {
		debug!("inserting the following add doc: {}", doc)?;
	}
	state
		.ret
		.extend("#[doc = \"********inserted doc here*******\"]".parse::<TokenStream>());
	Ok(())
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
	//debug!("token='{}',type={:?}", token, token_type)?;

	if token_type == TokenType::Punct {
		state.in_punct = true;
	}

	if (token == "trait" || token == "pub" || token == "fn") && !state.insert {
		update_state(state)?;
	}

	if token_type != TokenType::GroupItem {
		let x = token.parse();
		let x: TokenStream = map_err!(x, ErrKind::Parse)?;

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

	Ok(false)
}

fn extend(strm: &TokenStream, token: &String) -> Result<TokenStream, Error> {
	let pre = strm.to_string();

	map_err!(format!("{}{}", pre, token).parse(), ErrKind::Parse)
}
