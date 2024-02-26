// Copyright (c) 2023, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_log::*;

info!();

#[macro_export]
macro_rules! http_client_init {
	( $( $config:expr ),* ) => {{
                let mut config = bmw_http::HttpClientConfig::default();
                let mut threads_specified = false;
                let mut max_handles_per_thread_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.threads == 0 { config.threads = 0; }
                if error.is_some() { error = None; }
                if threads_specified { threads_specified = false; }
                if max_handles_per_thread_specified { max_handles_per_thread_specified = false; }
                if threads_specified {}
                if max_handles_per_thread_specified {}

                $(
                        match $config {
                                bmw_http::ConfigOption::Threads(threads) => {
                                        config.threads = threads;

                                        if threads_specified {
                                                error = Some("Threads was specified more than once!".to_string());
                                        }
                                        threads_specified = true;
                                        if threads_specified {}

                                },
                                bmw_http::ConfigOption::MaxHandlesPerThread(mhpt) => {
                                        config.max_handles_per_thread = mhpt;

                                        if max_handles_per_thread_specified {
                                                error = Some("MaxHandlesPerThread was specified more than once!".to_string());
                                        }

                                        max_handles_per_thread_specified = true;
                                        if max_handles_per_thread_specified {}
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for http_client", $config));
                                }
                        }
                )*

                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => {
                                bmw_http::HttpClientContainer::init(&config)
                        }
                }
        }};
}

#[macro_export]
macro_rules! http_client_request {
        () => {{
                bmw_http::HTTP_CLIENT_CONTEXT.with(|f| match &(*f.borrow()) {
                        Some((request, _response)) => Ok(request.clone()),
                        None => Err(err!(
                                ErrKind::IllegalState,
                                "Could not find HttpRequest given the current context"
                        )),
                })
        }};
	( $( $config:expr ),* ) => {{
                let mut config = bmw_http::HttpRequestConfig::default();
                let mut request_url_specified = false;
                let mut request_uri_specified = false;
                let mut user_agent_specified = false;
                let mut accept_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.user_agent == "" { config.user_agent = "".to_string(); }
                if error.is_some() { error = None; }
                if request_url_specified { request_url_specified = false; }
                if request_uri_specified { request_uri_specified = false; }
                if user_agent_specified { user_agent_specified = false; }
                if accept_specified { accept_specified = false; }
                if request_url_specified {}
                if request_uri_specified {}
                if user_agent_specified {}
                if accept_specified {}

                $(
                        match $config {
                                bmw_http::ConfigOption::Url(url) => {
                                        config.request_url = Some(url.to_string());

                                        if request_url_specified {
                                                error = Some("Url was specified more than once!".to_string());
                                        }
                                        request_url_specified = true;
                                        if request_url_specified {}

                                },
                                bmw_http::ConfigOption::Uri(uri) => {
                                        config.request_uri = Some(uri.to_string());

                                        if request_uri_specified {
                                                error = Some("Uri was specified more than once!".to_string());
                                        }

                                        request_uri_specified = true;
                                        if request_uri_specified {}
                                },
                                bmw_http::ConfigOption::UserAgent(user_agent) => {
                                        config.user_agent = user_agent.to_string();

                                        if user_agent_specified {
                                                error = Some("UserAgent was specified more than once!".to_string());
                                        }

                                        user_agent_specified = true;
                                        if user_agent_specified {}
                                },
                                bmw_http::ConfigOption::Accept(accept) => {
                                        config.accept = accept.to_string();

                                        if accept_specified {
                                                error = Some("Accept was specified more than once!".to_string());
                                        }

                                        accept_specified = true;
                                        if accept_specified {}
                                },
                                bmw_http::ConfigOption::Header((header_name, header_value)) => {
                                        config.headers.push((header_name.to_string(), header_value.to_string()));
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for http_client", $config));
                                }
                        }
                )*

                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => {
                                bmw_http::Builder::build_http_request(&config)
                        }
                }
        }};
}

#[macro_export]
macro_rules! http_client_response {
	() => {{
		bmw_http::HTTP_CLIENT_CONTEXT.with(|f| match &(*f.borrow()) {
			Some((_request, response)) => Ok(response.clone()),
			None => Err(err!(
				ErrKind::IllegalState,
				"Could not find HttpResponse given the current context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! http_client_send {
	($requests:expr, $handler:expr) => {{
		match bmw_http::HTTP_CLIENT_CONTAINER.write() {
			Ok(mut container) => {
				match (*container).get_mut(&std::thread::current().id()) {
					Some(http_client) => {
						let handler = Box::pin(
							move |request: &Box<dyn HttpRequest + Send + Sync>,
							      response: &Box<dyn HttpResponse + Send + Sync>| {
								bmw_http::HTTP_CLIENT_CONTEXT.with(|f| {
									*f.borrow_mut() =
										Some(((*request).clone(), (*response).clone()));
								});
								{
									$handler
								}
							},
						);

						for request in $requests {
							http_client.send(request, handler.clone())?;
						}
					}
					None => {}
				}
				Ok(())
			}
			Err(e) => Err(err!(
				ErrKind::IllegalState,
				format!(
					"Could not obtain write lock on HTTP_CLIENT_CONTAINER due to: {}",
					e
				)
			)),
		}
	}};
}

#[cfg(test)]
mod test {
	use crate as bmw_http;
	use crate::{Builder, HttpConfig, HttpInstance, HttpInstanceType, PlainConfig};
	use bmw_err::*;
	use bmw_http::*;
	use bmw_log::*;
	use bmw_test::*;
	use bmw_util::*;
	use std::collections::HashMap;
	use std::fs::File;
	use std::io::Write;
	use std::thread::sleep;
	use std::time::Duration;

	info!();

	#[test]
	fn test_http_macros_basic() -> Result<(), Error> {
		let test_dir = ".test_http_macros_basic.bmw";
		let data_text = "Hello Macro World!";
		setup_test_dir(test_dir)?;
		{
			let mut file = File::create(format!("{}/foo.html", test_dir))?;
			file.write_all(data_text.as_bytes())?;
		}

		let port = pick_free_port()?;
		info!("port={}", port)?;
		let addr = "127.0.0.1".to_string();

		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				addr: addr.clone(),
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		// begin macros
		http_client_init!()?;
		let request1 = http_client_request!(
			Url(&format!("http://{}:{}/foo.html", addr, port)),
			Header(("some", "thing")),
			Header(("another", "header"))
		)?;

		let request2 = http_client_request!(Url(&format!("http://{}:{}/foo2.html", addr, port)))?;

		let guid1 = request1.guid();
		let guid2 = request2.guid();
		info!("request1.request_url = {:?}", request1.request_url())?;
		info!("headers = {:?}", request1.headers())?;

		let mut data = lock_box!(0)?;
		let data_clone = data.clone();

		http_client_send!([request1, request2], {
			let response = http_client_response!()?;
			let request = http_client_request!()?;

			if guid1 == request.guid() {
				assert_eq!(response.code().unwrap_or(u16::MAX), 200);
				assert_eq!(
					std::str::from_utf8(response.content()?).unwrap_or("utf8err"),
					data_text.to_string()
				);
			} else if guid2 == request.guid() {
				assert_eq!(response.code().unwrap_or(u16::MAX), 404);
			} else {
				// should not get here
				return Ok(());
			}

			let mut data = data.wlock()?;
			let guard = data.guard();

			(**guard) += 1;

			info!(
				"request complete! code={}, guid = {}, guid1 = {}, guid2 = {}",
				response.code().unwrap_or(u16::MAX),
				request.guid(),
				guid1,
				guid2,
			)?;
			Ok(())
		})?;

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			count += 1;

			let data = data_clone.rlock()?;
			let guard = data.guard();

			if count < 30_000 && (**guard) != 2 {
				continue;
			}

			assert_eq!((**guard), 2);
			break;
		}

		tear_down_test_dir(test_dir)?;
		Ok(())
	}
}
