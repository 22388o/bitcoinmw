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

use crate::types::ConfMacroState as MacroState;
use bmw_err::{err, Error};
use proc_macro::TokenTree::*;
use proc_macro::{Group, TokenStream, TokenTree};

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
                        Err(err!(ErrKind::Log, "impossible logging error"))
                }
        }};
        ($line:expr, $($values:tt)*) => {{
                if DEBUG {
                        println!($line, $($values)*);
                }
                if true {
                        Ok(())
                } else {
                        Err(err!(ErrKind::Log, "impossible logging error"))
                }
        }};
}
macro_rules! error {
        ($line:expr, $($values:tt)*) => {{
                println!($line, $($values)*);
        }};
}

impl MacroState {
	fn new() -> Self {
		Self {
			count: 0,
			name: None,
			u8_configs: vec![],
			u16_configs: vec![],
			u32_configs: vec![],
			u64_configs: vec![],
			u128_configs: vec![],
			usize_configs: vec![],
			string_configs: vec![],
			bool_configs: vec![],
			string_tuple_configs: vec![],
		}
	}

	fn build_set_u8(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u8_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_u16(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u16_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_u32(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u32_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_u64(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u64_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_u128(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u128_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_usize(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.usize_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_string(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.string_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value.clone()); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value.clone(); }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_string_tuple(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.string_tuple_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value.clone()); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value.clone(); }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_set_bool(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.bool_configs {
			ret = format!(
				"{}{}",
				ret,
				if config.2 {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{}.push(value); }}",
						config.0, config.0
					)
				} else {
					format!(
						"\n\t\tif name == \"{}\" {{ self.{} = value; }}",
						config.0, config.0
					)
				}
			);
		}
		ret = format!("{}\n", ret);
		ret
	}

	fn build_value_u8(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.u8_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}

		ret
	}

	fn build_value_u16(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.u16_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}
		ret
	}

	fn build_value_u32(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.u32_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}
		ret
	}

	fn build_value_u64(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.u64_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}

		ret
	}

	fn build_value_u128(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.u128_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}

		ret
	}

	fn build_value_usize(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.usize_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}
		ret
	}

	fn build_value_string(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.string_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(v.to_string()),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}
		ret
	}

	fn build_value_bool(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.bool_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some(*v),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}

		ret
	}

	fn build_value_string_tuple(&self) -> String {
		let mut ret = "None".to_string();

		match &self.name {
			Some(name) => {
				for config in &self.string_tuple_configs {
					if ret == "None".to_string() {
						ret = "\n\t\tmatch self {".to_string();
					}
					ret = format!(
						"{}\n\t\t\t{}_Options::{}(v) => Some((v.0.to_string(), v.1.to_string())),",
						ret, name, config.0
					);
				}
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
				] {
					for config in config_vec {
						if ret == "None".to_string() {
							ret = "\n\t\tmatch self {".to_string();
						}
						ret = format!("{}\n\t\t\t{}_Options::{}(_v) => None,", ret, name, config.0);
					}
				}
			}
			None => {}
		}

		if ret != "None".to_string() {
			ret = format!("{} \n\t\t}}\n", ret);
		}
		ret
	}

	fn build_options_enum(&self) -> String {
		let mut ret = "".to_string();
		for config in &self.u8_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(u8),", config.0));
		}
		for config in &self.u16_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(u16),", config.0));
		}
		for config in &self.u32_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(u32),", config.0));
		}
		for config in &self.u64_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(u64),", config.0));
		}
		for config in &self.u128_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(u128),", config.0));
		}
		for config in &self.usize_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(usize),", config.0));
		}
		for config in &self.string_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(&'a str),", config.0));
		}
		for config in &self.bool_configs {
			ret = format!("{}{}", ret, format!("\n\t{}(bool),", config.0));
		}
		for config in &self.string_tuple_configs {
			ret = format!(
				"{}{}",
				ret,
				format!("\n\t{}((&'a str, &'a str)),", config.0)
			);
		}
		ret
	}

	fn build_name_fn(&self) -> String {
		let mut ret = "\n\t\tmatch self {".to_string();
		match &self.name {
			Some(name) => {
				let mut total = 0;

				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						total += 1;
						let n = format!("{}_Options::{}(_) => \"{}\",", name, config.0, config.0);
						ret = format!("{}\n\t\t\t{}", ret, n);
					}
				}

				ret = format!("{}\n\t\t}}\n", ret);

				// in case there are no actual configurations
				if total == 0 {
					"\"\"".to_string()
				} else {
					ret
				}
			}
			None => "\"\"".to_string(),
		}
	}

	fn build_required(&self) -> String {
		let mut ret = "".to_string();
		match &self.name {
			Some(_name) => {
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						if config.1 {
							ret = format!("{}\n\t\t\t\"{}\".to_string(),", ret, config.0);
						}
					}
				}
			}
			None => {}
		}

		ret
	}

	fn build_allow_dupes(&self) -> String {
		let mut ret = "\n\t\tlet mut d = std::collections::HashSet::new();".to_string();
		match &self.name {
			Some(_name) => {
				for config_vec in vec![
					&self.u8_configs,
					&self.u16_configs,
					&self.u32_configs,
					&self.u64_configs,
					&self.u128_configs,
					&self.usize_configs,
					&self.string_configs,
					&self.bool_configs,
					&self.string_tuple_configs,
				] {
					for config in config_vec {
						ret = format!(
							"{}{}",
							ret,
							if config.2 {
								format!("\n\t\td.insert(\"{}\".to_string());", config.0)
							} else {
								format!("")
							}
						);
					}
				}
			}
			None => {}
		}

		ret = format!("{}\n\t\td\n", ret);

		ret
	}

	fn anon_lifetime(&self) -> String {
		if self.string_configs.len() > 0 {
			"<'_>".to_string()
		} else {
			"".to_string()
		}
	}

	fn named_lifetime(&self) -> String {
		if self.string_configs.len() > 0 {
			"<'a>".to_string()
		} else {
			"".to_string()
		}
	}

	fn ret(&self) -> String {
		match &self.name {
			Some(name) => format!(
				"\n\
                                impl {} {{\n\
                                \tfn new() -> Self {{ Self::default() }}\n\
				\tfn required() -> Vec<String> {{\n\
					\t\tvec![{}\n\t\t]\n\
				\t}}\n\
			}}\n\
			\n\
			enum {}_Options {} {{ {}\n}}\n\
			\n\
			impl Configurable for {} {{\n\
			\n\
				\tfn set_u8(&mut self, name: &str, value: u8) {{ {}\t}}\n\
				\tfn set_u16(&mut self, name: &str, value: u16) {{ {}\t}}\n\
				\tfn set_u32(&mut self, name: &str, value: u32) {{ {}\t}}\n\
				\tfn set_u64(&mut self, name: &str, value: u64) {{ {}\t}}\n\
				\tfn set_u128(&mut self, name: &str, value: u128) {{ {}\t}}\n\
				\tfn set_usize(&mut self, name: &str, value: usize) {{ {}\t}}\n\
				\tfn set_string(&mut self, name: &str, value: String) {{ {}\t}}\n\
				\tfn set_string_tuple(&mut self, name: &str, value: (String, String)) {{ {}\t}}\n\
				\tfn set_bool(&mut self, name: &str, value: bool) {{ {}\t}}\n\
				\tfn allow_dupes(&self) -> std::collections::HashSet<String> {{ {}\t}}\n\
			}}\n\
			\n\
		        impl {}_Options {} {{\n\
			        \tfn name(&self) -> &str {{ {}\t}}\n\
                                \tfn value_u8(&self) -> Option<u8> {{ {}\t}}\n\
                                \tfn value_u16(&self) -> Option<u16> {{ {}\t}}\n\
                                \tfn value_u32(&self) -> Option<u32> {{ {}\t}}\n\
                                \tfn value_u64(&self) -> Option<u64> {{ {}\t}}\n\
                                \tfn value_u128(&self) -> Option<u128> {{ {}\t}}\n\
                                \tfn value_usize(&self) -> Option<usize> {{ {}\t}}\n\
                                \tfn value_string(&self) -> Option<String> {{ {}\t}}\n\
                                \tfn value_bool(&self) -> Option<bool> {{ {}\t}}\n\
                                \tfn value_string_tuple(&self) -> Option<(String, String)> {{ {}\t}}\n\
			}}\n\
			",
				name,
                                self.build_required(),
				name,
                                self.named_lifetime(),
                                self.build_options_enum(),
                                name,
				self.build_set_u8(),
                                self.build_set_u16(),
                                self.build_set_u32(),
                                self.build_set_u64(),
                                self.build_set_u128(),
                                self.build_set_usize(),
                                self.build_set_string(),
                                self.build_set_string_tuple(),
                                self.build_set_bool(),
                                self.build_allow_dupes(),
				name,
                                self.anon_lifetime(),
                                self.build_name_fn(),
                                self.build_value_u8(),
                                self.build_value_u16(),
                                self.build_value_u32(),
                                self.build_value_u64(),
                                self.build_value_u128(),
                                self.build_value_usize(),
                                self.build_value_string(),
                                self.build_value_bool(),
                                self.build_value_string_tuple(),
			),
			None => "".to_string(),
		}
	}
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_configurable(strm: TokenStream) -> TokenStream {
	let mut state = MacroState::new();
	let _ = debug!("begin derive conf");
	match process_strm(strm, &mut state) {
		Ok(_) => {
			let ret = state.ret();
			let _ = debug!("ret='{}'", ret);
			ret.parse().unwrap()
		}
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
			let ident_str = ident.to_string();
			debug!("ident[{}]={}", state.count, ident_str)?;
			// name
			if state.count >= 1
				&& ident_str != "struct"
				&& ident_str != "pub"
				&& ident_str != "enum"
			{
				state.name = Some(ident_str);
			}
		}
		Group(group) => {
			debug!("group={}", group)?;
			process_group(group, state)?;
		}
		Literal(literal) => {
			debug!("literal={}", literal)?;
		}
		Punct(punct) => {
			debug!("punct={}", punct)?;
		}
	}

	state.count += 1;
	Ok(())
}

fn process_group(group: Group, state: &mut MacroState) -> Result<(), Error> {
	let mut last_name: Option<(String, bool)> = None;
	let mut required = false;
	let mut in_vec = false;
	for item in group.stream() {
		match item {
			Ident(ref ident) => {
				let ident_str = ident.to_string();
				debug!("ident: {}", ident_str)?;
				if ident_str != "pub"
					&& ident_str != "u8" && ident_str != "u16"
					&& ident_str != "u32"
					&& ident_str != "u64"
					&& ident_str != "u128"
					&& ident_str != "usize"
					&& ident_str != "String"
					&& ident_str != "bool"
					&& ident_str != "Vec"
				{
					debug!("name: {}", ident)?;
					last_name = Some((ident_str.clone(), required));
					required = false;
				}

				if ident_str == "u8" {
					match last_name {
						Some(ref v) => state.u8_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "u16" {
					match last_name {
						Some(ref v) => state.u16_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "u32" {
					match last_name {
						Some(ref v) => state.u32_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "u64" {
					match last_name {
						Some(ref v) => state.u64_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "u128" {
					match last_name {
						Some(ref v) => state.u128_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "usize" {
					match last_name {
						Some(ref v) => state.usize_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "String" {
					match last_name {
						Some(ref v) => state.string_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "bool" {
					match last_name {
						Some(ref v) => state.bool_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
				if ident_str == "Vec" {
					in_vec = true;
				}
			}
			_ => {
				debug!("other={}", item)?;
				let item_str = item.to_string();
				if item_str == "[required]" {
					debug!("found a required")?;
					required = true;
				}
				if item_str == ">" {
					in_vec = false;
				}
				if item_str == "(String, String)" {
					match last_name {
						Some(ref v) => state.string_tuple_configs.push((v.0.clone(), v.1, in_vec)),
						None => {}
					}
				}
			}
		}
	}
	Ok(())
}
