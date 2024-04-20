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

use crate::do_derive_configurable;
use crate::types::TokenType;
use crate::types::TraitifyMacroState as MacroState;
use crate::utils::{trim_braces, trim_brackets};
use bmw_deps::substring::Substring;
use bmw_err::*;
use proc_macro::TokenTree::*;
use proc_macro::{Group, TokenStream};
use std::collections::HashMap;

const DEBUG: bool = true;

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

impl MacroState {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			in_config: false,
			has_config: false,
			expect_struct_name: false,
			in_add: false,
			in_builder: false,
			in_type: false,
			in_views: false,
			expect_command: false,
			expect_equal: false,
			struct_name: None,
			expect_trait: false,
			cur_fn_name: None,
			cur_fn_view_list: vec![],
			expect_fn_name: false,
			expect_fn_signature: false,
			views: HashMap::new(),
		}
	}
}

pub(crate) fn do_derive_traitify(attr: TokenStream, item: TokenStream) -> TokenStream {
	match do_derive_traitify_impl(attr, item.clone()) {
		Ok(item) => item,
		Err(e) => {
			println!("ERROR: traitify generated error: {}", e);
			item
		}
	}
}

fn do_derive_traitify_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
	debug!("in do_derive_traitify_impl")?;
	let mut state = MacroState::new();
	state.expect_command = true;
	state.ret.extend(item.clone());

	// process the attribute
	for token in attr.clone() {
		let ret = match token {
			Ident(ident) => {
				process_attr_token(ident.to_string(), TokenType::Ident, &mut state, &attr)?
			}
			Group(group) => {
				process_attr_token(group.to_string(), TokenType::GroupItem, &mut state, &attr)?
			}
			Literal(literal) => {
				process_attr_token(literal.to_string(), TokenType::Literal, &mut state, &attr)?
			}
			Punct(punct) => {
				process_attr_token(punct.to_string(), TokenType::Punct, &mut state, &attr)?
			}
		};

		// if true is returned we break
		cbreak!(ret);
	}

	//debug!("traitify ret='{}'", state.ret)?;

	// only add traits for main config
	if state.has_config {
		add_traits(&item, &mut state)?;
	}

	Ok(state.ret)
}

fn process_attr_token(
	token: String,
	token_type: TokenType,
	state: &mut MacroState,
	attr: &TokenStream,
) -> Result<bool, Error> {
	//debug!("token='{}',token_type={:?}", token, token_type)?;

	if state.expect_command {
		if token == "config" && token_type == TokenType::Ident {
			state.in_config = true;
			state.expect_equal = true;
		} else if token == "views" && token_type == TokenType::Ident {
			state.in_views = true;
			state.expect_equal = true;
		} else if token == "type" && token_type == TokenType::Ident {
			state.in_type = true;
			state.expect_equal = true;
		} else if token == "add" && token_type == TokenType::Ident {
			state.in_add = true;
			state.expect_equal = true;
		} else if token == "builder" && token_type == TokenType::Ident {
			state.in_builder = true;
			state.expect_equal = true;
		} else {
			return Err(err!(
				ErrKind::Parse,
				"unexpected token '{}' expecting command: '{}'",
				token,
				attr.to_string()
			));
		}

		state.expect_command = false;
	} else if state.expect_equal {
		if token_type != TokenType::Punct || token != "=" {
			return Err(err!(
				ErrKind::Parse,
				"unexpected token '{}' expecting equal: '{}'",
				token,
				attr.to_string()
			));
		}
		state.expect_equal = false;
	} else if token_type == TokenType::GroupItem {
		if state.in_config {
			let group = trim_brackets(&token);
			let config_token_stream = map_err!(group.parse::<TokenStream>(), ErrKind::Parse)?;
			state.ret.extend(config_token_stream.clone());
			let configurable = do_derive_configurable(config_token_stream);
			state.ret.extend(configurable);
			state.in_config = false;
			state.has_config = true;
		} else if state.in_add {
			//debug!("got an add: '{}'", token)?;
		}
	} else if token_type == TokenType::Punct && token == "," {
		state.expect_command = true;
	} else {
		return Err(err!(
			ErrKind::Parse,
			"unexpected token '{}' '{}'",
			token,
			attr.to_string()
		));
	}

	Ok(state.in_builder)
}

fn add_traits(item: &TokenStream, state: &mut MacroState) -> Result<(), Error> {
	// process the item
	for token in item.clone() {
		let ret = match token {
			Ident(ident) => process_item_token(ident.to_string(), TokenType::Ident, state)?,
			Group(group) => process_item_token(group.to_string(), TokenType::GroupItem, state)?,
			Literal(literal) => process_item_token(literal.to_string(), TokenType::Literal, state)?,
			Punct(punct) => process_item_token(punct.to_string(), TokenType::Punct, state)?,
		};

		// if true is returned we break
		cbreak!(ret);
	}

	Ok(())
}

fn add_to_view_list(group: Group, state: &mut MacroState) -> Result<(), Error> {
	let group_str = group.to_string();
	if group_str.find("[") == Some(0) {
		// we have a bracket item check if it's a traitify
		// tag
		let trimmed = trim_brackets(&group_str);
		let token_stream = map_err!(trimmed.parse::<TokenStream>(), ErrKind::Parse)?;
		let mut first = true;
		let mut is_traitify = false;
		for token in token_stream {
			debug!("tokenx={:?}", token)?;
			if token.to_string() == "traitify" && first {
				// traitify tag
				debug!("found a traitify tag")?;
				is_traitify = true;
			} else if is_traitify {
				// we should have a group here try to iterate and add the views to
				// our list
				match token {
					Group(group) => {
						debug!("Traitify group = {}", group)?;
						for item in group.stream() {
							debug!("traitify item={:?}", item)?;
						}
					}
					_ => {}
				}
			}
			first = false;
		}
	}
	Ok(())
}

fn process_item_token(
	token: String,
	token_type: TokenType,
	state: &mut MacroState,
) -> Result<bool, Error> {
	debug!("item_token='{}',token_type={:?}", token, token_type)?;

	if state.expect_struct_name {
		if token_type != TokenType::Ident {
			return Err(err!(ErrKind::Parse, "expected name"));
		}
		state.expect_struct_name = false;
		state.expect_trait = true;
		state.struct_name = Some(token);
	} else if token == "impl" && token_type == TokenType::Ident {
		state.expect_struct_name = true;
	} else if state.expect_trait {
		let token = trim_braces(&token);
		let token_stream = map_err!(token.parse::<TokenStream>(), ErrKind::Parse)?;
		for token in token_stream {
			match token {
				Ident(ident) => {
					let ident_str = ident.to_string();
					debug!("ident={}", ident_str)?;
					if state.expect_fn_name {
						state.cur_fn_name = Some(ident_str);
						state.expect_fn_name = false;
						state.expect_fn_signature = true;
					} else if ident_str == "fn" {
						state.expect_fn_name = true;
					}
				}
				Group(group) => {
					debug!("group={}", group)?;
					if state.expect_fn_signature {
						debug!("Found fn signature: {}", group)?;
						state.expect_fn_signature = false;
					} else {
						add_to_view_list(group, state)?;
					}
				}
				Literal(literal) => {
					debug!("literal={}", literal)?;
				}
				Punct(punct) => {
					debug!("punct={}", punct)?;
				}
			}
		}
	}
	Ok(false)
}
