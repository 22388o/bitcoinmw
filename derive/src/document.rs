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
use bmw_deps::lazy_static::lazy_static;
use bmw_deps::rand::random;
use bmw_err::*;
use proc_macro::TokenTree::*;
use proc_macro::{Group, Ident, Literal, Punct, TokenStream, TokenTree};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

const DEBUG: bool = true;
lazy_static! {
	pub static ref LOCK: Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
}

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
		}
	}
}

pub(crate) fn do_derive_document(
	_attr: TokenStream,
	item: TokenStream,
) -> Result<TokenStream, Error> {
	let rand: u64 = random();
	let rand = rand % 3_000;
	sleep(Duration::from_millis(rand));

	let _lock = LOCK.write()?;
	debug!("in do_derive_document")?;
	let mut state = MacroState::new();
	state.ret = "/// do_derive_document generated comment".to_string();

	for tree in item.clone() {
		match tree {
			Ident(ident) => {
				process_ident(ident, &mut state)?;
			}
			Group(group) => {
				process_group(group.clone(), &mut state)?;
			}
			Literal(literal) => {
				process_literal(literal, &mut state)?;
			}
			Punct(punct) => {
				process_punct(punct, &mut state)?;
			}
		}
	}

	debug!("ret='{}'", state.ret)?;

	//Ok(item)
	map_err!(state.ret.parse(), ErrKind::Parse)
}

fn process_punct(punct: Punct, state: &mut MacroState) -> Result<(), Error> {
	debug!("punct[{}]='{}'", state.counter, punct)?;
	if punct == '#' {
		debug!("setting in hash = true")?;
		state.in_hash = true;
	} else {
		state.ret = format!("{}\n{}", state.ret, punct.to_string(),);
	}
	state.counter += 1;
	Ok(())
}

fn process_literal(literal: Literal, state: &mut MacroState) -> Result<(), Error> {
	debug!("literal[{}]='{}'", state.counter, literal)?;
	state.ret = format!("{}\n{}", state.ret, literal.to_string(),);
	state.counter += 1;
	Ok(())
}

fn process_group(group: Group, state: &mut MacroState) -> Result<(), Error> {
	let is_trait = state.in_trait;
	if is_trait {
		state.ret = format!("{}{}", state.ret, "\n{");
	}
	state.in_trait = false;
	debug!("begin group {}", state.counter)?;
	for item in group.stream() {
		process_group_item(item, state)?;
	}
	debug!("end group {}", state.counter)?;

	if is_trait {
		state.ret = format!("{}{}", state.ret, "\n}");
	}
	Ok(())
}

fn process_group_item(item: TokenTree, state: &mut MacroState) -> Result<(), Error> {
	debug!("group_item[{}]='{}'", state.counter, item)?;
	let item_str = item.to_string();
	if item_str == "#" {
		debug!("setting in hash = true")?;
		state.in_hash = true;
	} else if state.in_hash {
		debug!("checking item str = {}", item_str)?;
		if item_str != "=" && item_str != "doc" && item_str != "add_doc" {
			debug!("setting out of hash")?;
			state.in_hash = false;
			if item_str.find("[add_doc(") == Some(0) {
				state.ret = format!("{}\n/// add_doc comment2 added\n///\n", state.ret);
			}
		} else if item_str == "add_doc" {
			state.ret = format!("{}\n///\n/// add_doc comment added\n", state.ret);
		}
	} else {
		if item_str == "-" {
			// special case handling for ->
			state.ret = format!("{}{}", state.ret, item);
		} else if item_str == ">" {
			// special case can't prepend newline for ->
			state.ret = format!("{}{} ", state.ret, item);
		} else {
			state.ret = format!("{}\n{} ", state.ret, item);
		}
	}
	state.counter += 1;
	Ok(())
}

fn process_ident(ident: Ident, state: &mut MacroState) -> Result<(), Error> {
	debug!("ident[{}='{}'", state.counter, ident)?;
	let ident_str = ident.to_string();

	state.ret = format!("{}\n{} ", state.ret, ident);
	if ident_str == "trait" {
		state.in_trait = true;
	}
	state.counter += 1;
	Ok(())
}
