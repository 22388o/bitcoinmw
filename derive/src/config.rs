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
use bmw_deps::convert_case::{Case, Casing};
use proc_macro::TokenTree::*;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};
use std::str::FromStr;
use State::*;

#[derive(Debug, Clone, PartialEq)]
enum FieldType {
	Usize,
	U8,
	U16,
	U32,
	U64,
	U128,
	Bool,
	CString,
	Configurable,
	VecUsize,
	VecBool,
	VecU8,
	VecU16,
	VecU32,
	VecU64,
	VecU128,
	VecCString,
	VecConfigurable,
}

impl FromStr for FieldType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Usize" => Ok(FieldType::Usize),
			"U8" => Ok(FieldType::U8),
			"U16" => Ok(FieldType::U16),
			"U32" => Ok(FieldType::U32),
			"U64" => Ok(FieldType::U64),
			"U128" => Ok(FieldType::U128),
			"Bool" => Ok(FieldType::Bool),
			"String" => Ok(FieldType::CString),
			"Configurable" => Ok(FieldType::Configurable),
			"VecUsize" => Ok(FieldType::VecUsize),
			"VecU8" => Ok(FieldType::VecU8),
			"VecU16" => Ok(FieldType::VecU16),
			"VecU32" => Ok(FieldType::VecU32),
			"VecU64" => Ok(FieldType::VecU64),
			"VecU128" => Ok(FieldType::VecU128),
			"VecString" => Ok(FieldType::VecCString),
			"VecConfigurable" => Ok(FieldType::VecConfigurable),
			"VecBool" => Ok(FieldType::VecBool),
			_ => err!(CoreErrorKind::IllegalArgument, "unknown FieldType"),
		}
	}
}

#[derive(Clone, Debug)]
struct Field {
	name: Option<String>,
	field_type: Option<FieldType>,
	type_str: Option<String>,
	required: bool,
	span: Span,
}

impl Field {
	fn new(span: Span) -> Self {
		Self {
			name: None,
			type_str: None,
			field_type: None,
			required: false,
			span,
		}
	}
}

struct SpanError {
	span: Span,
	msg: String,
}

enum State {
	WantsStruct,
	WantsName,
	WantsGroup,
	WantsFieldName,
	WantsColon,
	WantsType,
	WantsComma,
	WantsLessThan,
	WantsTypeExtension,
	WantsGreaterThan,
	WantsAttribute,
}

struct StateMachine {
	state: State,
	ret: TokenStream,
	configurable_struct_name: Option<String>,
	span: Option<Span>,
	error_list: Vec<SpanError>,
	cur_field: Option<Field>,
	fields: Vec<Field>,
}

macro_rules! update_template {
	($template:expr, $type_name:ident, $fields:expr, $name:expr) => {{
		let type_upper = stringify!($type_name).to_uppercase();
		let type_pascal = stringify!($type_name).to_case(Case::Pascal);
		let type_vec_pascal = format!("Vec{}", type_pascal);
		let match_type = FieldType::from_str(&type_pascal)?;
		let match_type_vec = FieldType::from_str(&type_vec_pascal)?;

		let mut replace_str = "".to_string();
		for field in $fields {
			let field_name = field.name.as_ref().unwrap();
			let field_type = field.field_type.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			if field_type == &match_type {
				replace_str = format!(
					"{}if name == \"{}\" {{ self.{} = value; }}\n",
					replace_str, field_pascal, field_name
				);
			} else if field_type == &match_type_vec {
				replace_str = format!(
					"{}if name == \"{}\" {{ self.{}.push(value); }}\n",
					replace_str, field_pascal, field_name
				);
			}
		}
		let var_replace_name = &format!("${{SET_{}}}", type_upper);
		let template = $template.replace(var_replace_name, &replace_str);

		let mut replace_str = "".to_string();
		for field in $fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			if field_type == &match_type || field_type == &match_type_vec {
				replace_str = format!(
					"{}{}Options::{}(v) => Some(*v),\n",
					replace_str, $name, field_pascal
				);
			}
		}

		let var_replace_name = &format!("${{OPTIONS_ENUM_VALUE_{}_MATCH}}", type_upper);
		let template = template.replace(var_replace_name, &replace_str);

		let mut replace_str = "".to_string();
		for field in $fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			if field_type == &match_type {
				replace_str = format!(
					"{}ret.push((\"{}\".to_string(), self.{}));\n",
					replace_str, field_pascal, field_name
				);
			}
		}

		let var_replace_name = &format!("${{{}_PARAMS_PUSH}}", type_upper);
		let template = template.replace(var_replace_name, &replace_str);

		let mut replace_str = "".to_string();
		for field in $fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			if field_type == &match_type_vec {
				replace_str = format!(
					"{}ret.push((\"{}\".to_string(), self.{}.clone()));\n",
					replace_str, field_pascal, field_name
				);
			}
		}

		let var_replace_name = &format!("${{VEC_{}_PARAMS_PUSH}}", type_upper);
		let template = template.replace(var_replace_name, &replace_str);

		template
	}};
}

impl StateMachine {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			configurable_struct_name: None,
			state: WantsStruct,
			span: None,
			error_list: vec![],
			cur_field: None,
			fields: vec![],
		}
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

	fn expected(&mut self, expected: &str, found: &str) -> Result<(), Error> {
		self.append_error(&format!(
			"expected token '{}', found token '{}'",
			expected, found
		))?;
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

	fn derive(&mut self, strm: &TokenStream) -> Result<TokenStream, Error> {
		for token in strm.clone() {
			self.process_token_tree(token)?;
		}

		self.check_fields()?;
		if self.error_list.len() > 0 {
			self.print_errors()?;
		}

		self.generate()?;
		Ok(self.ret.clone())
	}

	fn check_fields(&mut self) -> Result<(), Error> {
		for field in self.fields.clone() {
			if field.name.is_none() {
				self.span = Some(field.span);
				self.append_error("could not parse this field's name")?;
			}
			if field.type_str.is_none() {
				self.span = Some(field.span);
				self.append_error("could not parse this field's type")?;
			}
		}
		Ok(())
	}

	fn process_token_tree(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.state {
			WantsStruct => self.process_wants_struct(token)?,
			WantsName => self.process_wants_name(token)?,
			WantsGroup => self.process_wants_group(token)?,
			WantsFieldName => self.process_wants_field_name(token)?,
			WantsColon => self.process_wants_colon(token)?,
			WantsType => self.process_wants_type(token)?,
			WantsComma => self.process_wants_comma(token)?,
			WantsLessThan => self.process_wants_less_than(token)?,
			WantsTypeExtension => self.process_wants_type_extension(token)?,
			WantsGreaterThan => self.process_wants_greater_than(token)?,
			WantsAttribute => self.process_wants_attribute(token)?,
		}
		Ok(())
	}

	fn process_wants_attribute(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Group(group) => {
				for item in group.stream() {
					let item_str = item.to_string();
					if item_str == "required" {
						let mut field = Field::new(self.span.as_ref().unwrap().clone());
						field.required = true;
						self.cur_field = Some(field);
					}
				}
			}
			_ => {}
		}
		self.state = State::WantsFieldName;
		Ok(())
	}

	fn process_wants_greater_than(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() != ">" {
			self.expected(">", &token.to_string())?;
		} else {
			match self.cur_field.as_mut() {
				Some(cur_field) => {
					self.state = State::WantsComma;
					self.fields.push(cur_field.clone());
					self.cur_field = None;
				}
				_ => {}
			}
		}
		Ok(())
	}

	fn process_wants_type_extension(&mut self, token: TokenTree) -> Result<(), Error> {
		match self.cur_field.as_mut() {
			Some(cur_field) => {
				let token_str = token.to_string();
				match token {
					Ident(ident) => {
						let ident_str = ident.to_string();
						let type_str = match cur_field.type_str.as_ref() {
							Some(type_str) => type_str.clone(),
							None => "".to_string(),
						};
						if ident_str == "usize" {
							cur_field.field_type = Some(FieldType::VecUsize);
						} else if ident_str == "String" {
							cur_field.field_type = Some(FieldType::VecCString);
						} else if ident_str == "bool" {
							cur_field.field_type = Some(FieldType::VecBool);
						} else if ident_str == "u8" {
							cur_field.field_type = Some(FieldType::VecU8);
						} else if ident_str == "u16" {
							cur_field.field_type = Some(FieldType::VecU16);
						} else if ident_str == "u32" {
							cur_field.field_type = Some(FieldType::VecU32);
						} else if ident_str == "u64" {
							cur_field.field_type = Some(FieldType::VecU64);
						} else if ident_str == "u128" {
							cur_field.field_type = Some(FieldType::VecU128);
						} else {
							cur_field.field_type = Some(FieldType::VecConfigurable);
						}
						cur_field.type_str = Some(format!("{}{}", type_str, ident_str));
						self.state = State::WantsGreaterThan;
					}
					_ => {
						self.expected("<type>", &token_str)?;
					}
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn process_wants_less_than(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() != "<" {
			self.expected("<", &token.to_string())?;
		}

		self.state = State::WantsTypeExtension;
		Ok(())
	}

	fn process_wants_comma(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() != "," {
			self.expected(",", &token.to_string())?;
		} else {
			self.state = State::WantsFieldName;
		}
		Ok(())
	}

	fn process_wants_type(&mut self, token: TokenTree) -> Result<(), Error> {
		match self.cur_field.as_mut() {
			Some(cur_field) => {
				let token_str = token.to_string();
				match token {
					Ident(ident) => {
						let ident_str = ident.to_string();

						if ident_str != "Vec" {
							cur_field.type_str = Some(ident_str.clone());
							self.state = State::WantsComma;

							if ident_str == "usize" {
								cur_field.field_type = Some(FieldType::Usize);
							} else if ident_str == "String" {
								cur_field.field_type = Some(FieldType::CString);
							} else if ident_str == "bool" {
								cur_field.field_type = Some(FieldType::Bool);
							} else if ident_str == "u8" {
								cur_field.field_type = Some(FieldType::U8);
							} else if ident_str == "u16" {
								cur_field.field_type = Some(FieldType::U16);
							} else if ident_str == "u32" {
								cur_field.field_type = Some(FieldType::U32);
							} else if ident_str == "u64" {
								cur_field.field_type = Some(FieldType::U64);
							} else if ident_str == "u128" {
								cur_field.field_type = Some(FieldType::U128);
							} else {
								cur_field.field_type = Some(FieldType::Configurable);
							}

							self.fields.push(cur_field.clone());
							self.cur_field = None;
						} else {
							self.state = State::WantsLessThan;
						}
					}
					_ => {
						self.expected("<type>", &token_str)?;
					}
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn process_wants_colon(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == ":" {
			self.state = State::WantsType;
		} else {
			self.expected(":", &token.to_string())?;
		}
		Ok(())
	}

	fn process_wants_field_name(&mut self, token: TokenTree) -> Result<(), Error> {
		match token {
			Ident(ident) => {
				match self.cur_field.as_mut() {
					Some(cur_field) => {
						cur_field.name = Some(ident.to_string());
						cur_field.span = self.span.as_ref().unwrap().clone();
					}
					None => {
						let mut field = Field::new(self.span.as_ref().unwrap().clone());
						field.name = Some(ident.to_string());
						self.cur_field = Some(field);
					}
				}
				self.state = State::WantsColon;
			}
			Punct(p) => {
				if p != '#' {
					self.append_error(&format!("expected field name, found, '{}'", p.to_string()))?;
				} else {
					// comments or 'required' attribute
					self.state = State::WantsAttribute;
				}
			}
			_ => {
				self.append_error(&format!("expected field name, found, '{}'", token))?;
			}
		}
		Ok(())
	}

	fn process_wants_group(&mut self, group: TokenTree) -> Result<(), Error> {
		match group {
			Group(group) => {
				if group.delimiter() != Delimiter::Brace {
					self.expected("{", &format!("{:?}", group.delimiter()))?;
				} else {
					self.state = State::WantsFieldName;
					for token in group.stream() {
						self.process_token_tree(token)?;
					}
				}
			}
			_ => {
				self.expected("{", &format!("{}", group))?;
			}
		}
		Ok(())
	}

	fn process_wants_struct(&mut self, token: TokenTree) -> Result<(), Error> {
		if token.to_string() == "struct" {
			self.state = State::WantsName;
		}
		Ok(())
	}

	fn process_wants_name(&mut self, token: TokenTree) -> Result<(), Error> {
		self.configurable_struct_name = Some(token.to_string());
		self.state = State::WantsGroup;
		Ok(())
	}

	fn update_enum_variants(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_pascal = field.name.as_ref().unwrap().to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			match field_type {
				FieldType::Configurable | FieldType::VecConfigurable => {
					replace_str =
						format!("{}{}(Box<dyn Configurable>),\n", replace_str, field_pascal,);
				}
				FieldType::Usize | FieldType::VecUsize => {
					replace_str = format!("{}{}(usize),\n", replace_str, field_pascal);
				}
				FieldType::CString | FieldType::VecCString => {
					replace_str = format!("{}{}(&'a str),\n", replace_str, field_pascal);
				}
				FieldType::Bool | FieldType::VecBool => {
					replace_str = format!("{}{}(bool),\n", replace_str, field_pascal);
				}
				FieldType::U8 | FieldType::VecU8 => {
					replace_str = format!("{}{}(u8),\n", replace_str, field_pascal);
				}
				FieldType::U16 | FieldType::VecU16 => {
					replace_str = format!("{}{}(u16),\n", replace_str, field_pascal);
				}
				FieldType::U32 | FieldType::VecU32 => {
					replace_str = format!("{}{}(u32),\n", replace_str, field_pascal);
				}
				FieldType::U64 | FieldType::VecU64 => {
					replace_str = format!("{}{}(u64),\n", replace_str, field_pascal);
				}
				FieldType::U128 | FieldType::VecU128 => {
					replace_str = format!("{}{}(u128),\n", replace_str, field_pascal);
				}
			}
		}
		*template = template.replace("${OPTIONS_ENUM_VARIANTS}", &replace_str);
		Ok(())
	}

	fn update_options_enum_names_match(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		let name = self.configurable_struct_name.as_ref().unwrap();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			replace_str = format!(
				"{}{}Options::{}(_) => \"{}\",\n",
				replace_str, name, field_pascal, field_pascal
			);
		}
		*template = template.replace("${OPTIONS_ENUM_NAMES_MATCH}", &replace_str);
		Ok(())
	}

	fn update_set_string(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_type = field.field_type.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			match field_type {
				FieldType::CString => {
					replace_str = format!(
						"{}if name == \"{}\" {{ self.{} = value.clone(); }}\n",
						replace_str, field_pascal, field_name
					);
				}
				FieldType::VecCString => {
					replace_str = format!(
						"{}if name == \"{}\" {{ self.{}.push(value.clone()); }}\n",
						replace_str, field_pascal, field_name
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${SET_STRING}", &replace_str);
		Ok(())
	}

	fn update_options_enum_string_match(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		let name = self.configurable_struct_name.as_ref().unwrap();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			match field_type {
				FieldType::CString | FieldType::VecCString => {
					replace_str = format!(
						"{}{}Options::{}(v) => Some(v.to_string()),\n",
						replace_str, name, field_pascal
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${OPTIONS_ENUM_VALUE_STRING_MATCH}", &replace_str);
		Ok(())
	}

	fn update_string_params_push(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_type = field.field_type.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			match field_type {
				FieldType::CString => {
					replace_str = format!(
						"{}ret.push((\"{}\".to_string(), self.{}.clone()));\n",
						replace_str, field_pascal, field_name
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${STRING_PARAMS_PUSH}", &replace_str);

		Ok(())
	}

	fn update_vec_string_params_push(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_type = field.field_type.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			match field_type {
				FieldType::VecCString => {
					replace_str = format!(
						"{}ret.push((\"{}\".to_string(), self.{}.clone()));\n",
						replace_str, field_pascal, field_name
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${VEC_STRING_PARAMS_PUSH}", &replace_str);
		Ok(())
	}

	fn update_set_configurable(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		let set_configurable_template =
			include_str!("../templates/config_set_configurable.template.txt").to_string();
		let set_configurable_vec_template =
			include_str!("../templates/config_set_configurable_arr.template.txt").to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();

			match field_type {
				FieldType::Configurable => {
					replace_str = format!("{}if name == \"{}\" {{\n", replace_str, field_pascal);

					let set_configurable =
						set_configurable_template.replace("${CONFIGURABLE_NAME}", field_name);
					replace_str = format!("{}{}\n}}\n", replace_str, set_configurable);
				}
				FieldType::VecConfigurable => {
					replace_str = format!("{}if name == \"{}\" {{\n", replace_str, field_pascal);
					let set_configurable =
						set_configurable_vec_template.replace("${CONFIGURABLE_NAME}", field_name);
					let set_configurable = set_configurable
						.replace("${CONFIGURABLE_TYPE}", field.type_str.as_ref().unwrap());
					replace_str = format!("{}{}\n}}\n", replace_str, set_configurable);
				}
				_ => {}
			}
		}
		*template = template.replace("${SET_CONFIGURABLE}", &replace_str);
		Ok(())
	}

	fn update_options_enum_configurable_match(
		&mut self,
		template: &mut String,
	) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		let name = self.configurable_struct_name.as_ref().unwrap();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			match field_type {
				FieldType::Configurable | FieldType::VecConfigurable => {
					replace_str = format!(
						"{}{}Options::{}(v) => Some(v.clone()),\n",
						replace_str, name, field_pascal
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${OPTIONS_ENUM_VALUE_CONFIGURABLE_MATCH}", &replace_str);
		Ok(())
	}

	fn update_configurable_params_push(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			match field_type {
				FieldType::Configurable => {
					replace_str = format!(
						"{}ret.push((\"{}\".to_string(), Box::new(self.{}.clone())));\n",
						replace_str, field_pascal, field_name
					);
				}
				_ => {}
			}
		}
		*template = template.replace("${CONFIGURABLE_PARAMS_PUSH}", &replace_str);

		Ok(())
	}

	fn update_vec_configurable_params_push(&mut self, template: &mut String) -> Result<(), Error> {
		let set_configurable_template =
			include_str!("../templates/config_vec_configurable_params_push.template.txt")
				.to_string();
		let mut replace_str = "".to_string();
		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_type = field.field_type.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type_str = field.type_str.as_ref().unwrap();
			match field_type {
				FieldType::VecConfigurable => {
					let replace_template =
						set_configurable_template.replace("${FIELD_NAME}", field_name);
					let replace_template =
						replace_template.replace("${FIELD_PASCAL}", &field_pascal);
					let replace_template =
						replace_template.replace("${FIELD_TYPE}", &field_type_str);
					replace_str = format!("{}{}", replace_str, replace_template);
				}
				_ => {}
			}
		}
		*template = template.replace("${VEC_CONFIGURABLE_PARAMS_PUSH}", &replace_str);
		Ok(())
	}

	fn update_dupes_inserts(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();

		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			let field_type = field.field_type.as_ref().unwrap();
			match field_type {
				FieldType::VecU8
				| FieldType::VecU16
				| FieldType::VecU32
				| FieldType::VecU64
				| FieldType::VecU128
				| FieldType::VecBool
				| FieldType::VecUsize
				| FieldType::VecCString
				| FieldType::VecConfigurable => {
					replace_str = format!(
						"{}ret.insert(\"{}\".to_string());\n",
						replace_str, field_pascal
					);
				}
				_ => {}
			}
		}

		*template = template.replace("${DUPES_INSERTS}", &replace_str);
		Ok(())
	}

	fn update_required_inserts(&mut self, template: &mut String) -> Result<(), Error> {
		let mut replace_str = "".to_string();

		for field in &self.fields {
			let field_name = field.name.as_ref().unwrap();
			let field_pascal = field_name.to_case(Case::Pascal);
			if field.required {
				replace_str = format!(
					"{}ret.push(\"{}\".to_string());\n",
					replace_str, field_pascal
				);
			}
		}

		*template = template.replace("${REQUIRED_INSERTS}", &replace_str);
		Ok(())
	}

	fn generate(&mut self) -> Result<(), Error> {
		let name = self.configurable_struct_name.as_ref().unwrap().clone();
		let mut template = include_str!("../templates/config.template.txt").to_string();
		self.update_enum_variants(&mut template)?;
		template = update_template!(template, usize, &self.fields, name);
		template = update_template!(template, u8, &self.fields, name);
		template = update_template!(template, u16, &self.fields, name);
		template = update_template!(template, u32, &self.fields, name);
		template = update_template!(template, u64, &self.fields, name);
		template = update_template!(template, u128, &self.fields, name);
		template = update_template!(template, bool, &self.fields, name);
		self.update_set_string(&mut template)?;
		self.update_set_configurable(&mut template)?;
		self.update_dupes_inserts(&mut template)?;
		self.update_required_inserts(&mut template)?;
		self.update_vec_string_params_push(&mut template)?;
		self.update_string_params_push(&mut template)?;
		self.update_vec_configurable_params_push(&mut template)?;
		self.update_configurable_params_push(&mut template)?;
		self.update_options_enum_names_match(&mut template)?;
		self.update_options_enum_string_match(&mut template)?;
		self.update_options_enum_configurable_match(&mut template)?;
		template = template.replace(
			"${CONFIGURABLE_STRUCT}",
			&self.configurable_struct_name.as_ref().unwrap(),
		);

		self.ret.extend(template.parse::<TokenStream>());
		//println!("self.ret='{}'", self.ret);

		Ok(())
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_configurable(strm: TokenStream) -> TokenStream {
	let mut state = StateMachine::new();
	match state.derive(&strm) {
		Ok(ret) => ret,
		Err(e) => {
			println!("parsing Configurable generated error: {}", e);
			strm
		}
	}
}
