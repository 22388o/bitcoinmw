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
use bmw_deps::substring::Substring;
use proc_macro::TokenStream;
use proc_macro::TokenTree::*;

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_errorkind(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
	let mut name = "".to_string();

	ret.extend(
		"use bmw_deps::failure::Fail;\nuse bmw_deps::failure;#[derive(Debug, Fail)]"
			.parse::<TokenStream>(),
	);

	for token in item.clone() {
		let mut extended = false;
		let mut last_doc: Option<String> = None;
		match token {
			Ident(_) => {
				ret.extend(token.to_string().parse::<TokenStream>());
				extended = true;
			}
			Group(ref g) => {
				let mut extension = "{".to_string();
				for g in g.stream() {
					let ginner = g.to_string();
					if ginner.find("[doc") == Some(0) {
						match ginner.find("\"") {
							Some(start) => match ginner.rfind("\"") {
								Some(end) => {
									if end > start + 2 {
										let d = ginner.substring(start + 2, end);
										last_doc = Some(d.to_string());
									}
								}
								None => {}
							},
							None => {}
						}
					}
					match g {
						Ident(g) => {
							extension = format!(
								"{}#[fail(display = \"{}: {{}}\", _0)]",
								extension,
								match last_doc {
									Some(last_doc) => last_doc,
									None => g.to_string(),
								}
							);
							extension = format!("{} {}(String)", extension, g.to_string());
							last_doc = None;
						}
						Punct(g) => {
							extension = format!("{} {}", extension, g.to_string());
						}
						_ => extension = format!("{} {}", extension, g.to_string()),
					}
				}
				extension = format!("{}}}", extension);
				ret.extend(extension.parse::<TokenStream>());
				extended = true;
			}
			_ => {}
		}

		if !extended {
			ret.extend(token.to_string().parse::<TokenStream>());
		}

		match token {
			Ident(ident) => {
				if expect_name {
					name = ident.to_string();
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

	build_impls(name, &mut ret)?;

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
