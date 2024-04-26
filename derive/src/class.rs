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

use crate::derive_configurable;
use crate::types::DeriveErrorKind::*;
use crate::utils::trim_outer;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use bmw_deps::convert_case::{Case, Casing};
use bmw_deps::substring::Substring;
use proc_macro::{Delimiter, Group, Spacing, Span, TokenStream, TokenTree, TokenTree::*};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};
use std::collections::{HashMap, HashSet};
use std::str::from_utf8;

const DEBUG: bool = false;

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
	PublicBlock,
	ProtectedBlock,
	CloneBlock,
	CommentBlock,
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
	comments: Vec<Group>,
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

#[derive(Debug, Clone)]
struct FnInfo {
	name: String,
	signature: String,
	param_string: String,
	fn_block: String,
	views: Vec<String>,
	comments: Vec<Group>,
}

impl FnInfo {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			signature: "".to_string(),
			param_string: "".to_string(),
			fn_block: "".to_string(),
			views: vec![],
			comments: vec![],
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
	generics: Option<String>,
	generics2: Option<String>,
	where_clause: Option<String>,

	fn_list: Vec<FnInfo>,
	const_list: Vec<Const>,
	var_list: Vec<Var>,
	public_set: HashSet<String>,
	protected_set: HashSet<String>,

	accumulated_comments: Vec<Group>,

	trait_set: HashSet<String>,
	trait_comments: HashMap<String, Vec<Group>>,
	prev_is_joint: bool,
	clone_set: HashSet<String>,
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
			generics: None,
			generics2: None,
			where_clause: None,
			fn_list: vec![],
			const_list: vec![],
			var_list: vec![],
			clone_set: HashSet::new(),
			public_set: HashSet::new(),
			protected_set: HashSet::new(),
			accumulated_comments: vec![],
			trait_set: HashSet::new(),
			trait_comments: HashMap::new(),
			prev_is_joint: false,
		}
	}

	fn process_item(&mut self, item: TokenStream) -> Result<(), Error> {
		let mut expect_impl = true;
		let mut expect_name = false;
		let mut in_generics = false;
		let mut expect_group = false;
		let mut term_where = false;
		let mut in_where = false;

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
								/*
								println!(
									"final=gen={:?},gen2={:?},where={:?},post={:?},pre={:?}",
									self.generics,
									self.generics2,
									self.where_clause,
									&self.get_post_name_clause()?,
									&self.get_pre_name_clause()?
								);
																*/
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
							debug!("group='{}'", group.to_string())?;
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

	fn build(&mut self) -> Result<(), Error> {
		/*
		let r: u64 = bmw_deps::rand::random();
		let r = r % 1_000;
		std::thread::sleep(std::time::Duration::from_millis(r));
			*/

		debug!("name={:?}", self.name)?;
		debug!("fn_list={:?}", self.fn_list)?;
		debug!("const_list={:?}", self.const_list)?;
		debug!("var_list={:?}", self.var_list)?;

		// we can unwrap here
		// if we get to this point all these are ok to unwrap
		let name = self.name.as_ref().unwrap();

		let conf_bytes = include_bytes!("../resources/class_struct_const_template.txt");
		let conf_bytes = from_utf8(conf_bytes)?;
		let conf_bytes = conf_bytes.replace(
			"${CLONE}",
			if self.clone_set.len() > 0 {
				"#[derive(Clone)]"
			} else {
				""
			},
		);
		let conf_bytes = conf_bytes.replace("${NAME}", &name);

		let struct_bytes = include_bytes!("../resources/class_struct_template.txt");

		let struct_bytes = from_utf8(struct_bytes)?;
		let struct_bytes = struct_bytes.replace(
			"${CLONE}",
			if self.clone_set.len() > 0 {
				"#[derive(Clone)]"
			} else {
				""
			},
		);
		let struct_bytes = struct_bytes.replace("${NAME}", &name);
		let struct_bytes = struct_bytes.replace("${GENERICS}", &self.get_post_name_clause()?);
		let struct_bytes = struct_bytes.replace("${GENERICS_PRE}", &self.get_pre_name_clause()?);

		let mut var_params = "".to_string();
		for var_param in &self.var_list {
			var_params = format!(
				"{}\n\t{}: {},",
				var_params, var_param.name, var_param.type_str
			);
		}
		let struct_bytes = struct_bytes.replace("${VAR_PARAMS}", &var_params);

		let mut const_params = "".to_string();
		// export the config options here
		let mut conf_default = format!("#[doc(hidden)]\npub use {}ConstOptions::*;", name);
		conf_default = format!("{}\nimpl Default for {}Const {{", conf_default, name);
		conf_default = format!("{}\n\tfn default() -> Self {{ Self {{", conf_default);
		for const_param in &self.const_list {
			const_params = format!(
				"{}\n\t{}: {},",
				const_params, const_param.name, const_param.type_str
			);
			conf_default = format!(
				"{}\n{}: {},",
				conf_default, const_param.name, const_param.value_str
			);
		}

		conf_default = format!("{}\n\t}}\n}}\n}}", conf_default);
		debug!("conf_default = '{}'", conf_default)?;
		let struct_bytes = struct_bytes.replace("${CONST_PARAMS}", &const_params);
		let conf_bytes = conf_bytes.replace("${CONST_PARAMS}", &const_params);

		let options = derive_configurable(map_err!(conf_bytes.parse::<TokenStream>(), Parse)?);

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
		let mut var_impl = format!(
			"impl {} {}Var {} {{",
			&self.get_pre_name_clause()?,
			name,
			&self.get_post_name_clause()?
		);
		for var_param in &self.var_list {
			let mutter = get_mut_bytes_template.replace("${PARAM_NAME}", &var_param.name);
			let mutter = mutter.replace("${PARAM_TYPE}", &var_param.type_str);
			var_impl = format!("{}\n{}", var_impl, mutter);
		}

		let mut macro_comments = "".to_string();
		let mut use_comment = None;
		let mut test_init = "".to_string();

		// add builder
		for fn_info in &self.fn_list {
			if fn_info.name == "builder" {
				for comment in &fn_info.comments {
					for token in comment.stream() {
						match token {
							Literal(l) => {
								let l = trim_outer(&l.to_string(), "\"", "\"");
								let c = l.trim();
								if c.find("@module ") == Some(0) {
									if c.len() > 8 {
										let usec = c.substring(8, c.len());
										use_comment = Some(usec.to_string());
									}
								} else if c.find("@add_test_init ") == Some(0) {
									if c.len() > 15 {
										let ncomment = c.substring(15, c.len()).to_string();
										test_init =
											format!("{}\n#[doc=\"\t{}\"]", test_init, ncomment);
									}
								} else {
									macro_comments =
										format!("{}\n#[doc=\"{}\"]", macro_comments, c.to_string());
								}
							}
							_ => {}
						}
					}
				}

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
		let mut main_impl = format!(
			"impl {} {} {} {{",
			&self.get_pre_name_clause()?,
			name,
			&self.get_post_name_clause()?
		);

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

		// add traits

		let mut map: HashMap<String, Vec<FnInfo>> = HashMap::new();
		for fn_info in &self.fn_list {
			for view in &fn_info.views {
				match map.get_mut(view) {
					Some(info_list) => {
						info_list.push(fn_info.clone());
					}
					None => {
						map.insert(view.clone(), vec![fn_info.clone()]);
					}
				}
			}
		}

		let mut trait_text = format!("");
		let mut trait_impl = format!("");
		let mut trait_impl_mut = format!("");
		let mut macro_text = format!("");
		let visibility = if self.public_set.len() > 0 {
			"pub"
		} else if self.protected_set.len() > 0 {
			"pub (crate)"
		} else {
			""
		};
		let mut builder_text = format!(
			"#[doc=\"Builder Struct for the `{}` class.\"]{} struct {}Builder {{}}\nimpl {}Builder {{",
			name, visibility, name, name
		);
		let macro_bytes_raw = include_bytes!("../resources/class_macro_template.txt");
		let macro_bytes_raw = from_utf8(macro_bytes_raw)?;

		let builder_bytes_raw = include_bytes!("../resources/class_builder_template.txt");
		let builder_bytes_raw = from_utf8(builder_bytes_raw)?;
		let builder_bytes_raw =
			builder_bytes_raw.replace("${GENERIC_PRE}", &self.get_pre_name_clause()?);
		let builder_bytes_raw =
			builder_bytes_raw.replace("${WHERE_CLAUSE}", &self.get_where_clause()?);

		macro_comments = format!("{}\n#[doc=\"# Input Parameters\"]", macro_comments);
		macro_comments = format!(
			"{}\n#[doc=\"| Parameter | Comment | Default Value|\"]",
			macro_comments
		);
		macro_comments = format!("{}\n#[doc=\"|-------|-----|----|\"]", macro_comments);

		for constant in &self.const_list {
			let mut first = true;
			let name = &constant.name.to_case(Case::Pascal);
			let type_str = &constant.type_str;
			for comment in &constant.comments {
				for token in comment.stream() {
					match token {
						Literal(l) => {
							let l = trim_outer(&l.to_string(), "\"", "\"");
							if first {
								macro_comments = format!(
									"{}\n#[doc=\"| `{}`([`{}`]) | {}",
									macro_comments,
									name,
									type_str,
									l.to_string()
								);
							} else {
								macro_comments = format!("{} {}", macro_comments, l.to_string());
							}
							first = false;
						}
						_ => {}
					}
				}
			}
			if !first {
				let value = constant
					.value_str
					.replace("\"", "\\\"")
					.replace(".to_string()", "");
				macro_comments = format!("{}| {} |\"]", macro_comments, value);
			} else {
				macro_comments = format!(
					"{}\n#[doc=\"| `{}`([`{}`]) | {}",
					macro_comments, name, type_str, " - "
				);
				let value = constant
					.value_str
					.replace("\"", "\\\"")
					.replace(".to_string()", "");
				macro_comments = format!("{}| {} |\"]", macro_comments, value);
			}
		}

		macro_comments = format!("{}\n#[doc=\"# Return\"]", macro_comments);
		macro_comments = format!("{}\n#[doc=\"REPLACE_RETURN\"]", macro_comments);
		macro_comments = format!("{}\n#[doc=\"# Errors \"]", macro_comments);
		macro_comments = format!("{}\n#[doc=\"* [`bmw_base::BaseErrorKind::Builder`] - if the builder function returns an error,", macro_comments);
		macro_comments = format!(
			"{} it will be wrapped in the builder error with the details of the original error preserved.\"]",
			macro_comments
		);
		macro_comments = format!("{}\n#[doc=\"# Also See\"]", macro_comments);
		macro_comments = format!("{}\n#[doc=\" * [`REPLACE_TRAIT_NAME`] \"]", macro_comments);
		macro_comments = format!("{}\n#[doc=\" * [`bmw_base::ErrorKind`] \"]", macro_comments);
		macro_comments = format!(
			"{}\n#[doc=\" * [`bmw_base::BaseErrorKind::Builder`] \"]",
			macro_comments
		);

		for (view, list) in map {
			let snake_view = view.clone();
			let view = view.to_case(Case::Pascal);
			let mut macro_post = format!("#[doc=\"# Example\"]\n#[doc=\"\"]");
			let replace_param = "REPLACE_PARAM";

			if self.public_set.get(&format!("{}", snake_view)).is_some()
				|| self
					.public_set
					.get(&format!("{}_send", snake_view))
					.is_some() || self
				.public_set
				.get(&format!("{}_sync", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_sync_box", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_send_box", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_box", snake_view))
				.is_some()
			{
				macro_post = format!("{}\n#[doc=\"```\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"use bmw_base::*;\"]", macro_post);
				let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

				macro_post = format!(
					"{}\n#[doc=\"use {}::REPLACE_PARAM;\"]",
					macro_post, crate_name
				);

				match use_comment {
					Some(ref comment) => {
						macro_post = format!(
							"{}\n#[doc=\"use {}::{}Builder;\"]",
							macro_post, comment, name
						);
						macro_post = format!(
							"{}\n#[doc=\"use {}::{}ConstOptions::*;\"]",
							macro_post, comment, name
						);
					}
					None => {
						macro_post = format!(
							"{}\n#[doc=\"use {}::{}Builder;\"]",
							macro_post, crate_name, name
						);
						macro_post = format!(
							"{}\n#[doc=\"use {}::{}ConstOptions::*;\"]",
							macro_post, crate_name, name
						);
					}
				}
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!(
					"{}\n#[doc=\"fn main() -> Result<(), Error> {{\"]",
					macro_post
				);

				let mut param_list = "".to_string();
				let mut first = true;
				for item in &self.const_list {
					if item.type_str.find("Vec<") == Some(0) {
						// for now don't display these because safe defaults
						// are not known
					} else {
						if first {
							param_list = format!(
								"{}\n\t\t{}({})",
								param_list,
								item.name.to_case(Case::Pascal),
								item.value_str
									.replace("\"", "\\\"")
									.replace(".to_string()", "")
							);
						} else {
							param_list = format!(
								"{},\n\t\t{}({})",
								param_list,
								item.name.to_case(Case::Pascal),
								item.value_str
									.replace("\"", "\\\"")
									.replace(".to_string()", "")
							);
						}
					}
					first = false;
				}
				macro_post = format!(
					"{}#[doc=\"    // instantiate {}! with parameters explicitly specified.\"]\n",
					macro_post, replace_param
				);
				macro_post = format!(
					"{}\n#[doc=\"    let mut x = REPLACE_BUILDER_OR_MACRO{}REPLACE_MACRO_BANG{}\n\tREPLACE_MACRO_END?;\"]",
					macro_post, replace_param, param_list
				);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!("{}\n{}", macro_post, test_init);
				macro_post = format!("{}\n#[doc=\"    // use x...\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!(
                                        "{}#[doc=\"    // instantiate {}! with no parameters explicitly specified,\"]\n",
                                        macro_post,replace_param
                                );
				macro_post = format!("{}#[doc=\"    // defaults used.\"]\n", macro_post);
				macro_post = format!(
					"{}\n#[doc=\"    let mut x = REPLACE_BUILDER_OR_MACRO{}REPLACE_MACRO_BANGREPLACE_MACRO_END?;\"]",
					macro_post, replace_param
				);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!("{}\n{}", macro_post, test_init);
				macro_post = format!("{}\n#[doc=\"    // use x...\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"    Ok(())\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"}}\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);
				macro_post = format!("{}\n#[doc=\"\"]", macro_post);

				macro_post = format!("{}\n#[doc=\"```\"]", macro_post);
			}

			let macro_pre = format!(
				"#[doc=\"Constructs an implementation of the [`{}`] trait using the provided input parameters.<br/>\"]\n{}{}",
				view, macro_comments, macro_post
			);
			let macro_pre = macro_pre.replace("REPLACE_TRAIT_NAME", &view);
			let comments = &macro_pre;

			let mut trait_visibility = "";
			if self.protected_set.get(&format!("{}", snake_view)).is_some()
				|| self
					.protected_set
					.get(&format!("{}_send_box", snake_view))
					.is_some() || self
				.protected_set
				.get(&format!("{}_sync_box", snake_view))
				.is_some() || self
				.protected_set
				.get(&format!("{}_box", snake_view))
				.is_some() || self
				.protected_set
				.get(&format!("{}_send", snake_view))
				.is_some() || self
				.protected_set
				.get(&format!("{}_sync", snake_view))
				.is_some()
			{
				trait_visibility = "pub (crate)";
			} else if self.public_set.get(&format!("{}", snake_view)).is_some()
				|| self
					.public_set
					.get(&format!("{}_send_box", snake_view))
					.is_some() || self
				.public_set
				.get(&format!("{}_sync_box", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_box", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_send", snake_view))
				.is_some() || self
				.public_set
				.get(&format!("{}_sync", snake_view))
				.is_some()
			{
				trait_visibility = "pub";
			}

			match self.trait_comments.get(&snake_view) {
				Some(comments) => {
					for comment in comments {
						for token in comment.stream() {
							match token {
								Literal(l) => {
									trait_text =
										format!("{}\n#[doc={}]", trait_text, l.to_string());
								}
								_ => {}
							}
						}
					}
				}
				None => {}
			}
			trait_text = format!(
				"{}\n{} trait {} {} {} {{\n",
				trait_text,
				trait_visibility,
				view,
				&self.get_post_name_clause()?,
				if self.clone_set.contains(&snake_view) {
					": bmw_deps::dyn_clone::DynClone"
				} else {
					""
				}
			);
			trait_impl = format!(
				"{}\nimpl {} {} {} for {} {} {{\n",
				trait_impl,
				&self.get_pre_name_clause()?,
				view,
				&self.get_pre_name_clause()?,
				name,
				&self.get_post_name_clause()?,
			);

			if !self.clone_set.contains(&snake_view) {
				trait_impl_mut = format!(
					"{}\nimpl {} {} {} for &mut {} {} {{\n",
					trait_impl_mut,
					&self.get_pre_name_clause()?,
					view,
					&self.get_pre_name_clause()?,
					name,
					&self.get_post_name_clause()?,
				);
			}

			// add non-send non-sync builder fns
			let builder_bytes = builder_bytes_raw.replace("${NAME}", name);
			let builder_bytes = builder_bytes.replace(
				"${IMPL_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					snake_view.clone(),
					comments,
					false,
					false,
					false,
					name,
					self.public_set.get(&snake_view).is_some(),
				)?,
			);
			let builder_bytes = builder_bytes.replace(
				"${BOX_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					format!("{}_box", snake_view.clone()),
					comments,
					false,
					false,
					true,
					name,
					self.public_set
						.get(&format!("{}_box", snake_view))
						.is_some(),
				)?,
			);
			let builder_bytes = builder_bytes.replace("${VIEW_SNAKE_CASE}", &snake_view);
			let builder_bytes = builder_bytes.replace(
				"${TRAIT_LIST}",
				&format!("{} {}", view, &self.get_pre_name_clause()?),
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_IMPL}",
				if self.public_set.get(&snake_view).is_some() {
					"pub"
				} else if self.protected_set.get(&snake_view).is_some() {
					"pub(crate)"
				} else {
					""
				},
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_BOX}",
				if self
					.public_set
					.get(&format!("{}_box", snake_view))
					.is_some()
				{
					"pub"
				} else if self
					.protected_set
					.get(&format!("{}_box", snake_view))
					.is_some()
				{
					"pub(crate)"
				} else {
					""
				},
			);

			builder_text = format!("{}{}", builder_text, builder_bytes);

			// add send
			let builder_bytes = builder_bytes_raw.replace("${NAME}", name);
			let builder_bytes = builder_bytes.replace(
				"${IMPL_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					format!("{}_send", snake_view.clone()),
					comments,
					true,
					false,
					false,
					name,
					self.public_set
						.get(&format!("{}_send", snake_view))
						.is_some(),
				)?,
			);
			let builder_bytes = builder_bytes.replace(
				"${BOX_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					format!("{}_send_box", snake_view.clone()),
					comments,
					true,
					false,
					true,
					name,
					self.public_set
						.get(&format!("{}_send_box", snake_view))
						.is_some(),
				)?,
			);
			let builder_bytes =
				builder_bytes.replace("${VIEW_SNAKE_CASE}", &format!("{}_send", snake_view));
			let builder_bytes = builder_bytes.replace(
				"${TRAIT_LIST}",
				&format!("{} {} + Send", view, &self.get_pre_name_clause()?,),
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_IMPL}",
				if self
					.public_set
					.get(&format!("{}_send", snake_view))
					.is_some()
				{
					"pub"
				} else if self
					.protected_set
					.get(&format!("{}_send", snake_view))
					.is_some()
				{
					"pub(crate)"
				} else {
					""
				},
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_BOX}",
				if self
					.public_set
					.get(&format!("{}_send_box", snake_view))
					.is_some()
				{
					"pub"
				} else if self
					.protected_set
					.get(&format!("{}_send_box", snake_view))
					.is_some()
				{
					"pub(crate)"
				} else {
					""
				},
			);

			builder_text = format!("{}{}", builder_text, builder_bytes);

			// add sync
			let builder_bytes = builder_bytes_raw.replace("${NAME}", name);
			let builder_bytes = builder_bytes.replace(
				"${IMPL_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					format!("{}_sync", snake_view.clone()),
					comments,
					false,
					true,
					false,
					name,
					self.public_set
						.get(&format!("{}_sync", snake_view))
						.is_some(),
				)?,
			);
			let builder_bytes = builder_bytes.replace(
				"${BOX_COMMENTS}",
				&self.build_builder_comments(
					view.clone(),
					format!("{}_sync_box", snake_view.clone()),
					comments,
					false,
					true,
					true,
					name,
					self.public_set
						.get(&format!("{}_sync_box", snake_view))
						.is_some(),
				)?,
			);

			let comments = comments.replace("REPLACE_BUILDER_OR_MACRO", "");
			let comments = comments.replace("REPLACE_MACRO_BANG", "!(");
			let comments = comments.replace("REPLACE_MACRO_END", ")");

			let builder_bytes =
				builder_bytes.replace("${VIEW_SNAKE_CASE}", &format!("{}_sync", snake_view));
			let builder_bytes = builder_bytes.replace(
				"${TRAIT_LIST}",
				&format!("{} {} + Send + Sync", view, &self.get_pre_name_clause()?),
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_IMPL}",
				if self
					.public_set
					.get(&format!("{}_sync", snake_view))
					.is_some()
				{
					"pub"
				} else if self
					.protected_set
					.get(&format!("{}_sync", snake_view))
					.is_some()
				{
					"pub(crate)"
				} else {
					""
				},
			);
			let builder_bytes = builder_bytes.replace(
				"${VISIBILITY_BOX}",
				if self
					.public_set
					.get(&format!("{}_sync_box", snake_view))
					.is_some()
				{
					"pub"
				} else if self
					.protected_set
					.get(&format!("{}_sync_box", snake_view))
					.is_some()
				{
					"pub(crate)"
				} else {
					""
				},
			);
			builder_text = format!("{}{}", builder_text, builder_bytes);

			// add non-send non sync macros
			let mut macro_bytes = macro_bytes_raw.replace("${NAME}", name);
			let impl_comments = comments.replace("REPLACE_PARAM", &snake_view);
			let ret = format!("[`Result`] <impl [`{}`], [`bmw_base::Error`]>", view);
			let impl_comments = impl_comments.replace("REPLACE_RETURN", &ret);
			let snake_view_box = format!("{}_box", snake_view);
			let box_comments = comments.replace("REPLACE_PARAM", &snake_view_box);

			let ret = format!(
				"[`Result`] <[`Box`] <dyn [`{}`]>, [`bmw_base::Error`]>",
				view
			);
			let box_comments = box_comments.replace("REPLACE_RETURN", &ret);

			if self.public_set.get(&format!("{}", snake_view)).is_some() {
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", &impl_comments);
			} else {
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", "");
			}
			if self
				.public_set
				.get(&format!("{}_box", snake_view))
				.is_some()
			{
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", &box_comments);
			} else {
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", "");
			}
			let macro_bytes = macro_bytes.replace("${VIEW_SNAKE_CASE}", &snake_view);
			let macro_bytes = macro_bytes.replace(
				"${BOX_PUBLIC}",
				&if self
					.public_set
					.get(&format!("{}_box", snake_view))
					.is_some()
				{
					"#[macro_export]".to_string()
				} else {
					"".to_string()
				},
			);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PUBLIC}",
				if self.public_set.get(&snake_view).is_some() {
					"#[macro_export]"
				} else {
					""
				},
			);
			let s = format!("pub(crate) use {};", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PROTECTED}",
				if self.protected_set.get(&snake_view).is_some() {
					&s
				} else {
					""
				},
			);
			let s = format!("pub(crate) use {}_box;", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${BOX_PROTECTED}",
				if self
					.protected_set
					.get(&format!("{}_box", snake_view))
					.is_some()
				{
					&s
				} else {
					""
				},
			);

			macro_text = format!("{}\n{}", macro_text, macro_bytes);

			// add send
			let mut macro_bytes = macro_bytes_raw.replace("${NAME}", name);
			let snake_view_send = format!("{}_send", snake_view);
			let impl_comments = comments.replace("REPLACE_PARAM", &snake_view_send);
			let snake_view_send_box = format!("{}_send_box", snake_view);
			let box_comments = comments.replace("REPLACE_PARAM", &snake_view_send_box);

			let ret = format!(
				"[`Result`] <impl [`{}`] + [`Send`], [`bmw_base::Error`]>",
				view
			);
			let impl_comments = impl_comments.replace("REPLACE_RETURN", &ret);
			let ret = format!(
				"[`Result`] <[`Box`] <dyn [`{}`] + [`Send`]>, [`bmw_base::Error`]>",
				view
			);
			let box_comments = box_comments.replace("REPLACE_RETURN", &ret);

			if self
				.public_set
				.get(&format!("{}_send", snake_view))
				.is_some()
			{
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", &impl_comments);
			} else {
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", "");
			}
			if self
				.public_set
				.get(&format!("{}_send_box", snake_view))
				.is_some()
			{
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", &box_comments);
			} else {
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", "");
			}

			let macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", &impl_comments);
			let macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", &box_comments);

			let macro_bytes =
				macro_bytes.replace("${VIEW_SNAKE_CASE}", &format!("{}_send", snake_view));
			let macro_bytes = macro_bytes.replace(
				"${BOX_PUBLIC}",
				if self
					.public_set
					.get(&format!("{}_send_box", snake_view))
					.is_some()
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PUBLIC}",
				if self
					.public_set
					.get(&format!("{}_send", snake_view))
					.is_some()
				{
					"#[macro_export]"
				} else {
					""
				},
			);

			let s = format!("pub(crate) use {}_send;", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PROTECTED}",
				if self
					.protected_set
					.get(&format!("{}_send", snake_view))
					.is_some()
				{
					&s
				} else {
					""
				},
			);
			let s = format!("pub(crate) use {}_send_box;", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${BOX_PROTECTED}",
				if self
					.protected_set
					.get(&format!("{}_send_box", snake_view))
					.is_some()
				{
					&s
				} else {
					""
				},
			);

			macro_text = format!("{}\n{}", macro_text, macro_bytes);

			// add sync
			let mut macro_bytes = macro_bytes_raw.replace("${NAME}", name);

			let snake_view_sync = format!("{}_sync", snake_view);
			let impl_comments = comments.replace("REPLACE_PARAM", &snake_view_sync);
			let snake_view_sync_box = format!("{}_sync_box", snake_view);
			let box_comments = comments.replace("REPLACE_PARAM", &snake_view_sync_box);

			let ret = format!(
				"[`Result`] <impl [`{}`] + [`Send`] + [`Sync`], [`bmw_base::Error`]>",
				view
			);
			let impl_comments = impl_comments.replace("REPLACE_RETURN", &ret);
			let ret = format!(
				"[`Result`] <[`Box`] <dyn [`{}`] + [`Send`] + [`Sync`]>, [`bmw_base::Error`]>",
				view
			);
			let box_comments = box_comments.replace("REPLACE_RETURN", &ret);

			if self
				.public_set
				.get(&format!("{}_sync", snake_view))
				.is_some()
			{
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", &impl_comments);
			} else {
				macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", "");
			}
			if self
				.public_set
				.get(&format!("{}_sync_box", snake_view))
				.is_some()
			{
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", &box_comments);
			} else {
				macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", "");
			}

			let macro_bytes = macro_bytes.replace("${IMPL_COMMENTS}", &impl_comments);
			let macro_bytes = macro_bytes.replace("${BOX_COMMENTS}", &box_comments);

			let macro_bytes =
				macro_bytes.replace("${VIEW_SNAKE_CASE}", &format!("{}_sync", snake_view));
			let macro_bytes = macro_bytes.replace(
				"${BOX_PUBLIC}",
				if self
					.public_set
					.get(&format!("{}_sync_box", snake_view))
					.is_some()
				{
					"#[macro_export]"
				} else {
					""
				},
			);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PUBLIC}",
				if self
					.public_set
					.get(&format!("{}_sync", snake_view))
					.is_some()
				{
					"#[macro_export]"
				} else {
					""
				},
			);

			let s = format!("pub(crate) use {}_sync;", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${IMPL_PROTECTED}",
				if self
					.protected_set
					.get(&format!("{}_sync", snake_view))
					.is_some()
				{
					&s
				} else {
					""
				},
			);
			let s = format!("pub(crate) use {}_sync_box;", snake_view);
			let macro_bytes = macro_bytes.replace(
				"${BOX_PROTECTED}",
				if self
					.protected_set
					.get(&format!("{}_sync_box", snake_view))
					.is_some()
				{
					&s
				} else {
					""
				},
			);

			macro_text = format!("{}\n{}", macro_text, macro_bytes);

			for fn_info in list {
				trait_text = format!(
					"{}\n{}",
					trait_text,
					self.build_comments(
						fn_info.comments,
						fn_info.param_string.clone(),
						view.clone(),
						fn_info.signature.clone(),
					)?
				);
				trait_text = format!("{}\n{};", trait_text, fn_info.signature);
				let param_string = self.convert_param_string(&fn_info.param_string)?;
				trait_impl = format!(
					"{}\n{} {{ {}::{}{} }}",
					trait_impl, fn_info.signature, name, fn_info.name, param_string.0
				);
				if !self.clone_set.contains(&snake_view) {
					trait_impl_mut = format!(
						"{}\n{} {{ {}::{}{} }}",
						trait_impl_mut, fn_info.signature, name, fn_info.name, param_string.0
					);
				}
			}

			trait_text = format!("{}}}\n", trait_text);
			if self.clone_set.contains(&snake_view) {
				trait_text = format!(
					"{}bmw_deps::dyn_clone::clone_trait_object!({});\n",
					trait_text, view
				);
			}
			trait_impl = format!("{}}}\n", trait_impl);
			if !self.clone_set.contains(&snake_view) {
				trait_impl_mut = format!("{}}}\n", trait_impl_mut);
			}
		}
		builder_text = format!("{}}}\n", builder_text);

		let build_class = format!("{}{}", struct_bytes, const_impl);
		let build_class = format!("{}\n{}", build_class, conf_bytes);
		let build_class = format!("{}\n{}", build_class, conf_default);
		let build_class = format!("{}\n{}", build_class, options);
		let build_class = format!("{}\n{}", build_class, var_impl);
		let build_class = format!("{}\n{}", build_class, main_impl);
		let build_class = format!("{}\n{}", build_class, trait_text);
		let build_class = format!("{}\n{}", build_class, trait_impl);
		let build_class = format!("{}\n{}", build_class, trait_impl_mut);
		let build_class = format!("{}\n{}", build_class, macro_text);
		let build_class = format!("{}\n{}", build_class, builder_text);

		self.ret.extend(build_class.parse::<TokenStream>());
		//println!("ret='{}'", self.ret);

		Ok(())
	}

	fn build_builder_comments(
		&self,
		view: String,
		snake_view: String,
		comments: &str,
		is_send: bool,
		is_sync: bool,
		is_box: bool,
		name: &String,
		display: bool,
	) -> Result<String, Error> {
		if display {
			let ret = format!("{}", comments);
			let ret = ret.replace(
				"REPLACE_BUILDER_OR_MACRO",
				&format!("{}Builder::build_", name),
			);
			let ret = ret.replace("REPLACE_MACRO_BANG", "(vec![");
			let ret = ret.replace("REPLACE_MACRO_END", "])");
			let ret = ret.replace("REPLACE_PARAM", &snake_view);
			let replace_return = if is_sync && is_box {
				// _sync_box
				format!(
					"[`Result`]<[`Box`]<dyn [`{}`] + [`Send`] + [`Sync`]>, [`Error`]>",
					view
				)
			} else if is_send && is_box {
				// _send_box
				format!(
					"[`Result`]<[`Box`]<dyn [`{}`] + [`Send`]>, [`Error`]>",
					view
				)
			} else if is_sync {
				// _sync
				format!(
					"[`Result`]<impl [`{}`] + [`Send`] + [`Sync`], [`Error`]>",
					view
				)
			} else if is_send {
				// _send
				format!("[`Result`]<impl [`{}`] + [`Send`], [`Error`]>", view)
			} else if is_box {
				// _box
				format!("[`Result`]<[`Box`]<dyn [`{}`]>, [`Error`]>", view)
			} else {
				// impl
				format!("[`Result`]<impl [`{}`], [`Error`]>", view)
			};
			let ret = ret.replace("REPLACE_RETURN", &replace_return);
			Ok(ret)
		} else {
			Ok("".to_string())
		}
	}

	fn build_comments(
		&self,
		comments: Vec<Group>,
		param_string: String,
		trait_name: String,
		signature: String,
	) -> Result<String, Error> {
		// for now, just check if result is in the return type. Could be improved.
		let return_errors = signature.find("Result").is_some();

		let conv_param_string = self.convert_param_string(&param_string)?;
		let stream = map_err!(conv_param_string.0.parse::<TokenStream>(), Parse)?;
		let mut inputs = vec![];
		for token in stream {
			match token {
				Group(token) => {
					for token in token.stream() {
						match token {
							Ident(token) => inputs.push(token.to_string()),
							_ => {}
						}
					}
				}
				_ => {}
			}
		}
		let mut type_list = vec![];
		type_list.push(trait_name);
		let stream = map_err!(conv_param_string.1.parse::<TokenStream>(), Parse)?;
		let mut next_token = "".to_string();
		for token in stream {
			let token_str = token.to_string();

			if token_str == "," {
				type_list.push(next_token.clone());
				next_token = "".to_string();
			} else {
				if next_token.len() == 0 {
					next_token = format!("{}", token_str);
				} else {
					match token {
						Ident(_) => next_token = format!("{} {}", next_token, token_str),
						Group(_) => next_token = format!("{} {}", next_token, token_str),
						_ => next_token = format!("{}{}", next_token, token_str),
					}
				}
			}
		}
		if next_token.len() > 0 {
			type_list.push(next_token);
		}

		let mut trait_text = format!("");
		let mut comment_map: HashMap<String, String> = HashMap::new();
		let mut return_value = None;
		let mut error_value = vec![];
		let mut see_value = vec![];

		let mut comment_vec = vec![];
		for comment in comments {
			for token in comment.stream() {
				match token {
					Literal(l) => {
						let l = l.to_string();
						comment_vec.push(l);
					}
					_ => {}
				}
			}
		}

		let mut examples = vec![];
		let mut start_examples = false;
		let mut deprecated = false;

		let mut last_error = false;
		let mut last_return = false;
		let mut last_param: Option<String> = None;

		for comment in comment_vec {
			let trim = trim_outer(&comment, "\"", "\"");
			let trim = trim.trim();
			let mut found = false;
			if trim.find("@param ") == Some(0) {
				if trim.len() > 7 {
					let trim = trim.substring(7, trim.len());
					match trim.find(" ") {
						Some(pos) => {
							if pos + 1 < trim.len() {
								let name = trim.substring(0, pos);
								let value = trim.substring(pos + 1, trim.len());
								comment_map.insert(name.to_string(), value.to_string());
								found = true;
								last_param = Some(name.to_string());
								last_return = false;
								last_error = false;
							}
						}
						None => {}
					}
				}
			} else if trim.find("@deprecated") == Some(0) {
				deprecated = true;
				found = true;
				last_return = false;
				last_param = None;
				last_error = false;
			} else if trim.find("@return ") == Some(0) {
				if trim.len() > 8 {
					let trim = trim.substring(8, trim.len());
					match trim.find(" ") {
						Some(pos) => {
							if pos + 1 < trim.len() {
								let name = trim.substring(0, pos);
								let value = trim.substring(pos + 1, trim.len());
								return_value = Some(format!("[`{}`] - {}", name, value));
								found = true;
								last_return = true;
								last_param = None;
								last_error = false;
							}
						}
						None => {}
					}
				}
			} else if trim.find("@error ") == Some(0) {
				if trim.len() > 7 {
					let trim = trim.substring(7, trim.len());
					match trim.find(" ") {
						Some(pos) => {
							if pos + 1 < trim.len() {
								let name = trim.substring(0, pos);
								let value = trim.substring(pos + 1, trim.len());
								error_value.push(format!("* [`{}`] - {}", name, value));
								found = true;
								last_error = true;
								last_return = false;
								last_param = None;
							}
						}
						None => {}
					}
				}
			} else if trim.find("@see ") == Some(0) {
				if trim.len() > 5 {
					let trim = trim.substring(5, trim.len());
					see_value.push(format!("* [`{}`]", trim));
					found = true;
				}
			} else if trim.find("# Example") == Some(0) || start_examples {
				// part of our example block
				start_examples = true;
				examples.push(comment.clone());
			}
			if !found && !start_examples {
				if last_return {
					let return_value_ref = return_value.as_ref().unwrap();
					let comment = trim_outer(&comment, "\"", "\"");
					let nreturn_value = format!("{}{}", return_value_ref, comment);
					return_value = Some(nreturn_value);
				} else if last_error {
					let index = error_value.len() - 1;
					let comment = trim_outer(&comment, "\"", "\"");
					error_value[index] = format!("{}{}", error_value[index], comment);
				} else if last_param.is_some() {
					let last_param_ref = last_param.as_ref().unwrap();
					let comment = trim_outer(&comment, "\"", "\"");
					let mut nvalue = "".to_string();
					match comment_map.get(last_param_ref) {
						Some(last) => {
							nvalue = format!("{}{}", last, comment);
						}
						None => {}
					}
					comment_map.insert(last_param_ref.to_string(), nvalue);
				} else {
					trait_text = format!("{}\n#[doc={}]", trait_text, comment);
				}
			}
		}

		if deprecated {
			trait_text = format!("{}\n#[doc=\"\"]", trait_text);
			trait_text = format!(
				"{}\n#[doc=\"<div class=\\\"warning\\\">This function is deprecated</div>\"]",
				trait_text
			);
			trait_text = format!("{}\n#[doc=\"\"]", trait_text);
		}
		trait_text = format!("{}\n#[doc=\"# Input Parameters\"]", trait_text);
		trait_text = format!("{}\n#[doc=\"|Parameter Name|Type|Comment|\"]", trait_text,);
		trait_text = format!("{}\n#[doc=\"\n|---|---|---|\"]", trait_text);
		let mut i = 0;
		for input in inputs {
			let type_str = if i < type_list.len() {
				format!("[`{}`]", type_list[i].clone())
			} else {
				format!("-")
			};
			let comment = match comment_map.get(&input) {
				Some(comment) => comment.to_string(),
				None => "`TODO: add @param documentation to describe this parameter`".to_string(),
			};
			trait_text = format!(
				"{}\n#[doc=\"\n|`{}`|{}|{}|\"]",
				trait_text, input, type_str, comment
			);

			i += 1;
		}
		trait_text = format!("{}\n#[doc=\"# Errors\"]", trait_text);
		if error_value.len() == 0 && return_errors {
			trait_text = format!(
				"{}\n#[doc=\"`TODO: add @error documentation to describe this function`\"]",
				trait_text
			);
		}
		if error_value.len() == 0 && !return_errors {
			trait_text = format!("{}\n#[doc=\"n/a\"]", trait_text);
		}
		for error in &error_value {
			trait_text = format!("{}\n#[doc=\"{}\"]", trait_text, error);
		}
		trait_text = format!("{}\n#[doc=\"# Return\"]", trait_text);
		trait_text = format!(
			"{}\n#[doc=\"{}\"]",
			trait_text,
			match return_value {
				Some(r) => r,
				None => "`TODO: add @return documentation to describe this function`".to_string(),
			}
		);
		trait_text = format!("{}\n#[doc=\"# Also See\"]", trait_text);
		if see_value.len() == 0 {
			trait_text = format!(
				"{}\n#[doc=\"`TODO: add @see documentation to link to related documentation`\"]",
				trait_text
			);
		}
		for see in see_value {
			trait_text = format!("{}\n#[doc=\"{}\"]", trait_text, see);
		}
		for example in examples {
			trait_text = format!("{}\n#[doc={}]", trait_text, example);
		}
		Ok(trait_text)
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
						debug!("ret_converloop token = '{}'", token)?;
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
		debug!("ret_convert='{}'", ret)?;
		Ok((ret, ret_types))
	}

	fn process_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.stage {
			Stage::ClassBlock => self.process_class_block(token),
			Stage::FnBlock => self.process_fn_block(token),
			Stage::VarBlock => self.process_var_block(token),
			Stage::ConstBlock => self.process_const_block(token),
			Stage::PublicBlock => self.process_public_block(token),
			Stage::ProtectedBlock => self.process_protected_block(token),
			Stage::CloneBlock => self.process_clone_block(token),
			Stage::CommentBlock => self.process_comment_block(token),
			Stage::Complete => err!(UnexpectedToken, "unexpected token after class definition"),
		}
	}

	fn process_comment_block(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(ref g) => {
				self.accumulated_comments.push(g.clone());
			}
			_ => self.append_error(&format!("Unexecpted token: '{}'", token))?,
		}
		debug!("comment token = {:?}", token)?;
		self.stage = Stage::ClassBlock;
		Ok(())
	}

	fn get_trait_name(&self, s: &String) -> String {
		match s.rfind("_sync_box") {
			Some(i) => {
				if i == s.len().saturating_sub("_sync_box".len()) {
					return s.substring(0, i).to_string();
				}
			}
			_ => {}
		}
		match s.rfind("_send_box") {
			Some(i) => {
				if i == s.len().saturating_sub("_send_box".len()) {
					return s.substring(0, i).to_string();
				}
			}
			_ => {}
		}
		match s.rfind("_send") {
			Some(i) => {
				if i == s.len().saturating_sub("_send".len()) {
					return s.substring(0, i).to_string();
				}
			}
			_ => {}
		}
		match s.rfind("_sync") {
			Some(i) => {
				if i == s.len().saturating_sub("_sync".len()) {
					return s.substring(0, i).to_string();
				}
			}
			_ => {}
		}
		match s.rfind("_box") {
			Some(i) => {
				if i == s.len().saturating_sub("_box".len()) {
					return s.substring(0, i).to_string();
				}
			}
			_ => {}
		}
		s.clone()
	}

	fn process_public_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("public_token={}", token)?;
		let token_str = token.to_string();
		if token_str == ";" {
			for t in &self.trait_set {
				match &mut self.trait_comments.get_mut(t) {
					Some(tc) => {
						tc.extend(self.accumulated_comments.clone());
					}
					None => {
						let nvec = self.accumulated_comments.clone();
						self.trait_comments.insert(t.clone(), nvec);
					}
				}
			}
			self.accumulated_comments.clear();
			self.trait_set.clear();
			self.stage = Stage::ClassBlock;
		} else {
			match token {
				Ident(ident) => {
					debug!("add to public: {}", ident)?;
					self.public_set.insert(ident.to_string());
					let trait_name = self.get_trait_name(&ident.to_string());
					self.trait_set.insert(trait_name);
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn process_clone_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("clone_block token={}", token)?;
		let token_str = token.to_string();
		if token_str == ";" {
			self.stage = Stage::ClassBlock;
		} else {
			match token {
				Ident(ident) => {
					self.clone_set.insert(ident.to_string());
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn process_protected_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("protected_token={}", token)?;
		let token_str = token.to_string();
		if token_str == ";" {
			self.stage = Stage::ClassBlock;
		} else {
			match token {
				Ident(ident) => {
					debug!("add to protected: {}", ident)?;
					self.protected_set.insert(ident.to_string());
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn process_fn_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("fnblock token = {:?}", token)?;
		match token {
			Group(ref group) => {
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
							if cur_fn.param_string.len() == 0 {
								cur_fn.param_string = group.to_string();
								cur_fn.signature =
									format!("{}{}", cur_fn.signature, group.to_string());
							} else {
								cur_fn.signature =
									format!("{}{}", cur_fn.signature, group.to_string());
							}
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
			Ident(ref ident) => match &mut self.cur_fn {
				Some(ref mut cur_fn) => {
					if cur_fn.name.len() == 0 {
						cur_fn.name = ident.to_string();
						cur_fn.signature = format!("fn {}", ident.to_string());
					} else {
						if self.prev_is_joint {
							cur_fn.signature = format!("{}{}", cur_fn.signature, ident.to_string());
						} else {
							cur_fn.signature =
								format!("{} {}", cur_fn.signature, ident.to_string());
						}
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
					debug!("other token type = '{}'", token)?;
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

		self.prev_is_joint = false;
		match token {
			Punct(p) => {
				if p.spacing() == Spacing::Joint {
					self.prev_is_joint = true
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
								match token {
									Ident(ident) => {
										var.type_str =
											format!("{} {}", var.type_str, ident.to_string());
									}
									_ => {
										var.type_str = format!("{}{}", var.type_str, token_str);
									}
								}
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
			match &mut self.cur_const {
				None => {
					self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
				}
				Some(ref mut c) => {
					c.comments.extend(self.accumulated_comments.clone());
					if !c.found_colon
						|| !c.found_equal || c.type_str.len() == 0
						|| c.value_str.len() == 0
					{
						self.append_error(&format!("unexpected token: '{}'", token_str)[..])?;
					} else {
						self.const_list.push(c.clone());
					}
					self.cur_const = None;
				}
			}
			self.accumulated_comments.clear();
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
				// for now just clear them
				} else if ident_str == "fn" {
					if self.cur_fn.is_none() {
						debug!("creating a cur_fn")?;
						let mut fn_info = FnInfo::new();
						fn_info.comments.extend(self.accumulated_comments.clone());
						self.accumulated_comments.clear();
						self.cur_fn = Some(fn_info);
					}
					self.stage = Stage::FnBlock;
				} else if ident_str == "public" {
					self.stage = Stage::PublicBlock;
				} else if ident_str == "protected" {
					self.stage = Stage::ProtectedBlock;
				} else if ident_str == "clone" {
					self.stage = Stage::CloneBlock;
				} else {
					self.append_error(
						&format!("Parse Error: unexpected token '{}'", ident_str)[..],
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
					let mut fn_info = FnInfo::new();
					fn_info.comments.extend(self.accumulated_comments.clone());
					self.accumulated_comments.clear();
					self.cur_fn = Some(fn_info);
					self.process_method_list(group)?;
				} else {
					self.append_error(&format!(
						"Parse Error: unexpected token: '{:?}'",
						group.delimiter()
					))?;
				}
			}
			Punct(p) => {
				if p == '#' {
					self.stage = Stage::CommentBlock;
				} else {
					self.append_error(&format!("Parse Error: unexecpted token '{}'", p)[..])?;
				}
				//self.process_abort("abort here!".to_string())?;
			}
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
