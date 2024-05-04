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

#[macro_export]
macro_rules! err {
        ($kind:expr, $msg:expr, $($param:tt)*) => {{
                let msg = &format!($msg, $($param)*)[..];
                let error: Error = $kind(msg.to_string()).into();
                Err(error)
        }};
        ($kind:expr, $msg:expr) => {{
                let error: Error = $kind($msg.to_string()).into();
                Err(error)
        }};
}

#[macro_export]
macro_rules! err_only {
        ($kind:expr, $msg:expr, $($param:tt)*) => {{
                let msg = &format!($msg, $($param)*)[..];
                let error: Error = $kind(msg.to_string()).into();
                error
        }};
        ($kind:expr, $m:expr) => {{
                let error: Error = $kind($m.to_string()).into();
                error
        }};
}

#[macro_export]
macro_rules! kind {
	($kind:expr, $msg:expr) => {{
		let r: Box<dyn ErrorKind> = Box::new($kind($msg.to_string()));
		r
	}};
	($kind:expr) => {{
		kind!($kind, "")
	}};
}

#[macro_export]
macro_rules! map_err {
	($in_err:expr, $kind:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}", e)).into() })
	}};
	($in_err:expr, $kind:expr, $msg:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}: {}", $msg, e)).into() })
	}};
}

#[macro_export]
macro_rules! ret_err {
        ($kind:expr, $msg:expr, $($param:tt)*) => {{
                let msg = &format!($msg, $($param)*)[..];
                let error: Error = $kind(msg.to_string()).into();
                return Err(error)
        }};
        ($kind:expr, $msg:expr) => {{
                let error: Error = $kind($msg.to_string()).into();
                return Err(error)
        }};
}

#[macro_export]
macro_rules! cbreak {
	($cond:expr) => {{
		if $cond {
			break;
		}
	}};
}

#[macro_export]
macro_rules! try_into {
	($value:expr) => {{
		use std::convert::TryInto;
		map_err!($value.try_into(), CoreErrorKind::TryInto)
	}};
}

#[macro_export]
macro_rules! configure_box {
	( $configurable:ident, $enum_name:ident, $vec:expr ) => {{
		match configure!($configurable, $enum_name, $vec) {
			Ok(res) => Ok(Box::new(res)),
			Err(e) => Err(e),
		}
	}};
}

#[macro_export]
macro_rules! configure {
	( $configurable:ident, $enum_name:ident, $vec:expr ) => {{
		use bmw_base::*;
		use std::collections::HashSet;
		use $enum_name::*;

		let mut ret = $configurable::default();

		let mut name_set: HashSet<String> = HashSet::new();
		let mut err = None;
		let options: Vec<$enum_name> = $vec;

		for cfg in options {
			let name = cfg.name();
			if name_set.contains(name.clone()) && !ret.allow_dupes().contains(name.clone()) {
				let text = format!("config option ({}) was specified more than once", name);
				err = Some(err!(CoreErrorKind::Configuration, text));
			}
			name_set.insert(name.to_string());
			match cfg.value_u8() {
				Some(value) => ret.set_u8(name, value),
				None => {}
			}
			match cfg.value_u16() {
				Some(value) => ret.set_u16(name, value),
				None => {}
			}
			match cfg.value_u32() {
				Some(value) => ret.set_u32(name, value),
				None => {}
			}
			match cfg.value_u64() {
				Some(value) => ret.set_u64(name, value),
				None => {}
			}
			match cfg.value_u128() {
				Some(value) => ret.set_u128(name, value),
				None => {}
			}
			match cfg.value_usize() {
				Some(value) => ret.set_usize(name, value),
				None => {}
			}
			match cfg.value_string() {
				Some(value) => ret.set_string(name, value),
				None => {}
			}
			match cfg.value_bool() {
				Some(value) => ret.set_bool(name, value),
				None => {}
			}
			match cfg.value_configurable() {
				Some(value) => ret.set_configurable(name, &*value),
				None => {}
			}
		}

		for r in ret.required() {
			if !name_set.contains(&r) {
				let text = format!("required option ({}) was not specified", r);
				err = Some(err!(CoreErrorKind::Configuration, text));
			}
		}

		match err {
			Some(e) => e,
			None => Ok(ret),
		}
	}};
}
