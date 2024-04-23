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

use crate::types::DeriveErrorKind::*;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree, TokenTree::*};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};

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
                        err!(Log, "impossible logging error")
                }
        }};
        ($line:expr, $($values:tt)*) => {{
                if DEBUG {
                        println!($line, $($values)*);
                }
                if true {
                        Ok(())
                } else {
                        err!(Log, "impossible logging error")
                }
        }};
}

struct SpanError {
	span: Span,
	msg: String,
}

#[derive(PartialEq)]
enum Stage {
	Init,
	ClassGroup,
	ClassBlock,
	FnBlock,
	VarBlock,
	ConstBlock,
	Complete,
}

#[derive(Debug)]
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

#[derive(Debug)]
struct Const {
	name: String,
	type_str: String,
	value_str: String,
	found_colon: bool,
	found_equal: bool,
}

impl Const {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			type_str: "".to_string(),
			value_str: "".to_string(),
			found_colon: false,
			found_equal: false,
		}
	}
}

struct State {
	stage: Stage,
	ret: TokenStream,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	cur_var: Option<Var>,
	cur_const: Option<Const>,
}

impl State {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			stage: Stage::Init,
			span: None,
			error_list: vec![],
			cur_var: None,
			cur_const: None,
		}
	}

	fn derive(&mut self, tag: TokenStream) -> Result<TokenStream, Error> {
		match self.do_derive(tag) {
			Ok(_) => match self.stage {
				Stage::Complete => Ok(self.ret.clone()),
				_ => {
					err!(IllegalState, "class proc_macro unexpectedly terminated")
				}
			},
			Err(e) => {
				err!(
					IllegalState,
					"class proc_macro derive generated error: {}",
					e
				)
			}
		}
	}

	fn do_derive(&mut self, tag: TokenStream) -> Result<(), Error> {
		debug!("in do_derive_class")?;
		for token in tag {
			match self.process_token(token) {
				Ok(_) => {}
				Err(e) => {
					self.stage = Stage::Complete;
					self.process_abort(e.kind().to_string())?;
					break;
				}
			}
		}

		if self.error_list.len() > 0 {
			self.print_errors()
		} else {
			self.build()
		}
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

	fn process_abort(&self, msg: String) -> Result<(), Error> {
		match self.span {
			Some(span) => {
				let diag = Diagnostic::spanned(span.into(), Level::Error, msg.clone());
				abort!(diag, msg);
			}
			None => {}
		}
		Ok(())
	}

	fn build(&mut self) -> Result<(), Error> {
		Ok(())
	}

	fn process_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.stage {
			Stage::Init => self.process_init(token),
			Stage::ClassGroup => self.process_class_group(token),
			Stage::ClassBlock => self.process_class_block(token),
			Stage::FnBlock => self.process_fn_block(token),
			Stage::VarBlock => self.process_var_block(token),
			Stage::ConstBlock => self.process_const_block(token),
			Stage::Complete => err!(UnexpectedToken, "unexpected token after class definition"),
		}
	}

	fn process_fn_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("fblock token = {}", token)?;
		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					self.stage = Stage::ClassBlock;
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn process_var_block(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		debug!("process_var token = {:?}", token)?;
		if token_str == ";" {
			match &self.cur_var {
				None => {
					self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
				}
				Some(var) => {
					if !var.found_colon || var.type_str.len() == 0 {
						self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
					} else {
						debug!("COMPLETE var = {:?}", self.cur_var)?;
					}
					self.cur_var = None;
				}
			}
			self.stage = Stage::ClassBlock;
		} else {
			match &mut self.cur_var {
				Some(var) => {
					// if this is not true, we know we already have an error
					if var.name.len() > 0 {
						if !var.found_colon {
							if token_str != ":" {
								self.append_error(&format!("unexpected token: {}", token_str)[..])?;
							} else {
								var.found_colon = true;
							}
						} else {
							if token_str == ":" {
								self.append_error("unexpected token: ':'")?;
							} else {
								debug!("type_str += '{:?}'", token)?;
								// append the rest
								var.type_str = format!("{}{}", var.type_str, token_str);
							}
						}
					}
				}
				None => match token {
					Ident(name) => {
						let mut var = Var::new();
						var.name = name.to_string();
						self.cur_var = Some(var);
					}
					_ => {
						self.append_error(&format!("unexpected token: {}", token_str)[..])?;
					}
				},
			}
		}
		Ok(())
	}

	fn process_const_block(&mut self, token: TokenTree) -> Result<(), Error> {
		let token_str = token.to_string();
		debug!("process const block token = {:?}", token)?;
		if token_str == ";" {
			self.stage = Stage::ClassBlock;
			match &self.cur_const {
				None => {
					self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
				}
				Some(c) => {
					if !c.found_colon
						|| !c.found_equal || c.type_str.len() == 0
						|| c.value_str.len() == 0
					{
						self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
					} else {
						debug!("COMPLETE const = {:?}", self.cur_const)?;
					}
					self.cur_const = None;
				}
			}
		} else {
			match &mut self.cur_const {
				Some(c) => {
					if !c.found_colon {
						if token_str != ":" {
							self.append_error(&format!("unexpected token: {}", token_str)[..])?;
						} else {
							c.found_colon = true;
						}
					} else {
						if token_str == ":" {
							self.append_error("unexpected token: ':'")?;
						} else {
							debug!("additional const token += '{:?}'", token)?;
							if !c.found_equal {
								// append to type_str
								if token_str != "=" {
									c.type_str = format!("{}{}", c.type_str, token_str);
								} else {
									c.found_equal = true;
								}
							} else {
								// append to value str
								c.value_str = format!("{}{}", c.value_str, token_str);
							}
						}
					}
				}
				None => match token {
					Ident(name) => {
						let mut c = Const::new();
						c.name = name.to_string();
						self.cur_const = Some(c);
					}
					_ => self.append_error(&format!("unexpected token: {}", token_str)[..])?,
				},
			}
		}
		Ok(())
	}

	fn process_class_group(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("ClassGroup token = {:?}", token)?;
		match token {
			Group(group) => {
				if group.delimiter() != Delimiter::Brace {
					self.append_error("unexpected token error: expected '{'")?;
				}
				self.stage = Stage::ClassBlock;
				for token in group.stream() {
					self.process_token(token)?;
				}
			}
			_ => {
				return err!(UnexpectedToken, "expected '{'");
			}
		}
		self.stage = Stage::Complete;
		Ok(())
	}

	fn process_class_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("class_block_token={:?}", token)?;
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "var" {
					self.stage = Stage::VarBlock;
				} else if ident_str == "const" {
					self.stage = Stage::ConstBlock;
				} else if ident_str == "fn" {
					self.stage = Stage::FnBlock;
				} else {
					self.append_error(
						&format!("Parse Error: unexecpted token '{}'", ident_str)[..],
					)?;
				}
			}
			Group(group) => {
				if group.delimiter() == Delimiter::Bracket {
					// method list here
				} else {
					self.append_error(&format!(
						"Parse Error: unexpected token: '{:?}'",
						group.delimiter()
					))?;
				}
			}
			Punct(p) => self.append_error(&format!("Parse Error: unexecpted token '{}'", p)[..])?,

			Literal(l) => {
				self.append_error(&format!("Parse Error: unexecpted token '{}'", l)[..])?
			}
		}
		Ok(())
	}

	fn process_init(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("init token={:?}", token)?;
		if token.to_string() == "class" {
			self.append_error("Reserved Word Error: the name 'class' is reserved.")?;
		}
		self.stage = Stage::ClassGroup;
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
}

pub(crate) fn do_derive_class(attr: TokenStream, _item: TokenStream) -> TokenStream {
	let mut state = State::new();
	match state.derive(attr) {
		Ok(strm) => strm,
		Err(e) => {
			let _ = debug!("Internal Error class proc_macro generated: {}", e);
			TokenStream::new()
		}
	}
}
