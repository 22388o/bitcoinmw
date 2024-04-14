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

#[cfg(test)]
mod test {
	use crate as bmw_conf2;
	use crate::*;
	use bmw_err::*;
	use std::collections::HashSet;

	//#[derive(Configurable)]
	struct MyConfig {
		//#[param(required)]
		v1: u8,
		v2: u16,
		//#[param(required)]
		v3: u32,
		v4: u8,
		v5: String,
		v6: Vec<String>,
		v7: Vec<(String, String)>,
	}

	impl Default for MyConfig {
		fn default() -> Self {
			let v1 = 0;
			let v2 = 0;
			let v3 = 20;
			let v4 = 30;
			let v5 = "".to_string();
			let v6 = vec![];
			let v7 = vec![];
			Self {
				v1,
				v2,
				v3,
				v4,
				v5,
				v6,
				v7,
			}
		}
	}

	// made by the Configrable proc macro
	impl MyConfig {
		fn new() -> Self {
			Self::default()
		}

		fn required() -> Vec<String> {
			vec!["v1".to_string(), "v3".to_string()]
		}
	}

	// made by the Configurable proc macro
	impl Configurable for MyConfig {
		fn set_u8(&mut self, name: &str, value: u8) {
			if name == "v1" {
				self.v1 = value;
			} else if name == "v4" {
				self.v4 = value;
			}
		}
		fn set_u16(&mut self, name: &str, value: u16) {
			if name == "v2" {
				self.v2 = value;
			}
		}
		fn set_u32(&mut self, name: &str, value: u32) {
			if name == "v3" {
				self.v3 = value;
			}
		}
		fn set_u64(&mut self, _name: &str, _value: u64) {}
		fn set_u128(&mut self, _name: &str, _value: u128) {}
		fn set_usize(&mut self, _name: &str, _value: usize) {}
		fn set_string(&mut self, name: &str, value: String) {
			if name == "v5" {
				self.v5 = value.clone();
			}
			if name == "v6" {
				self.v6.push(value);
			}
		}
		fn set_bool(&mut self, _name: &str, _value: bool) {}
		fn set_string_tuple(&mut self, name: &str, value: (String, String)) {
			if name == "v7" {
				self.v7.push(value);
			}
		}

		fn allow_dupes(&self) -> HashSet<String> {
			let mut ret = HashSet::new();
			ret.insert("v6".to_string());
			ret.insert("v7".to_string());
			ret
		}
	}

	// made by the Configurable proc macro
	#[derive(Debug)]
	#[allow(non_camel_case_types, dead_code)]
	pub enum MyConfig_Options {
		v1(u8),
		v2(u16),
		v3(u32),
		v4(u8),
		v5(String),
		v6(String),
		v7((String, String)),
	}

	// made by the Configurable proc macro
	#[allow(dead_code)]
	impl MyConfig_Options {
		fn name(&self) -> &str {
			match self {
				MyConfig_Options::v1(_) => "v1",
				MyConfig_Options::v2(_) => "v2",
				MyConfig_Options::v3(_) => "v3",
				MyConfig_Options::v4(_) => "v4",
				MyConfig_Options::v5(_) => "v5",
				MyConfig_Options::v6(_) => "v6",
				MyConfig_Options::v7(_) => "v7",
			}
		}

		fn value_u8(&self) -> Option<u8> {
			match self {
				MyConfig_Options::v1(v) => Some(*v),
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(v) => Some(*v),
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_u16(&self) -> Option<u16> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(v) => Some(*v),
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_u32(&self) -> Option<u32> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(v) => Some(*v),
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_u64(&self) -> Option<u64> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_u128(&self) -> Option<u128> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_usize(&self) -> Option<usize> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_string(&self) -> Option<String> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(v) => Some(v.clone()),
				MyConfig_Options::v6(v) => Some(v.clone()),
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_bool(&self) -> Option<bool> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(_v) => None,
			}
		}

		fn value_string_tuple(&self) -> Option<(String, String)> {
			match self {
				MyConfig_Options::v1(_v) => None,
				MyConfig_Options::v2(_v) => None,
				MyConfig_Options::v3(_v) => None,
				MyConfig_Options::v4(_v) => None,
				MyConfig_Options::v5(_v) => None,
				MyConfig_Options::v6(_v) => None,
				MyConfig_Options::v7(v) => Some(v.clone()),
			}
		}
	}

	#[test]
	fn test_multi_configs_same_struct() -> Result<(), Error> {
		let my_config = config!(MyConfig, MyConfig_Options, vec![v1(100), v3(3), v4(50)])?;

		assert_eq!(my_config.v1, 100);
		assert_eq!(my_config.v2, 0);
		assert_eq!(my_config.v3, 3);
		assert_eq!(my_config.v4, 50);

		// test a duplicate
		assert!(config!(MyConfig, MyConfig_Options, vec![v1(100), v3(3), v1(50)]).is_err());

		// required option v3 not specified
		assert!(config!(MyConfig, MyConfig_Options, vec![v1(100)]).is_err());

		let my_config = config!(
			MyConfig,
			MyConfig_Options,
			vec![v1(100), v3(3), v4(50), v5("test".to_string())]
		)?;

		assert_eq!(my_config.v1, 100);
		assert_eq!(my_config.v2, 0);
		assert_eq!(my_config.v3, 3);
		assert_eq!(my_config.v4, 50);
		assert_eq!(my_config.v5, "test".to_string());
		let empty: Vec<String> = vec![];
		assert_eq!(my_config.v6, empty);

		let my_config = config!(
			MyConfig,
			MyConfig_Options,
			vec![
				v1(100),
				v3(3),
				v4(50),
				v5("test".to_string()),
				v6("abc".to_string()),
				v6("def".to_string())
			]
		)?;

		assert_eq!(my_config.v6, vec!["abc".to_string(), "def".to_string()]);

		let my_config = config!(
			MyConfig,
			MyConfig_Options,
			vec![
				v7(("z".to_string(), "Z".to_string())),
				v1(10),
				v3(20),
				v7(("x".to_string(), "y".to_string()))
			]
		)?;

		assert_eq!(
			my_config.v7,
			vec![
				("z".to_string(), "Z".to_string()),
				("x".to_string(), "y".to_string())
			]
		);

		Ok(())
	}

	struct MyConfigStr {
		v1: String,
		v2: u8,
	}

	impl MyConfigStr {
		fn new() -> Self {
			Self::default()
		}

		fn required() -> Vec<String> {
			vec![]
		}
	}

	impl Default for MyConfigStr {
		fn default() -> Self {
			let v1 = "".to_string();
			let v2 = 0;
			Self { v1, v2 }
		}
	}

	#[allow(non_camel_case_types)]
	pub enum MyConfigStr_Options<'a> {
		v1(&'a str),
		v2(u8),
	}

	impl Configurable for MyConfigStr {
		fn set_u8(&mut self, name: &str, value: u8) {
			if name == "v2" {
				self.v2 = value;
			}
		}
		fn set_u16(&mut self, _name: &str, _value: u16) {}
		fn set_u32(&mut self, _name: &str, _value: u32) {}
		fn set_u64(&mut self, _name: &str, _value: u64) {}
		fn set_u128(&mut self, _name: &str, _value: u128) {}
		fn set_usize(&mut self, _name: &str, _value: usize) {}
		fn set_string(&mut self, name: &str, value: String) {
			if name == "v1" {
				self.v1 = value;
			}
		}
		fn set_bool(&mut self, _name: &str, _value: bool) {}
		fn set_string_tuple(&mut self, _name: &str, _value: (String, String)) {}
		fn allow_dupes(&self) -> HashSet<String> {
			HashSet::new()
		}
	}

	impl MyConfigStr_Options<'_> {
		fn name(&self) -> &str {
			match self {
				MyConfigStr_Options::v1(_) => "v1",
				MyConfigStr_Options::v2(_) => "v2",
			}
		}

		fn value_u8(&self) -> Option<u8> {
			match self {
				MyConfigStr_Options::v2(v) => Some(*v),
				_ => None,
			}
		}

		fn value_u16(&self) -> Option<u16> {
			None
		}

		fn value_u32(&self) -> Option<u32> {
			None
		}

		fn value_u64(&self) -> Option<u64> {
			None
		}

		fn value_u128(&self) -> Option<u128> {
			None
		}

		fn value_usize(&self) -> Option<usize> {
			None
		}

		fn value_string(&self) -> Option<String> {
			match self {
				MyConfigStr_Options::v1(v) => Some((*v).to_string()),
				_ => None,
			}
		}

		fn value_bool(&self) -> Option<bool> {
			None
		}

		fn value_string_tuple(&self) -> Option<(String, String)> {
			None
		}
	}

	#[test]
	fn test_config_str() -> Result<(), Error> {
		let myconf = config!(MyConfigStr, MyConfigStr_Options, vec![v2(55)])?;

		assert_eq!(myconf.v1, "".to_string());
		assert_eq!(myconf.v2, 55);

		let myconf = config!(MyConfigStr, MyConfigStr_Options, vec![v2(55), v1("test8")])?;

		assert_eq!(myconf.v1, "test8".to_string());
		assert_eq!(myconf.v2, 55);
		Ok(())
	}
}
