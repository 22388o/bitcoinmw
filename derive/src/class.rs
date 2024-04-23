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
use crate::utils::trim_outer;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree, TokenTree::*};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};
use std::str::from_utf8;

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
	ClassBlock,
	FnBlock,
	VarBlock,
	ConstBlock,
	Complete,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct FnInfo {
	name: String,
	signature: String,
	param_string: String,
	fn_block: String,
	views: Vec<String>,
}

impl FnInfo {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			signature: "".to_string(),
			param_string: "".to_string(),
			fn_block: "".to_string(),
			views: vec![],
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
	cur_fn: Option<FnInfo>,
	name: Option<String>,

	fn_list: Vec<FnInfo>,
	const_list: Vec<Const>,
	var_list: Vec<Var>,
}

impl State {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			stage: Stage::ClassBlock,
			span: None,
			error_list: vec![],
			cur_var: None,
			cur_const: None,
			cur_fn: None,
			name: None,
			fn_list: vec![],
			const_list: vec![],
			var_list: vec![],
		}
	}

	fn process_item(&mut self, item: TokenStream) -> Result<(), Error> {
		let mut expect_impl = true;
		let mut expect_name = false;
		let mut expect_group = false;

		for token in item {
			self.span = Some(token.span());
			debug!("item_token='{}'", token)?;
			let token_str = token.to_string();
			if expect_impl && token_str != "impl" {
				debug!("abort")?;
				self.process_abort("expected keyword impl".to_string())?;
			} else if expect_impl {
				expect_impl = false;
				expect_name = true;
			} else if expect_name {
				match token {
					Ident(name) => {
						self.name = Some(name.to_string());
						expect_name = false;
						expect_group = true;
					}
					_ => {
						self.process_abort("expected class name".to_string())?;
					}
				}
			} else if expect_group {
				match token {
					Group(group) => {
						expect_group = false;
						debug!("group='{}'", group.to_string())?;
						for _group_item in group.stream() {
							self.process_abort("impl must be empty for classes".to_string())?;
						}
					}
					_ => self.process_abort("expected 'impl <name> {}'".to_string())?,
				}
			} else {
				self.process_abort("unexpected token".to_string())?;
			}
		}
		Ok(())
	}

	fn derive(&mut self, tag: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
		self.process_item(item)?;
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
		self.stage = Stage::ClassBlock;
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
		self.stage = Stage::Complete;

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
		debug!("name={:?}", self.name)?;
		debug!("fn_list={:?}", self.fn_list)?;
		debug!("const_list={:?}", self.const_list)?;
		debug!("var_list={:?}", self.var_list)?;

		// we can unwrap here
		// if we get to this point all these are ok to unwrap
		let name = self.name.as_ref().unwrap();

		let struct_bytes = include_bytes!("../resources/class_struct_template.txt");

		let struct_bytes = from_utf8(struct_bytes)?;
		let struct_bytes = struct_bytes.replace("${NAME}", &name);

		let mut var_params = "".to_string();
		for var_param in &self.var_list {
			var_params = format!(
				"{}\n\t{}: {}",
				var_params, var_param.name, var_param.type_str
			);
		}
		let struct_bytes = struct_bytes.replace("${VAR_PARAMS}", &var_params);

		let mut const_params = "".to_string();
		for const_param in &self.const_list {
			const_params = format!(
				"{}\n\t{}: {}",
				const_params, const_param.name, const_param.type_str
			);
		}
		let struct_bytes = struct_bytes.replace("${CONST_PARAMS}", &const_params);

		// create the Const impl
		let get_bytes_template = include_bytes!("../resources/class_get_template.txt");
		let get_bytes_template = from_utf8(get_bytes_template)?;
		let mut const_impl = format!("impl {}Const {{", name);
		for const_param in &self.const_list {
			let getter = get_bytes_template.replace("${PARAM_NAME}", &const_param.name);
			let getter = getter.replace("${PARAM_TYPE}", &const_param.type_str);
			const_impl = format!("{}\n{}", const_impl, getter);
		}
		let const_impl = format!("{}}}", const_impl);

		// create the Var impl
		let get_mut_bytes_template = include_bytes!("../resources/class_get_mut_template.txt");
		let get_mut_bytes_template = from_utf8(get_mut_bytes_template)?;
		let mut var_impl = format!("impl {}Var {{", name);
		for var_param in &self.var_list {
			let mutter = get_mut_bytes_template.replace("${PARAM_NAME}", &var_param.name);
			let mutter = mutter.replace("${PARAM_TYPE}", &var_param.type_str);
			var_impl = format!("{}\n{}", var_impl, mutter);
		}

		// add builder
		for fn_info in &self.fn_list {
			if fn_info.name == "builder" {
				let param_list = trim_outer(&fn_info.param_string, "(", ")");
				let param_name = trim_outer(&param_list, "&", ")");
				debug!("param_list='{}'", param_list)?;
				var_impl = format!(
					"{}\n\tfn builder({}: &{}Const) -> Result<Self, Error> {}\n",
					var_impl, param_name, name, fn_info.fn_block
				);
			}
		}

		let var_impl = format!("{}}}", var_impl);

		// create the main struct impl
		let mut main_impl = format!("impl {} {{", name);

		for fn_info in &self.fn_list {
			if fn_info.name == "builder" {
				let impl_builder = include_bytes!("../resources/class_impl_builder_template.txt");
				let impl_builder = from_utf8(impl_builder)?;
				let impl_builder = impl_builder.replace("${NAME}", name);
				main_impl = format!("{}\n{}", main_impl, impl_builder);
			} else {
				main_impl = format!("{}\n{}{}", main_impl, fn_info.signature, fn_info.fn_block);
			}
		}

		let get_bytes_template = include_bytes!("../resources/class_impl_get_template.txt");
		let get_bytes_template = from_utf8(get_bytes_template)?;

		let get_mut_bytes_template = include_bytes!("../resources/class_impl_get_mut_template.txt");
		let get_mut_bytes_template = from_utf8(get_mut_bytes_template)?;

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

		let build_classes = format!("{}{}", struct_bytes, const_impl);
		let build_classes = format!("{}\n{}", build_classes, var_impl);
		let build_classes = format!("{}\n{}", build_classes, main_impl);

		debug!("struct_bytes={}", build_classes)?;

		Ok(())
	}

	fn process_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.stage {
			Stage::ClassBlock => self.process_class_block(token),
			Stage::FnBlock => self.process_fn_block(token),
			Stage::VarBlock => self.process_var_block(token),
			Stage::ConstBlock => self.process_const_block(token),
			Stage::Complete => err!(UnexpectedToken, "unexpected token after class definition"),
		}
	}

	fn process_fn_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("fnblock token = {:?}", token)?;
		match token {
			Group(group) => {
				if group.delimiter() == Delimiter::Brace {
					match &mut self.cur_fn {
						Some(ref mut cur_fn) => {
							cur_fn.fn_block = group.to_string();
							self.fn_list.push(cur_fn.clone());
						}
						None => {
							return err!(
								IllegalState,
								"Internal error: expected cur_fn in process_fn_block"
							)
						}
					}

					debug!("FN COMPLETE = '{:?}'", self.cur_fn)?;
					debug!("setting cur_fn to none")?;

					self.cur_fn = None;
					self.stage = Stage::ClassBlock;
				} else if group.delimiter() == Delimiter::Parenthesis {
					match &mut self.cur_fn {
						Some(ref mut cur_fn) => {
							cur_fn.param_string = group.to_string();
							cur_fn.signature = format!("{}{}", cur_fn.signature, group.to_string());
						}
						None => {
							return err!(
								IllegalState,
								"Internal error: expected cur_fn in process_fn_block"
							)
						}
					}
				}
			}
			Ident(ident) => match &mut self.cur_fn {
				Some(ref mut cur_fn) => {
					if cur_fn.name.len() == 0 {
						cur_fn.name = ident.to_string();
						cur_fn.signature = format!("fn {}", ident.to_string());
					} else {
						cur_fn.signature = format!("{}{}", cur_fn.signature, ident.to_string());
					}
				}
				None => {
					return err!(
						IllegalState,
						"internal error: expected a cur_fn in fn_block"
					);
				}
			},
			_ => match &mut self.cur_fn {
				Some(ref mut cur_fn) => {
					cur_fn.signature = format!("{}{}", cur_fn.signature, token.to_string());
				}
				None => {
					return err!(
						IllegalState,
						"internal error: expected a cur_fn in fn_block"
					);
				}
			},
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
						self.var_list.push(var.clone());
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
						self.const_list.push(c.clone());
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

	fn process_class_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("class_block_token={:?}", token)?;
		match token {
			Ident(ident) => {
				let ident_str = ident.to_string();
				if ident_str == "var" {
					if self.cur_fn.is_some() {
						self.append_error("did not expect a var after a method list")?;
					}
					self.stage = Stage::VarBlock;
				} else if ident_str == "const" {
					if self.cur_fn.is_some() {
						self.append_error("did not expect a const after a method list")?;
					}
					self.stage = Stage::ConstBlock;
				} else if ident_str == "fn" {
					if self.cur_fn.is_none() {
						debug!("creating a cur_fn")?;
						self.cur_fn = Some(FnInfo::new());
					}
					self.stage = Stage::FnBlock;
				} else {
					self.append_error(
						&format!("Parse Error: unexecpted token '{}'", ident_str)[..],
					)?;
				}
			}
			Group(group) => {
				if group.delimiter() == Delimiter::Bracket {
					if self.cur_fn.is_some() {
						self.append_error("did not expect a method list after a method list")?;
					}

					// method list here
					debug!("create a cur_fn")?;
					self.cur_fn = Some(FnInfo::new());
					self.process_method_list(group)?;
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

	fn process_method_list(&mut self, group: Group) -> Result<(), Error> {
		let mut expect_comma = false;
		for token in group.stream() {
			debug!("method_list_token={:?}", token)?;
			match token {
				Punct(p) => {
					if !expect_comma || p.to_string() != "," {
						self.append_error(&format!("Parse error, expected ','")[..])?;
					}
				}
				Ident(ident) => {
					if expect_comma {
						self.append_error(&format!("Parse error, expected ','")[..])?;
					} else {
						match &mut self.cur_fn {
							Some(ref mut cur_fn) => {
								cur_fn.views.push(ident.to_string());
							}
							None => {
								return err!(
									IllegalState,
									"internal error, expected to have a cur_fn here"
								);
							}
						}
					}
				}
				_ => {
					self.append_error(&format!("expected view name"))?;
				}
			}

			expect_comma = !expect_comma;
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
}

pub(crate) fn do_derive_class(attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut state = State::new();
	match state.derive(attr, item) {
		Ok(strm) => strm,
		Err(e) => {
			let _ = debug!("Internal Error class proc_macro generated: {}", e);
			TokenStream::new()
		}
	}
}
