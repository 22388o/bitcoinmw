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

use crate::types::DeriveErrorKind::*;
use bmw_base::BaseErrorKind::*;
use bmw_base::*;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree, TokenTree::*};
use proc_macro_error::{abort, emit_error, Diagnostic, Level};

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

struct SpanError {
	span: Span,
	msg: String,
}

#[derive(PartialEq)]
enum Stage {
	Init,
	ClassGroup,
	ClassBlock,
	Complete,
}

struct State {
	stage: Stage,
	ret: TokenStream,
	span: Option<Span>,
	error_list: Vec<SpanError>,
}

impl State {
	fn new() -> Self {
		Self {
			ret: TokenStream::new(),
			stage: Stage::Init,
			span: None,
			error_list: vec![],
		}
	}

	fn derive(&mut self, tag: TokenStream) -> Result<TokenStream, Error> {
		match self.do_derive(tag) {
			Ok(_) => match self.stage {
				Stage::Complete => Ok(self.ret.clone()),
				_ => {
					err!(IllegalState, "class proc_macro unexpectedly terminated")
				}
			},
			Err(e) => {
				err!(
					IllegalState,
					"class proc_macro derive generated error: {}",
					e
				)
			}
		}
	}

	fn do_derive(&mut self, tag: TokenStream) -> Result<(), Error> {
		debug!("in do_derive_class")?;
		for token in tag {
			match self.process_token(token) {
				Ok(_) => {}
				Err(e) => {
					self.stage = Stage::Complete;
					self.process_abort(e.kind().to_string())?;
					break;
				}
			}
		}

		if self.error_list.len() > 0 {
			self.print_errors()
		} else {
			self.build()
		}
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

	fn process_abort(&self, msg: String) -> Result<(), Error> {
		match self.span {
			Some(span) => {
				let diag = Diagnostic::spanned(span.into(), Level::Error, msg.clone());
				abort!(diag, msg);
			}
			None => {}
		}
		Ok(())
	}

	fn build(&mut self) -> Result<(), Error> {
		Ok(())
	}

	fn process_token(&mut self, token: TokenTree) -> Result<(), Error> {
		self.span = Some(token.span());
		match self.stage {
			Stage::Init => self.process_init(token),
			Stage::ClassGroup => self.process_class_group(token),
			Stage::ClassBlock => self.process_class_block(token),
			Stage::Complete => err!(UnexpectedToken, "unexpected token after class definition"),
		}
	}

	fn process_class_group(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("ClassGroup token = {:?}", token)?;
		match token {
			Group(group) => {
				if group.delimiter() != Delimiter::Brace {
					self.append_error("unexpected token error: expected '{'")?;
				}
				self.stage = Stage::ClassBlock;
				for token in group.stream() {
					self.process_token(token)?;
				}
			}
			_ => {
				return err!(UnexpectedToken, "expected '{'");
			}
		}
		self.stage = Stage::Complete;
		Ok(())
	}

	fn process_class_block(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("class_block_token={:?}", token)?;
		match token {
			Punct(p) => {
				if p == '#' {
					self.append_error("Parse Error: unexecpted token '#'")?;
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn process_init(&mut self, token: TokenTree) -> Result<(), Error> {
		debug!("init token={:?}", token)?;
		if token.to_string() == "class" {
			self.append_error("Reserved Word Error: the name 'class' is reserved.")?;
		}
		self.stage = Stage::ClassGroup;
		Ok(())
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
}

pub(crate) fn do_derive_class(attr: TokenStream, _item: TokenStream) -> TokenStream {
	let mut state = State::new();
	match state.derive(attr) {
		Ok(strm) => strm,
		Err(e) => {
			let _ = debug!("Internal Error class proc_macro generated: {}", e);
			TokenStream::new()
		}
	}
}
