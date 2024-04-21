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
	($kind:expr, $m:expr) => {{
                let error: Error = $kind($m.to_string()).into();
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
macro_rules! map_err {
	($in_err:expr, $kind:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}", e)).into() })
	}};
	($in_err:expr, $kind:expr, $m:expr) => {{
		$in_err.map_err(|e| -> Error { $kind(format!("{}: {}", $m, e)).into() })
	}};
}

#[macro_export]
macro_rules! kind {
	($kind:expr, $s:expr) => {{
		let r: Box<dyn ErrorKind> = Box::new($kind($s.to_string()));
		r
	}};
}

/// Macro to do a conditional break
#[macro_export]
macro_rules! cbreak {
	($cond:expr) => {{
		if $cond {
			break;
		}
	}};
}

/// Macro to map the try_from error into an appropriate error.
#[macro_export]
macro_rules! try_into {
	($v:expr) => {{
		use std::convert::TryInto;
		map_err!($v.try_into(), CoreErrorKind::TryInto)
	}};
}
