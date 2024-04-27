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
use proc_macro::{Delimiter, Group, Spacing, TokenStream, TokenTree::*};
use std::collections::HashMap;

#[cfg(not(tarpaulin_include))]
pub(crate) fn do_derive_document(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// try to make the document, if an internal error occurs print warning message below.
	match do_derive_document_impl(&item) {
		Ok(stream) => stream,
		Err(e) => {
			println!(
				"WARNING: do_derive_document generated error, cannot produce output: {}",
				e
			);
			item
		}
	}
}

#[cfg(not(tarpaulin_include))]
fn do_derive_document_impl(item: &TokenStream) -> Result<TokenStream, Error> {
	let mut last_is_hash = false;
	let mut last_joint = None;
	let mut omit = false;
	let mut in_signature = false;
	let mut non_comments = TokenStream::new();
	let mut signature = TokenStream::new();
	let mut parameter_list = TokenStream::new();
	let mut comment_vec = vec![];

	for token in item.clone() {
		match token {
			Ident(ref ident) => {
				let ident_str = ident.to_string();
				if ident_str == "fn" {
					in_signature = true;
				}
				last_is_hash = false;
			}
			Group(ref group) => {
				//println!("group={}", group);
				if group.delimiter() == Delimiter::Brace {
					in_signature = false;
				} else {
					if group.delimiter() == Delimiter::Parenthesis && in_signature {
						// parameter list
						parameter_list.extend(group.stream());
					}
					if last_is_hash {
						let mut first = true;
						for g in group.stream() {
							if first && g.to_string() == "doc" {
								omit = true;
							//println!("comment group = {:?}", group);
							} else if !first && !omit {
								break;
							}
							match g {
								Literal(l) => {
									// the comment
									let l = l.to_string();
									let len_decr = if l.len() > 0 { l.len() - 1 } else { 0 };
									let lf = l.find("\"");
									let lrf = l.rfind("\"");

									if lf == Some(0) && lrf == Some(len_decr) {
										let l = l.substring(1, len_decr).to_string();
										//println!("comment='{}'", l);
										comment_vec.push(l);
									}
								}
								_ => {}
							}
							first = false;
						}
					}
				}
				last_is_hash = false;
			}
			Punct(ref punct) => {
				if *punct == '#' {
					last_is_hash = true;
					omit = true;
				} else {
					last_is_hash = false;
				}

				if punct.spacing() == Spacing::Joint {
					last_joint = Some(punct.to_string());
					omit = true;
				}
			}
			Literal(ref literal) => {
				last_is_hash = false;
			}
		}

		if !omit {
			let token = match last_joint {
				Some(ref last) => format!("{}{}", last, token).parse::<TokenStream>(),
				None => token.to_string().parse::<TokenStream>(),
			};
			let token = map_err!(token, BaseErrorKind::Parse)?;
			last_joint = None;
			non_comments.extend(token.clone());

			if in_signature {
				signature.extend(token);
			}
		}

		omit = false;
	}
	Ok(build_docs(
		comment_vec,
		non_comments,
		signature,
		parameter_list,
	)?)
}

fn build_docs(
	comments: Vec<String>,
	non_comments: TokenStream,
	signature: TokenStream,
	param_list: TokenStream,
) -> Result<TokenStream, Error> {
	//println!("sig='{}'", signature);
	//println!("param_list='{}'", param_list);

	let mut pre_comments = vec![];
	let mut post_comments = vec![];
	let mut param_comments = HashMap::new();
	let mut last_is_param = false;
	let mut last_param_name = None;
	let mut in_post = false;
	for comment in comments {
		let comment_trim = comment.trim();
		let comment_trim_len = comment_trim.len();
		if in_post {
			post_comments.push(comment.clone());
		} else if comment_trim.find("@param ") == Some(0) {
			let param_comment = comment_trim.substring(7, comment_trim_len).to_string();
			match param_comment.find(" ") {
				Some(space) => {
					let plen = param_comment.len();
					if plen > space {
						let param_name = param_comment.substring(0, space);
						let param_value = param_comment.substring(space + 1, plen);
						let param_name = param_name.to_string();
						let param_value = param_value.to_string();
						last_param_name = Some(param_name.clone());
						last_is_param = true;
						match param_comments.get_mut(&param_name) {
							Some(param_value_pre) => {
								*param_value_pre = format!("{} {}", param_value_pre, param_value);
							}
							None => {
								param_comments.insert(param_name, param_value);
							}
						}
					}
				}
				None => {}
			}
		} else if comment.trim().find("# Example") == Some(0) {
			in_post = true;
			post_comments.push(comment.clone());
		} else if last_is_param {
			let last_param_name = last_param_name.as_ref().unwrap();
			match param_comments.get_mut(last_param_name) {
				Some(param_value_pre) => {
					*param_value_pre = format!("{} {}", param_value_pre, comment);
				}
				None => {
					param_comments.insert(last_param_name.to_string(), comment);
				}
			}
		} else {
			pre_comments.push(comment.clone());
		}
	}

	let mut ret = TokenStream::new();
	for comment in pre_comments {
		ret.extend(format!("/// {}", comment).parse::<TokenStream>());
	}
	ret.extend("/// # Input Parameters".parse::<TokenStream>());
	build_input_list(&mut ret, param_list, param_comments)?;
	ret.extend("/// # Return".parse::<TokenStream>());
	ret.extend("/// return type".parse::<TokenStream>());
	ret.extend("/// # Errors".parse::<TokenStream>());
	ret.extend("/// error list".parse::<TokenStream>());
	ret.extend("/// # Also See".parse::<TokenStream>());
	ret.extend("/// see list".parse::<TokenStream>());
	for comment in post_comments {
		ret.extend(format!("/// {}", comment).parse::<TokenStream>());
	}

	ret.extend(non_comments);
	Ok(ret)
}

fn build_input_list(
	ret: &mut TokenStream,
	param_list: TokenStream,
	param_comments: HashMap<String, String>,
) -> Result<(), Error> {
	let params = parse_param_list(param_list)?;
	println!("params={:?}", params);
	ret.extend("/// | Parameter | Type | Comment |".parse::<TokenStream>());
	ret.extend("/// |-----------|------|---------|".parse::<TokenStream>());
	let mut first = true;
	for param in params {
		let name = param.0.clone();
		/*
		let name = if first {
			if param.0 == "& mut self" || param.0 == "& self" {
				"self".to_string()
			} else {
				param.0.clone()
			}
		} else {
			param.0.clone()
		};
			*/
		let comment_name = if first {
			if param.0 == "& mut self" || param.0 == "& self" {
				"self".to_string()
			} else {
				param.0.clone()
			}
		} else {
			param.0.clone()
		};
		let comment = match param_comments.get(&comment_name) {
			Some(comment) => comment.clone(),
			None => format!("__TODO__: add '@param {} ...'", comment_name),
		};
		ret.extend(format!("/// | `{}` | {} | {}", name, param.1, comment).parse::<TokenStream>());
		first = false;
	}

	Ok(())
}

fn process_group(group: Group) -> Result<String, Error> {
	let token_str = group.to_string();
	if token_str == "()" {
		Ok("[`()`](unit)".to_string())
	} else {
		let delimiter = group.delimiter();
		if delimiter == Delimiter::Parenthesis {
			let mut ret = format!("");

			for token in group.stream() {
				let next = match token {
					Ident(ident) => {
						let mut ident_str = ident.to_string();
						if ident_str != "dyn" && ident_str != "mut" {
							ident_str = format!("[`{}`]", ident_str);
						}

						ident_str
					}
					Group(group) => process_group(group)?,
					_ => token.to_string(),
				};
				if ret.len() == 0 {
					ret = format!("({}", next);
				} else {
					ret = format!("{} {}", ret, next);
				}
			}

			// strip trailing commas
			let ret_len = ret.len();
			if ret_len > 0 && ret.rfind(",") == Some(ret_len - 1) {
				ret = ret.substring(0, ret.len() - 1).trim().to_string();
			}

			let ret = format!("{})", ret);
			//Ok(token_str)
			Ok(ret)
		} else {
			Ok(token_str)
		}
	}
}

fn parse_param_list(strm: TokenStream) -> Result<Vec<(String, String)>, Error> {
	let mut ret = vec![];
	let mut name: Option<String> = None;
	let mut value: Option<String> = None;
	let mut bracket_count = 0usize;
	for token in strm {
		let mut is_non_keyword = false;
		let token = match token {
			Ident(i) => {
				let i = i.to_string();
				if i != "dyn" && i != "mut" {
					is_non_keyword = true;
				}
				i
			}
			Punct(p) => {
				if p == '<' {
					bracket_count += 1;
				} else if p == '>' {
					bracket_count = bracket_count.saturating_sub(1);
				}
				println!("p={}", p);
				p.to_string()
			}
			Group(ref g) => process_group(g.clone())?,
			_ => {
				let token_str = token.to_string();
				println!("other={}", token_str);
				if token_str == "()" {
					"[`()`](unit)".to_string()
				} else {
					token_str
				}
			}
		};

		if token.rfind(",") == Some(token.len().saturating_sub(1)) && bracket_count == 0 {
			//println!("name={:?},value={:?}", name, value);
			let name_ret = match name {
				Some(name) => name,
				None => "".to_string(),
			};
			let value_ret = match value {
				Some(name) => name,
				None => "".to_string(),
			};
			ret.push((name_ret, value_ret));
			//println!("=======NEXT========");
			name = None;
			value = None;
		} else if token == ":" {
			value = Some("".to_string());
		} else {
			match value.as_mut() {
				Some(value) => {
					let formatted_token = if is_non_keyword {
						format!("[`{}`]", token)
					} else {
						token
					};
					if value.len() == 0 {
						*value = format!("{}", formatted_token);
					} else {
						*value = format!("{} {}", value, formatted_token);
					}
				}
				None => match name.as_mut() {
					Some(name) => {
						*name = format!("{} {}", name, token);
					}
					None => {
						name = Some(token);
					}
				},
			}
		}
	}
	Ok(ret)
}
