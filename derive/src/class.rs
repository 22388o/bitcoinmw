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
	signature: String,
	param_list: String,
	view_list: Vec<String>,
}

impl Fn {
	fn new(span: Span) -> Self {
		Self {
			span,
			name: "".to_string(),
			signature: "".to_string(),
			param_list: "".to_string(),
			view_list: vec![],
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
}

struct StateMachine {
	state: State,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	module: Option<String>,
	is_pub_crate: bool,
	pub_views: Vec<Pub>,
	pub_crate_views: Vec<PubCrate>,
	cur_const: Option<Const>,
	cur_var: Option<Var>,
	cur_fn: Option<Fn>,
	const_list: Vec<Const>,
	var_list: Vec<Var>,
	fn_list: Vec<Fn>,
}

impl StateMachine {
	fn new() -> Self {
		Self {
			state: State::Base,
			span: None,
			error_list: vec![],
			module: None,
			is_pub_crate: false,
			pub_views: vec![],
			pub_crate_views: vec![],
			cur_const: None,
			cur_var: None,
			cur_fn: None,
			const_list: vec![],
			var_list: vec![],
			fn_list: vec![],
		}
	}

	fn derive(&mut self, attr: TokenStream, _item: TokenStream) -> Result<(), Error> {
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

	fn parse_attr(&mut self, attr: TokenStream) -> Result<(), Error> {
		for token in attr {
			self.process_attr_token(token)?;
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

	fn process_wants_view_list_param_list(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == ";" {
			self.fn_list.push(self.cur_fn.as_ref().unwrap().clone());
			self.state = State::Base;
		}
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
			Ident(ident) => {
				println!("view list id = {}", ident.to_string());
				match self.cur_fn.as_mut() {
					Some(cur_fn) => {
						cur_fn.view_list.push(ident.to_string());
					}
					None => {}
				}
			}
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
