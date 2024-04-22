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
use crate::types::{ConstType::*, Method, ObjectConst, ObjectField};
use crate::utils::trim_outer;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use bmw_deps::convert_case::{Case, Casing};
use bmw_deps::proc_macro_error::{emit_error, Diagnostic, Level};
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;
use std::collections::HashMap;

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

impl MacroState {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			span: None,
			expect_name: false,
			expect_impl: false,
			expect_tag: false,
			name: None,
			builder: None,
			const_list: vec![],
			field_list: vec![],
			views: HashMap::new(),
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
		let (default_name, default_value) = match conf.default {
			U8(v) => ("u8", v.to_string()),
			U32(v) => ("u32", v.to_string()),
			Usize(v) => ("usize", v.to_string()),
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
		for method in methods {
			ntrait = format!("{} {};", ntrait, method.signature);
			nimpl = format!(
				"{} {} {{ {}::{}({}) }}",
				nimpl, method.signature, impl_name, method.name, method.param_string,
			);
		}
		nimpl = format!("{} }}", nimpl);
		ntrait = format!("{} }}", ntrait);
		state.ret.extend(ntrait.parse::<TokenStream>());
		state.ret.extend(nimpl.parse::<TokenStream>());
	}
	Ok(())
}

fn build_macros(state: &mut MacroState) -> Result<(), Error> {
	let builder = match &state.builder {
		Some(builder) => builder,
		None => return err!(Parse, "expected builder"),
	};
	for (name, methods) in &state.views {
		// if we get here the name is not none so unwrap is ok.
		let impl_name = state.name.as_ref().unwrap();
		let trait_name = name.to_case(Case::Pascal);
		let mut macro_text = format!("#[macro_export]\nmacro_rules! {} {{\n() => {{{{\n", name);

		macro_text = format!(
			"{}\nlet config = configure!({}Config, {}ConfigOptions, vec![Y(2)])?;",
			macro_text, impl_name, impl_name
		);
		macro_text = format!(
			"{} match {}::{}(config) {{\nOk(ret) => {{let ret: Box<dyn {}> = Box::new(ret); Ok(ret)}}, Err(e) => err!(BaseErrorKind::Parse, e.to_string()), }}",
			macro_text, impl_name, builder, trait_name,
		);
		macro_text = format!("{}\n}}}};\n}}", macro_text);
		debug!("macro_text={}", macro_text)?;
		state.ret.extend(macro_text.parse::<TokenStream>());
	}
	/*
	match Animal::builder(config) {
		Ok(ret) => Ok(ret),
		Err(e) => err!(Parse, "{}", e),
	}
		*/

	Ok(())
}

fn process_tag(tag: String, state: &mut MacroState) -> Result<(), Error> {
	let tag = trim_outer(&tag, "[", "]");
	debug!("Tag='{}'", tag)?;
	Ok(())
}

fn do_derive_object_impl(
	_attr: TokenStream,
	item: TokenStream,
	state: &mut MacroState,
) -> Result<(), Error> {
	debug!("in do_derive_object_impl")?;
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
				debug!("group={}", group)?;
				if state.expect_tag {
					debug!("found a tag")?;
					state.expect_tag = false;
					process_tag(group.to_string(), state)?;
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
	state.field_list = vec![ObjectField {
		name: "x".to_string(),
		otype: "i32".to_string(),
	}];
	let method_list = vec![Method {
		name: "bark".to_string(),
		signature: "fn bark(&mut self) -> Result<String, Error>".to_string(),
		views: vec!["dog".to_string(), "test".to_string()],
		param_string: "self".to_string(),
	}];
	state.views = HashMap::new();
	state.views.insert("dog".to_string(), method_list.clone());
	state.views.insert("test".to_string(), method_list);
	state.builder = Some("builder".to_string());
	build_object(state)?;
	debug!("state.ret='{}'", state.ret)?;

	Ok(())
}
