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

use bmw_base::*;
use bmw_deps::convert_case::{Case, Casing};
use bmw_deps::substring::Substring;
use bmw_deps::syn;
use bmw_deps::syn::{parse_str, Expr, Type};
use proc_macro::TokenTree::{Group, Ident, Literal, Punct};
use proc_macro::{Delimiter, Spacing, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

static CHECK_RECURSION_CONST_PREFIX: &str =
	"if bmw_core::is_recursive() { panic!(\"Recursion detected! Perhaps ";
static CHECK_RECURSION_CONST_SUFFIX: &str = " is not implemented?\"); }";

#[derive(Debug, PartialEq, Clone)]
enum Visibility {
	Pub,
	PubCrate,
	Private,
}

#[derive(Clone, Debug)]
struct Fn {
	name: String,
	span: Span,
	name_span: Option<Span>,
	return_list: String,
	param_list: String,
	view_list: Vec<String>,
	param_names: Vec<String>,
	param_types: Vec<String>,
	param_type_spans: Vec<Span>,
	prev_token_is_joint: bool,
	expect_dash_return_list: bool,
	expect_gt_return_list: bool,
	return_list_span: Option<Span>,
	generic_string: String,
	comments: Vec<String>,
	as_fn: Option<String>,
}

impl Fn {
	#[cfg(not(tarpaulin_include))]
	fn new(span: Span) -> Self {
		Self {
			span,
			name_span: None,
			return_list_span: None,
			name: "".to_string(),
			return_list: "".to_string(),
			param_list: "".to_string(),
			view_list: vec![],
			param_names: vec![],
			param_types: vec![],
			param_type_spans: vec![],
			prev_token_is_joint: false,
			expect_dash_return_list: false,
			expect_gt_return_list: false,
			generic_string: "".to_string(),
			comments: vec![],
			as_fn: None,
		}
	}
}

#[derive(Clone, Debug)]
struct Var {
	name: String,
	type_str: String,
	span: Span,
	prev_token_is_joint: bool,
}

impl Var {
	#[cfg(not(tarpaulin_include))]
	fn new(name: String, span: Span) -> Self {
		Self {
			name,
			type_str: "".to_string(),
			span,
			prev_token_is_joint: false,
		}
	}
}

impl Display for FieldType {
	#[cfg(not(tarpaulin_include))]
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		match self {
			FieldType::Usize => write!(f, "[`usize`]")?,
			FieldType::U8 => write!(f, "[`u8`]")?,
			FieldType::U16 => write!(f, "[`u16`]")?,
			FieldType::U32 => write!(f, "[`u32`]")?,
			FieldType::U64 => write!(f, "[`u64`]")?,
			FieldType::U128 => write!(f, "[`u128`]")?,
			FieldType::Bool => write!(f, "[`bool`]")?,
			FieldType::String => write!(f, "[`String`]")?,
			FieldType::Configurable => write!(f, "[`Configurable`]")?,
			FieldType::VecUsize => write!(f, "[`usize`]")?,
			FieldType::VecBool => write!(f, "[`bool`]")?,
			FieldType::VecU8 => write!(f, "[`u8`]")?,
			FieldType::VecU16 => write!(f, "[`u16`]")?,
			FieldType::VecU32 => write!(f, "[`u32`]")?,
			FieldType::VecU64 => write!(f, "[`u64`]")?,
			FieldType::VecU128 => write!(f, "[`u128`]")?,
			FieldType::VecString => write!(f, "[`String`]")?,
			FieldType::VecConfigurable => write!(f, "[`Configurable`]")?,
		}
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq)]
enum FieldType {
	Usize,
	U8,
	U16,
	U32,
	U64,
	U128,
	Bool,
	String,
	Configurable,
	VecUsize,
	VecBool,
	VecU8,
	VecU16,
	VecU32,
	VecU64,
	VecU128,
	VecString,
	VecConfigurable,
}

#[derive(Clone, Debug)]
struct Const {
	name: String,
	field_type: Option<FieldType>,
	field_string: Option<String>,
	value_str: String,
	span: Span,
	prev_token_is_joint: bool,
	comments: Vec<String>,
}

impl Const {
	#[cfg(not(tarpaulin_include))]
	fn new(name: String, span: Span) -> Self {
		Self {
			name,
			value_str: "".to_string(),
			field_type: None,
			field_string: None,
			span,
			prev_token_is_joint: false,
			comments: vec![],
		}
	}

	#[cfg(not(tarpaulin_include))]
	fn is_multi(&self) -> bool {
		match &self.field_type {
			Some(f) => match f {
				FieldType::VecUsize => true,
				FieldType::VecBool => true,
				FieldType::VecString => true,
				FieldType::VecConfigurable => true,
				FieldType::VecU8 => true,
				FieldType::VecU16 => true,
				FieldType::VecU32 => true,
				FieldType::VecU64 => true,
				FieldType::VecU128 => true,
				_ => false,
			},
			None => false,
		}
	}
}

#[derive(Clone, Debug)]
struct Pub {
	name: String,
	span: Span,
	macro_name: String,
	comments: Vec<String>,
	no_example: bool,
}

impl Pub {
	#[cfg(not(tarpaulin_include))]
	fn new(name: String, span: Span, comments: Vec<String>) -> Self {
		Self {
			name: name.clone(),
			span,
			macro_name: name,
			comments,
			no_example: false,
		}
	}
}

#[derive(Clone)]
struct PubCrate {
	name: String,
	span: Span,
	macro_name: String,
}

impl PubCrate {
	#[cfg(not(tarpaulin_include))]
	fn new(name: String, span: Span) -> Self {
		Self {
			name: name.clone(),
			span,
			macro_name: name,
		}
	}
}

#[derive(Clone)]
struct CloneItem {
	name: String,
	span: Span,
}

struct SpanError {
	span: Span,
	msg: String,
}

#[derive(Debug)]
enum ItemState {
	Base,
	WantsCrateOrImpl,
	WantsGeneric1OrName,
	WantsImpl,
	WantsGeneric2WhereOrBrace,
	WantsWhereClause,
	WantsGeneric2,
	WantsWhereOrBrace,
	WantsName,
	WantsFn,
	WantsFnName,
	WantsAppendFn,
	Complete,
}

enum State {
	Base,
	NoSync,
	NoSend,
	WantsSemi,
	Pub,
	Module,
	Const,
	Var,
	ViewList,
	WantsPubAs,
	WantsPubIdentifier,
	WantsPubComma,
	WantsConstColon,
	WantsConstFieldType,
	WantsConstEqual,
	WantsConstValue,
	WantsConstLt,
	WantsConstFieldTypeVec,
	WantsConstGt,
	WantsVarColon,
	WantsVarType,
	WantsViewListIdentifier,
	WantsViewListComma,
	WantsViewListFn,
	WantsFnAs,
	WantsViewListFnName,
	WantsComment,
	WantsViewListParamList,
	WantsViewListReturnList,
	WantsViewListGenerics,
	Clone,
	WantsCloneComma,
}

struct StateMachine {
	debug: bool,
	state: State,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	module: Option<String>,
	cur_is_pub_crate: bool,
	in_generic2: bool,
	pub_views: Vec<Pub>,
	pub_crate_views: Vec<PubCrate>,
	cur_const: Option<Const>,
	cur_var: Option<Var>,
	cur_fn: Option<Fn>,
	const_list: Vec<Const>,
	var_list: Vec<Var>,
	fn_list: Vec<Fn>,
	item_state: ItemState,
	class_name: Option<String>,
	generic1: Option<String>,
	generic2: Option<String>,
	where_clause: Option<String>,
	class_is_pub: bool,
	class_is_pub_crate: bool,
	prev_is_joint: bool,
	impl_fns: Vec<String>,
	clone_list: Vec<CloneItem>,
	builder_fn: String,
	cur_fn_str: String,
	in_builder: bool,
	ret: TokenStream,
	found_builder: bool,
	no_sync: bool,
	no_send: bool,
	cur_comments: Vec<String>,
}

impl StateMachine {
	#[cfg(not(tarpaulin_include))]
	fn new(debug: bool) -> Self {
		Self {
			debug,
			found_builder: false,
			state: State::Base,
			item_state: ItemState::Base,
			span: None,
			error_list: vec![],
			module: None,
			cur_is_pub_crate: false,
			in_generic2: false,
			in_builder: false,
			pub_views: vec![],
			pub_crate_views: vec![],
			cur_const: None,
			cur_var: None,
			cur_fn: None,
			const_list: vec![],
			var_list: vec![],
			clone_list: vec![],
			fn_list: vec![],
			class_name: None,
			generic1: None,
			generic2: None,
			where_clause: None,
			class_is_pub: false,
			class_is_pub_crate: false,
			prev_is_joint: false,
			builder_fn: "".to_string(),
			cur_fn_str: "".to_string(),
			impl_fns: vec![],
			ret: TokenStream::new(),
			no_sync: false,
			no_send: false,
			cur_comments: vec![],
		}
	}

	#[cfg(not(tarpaulin_include))]
	fn derive(&mut self, attr: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
		self.parse_attr(attr)?;

		if self.debug {
			println!("const list:");
			for c in &self.const_list {
				println!("{:?}", c);
			}

			println!("var list:");
			for v in &self.var_list {
				println!("{:?}", v);
			}

			println!("fn list:");
			for f in &self.fn_list {
				println!("{:?}", f);
			}

			println!("pub list:");
			for p in &self.pub_views {
				println!("{:?}", p);
			}
		}
		self.item_state = ItemState::Base;
		self.parse_item(item)?;

		if self.debug {
			println!(
				"class_name={:?},pub={},pub(crate)={},generics1={:?},generic2={:?},where={:?}",
				self.class_name,
				self.class_is_pub,
				self.class_is_pub_crate,
				self.generic1,
				self.generic2,
				self.where_clause,
			);
			println!("builder='{}'", self.builder_fn);
			println!("other fns:");
			for impl_fn in &self.impl_fns {
				println!("{:?}", impl_fn);
			}
		}

		if self.error_list.len() > 0 {
			self.print_errors()?;
		}

		self.check_semantics()?;

		if self.error_list.len() > 0 {
			self.print_errors()?;
		}

		self.generate_code()?;

		Ok(self.ret.clone())
	}

	#[cfg(not(tarpaulin_include))]
	fn check_semantics(&mut self) -> Result<(), Error> {
		let mut trait_views = self.build_trait_views()?;

		for c in self.clone_list.clone() {
			if trait_views.get(&c.name).is_none() {
				self.span = Some(c.span);
				self.append_error(&format!("view '{}' not found", c.name))?;
			}
		}

		for (k, v) in trait_views.clone() {
			trait_views.insert(format!("{}_send_box", k), v.clone());
			trait_views.insert(format!("{}_send", k), v.clone());

			trait_views.insert(format!("{}_sync", k), v.clone());
			trait_views.insert(format!("{}_sync_box", k), v.clone());

			trait_views.insert(format!("{}_box", k), v);
		}

		for view in self.pub_crate_views.clone() {
			if trait_views.get(&view.name).is_none() {
				self.span = Some(view.span);
				self.append_error(&format!("unknown view"))?;
			}
		}

		for view in self.pub_views.clone() {
			if trait_views.get(&view.name).is_none() {
				self.span = Some(view.span);
				self.append_error(&format!("unknown view"))?;
			}
		}

		let mut set = HashSet::new();
		for v in self.var_list.clone() {
			if set.contains(&v.name) {
				self.span = Some(v.span);
				self.append_error(&format!("duplicate var. {} is already defined.", v.name))?;
			}
			set.insert(v.name.clone());
		}

		let mut set = HashSet::new();
		for c in self.const_list.clone() {
			if set.contains(&c.name) {
				self.span = Some(c.span);
				self.append_error(&format!("duplicate const. {} is already defined.", c.name))?;
			}
			set.insert(c.name.clone());
		}

		let mut set = HashSet::new();
		for f in self.fn_list.clone() {
			if set.contains(&f.name) {
				self.span = match f.name_span {
					Some(ns) => Some(ns),
					None => Some(f.span),
				};
				self.append_error(&format!("duplicate fn. '{}' is already defined.", f.name))?;
			}
			set.insert(f.name.clone());
		}

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn build_generic1(&self) -> Result<String, Error> {
		Ok(match &self.generic1 {
			Some(generic) => format!("<{}>", generic),
			None => "".to_string(),
		})
	}

	#[cfg(not(tarpaulin_include))]
	fn build_generic2(&self) -> Result<String, Error> {
		Ok(match &self.generic2 {
			Some(generic) => format!("<{}>", generic),
			None => "".to_string(),
		})
	}

	#[cfg(not(tarpaulin_include))]
	fn build_where(&self) -> Result<String, Error> {
		Ok(match &self.where_clause {
			Some(where_clause) => format!("where {}", where_clause),
			None => "".to_string(),
		})
	}

	#[cfg(not(tarpaulin_include))]
	fn build_var_params_replace(&self) -> Result<String, Error> {
		let mut replace = "".to_string();
		for item in &self.var_list {
			replace = format!("{}{}: {},\n\t", replace, item.name, item.type_str);
		}
		Ok(replace)
	}

	#[cfg(not(tarpaulin_include))]
	fn const_type_string(&self, item: &Const) -> Result<String, Error> {
		let configurable_name = match &item.field_string {
			Some(field_string) => field_string.clone(),
			None => "".to_string(),
		};
		let vec_configurable_name = match &item.field_string {
			Some(field_string) => format!("Vec<{}>", field_string.clone()),
			None => "".to_string(),
		};
		let type_str = match &item.field_type {
			Some(field_type) => match field_type {
				FieldType::Usize => "usize",
				FieldType::String => "String",
				FieldType::U8 => "u8",
				FieldType::Bool => "bool",
				FieldType::U16 => "u16",
				FieldType::U32 => "u32",
				FieldType::U64 => "u64",
				FieldType::U128 => "u128",
				FieldType::VecUsize => "Vec<usize>",
				FieldType::VecString => "Vec<String>",
				FieldType::VecU8 => "Vec<u8>",
				FieldType::VecBool => "Vec<bool>",
				FieldType::VecU16 => "Vec<u16>",
				FieldType::VecU32 => "Vec<u32>",
				FieldType::VecU64 => "Vec<u64>",
				FieldType::VecU128 => "Vec<u128>",
				FieldType::VecConfigurable => &vec_configurable_name,
				FieldType::Configurable => &configurable_name,
			},
			None => ret_err!(CoreErrorKind::Parse, "unexpected type is none"),
		};
		Ok(type_str.to_string())
	}

	#[cfg(not(tarpaulin_include))]
	fn build_const_params_replace(&self) -> Result<String, Error> {
		let mut replace = "".to_string();
		for item in &self.const_list {
			let type_str = self.const_type_string(item)?;
			replace = format!("{}{}: {},\n\t", replace, item.name, type_str);
		}
		Ok(replace)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_structs(&mut self, template: &String) -> Result<String, Error> {
		let mut template = if self.clone_list.len() > 0 {
			template.replace("${CLONE}", "#[derive(Clone)]").to_string()
		} else {
			template.replace("${CLONE}", "").to_string()
		};
		template = template.replace("${NAME}", &self.class_name.as_ref().unwrap());
		template = template.replace("${GENERICS2}", &self.build_generic2()?);
		template = template.replace("${WHERE}", &self.build_where()?);
		template = template.replace("${GENERICS1}", &self.build_generic1()?);
		template = template.replace("${VAR_PARAMS}", &self.build_var_params_replace()?);
		template = template.replace("${CONST_PARAMS}", &self.build_const_params_replace()?);
		if self.class_is_pub_crate {
			template = template.replace("${CLASS_VISIBILITY}", "pub(crate)");
		} else if self.class_is_pub {
			template = template.replace("${CLASS_VISIBILITY}", "pub");
		} else {
			template = template.replace("${CLASS_VISIBILITY}", "");
		}

		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn build_trait_views(&self) -> Result<HashMap<String, Vec<Fn>>, Error> {
		let mut ret: HashMap<String, Vec<Fn>> = HashMap::new();

		for fn_info in &self.fn_list {
			for view in &fn_info.view_list {
				match ret.get_mut(view) {
					Some(v) => {
						v.push(fn_info.clone());
					}
					None => {
						ret.insert(view.clone(), vec![fn_info.clone()]);
					}
				}
			}
		}

		Ok(ret)
	}

	#[cfg(not(tarpaulin_include))]
	fn build_view_pub_map(&self) -> Result<HashMap<String, (Visibility, String)>, Error> {
		let mut ret = HashMap::new();
		let mut pub_view_set: HashSet<String> = HashSet::new();
		let mut pub_crate_view_set: HashSet<String> = HashSet::new();
		let mut pub_view_name_map: HashMap<String, String> = HashMap::new();
		for pub_view in &self.pub_views {
			pub_view_name_map.insert(pub_view.name.clone(), pub_view.macro_name.clone());
			pub_view_set.insert(pub_view.name.clone());
		}
		for pub_crate_view in &self.pub_crate_views {
			pub_view_name_map.insert(
				pub_crate_view.name.clone(),
				pub_crate_view.macro_name.clone(),
			);
			pub_crate_view_set.insert(pub_crate_view.name.clone());
		}

		for fn_info in &self.fn_list {
			for v in &fn_info.view_list {
				let mut trait_visibility = Visibility::Private;

				let view = v.clone();

				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view = format!("{}_box", v);
				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view = format!("{}_send", v);
				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view = format!("{}_send_box", v);
				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view = format!("{}_sync", v);
				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view = format!("{}_sync_box", v);
				if pub_crate_view_set.contains(&view) {
					if trait_visibility == Visibility::Private {
						trait_visibility = Visibility::PubCrate;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::PubCrate,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				} else if pub_view_set.contains(&view) {
					if trait_visibility != Visibility::Pub {
						trait_visibility = Visibility::Pub;
					}
					ret.insert(
						view.clone(),
						(
							Visibility::Pub,
							pub_view_name_map.get(&view).unwrap_or(&view).clone(),
						),
					);
				}

				let view_pascal = v.to_case(Case::Pascal);
				match trait_visibility {
					Visibility::Pub => {
						ret.insert(
							view_pascal,
							(
								Visibility::Pub,
								pub_view_name_map.get(&view).unwrap_or(&view).clone(),
							),
						);
					}
					Visibility::PubCrate => {
						ret.insert(
							view_pascal,
							(
								Visibility::PubCrate,
								pub_view_name_map.get(&view).unwrap_or(&view).clone(),
							),
						);
					}
					Visibility::Private => {}
				}
			}
		}

		Ok(ret)
	}

	#[cfg(not(tarpaulin_include))]
	fn get_const_default_inits(&self) -> Result<String, Error> {
		let mut ret = "".to_string();
		for const_value in &self.const_list {
			ret = format!(
				"{}let {} = {};\n\t\t",
				ret, const_value.name, const_value.value_str
			);
		}
		Ok(ret)
	}

	#[cfg(not(tarpaulin_include))]
	fn get_const_default_params(&self) -> Result<String, Error> {
		let mut ret = "".to_string();
		for const_value in &self.const_list {
			ret = format!("{}{},\n\t\t\t", ret, const_value.name);
		}
		Ok(ret)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_const_default(&mut self, template: &String) -> Result<String, Error> {
		let mut replace = include_str!("../templates/class_const_default.txt").to_string();
		replace = replace.replace("${NAME}", &self.class_name.as_ref().unwrap());
		replace = replace.replace("${DEFAULT_INITS}", &self.get_const_default_inits()?);
		replace = replace.replace("${DEFAULT_PARAMS}", &self.get_const_default_params()?);
		let template = template.replace("${CONST_DEFAULT}", &replace);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_impl_struct(&mut self, template: &String) -> Result<String, Error> {
		let mut replace = include_str!("../templates/class_impl_struct_template.txt").to_string();
		replace = replace.replace("${GENERIC1}", &self.build_generic1()?);
		replace = replace.replace("${GENERIC2}", &self.build_generic2()?);
		replace = replace.replace("${WHERE}", &self.build_where()?);
		replace = replace.replace("${NAME}", &self.class_name.as_ref().unwrap());
		let template = template.replace("${IMPL_STRUCT}", &replace);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_impl_var(&mut self, template: &String) -> Result<String, Error> {
		let class_name = &self.class_name.as_ref().unwrap();
		let mut replace = format!(
			"impl {} {}Var {}{} {{\n",
			self.build_generic1()?,
			class_name,
			self.build_generic2()?,
			self.build_where()?
		);
		let get_template = include_str!("../templates/class_get_mut_template.txt").to_string();
		for c in &self.var_list {
			let type_str = &c.type_str;
			replace = format!(
				"{}\n{}",
				replace,
				get_template
					.replace("${PARAM_NAME}", &c.name)
					.replace("${PARAM_TYPE}", &type_str)
			);
		}
		// add builder
		replace = format!("{}\t{}", replace, self.builder_fn);
		replace = format!("{}}}", replace);
		let template = template.replace("${IMPL_VAR}", &replace);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_impl_const(&mut self, template: &String) -> Result<String, Error> {
		let mut replace = format!("impl {}Const {{", &self.class_name.as_ref().unwrap());
		let get_template = include_str!("../templates/class_get_template.txt").to_string();
		for c in &self.const_list {
			let type_str = self.const_type_string(c)?;
			replace = format!(
				"{}\n{}",
				replace,
				get_template
					.replace("${PARAM_NAME}", &c.name)
					.replace("${PARAM_TYPE}", &type_str)
			);
		}
		replace = format!("{}}}", replace);
		let template = template.replace("${IMPL_CONST}", &replace);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_traits(
		&mut self,
		template: &String,
		views: &HashMap<String, Vec<Fn>>,
		view_pub_map: &HashMap<String, (Visibility, String)>,
	) -> Result<String, Error> {
		let mut trait_text = "".to_string();
		let mut clone_set = HashSet::new();
		for c in &self.clone_list {
			clone_set.insert(c.name.clone());
		}

		for (k, v) in views {
			let mut comment_map = HashMap::new();
			for item in &mut self.pub_views {
				if item.name.find(k) == Some(0) {
					// TODO: improve this, could have some false positives
					let mut vec = vec![];
					for comment in item.comments.clone() {
						if comment.find("@noexample").is_none() {
							vec.push(comment);
						} else {
							item.no_example = true;
						}
					}
					comment_map.insert(k, vec);
				}
			}
			let clone_text = if clone_set.contains(k) {
				": bmw_core::dyn_clone::DynClone"
			} else {
				""
			};
			let trait_name = k.to_case(Case::Pascal);
			let vis = view_pub_map.get(&trait_name);
			let vis = match vis {
				Some(vis) => match vis.0 {
					Visibility::Pub => "pub",
					Visibility::PubCrate => "pub(crate)",
					Visibility::Private => "",
				},
				None => "",
			};

			match comment_map.get(k) {
				Some(comments) => {
					for comment in comments {
						trait_text = format!("{}\n#[doc={}]", trait_text, comment);
					}
				}
				None => {}
			}
			trait_text = format!(
				"{}\n{} trait {} {} {} {} {{",
				trait_text,
				vis,
				trait_name,
				self.build_generic2()?,
				clone_text,
				self.build_where()?,
			);
			for fn_info in v {
				trait_text = format!("{}\n#[document]", trait_text);
				for comment in &fn_info.comments {
					trait_text = format!("{}\n\t#[doc={}]", trait_text, comment);
				}
				trait_text = format!(
					"{}\nfn {}({}) -> {};",
					trait_text, fn_info.name, fn_info.param_list, fn_info.return_list
				);
			}

			// add get_configurables
			trait_text = format!(
				"{}\n#[doc(hidden)] fn configurable_mut(&mut self) -> &mut dyn Configurable;",
				trait_text
			);
			trait_text = format!(
				"{}\n#[doc(hidden)] fn configurable(&self) -> &dyn Configurable;",
				trait_text
			);
			trait_text = format!("{}\n}}", trait_text);
		}
		let template = template.replace("${TRAITS}", &trait_text);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_trait_impl(
		&mut self,
		template: &String,
		views: &HashMap<String, Vec<Fn>>,
	) -> Result<String, Error> {
		let mut clone_set = HashSet::new();
		for c in &self.clone_list {
			clone_set.insert(c.name.clone());
		}
		let mut trait_impl = "".to_string();
		let class_name = &self.class_name.as_ref().unwrap();
		for (k, v) in views {
			let trait_name = k.to_case(Case::Pascal);

			// trait implementation
			trait_impl = format!(
				"{}\nimpl {} {} {} for {} {}{} {{",
				trait_impl,
				self.build_generic1()?,
				trait_name,
				self.build_generic1()?,
				class_name,
				self.build_generic2()?,
				self.build_where()?,
			);
			for fn_info in v {
				trait_impl = format!(
					"{}\n\tfn {}({}) -> {} {{",
					trait_impl, fn_info.name, fn_info.param_list, fn_info.return_list
				);
				let mut param_names = "self".to_string();
				for i in 1..fn_info.param_names.len() {
					param_names = format!("{}, {}", param_names, fn_info.param_names[i]);
				}

				trait_impl = format!(
					"{}\n\t\t{}\n\t\t{}::{}({})",
					trait_impl,
					format!(
						"{}{}::{}{}",
						CHECK_RECURSION_CONST_PREFIX,
						class_name,
						match &fn_info.as_fn {
							Some(as_fn) => as_fn,
							None => &fn_info.name,
						},
						CHECK_RECURSION_CONST_SUFFIX
					),
					class_name,
					match &fn_info.as_fn {
						Some(as_fn) => as_fn,
						None => &fn_info.name,
					},
					param_names
				);

				trait_impl = format!("{}\n\t}}", trait_impl);
			}
			trait_impl = format!(
                            "{}\nfn configurable_mut(&mut self) -> &mut dyn Configurable {{ &mut self._hidden_const_struct }}",
				trait_impl
			);
			trait_impl = format!(
				"{}\nfn configurable(&self) -> &dyn Configurable {{ &self._hidden_const_struct }}",
				trait_impl
			);

			trait_impl = format!("{}\n}}", trait_impl);

			if !clone_set.contains(k) {
				// trait implementation for &mut
				trait_impl = format!(
					"{}\nimpl {} {} {} for &mut {} {}{} {{",
					trait_impl,
					self.build_generic1()?,
					trait_name,
					self.build_generic1()?,
					class_name,
					self.build_generic2()?,
					self.build_where()?,
				);
				for fn_info in v {
					trait_impl = format!(
						"{}\n\tfn {}({}) -> {} {{",
						trait_impl, fn_info.name, fn_info.param_list, fn_info.return_list
					);
					let mut param_names = "self".to_string();
					for i in 1..fn_info.param_names.len() {
						param_names = format!("{}, {}", param_names, fn_info.param_names[i]);
					}
					trait_impl = format!(
						"{}\n\t\t{}\n\t\t{}::{}({})",
						trait_impl,
						format!(
							"{}{}::{}{}",
							CHECK_RECURSION_CONST_PREFIX,
							class_name,
							fn_info.name,
							CHECK_RECURSION_CONST_SUFFIX
						),
						class_name,
						fn_info.name,
						param_names
					);
					trait_impl = format!("{}\n\t}}", trait_impl);
				}

				trait_impl = format!(
                            "{}\nfn configurable_mut(&mut self) -> &mut dyn Configurable {{ &mut self._hidden_const_struct }}",
                                trait_impl
                        );
				trait_impl = format!(
                            "{}\nfn configurable(&self) -> &dyn Configurable {{ &self._hidden_const_struct }}",
                                trait_impl
                        );

				trait_impl = format!("{}\n}}", trait_impl);
			} else {
				trait_impl = format!(
					"{}\nbmw_deps::dyn_clone::clone_trait_object!({}{}{}{});",
					trait_impl,
					self.build_generic1()?,
					trait_name,
					self.build_generic2()?,
					self.build_where()?
				);
			}
		}
		let template = template.replace("${TRAIT_IMPL}", &trait_impl);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn escape(&self, s: &String) -> String {
		s.replace("\"", "\\\"").to_string()
	}

	#[cfg(not(tarpaulin_include))]
	fn build_comments(
		&self,
		is_macro: bool,
		is_send: bool,
		is_sync: bool,
		is_box: bool,
		trait_name: String,
		macro_name: String,
		class_name: &String,
	) -> Result<String, Error> {
		let mut no_example = true;
		for pub_view in &self.pub_views {
			if pub_view.name == macro_name {
				no_example = pub_view.no_example;
			}
		}
		let fmt = if is_send && is_box {
			"_send_box"
		} else if is_send {
			"_send"
		} else if is_sync && is_box {
			"_sync_box"
		} else if is_sync {
			"_sync"
		} else if is_box {
			"_box"
		} else {
			""
		};
		let builder_fn_name = trait_name.to_case(Case::Snake);
		let mut comment_text = format!("#[doc=\"Builds an instance of the [`{}`]\"]\n", trait_name);
		comment_text = format!(
			"{}#[doc=\"trait using the specified input parameters.\"]\n",
			comment_text
		);

		comment_text = format!("{}\n#[doc=\"# Input Parameters\"]", comment_text);
		let comment_text = format!(
			"{}#[doc=\"| Parameter | Multi [^1] | Description | Default Value |\"]\n",
			comment_text
		);
		let mut comment_text = format!("{}#[doc=\"|---|---|---|---|\"]\n", comment_text);
		for c in &self.const_list {
			let mut comments = "".to_string();
			for comment in &c.comments {
				let comment = self.dequote(comment);
				let comment = comment.trim();
				if comments.len() == 0 {
					comments = format!("{}{}", comments, comment);
				} else {
					comments = format!("{} {}", comments, comment);
				}
			}
			if c.comments.len() == 0 {
				comments = "TODO: document this parameter.".to_string();
			}
			comment_text = format!(
				"{}\n#[doc=\"{}({}) | {} | {} | {}<br/>\"]",
				comment_text,
				c.name.to_case(Case::Pascal),
				c.field_type.as_ref().unwrap(),
				if c.is_multi() { "yes" } else { "no" },
				comments,
				self.escape(&c.value_str),
			);
		}

		comment_text = format!("{}\n#[doc=\"# Return\n\"]", comment_text);

		if is_send && is_box {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<[`Box`]<dyn [`{}`] + [`Send`]>, [`Error`]>\n\"]",
				comment_text, trait_name
			);
		} else if is_send {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<impl [`{}`] + [`Send`], [`Error`]>\n\"]",
				comment_text, trait_name
			);
		} else if is_sync && is_box {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<[`Box`]<dyn [`{}`] + [`Send`] + [`Sync`]>, [`Error`]>\n\"]",
				comment_text, trait_name
			);
		} else if is_sync {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<impl [`{}`] + [`Send`] + [`Sync`], [`Error`]>\n\"]",
				comment_text, trait_name
			);
		} else if is_box {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<[`Box`]<dyn [`{}`]>, [`Error`]>\n\"]",
				comment_text, trait_name
			);
		} else {
			comment_text = format!(
				"{}\n#[doc=\"[`Result`]<impl [`{}`], [`Error`]>\n\"]",
				comment_text, trait_name
			);
		}

		comment_text = format!("{}\n#[doc=\"# Errors\n\"]", comment_text);
		if is_macro {
			comment_text = format!("{}\n#[doc=\"[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind.\"]", comment_text);
		} else {
			comment_text = format!(
				"{}\n#[doc=\"[`CoreErrorKind::Configuration`] - If the configuration is invalid.<br/>\"]",
				comment_text
			);
			comment_text = format!("{}\n#[doc=\"[`CoreErrorKind::Builder`] - Errors returned by the builder are wrapped in this error kind.\"]", comment_text);
		}
		comment_text = format!("{}\n#[doc=\"# Also see\"]", comment_text);
		comment_text = format!("{}\n#[doc=\"[`{}`]<br/>\"]", comment_text, trait_name);
		if is_macro {
			comment_text = format!(
				"{}\n#[doc=\"[`{}Builder::build_{}{}`]\"]",
				comment_text, class_name, builder_fn_name, fmt
			);
		} else {
			comment_text = format!("{}\n#[doc=\"[`crate::{}`]\"]", comment_text, macro_name);
		}

		let crate_name = std::env::var("CARGO_PKG_NAME")?;

		if !no_example {
			comment_text = format!("{}\n#[doc=\"# Examples\"]", comment_text);
			comment_text = format!("{}\n#[doc=\"```\"]", comment_text);
			comment_text = format!("{}\n#[doc=\" use bmw_core::*;\"]", comment_text);

			match &self.module {
				Some(module) => {
					comment_text = format!("{}\n#[doc=\" use {}::*;\"]", comment_text, module,);
					comment_text = format!(
						"{}\n#[doc=\" use {}::{};\"]",
						comment_text, crate_name, macro_name
					);
				}
				None => {
					comment_text = format!("{}\n#[doc=\" use {}::*;\"]", comment_text, crate_name,);
				}
			}

			comment_text = format!("{}\n#[doc=\"\"]", comment_text);
			comment_text = format!(
				"{}\n#[doc=\" fn main() -> Result<(), Error> {{\"]",
				comment_text
			);

			comment_text = format!(
				"{}#[doc=\"     // build an instance of {} with default parameters.\"]\n",
				comment_text, trait_name
			);

			if is_macro {
				comment_text = format!(
					"{}\n#[doc=\"     let mut object = {}!()?;\"]",
					comment_text, macro_name
				);
			} else {
				comment_text = format!(
					"{}\n#[doc=\"     let mut object = {}Builder::build_{}{}(vec![])?;\"]",
					comment_text, class_name, builder_fn_name, fmt
				);
			}

			comment_text = format!("{}\n#[doc=\"\"]", comment_text);

			comment_text = format!(
			"{}#[doc=\"     // build an instance of {} with parameters explicitly specified.\"]\n",
			comment_text, trait_name
		);

			if is_macro {
				comment_text = format!(
					"{}#[doc=\"\tlet object = {}!(\"]\n",
					comment_text, macro_name
				);
			} else {
				comment_text = format!(
					"{}#[doc=\"\tlet object = {}Builder::build_{}{}(vec![\"]\n",
					comment_text, class_name, builder_fn_name, fmt
				);
			}

			for param in &self.const_list {
				let pascal = param.name.to_case(Case::Pascal);
				let default_value = param.value_str.clone();
				let default_value = default_value.trim();
				let default_value = self.escape(&default_value.to_string());

				if !param.is_multi()
					&& param.field_type != Some(FieldType::Configurable)
					&& param.field_type != Some(FieldType::String)
				{
					// bypass Vec & Configurable
					comment_text = format!(
						"{}#[doc=\"         {}({}),\"]\n",
						comment_text, pascal, default_value
					);
				} else if param.field_type == Some(FieldType::String) {
					comment_text = format!(
						"{}#[doc=\"         {}(&{}),\"]\n",
						comment_text, pascal, default_value
					);
				} else if param.field_type == Some(FieldType::Configurable) {
					comment_text = format!(
						"{}#[doc=\"         {}(Box::new({})),\"]\n",
						comment_text, pascal, default_value
					);
				}
			}
			if is_macro {
				comment_text = format!("{}#[doc=\"     )?;\"]\n", comment_text);
			} else {
				comment_text = format!("{}#[doc=\"     ])?;\"]\n", comment_text);
			}

			comment_text = format!("{}\n#[doc=\"\"]", comment_text);
			comment_text = format!("{}\n#[doc=\"     Ok(())\"]", comment_text);
			comment_text = format!("{}\n#[doc=\" }}\"]", comment_text);

			comment_text = format!("{}\n#[doc=\"```\"]", comment_text);
		}

		comment_text = format!(
			"{}\n#[doc=\"[^1]: Multiple values allowed.\n\"]",
			comment_text
		);

		Ok(comment_text)
	}

	#[cfg(not(tarpaulin_include))]
	fn update_macros(
		&mut self,
		template: &String,
		views: &HashMap<String, Vec<Fn>>,
		view_pub_map: &HashMap<String, (Visibility, String)>,
	) -> Result<String, Error> {
		let class_name = &self.class_name.as_ref().unwrap();
		let macro_template = include_str!("../templates/class_macro_template.txt").to_string();
		let mut macro_builder = "".to_string();

		for (view, _v) in views {
			let view_pascal = view.to_case(Case::Pascal);
			let mut mbt = macro_template.clone();

			if self.no_send {
				mbt = mbt.replace("${START_SEND}", "/*");
				mbt = mbt.replace("${END_SEND}", "*/");
			} else {
				mbt = mbt.replace("${START_SEND}", "");
				mbt = mbt.replace("${END_SEND}", "");
			}

			if self.no_send || self.no_sync {
				mbt = mbt.replace("${START_SYNC}", "/*");
				mbt = mbt.replace("${END_SYNC}", "*/");
			} else {
				mbt = mbt.replace("${START_SYNC}", "");
				mbt = mbt.replace("${END_SYNC}", "");
			}

			mbt = mbt.replace("${VIEW_PASCAL}", &view_pascal);

			mbt = mbt.replace("${NAME}", &class_name);
			mbt = mbt.replace("${VIEW}", &view);

			let view_fmt = format!("{}", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_IMPL}", &macro_name);
			mbt = mbt.replace(
				"${IMPL_COMMENTS}",
				&self.build_comments(
					true,
					false,
					false,
					false,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_BOX}", &macro_name);
			mbt = mbt.replace(
				"${BOX_COMMENTS}",
				&self.build_comments(
					true,
					false,
					false,
					true,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_send", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_SEND_IMPL}", &macro_name);
			mbt = mbt.replace(
				"${SEND_IMPL_COMMENTS}",
				&self.build_comments(
					true,
					true,
					false,
					false,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_send_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_SEND_BOX}", &macro_name);
			mbt = mbt.replace(
				"${SEND_BOX_COMMENTS}",
				&self.build_comments(
					true,
					true,
					false,
					true,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_sync", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_SYNC_IMPL}", &macro_name);
			mbt = mbt.replace(
				"${SYNC_IMPL_COMMENTS}",
				&self.build_comments(
					true,
					false,
					true,
					false,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_sync_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			mbt = mbt.replace("${MACRO_NAME_SYNC_BOX}", &macro_name);
			mbt = mbt.replace(
				"${SYNC_BOX_COMMENTS}",
				&self.build_comments(
					true,
					false,
					true,
					true,
					view_pascal.clone(),
					macro_name,
					&class_name,
				)?,
			);

			mbt = mbt.replace(
				"${IMPL_PROTECTED}",
				&if view_pub_map
					.get(view)
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {};", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${BOX_PROTECTED}",
				&if view_pub_map
					.get(&format!("{}_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {}_box;", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${IMPL_SEND_PROTECTED}",
				&if view_pub_map
					.get(&format!("{}_send", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {}_send;", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${BOX_SEND_PROTECTED}",
				&if view_pub_map
					.get(&format!("{}_send_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {}_send_box;", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${IMPL_SYNC_PROTECTED}",
				&if view_pub_map
					.get(&format!("{}_sync", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {}_sync;", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${BOX_SYNC_PROTECTED}",
				&if view_pub_map
					.get(&format!("{}_sync_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::PubCrate
				{
					format!("pub(crate) use {}_sync_box;", view)
				} else {
					format!("")
				},
			);
			mbt = mbt.replace(
				"${IMPL_PUBLIC}",
				if view_pub_map
					.get(view)
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			mbt = mbt.replace(
				"${BOX_PUBLIC}",
				if view_pub_map
					.get(&format!("{}_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			mbt = mbt.replace(
				"${IMPL_SEND_PUBLIC}",
				if view_pub_map
					.get(&format!("{}_send", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			mbt = mbt.replace(
				"${BOX_SEND_PUBLIC}",
				if view_pub_map
					.get(&format!("{}_send_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			mbt = mbt.replace(
				"${IMPL_SYNC_PUBLIC}",
				if view_pub_map
					.get(&format!("{}_sync", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			mbt = mbt.replace(
				"${BOX_SYNC_PUBLIC}",
				if view_pub_map
					.get(&format!("{}_sync_box", view))
					.unwrap_or(&(Visibility::Private, "".to_string()))
					.0 == Visibility::Pub
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			macro_builder = format!("{}\n{}", macro_builder, mbt);
		}
		let template = template.replace("${MACROS}", &macro_builder);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn vis_for(
		&self,
		view: &String,
		view_pub_map: &HashMap<String, (Visibility, String)>,
	) -> String {
		match view_pub_map.get(view) {
			Some(vis) => match vis.0 {
				Visibility::Pub => "pub",
				Visibility::PubCrate => "pub(crate)",
				_ => "",
			},
			None => "",
		}
		.to_string()
	}

	#[cfg(not(tarpaulin_include))]
	fn update_builder(
		&mut self,
		template: &String,
		views: &HashMap<String, Vec<Fn>>,
		view_pub_map: &HashMap<String, (Visibility, String)>,
	) -> Result<String, Error> {
		let class_name = &self.class_name.as_ref().unwrap();
		let builder_template = include_str!("../templates/class_builder_template.txt").to_string();

		let mut min_visibility = Visibility::Private;
		for (_k, vis) in view_pub_map {
			match vis.0 {
				Visibility::Pub => min_visibility = Visibility::Pub,
				Visibility::PubCrate => {
					if min_visibility != Visibility::Pub {
						min_visibility = Visibility::PubCrate
					}
				}
				_ => {}
			}
		}

		let mut visibility = if self.class_is_pub_crate {
			"pub(crate)"
		} else if self.class_is_pub {
			"pub"
		} else {
			""
		};
		if min_visibility == Visibility::Pub && visibility != "pub" {
			visibility = "pub";
		} else if min_visibility == Visibility::PubCrate && visibility == "" {
			visibility = "pub(crate)";
		}

		let mut builder_text = format!(
			"#[doc=\"Builder for the `{}` class.\"]{} struct {}Builder {{}}\nimpl {}Builder {{",
			class_name, visibility, class_name, class_name
		);

		for (view, _v) in views {
			let trait_text = view.to_case(Case::Pascal);
			let mut view_template = builder_template.clone();
			if self.no_send {
				view_template = view_template.replace("${START_SEND}", "/*");
				view_template = view_template.replace("${END_SEND}", "*/");
			} else {
				view_template = view_template.replace("${START_SEND}", "");
				view_template = view_template.replace("${END_SEND}", "");
			}

			if self.no_send || self.no_sync {
				view_template = view_template.replace("${START_SYNC}", "/*");
				view_template = view_template.replace("${END_SYNC}", "*/");
			} else {
				view_template = view_template.replace("${START_SYNC}", "");
				view_template = view_template.replace("${END_SYNC}", "");
			}

			view_template = view_template.replace("${START_SYNC}", "");
			view_template = view_template.replace("${END_SYNC}", "");

			let view_fmt = format!("{}", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${IMPL_COMMENTS}",
				&self.build_comments(
					false,
					false,
					false,
					false,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${BOX_COMMENTS}",
				&self.build_comments(
					false,
					false,
					false,
					true,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_sync_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${SYNC_BOX_COMMENTS}",
				&self.build_comments(
					false,
					false,
					true,
					true,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_sync", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${SYNC_IMPL_COMMENTS}",
				&self.build_comments(
					false,
					false,
					true,
					false,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_send", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${SEND_IMPL_COMMENTS}",
				&self.build_comments(
					false,
					true,
					false,
					false,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);

			let view_fmt = format!("{}_send_box", view);
			let macro_name = match view_pub_map.get(&view_fmt) {
				Some(v) => v.1.clone(),
				None => view_fmt,
			};
			view_template = view_template.replace(
				"${SEND_BOX_COMMENTS}",
				&self.build_comments(
					false,
					true,
					false,
					true,
					trait_text.clone(),
					macro_name,
					&class_name,
				)?,
			);
			view_template =
				view_template.replace("${VISIBILITY_IMPL}", &self.vis_for(view, view_pub_map));
			view_template = view_template.replace(
				"${VISIBILITY_BOX}",
				&self.vis_for(&format!("{}_box", view), view_pub_map),
			);
			view_template = view_template.replace(
				"${VISIBILITY_SEND_IMPL}",
				&self.vis_for(&format!("{}_send", view), view_pub_map),
			);
			view_template = view_template.replace(
				"${VISIBILITY_SYNC_IMPL}",
				&self.vis_for(&format!("{}_sync", view), view_pub_map),
			);
			view_template = view_template.replace(
				"${VISIBILITY_SEND_BOX}",
				&self.vis_for(&format!("{}_send_box", view), view_pub_map),
			);
			view_template = view_template.replace(
				"${VISIBILITY_SYNC_BOX}",
				&self.vis_for(&format!("{}_sync_box", view), view_pub_map),
			);
			view_template = view_template.replace("${WHERE_CLAUSE}", &self.build_where()?);

			view_template = view_template.replace("${GENERIC_PRE}", &self.build_generic1()?);
			let gen_text = self.build_generic1()?;
			let gen_text = gen_text.trim();
			let lifetime = if gen_text.find("<'") == Some(0) {
				let lifetime = gen_text.substring(2, gen_text.len());
				let lifetime = match lifetime.find(",") {
					Some(pos) => lifetime.substring(0, pos),
					None => match lifetime.find(">") {
						Some(pos) => lifetime.substring(0, pos),
						None => lifetime,
					},
				};
				format!("+ '{}", lifetime)
			} else {
				"".to_string()
			};
			let trait_text = format!("{}{}{}", trait_text, gen_text, lifetime);
			view_template = view_template.replace("${TRAIT}", &trait_text);
			view_template = view_template.replace("${NAME}", class_name);
			view_template = view_template.replace("${VIEW}", view);
			builder_text = format!("{}{}", builder_text, view_template);
		}
		builder_text = format!("{}\n}}", builder_text);
		let template = template.replace("${BUILDER}", &builder_text);
		Ok(template)
	}

	#[cfg(not(tarpaulin_include))]
	fn generate_code(&mut self) -> Result<(), Error> {
		let views = self.build_trait_views()?;
		let view_pub_map = self.build_view_pub_map()?;
		let mut template = include_str!("../templates/class_template.txt").to_string();
		template = self.update_structs(&template)?;
		template = self.update_const_default(&template)?;
		template = self.update_impl_struct(&template)?;
		template = self.update_impl_var(&template)?;
		template = self.update_impl_const(&template)?;
		template = self.update_traits(&template, &views, &view_pub_map)?;
		template = self.update_trait_impl(&template, &views)?;
		template = self.update_macros(&template, &views, &view_pub_map)?;
		template = self.update_builder(&template, &views, &view_pub_map)?;

		self.ret.extend(template.parse::<TokenStream>());

		// add back in the non-builder fns
		let mut other_fns = format!(
			"impl {} {} {}{} {{",
			self.build_generic1()?,
			self.class_name.as_ref().unwrap(),
			self.build_generic2()?,
			self.build_where()?
		);
		for impl_fn in &self.impl_fns {
			other_fns = format!("{} {}", other_fns, impl_fn);
		}
		other_fns = format!("{}}}", other_fns);
		self.ret.extend(other_fns.parse::<TokenStream>());
		if self.debug {
			println!("ret='{}'", self.ret);
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn expected(&mut self, expected: Vec<&str>, found: &str) -> Result<bool, Error> {
		for expect in &expected {
			if *expect == found {
				return Ok(true);
			}
		}
		self.append_error(&format!(
			"expected one of {:?}, found token '{}'",
			expected, found
		))?;
		Ok(false)
	}

	#[cfg(not(tarpaulin_include))]
	fn append_error(&mut self, msg: &str) -> Result<(), Error> {
		match self.span {
			Some(span) => self.error_list.push(SpanError {
				span,
				msg: msg.to_string(),
			}),
			None => {}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn print_errors(&self) -> Result<(), Error> {
		let len = self.error_list.len();
		for i in 0..len.saturating_sub(1) {
			let err = &self.error_list[i];
			let diag = Diagnostic::spanned(err.span.into(), Level::Error, err.msg.clone());
			emit_error!(diag, err.msg);
		}

		let err = &self.error_list[len.saturating_sub(1)];
		let diag = Diagnostic::spanned(err.span.into(), Level::Error, err.msg.clone());
		abort!(diag, err.msg);
	}

	#[cfg(not(tarpaulin_include))]
	fn parse_item(&mut self, item: TokenStream) -> Result<(), Error> {
		for token in item {
			self.process_item_token(token)?;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn parse_attr(&mut self, attr: TokenStream) -> Result<(), Error> {
		for token in attr {
			self.process_attr_token(token)?;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.item_state {
			ItemState::Base => self.process_item_base(token)?,
			ItemState::WantsCrateOrImpl => self.process_item_wants_crate_or_impl(token)?,
			ItemState::WantsGeneric1OrName => self.process_item_wants_generic1_or_name(token)?,
			ItemState::WantsImpl => self.process_item_wants_impl(token)?,
			ItemState::WantsGeneric2WhereOrBrace => {
				self.process_item_wants_generic2_where_or_brace(token)?
			}
			ItemState::WantsWhereClause => self.process_wants_where_clause(token)?,
			ItemState::WantsGeneric2 => self.process_wants_generic2(token)?,
			ItemState::WantsWhereOrBrace => self.process_wants_where_or_brace(token)?,
			ItemState::WantsName => self.process_wants_name(token)?,
			ItemState::Complete => self.process_item_complete(token)?,
			ItemState::WantsFn => self.process_item_wants_fn(token)?,
			ItemState::WantsFnName => self.process_item_wants_fn_name(token)?,
			ItemState::WantsAppendFn => self.process_append_fn(token)?,
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_braces(&mut self, strm: TokenStream) -> Result<(), Error> {
		self.item_state = ItemState::WantsFn;
		for token in strm {
			self.span = Some(token.span());
			self.process_item_token(token)?;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_append_fn(&mut self, token: TokenTree) -> Result<(), Error> {
		if self.in_builder {
			if self.builder_fn.len() == 0 {
				let name = self.class_name.as_ref().unwrap().clone();
				let token_str = token.to_string();
				let token_str = token_str.trim();
				let expected = format!("(constants: &{}Const)", name);
				if token_str.find(expected.as_str()) != Some(0) {
					self.append_error(&format!(
						"builder must have signature, fn builder{} -> Result<Self, Error>;",
						expected
					))?;
				}
				self.builder_fn = format!("fn builder(constants: &{}Const)", name);
			} else {
				self.builder_fn = format!(
					"{}{}{}",
					self.builder_fn,
					if self.prev_is_joint { "" } else { " " },
					token.to_string()
				);
			}
		} else {
			self.append_error(
				"fn blocks other than the builder are not allowed. Please use another impl block.",
			)?;
			self.cur_fn_str = format!(
				"{}{}{}",
				self.cur_fn_str,
				if self.prev_is_joint { "" } else { " " },
				token.to_string()
			);
		}

		match token {
			Punct(ref p) => {
				if p.spacing() == Spacing::Joint {
					self.prev_is_joint = true;
				} else {
					self.prev_is_joint = false;
				}
			}
			_ => {
				self.prev_is_joint = false;
			}
		}

		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					// the rest of the fn info is here change state to WantsFn
					self.item_state = ItemState::WantsFn;
					if !self.in_builder {
						self.impl_fns.push(self.cur_fn_str.clone());
					}
				}
			}
			_ => {}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_fn_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "builder" {
					self.found_builder = true;
					self.in_builder = true;
				} else {
					self.in_builder = false;
				}
				self.cur_fn_str = format!("fn {}", ident_str);
				self.item_state = ItemState::WantsAppendFn;
			}
			_ => {
				// error
				self.append_error(&format!("expected fn name found, '{}'", token.to_string()))?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_fn(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["fn"], &token.to_string())?;
		self.item_state = ItemState::WantsFnName;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.class_name = Some(ident.to_string());
				self.item_state = ItemState::WantsGeneric2WhereOrBrace;
			}
			_ => {
				self.append_error("expected ident")?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_complete(&mut self, token: TokenTree) -> Result<(), Error> {
		self.append_error(&format!(
			"unexpected additional tokens. token: {}",
			token.to_string()
		))?;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_where_or_brace(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expected(vec!["where"], &ident.to_string())?;
				self.item_state = ItemState::WantsWhereClause;
			}
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.item_state = ItemState::Complete;
					self.process_item_braces(group.stream())?;
				} else {
				}
			}
			_ => {}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_generic2(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == ">" {
			if self.in_generic2 {
				self.item_state = ItemState::WantsWhereOrBrace;
			} else {
				self.item_state = ItemState::WantsName;
				self.in_generic2 = true;
			}
		} else {
			let generic = if self.in_generic2 {
				match self.generic2.as_mut() {
					Some(generic) => generic,
					None => {
						self.generic2 = Some("".to_string());
						self.generic2.as_mut().unwrap()
					}
				}
			} else {
				match self.generic1.as_mut() {
					Some(generic) => generic,
					None => {
						self.generic1 = Some("".to_string());
						self.generic1.as_mut().unwrap()
					}
				}
			};

			let mut is_error = false;
			match token {
				Ident(ref ident) => {
					if self.prev_is_joint {
						*generic = format!("{}{}", *generic, ident).trim().to_string();
					} else {
						*generic = format!("{} {}", *generic, ident).trim().to_string();
					}
					self.prev_is_joint = false;
				}
				Punct(ref p) => {
					let prev_is_joint = self.prev_is_joint;
					if p.spacing() == Spacing::Joint {
						self.prev_is_joint = true;
					} else {
						self.prev_is_joint = false;
					}
					if *p != ',' && *p != ':' && *p != '\'' && *p != '<' {
						is_error = true;
					}
					if *p != '<' {
						if prev_is_joint {
							*generic = format!("{}{}", *generic, p).trim().to_string();
						} else {
							*generic = format!("{} {}", *generic, p).trim().to_string();
						}
					}
				}
				_ => {
					self.expected(vec![",", "<ident>", ":", "\'"], &token.to_string())?;
					self.prev_is_joint = true;
				}
			}

			if is_error {
				self.append_error(&format!(
					"expected ',', '<ident>', ':', or ''', found '{}'",
					token.to_string()
				))?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_where_clause(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.item_state = ItemState::Complete;
					self.process_item_braces(group.stream())?;
				} else {
					self.expected(vec!["{"], &format!("{:?}", group.delimiter()))?;
				}
			}
			_ => {
				let prev_is_joint = self.prev_is_joint;
				match token {
					Punct(ref p) => {
						if p.spacing() == Spacing::Joint {
							self.prev_is_joint = true;
						} else {
							self.prev_is_joint = false;
						}
					}
					_ => {
						self.prev_is_joint = false;
					}
				}
				if self.where_clause.is_none() {
					self.where_clause = Some("".to_string());
				}
				match self.where_clause.as_mut() {
					Some(where_clause) => {
						if prev_is_joint {
							*where_clause = format!("{}{}", *where_clause, token.to_string());
						} else {
							*where_clause = format!("{} {}", *where_clause, token.to_string());
						}
					}
					None => {}
				}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_generic2_where_or_brace(
		&mut self,
		token: TokenTree,
	) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				if self.expected(vec!["where"], &ident.to_string())? {
					self.item_state = ItemState::WantsWhereClause;
				}
			}
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.item_state = ItemState::Complete;
					self.process_item_braces(group.stream())?;
				} else {
					// error
				}
			}
			_ => {
				if token.to_string() == "<" {
					self.item_state = ItemState::WantsGeneric2;
				} else {
					self.expected(vec!["<", "{"], &token.to_string())?;
				}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_impl(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["impl"], &token.to_string())?;
		self.item_state = ItemState::WantsGeneric1OrName;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_generic1_or_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.class_name = Some(ident.to_string());
				self.item_state = ItemState::WantsGeneric2WhereOrBrace;
				self.in_generic2 = true;
			}
			Punct(punct) => {
				if punct == '<' {
					// generics
					self.item_state = ItemState::WantsGeneric2;
				} else {
					// error
					self.expected(vec!["<", "<ident>"], &punct.to_string())?;
				}
			}
			_ => {
				// error
				self.expected(vec!["<", "<ident>"], &token.to_string())?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_wants_crate_or_impl(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				if ident.to_string() == "impl" {
					self.item_state = ItemState::WantsGeneric1OrName;
				} else {
					self.expected(vec!["(crate)", "impl"], &ident.to_string())?;
				}
			}
			Group(group) => {
				if group.to_string() == "(crate)" {
					self.item_state = ItemState::WantsImpl;
					self.class_is_pub_crate = true;
				} else {
					self.expected(vec!["(crate)", "impl"], &group.to_string())?;
				}
			}
			_ => {
				self.expected(vec!["(crate)", "impl"], &token.to_string())?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_item_base(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == "pub" {
			self.item_state = ItemState::WantsCrateOrImpl;
			self.class_is_pub = true;
		} else if token_str == "impl" {
			self.item_state = ItemState::WantsGeneric1OrName;
		} else {
			self.expected(vec!["pub", "impl"], &token_str)?;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_attr_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.state {
			State::Base => self.process_base(token)?,
			State::Pub => self.process_pub(token)?,
			State::Module => self.process_module(token)?,
			State::Const => self.process_const(token)?,
			State::Var => self.process_var(token)?,
			State::ViewList => self.process_wants_view_list_identifier(token)?,
			State::WantsPubIdentifier => self.process_wants_pub_identifier(token)?,
			State::WantsPubComma => self.process_wants_pub_comma(token)?,
			State::WantsConstColon => self.process_wants_const_colon(token)?,
			State::WantsConstFieldType => self.process_wants_const_field_type(token)?,
			State::WantsConstEqual => self.process_wants_const_equal(token)?,
			State::WantsConstValue => self.process_wants_const_value(token)?,
			State::WantsConstLt => self.process_wants_const_lt(token)?,
			State::WantsConstFieldTypeVec => self.process_wants_const_field_type_vec(token)?,
			State::WantsConstGt => self.process_wants_const_gt(token)?,
			State::WantsVarColon => self.process_wants_var_colon(token)?,
			State::WantsVarType => self.process_wants_var_type(token)?,
			State::WantsViewListIdentifier => self.process_wants_view_list_identifier(token)?,
			State::WantsViewListComma => self.process_wants_view_list_comma(token)?,
			State::WantsViewListFn => self.process_wants_view_list_fn(token)?,
			State::WantsViewListFnName => self.process_wants_view_list_fn_name(token)?,
			State::WantsComment => self.process_wants_comment(token)?,
			State::WantsViewListParamList => self.process_wants_view_list_param_list(token)?,
			State::WantsViewListReturnList => self.process_wants_view_list_return_list(token)?,
			State::WantsViewListGenerics => self.process_generics(token)?,
			State::WantsPubAs => self.process_wants_pub_as(token)?,
			State::WantsFnAs => self.process_wants_fn_as(token)?,
			State::Clone => self.process_wants_clone_identifier(token)?,
			State::WantsCloneComma => self.process_wants_clone_comma(token)?,
			State::NoSend => self.process_no_send_wants_semi(token)?,
			State::NoSync => self.process_no_sync_wants_semi(token)?,
			State::WantsSemi => self.process_wants_semi(token)?,
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn dequote(&self, s: &String) -> String {
		let s = s.trim().to_string();
		let s = match s.find("\"") {
			Some(pos) => {
				if pos == 0 {
					s.substring(1, s.len()).to_string()
				} else {
					s
				}
			}
			None => s,
		};
		let s = match s.rfind("\"") {
			Some(pos) => {
				let len = s.len();
				if len > 0 {
					if pos == len - 1 {
						s.substring(0, len - 1).to_string()
					} else {
						s
					}
				} else {
					s
				}
			}
			None => s,
		};
		s
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_comment(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(ref group) => {
				if group.delimiter() == Delimiter::Bracket {
					let mut first = true;
					let mut second = false;
					for inner in group.stream() {
						let inner_str = inner.to_string();
						if first && inner_str != "doc" {
							self.append_error(&format!(
								"expected comment block here, found '{}'",
								inner_str,
							))?;
						} else if first {
							second = true;
						} else if second {
							match inner {
								Literal(lit) => {
									self.cur_comments.push(lit.to_string());
								}
								_ => {
									if inner_str != "=" {
										self.append_error(&format!(
											"expected comment block here, found '{}'",
											inner_str,
										))?;
									}
								}
							}
						}
						first = false;
					}
				} else {
					self.append_error(&format!(
						"expected comment block here, found '{}'",
						token.to_string(),
					))?;
				}
			}
			_ => {
				self.append_error(&format!(
					"expected comment block here, found '{}'",
					token.to_string()
				))?;
			}
		}
		self.state = State::Base;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_semi(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![";"], &token.to_string())?;
		self.state = State::Base;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_no_send_wants_semi(&mut self, token: TokenTree) -> Result<(), Error> {
		self.no_send = true;
		self.expected(vec![";"], &token.to_string())?;
		self.state = State::Base;

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_no_sync_wants_semi(&mut self, token: TokenTree) -> Result<(), Error> {
		self.no_sync = true;
		self.expected(vec![";"], &token.to_string())?;
		self.state = State::Base;

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_generics(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ">" {
			self.append_error("class functions cannot have generics")?;
			self.state = State::WantsViewListParamList;
		} else {
			match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					cur_fn.generic_string = format!("{} {}", cur_fn.generic_string, token_str);
				}
				None => {}
			}
		}

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_clone_identifier(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ref ident) => {
				self.clone_list.push(CloneItem {
					name: ident.to_string(),
					span: token.span(),
				});
			}
			_ => {
				self.append_error(&format!("expected view name, found, '{}'", token))?;
			}
		}

		self.state = State::WantsCloneComma;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_clone_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			self.state = State::Base;
		} else if token_str == "," {
			self.state = State::Clone;
		} else {
			self.expected(vec![",", ";"], &token_str)?;
			self.state = State::Clone;
		}

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_base(&mut self, token: TokenTree) -> Result<(), Error> {
		self.cur_is_pub_crate = false;
		let token_str = token.to_string();
		if token_str == "pub" {
			self.state = State::Pub;
		} else if token_str == "module" {
			self.state = State::Module;
		} else if token_str == "const" {
			self.state = State::Const;
		} else if token_str == "var" {
			self.state = State::Var;
		} else if token_str == "clone" {
			self.state = State::Clone;
		} else if token_str == "no_send" {
			self.state = State::NoSend;
		} else if token_str == "no_sync" {
			self.state = State::NoSync;
		} else {
			match token {
				Group(ref group) => {
					if group.delimiter() == Delimiter::Bracket {
						self.state = State::ViewList;
						self.cur_fn = Some(Fn::new(token.span()));
						for token in group.stream() {
							self.process_attr_token(token)?;
						}
						self.state = State::WantsViewListFn;
					} else {
						// error
						self.expected(vec!["[", "pub", "var", "const", "module"], &token_str)?;
					}
				}
				_ => {
					if token_str == "#" {
						// comment potential
						self.state = State::WantsComment;
					} else {
						// error
						self.expected(vec!["[", "pub", "var", "const", "module"], &token_str)?;
					}
				}
			}
		}

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_pub_as(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				// update last pushed view
				if self.cur_is_pub_crate {
					let len = self.pub_crate_views.len();
					self.pub_crate_views[len - 1].macro_name = ident.to_string();
				} else {
					let len = self.pub_views.len();
					self.pub_views[len - 1].macro_name = ident.to_string();
				}
			}
			_ => {
				self.append_error(&format!("expected macro_name, found, '{}'", token))?;
			}
		}

		self.state = State::WantsPubComma;

		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_pub_identifier(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			self.append_error(&format!("expected view name, found: '{}'", token_str))?;
			self.state = State::Base;
		} else {
			match token {
				Ident(ident) => {
					if self.cur_is_pub_crate {
						self.pub_crate_views.push(PubCrate::new(
							ident.to_string(),
							self.span.as_ref().unwrap().clone(),
						));
					} else {
						self.pub_views.push(Pub::new(
							ident.to_string(),
							self.span.as_ref().unwrap().clone(),
							self.cur_comments.clone(),
						));
					}
				}
				_ => {
					self.append_error(&format!("expected view name, found: '{}'", token_str))?;
				}
			}
			self.state = State::WantsPubComma;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_pub_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == "as" {
			self.state = State::WantsPubAs;
		} else if token_str == ";" {
			self.cur_comments.clear();
			self.state = State::Base;
		} else {
			self.expected(vec![","], &token_str)?;
			self.state = State::WantsPubIdentifier;
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_pub(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.state = State::WantsPubComma;
				if self.cur_is_pub_crate {
					self.pub_crate_views.push(PubCrate::new(
						ident.to_string(),
						self.span.as_ref().unwrap().clone(),
					));
				} else {
					self.pub_views.push(Pub::new(
						ident.to_string(),
						self.span.as_ref().unwrap().clone(),
						self.cur_comments.clone(),
					));
				}
			}
			Group(group) => {
				if group.delimiter() != Delimiter::Parenthesis || group.to_string() != "(crate)" {
					self.append_error("expected, '(crate)' or view name")?;
				} else {
					self.cur_is_pub_crate = true;
					self.state = State::WantsPubIdentifier;
				}
			}
			_ => {
				self.append_error("expected, '(crate)' or view name")?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_module(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();

		if token_str == ";" {
			self.state = State::Base;
		} else {
			if self.module.is_some() {
				self.append_error("module may only be set once.")?;
			}
			match token {
				Literal(literal) => {
					let lit_str = literal.to_string();
					let lit_str = lit_str.trim();
					let lit_str = match lit_str.find("\"") {
						Some(start) => {
							if start == 0 {
								match lit_str.rfind("\"") {
									Some(end) => {
										if end > start + 1 {
											Some(lit_str.substring(start + 1, end).to_string())
										} else {
											None
										}
									}
									None => None,
								}
							} else {
								None
							}
						}
						None => None,
					};

					if lit_str.is_none() {
						self.append_error(&format!(
							"unexpected literal string found: '{}', expected, '\"<module_name>\"",
							token_str
						))?;
					}
					self.module = lit_str;
				}
				_ => {
					self.append_error(&format!(
						"unexpected token found: '{}'. expected, '\"<module_name>\"",
						token_str
					))?;
				}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_gt(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![">"], &token.to_string())?;
		self.state = State::WantsConstEqual;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_field_type_vec(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "Vec" {
					self.append_error(&format!(
						"expected const field type, found '{}'",
						ident_str
					))?;
				} else {
					match self.cur_const.as_mut() {
						Some(cur_const) => {
							if ident_str == "usize" {
								cur_const.field_type = Some(FieldType::VecUsize);
							} else if ident_str == "u8" {
								cur_const.field_type = Some(FieldType::VecU8);
							} else if ident_str == "u16" {
								cur_const.field_type = Some(FieldType::VecU16);
							} else if ident_str == "u32" {
								cur_const.field_type = Some(FieldType::VecU32);
							} else if ident_str == "u64" {
								cur_const.field_type = Some(FieldType::VecU64);
							} else if ident_str == "u128" {
								cur_const.field_type = Some(FieldType::VecU128);
							} else if ident_str == "bool" {
								cur_const.field_type = Some(FieldType::VecBool);
							} else if ident_str == "String" {
								cur_const.field_type = Some(FieldType::VecString);
							} else {
								cur_const.field_string = Some(format!("{}", ident_str));
								cur_const.field_type = Some(FieldType::VecConfigurable);
							}
						}
						None => {}
					}
				}
			}
			_ => {
				self.append_error(&format!(
					"expected const field type, found '{}'",
					token.to_string()
				))?;
			}
		}
		self.state = State::WantsConstGt;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_lt(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["<"], &token.to_string())?;

		self.state = State::WantsConstFieldTypeVec;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_value(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			let cur_const = self.cur_const.as_ref().unwrap().clone();

			let expr: Result<Expr, syn::Error> = parse_str(&cur_const.value_str);
			match expr {
				Ok(_) => {}
				Err(ref e) => {
					self.span = Some(cur_const.span);
					self.append_error(&format!(
						"failed to parse '{}'. Error: {:?}.",
						cur_const.value_str, e
					))?;
				}
			}

			self.const_list.push(cur_const);
			self.cur_const = None;
			self.state = State::Base;
		} else {
			match self.cur_const.as_mut() {
				Some(cur_const) => {
					let prev_token_is_joint = cur_const.prev_token_is_joint;
					match token {
						Punct(p) => {
							if p.spacing() == Spacing::Joint {
								cur_const.prev_token_is_joint = true;
							} else {
								cur_const.prev_token_is_joint = false;
							}
						}
						_ => {
							cur_const.prev_token_is_joint = false;
						}
					}
					if prev_token_is_joint {
						cur_const.value_str = format!("{}{}", cur_const.value_str, token_str);
					} else {
						cur_const.value_str = format!("{} {}", cur_const.value_str, token_str)
							.trim()
							.to_string();
					}
				}
				None => {}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_equal(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["="], &token.to_string())?;
		self.state = State::WantsConstValue;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_field_type(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "Vec" {
					self.state = State::WantsConstLt;
				} else {
					self.state = State::WantsConstEqual;
					match self.cur_const.as_mut() {
						Some(cur_const) => {
							if ident_str == "usize" {
								cur_const.field_type = Some(FieldType::Usize);
							} else if ident_str == "u8" {
								cur_const.field_type = Some(FieldType::U8);
							} else if ident_str == "u16" {
								cur_const.field_type = Some(FieldType::U16);
							} else if ident_str == "u32" {
								cur_const.field_type = Some(FieldType::U32);
							} else if ident_str == "u64" {
								cur_const.field_type = Some(FieldType::U64);
							} else if ident_str == "u128" {
								cur_const.field_type = Some(FieldType::U128);
							} else if ident_str == "bool" {
								cur_const.field_type = Some(FieldType::Bool);
							} else if ident_str == "String" {
								cur_const.field_type = Some(FieldType::String);
							} else {
								cur_const.field_string = Some(format!("{}", ident_str));
								cur_const.field_type = Some(FieldType::Configurable);
							}
						}
						None => {}
					}
				}
			}
			_ => {
				self.state = State::WantsConstEqual;
				self.append_error(&format!(
					"expected const field type, found '{}'",
					token.to_string()
				))?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_const_colon(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![":"], &token.to_string())?;
		self.state = State::WantsConstFieldType;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_const(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ref ident) => {
				let mut nconst = Const::new(ident.to_string(), token.span());
				nconst.comments.extend(self.cur_comments.clone());
				self.cur_comments.clear();
				self.cur_const = Some(nconst);
			}
			_ => {
				self.append_error(&format!(
					"expected const name, found, '{}'",
					token.to_string()
				))?;
			}
		}

		self.state = State::WantsConstColon;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_var_type(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			let cur_var = self.cur_var.as_ref().unwrap().clone();

			let expr: Result<Type, syn::Error> = parse_str(&cur_var.type_str);
			match expr {
				Ok(_) => {}
				Err(ref e) => {
					self.span = Some(cur_var.span);
					self.append_error(&format!(
						"failed to parse '{}'. Error: {:?}.",
						cur_var.type_str, e
					))?;
				}
			}

			self.var_list.push(cur_var);
			self.cur_var = None;

			self.state = State::Base;
		} else {
			match self.cur_var.as_mut() {
				Some(cur_var) => {
					let prev_token_is_joint = cur_var.prev_token_is_joint;
					match token {
						Punct(p) => {
							if p.spacing() == Spacing::Joint {
								cur_var.prev_token_is_joint = true;
							} else {
								cur_var.prev_token_is_joint = false;
							}
						}
						_ => {
							cur_var.prev_token_is_joint = false;
						}
					}
					if prev_token_is_joint {
						cur_var.type_str = format!("{}{}", cur_var.type_str, token_str);
					} else {
						cur_var.type_str = format!("{} {}", cur_var.type_str, token_str)
							.trim()
							.to_string();
					}
				}
				None => {}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_var_colon(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![":"], &token.to_string())?;
		self.state = State::WantsVarType;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_var(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ref ident) => {
				self.cur_var = Some(Var::new(ident.to_string(), token.span()));
			}
			_ => {
				self.append_error(&format!(
					"expected var name, found, '{}'",
					token.to_string()
				))?;
			}
		}

		self.state = State::WantsVarColon;
		Ok(())
	}

	fn process_wants_fn_as(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					cur_fn.as_fn = Some(ident.to_string());
					// if return_list is "" make it "()"
					match self.cur_fn.as_mut() {
						Some(cur_fn) => {
							if cur_fn.return_list == "" {
								cur_fn.return_list = "()".to_string();
							}
						}
						None => {}
					}

					// check return list and param list with syn
					let mut cur_fn = self.cur_fn.as_ref().unwrap().clone();
					let expr: Result<Type, syn::Error> = parse_str(&cur_fn.return_list);
					match expr {
						Ok(_) => {}
						Err(ref e) => {
							self.span = match cur_fn.return_list_span {
								Some(s) => Some(s),
								None => Some(cur_fn.span),
							};
							self.append_error(&format!(
								"failed to parse '{}'. Error: {:?}.",
								cur_fn.return_list, e
							))?;
						}
					}

					cur_fn.comments.extend(self.cur_comments.clone());
					self.cur_comments.clear();

					self.fn_list.push(cur_fn.clone());
				}
				None => {}
			},
			_ => {
				self.append_error(&format!("expected ident found, '{}'", token.to_string()))?;
			}
		}
		self.state = State::WantsSemi;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_return_list(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			// if return_list is "" make it "()"
			match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					if cur_fn.return_list == "" {
						cur_fn.return_list = "()".to_string();
					}
				}
				None => {}
			}

			// check return list and param list with syn
			let mut cur_fn = self.cur_fn.as_ref().unwrap().clone();
			let expr: Result<Type, syn::Error> = parse_str(&cur_fn.return_list);
			match expr {
				Ok(_) => {}
				Err(ref e) => {
					self.span = match cur_fn.return_list_span {
						Some(s) => Some(s),
						None => Some(cur_fn.span),
					};
					self.append_error(&format!(
						"failed to parse '{}'. Error: {:?}.",
						cur_fn.return_list, e
					))?;
				}
			}

			cur_fn.comments.extend(self.cur_comments.clone());
			self.cur_comments.clear();

			self.fn_list.push(cur_fn.clone());
			self.state = State::Base;
		} else if token_str == "as" {
			self.state = State::WantsFnAs;
		} else {
			let (expect_dash_return_list, expect_gt_return_list) = {
				let cur_fn = self.cur_fn.as_ref().unwrap();
				(cur_fn.expect_dash_return_list, cur_fn.expect_gt_return_list)
			};
			if expect_dash_return_list {
				self.expected(vec!["-"], &token.to_string())?;
				self.cur_fn.as_mut().unwrap().expect_dash_return_list = false;
			} else if expect_gt_return_list {
				self.expected(vec![">"], &token.to_string())?;
				self.cur_fn.as_mut().unwrap().expect_gt_return_list = false;
			} else {
				match self.cur_fn.as_mut() {
					Some(cur_fn) => match cur_fn.return_list_span {
						Some(_) => {}
						None => cur_fn.return_list_span = Some(token.span()),
					},
					None => {}
				}
				match self.cur_fn.as_mut() {
					Some(cur_fn) => {
						let prev_is_joint = cur_fn.prev_token_is_joint;
						match token {
							Punct(ref p) => {
								if p.spacing() == Spacing::Joint {
									cur_fn.prev_token_is_joint = true;
								} else {
									cur_fn.prev_token_is_joint = false;
								}
							}
							_ => {
								cur_fn.prev_token_is_joint = false;
							}
						}

						if prev_is_joint {
							cur_fn.return_list = format!("{}{}", cur_fn.return_list, token);
						} else {
							cur_fn.return_list = format!("{} {}", cur_fn.return_list, token)
								.trim()
								.to_string();
						}
					}
					None => {}
				}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn check_type(
		&mut self,
		_param_name: String,
		type_str: String,
		span: Span,
	) -> Result<(), Error> {
		let expr: Result<Type, syn::Error> = parse_str(&type_str);
		match expr {
			Ok(_) => {}
			Err(ref e) => {
				self.span = Some(span);
				self.append_error(&format!("failed to parse '{}'. Error: {:?}.", type_str, e))?;
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_param_list(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(ref group) => {
				if group.delimiter() != Delimiter::Parenthesis {
					self.append_error(&format!("expected '(' found '{:?}'", group.delimiter()))?;
				} else {
					let mut self_error = false;
					let mut name_errors: Vec<Span> = vec![];
					match self.cur_fn.as_mut() {
						Some(cur_fn) => {
							let mut cur_name = "".to_string();
							let mut cur_type = "".to_string();
							let mut in_type = false;
							let mut last_token = token.clone();
							let mut first = true;
							for token in group.stream() {
								last_token = token.clone();
								self.span = Some(token.span());
								let token_str = token.to_string();
								if token_str == "," {
									cur_fn.param_names.push(cur_name.clone());
									cur_fn.param_types.push(cur_type.clone());
									first = false;
									cur_fn.param_type_spans.push(token.span());

									cur_name = "".to_string();
									cur_type = "".to_string();
									in_type = false;
								} else if token_str == ":" {
									in_type = true;
								} else if in_type {
									cur_type =
										format!("{} {}", cur_type, token_str).trim().to_string();
								} else {
									if first {
										if token_str != "&"
											&& token_str != "mut" && token_str != "self"
										{
											self_error = true;
										}
										cur_name = format!("{} {}", cur_name, token_str.clone())
											.trim()
											.to_string();
									} else {
										if cur_name.len() != 0 {
											name_errors.push(token.span());
										}
										match token {
											Ident(_) => {}
											_ => name_errors.push(token.span()),
										}
										cur_name = token_str.clone();
									}
								}
							}
							if cur_name.len() > 0 {
								cur_fn.param_names.push(cur_name);
								cur_fn.param_types.push(cur_type);
								cur_fn.param_type_spans.push(last_token.span());
							}

							cur_fn.param_list = group.stream().to_string();
							cur_fn.expect_dash_return_list = true;
							cur_fn.expect_gt_return_list = true;
						}
						None => {}
					}

					if name_errors.len() > 0 {
						for span in name_errors {
							self.span = Some(span);
							self.append_error("invalid name")?;
						}
					}

					if self_error {
						self.append_error("first param must be either '&self' or '&mut self'")?;
					}

					let cur_fn = self.cur_fn.as_ref().unwrap().clone();
					if cur_fn.param_names.len() == 0 {
						self.append_error("functions must have at least one param and it must be either '&self' or '&mut self'")?;
					} else {
						if cur_fn.param_types[0].len() != 0 {
							self.span = Some(cur_fn.param_type_spans[0]);
							self.append_error("first param must be either '&self' or '&mut self'")?;
						}
						if cur_fn.param_names[0].find("self").is_none() {
							self.span = Some(cur_fn.param_type_spans[0]);
							self.append_error("first param must be either '&self' or '&mut self'")?;
						}
						if cur_fn.param_names[0].find("&").is_none() {
							self.span = Some(cur_fn.param_type_spans[0]);
							self.append_error("first param must be either '&self' or '&mut self'")?;
						}
					}
					for i in 1..cur_fn.param_types.len() {
						self.check_type(
							cur_fn.param_names[i].clone(),
							cur_fn.param_types[i].clone(),
							cur_fn.param_type_spans[i].clone(),
						)?;
					}
				}
				self.state = State::WantsViewListReturnList;
			}
			_ => {
				if token.to_string() == "<" {
					self.state = State::WantsViewListGenerics;
				} else {
					self.append_error(&format!(
						"expected param list or generics found, '{}'",
						token.to_string()
					))?;
					self.state = State::WantsViewListReturnList;
				}
			}
		}
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_fn_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ref ident) => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					cur_fn.name = ident.to_string();
					cur_fn.name_span = Some(token.span());
				}
				None => {}
			},
			_ => {
				self.append_error(&format!("expected fn name found token '{}'", token))?;
			}
		}
		self.state = State::WantsViewListParamList;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_fn(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["fn"], &token.to_string())?;
		self.state = State::WantsViewListFnName;
		Ok(())
	}

	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_identifier(&mut self, token: TokenTree) -> Result<(), Error> {
		let mut snake_err = false;
		match token {
			Ident(ident) => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					let ident_str = ident.to_string();
					if !ident_str.is_case(Case::Snake) {
						snake_err = true;
					}
					cur_fn.view_list.push(ident_str);
				}
				None => {}
			},
			_ => {
				self.append_error(&format!("expected view list id, found, '{}'", token))?;
			}
		}

		if snake_err {
			self.append_error("views must be of the snake case format")?;
		}
		self.state = State::WantsViewListComma;
		Ok(())
	}
	#[cfg(not(tarpaulin_include))]
	fn process_wants_view_list_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![","], &token.to_string())?;
		self.state = State::WantsViewListIdentifier;
		Ok(())
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_class(attr: TokenStream, item: TokenStream, debug: bool) -> TokenStream {
	let mut state = StateMachine::new(debug);
	match state.derive(attr, item) {
		Ok(strm) => strm,
		Err(e) => {
			println!("do_derive_class generated error: {}", e);
			"".parse::<TokenStream>().unwrap()
		}
	}
}
