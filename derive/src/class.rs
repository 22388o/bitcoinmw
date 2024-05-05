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
use bmw_deps::substring::Substring;
use bmw_deps::syn;
use bmw_deps::syn::{parse_str, Expr, Type};
use proc_macro::TokenTree::{Group, Ident, Literal, Punct};
use proc_macro::{Delimiter, Spacing, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};

#[derive(Clone, Debug)]
struct Fn {
	name: String,
	span: Span,
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
}

impl Fn {
	fn new(span: Span) -> Self {
		Self {
			span,
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
	fn new(name: String, span: Span) -> Self {
		Self {
			name,
			type_str: "".to_string(),
			span,
			prev_token_is_joint: false,
		}
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
	value_str: String,
	span: Span,
	prev_token_is_joint: bool,
}

impl Const {
	fn new(name: String, span: Span) -> Self {
		Self {
			name,
			value_str: "".to_string(),
			field_type: None,
			span,
			prev_token_is_joint: false,
		}
	}
}

struct Pub {
	name: String,
	span: Span,
}

impl Pub {
	fn new(name: String, span: Span) -> Self {
		Self { name, span }
	}
}

struct PubCrate {
	name: String,
	span: Span,
}

impl PubCrate {
	fn new(name: String, span: Span) -> Self {
		Self { name, span }
	}
}

struct SpanError {
	span: Span,
	msg: String,
}

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
	Complete,
}

enum State {
	Base,
	Pub,
	Module,
	Const,
	Var,
	ViewList,
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
	WantsViewListFnName,
	WantsViewListParamList,
	WantsViewListReturnList,
}

struct StateMachine {
	state: State,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	module: Option<String>,
	is_pub_crate: bool,
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
	inner: String,
	prev_is_joint: bool,
}

impl StateMachine {
	fn new() -> Self {
		Self {
			state: State::Base,
			item_state: ItemState::Base,
			span: None,
			error_list: vec![],
			module: None,
			is_pub_crate: false,
			in_generic2: false,
			pub_views: vec![],
			pub_crate_views: vec![],
			cur_const: None,
			cur_var: None,
			cur_fn: None,
			const_list: vec![],
			var_list: vec![],
			fn_list: vec![],
			class_name: None,
			generic1: None,
			generic2: None,
			where_clause: None,
			class_is_pub: false,
			class_is_pub_crate: false,
			inner: "".to_string(),
			prev_is_joint: false,
		}
	}

	fn derive(&mut self, attr: TokenStream, item: TokenStream) -> Result<(), Error> {
		self.parse_attr(attr)?;

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

		self.item_state = ItemState::Base;
		self.parse_item(item)?;

		println!(
			"class_name={:?},pub={},pub(crate)={},generics1={:?},generic2={:?},where={:?}",
			self.class_name,
			self.class_is_pub,
			self.class_is_pub_crate,
			self.generic1,
			self.generic2,
			self.where_clause,
		);

		if self.error_list.len() > 0 {
			self.print_errors()?;
		}
		Ok(())
	}

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

	fn parse_item(&mut self, item: TokenStream) -> Result<(), Error> {
		for token in item {
			self.process_item_token(token)?;
		}
		Ok(())
	}

	fn parse_attr(&mut self, attr: TokenStream) -> Result<(), Error> {
		for token in attr {
			self.process_attr_token(token)?;
		}
		Ok(())
	}

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
		}
		Ok(())
	}

	fn process_wants_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.class_name = Some(ident.to_string());
				self.item_state = ItemState::WantsGeneric2;
			}
			_ => {
				self.append_error("expected ident")?;
			}
		}
		Ok(())
	}
	fn process_item_complete(&mut self, token: TokenTree) -> Result<(), Error> {
		self.append_error("unexpected additional tokens")?;
		Ok(())
	}

	fn process_wants_where_or_brace(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expected(vec!["where"], &ident.to_string())?;
				self.item_state = ItemState::WantsWhereClause;
			}
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.item_state = ItemState::Complete;
				} else {
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn process_wants_generic2(&mut self, token: TokenTree) -> Result<(), Error> {
		println!("wants gen token = {}", token);
		if token.to_string() == ">" {
			if self.in_generic2 {
				self.item_state = ItemState::WantsWhereOrBrace;
			} else {
				println!("setting to in_gen2");
				self.item_state = ItemState::WantsName;
				self.in_generic2 = true;
			}
		} else {
			let mut generic = if self.in_generic2 {
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

	fn process_wants_where_clause(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.item_state = ItemState::Complete;
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
				} else {
					// error
				}
			}
			_ => {
				// error
			}
		}
		Ok(())
	}

	fn process_item_wants_impl(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["impl"], &token.to_string())?;
		self.item_state = ItemState::WantsGeneric1OrName;
		Ok(())
	}

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

	fn process_item_base(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		println!("item base token = {}", token_str);
		if token_str == "pub" {
			println!("set class_is_pub = true");
			self.item_state = ItemState::WantsCrateOrImpl;
			self.class_is_pub = true;
		} else if token_str == "impl" {
			self.item_state = ItemState::WantsGeneric1OrName;
		} else {
			self.expected(vec!["pub", "impl"], &token_str)?;
		}
		Ok(())
	}

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
			State::WantsViewListParamList => self.process_wants_view_list_param_list(token)?,
			State::WantsViewListReturnList => self.process_wants_view_list_return_list(token)?,
		}
		Ok(())
	}

	fn process_base(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == "pub" {
			self.state = State::Pub;
		} else if token_str == "module" {
			self.state = State::Module;
		} else if token_str == "const" {
			self.state = State::Const;
		} else if token_str == "var" {
			self.state = State::Var;
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
					// error
					self.expected(vec!["[", "pub", "var", "const", "module"], &token_str)?;
				}
			}
		}

		Ok(())
	}

	fn process_wants_pub_identifier(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			self.append_error(&format!("expected view name, found: '{}'", token_str))?;
			self.state = State::Base;
		} else {
			match token {
				Ident(ident) => {
					if self.is_pub_crate {
						self.pub_crate_views.push(PubCrate::new(
							ident.to_string(),
							self.span.as_ref().unwrap().clone(),
						));
					} else {
						self.pub_views.push(Pub::new(
							ident.to_string(),
							self.span.as_ref().unwrap().clone(),
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

	fn process_wants_pub_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if token_str == ";" {
			self.state = State::Base;
		} else {
			self.expected(vec![","], &token_str)?;
			self.state = State::WantsPubIdentifier;
		}
		Ok(())
	}

	fn process_pub(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.state = State::WantsPubComma;
				if self.is_pub_crate {
					self.pub_crate_views.push(PubCrate::new(
						ident.to_string(),
						self.span.as_ref().unwrap().clone(),
					));
				} else {
					self.pub_views.push(Pub::new(
						ident.to_string(),
						self.span.as_ref().unwrap().clone(),
					));
				}
			}
			Group(group) => {
				if group.delimiter() != Delimiter::Parenthesis || group.to_string() != "(crate)" {
					self.append_error("expected, '(crate)' or view name")?;
				} else {
					self.is_pub_crate = true;
					self.state = State::WantsPubIdentifier;
				}
			}
			_ => {
				self.append_error("expected, '(crate)' or view name")?;
			}
		}
		Ok(())
	}

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

	fn process_wants_const_gt(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![">"], &token.to_string())?;
		self.state = State::WantsConstEqual;
		Ok(())
	}

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

	fn process_wants_const_lt(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["<"], &token.to_string())?;

		self.state = State::WantsConstFieldTypeVec;
		Ok(())
	}

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

	fn process_wants_const_equal(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["="], &token.to_string())?;
		self.state = State::WantsConstValue;
		Ok(())
	}

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

	fn process_wants_const_colon(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![":"], &token.to_string())?;
		self.state = State::WantsConstFieldType;
		Ok(())
	}

	fn process_const(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ref ident) => {
				self.cur_const = Some(Const::new(ident.to_string(), token.span()));
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

	fn process_wants_var_colon(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![":"], &token.to_string())?;
		self.state = State::WantsVarType;
		Ok(())
	}

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

	fn process_wants_view_list_return_list(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == ";" {
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
			let cur_fn = self.cur_fn.as_ref().unwrap().clone();
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

			self.fn_list.push(cur_fn.clone());
			self.state = State::Base;
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

	fn check_type(
		&mut self,
		param_name: String,
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
			}
			_ => {}
		}
		self.state = State::WantsViewListReturnList;
		Ok(())
	}

	fn process_wants_view_list_fn_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					cur_fn.name = ident.to_string();
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

	fn process_wants_view_list_fn(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec!["fn"], &token.to_string())?;
		self.state = State::WantsViewListFnName;
		Ok(())
	}

	fn process_wants_view_list_identifier(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					cur_fn.view_list.push(ident.to_string());
				}
				None => {}
			},
			_ => {
				self.append_error(&format!("expected view list id, found, '{}'", token))?;
			}
		}
		self.state = State::WantsViewListComma;
		Ok(())
	}

	fn process_wants_view_list_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		self.expected(vec![","], &token.to_string())?;
		self.state = State::WantsViewListIdentifier;
		Ok(())
	}
}

pub(crate) fn do_derive_class(attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut state = StateMachine::new();
	match state.derive(attr, item) {
		Ok(_) => {}
		Err(e) => {
			println!("do_derive_class generated error: {}", e);
		}
	}
	"".parse::<TokenStream>().unwrap()
}
