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
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_errorkind(item: TokenStream) -> TokenStream {
	match do_derive_errorkind_impl(&item) {
		Ok(stream) => stream,
		Err(e) => {
			println!(
				"WARNING: do_derive_errorkind generated error, cannot produce output: {}",
				e
			);
			item
		}
	}
}

fn do_derive_errorkind_impl(item: &TokenStream) -> Result<TokenStream, Error> {
	let mut ret = TokenStream::new();
	let mut expect_name = false;

	for token in item.clone() {
		match token {
			Ident(ident) => {
				if expect_name {
					build_impls(ident.to_string(), &mut ret)?;
					expect_name = false;
				} else if ident.to_string() == "enum" {
					expect_name = true;
				} else {
					expect_name = false;
				}
			}
			_ => {
				expect_name = false;
			}
		}
	}

	Ok(ret)
}

fn build_impls(name: String, strm: &mut TokenStream) -> Result<(), Error> {
	let impls = format!(
		"\n\timpl ErrorKind for {} {{ }}\n\
                \timpl From<{}> for Error {{\n\
                \t\tfn from(kind: {}) -> Error {{\n\
                \t\t\tlet kind: Box<dyn ErrorKind> = Box::new(kind);\n\
                \t\t\tError::new(kind)\n\
                \t\t}}\n\
                \t}}   ",
		name, name, name
	)
	.parse::<TokenStream>();
	let impls = map_err!(impls, BaseErrorKind::Parse)?;
	strm.extend(impls);
	Ok(())
}
