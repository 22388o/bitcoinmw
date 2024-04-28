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
use proc_macro::TokenTree::*;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};

struct SpanError {
	span: Span,
	msg: String,
}

#[derive(Clone, Debug)]
struct Var {
	name: String,
	type_str: String,
	found_colon: bool,
}

impl Var {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			type_str: "".to_string(),
			found_colon: false,
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
}

impl Const {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			type_str: "".to_string(),
			value_str: "".to_string(),
			found_colon: false,
			found_equal: false,
			comments: vec![],
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
	Comment,
}

struct StateMachine {
	state: State,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	public_list: Vec<String>,
	protected_list: Vec<String>,
	clone_list: Vec<String>,
	var_list: Vec<Var>,
	const_list: Vec<Const>,
	expect_comma: bool,
	cur_var: Option<Var>,
	cur_const: Option<Const>,
}

impl StateMachine {
	fn new() -> Self {
		Self {
			state: State::Base,
			span: None,
			error_list: vec![],
			public_list: vec![],
			protected_list: vec![],
			clone_list: vec![],
			var_list: vec![],
			const_list: vec![],
			expect_comma: false,
			cur_var: None,
			cur_const: None,
		}
	}
	fn derive(&mut self, attr: &TokenStream, item: &TokenStream) -> Result<TokenStream, Error> {
		self.parse_attr(attr)?;
		self.parse_item(item)?;
		self.build_response()
	}
	fn parse_attr(&mut self, strm: &TokenStream) -> Result<(), Error> {
		for token in strm.clone() {
			self.process_token(token)?;
		}

		if self.state != State::Base {
			self.abort("unexpectedly ended class attribute")?;
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

	fn abort(&mut self, msg: &str) -> Result<(), Error> {
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
		}
		Ok(())
	}

	fn process_protected(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				self.expect_comma = true;
				self.protected_list.push(ident.to_string());
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
				self.clone_list.push(ident.to_string());
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
		Ok(())
	}

	fn process_comment(&mut self, token: TokenTree) -> Result<(), Error> {
		Ok(())
	}

	fn process_var(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();

		if self.cur_var.is_none() {
			match token {
				Ident(ident) => {
					let mut v = Var::new();
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
		println!("token={}", token_str);
		if self.cur_const.is_none() {
			match token {
				Ident(ident) => {
					let mut c = Const::new();
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
			println!("not found equal");
			if token_str == "=" {
				println!("x");
				if self.cur_const.as_ref().unwrap().type_str.len() == 0 {
					println!("y");
					self.append_error("expected type string, found '='")?;
				} else {
					println!("z");
					self.cur_const.as_mut().unwrap().found_equal = true;
				}
			} else {
				println!("else: {}", token_str);
				if token_str == ";" {
					println!("err");
					self.append_error("expected type string, found ';'")?;
					self.state = State::Base;
				} else {
					println!("append");
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
				println!("token_debug vstring = {:?}", token);
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
				self.public_list.push(ident.to_string());
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
				} else {
					self.append_error(&format!("unexpected token: '{}'", ident_str))?;
				}
			}
			_ => {
				self.append_error(&format!("unexpected token: '{}'", token))?;
			}
		}
		Ok(())
	}

	fn parse_item(&mut self, strm: &TokenStream) -> Result<(), Error> {
		Ok(())
	}

	fn build_response(&self) -> Result<TokenStream, Error> {
		println!("protected list = {:?}", self.protected_list);
		println!("public list = {:?}", self.public_list);
		println!("clone = {:?}", self.clone_list);
		println!("var list = {:?}", self.var_list);
		println!("const list = {:?}", self.const_list);
		Ok(TokenStream::new())
	}
}

pub(crate) fn do_derive_class(attr: TokenStream, item: TokenStream) -> TokenStream {
	println!("in do_derive_class");
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
