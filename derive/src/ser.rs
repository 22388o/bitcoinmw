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

use crate::types::DeriveErrorKind;
use crate::types::SerMacroState as MacroState;
use bmw_base::*;
use proc_macro::TokenStream;
use proc_macro::TokenTree;
use proc_macro::TokenTree::{Group, Ident, Literal, Punct};
use std::str::from_utf8;

// Note about tarpaulin. Tarpaulin doesn't cover proc_macros so we disable it throughout this
// crate.

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
                        err!(DeriveErrorKind::Log, "impossible logging error")
                }
        }};
	($line:expr, $($values:tt)*) => {{
		if DEBUG {
			println!($line, $($values)*);
		}
                if true {
                        Ok(())
                } else {
                        err!(DeriveErrorKind::Log, "impossible logging error")
                }
	}};
}
macro_rules! error {
        ($line:expr, $($values:tt)*) => {{
                println!($line, $($values)*);
        }};
}

#[cfg(not(tarpaulin_include))]
impl MacroState {
	pub(crate) fn new() -> Self {
		Self {
			ret_read: "".to_string(),
			ret_write: "".to_string(),
			expect_name: false,
			name: "".to_string(),
			field_names: vec![],
			is_enum: false,
		}
	}

	pub(crate) fn ret(&self) -> String {
		let ret = if self.is_enum {
			let include_bytes = include_bytes!("../resources/ser_enum_template.txt");
			// unwrap ok here because we control templates as they are in the binary
			let template = from_utf8(include_bytes).unwrap();
			let template = template.replace("${NAME}", &self.name);
			let template = template.replace("${RET_READ}", &self.ret_read);
			let template = template.replace("${RET_WRITE}", &self.ret_write);

			template.to_string()
		} else {
			let include_bytes = include_bytes!("../resources/ser_struct_template.txt");
			// unwrap ok here because we control templates as they are in the binary
			let template = from_utf8(include_bytes).unwrap();

			// concat field names
			let mut field_name_return = "Ok(Self {".to_string();
			for x in &self.field_names {
				field_name_return = format!("{} {},", field_name_return, x);
			}
			field_name_return = format!("{} }})", field_name_return);

			let template = template.replace("${NAME}", &self.name);
			let template = template.replace("${RET_READ}", &self.ret_read);
			let template = template.replace("${FIELD_NAME_RETURN}", &field_name_return);
			let template = template.replace("${RET_WRITE}", &self.ret_write);

			template.to_string()
		};

		let _ = debug!("ret='{}'", ret);

		ret
	}

	fn append_read(&mut self, s: &str) {
		self.ret_read = format!("{}{}", self.ret_read, s);
	}

	fn append_write(&mut self, s: &str) {
		self.ret_write = format!("{}{}", self.ret_write, s);
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_serialize(strm: TokenStream) -> TokenStream {
	let mut state = MacroState::new();
	let _ = debug!("-----------------derive serialization----------------");
	match process_strm(strm, &mut state) {
		Ok(_) => state.ret().parse().unwrap(),
		Err(e) => {
			let _ = error!("parsing Serializable generated error: {}", e);
			"".parse().unwrap()
		}
	}
}

#[cfg(not(tarpaulin_include))]
fn process_strm(strm: TokenStream, state: &mut MacroState) -> Result<(), Error> {
	for tree in strm {
		process_token_tree(tree, state)?;
	}
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_token_tree(tree: TokenTree, state: &mut MacroState) -> Result<(), Error> {
	match tree {
		Ident(ident) => {
			let ident = ident.to_string();
			debug!("ident={}", ident)?;

			if state.expect_name {
				debug!("struct/enum name = {}", ident)?;
				state.name = ident.clone();
				state.expect_name = false;
			} else if ident != "pub" && ident != "struct" && ident != "enum" {
				let fmt = format!("error expected pub or struct. Found '{}'", ident);
				let e = err!(BaseErrorKind::IllegalState, fmt);
				return e;
			}

			if ident == "struct" || ident == "enum" {
				state.expect_name = true;
				if ident == "struct" {
					state.is_enum = false;
				} else {
					state.is_enum = true;
				}
			}
		}
		Group(group) => {
			process_group(group, state)?;
		}
		Literal(literal) => {
			debug!("literal={}", literal)?;
		}
		Punct(punct) => {
			debug!("punct={}", punct)?;
		}
	}
	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_group(group: proc_macro::Group, state: &mut MacroState) -> Result<(), Error> {
	debug!("group={}", group)?;

	let mut expect_name = true;
	let mut name = "".to_string();
	let mut has_inner = false;

	for item in group.stream() {
		match item {
			Ident(ident) => {
				let ident = ident.to_string();
				debug!("groupident={}", ident)?;
				if expect_name && ident != "pub" && ident != "doc" && ident != "crate" {
					expect_name = false;
					has_inner = false;
					name = ident.clone();
				}
			}
			Group(group) => {
				// we don't need to process the inner group because the read function
				// only requires the name, we do use this to determine if there's
				// an inner value
				debug!("group={}", group)?;
				has_inner = true;
			}
			Literal(literal) => {
				debug!("groupliteral={}", literal)?;
			}
			Punct(punct) => {
				debug!("grouppunct={}", punct)?;
				if punct.to_string() == ",".to_string() {
					debug!("end a name: {}", name)?;
					process_field(&name, &group, state, has_inner)?;
					expect_name = true;
				}
			}
		}
	}

	// if there's no trailing comma.
	if !expect_name {
		debug!("end name end loop: {}", name)?;
		process_field(&name, &group, state, has_inner)?;
	}

	Ok(())
}

#[cfg(not(tarpaulin_include))]
fn process_field(
	name: &String,
	group: &proc_macro::Group,
	state: &mut MacroState,
	has_inner: bool,
) -> Result<(), Error> {
	if name.len() == 0 {
		let fmt = format!("expected name for this group: {:?}", group);
		let e = err!(BaseErrorKind::IllegalState, fmt);
		return e;
	}

	debug!("state.is_enum={},has_inner={}", state.is_enum, has_inner)?;
	if state.is_enum {
		debug!("do an append enum")?;
		if has_inner {
			state.append_read(
				&format!(
					"{} => {}::{}(Serializable::read(reader)?),\n\t\t\t",
					state.field_names.len(),
					state.name,
					name,
				)[..],
			);
			state.append_write(
				&format!(
					"{}::{}(x) => {{ writer.write_u16({})?; Serializable::write(x, writer)?; }},\n\t\t\t",
					state.name,
					name,
					state.field_names.len()
				)[..],
			);
		} else {
			state.append_read(
				&format!("{} => {}::{},\n", state.field_names.len(), state.name, name)[..],
			);
			state.append_write(
				&format!(
					"{}::{} => {{ writer.write_u16({})?; }},\n",
					state.name,
					name,
					state.field_names.len()
				)[..],
			);
		}
	} else {
		state.append_read(
			&format!(
				"let {} = bmw_derive::Serializable::read(reader)?;\n\t\t",
				name
			)[..],
		);
		state.append_write(
			&format!(
				"bmw_derive::Serializable::write(&self.{}, writer)?;\n\t\t",
				name
			)[..],
		);
	}
	state.field_names.push(name.clone());

	Ok(())
}
