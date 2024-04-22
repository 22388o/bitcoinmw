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
use crate::types::ObjectMacroState as MacroState;
use crate::types::{ConstType, ConstType::*, Method, ObjectConst, ObjectField};
use crate::utils::trim_outer;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use bmw_deps::convert_case::{Case, Casing};
use bmw_deps::proc_macro_error::{emit_error, Diagnostic, Level};
use bmw_deps::substring::Substring;
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;
use std::collections::HashMap;

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

impl Method {
	fn new() -> Self {
		Self {
			name: "".to_string(),
			param_string: "".to_string(),
			signature: "".to_string(),
			views: vec![],
		}
	}
}

impl MacroState {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			span: None,
			expect_name: false,
			expect_impl: false,
			expect_tag: false,
			expect_method_name: false,
			in_builder: false,
			in_method: false,
			in_field: false,
			in_config: false,
			expect_builder_name: false,
			name: None,
			builder: None,
			const_list: vec![],
			field_list: vec![],
			views: HashMap::new(),
			cur_method: None,
		}
	}

	fn reset_bools(&mut self) {
		self.expect_name = false;
		self.expect_impl = false;
		self.expect_tag = false;
	}
}

pub(crate) fn do_derive_object(attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut state = MacroState::new();
	match do_derive_object_impl(attr, item.clone(), &mut state) {
		Ok(_) => state.ret,
		Err(e) => {
			match state.span {
				Some(span) => {
					let msg = format!(
						"object proc_macro_attribute generated error: {}",
						e.kind().to_string()
					);
					let diag = Diagnostic::spanned(span.into(), Level::Error, msg.clone());
					emit_error!(diag, msg);
				}
				None => {
					println!("unknown error occurred in proc_macro: {}", e);
				}
			}
			TokenStream::new()
		}
	}
}

fn build_object(state: &mut MacroState) -> Result<(), Error> {
	let name = match &state.name {
		Some(name) => name.clone(),
		None => return err!(Parse, "could not find the name of the object"),
	};
	let mut base_struct = format!("struct {} {{ config: {}Config,", name, name);
	for field in &state.field_list {
		base_struct = format!("{}{}:{},", base_struct, field.name, field.otype);
	}

	base_struct = format!("{}}}", base_struct);
	state.ret.extend(base_struct.parse::<TokenStream>());

	// we know we can unwrap because otherwise an error would have been returned
	let name = state.name.as_ref().unwrap();

	let mut config_text = format!("struct {}Config {{", name);
	let mut default_text = format!(
		"impl Default for {}Config {{ fn default() -> Self {{ Self {{",
		name
	);

	for conf in &state.const_list {
		let (default_name, default_value) = match &conf.default {
			U8(v) => ("u8", v.to_string()),
			U32(v) => ("u32", v.to_string()),
			U64(v) => ("u64", v.to_string()),
			U128(v) => ("u128", v.to_string()),
			Usize(v) => ("usize", v.to_string()),
			Bool(v) => ("bool", v.to_string()),
			Tuple(v) => ("(String, String)", format!("(\"{}\", \"{}\")", v.0, v.1)),
			ConfString(v) => ("String", format!("\"{}\"", v)),
			_ => ("", "".to_string()),
		};
		config_text = format!("{}{}: {},", config_text, conf.name, default_name,);
		default_text = format!("{}{}: {},", default_text, conf.name, default_value);
	}

	default_text = format!("{}}} }} }}", default_text);
	config_text = format!("{}}}", config_text);
	let config_token_stream = map_err!(config_text.parse::<TokenStream>(), Parse)?;
	let options = derive_configurable(config_token_stream.clone());
	state.ret.extend(config_token_stream);
	state.ret.extend(options);
	state.ret.extend(default_text.parse::<TokenStream>());

	build_traits(state)?;
	build_macros(state)?;

	Ok(())
}

fn build_traits(state: &mut MacroState) -> Result<(), Error> {
	// if we're at this point, unwrap is ok because we already unwrapped it
	let impl_name = state.name.as_ref().unwrap();
	for (name, methods) in &state.views {
		let name = name.to_case(Case::Pascal);
		let mut ntrait = format!("\npub trait {} {{", name);
		let mut nimpl = format!("\nimpl {} for {} {{", name, impl_name);
		let mut nimplref = format!("\nimpl {} for &mut {} {{", name, impl_name);
		for method in methods {
			ntrait = format!("{} {};", ntrait, method.signature);
			nimpl = format!(
				"{} {} {{ {}::{}({}) }}",
				nimpl, method.signature, impl_name, method.name, method.param_string,
			);
			nimplref = format!(
				"{} {} {{ {}::{}({}) }}",
				nimplref, method.signature, impl_name, method.name, method.param_string,
			);
		}
		nimpl = format!("{} }}", nimpl);
		nimplref = format!("{} }}", nimplref);
		ntrait = format!("{} }}", ntrait);
		state.ret.extend(ntrait.parse::<TokenStream>());
		state.ret.extend(nimpl.parse::<TokenStream>());
		state.ret.extend(nimplref.parse::<TokenStream>());
	}
	Ok(())
}

fn build_macros(state: &mut MacroState) -> Result<(), Error> {
	let builder = match &state.builder {
		Some(builder) => builder,
		None => return err!(Parse, "expected builder"),
	};
	for (name, _methods) in &state.views {
		// if we get here the name is not none so unwrap is ok.
		let impl_name = state.name.as_ref().unwrap();
		let trait_name = name.to_case(Case::Pascal);
		let mut macro_text = format!(
			"#[macro_export]\nmacro_rules! {} {{\n($($param:tt)*) => {{{{\n",
			name
		);

		macro_text = format!(
			"{}\nlet config = configure!({}Config, {}ConfigOptions, vec![$($param)*])?;",
			macro_text, impl_name, impl_name
		);
		macro_text = format!(
			"{} match {}::{}(config) {{\nOk(ret) => {{let ret: Box<dyn {}> = Box::new(ret);",
			macro_text, impl_name, builder, trait_name,
		);
		macro_text = format!(
			"{}Ok(ret)}}, Err(e) => err!(BaseErrorKind::Parse, e.to_string()), }}",
			macro_text
		);
		macro_text = format!("{}\n}}}};\n}}", macro_text);
		state.ret.extend(macro_text.parse::<TokenStream>());
	}

	Ok(())
}

fn process_tag(tag: String, state: &mut MacroState) -> Result<(), Error> {
	let tag = trim_outer(&tag, "[", "]");
	debug!("Tag='{}'", tag)?;
	if tag == "builder" {
		state.in_builder = true;
	} else {
		state.in_builder = false;
		let tag = map_err!(tag.parse::<TokenStream>(), Parse)?;
		for token in tag {
			match token {
				Ident(ident) => {
					let ident_str = ident.to_string();
					debug!("tag ident = {}", ident)?;
					if ident_str == "method" {
						state.cur_method = Some(Method::new());
						state.in_method = true;
					} else if ident_str == "field" {
						state.in_field = true;
						state.in_method = false;
					} else if ident_str == "config" {
						state.in_config = true;
						state.in_field = false;
						state.in_method = false;
					} else {
						state.in_method = false;
						state.in_field = false;
					}
				}
				Group(group) => {
					let group_str = group.to_string();
					debug!("tag_group = '{}'", group_str)?;
					if state.in_method {
						let list = trim_outer(&group_str, "(", ")");
						let list = map_err!(list.parse::<TokenStream>(), Parse)?;
						for item in list {
							match item {
								Ident(view) => {
									let view_str = view.to_string();
									debug!("view={}", view_str)?;
									match &mut state.cur_method {
										Some(ref mut method) => {
											method.views.push(view_str);
										}
										None => {
											return err!(IllegalState, "Expected a method");
										}
									}
								}
								_ => {}
							}
						}
					} else if state.in_field {
						let list = trim_outer(&group_str, "(", ")");
						let list = map_err!(list.parse::<TokenStream>(), Parse)?;
						let mut name = None;
						let mut type_str = None;
						let mut expect_colon = true;
						for item in list {
							let v = match item {
								Ident(v) => v.to_string(),
								Group(v) => v.to_string(),
								Literal(v) => v.to_string(),
								Punct(v) => v.to_string(),
							};
							debug!("fieldv={}", v)?;
							if name == None {
								name = Some(v);
							} else if expect_colon {
								expect_colon = false;
								if v != ":" {
									return err!(Parse, "expected colon");
								}
							} else {
								match type_str {
									Some(cur) => {
										type_str = Some(format!("{}{}", cur, v));
									}
									None => {
										type_str = Some(v);
									}
								}
							}
						}

						if name == None || type_str == None {
							return err!(Parse, "invalid field tag");
						}

						state.field_list.push(ObjectField {
							name: name.unwrap(),
							otype: type_str.unwrap(),
						});

						state.in_field = false;
					} else if state.in_config {
						let list = trim_outer(&group_str, "(", ")");
						let list = map_err!(list.parse::<TokenStream>(), Parse)?;

						let mut name = None;
						let mut type_str = None;
						let mut init_str = None;
						let mut expect_colon = true;
						let mut in_init = false;
						for item in list {
							let v = match item {
								Ident(v) => v.to_string(),
								Group(v) => v.to_string(),
								Literal(v) => v.to_string(),
								Punct(v) => v.to_string(),
							};
							debug!("fieldv={}", v)?;
							if name == None {
								name = Some(v);
							} else if expect_colon {
								expect_colon = false;
								if v != ":" {
									return err!(Parse, "expected colon");
								}
							} else if v == "=" {
								if in_init {
									return err!(Parse, "unexpected equal sign");
								}
								in_init = true;
							} else if in_init {
								match init_str {
									Some(cur) => {
										init_str = Some(format!("{}{}", cur, v));
									}
									None => {
										init_str = Some(v);
									}
								}
							} else {
								match type_str {
									Some(cur) => {
										type_str = Some(format!("{}{}", cur, v));
									}
									None => {
										type_str = Some(v);
									}
								}
							}
						}

						if name == None || type_str == None || init_str == None {
							return err!(
								Parse,
								"invalid config tag name={:?},type_str={:?},init_str={:?}",
								name,
								type_str,
								init_str
							);
						}

						debug!(
							"config tag name={:?},type_str={:?},init_str={:?}",
							name, type_str, init_str
						)?;

						let const_type = match type_str.unwrap().as_str() {
							"u8" => {
								let const_type: ConstType = U8(init_str.unwrap().parse()?);
								const_type
							}
							"u16" => {
								let const_type: ConstType = U16(init_str.unwrap().parse()?);
								const_type
							}
							"u32" => {
								let const_type: ConstType = U32(init_str.unwrap().parse()?);
								const_type
							}
							"u64" => {
								let const_type: ConstType = U64(init_str.unwrap().parse()?);
								const_type
							}
							"u128" => {
								let const_type: ConstType = U128(init_str.unwrap().parse()?);
								const_type
							}
							"usize" => {
								let const_type: ConstType = Usize(init_str.unwrap().parse()?);
								const_type
							}
							"bool" => {
								let const_type: ConstType = Bool(init_str.unwrap().parse()?);
								const_type
							}
							"(String, String)" => {
								todo!()
							}
							"String" => {
								let const_type: ConstType = ConfString(init_str.unwrap());
								const_type
							}
							_ => {
								return err!(Parse, "unexpected type in config");
							}
						};
						state.const_list.push(ObjectConst {
							name: name.unwrap(),
							default: const_type,
						});

						state.in_config = false;
					}
				}
				_ => {}
			}
		}
	}

	Ok(())
}

fn do_derive_object_impl(
	attr: TokenStream,
	item: TokenStream,
	state: &mut MacroState,
) -> Result<(), Error> {
	debug!("in do_derive_object_impl")?;

	let mut has_inner = false;
	let mut name = None;
	for token in attr {
		match token {
			Ident(v) => {
				debug!("vident={}", v)?;
				name = Some(v.to_string());
				has_inner = true;
				break;
			}
			Group(v) => {
				debug!("vgroup={}", v)?;
				name = Some("Empty".to_string());
				has_inner = true;
			}
			Literal(_v) => {}
			Punct(v) => {
				debug!("vpunct={}", v)?;
			}
		}
	}

	// return for 2.0 style
	if has_inner {
		let empty_struct = format!("struct {} {{}}", name.unwrap()).parse::<TokenStream>();
		state.ret.extend(empty_struct);
		return Ok(());
	}

	state.expect_impl = true;
	state.ret.extend(item.clone());
	state.ret.extend("#[doc = \"test\"]".parse::<TokenStream>());

	debug!("======================================item======================================")?;

	for token in item {
		match token {
			Ident(ident) => {
				state.span = Some(ident.span());
				let ident_str = ident.to_string();
				debug!("ident={:?}", ident_str)?;
				if state.expect_impl {
					if ident_str != "impl" {
						return err!(Parse, "expected 'impl', found '{}'", ident_str);
					}
					state.reset_bools();
					state.expect_name = true;
					debug!("expect name")?;
				} else if state.expect_name {
					debug!("name={}", ident_str)?;
					state.name = Some(ident_str);
					state.reset_bools();
				} else {
					state.reset_bools();
				}
			}
			Group(group) => {
				state.span = Some(group.span());
				let group_str = group.to_string();
				debug!("group={}", group)?;
				if state.expect_tag {
					debug!("found a tag")?;
					state.expect_tag = false;
					process_tag(group_str, state)?;
				} else {
					// this is our function block
					let inner_group = trim_outer(&group_str, "{", "}");
					debug!("inner_grp='{}'", inner_group)?;
					let inner_group_tokens = map_err!(inner_group.parse::<TokenStream>(), Parse)?;
					process_inner_group_tokens(inner_group_tokens, state)?;
				}
			}
			Literal(literal) => {
				state.span = Some(literal.span());
				debug!("literal={:?}", literal)?;
				state.reset_bools();
			}
			Punct(punct) => {
				state.span = Some(punct.span());
				debug!("punct={:?}", punct)?;
				if punct != '#' {
					return err!(Parse, "expected '#' or '{' in this position");
				}
				state.expect_tag = true;
			}
		}
	}

	/*
	state.const_list = vec![
		ObjectConst {
			name: "y".to_string(),
			default: Usize(1),
		},
		ObjectConst {
			name: "z".to_string(),
			default: U8(10),
		},
	];
		*/
	/*
	state.field_list = vec![ObjectField {
		name: "x".to_string(),
		otype: "i32".to_string(),
	}];
		*/
	/*
	let method_list = vec![Method {
		name: "bark".to_string(),
		signature: "fn bark(&mut self) -> Result<String, Error>".to_string(),
		views: vec!["dog".to_string(), "test".to_string()],
		param_string: "self".to_string(),
	}];
		*/
	//state.views = HashMap::new();
	//state.views.insert("dog".to_string(), method_list.clone());
	//state.views.insert("test".to_string(), method_list);
	build_object(state)?;
	//debug!("state.ret='{}'", state.ret)?;

	Ok(())
}

fn process_inner_group_tokens(strm: TokenStream, state: &mut MacroState) -> Result<(), Error> {
	for token in strm {
		match token {
			Ident(ident) => {
				state.span = Some(ident.span());
				let ident_str = ident.to_string();
				debug!("inner ident = '{}'", ident_str)?;
				state.expect_tag = false;
				if state.expect_method_name {
					state.expect_method_name = false;
					match &mut state.cur_method {
						Some(ref mut method) => method.name = ident_str.clone(),
						None => {
							return err!(IllegalState, "expected a method");
						}
					}
				} else if state.in_method && ident_str == "fn" {
					state.expect_method_name = true;
				} else if state.expect_builder_name {
					debug!("FOUND BUILDER={}", ident_str)?;
					state.builder = Some(ident_str.clone());
					state.expect_builder_name = false;
				} else if ident_str == "fn" && state.in_builder {
					state.expect_builder_name = true;
				} else {
					state.expect_builder_name = false;
				}

				if state.in_method {
					match state.cur_method {
						Some(ref mut method) => {
							if ident_str == "_" {
								method.signature = format!("{}{}", method.signature, ident);
							} else {
								method.signature = format!("{} {}", method.signature, ident);
							}
						}
						None => return err!(IllegalState, "expected a cur_method"),
					}
				}
			}
			Group(group) => {
				state.span = Some(group.span());
				let group_str = group.to_string();

				if state.in_method && group_str.find("{") == Some(0) {
					state.in_method = false;
					debug!("METHOD COMPLETE. Cur = {:?}", state.cur_method)?;

					let mut view_list = vec![];
					match state.cur_method {
						Some(ref mut method) => {
							for view in &method.views {
								view_list.push(view.clone());
							}
						}
						None => return err!(IllegalState, "expected a cur_method"),
					}

					for view in view_list {
						let mut found = false;
						match state.views.get_mut(&view.clone()) {
							Some(view) => {
								view.push(state.cur_method.as_ref().unwrap().clone());
								found = true;
							}
							None => {}
						}

						if !found {
							state.views.insert(
								view.clone(),
								vec![state.cur_method.as_ref().unwrap().clone()],
							);
						}
					}

					state.cur_method = None;
				} else if state.in_method {
					// param list
					if state.in_method {
						match state.cur_method {
							Some(ref mut method) => {
								method.signature = format!("{} {} ", method.signature, group_str);
							}
							None => return err!(IllegalState, "expected a cur_method"),
						}
					}

					let inner = trim_outer(&group_str, "(", ")");
					match &mut state.cur_method {
						Some(ref mut method) => {
							let inner = match inner.find("self") {
								Some(i) => inner.substring(i, inner.len()).to_string(),
								None => inner,
							};
							method.param_string = inner
						}
						None => return err!(IllegalState, "expected a cur_method"),
					}
				}
				debug!("inner group = '{}'", group)?;
				if state.expect_tag {
					state.expect_tag = false;
					process_tag(group_str.clone(), state)?;
				}
			}
			Literal(literal) => {
				state.span = Some(literal.span());
				debug!("inner lit = '{}'", literal)?;
				state.expect_tag = false;
			}
			Punct(punct) => {
				state.span = Some(punct.span());
				debug!("inner punct = '{}'", punct)?;
				let punct_str = punct.to_string();

				if state.in_method {
					match state.cur_method {
						Some(ref mut method) => {
							method.signature = format!("{}{}", method.signature, punct_str);
						}
						None => return err!(IllegalState, "expected a cur_method"),
					}
				}

				if punct == '#' {
					state.expect_tag = true;
				} else {
					state.expect_tag = false;
				}
			}
		}
	}
	Ok(())
}
