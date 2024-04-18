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
use crate::types::MvcAttrMacroState as AttrMacroState;
use crate::types::MvcItemMacroState as ItemMacroState;
use crate::types::{
	MvcAttrMacroState, MvcAttrState, MvcFnInfo, MvcItemMacroState, MvcItemState, ViewInfo,
};
use bmw_conf::{Configurable, InstanceType};
use bmw_deps::substring::Substring;
use bmw_err::*;
use proc_macro::TokenTree::*;
use proc_macro::{Group, Ident, Literal, Punct, TokenStream, TokenTree};
use std::collections::{HashMap, HashSet};

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

impl AttrMacroState {
	fn new() -> Self {
		Self {
			cur_view_is_pub: false,
			cur_view_is_pub_crate: false,
			expect_command: false,
			expect_equal: false,
			expect_comma: false,
			is_builder: false,
			view_depth: 0,

			views: vec![],
			add_list: vec![],
			macro_list: vec![],
			config_text: "".to_string(),

			counter: 0,
			in_view_list: false,
			in_add_list: false,
			in_macro_list: false,
			in_config: false,
		}
	}
}

impl From<MvcAttrMacroState> for MvcAttrState {
	fn from(attr_state: MvcAttrMacroState) -> Self {
		Self {
			adds: attr_state.add_list,
			is_builder: attr_state.is_builder,
			macros: attr_state.macro_list,
			views: attr_state.views,
			config_text: attr_state.config_text,
		}
	}
}

impl ItemMacroState {
	fn new() -> Self {
		Self {
			counter: 0,
			config_text: "".to_string(),
		}
	}
}

impl From<MvcItemMacroState> for MvcItemState {
	fn from(state: MvcItemMacroState) -> Self {
		Self {
			builder: "new".to_string(),
			macros: vec![],
			fn_list: HashMap::new(),
			struct_name: "test".to_string(),
			views: vec![],
			config_text: state.config_text,
		}
	}
}

#[derive(Debug, PartialEq)]
enum TokenType {
	Ident,
	GroupItem,
	Literal,
	Punct,
}

pub(crate) fn do_derive_mvc(attr: &TokenStream, item: &TokenStream) -> Result<TokenStream, Error> {
	debug!("===========================in mvc proc-macro==========================")?;
	debug!("attrs: {}", attr)?;

	let attr_state = parse_attr(attr)?;
	let attr_state: MvcAttrState = attr_state.into();
	debug!("attr_state={:?}", attr_state)?;

	// do a symantic check as best we can at this phase
	check_attr_state(&attr_state)?;

	// if we don't have any views we can exit
	if attr_state.views.len() == 0 {
		Ok(item.clone())
	} else {
		// TODO: build item state with attr, for now just create one
		let mut item_state = ItemMacroState::new();
		item_state.config_text = attr_state.config_text.clone();

		// we go on to build our updated structures
		// make a mock version for testing

		let mut fn_list = HashMap::new();
		fn_list.insert(
			"Dog".to_string(),
			vec![
				MvcFnInfo {
					fn_name: "bark".to_string(),
					trait_signature: "fn bark(&self) -> Result<(), Error>".to_string(),
					parameter_list: "&self".to_string(),
				},
				MvcFnInfo {
					fn_name: "speak".to_string(),
					trait_signature: "fn speak(&self) -> Result<(), Error>".to_string(),
					parameter_list: "&self".to_string(),
				},
			],
		);

		let item_state = MvcItemState {
			builder: "builder".to_string(),
			struct_name: "MyStruct".to_string(),
			views: vec![ViewInfo {
				name: "Dog".to_string(),
				is_pub: true,
				is_pub_crate: false,
			}],
			macros: vec![InstanceType::Impl, InstanceType::Box],
			config_text: attr_state.config_text,
			fn_list,
		};

		build_updated_stream(item, item_state)
	}
}

fn build_item_state(attr_state: MvcAttrState, item: &TokenStream) -> MvcItemState {
	MvcItemState {
		builder: "new".to_string(),
		fn_list: HashMap::new(),
		struct_name: "test".to_string(),
		views: vec![],
		macros: vec![],
		config_text: attr_state.config_text.clone(),
	}
}

fn check_attr_state(state: &MvcAttrState) -> Result<(), Error> {
	if state.macros.len() > 0 && state.views.len() == 0 {
		return Err(err!(
			ErrKind::IllegalState,
			"there must be at least one view to build a macro"
		));
	}
	if state.adds.len() > 0 && state.views.len() != 0 {
		return Err(err!(
			ErrKind::IllegalState,
			"add and views command may not be in the same mvc tag"
		));
	}

	let mut set = HashSet::new();
	for add in &state.adds {
		if set.contains(add) {
			return Err(err!(
				ErrKind::IllegalState,
				"duplicate in add list: '{}'",
				add
			));
		}
		set.insert(add);
	}

	let mut set = HashSet::new();
	for v in &state.views {
		let name = v.name.to_string();
		if set.contains(&name) {
			return Err(err!(
				ErrKind::IllegalState,
				"duplicate in view list: '{}'",
				v.name
			));
		}
		set.insert(v.name.clone());
	}

	let mut set = HashSet::new();
	for v in &state.macros {
		if set.contains(&v) {
			return Err(err!(
				ErrKind::IllegalState,
				"duplicate in macro list: '{:?}'",
				v
			));
		}
		set.insert(v);
	}

	Ok(())
}

fn parse_attr(attr: &TokenStream) -> Result<MvcAttrState, Error> {
	let mut state = AttrMacroState::new();
	state.expect_command = true;

	let mut exit = false;
	for tree in attr.clone() {
		exit = match tree {
			Ident(ident) => process_attr_ident(ident, &mut state)?,
			Group(group) => process_attr_group(group.clone(), &mut state)?,
			Literal(literal) => process_attr_literal(literal, &mut state)?,
			Punct(punct) => process_attr_punct(punct, &mut state)?,
		};

		if exit {
			break;
		}
	}

	Ok(state.into())
}

fn process_attr_punct(punct: Punct, state: &mut AttrMacroState) -> Result<bool, Error> {
	let ret = process_attr_token(punct.to_string(), TokenType::Punct, state)?;
	state.counter += 1;
	Ok(ret)
}

fn process_attr_literal(literal: Literal, state: &mut AttrMacroState) -> Result<bool, Error> {
	let ret = process_attr_token(literal.to_string(), TokenType::Literal, state)?;
	state.counter += 1;
	Ok(ret)
}

fn process_attr_group(group: Group, state: &mut AttrMacroState) -> Result<bool, Error> {
	if state.in_config {
		let group = group.to_string();
		let group = group.trim();
		if group.len() >= 3 {
			state.config_text = group.substring(1, group.len() - 1).to_string();
		}

		debug!("group with in_config=true: '{}'", group)?;
		Ok(false)
	} else {
		for item in group.stream() {
			let ret = process_attr_group_item(item, state)?;
			if ret == true {
				return Ok(true);
			}
		}
		Ok(false)
	}
}

fn process_attr_group_item(item: TokenTree, state: &mut AttrMacroState) -> Result<bool, Error> {
	let ret = process_attr_token(item.to_string(), TokenType::GroupItem, state)?;
	state.counter += 1;
	Ok(ret)
}

fn process_attr_ident(ident: Ident, state: &mut AttrMacroState) -> Result<bool, Error> {
	let ret = process_attr_token(ident.to_string(), TokenType::Ident, state)?;
	state.counter += 1;
	Ok(ret)
}

fn process_attr_token(
	token: String,
	token_type: TokenType,
	state: &mut AttrMacroState,
) -> Result<bool, Error> {
	debug!(
		"process_attr_token=[{}][{:?}]='{}'",
		state.counter, token_type, token
	)?;

	if state.expect_command && token_type != TokenType::Ident {
		return Err(err!(
			ErrKind::Parse,
			"expected (builder, add, views, or macros), found '{}'",
			token
		));
	}

	match token_type {
		TokenType::Ident => {
			if !state.expect_command {
				return Err(err!(ErrKind::Parse, "unexpected ident found: '{}'", token));
			}
			if token == "builder" {
				state.is_builder = true;
				if state.views.len() > 0 || state.macro_list.len() > 0 || state.add_list.len() > 0 {
					return Err(err!(
						ErrKind::Parse,
						"a builder command cannot be accompanied by any other commands"
					));
				}
			} else if token == "views" {
				state.in_view_list = true;
				state.cur_view_is_pub = false;
				state.cur_view_is_pub_crate = false;
				if state.is_builder {
					return Err(err!(
						ErrKind::Parse,
						"a builder command cannot be accompanied by any other commands"
					));
				}
			} else if token == "add" {
				state.in_add_list = true;
				if state.is_builder {
					return Err(err!(
						ErrKind::Parse,
						"a builder command cannot be accompanied by any other commands"
					));
				}
			} else if token == "macros" {
				state.in_macro_list = true;
				if state.is_builder {
					return Err(err!(
						ErrKind::Parse,
						"a builder command cannot be accompanied by any other commands"
					));
				}
			} else if token == "config" {
				state.in_config = true;
			} else {
				return Err(err!(
					ErrKind::Parse,
					"expected (builder, add, views, or macros), found '{}'",
					token
				));
			}

			debug!("COMMAND={}", token)?;

			state.expect_command = false;
			state.expect_equal = true;
		}
		TokenType::GroupItem => {
			if state.in_macro_list {
				if !state.expect_comma && token == "," {
					return Err(err!(ErrKind::Parse, "duplicate comma found"));
				} else if state.expect_command && token != "," {
					return Err(err!(ErrKind::Parse, "expected comma"));
				}

				if state.expect_comma {
					state.expect_comma = false;
				} else {
					// this is a new macro
					state.macro_list.push(try_into!(token)?);
					state.expect_comma = true;
				}
			} else if state.in_view_list {
				if !state.expect_comma && token == "," {
					return Err(err!(ErrKind::Parse, "duplicate comma found"));
				} else if state.expect_command && token != "," {
					return Err(err!(ErrKind::Parse, "expected comma"));
				}
				if state.expect_comma {
					state.expect_comma = false;
					state.view_depth = 0;
					state.cur_view_is_pub = false;
					state.cur_view_is_pub_crate = false;
				} else {
					if state.view_depth == 0 && token == "pub" {
						state.view_depth += 1;
						state.cur_view_is_pub = true;
					} else if state.view_depth == 1 && token == "(crate)" && state.cur_view_is_pub {
						state.view_depth += 1;
						state.cur_view_is_pub_crate = true;
					} else {
						// we have to have the name of the view here
						if token == "(crate)" {
							return Err(err!(ErrKind::Parse, "unexpected '(crate)'"));
						}

						// add view
						state.views.push(ViewInfo {
							name: token,
							is_pub: state.cur_view_is_pub,
							is_pub_crate: state.cur_view_is_pub_crate,
						});
						state.expect_comma = true;
					}
				}
			} else if state.in_add_list {
				if !state.expect_comma && token == "," {
					return Err(err!(ErrKind::Parse, "duplicate comma found"));
				} else if state.expect_command && token != "," {
					return Err(err!(ErrKind::Parse, "expected comma"));
				}

				if state.expect_comma {
					state.expect_comma = false;
				} else {
					// this is a new add
					state.add_list.push(token);
					state.expect_comma = true;
				}
			} else {
				return Err(err!(
					ErrKind::Parse,
					"unexpected GroupItem found: '{}'",
					token
				));
			}
		}
		TokenType::Punct => {
			if token == "=" {
				if !state.expect_equal {
					return Err(err!(ErrKind::Parse, "unexpected equal found"));
				}
				state.expect_equal = false;
			} else if token == "," {
				// this means the list is done, expect another command
				state.expect_command = true;
				state.in_macro_list = false;
				state.in_view_list = false;
				state.in_add_list = false;
				state.expect_comma = false;
			} else {
				return Err(err!(ErrKind::Parse, "unexpected punct found: '{}'", token));
			}
		}
		TokenType::Literal => {
			return Err(err!(
				ErrKind::Parse,
				"unexpected literal found: '{}'",
				token
			));
		}
	}

	Ok(false)
}

fn build_updated_stream(item: &TokenStream, state: MvcItemState) -> Result<TokenStream, Error> {
	debug!("building updated token stream for {:?}", state)?;

	let ntext = build_additional_code(&state)?;
	let nstrm: TokenStream = map_err!(ntext.parse(), ErrKind::Parse)?;
	let mut ret = item.clone();
	ret.extend(nstrm);
	debug!("ret='{}'", ret.to_string());
	Ok(ret)
}

fn build_options_enum(config_text: &String) -> Result<String, Error> {
	let ret = config_text;
	let ret = format!(
		"{}{}",
		ret,
		do_derive_configurable(map_err!(config_text.parse(), ErrKind::Parse)?).to_string()
	);
	debug!("ret_do_derive='{}'", ret)?;
	Ok(ret)
}

fn build_additional_code(state: &MvcItemState) -> Result<String, Error> {
	let mut ret = build_options_enum(&state.config_text)?;

	for view in &state.views {
		let fn_info = match state.fn_list.get(&view.name) {
			Some(fn_info) => fn_info,
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"expected a fn_list for '{}'",
					view.name
				));
			}
		};
		build_trait(&view.name, fn_info, &mut ret)?;
		build_impl(&view.name, fn_info, &state.struct_name, &mut ret)?;
	}
	ret = format!(
		"{}\npub struct {}Builder {{}}\nimpl {}Builder {{",
		ret, state.struct_name, state.struct_name
	);

	for view in &state.views {
		let fn_info = match state.fn_list.get(&view.name) {
			Some(fn_info) => fn_info,
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"expected a fn_list for '{}'",
					view.name
				));
			}
		};
		ret = format!(
			"{}\npub fn build_{}(configs: Vec<{}>) -> Result<Box<dyn {}>, Error> {{",
			ret, view.name, "MyStructConfigOptions", view.name
		);
		ret = format!("{}\n\tlet config = {}::default();", ret, "MyStructConfig",);
		ret = format!(
			"{}\n\tlet ret: Box<dyn {}> = Box::new({}::{}(config, \"{}\".to_string(), InstanceType::Box)?);",
			ret, view.name, state.struct_name, state.builder, view.name
		);
		ret = format!("{}\n\tOk(ret)", ret);
		ret = format!("{}\n}}", ret);
	}

	ret = format!("{}\n}}", ret);

	Ok(ret)
}

fn build_impl(
	name: &String,
	fn_info: &Vec<MvcFnInfo>,
	struct_name: &String,
	ret: &mut String,
) -> Result<(), Error> {
	*ret = format!("{}\nimpl {} for {} {{", ret, name, struct_name);
	for fn_inf in fn_info {
		*ret = format!(
			"{}\n{} {{ {}::{}({}) }}",
			ret, fn_inf.trait_signature, struct_name, fn_inf.fn_name, fn_inf.parameter_list
		);
	}
	*ret = format!("{}\n}}", ret);

	*ret = format!("{}\nimpl {} for &mut {} {{", ret, name, struct_name);
	for fn_inf in fn_info {
		*ret = format!(
			"{}\n{} {{ {}::{}({}) }}",
			ret, fn_inf.trait_signature, struct_name, fn_inf.fn_name, fn_inf.parameter_list
		);
	}
	*ret = format!("{}\n}}", ret);

	Ok(())
}

fn build_trait(name: &String, fn_info: &Vec<MvcFnInfo>, ret: &mut String) -> Result<(), Error> {
	*ret = format!("{}\npub trait {} {{", ret, name);

	for fn_inf in fn_info {
		*ret = format!("{}\n{};", ret, fn_inf.trait_signature);
	}

	*ret = format!("{}\n}}", ret);
	Ok(())
}

#[cfg(test)]
mod test {
	use bmw_err::*;

	struct MyStructConfig {
		a: usize,
		b: u8,
	}

	struct MyStruct {
		height: usize,
		name: String,
		count: u64,
		config: MyStructConfig,
	}

	impl Default for MyStructConfig {
		fn default() -> Self {
			Self { a: 1, b: 2 }
		}
	}

	//#[mvc(config=MyStructConfig,traits=[pub Cat, pub Dog, pub Human, pub(crate) test)]
	impl MyStruct {
		// reequired method
		fn new(config: MyStructConfig) -> Result<Self, Error> {
			// do any setup logic

			Ok(Self {
				config,
				height: 10,
				name: "joe".to_string(),
				count: 0,
			})
		}

		//#[mvc(add=[Dog, test])]
		fn bark(&self) -> Result<(), Error> {
			println!("ruff");
			Ok(())
		}

		//#[mvc(add=[Cat, test])]
		fn meow(&self) -> Result<(), Error> {
			println!("meow");
			Ok(())
		}

		//#[mvc(add=[Cat, Dog, Human, test])]
		fn sleep(&mut self, duration: usize) -> Result<(), Error> {
			self.name = duration.to_string();
			Ok(())
		}

		//#[mvc(add=[Dog, Human, test])]
		fn jump(&mut self, height: usize) -> usize {
			self.height += height;
			self.update();
			println!("a={},b={}", self.config.a, self.config.b);
			self.height
		}

		fn update(&mut self) {
			self.count += 1;
		}

		//#[mvc(add=[test])]
		fn set_debug_flag(&mut self) {}
	}

	// mvc will build these:
	pub trait Dog {
		// docs would be included here from the MyStruct::bark (same with other fns)
		fn bark(&self) -> Result<(), Error>;
		fn sleep(&mut self, duration: usize) -> Result<(), Error>;
		fn jump(&mut self, height: usize) -> usize;
	}

	impl Dog for MyStruct {
		fn bark(&self) -> Result<(), Error> {
			MyStruct::bark(self)
		}

		fn sleep(&mut self, duration: usize) -> Result<(), Error> {
			MyStruct::sleep(self, duration)
		}

		fn jump(&mut self, height: usize) -> usize {
			MyStruct::jump(self, height)
		}
	}

	impl Dog for &mut MyStruct {
		fn bark(&self) -> Result<(), Error> {
			MyStruct::bark(self)
		}

		fn sleep(&mut self, duration: usize) -> Result<(), Error> {
			MyStruct::sleep(self, duration)
		}

		fn jump(&mut self, height: usize) -> usize {
			MyStruct::jump(self, height)
		}
	}

	pub trait Cat {
		fn meow(&self) -> Result<(), Error>;
		fn sleep(&mut self, duration: usize) -> Result<(), Error>;
	}

	impl Cat for MyStruct {
		fn meow(&self) -> Result<(), Error> {
			MyStruct::meow(self)
		}
		fn sleep(&mut self, duration: usize) -> Result<(), Error> {
			MyStruct::sleep(self, duration)
		}
	}

	impl Cat for &mut MyStruct {
		fn meow(&self) -> Result<(), Error> {
			MyStruct::meow(self)
		}
		fn sleep(&mut self, duration: usize) -> Result<(), Error> {
			MyStruct::sleep(self, duration)
		}
	}

	// add as_dog method
	impl MyStruct {
		pub(crate) fn as_dog(&mut self) -> Box<dyn Dog + '_> {
			let ret: Box<dyn Dog> = Box::new(self);
			ret
		}
		pub(crate) fn as_cat(&mut self) -> Box<dyn Cat + '_> {
			let ret: Box<dyn Cat> = Box::new(self);
			ret
		}
		// .. as_human
	}

	pub enum MyStructConfigOptions {}

	#[doc(hidden)]
	pub struct MyStructBuilder {}

	impl MyStructBuilder {
		pub fn build_dog(_configs: Vec<MyStructConfigOptions>) -> Result<Box<dyn Dog>, Error> {
			let config = MyStructConfig::default();
			let ret: Box<dyn Dog> = Box::new(MyStruct::new(config)?);
			Ok(ret)
		}

		pub(crate) fn build_my_struct(
			_configs: Vec<MyStructConfigOptions>,
		) -> Result<MyStruct, Error> {
			let config = MyStructConfig::default();
			MyStruct::new(config)
		}
		// .. cat, human
	}

	// like our builder structs (this is impl)
	#[macro_export]
	macro_rules! dog {
		() => {{}};
	}

	// box, sync, and sync box version
	#[macro_export]
	macro_rules! dog_box {
		() => {{}};
	}

	// similar for cat and human

	#[test]
	fn test_my_struct_mvc() -> Result<(), Error> {
		let my_struct = MyStructBuilder::build_dog(vec![])?;
		my_struct.bark()?;

		let mut my_struct = MyStructBuilder::build_my_struct(vec![])?;

		my_struct.as_dog().bark()?;
		my_struct.as_cat().meow()?;

		Ok(())
	}
}
