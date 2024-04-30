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

use bmw_base::BaseErrorKind::Parse;
use bmw_base::*;
use bmw_deps::convert_case::{Case, Casing};
use bmw_deps::substring::Substring;
use proc_macro::TokenTree::*;
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

struct SpanError {
	span: Span,
	msg: String,
}

#[derive(Clone, Debug)]
struct PublicView {
	name: String,
	span: Span,
	comments: Vec<String>,
}

impl PublicView {
	fn new(name: String, span: Span, comments: Vec<String>) -> Self {
		Self {
			name,
			span,
			comments,
		}
	}
}

#[derive(Clone, Debug)]
struct ProtectedView {
	name: String,
	span: Span,
}

impl ProtectedView {
	fn new(name: String, span: Span) -> Self {
		Self { name, span }
	}
}

#[derive(Clone, Debug)]
struct CloneView {
	name: String,
	span: Span,
}

impl CloneView {
	fn new(name: String, span: Span) -> Self {
		Self { name, span }
	}
}

#[derive(Clone, Debug)]
struct Var {
	name: String,
	type_str: String,
	found_colon: bool,
	span: Span,
}

impl Var {
	fn new(span: Span) -> Self {
		Self {
			name: "".to_string(),
			type_str: "".to_string(),
			found_colon: false,
			span,
		}
	}
}

#[derive(Debug, Clone)]
struct Const {
	name: String,
	type_str: String,
	value_str: String,
	found_colon: bool,
	found_equal: bool,
	comments: Vec<String>,
	span: Span,
}

impl Const {
	fn new(span: Span) -> Self {
		Self {
			name: "".to_string(),
			type_str: "".to_string(),
			value_str: "".to_string(),
			found_colon: false,
			found_equal: false,
			comments: vec![],
			span,
		}
	}
}

#[derive(Clone)]
struct FnInfo {
	name: String,
	return_str: TokenStream,
	params: TokenStream,
	fn_block: TokenStream,
	views: Vec<String>,
	comments: Vec<String>,
	expect_ret_arrow1: bool,
	expect_ret_arrow2: bool,
	span: Span,
}

impl Debug for FnInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "FnInfo {{\n")?;
		write!(f, "\tname: '{}'\n", self.name)?;
		write!(f, "\tviews: '{:?}'\n", self.views)?;
		write!(f, "\tparams: '{}'\n", self.params)?;
		write!(f, "\treturn_str: '{}'\n", self.return_str)?;
		for comment in &self.comments {
			write!(f, "\t///{}\n", comment)?;
		}
		write!(f, "\tfn_block: '{}'\n", self.fn_block)?;
		write!(f, "}}")?;
		Ok(())
	}
}

impl FnInfo {
	fn new(span: Span) -> Self {
		Self {
			span,
			name: "".to_string(),
			return_str: TokenStream::new(),
			params: TokenStream::new(),
			fn_block: TokenStream::new(),
			views: vec![],
			comments: vec![],
			expect_ret_arrow1: false,
			expect_ret_arrow2: false,
		}
	}
}

#[derive(PartialEq)]
enum State {
	Base,
	Public,
	Protected,
	Clone,
	Var,
	Const,
	Fn,
	Module,
	Comment,
}

struct StateMachine {
	state: State,
	name: Option<String>,
	module: Option<String>,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	public_list: Vec<PublicView>,
	protected_list: Vec<ProtectedView>,
	clone_list: Vec<CloneView>,
	var_list: Vec<Var>,
	const_list: Vec<Const>,
	fn_list: Vec<FnInfo>,
	expect_comma: bool,
	expect_fn: bool,
	expect_fn_name: bool,
	expect_params: bool,
	cur_var: Option<Var>,
	cur_const: Option<Const>,
	cur_fn: Option<FnInfo>,
	comments: Vec<String>,
	generics: Option<String>,
	generics2: Option<String>,
	where_clause: Option<String>,
}

impl StateMachine {
	fn new() -> Self {
		Self {
			state: State::Base,
			name: None,
			module: None,
			span: None,
			error_list: vec![],
			public_list: vec![],
			protected_list: vec![],
			clone_list: vec![],
			var_list: vec![],
			const_list: vec![],
			fn_list: vec![],
			comments: vec![],
			expect_comma: false,
			expect_fn: false,
			expect_fn_name: false,
			expect_params: false,
			cur_var: None,
			cur_const: None,
			cur_fn: None,
			generics: None,
			generics2: None,
			where_clause: None,
		}
	}
	fn derive(&mut self, attr: &TokenStream, item: &TokenStream) -> Result<TokenStream, Error> {
		self.parse_attr(attr)?;
		self.parse_item(item)?;
		self.semantic_analysis()?;
		self.generate_code()
	}

	fn semantic_analysis(&mut self) -> Result<(), Error> {
		self.do_semantic_analysis()?;
		if self.error_list.len() != 0 {
			self.print_errors()?;
		}
		Ok(())
	}

	fn do_semantic_analysis(&mut self) -> Result<(), Error> {
		let view_set = self.build_view_set(false)?;
		self.check_public_list(&view_set)?;
		self.check_protected_list(&view_set)?;
		let view_set = self.build_view_set(true)?;
		self.check_clone_list(&view_set)?;
		self.check_var_list()?;
		self.check_const_list()?;
		self.check_fn_list()?;
		Ok(())
	}

	fn build_view_set(&self, is_clone: bool) -> Result<HashSet<String>, Error> {
		let mut ret = HashSet::new();
		for fn_info in &self.fn_list {
			for view in &fn_info.views {
				if !is_clone {
					ret.insert(format!("{}_sync", view));
					ret.insert(format!("{}_send", view));
					ret.insert(format!("{}_box", view));
					ret.insert(format!("{}_send_box", view));
					ret.insert(format!("{}_sync_box", view));
				}
				ret.insert(view.clone());
			}
		}
		Ok(ret)
	}

	fn check_var_list(&mut self) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.var_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!("var '{}' already delcared", item.name))?;
			}
			set.insert(item.name);
		}
		Ok(())
	}

	fn check_const_list(&mut self) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.const_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!("const '{}' already delcared", item.name))?;
			}
			set.insert(item.name);
		}
		Ok(())
	}

	fn check_fn_list(&mut self) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.fn_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!("fn '{}' already delcared", item.name))?;
			}
			set.insert(item.name);
		}
		Ok(())
	}

	fn check_clone_list(&mut self, view_set: &HashSet<String>) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.clone_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!(
					"view '{}' already declared to be clone",
					item.name
				))?;
			}
			set.insert(item.name.clone());

			if !view_set.contains(&item.name) {
				self.span = Some(item.span.clone());
				if item.name.rfind("_sync") == Some(item.name.len().saturating_sub(5))
					|| item.name.rfind("_send") == Some(item.name.len().saturating_sub(5))
					|| item.name.rfind("_box") == Some(item.name.len().saturating_sub(4))
				{
					self.append_error(&format!("view '{}' does not exist. Note: do not suffix with 'box', 'send', or 'sync' for clone.", item.name))?;
				} else {
					self.append_error(&format!("view '{}' does not exist", item.name))?;
				}
			}
		}

		Ok(())
	}

	fn check_protected_list(&mut self, view_set: &HashSet<String>) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.protected_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!(
					"view '{}' already declared to be protected",
					item.name
				))?;
			}
			set.insert(item.name.clone());

			if !view_set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!("view '{}' does not exist", item.name))?;
			}
		}

		Ok(())
	}

	fn check_public_list(&mut self, view_set: &HashSet<String>) -> Result<(), Error> {
		let mut set = HashSet::new();
		for item in self.public_list.clone() {
			if set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!(
					"view '{}' already declared to be public",
					item.name
				))?;
			}
			set.insert(item.name.clone());

			if !view_set.contains(&item.name) {
				self.span = Some(item.span.clone());
				self.append_error(&format!("view '{}' does not exist", item.name))?;
			}
		}

		Ok(())
	}

	fn parse_attr(&mut self, strm: &TokenStream) -> Result<(), Error> {
		for token in strm.clone() {
			self.process_token(token)?;
		}

		if self.state != State::Base {
			self.append_error("unexpectedly ended class attribute")?;
			self.print_errors()?;
		}
		if self.error_list.len() != 0 {
			self.print_errors()?;
		}

		Ok(())
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

	fn process_abort(&mut self, msg: String) -> Result<(), Error> {
		match self.span {
			Some(span) => {
				let diag = Diagnostic::spanned(span.into(), Level::Error, msg.to_string().clone());
				abort!(diag, msg);
			}
			None => {
				println!("ERROR: unexpected early abort. No spans!");
			}
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

	fn process_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.state {
			State::Base => self.process_base(token)?,
			State::Public => self.process_public(token)?,
			State::Protected => self.process_protected(token)?,
			State::Clone => self.process_clone(token)?,
			State::Fn => self.process_fn(token)?,
			State::Comment => self.process_comment(token)?,
			State::Var => self.process_var(token)?,
			State::Const => self.process_const(token)?,
			State::Module => self.process_module(token)?,
		}
		Ok(())
	}

	fn process_module(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == ";" {
			self.state = State::Base;
		} else {
			match token {
				Literal(ident) => {
					if self.module.is_some() {
						self.append_error("module already defined")?;
					} else {
						let ident = self.strip_start(&ident.to_string(), '\"');
						let ident = self.strip_end(&ident, '\"');
						self.module = Some(ident);
					}
				}
				_ => {
					self.append_error(&format!(
						"unexpected token, '{}', expected module name",
						token.to_string()
					))?;
				}
			}
		}
		Ok(())
	}

	fn process_protected(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expect_comma = true;
				self.protected_list.push(ProtectedView::new(
					ident.to_string(),
					self.span.as_ref().unwrap().clone(),
				));
			}
			Punct(p) => {
				if p == ';' {
					if !self.expect_comma {
						self.append_error("expected ident")?;
					}
					self.state = State::Base;
				} else if p == ',' {
					if !self.expect_comma {
						self.append_error("unexpected token ','")?;
					} else {
						self.expect_comma = false;
					}
				}
			}
			_ => {
				self.append_error(&format!("unexpected token '{}'", token))?;
			}
		}
		Ok(())
	}

	fn process_clone(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expect_comma = true;
				self.clone_list.push(CloneView::new(
					ident.to_string(),
					self.span.as_ref().unwrap().clone(),
				));
			}
			Punct(p) => {
				if p == ';' {
					if !self.expect_comma {
						self.append_error("expected ident")?;
					}
					self.state = State::Base;
				} else if p == ',' {
					if !self.expect_comma {
						self.append_error("unexpected token ','")?;
					} else {
						self.expect_comma = false;
					}
				}
			}
			_ => {
				self.append_error(&format!("unexpected token '{}'", token))?;
			}
		}
		Ok(())
	}

	fn process_fn(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if self.expect_fn && token_str != "fn" {
			self.append_error(&format!("expected 'fn' found '{}'", token))?;
		} else if self.expect_fn {
			self.expect_fn = false;
			self.expect_fn_name = true;
		} else if self.expect_fn_name {
			match self.cur_fn.as_mut() {
				Some(fn_info) => {
					fn_info.span = self.span.as_ref().unwrap().clone();
				}
				None => {
					let mut fn_info = FnInfo::new(self.span.as_ref().unwrap().clone());
					fn_info.comments.extend(self.comments.clone());
					self.comments.clear();
					self.cur_fn = Some(fn_info);
				}
			}
			match token {
				Ident(ident) => {
					self.cur_fn.as_mut().unwrap().name = ident.to_string();
					self.expect_fn_name = false;
					self.expect_params = true;
				}
				_ => self.append_error(&format!("expected function name found '{}'", token_str))?,
			}
		} else if self.expect_params {
			match token {
				Group(g) => {
					if g.delimiter() != Delimiter::Parenthesis {
						self.append_error(&format!(
							"expected param list found '{}'",
							g.to_string()
						))?;
					} else {
						self.expect_params = false;
						let cur_fn = self.cur_fn.as_mut().unwrap();
						cur_fn.params.extend(g.stream());
						cur_fn.expect_ret_arrow1 = true;
						cur_fn.expect_ret_arrow2 = false;
					}
				}
				_ => self.append_error(&format!("expected param list, found '{}'", token_str))?,
			}
		} else {
			// we're in the return list section
			self.process_return_list_token(token)?;
		}
		Ok(())
	}

	fn process_return_list_token(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(ref group) => {
				if group.delimiter() == Delimiter::Brace {
					match self.cur_fn.as_mut() {
						Some(cur_fn) => {
							cur_fn.fn_block.extend(group.stream());
							self.fn_list.push(cur_fn.clone());
						}
						None => {
							self.append_error("unexpected no cur_fn")?;
						}
					}
					self.cur_fn = None;
					self.expect_params = false;
					self.expect_fn_name = false;
					self.expect_fn = false;
					self.state = State::Base;
				} else {
					match self.cur_fn.as_mut() {
						Some(cur_fn) => {
							if cur_fn.expect_ret_arrow1 {
								self.append_error(&format!("expected '-', found '{}'", token))?;
							} else if cur_fn.expect_ret_arrow2 {
								self.append_error(&format!("expected '>', found '{}'", token))?;
							} else {
								(*cur_fn)
									.return_str
									.extend(group.to_string().parse::<TokenStream>());
							}
						}
						None => {
							self.append_error("unexpected no cur_fn")?;
						}
					}
				}
			}
			_ => match self.cur_fn.as_mut() {
				Some(cur_fn) => {
					if cur_fn.expect_ret_arrow1 {
						if token.to_string() != "-" {
							self.append_error(&format!("expected '-', found '{}'", token))?;
						} else {
							cur_fn.expect_ret_arrow1 = false;
							cur_fn.expect_ret_arrow2 = true;
						}
					} else if cur_fn.expect_ret_arrow2 {
						if token.to_string() != ">" {
							self.append_error(&format!("expected '>', found '{}'", token))?;
						} else {
							cur_fn.expect_ret_arrow2 = false;
						}
					} else {
						(*cur_fn)
							.return_str
							.extend(token.to_string().parse::<TokenStream>());
					}
				}
				None => {
					self.append_error("unexpected no cur_fn")?;
				}
			},
		}
		Ok(())
	}

	fn strip_start(&self, s: &String, ch: char) -> String {
		match s.trim().find(ch) {
			Some(pos) => {
				if pos == 0 {
					s.replace(ch, "").to_string()
				} else {
					s.clone()
				}
			}
			None => s.clone(),
		}
	}

	fn strip_end(&self, s: &String, ch: char) -> String {
		if s.len() == 0 {
			s.clone()
		} else {
			match s.trim().rfind(ch) {
				Some(pos) => {
					if pos == s.len() - 1 {
						s.replace(ch, "").to_string()
					} else {
						s.clone()
					}
				}
				None => s.clone(),
			}
		}
	}

	fn process_comment(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Bracket {
					let mut expect_doc = true;
					let mut expect_equal = false;
					for token in group.stream() {
						let token_str = token.to_string();
						if expect_doc && token_str != "doc" {
							self.append_error(&format!("expected 'doc' found '{}'", token))?;
						} else if expect_doc {
							expect_doc = false;
							expect_equal = true;
						} else if expect_equal && token_str != "=" {
							self.append_error(&format!("expected '=' found '{}'", token))?;
						} else if expect_equal {
							expect_equal = false;
						} else {
							let token_str = self.strip_start(&token_str, '\"');
							let token_str = self.strip_end(&token_str, '\"');
							self.comments.push(token_str);
						}
					}
					self.state = State::Base;
				} else {
					self.append_error(&format!(
						"unexpected token '{:?}'. Expected '['",
						group.delimiter()
					))?;
				}
			}
			_ => {
				self.append_error(&format!("unexpected token '{}'. expected '['", token))?;
			}
		}
		Ok(())
	}

	fn process_var(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();

		if self.cur_var.is_none() {
			match token {
				Ident(ident) => {
					let mut v = Var::new(self.span.as_ref().unwrap().clone());
					v.name = ident.to_string();
					self.cur_var = Some(v);
				}
				_ => {
					self.append_error(&format!("expected var name, found '{}'", token))?;
				}
			}
		} else if !self.cur_var.as_ref().unwrap().found_colon {
			if token_str != ":" {
				self.append_error(&format!("expected ':', found '{}'", token))?;
			} else {
				self.cur_var.as_mut().unwrap().found_colon = true;
			}
		} else if token_str == ";" {
			if self.cur_var.as_ref().unwrap().type_str.len() == 0 {
				self.append_error("expected type string, found ';'")?;
			} else {
				self.var_list.push(self.cur_var.as_ref().unwrap().clone());
				self.cur_var = None;
			}
			self.state = State::Base;
		} else {
			match self.cur_var.as_mut() {
				Some(cur_var) => {
					cur_var.type_str = format!("{} {}", cur_var.type_str, token_str);
				}
				None => {}
			}
		}
		Ok(())
	}

	fn process_const(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		if self.cur_const.is_none() {
			match token {
				Ident(ref ident) => {
					let mut c = Const::new(token.span());
					c.comments.extend(self.comments.clone());
					self.comments.clear();
					c.name = ident.to_string();
					self.cur_const = Some(c);
				}
				_ => {
					self.append_error(&format!("expected const name, found '{}'", token))?;
				}
			}
		} else if !self.cur_const.as_ref().unwrap().found_colon {
			if token_str != ":" {
				self.append_error(&format!("expected ':', found '{}'", token))?;
			} else {
				self.cur_const.as_mut().unwrap().found_colon = true;
			}
		} else if !self.cur_const.as_ref().unwrap().found_equal {
			if token_str == "=" {
				if self.cur_const.as_ref().unwrap().type_str.len() == 0 {
					self.append_error("expected type string, found '='")?;
				} else {
					self.cur_const.as_mut().unwrap().found_equal = true;
				}
			} else {
				if token_str == ";" {
					self.append_error("expected type string, found ';'")?;
					self.state = State::Base;
				} else {
					let type_str = &self.cur_const.as_ref().unwrap().type_str;
					self.cur_const.as_mut().unwrap().type_str =
						format!("{} {}", type_str, token_str);
				}
			}
		} else {
			if token_str == ";" {
				if self.cur_const.as_ref().unwrap().value_str.len() == 0 {
					self.append_error("expected value string, found ';'")?;
					self.state = State::Base;
				} else {
					self.const_list
						.push(self.cur_const.as_ref().unwrap().clone());
					self.cur_const = None;
					self.state = State::Base;
				}
			} else {
				let value_str = &self.cur_const.as_ref().unwrap().value_str;
				self.cur_const.as_mut().unwrap().value_str = format!("{} {}", value_str, token_str);
			}
		}
		Ok(())
	}

	fn process_public(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expect_comma = true;
				let comments = self.comments.clone();
				self.public_list.push(PublicView::new(
					ident.to_string(),
					self.span.as_ref().unwrap().clone(),
					comments,
				));
			}
			Punct(p) => {
				if p == ';' {
					self.comments.clear();
					if !self.expect_comma {
						self.append_error("expected ident")?;
					}
					self.state = State::Base;
				} else if p == ',' {
					if !self.expect_comma {
						self.append_error("unexpected token ','")?;
					} else {
						self.expect_comma = false;
					}
				}
			}
			_ => {
				self.append_error(&format!("unexpected token '{}'", token))?;
			}
		}
		Ok(())
	}

	fn process_base(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "public" {
					self.expect_comma = false;
					self.state = State::Public;
				} else if ident_str == "protected" {
					self.expect_comma = false;
					self.state = State::Protected;
				} else if ident_str == "clone" {
					self.expect_comma = false;
					self.state = State::Clone;
				} else if ident_str == "var" {
					self.state = State::Var;
				} else if ident_str == "const" {
					self.state = State::Const;
				} else if ident_str == "fn" {
					self.state = State::Fn;
					self.expect_fn_name = true;
				} else if ident_str == "module" {
					self.state = State::Module;
				} else {
					self.append_error(&format!("unexpected token: '{}'", ident_str))?;
				}
			}
			Group(ref group) => {
				if group.delimiter() == Delimiter::Bracket {
					self.process_view_list(group)?;
					self.expect_comma = false;
					self.state = State::Fn;
				} else {
					self.append_error(&format!("unexpected token: '{}'", token))?;
				}
			}
			Punct(ref p) => {
				if *p == '#' {
					self.state = State::Comment;
				} else {
					self.append_error(&format!("unexpected token: '{}'", token))?;
				}
			}
			_ => {
				self.append_error(&format!("unexpected token: '{}'", token))?;
			}
		}
		Ok(())
	}

	fn process_view_list(&mut self, group: &Group) -> Result<(), Error> {
		let mut fn_info = FnInfo::new(self.span.as_ref().unwrap().clone());
		fn_info.comments.extend(self.comments.clone());
		self.comments.clear();
		let mut expect_comma = false;
		for token in group.stream() {
			self.span = Some(token.span());
			match token {
				Ident(ident) => {
					if expect_comma {
						self.append_error(&format!("expected ',' found '{}'", ident))?;
					} else {
						expect_comma = true;
						fn_info.views.push(ident.to_string());
					}
				}
				Punct(p) => {
					if p != ',' || !expect_comma {
						if expect_comma {
							self.append_error(&format!("expected ',', found, '{}'", p))?;
						} else {
							self.append_error(&format!("found comma when expected view name"))?;
						}
					} else {
						expect_comma = false;
					}
				}
				_ => self.append_error(&format!("unexpected token: '{}'", token.to_string()))?,
			}
		}
		self.cur_fn = Some(fn_info);
		self.expect_fn = true;
		Ok(())
	}

	fn get_pre_name_clause(&self) -> Result<String, Error> {
		Ok(match &self.generics {
			Some(g) => {
				format!("<{}>", g)
			}
			None => "".to_string(),
		})
	}

	fn get_post_name_clause(&self) -> Result<String, Error> {
		Ok(match &self.generics2 {
			Some(generics) => {
				format!(
					"{}{}",
					format!("<{}>", generics),
					match &self.where_clause {
						Some(w) => {
							format!(" where {}", w)
						}
						None => {
							"".to_string()
						}
					}
				)
			}
			None => "".to_string(),
		})
	}

	fn get_where_clause(&self) -> Result<String, Error> {
		Ok(match &self.where_clause {
			Some(w) => {
				format!(" where {}", w)
			}
			None => "".to_string(),
		})
	}

	fn parse_item(&mut self, strm: &TokenStream) -> Result<(), Error> {
		let mut expect_impl = true;
		let mut expect_name = false;
		let mut in_generics = false;
		let mut expect_group = false;
		let mut term_where = false;
		let mut in_where = false;

		for token in strm.clone() {
			self.span = Some(token.span());
			let token_str = token.to_string();
			if expect_impl && token_str != "impl" {
				self.process_abort("expected keyword impl".to_string())?;
			} else if expect_impl {
				expect_impl = false;
				expect_name = true;
			} else if expect_name {
				match token {
					Ident(ref ident) => {
						if !in_generics {
							self.name = Some(ident.to_string());
							expect_name = false;
							expect_group = true;
						} else {
							match self.generics.as_mut() {
								Some(generic) => {
									*generic = format!("{}{}", *generic, ident.to_string());
								}
								None => {
									self.generics = Some(ident.to_string());
								}
							}
						}
					}
					Punct(ref p) => {
						if p.to_string() == "<" {
							in_generics = true;
						} else if p.to_string() == ">" {
							in_generics = false;
						} else {
							match self.generics.as_mut() {
								Some(generic) => {
									*generic = format!("{}{}", *generic, p.to_string());
								}
								None => {
									self.generics = Some(p.to_string());
								}
							}
						}
					}
					_ => {
						self.process_abort("expected class name".to_string())?;
					}
				}
			} else if expect_group {
				if self.generics.is_some() {
					if term_where {
						self.process_abort("unterminated after impl clause".to_string())?;
					}
					match token {
						Group(ref group) => {
							if group.delimiter() == Delimiter::Brace {
								term_where = true;
								if group.stream().to_string().len() != 0 {
									self.process_abort("impl block must be empty".to_string())?;
								}
							}
						}
						_ => {}
					}
					if self.generics2.is_none() && token.to_string() == "<" {
					} else if !in_where && token.to_string() != "where" {
						if !term_where {
							match self.generics2.as_mut() {
								Some(g) => match token {
									Ident(ident) => {
										*g = format!("{}{}", *g, ident.to_string());
									}
									_ => {
										*g = format!("{}{}", *g, token.to_string());
									}
								},
								None => {
									self.generics2 = Some(token.to_string());
								}
							}
						}
					} else if token.to_string() == "where" {
						in_where = true;
					} else {
						if !term_where {
							match self.where_clause.as_mut() {
								Some(w) => match token {
									Ident(ident) => {
										*w = format!("{}{}", *w, ident.to_string());
									}
									_ => {
										*w = format!("{}{}", *w, token.to_string());
									}
								},
								None => {
									self.where_clause = Some(token.to_string());
								}
							}
						}
					}
				} else {
					match token {
						Group(ref group) => {
							expect_group = false;
							for _group_item in group.stream() {
								self.process_abort("impl must be empty for classes".to_string())?;
							}
						}
						_ => self.process_abort("expected 'impl <name> {}'".to_string())?,
					}
				}
			} else {
				self.process_abort("unexpected token".to_string())?;
			}
			match self.generics2.as_mut() {
				Some(g) => {
					if (*g).rfind(">") == Some((*g).len().saturating_sub(1)) {
						*g = (*g).substring(0, (*g).len().saturating_sub(1)).to_string();
					}
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn generate_code(&self) -> Result<TokenStream, Error> {
		let template = include_str!("../templates/class_template.txt").to_string();
		let template = self.update_structs(template)?;
		let template = self.update_impl_struct(template)?;
		let template = self.update_impl_var(template)?;
		let template = self.update_impl_const(template)?;
		let template = self.update_traits(template)?;
		let template = self.update_trait_impl(template)?;
		let template = self.update_macros(template)?;
		let template = self.update_builder(template)?;
		Ok(map_err!(template.parse::<TokenStream>(), Parse)?)
	}

	fn build_clone_set(&self) -> HashSet<String> {
		let mut ret = HashSet::new();
		for item in &self.clone_list {
			ret.insert(item.name.clone());
		}
		ret
	}

	fn build_trait_hashmap(&self) -> HashMap<String, Vec<FnInfo>> {
		let mut ret: HashMap<String, Vec<FnInfo>> = HashMap::new();
		for item in &self.fn_list {
			for view in &item.views {
				match ret.get_mut(view) {
					Some(fn_vec) => {
						fn_vec.push(item.clone());
					}
					None => {
						ret.insert(view.clone(), vec![item.clone()]);
					}
				}
			}
		}
		ret
	}

	fn get_visibility_trait(
		&self,
		name: &String,
		public_set: &HashSet<String>,
		protected_set: &HashSet<String>,
	) -> String {
		if public_set.contains(name)
			|| public_set.contains(&format!("{}_box", name))
			|| public_set.contains(&format!("{}_sync", name))
			|| public_set.contains(&format!("{}_box", name))
			|| public_set.contains(&format!("{}_sync_box", name))
			|| public_set.contains(&format!("{}_send_box", name))
		{
			"pub ".to_string()
		} else if protected_set.contains(name)
			|| protected_set.contains(&format!("{}_box", name))
			|| protected_set.contains(&format!("{}_sync", name))
			|| protected_set.contains(&format!("{}_box", name))
			|| protected_set.contains(&format!("{}_sync_box", name))
			|| protected_set.contains(&format!("{}_send_box", name))
		{
			"pub(crate) ".to_string()
		} else {
			"".to_string()
		}
	}

	fn build_public_set(&self) -> HashSet<String> {
		let mut ret = HashSet::new();
		for item in &self.public_list {
			ret.insert(item.name.clone());
		}
		ret
	}

	fn build_protected_set(&self) -> HashSet<String> {
		let mut ret = HashSet::new();
		for item in &self.protected_list {
			ret.insert(item.name.clone());
		}
		ret
	}

	fn update_traits(&self, template: String) -> Result<String, Error> {
		let view_set = self.build_view_set(true)?;
		let clone_set = self.build_clone_set();
		let trait_hashmap = self.build_trait_hashmap();
		let public_set = self.build_public_set();
		let protected_set = self.build_protected_set();
		let mut all_traits = "".to_string();

		for view in view_set {
			let trait_name = view.to_case(Case::Pascal);
			let trait_visibility = self.get_visibility_trait(&view, &public_set, &protected_set);
			let clone_text = if clone_set.contains(&view) {
				": bmw_deps::dyn_clone::DynClone"
			} else {
				""
			};

			let mut trait_comments = "".to_string();
			for public in &self.public_list {
				if public.name == view
					|| public.name == format!("{}_box", view)
					|| public.name == format!("{}_send_box", view)
					|| public.name == format!("{}_sync_box", view)
					|| public.name == format!("{}_send", view)
					|| public.name == format!("{}_sync", view)
				{
					for comment in &public.comments {
						trait_comments = format!("{}///{}\n", trait_comments, comment);
					}
					break;
				}
			}

			let mut trait_text = format!(
				"{}{}trait {} {}{}{{",
				trait_comments,
				trait_visibility,
				trait_name,
				&self.get_post_name_clause()?,
				clone_text,
			);

			match trait_hashmap.get(&view) {
				Some(fn_vec) => {
					for fn_info in fn_vec {
						trait_text = format!("{}\n\t#[document]", trait_text);
						for comment in &fn_info.comments {
							trait_text = format!("{}\n\t///{}", trait_text, comment);
						}
						trait_text = format!(
							"{}\n\tfn {}({}){}{};",
							trait_text,
							fn_info.name,
							fn_info.params.to_string(),
							if fn_info.return_str.to_string().len() > 0 {
								" -> "
							} else {
								""
							},
							fn_info.return_str.to_string()
						);
					}
				}
				None => {}
			}

			let trait_text = format!("{}\n}}", trait_text);
			all_traits = format!("{}{}\n", all_traits, trait_text);
		}

		let template = template.replace("${TRAITS}", &all_traits);
		Ok(template)
	}

	fn convert_param_string(&self, param_string: &String) -> Result<(String, String), Error> {
		let mut ret_types = format!("");
		let strm = map_err!(param_string.parse::<TokenStream>(), Parse)?;
		let mut ret = "".to_string();
		let mut first = true;
		let mut in_type = false;
		let mut gtlt_delim_sum = 0u128;
		for token in strm {
			match token {
				Group(token) => {
					ret = format!("{}(", ret);
					for token in token.stream() {
						let token = token.to_string();
						if first && (token == "&" || token == "mut") {
						} else {
							first = false;
							if token == ":" {
								in_type = true;
							} else if !in_type {
								ret = format!("{}{}", ret, token);
							} else {
								ret_types = format!("{} {}", ret_types, token);
								if token == "<" {
									gtlt_delim_sum += 1;
								} else if token == ">" {
									gtlt_delim_sum = gtlt_delim_sum.saturating_sub(1);
								} else if token == "," && gtlt_delim_sum == 0 {
									in_type = false;
									ret = format!("{}{}", ret, token);
								}
							}
						}
					}
					ret = format!("{})", ret);
				}
				_ => {
					ret = format!("{}{}", ret, token);
				}
			}
		}
		Ok((ret, ret_types))
	}

	fn update_trait_impl(&self, template: String) -> Result<String, Error> {
		let mut all_trait_impls = "".to_string();
		let mut all_trait_impls_mut = "".to_string();
		let trait_hashmap = self.build_trait_hashmap();
		let clone_set = self.build_clone_set();
		let name = self.name.as_ref().unwrap();

		for (view, fn_vec) in &trait_hashmap {
			let trait_name = view.to_case(Case::Pascal);
			all_trait_impls = format!(
				"{}\nimpl {} {} {} for {} {} {{",
				all_trait_impls,
				&self.get_pre_name_clause()?,
				trait_name,
				&self.get_pre_name_clause()?,
				name,
				&self.get_post_name_clause()?,
			);

			if !clone_set.contains(view) {
				all_trait_impls_mut = format!(
					"{}\nimpl {} {} {} for &mut {} {} {{\n",
					all_trait_impls_mut,
					&self.get_pre_name_clause()?,
					trait_name,
					&self.get_pre_name_clause()?,
					name,
					&self.get_post_name_clause()?,
				);
			}

			for fn_info in fn_vec {
				let param_pair =
					self.convert_param_string(&format!("({})", fn_info.params.to_string()))?;
				all_trait_impls = format!(
					"{}\n\tfn {}({}){} {{ {}::{}{} }}",
					all_trait_impls,
					fn_info.name,
					fn_info.params,
					if fn_info.return_str.to_string().len() > 0 {
						format!(" -> {}", fn_info.return_str)
					} else {
						"".to_string()
					},
					name,
					fn_info.name,
					param_pair.0
				);

				if !clone_set.contains(view) {
					all_trait_impls_mut = format!(
						"{}\n\tfn {}({}){} {{ {}::{}{} }}",
						all_trait_impls_mut,
						fn_info.name,
						fn_info.params,
						if fn_info.return_str.to_string().len() > 0 {
							format!(" -> {}", fn_info.return_str)
						} else {
							"".to_string()
						},
						name,
						fn_info.name,
						param_pair.0
					);
				}
			}

			all_trait_impls = format!("{}\n}}", all_trait_impls);
			if !clone_set.contains(view) {
				all_trait_impls_mut = format!("{}\n}}", all_trait_impls_mut);
			}
		}

		let all_trait_impls = format!("{}{}", all_trait_impls, all_trait_impls_mut);
		let template = template
			.replace("${TRAIT_IMPL}", &all_trait_impls)
			.to_string();
		Ok(template)
	}

	fn format_type_str(&self, type_str: &String) -> Result<(String, bool), Error> {
		let mut allow_multi = false;
		// we only need to special handling on our tuple and vec structures

		let value = match type_str.find("Vec") {
			Some(_) => {
				// it's a Vec type
				allow_multi = true;
				let start = match type_str.find("<") {
					Some(pos) => {
						if pos + 1 < type_str.len() {
							pos + 1
						} else {
							return err!(
								BaseErrorKind::IllegalState,
								"unexpected end of string: '{}'",
								type_str
							);
						}
					}
					None => return err!(BaseErrorKind::IllegalState, "expected '<'"),
				};

				let end = match type_str.rfind(">") {
					Some(end) => end,
					None => {
						return err!(
							BaseErrorKind::IllegalState,
							"expected '>' in '{}'",
							type_str
						)
					}
				};

				type_str.substring(start, end).to_string()
			}
			None => type_str.clone(),
		};

		let value = value.trim();

		let first_string = value.find("String");
		let last_string = value.rfind("String");
		if first_string.is_some() && last_string.is_some() && first_string != last_string {
			// only legal value would be a tuple here so return the formatted version
			Ok(("(&[`str`], &[`str`])".to_string(), allow_multi))
		} else if first_string.is_some() {
			// replace with &str
			Ok(("&[`str`]".to_string(), allow_multi))
		} else {
			Ok((format!("[`{}`]", value), allow_multi))
		}
	}

	fn format_value_str(&self, value_str: &String) -> String {
		let value_str = value_str.replace("\"", "\\\"");
		let value_str = value_str.replace("to_string", "");
		let value_str = value_str.replace(".", "");
		let value_str = value_str.replace("()", "");
		let value_str = value_str.replace("vec", "");
		let value_str = value_str.replace("!", "");

		value_str.to_string()
	}

	fn update_comments(
		&self,
		template: String,
		replacement: &str,
		module: Option<&String>,
		macro_name: String,
		class_name: &String,
		trait_name: &str,
		is_box: bool,
		is_send: bool,
		is_sync: bool,
		is_builder: bool,
	) -> Result<String, Error> {
		let builder_name = format!("{}Builder", class_name);
		let public_set = self.build_public_set();
		let visible = self
			.get_macro_pub_visibility(&macro_name, &public_set)
			.len() != 0;

		let comment_builder = if !visible {
			"".to_string()
		} else {
			let comment_builder = "".to_string();
			let comment_builder = format!(
				"{}#[doc=\"Builds an instance of the [`{}`]\"]\n",
				comment_builder, trait_name
			);
			let comment_builder = format!(
				"{}#[doc=\"trait using the specified input parameters.\"]\n",
				comment_builder
			);
			let comment_builder = format!("{}#[doc=\"# Input Parameters\"]\n", comment_builder);
			let comment_builder = format!(
				"{}#[doc=\"| Parameter | Multi | Description | Default Value |\"]\n",
				comment_builder
			);
			let mut comment_builder = format!("{}#[doc=\"|---|---|---|---|\"]\n", comment_builder);
			for value in &self.const_list {
				let mut description = format!("");
				for comment in &value.comments {
					description = format!("{} {}", description, comment);
				}
				let value_pascal = value.name.to_case(Case::Pascal);
				let (formatted_type_str, allow_multi) = self.format_type_str(&value.type_str)?;
				let value_param = format!("{}({})", value_pascal, formatted_type_str);
				comment_builder = format!(
					"{}#[doc=\"| {} | {} | {} | {} |\n\"]\n",
					comment_builder,
					value_param,
					if allow_multi { "yes" } else { "no" },
					description,
					self.format_value_str(&value.value_str)
				);
			}
			let comment_builder = format!("{}#[doc=\"# Return\"]\n", comment_builder);
			let mut ret_type = format!("[`{}`]", trait_name);
			if is_sync {
				ret_type = format!("{} + [`Send`] + [`Sync`]", ret_type);
			} else if is_send {
				ret_type = format!("{} + [`Send`]", ret_type);
			}
			if is_box {
				ret_type = format!("[`Box`]<dyn {}>", ret_type);
			} else {
				ret_type = format!("`impl` {}", ret_type);
			}
			let comment_builder = format!(
				"{}#[doc=\"This macro returns a [`Result`]<{}, [`Error`]>.\n\"]\n",
				comment_builder, ret_type
			);
			let comment_builder = format!("{}#[doc=\"# Errors\"]\n", comment_builder);

			let comment_builder = format!(
				"{}\n#[doc=\"* [`bmw_core::BaseErrorKind::Builder`] -",
				comment_builder
			);
			let comment_builder = format!(
				"{}If the builder function returns an error,",
				comment_builder
			);
			let comment_builder = format!(
				"{} it will be wrapped in an error of the kind `Builder` with the details",
				comment_builder
			);
			let comment_builder =
				format!("{} of the original error preserved.\"]", comment_builder);

			let comment_builder = format!("{}#[doc=\"\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"# Also See\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"* [`{}`]\"]\n", comment_builder, trait_name);
			let mut comment_builder = format!(
				"{}#[doc=\"* [`bmw_core::BaseErrorKind`]\"]\n",
				comment_builder
			);

			if is_send {
				comment_builder = format!("{}#[doc=\"* [`Send`]\"]\n", comment_builder);
			} else if is_sync {
				comment_builder = format!("{}#[doc=\"* [`Send`]\"]\n", comment_builder);
				comment_builder = format!("{}#[doc=\"* [`Sync`]\"]\n", comment_builder);
			}
			if is_box {
				comment_builder = format!("{}#[doc=\"* [`Box`]\"]\n", comment_builder);
			}

			let comment_builder = format!("{}#[doc=\"# Examples\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"```\"]\n", comment_builder);
			let comment_builder = format!(
				"{}#[doc=\"// use bmw_core::*, the macro, and the builder\"]\n",
				comment_builder
			);
			let mut comment_builder = format!("{}#[doc=\"use bmw_core::*;\"]\n", comment_builder);

			let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

			match module {
				Some(module) => {
					comment_builder = format!("{}\n#[doc=\"use {}::*;\"]", comment_builder, module);
					comment_builder = format!(
						"{}\n#[doc=\"use {}::{};\"]",
						comment_builder, crate_name, macro_name
					);
				}
				None => {
					comment_builder =
						format!("{}\n#[doc=\"use {}::*;\"]", comment_builder, crate_name,);
				}
			}

			let comment_builder = format!("{}#[doc=\"\"]\n", comment_builder);
			let comment_builder = format!(
				"{}#[doc=\"fn main() -> Result<(), Error> {{\"]\n",
				comment_builder
			);
			let comment_builder = format!(
				"{}#[doc=\"\t// build a {} with default parameters.\"]\n",
				comment_builder, trait_name
			);
			let comment_builder = if is_builder {
				format!(
					"{}#[doc=\"\tlet object = {}::build_{}(vec![])?;\"]\n",
					comment_builder, builder_name, macro_name
				)
			} else {
				format!(
					"{}#[doc=\"\tlet object = {}!()?;\"]\n",
					comment_builder, macro_name
				)
			};
			let comment_builder = format!("{}#[doc=\"\t// use object...\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"\"]\n", comment_builder);

			let comment_builder = format!(
				"{}#[doc=\"\t// build a {} with parameters explicitly specified.\"]\n",
				comment_builder, trait_name
			);
			let mut comment_builder = if is_builder {
				format!(
					"{}#[doc=\"\tlet object = {}::build_{}(vec![\"]\n",
					comment_builder, builder_name, macro_name
				)
			} else {
				format!(
					"{}#[doc=\"\tlet object = {}!(\"]\n",
					comment_builder, macro_name
				)
			};

			for param in &self.const_list {
				let pascal = param.name.to_case(Case::Pascal);
				let default_value = param.value_str.clone();
				let default_value = default_value.trim();

				if default_value.find("vec").is_some() || default_value.find("(").is_some() {
					// bypass Vec and tuple
				} else {
					comment_builder = format!(
						"{}#[doc=\"\t\t{}({}),\"]\n",
						comment_builder, pascal, default_value
					);
				}
			}

			let comment_builder = if is_builder {
				format!("{}#[doc=\"\t])?;\"]\n", comment_builder,)
			} else {
				format!("{}#[doc=\"\t)?;\"]\n", comment_builder,)
			};
			let comment_builder = format!("{}#[doc=\"\t// use object...\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"\tOk(())\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"}}\"]\n", comment_builder);
			let comment_builder = format!("{}#[doc=\"```\"]\n", comment_builder);
			comment_builder
		};
		Ok(template.replace(replacement, &comment_builder).to_string())
	}

	fn update_macros(&self, template: String) -> Result<String, Error> {
		let name = self.name.as_ref().unwrap();
		let public_set = self.build_public_set();
		let protected_set = self.build_protected_set();
		let view_set = self.build_view_set(true)?;
		let mut all_macros = "".to_string();
		for view in &view_set {
			let trait_name = view.to_case(Case::Pascal);
			let macro_template = include_str!("../templates/class_macro_template.txt");
			let macro_template = macro_template.replace("${NAME}", name).to_string();
			let macro_template = macro_template.replace("${VIEW}", view).to_string();
			let macro_template = self.update_comments(
				macro_template,
				"${IMPL_COMMENTS}",
				self.module.as_ref(),
				view.to_string(),
				name,
				&trait_name,
				false,
				false,
				false,
				false,
			)?;
			let macro_template = self.update_comments(
				macro_template,
				"${BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_box", view),
				name,
				&trait_name,
				true,
				false,
				false,
				false,
			)?;
			let macro_template = self.update_comments(
				macro_template,
				"${SEND_IMPL_COMMENTS}",
				self.module.as_ref(),
				format!("{}_send", view),
				name,
				&trait_name,
				false,
				true,
				false,
				false,
			)?;
			let macro_template = self.update_comments(
				macro_template,
				"${SEND_BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_send_box", view),
				name,
				&trait_name,
				true,
				true,
				false,
				false,
			)?;
			let macro_template = self.update_comments(
				macro_template,
				"${SYNC_IMPL_COMMENTS}",
				self.module.as_ref(),
				format!("{}_sync", view),
				name,
				&trait_name,
				false,
				false,
				true,
				false,
			)?;
			let macro_template = self.update_comments(
				macro_template,
				"${SYNC_BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_sync_box", view),
				name,
				&trait_name,
				true,
				false,
				true,
				false,
			)?;
			let macro_template = macro_template
				.replace(
					"${IMPL_PUBLIC}",
					self.get_macro_pub_visibility(view, &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${BOX_PUBLIC}",
					self.get_macro_pub_visibility(&format!("{}_box", view), &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${BOX_SEND_PUBLIC}",
					self.get_macro_pub_visibility(&format!("{}_send_box", view), &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${IMPL_SEND_PUBLIC}",
					self.get_macro_pub_visibility(&format!("{}_send", view), &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${BOX_SYNC_PUBLIC}",
					self.get_macro_pub_visibility(&format!("{}_sync_box", view), &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${IMPL_SYNC_PUBLIC}",
					self.get_macro_pub_visibility(&format!("{}_sync", view), &public_set),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${IMPL_PROTECTED}",
					&self.get_macro_protected_visibility(view, &public_set, &protected_set),
				)
				.to_string();

			let macro_template = macro_template
				.replace(
					"${BOX_PROTECTED}",
					&self.get_macro_protected_visibility(
						&format!("{}_box", view),
						&public_set,
						&protected_set,
					),
				)
				.to_string();

			let macro_template = macro_template
				.replace(
					"${IMPL_SEND_PROTECTED}",
					&self.get_macro_protected_visibility(
						&format!("{}_send", view),
						&public_set,
						&protected_set,
					),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${BOX_SEND_PROTECTED}",
					&self.get_macro_protected_visibility(
						&format!("{}_send_box", view),
						&public_set,
						&protected_set,
					),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${IMPL_SYNC_PROTECTED}",
					&self.get_macro_protected_visibility(
						&format!("{}_sync", view),
						&public_set,
						&protected_set,
					),
				)
				.to_string();
			let macro_template = macro_template
				.replace(
					"${BOX_SYNC_PROTECTED}",
					&self.get_macro_protected_visibility(
						&format!("{}_sync_box", view),
						&public_set,
						&protected_set,
					),
				)
				.to_string();

			all_macros = format!("{}{}", all_macros, macro_template);
		}

		let template = template.replace("${MACROS}", &all_macros).to_string();
		Ok(template)
	}

	fn get_macro_pub_visibility(&self, name: &String, public_set: &HashSet<String>) -> &str {
		if public_set.get(name).is_some() {
			"#[macro_export]\n"
		} else {
			""
		}
	}

	fn get_macro_protected_visibility(
		&self,
		name: &String,
		public_set: &HashSet<String>,
		protected_set: &HashSet<String>,
	) -> String {
		if !public_set.get(name).is_some() && protected_set.get(name).is_some() {
			format!("pub(crate) use {};", name)
		} else {
			"".to_string()
		}
	}

	fn get_visibility(
		&self,
		name: &String,
		public_set: &HashSet<String>,
		protected_set: &HashSet<String>,
	) -> &str {
		if public_set.get(name).is_some() {
			"pub "
		} else if protected_set.get(name).is_some() {
			"pub(crate) "
		} else {
			""
		}
	}

	fn update_builder(&self, template: String) -> Result<String, Error> {
		let name = self.name.as_ref().unwrap();
		let public_set = self.build_public_set();
		let protected_set = self.build_protected_set();
		let view_set = self.build_view_set(true)?;
		let builder_text = format!("/// Builder for `{}` class.", name);
		let mut builder_text = format!(
			"{}\npub struct {}Builder {{}}\nimpl {}Builder {{",
			builder_text, name, name
		);
		for view in &view_set {
			let trait_name = view.to_case(Case::Pascal);
			let builder_template = include_str!("../templates/class_builder_template.txt");
			let builder_template = builder_template
				.replace("${GENERIC_PRE}", &self.get_pre_name_clause()?)
				.to_string();
			let builder_template = builder_template
				.replace("${WHERE_CLAUSE}", &self.get_where_clause()?)
				.to_string();
			let builder_template = builder_template.replace("${NAME}", name).to_string();
			let builder_template = builder_template.replace("${TRAIT}", &trait_name);

			let builder_template = self.update_comments(
				builder_template,
				"${IMPL_COMMENTS}",
				self.module.as_ref(),
				format!("{}", view),
				name,
				&trait_name,
				false,
				false,
				false,
				true,
			)?;

			let builder_template = self.update_comments(
				builder_template,
				"${BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_box", view),
				name,
				&trait_name,
				true,
				false,
				false,
				true,
			)?;

			let builder_template = self.update_comments(
				builder_template,
				"${SEND_IMPL_COMMENTS}",
				self.module.as_ref(),
				format!("{}_send", view),
				name,
				&trait_name,
				false,
				true,
				false,
				true,
			)?;

			let builder_template = self.update_comments(
				builder_template,
				"${SEND_BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_send_box", view),
				name,
				&trait_name,
				true,
				true,
				false,
				true,
			)?;

			let builder_template = self.update_comments(
				builder_template,
				"${SYNC_IMPL_COMMENTS}",
				self.module.as_ref(),
				format!("{}_sync", view),
				name,
				&trait_name,
				false,
				false,
				true,
				true,
			)?;

			let builder_template = self.update_comments(
				builder_template,
				"${SYNC_BOX_COMMENTS}",
				self.module.as_ref(),
				format!("{}_sync_box", view),
				name,
				&trait_name,
				true,
				false,
				true,
				true,
			)?;

			let builder_template = builder_template.replace(
				"${VISIBILITY_BOX}",
				self.get_visibility(&format!("{}_box", view), &public_set, &protected_set),
			);
			let builder_template = builder_template.replace(
				"${VISIBILITY_IMPL}",
				self.get_visibility(view, &public_set, &protected_set),
			);
			let builder_template = builder_template.replace(
				"${VISIBILITY_SEND_BOX}",
				self.get_visibility(&format!("{}_send_box", view), &public_set, &protected_set),
			);
			let builder_template = builder_template.replace(
				"${VISIBILITY_SEND_IMPL}",
				self.get_visibility(&format!("{}_send", view), &public_set, &protected_set),
			);
			let builder_template = builder_template.replace(
				"${VISIBILITY_SYNC_BOX}",
				self.get_visibility(&format!("{}_sync_box", view), &public_set, &protected_set),
			);
			let builder_template = builder_template.replace(
				"${VISIBILITY_SYNC_IMPL}",
				self.get_visibility(&format!("{}_sync", view), &public_set, &protected_set),
			);
			let builder_template = builder_template.replace("${VIEW}", view);
			builder_text = format!("{}{}", builder_text, builder_template);
		}
		builder_text = format!("{}}}", builder_text);

		let template = template.replace("${BUILDER}", &builder_text).to_string();
		Ok(template)
	}

	fn update_impl_struct(&self, template: String) -> Result<String, Error> {
		let name = self.name.as_ref().unwrap();

		// create the main struct impl
		let mut main_impl = format!(
			"impl{} {} {} {{",
			&self.get_pre_name_clause()?,
			name,
			&self.get_post_name_clause()?
		);

		for fn_info in &self.fn_list {
			if fn_info.name == "builder" {
				let impl_builder = include_str!("../templates/class_impl_builder_template.txt");
				let impl_builder = impl_builder.replace("${NAME}", name);
				main_impl = format!("{}\n{}", main_impl, impl_builder);
			} else {
				main_impl = format!(
					"{}\n\tfn {}({}) {} {} {{\n\t\t{}\n\t}}\n",
					main_impl,
					fn_info.name,
					fn_info.params,
					if fn_info.return_str.to_string().trim().len() > 0 {
						" -> "
					} else {
						""
					},
					fn_info.return_str,
					fn_info.fn_block
				);
			}
		}

		let get_bytes_template = include_str!("../templates/class_impl_get_template.txt");
		let get_mut_bytes_template = include_str!("../templates/class_impl_get_mut_template.txt");

		for const_param in &self.const_list {
			let getter = get_bytes_template.replace("${PARAM_NAME}", &const_param.name);
			let getter = getter.replace("${PARAM_TYPE}", &const_param.type_str);
			main_impl = format!("{}\n{}", main_impl, getter);
		}

		for var_param in &self.var_list {
			let mutter = get_mut_bytes_template.replace("${PARAM_NAME}", &var_param.name);
			let mutter = mutter.replace("${PARAM_TYPE}", &var_param.type_str);
			main_impl = format!("{}\n{}", main_impl, mutter);
		}

		let main_impl = format!("{}\n}}", main_impl);

		let template = template.replace("${IMPL_STRUCT}", &main_impl).to_string();
		Ok(template)
	}

	fn update_impl_var(&self, template: String) -> Result<String, Error> {
		let name = self.name.as_ref().unwrap();
		// create the Var impl
		let get_mut_bytes_template = include_str!("../templates/class_get_mut_template.txt");
		let mut var_impl = format!(
			"impl{} {}Var {} {{",
			&self.get_pre_name_clause()?,
			name,
			&self.get_post_name_clause()?
		);
		for var_param in &self.var_list {
			let mutter = get_mut_bytes_template.replace("${PARAM_NAME}", &var_param.name);
			let mutter = mutter.replace("${PARAM_TYPE}", &var_param.type_str);
			var_impl = format!("{}\n{}", var_impl, mutter);
		}

		// add builder
		for fn_info in &self.fn_list {
			if fn_info.name == "builder" {
				let param_list = &fn_info.params.to_string();
				let param_name = self.strip_start(&param_list, '&');
				let param_name = self.strip_start(&param_name, ')');
				var_impl = format!(
					"{}\n\tfn builder({}: &{}Const) -> Result<Self, Error> {{\n\t\t{}\n\t}} \n",
					var_impl, param_name, name, fn_info.fn_block
				);
			}
		}

		let var_impl = format!("{}}}", var_impl);

		let template = template.replace("${IMPL_VAR}", &var_impl).to_string();
		Ok(template)
	}

	fn update_impl_const(&self, template: String) -> Result<String, Error> {
		// create the Const impl
		let name = self.name.as_ref().unwrap();
		let get_bytes_template = include_str!("../templates/class_get_template.txt");
		let mut const_impl = format!("impl {}Const {{", name);
		for const_param in &self.const_list {
			let getter = get_bytes_template.replace("${PARAM_NAME}", &const_param.name);
			let getter = getter.replace("${PARAM_TYPE}", &const_param.type_str);
			const_impl = format!("{}\n{}", const_impl, getter);
		}
		let const_impl = format!("{}}}", const_impl);
		let template = template.replace("${IMPL_CONST}", &const_impl).to_string();
		Ok(template)
	}

	fn update_structs(&self, template: String) -> Result<String, Error> {
		let name = self.name.as_ref().unwrap();
		let template = template.replace("${NAME}", &name).to_string();
		let template = template
			.replace("${GENERICS}", &self.get_post_name_clause()?)
			.to_string();
		let template = template
			.replace("${GENERICS_PRE}", &self.get_pre_name_clause()?)
			.to_string();

		let template = template
			.replace(
				"${CLONE}",
				if self.clone_list.len() == 0 {
					""
				} else {
					"#[derive(Clone)]"
				},
			)
			.to_string();

		let mut var_params = "".to_string();
		for var_param in &self.var_list {
			var_params = format!(
				"{}\n\t{}: {},",
				var_params, var_param.name, var_param.type_str
			);
		}
		let template = template.replace("${VAR_PARAMS}", &var_params);

		let mut const_params = "".to_string();
		// export the config options here
		let mut conf_default = format!("#[doc(hidden)]\npub use {}ConstOptions::*;", name);
		conf_default = format!("{}\nimpl Default for {}Const {{", conf_default, name);
		conf_default = format!("{}\n\tfn default() -> Self {{\n\t\tSelf {{", conf_default);
		for const_param in &self.const_list {
			const_params = format!(
				"{}\n\t{}: {},",
				const_params, const_param.name, const_param.type_str
			);
			conf_default = format!(
				"{}\n\t\t\t{}: {},",
				conf_default, const_param.name, const_param.value_str
			);
		}

		conf_default = format!("{}\n\t\t}}\n\t}}\n}}", conf_default);
		let template = template
			.replace("${CONST_PARAMS}", &const_params)
			.to_string();
		let template = template
			.replace("${CONST_DEFAULT}", &conf_default)
			.to_string();

		Ok(template)
	}
}

pub(crate) fn do_derive_class(attr: TokenStream, item: TokenStream) -> TokenStream {
	match do_derive_class_impl(&attr, &item) {
		Ok(token_stream) => token_stream,
		Err(e) => {
			println!("do_derive_class_impl generated error: {}", e);
			item
		}
	}
}

fn do_derive_class_impl(attr: &TokenStream, item: &TokenStream) -> Result<TokenStream, Error> {
	StateMachine::new().derive(attr, item)
}
