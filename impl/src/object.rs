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

use crate::types::ObjectMacroState as MacroState;
use bmw_err::*;
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;

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

impl MacroState {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
		}
	}
}

pub(crate) fn do_derive_object(attr: TokenStream, item: TokenStream) -> TokenStream {
	match do_derive_object_impl(attr, item.clone()) {
		Ok(item) => item,
		Err(e) => {
			println!("ERROR: object proc_macro_attribute generated error: {}", e);
			TokenStream::new()
		}
	}
}

fn do_derive_object_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
	debug!("in do_derive_object_impl")?;
	let mut state = MacroState::new();

	debug!("=====================================attr======================================")?;
	for token in attr {
		match token {
			Ident(ident) => {
				debug!("ident={:?}", ident)?;
			}
			Group(group) => {
				debug!("group={:?}", group)?;
			}
			Literal(literal) => {
				debug!("literal={:?}", literal)?;
			}
			Punct(punct) => {
				debug!("punct={:?}", punct)?;
			}
		}
	}

	debug!("======================================item======================================")?;

	for token in item {
		match token {
			Ident(ident) => {
				debug!("ident={:?}", ident)?;
			}
			Group(group) => {
				debug!("group={:?}", group)?;
			}
			Literal(literal) => {
				debug!("literal={:?}", literal)?;
			}
			Punct(punct) => {
				debug!("punct={:?}", punct)?;
			}
		}
	}

	debug!("state.ret='{}'", state.ret)?;

	Ok(state.ret)
}
