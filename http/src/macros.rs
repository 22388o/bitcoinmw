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
                let mut base_dir_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.threads == 0 { config.threads = 0; }
                if error.is_some() { error = None; }
                if threads_specified { threads_specified = false; }
                if max_handles_per_thread_specified { max_handles_per_thread_specified = false; }
                if base_dir_specified { base_dir_specified = false; }
                if threads_specified {}
                if max_handles_per_thread_specified {}
                if base_dir_specified {}

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
                                bmw_http::ConfigOption::BaseDir(base_dir) => {
                                        config.base_dir = base_dir.to_string();

                                        if base_dir_specified {
                                                error = Some("BaseDir was specified more than once!".to_string());
                                        }

                                        base_dir_specified = true;
                                        if base_dir_specified {}
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for http_client_init", $config));
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
                let mut timeout_millis_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.user_agent == "" { config.user_agent = "".to_string(); }
                if error.is_some() { error = None; }
                if request_url_specified { request_url_specified = false; }
                if request_uri_specified { request_uri_specified = false; }
                if user_agent_specified { user_agent_specified = false; }
                if accept_specified { accept_specified = false; }
                if timeout_millis_specified { timeout_millis_specified = false; }
                if request_url_specified {}
                if request_uri_specified {}
                if user_agent_specified {}
                if accept_specified {}
                if timeout_millis_specified {}

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
                                bmw_http::ConfigOption::TimeoutMillis(millis) => {
                                        config.timeout_millis = millis;

                                        if timeout_millis_specified {

                                        }

                                        timeout_millis_specified = true;
                                        if timeout_millis_specified {}
                                },
                                bmw_http::ConfigOption::Header((header_name, header_value)) => {
                                        config.headers.push((header_name.to_string(), header_value.to_string()));
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for http_client_request", $config));
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
	($request:expr) => {{
		match bmw_http::HTTP_CLIENT_CONTAINER.write() {
			Ok(mut container) => match (*container).get_mut(&std::thread::current().id()) {
				Some(http_client) => {
					let (tx, rx) = std::sync::mpsc::sync_channel(1);
					let tx_clone = tx.clone();
					let handler = Box::pin(
						move |_request: &Box<dyn HttpRequest + Send + Sync>,
						      response: &Box<dyn HttpResponse + Send + Sync>| {
							let res: Result<Box<dyn HttpResponse + Send + Sync>, bmw_err::Error> =
								Ok(response.clone());
							match tx.send(res) {
								Ok(_) => {}
								Err(_) => {
									// this probably means a
									// timeout occurred
								}
							}
							Ok(())
						},
					);

					let timeout_millis = $request.timeout_millis();
					if timeout_millis > 0 {
						std::thread::spawn(move || {
							let tx = tx_clone.clone();
							std::thread::sleep(Duration::from_millis(timeout_millis));
							match tx.send(Err(err!(
								ErrKind::IO,
								format!("timeout error: {} milliseconds expired", timeout_millis)
							))) {
								Ok(_) => {}
								Err(_) => {
									// this is ok because it means
									// the channel is closed
									// already due to a success
								}
							}
						});
					}

					match http_client.send($request, handler.clone()) {
						Ok(_) => match rx.recv() {
							Ok(response) => match response {
								Ok(response) => Ok(response),
								Err(e) => Err(bmw_err::err!(
									bmw_err::ErrKind::IO,
									format!("timeout error: {}", e)
								)),
							},
							Err(e) => Err(bmw_err::err!(
								bmw_err::ErrKind::IO,
								format!("recv_error: {}", e)
							)),
						},
						Err(e) => Err(bmw_err::err!(
							bmw_err::ErrKind::IO,
							format!("http_client send failed: {}", e)
						)),
					}
				}
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					"http_client not initialized"
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!(
					"Could not obtain write lock on HTTP_CLIENT_CONTAINER due to: {}",
					e
				)
			)),
		}
	}};
	($requests:expr, $handler:expr) => {{
		match bmw_http::HTTP_CLIENT_CONTAINER.write() {
			Ok(mut container) => {
				let mut err_vec = vec![];
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
							match http_client.send(request, handler.clone()) {
								Ok(_) => {}
								Err(e) => {
									err_vec.push(e);
								}
							}
						}
					}
					None => {
						return Err(bmw_err::err!(
							bmw_err::ErrKind::IllegalState,
							"http_client not initialized"
						))
					}
				}
				if err_vec.len() > 0 {
					let mut err_str = "The following errors occurred trying to send: ".to_string();
					for err in err_vec {
						err_str = format!("{}, {}", err_str, err);
					}
					Err(bmw_err::err!(bmw_err::ErrKind::Http, err_str))
				} else {
					Ok(())
				}
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
	($requests:expr, $connection:expr, $handler:expr) => {{
		let handler = Box::pin(
			move |request: &Box<dyn HttpRequest + Send + Sync>,
			      response: &Box<dyn HttpResponse + Send + Sync>| {
				bmw_http::HTTP_CLIENT_CONTEXT.with(|f| {
					*f.borrow_mut() = Some(((*request).clone(), (*response).clone()));
				});
				{
					$handler
				}
			},
		);

		let mut err_vec = vec![];
		for request in $requests {
			match $connection.send(request, handler.clone()) {
				Ok(_) => {}
				Err(e) => {
					err_vec.push(e);
				}
			}
		}

		if err_vec.len() > 0 {
			let mut err_str = "The following errors occurred trying to send: ".to_string();
			for err in err_vec {
				err_str = format!("{}, {}", err_str, err);
			}
			Err(bmw_err::err!(bmw_err::ErrKind::Http, err_str))
		} else {
			Ok(())
		}
	}};
}

#[macro_export]
macro_rules! http_connection {
        ( $( $config:expr ),* ) => {{
                let mut config = bmw_http::HttpConnectionConfig::default();
                let mut host_specified = false;
                let mut port_specified = false;
                let mut tls_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.port == 0 { config.port = 0; }
                if error.is_some() { error = None; }
                if host_specified { host_specified = false; }
                if port_specified { port_specified = false; }
                if tls_specified { tls_specified = false; }
                if host_specified {}
                if port_specified {}
                if tls_specified {}

                $(
                        match $config {
                                bmw_http::ConfigOption::Host(host) => {
                                        config.host = host.to_string();

                                        if host_specified {
                                                error = Some("Host was specified more than once!".to_string());
                                        }
                                        host_specified = true;
                                        if host_specified {}

                                },
                                bmw_http::ConfigOption::Port(port) => {
                                        config.port = port;

                                        if port_specified {
                                                error = Some("Port was specified more than once!".to_string());
                                        }

                                        port_specified = true;
                                        if port_specified {}
                                },
                                bmw_http::ConfigOption::Tls(tls) => {
                                        config.tls = tls;

                                        if tls_specified {
                                                error = Some("TLS was specified more than once!".to_string());
                                        }

                                        tls_specified = true;
                                        if tls_specified {}
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for http_connection", $config));
                                }
                        }
                )*

                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => {
                                match bmw_http::HTTP_CLIENT_CONTAINER.write() {
                                        Ok(mut container) => {
                                                match (*container).get_mut(&std::thread::current().id()) {
                                                        Some(http_client) => {
                                                                bmw_http::Builder::build_http_connection(&config, http_client.clone())
                                                        }
                                                        None => Err(bmw_err::err!(bmw_err::ErrKind::IllegalState, "no http_client found for this thread")),
                                                }
                                        }
                                        Err(e) => Err(bmw_err::err!(bmw_err::ErrKind::IllegalState, format!("could not obtain write lock from http client container: {}", e)))
                                }
                        }
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
	use std::io::{Read, Write};
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
			base_dir: test_dir.to_string(),
			server_version: "test1".to_string(),
			debug: true,
			..Default::default()
		};
		let mut http = Builder::build_http_server(&config)?;
		http.start()?;

		// begin macros
		http_client_init!(BaseDir(test_dir))?;
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

				let mut hcr = response.content_reader()?;
				let mut buf = [0u8; 1_000];
				let mut content = vec![];
				loop {
					let len = hcr.read(&mut buf)?;
					if len == 0 {
						break;
					}
					content.extend(&buf[0..len]);
				}
				let content = std::str::from_utf8(&content).unwrap();

				assert_eq!(content.to_string(), data_text.to_string());
			} else if guid2 == request.guid() {
				assert_eq!(response.code().unwrap_or(u16::MAX), 404);
			} else {
				// should not get here
				return Ok(());
			}

			wlock!(data) += 1;

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

			if count < 30_000 && rlock!(data_clone) != 2 {
				continue;
			}

			assert_eq!(2, rlock!(data_clone));
			break;
		}

		let request3 = http_client_request!(Uri("/foo.html"))?;
		let request3_guid = request3.guid();

		let mut resp_count = lock_box!(0usize)?;
		let mut resp_count_clone = resp_count.clone();
		let resp_count_clone2 = resp_count.clone();

		let mut connection = http_connection!(Tls(false), Host("127.0.0.1"), Port(port))?;
		http_client_send!([request3], connection, {
			let response = http_client_response!()?;
			let request = http_client_request!()?;

			assert_eq!(request3_guid, request.guid());
			assert_eq!(response.code()?, 200);

			wlock!(resp_count) += 1;

			let mut hcr = response.content_reader()?;
			let mut buf = [0u8; 1_000];
			let mut content = vec![];
			loop {
				let len = hcr.read(&mut buf)?;
				if len == 0 {
					break;
				}
				content.extend(&buf[0..len]);
			}
			let content = std::str::from_utf8(&content).unwrap();

			info!("http_conn response = '{}'", content)?;

			Ok(())
		})?;

		let request4 = http_client_request!(Uri("/foo2.html"))?;
		let request4_guid = request4.guid();

		http_client_send!([request4], connection, {
			let response = http_client_response!()?;
			let request = http_client_request!()?;

			assert_eq!(request4_guid, request.guid());
			assert_eq!(response.code()?, 404);

			wlock!(resp_count_clone) += 1;

			let mut hcr = response.content_reader()?;
			let mut buf = [0u8; 1_000];
			let mut content = vec![];
			loop {
				let len = hcr.read(&mut buf)?;
				if len == 0 {
					break;
				}
				content.extend(&buf[0..len]);
			}
			let content = std::str::from_utf8(&content).unwrap();

			info!("http_conn response = '{}'", content)?;

			Ok(())
		})?;

		let mut count = 0;
		loop {
			sleep(Duration::from_millis(1));
			count += 1;
			if rlock!(resp_count_clone2) != 2 && count < 10_000 {
				continue;
			}

			assert_eq!(rlock!(resp_count_clone2), 2);
			break;
		}

		let request5 = http_client_request!(Url(&format!("http://{}:{}/foo.html", addr, port)))?;
		let response = http_client_send!(request5)?;

		let response_code = response.code().unwrap_or(u16::MAX);

		let mut hcr = response.content_reader()?;
		let mut buf = [0u8; 1_000];
		let mut content = vec![];
		loop {
			let len = hcr.read(&mut buf)?;
			if len == 0 {
				break;
			}
			content.extend(&buf[0..len]);
		}
		let content = std::str::from_utf8(&content).unwrap();

		assert_eq!(response_code, 200);
		assert_eq!(content, "Hello Macro World!");

		tear_down_test_dir(test_dir)?;
		Ok(())
	}
}
