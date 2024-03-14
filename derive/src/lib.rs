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

//! This crate is a proc_macro crate and it includes the Serializable macro.
//! This macro implements the [`bmw_ser::Serializable`] trait for any struct or enum.
//!
//! # Examples
//!
//!```
//! use bmw_derive::Serializable;
//! use bmw_ser::Serializable;
//! use bmw_err::Error;
//!
//! #[derive(Serializable)]
//! struct MyStruct {
//!     id: u64,
//!     is_member: bool,
//!     name: String,
//! }
//!
//! #[derive(Serializable)]
//! enum MyEnum {
//!     Type1(String),
//!     Type2(u64),
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let _s1 = MyStruct {
//!         id: 1234,
//!         is_member: true,
//!         name: "Hagrid".to_string(),
//!     };
//!
//!     let _s2 = MyEnum::Type1("something".to_string());
//!
//!     Ok(())
//! }
//!
//!```
//!
//! For additional examples, see the bmw_util documentation.

extern crate proc_macro;
use bmw_err::{err, Error};
use bmw_log::*;
use proc_macro::TokenStream;
use proc_macro::TokenTree;
use proc_macro::TokenTree::{Group, Ident, Literal, Punct};

// Note about tarpaulin. Tarpaulin doesn't cover proc_macros so we disable it throughout this
// library.

info!();

struct MacroState {
	ret_read: String,
	ret_write: String,
	expect_name: bool,
	name: String,
	field_names: Vec<String>,
	is_enum: bool,
}

#[cfg(not(tarpaulin_include))]
impl MacroState {
	fn new() -> Self {
		Self {
			ret_read: "".to_string(),
			ret_write: "".to_string(),
			expect_name: false,
			name: "".to_string(),
			field_names: vec![],
			is_enum: false,
		}
	}

	fn append_read(&mut self, s: &str) {
		self.ret_read = format!("{}{}", self.ret_read, s);
	}

	fn append_write(&mut self, s: &str) {
		self.ret_write = format!("{}{}", self.ret_write, s);
	}

	fn ret(&self) -> String {
		let ret = if self.is_enum {
			format!("impl bmw_ser::Serializable for {} {{ \n\
                                        fn read<R>(reader: &mut R) -> Result<Self, bmw_err::Error> where R: bmw_ser::Reader {{\n\
                                            Ok(match reader.read_u16()? {{ {} _ => {{\n\
                                            let fmt = \"unexpected type returned in reader\";\n\
                                            let e = bmw_err::err!(bmw_err::ErrKind::CorruptedData, fmt);\n\
                                            return Err(e);\n\
                                            }}\n\
                                        }}) }} \n\
                    fn write<W>(&self, writer: &mut W) -> Result<(), bmw_err::Error> where W: bmw_ser::Writer {{ match self {{ {} }} Ok(()) }}\n\
                    }}", self.name, self.ret_read, self.ret_write)
		} else {
			let mut field_name_return = "Ok(Self {".to_string();
			for x in &self.field_names {
				field_name_return = format!("{} {},", field_name_return, x);
			}
			field_name_return = format!("{} }})", field_name_return);
			format!("impl bmw_ser::Serializable for {} {{ \n\
                    fn read<R>(reader: &mut R) -> Result<Self, bmw_err::Error> where R: bmw_ser::Reader {{ {} {} }}\n\
                    fn write<W>(&self, writer: &mut W) -> Result<(), bmw_err::Error> where W: bmw_ser::Writer {{ {} Ok(()) }}\n\
                    }}", self.name, self.ret_read, field_name_return, self.ret_write)
		};

		let _ = debug!("ret='{}'", ret);

		ret
	}
}

/// This is a proc macro for implementing the [`bmw_ser::Serializable`] trait. See the [`crate`]
/// documentation for examples.
#[proc_macro_derive(Serializable)]
#[cfg(not(tarpaulin_include))]
pub fn derive_serialize(strm: TokenStream) -> TokenStream {
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
				let e = err!(ErrKind::IllegalState, fmt);
				return Err(e);
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
		let e = err!(ErrKind::IllegalState, fmt);
		return Err(e);
	}

	debug!("state.is_enum={},has_inner={}", state.is_enum, has_inner)?;
	if state.is_enum {
		debug!("do an append enum")?;
		if has_inner {
			state.append_read(
				&format!(
					"{} => {}::{}(Serializable::read(reader)?),\n",
					state.field_names.len(),
					state.name,
					name,
				)[..],
			);
			state.append_write(
				&format!(
					"{}::{}(x) => {{ writer.write_u16({})?; Serializable::write(x, writer)?; }},\n",
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
		state.append_read(&format!("let {} = bmw_ser::Serializable::read(reader)?;\n", name)[..]);
		state
			.append_write(&format!("bmw_ser::Serializable::write(&self.{}, writer)?;\n", name)[..]);
	}
	state.field_names.push(name.clone());

	Ok(())
}
