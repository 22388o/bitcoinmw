// Copyright (c) 2023-2024, The BitcoinMW Developers
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
macro_rules! websocket_client_init {
	( $( $config:expr ),* ) => {{
		let mut config = bmw_http::WebSocketClientConfig::default();
                let mut threads_specified = false;
                let mut max_handles_per_thread_specified = false;
                let mut debug_specified = false;
                let mut sync_channel_size_specified = false;
                let mut write_queue_size_specified = false;
                let mut nhandles_queue_size_specified = false;
                let mut max_events_in_specified = false;
                let mut max_events_specified = false;
                let mut housekeeping_frequency_millis_specified = false;
                let mut evh_read_slab_count_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.evh_threads == 0 { config.evh_threads = 0; }
                if error.is_some() { error = None; }
                if threads_specified { threads_specified = false; }
                if max_handles_per_thread_specified { max_handles_per_thread_specified = false; }
                if threads_specified {}
                if max_handles_per_thread_specified {}

                $(
                        match $config {
                                bmw_http::ConfigOption::Threads(threads) => {
                                        config.evh_threads = threads;

                                        if threads_specified {
                                                error = Some("Threads was specified more than once!".to_string());
                                        }
                                        threads_specified = true;
                                        if threads_specified {}

                                },
                                bmw_http::ConfigOption::MaxHandlesPerThread(mhpt) => {
                                        config.evh_max_handles_per_thread = mhpt;

                                        if max_handles_per_thread_specified {
                                                error = Some("MaxHandlesPerThread was specified more than once!".to_string());
                                        }

                                        max_handles_per_thread_specified = true;
                                        if max_handles_per_thread_specified {}
                                },
                                bmw_http::ConfigOption::Debug(debug) => {
                                        config.debug = debug;

                                        if debug_specified {
                                                error = Some("DEBUG was specified more than once!".to_string());
                                        }

                                        debug_specified = true;
                                        if debug_specified {}
                                },
                                bmw_http::ConfigOption::SyncChannelSize(sync_channel_size) => {
                                        config.evh_sync_channel_size = sync_channel_size;

                                        if sync_channel_size_specified {
                                                error = Some("SyncChannelSize was specified more than once!".to_string());
                                        }

                                        sync_channel_size_specified = true;
                                        if sync_channel_size_specified {}
                                },
                                bmw_http::ConfigOption::WriteQueueSize(write_queue_size) => {
                                        config.evh_write_queue_size = write_queue_size;

                                        if write_queue_size_specified {
                                                error = Some("WriteQueueSize was specified more than once!".to_string());
                                        }

                                        write_queue_size_specified = true;
                                        if write_queue_size_specified {}
                                },
                                bmw_http::ConfigOption::NhandlesQueueSize(nhandles_queue_size) => {
                                        config.evh_nhandles_queue_size = nhandles_queue_size;

                                        if nhandles_queue_size_specified {
                                                error = Some("NhandlesQueueSize was specified more than once!".to_string());
                                        }

                                        nhandles_queue_size_specified = true;
                                        if nhandles_queue_size_specified {}
                                },
                                bmw_http::ConfigOption::MaxEventsIn(max_events_in) => {
                                        config.evh_max_events_in = max_events_in;

                                        if max_events_in_specified {
                                                error = Some("MaxEventsIn was specified more than once!".to_string());
                                        }

                                        max_events_in_specified = true;
                                        if max_events_in_specified {}
                                },
                                bmw_http::ConfigOption::MaxEvents(max_events) => {
                                        config.evh_max_events = max_events;

                                        if max_events_specified {
                                                error = Some("MaxEvents was specified more than once!".to_string());
                                        }

                                        max_events_specified = true;
                                        if max_events_specified {}
                                },
                                bmw_http::ConfigOption::HouseKeepingFrequencyMillis(housekeeping_frequency_millis) => {
                                        config.evh_housekeeping_frequency_millis = housekeeping_frequency_millis;

                                        if housekeeping_frequency_millis_specified {
                                                error = Some("HouseKeepingFrequencyMillis was specified more than once!".to_string());
                                        }

                                        housekeeping_frequency_millis_specified = true;
                                        if housekeeping_frequency_millis_specified {}
                                },
                                bmw_http::ConfigOption::EvhReadSlabCount(evh_read_slab_count) => {
                                        config.evh_read_slab_count = evh_read_slab_count;

                                        if evh_read_slab_count_specified {
                                                error = Some("EvhReadSlabCount was specified more than once!".to_string());
                                        }

                                        evh_read_slab_count_specified = true;
                                        if evh_read_slab_count_specified {}
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for websocket_client_init", $config));
                                }
                        }
                )*
                match error {
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
                        None => {
                                bmw_http::WebSocketClientContainer::init(&config)
                        }
                }


	}};
}

#[macro_export]
macro_rules! websocket_client_stop {
	() => {{
		bmw_http::WebSocketClientContainer::stop()
	}};
}

#[macro_export]
macro_rules! websocket_connection_config {
	( $( $config:expr ),* ) => {{
		let mut config = WebSocketConnectionConfig::default();
                let mut url_specified = false;
                let mut full_chain_cert_file_specified = false;
                let mut masked_specified = false;
                let mut protocols_specified = false;
		let mut error: Option<String> = None;

                $(
                        match $config {
                                bmw_http::ConfigOption::Url(url) => {
                                        if url_specified {
                                                error = Some(format!("Url was specified more than once"));
                                        }

                                        let parsed_url = bmw_deps::url::Url::parse(url)?;
                                        match parsed_url.scheme() {
                                                "wss" => config.tls = true,
                                                "ws" => config.tls = false,
                                                _ => error = Some("invalid url. scheme must be ws:// or wss://".to_string()),
                                        }

                                        match parsed_url.host() {
                                                Some(host) => config.host = host.to_string(),
                                                None => error = Some("invalid host specified".to_string()),
                                        }

                                        match parsed_url.port() {
                                            Some(port) => config.port = port,
                                            _ => {
                                                    if config.tls { config.port = 443; } else { config.port = 80; }
                                            },
                                        }

                                        config.path = parsed_url.path().to_string();
                                        url_specified = true;

                                        if url_specified {}
                                },
                                bmw_http::ConfigOption::FullChainCertFile(file) => {
                                        config.full_chain_cert_file = Some(file.to_string());

                                        if full_chain_cert_file_specified {
                                                error = Some("FullChainCertFile was specified more than once!".to_string());
                                        }

                                        full_chain_cert_file_specified = true;

                                        if full_chain_cert_file_specified {}
                                },
                                bmw_http::ConfigOption::Protocols(protocols) => {
                                        config.protocols = protocols;

                                        if protocols_specified {
                                                error = Some("Protocols was specified more than once!".to_string());
                                        }

                                        protocols_specified = true;

                                        if protocols_specified {}
                                },
                                bmw_http::ConfigOption::Masked(masked) => {
                                        config.masked = masked;

                                        if masked_specified {
                                                error = Some("Masked was specified more than once!".to_string());
                                        }

                                        masked_specified = true;

                                        if masked_specified {}
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for websocket_connection_config", $config));
                                }
                        }
                )*

                if !url_specified {
                        error = Some("url must be specified".to_string());
                }

		match error {
			Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::Configuration, error)),
			None => Ok(config),
		}
	}};
}

#[macro_export]
macro_rules! websocket_connection {
	( $config:expr, $handler:expr) => {{
		match bmw_http::WEBSOCKET_CLIENT_CONTAINER.write() {
			Ok(mut container) => match (*container).get_mut(&std::thread::current().id()) {
				Some(websocket_client) => {
					let handler =
						Box::pin(move |msg: &WebSocketMessage, wsh: &mut WebSocketHandle| {
							bmw_http::WEBSOCKET_CLIENT_CONTEXT.with(|f| {
								*f.borrow_mut() = Some((msg.clone(), wsh.clone()));
							});
							{
								$handler
							}
						});
					websocket_client.connect($config, handler)
				}
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					"no websocket_client found for this thread"
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!(
					"could not obtain write lock from websocket client container: {}",
					e
				)
			)),
		}
	}};
}

#[macro_export]
macro_rules! websocket_message {
	() => {{
		bmw_http::WEBSOCKET_CLIENT_CONTEXT.with(|f| match &(*f.borrow()) {
			Some((msg, _wsh)) => Ok(msg.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				"Could not find websocket message given the current context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! websocket_handle {
	() => {{
		bmw_http::WEBSOCKET_CLIENT_CONTEXT.with(|f| match &(*f.borrow()) {
			Some((_msg, wsh)) => Ok(wsh.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				"Could not find websocket handle given the current context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! http_client_init {
	( $( $config:expr ),* ) => {{
                let mut config = bmw_http::HttpClientConfig::default();
                let mut threads_specified = false;
                let mut max_handles_per_thread_specified = false;
                let mut base_dir_specified = false;
                let mut debug_specified = false;
                let mut max_headers_len_specified = false;
                let mut sync_channel_size_specified = false;
                let mut write_queue_size_specified = false;
                let mut nhandles_queue_size_specified = false;
                let mut max_events_in_specified = false;
                let mut max_events_specified = false;
                let mut housekeeping_frequency_millis_specified = false;
                let mut evh_read_slab_count_specified = false;
                let mut slab_count_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.evh_threads == 0 { config.evh_threads = 0; }
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
                                        config.evh_threads = threads;

                                        if threads_specified {
                                                error = Some("Threads was specified more than once!".to_string());
                                        }
                                        threads_specified = true;
                                        if threads_specified {}

                                },
                                bmw_http::ConfigOption::MaxHandlesPerThread(mhpt) => {
                                        config.evh_max_handles_per_thread = mhpt;

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
                                bmw_http::ConfigOption::Debug(debug) => {
                                        config.debug = debug;

                                        if debug_specified {
                                                error = Some("DEBUG was specified more than once!".to_string());
                                        }

                                        debug_specified = true;
                                        if debug_specified {}
                                },
                                bmw_http::ConfigOption::MaxHeadersLen(max_headers_len) => {
                                        config.max_headers_len= max_headers_len;

                                        if max_headers_len_specified {
                                                error = Some("MaxHeadersLen was specified more than once!".to_string());
                                        }

                                        max_headers_len_specified = true;
                                        if max_headers_len_specified {}
                                },
                                bmw_http::ConfigOption::SyncChannelSize(sync_channel_size) => {
                                        config.evh_sync_channel_size = sync_channel_size;

                                        if sync_channel_size_specified {
                                                error = Some("SyncChannelSize was specified more than once!".to_string());
                                        }

                                        sync_channel_size_specified = true;
                                        if sync_channel_size_specified {}
                                },
                                bmw_http::ConfigOption::WriteQueueSize(write_queue_size) => {
                                        config.evh_write_queue_size = write_queue_size;

                                        if write_queue_size_specified {
                                                error = Some("WriteQueueSize was specified more than once!".to_string());
                                        }

                                        write_queue_size_specified = true;
                                        if write_queue_size_specified {}
                                },
                                bmw_http::ConfigOption::NhandlesQueueSize(nhandles_queue_size) => {
                                        config.evh_nhandles_queue_size = nhandles_queue_size;

                                        if nhandles_queue_size_specified {
                                                error = Some("NhandlesQueueSize was specified more than once!".to_string());
                                        }

                                        nhandles_queue_size_specified = true;
                                        if nhandles_queue_size_specified {}
                                },
                                bmw_http::ConfigOption::MaxEventsIn(max_events_in) => {
                                        config.evh_max_events_in = max_events_in;

                                        if max_events_in_specified {
                                                error = Some("MaxEventsIn was specified more than once!".to_string());
                                        }

                                        max_events_in_specified = true;
                                        if max_events_in_specified {}
                                },
                                bmw_http::ConfigOption::MaxEvents(max_events) => {
                                        config.evh_max_events = max_events;

                                        if max_events_specified {
                                                error = Some("MaxEvents was specified more than once!".to_string());
                                        }

                                        max_events_specified = true;
                                        if max_events_specified {}
                                },
                                bmw_http::ConfigOption::HouseKeepingFrequencyMillis(housekeeping_frequency_millis) => {
                                        config.evh_housekeeping_frequency_millis = housekeeping_frequency_millis;

                                        if housekeeping_frequency_millis_specified {
                                                error = Some("HouseKeepingFrequencyMillis was specified more than once!".to_string());
                                        }

                                        housekeeping_frequency_millis_specified = true;
                                        if housekeeping_frequency_millis_specified {}
                                },
                                bmw_http::ConfigOption::EvhReadSlabCount(evh_read_slab_count) => {
                                        config.evh_read_slab_count = evh_read_slab_count;

                                        if evh_read_slab_count_specified {
                                                error = Some(" was specified more than once!".to_string());
                                        }

                                        evh_read_slab_count_specified = true;
                                        if evh_read_slab_count_specified {}
                                },
                                bmw_http::ConfigOption::SlabCount(slab_count) => {
                                        config.slab_count = slab_count;

                                        if slab_count_specified {
                                                error = Some("SlabCount was specified more than once!".to_string());
                                        }

                                        slab_count_specified = true;
                                        if slab_count_specified {}
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
macro_rules! http_client_stop {
	() => {{
		bmw_http::HttpClientContainer::stop()
	}};
}

#[macro_export]
macro_rules! http_client_request {
        () => {{
                bmw_http::HTTP_CLIENT_CONTEXT.with(|f| match &(*f.borrow()) {
                        Some((request, _response)) => Ok(request.clone()),
                        None => Err(bmw_err::err!(
                                bmw_err::ErrKind::IllegalState,
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
                let mut method_specified = false;
                let mut version_specified = false;
                let mut full_chain_specified = false;
                let mut content_data_specified = false;
                let mut content_file_specified = false;
                let mut keep_alive_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.user_agent == "" { config.user_agent = "".to_string(); }
                if error.is_some() { error = None; }
                if request_url_specified { request_url_specified = false; }
                if request_uri_specified { request_uri_specified = false; }
                if user_agent_specified { user_agent_specified = false; }
                if accept_specified { accept_specified = false; }
                if timeout_millis_specified { timeout_millis_specified = false; }
                if method_specified { method_specified = false; }
                if version_specified { version_specified = false; }
                if full_chain_specified { full_chain_specified = false; }
                if content_data_specified { content_data_specified = false; }
                if content_file_specified { content_file_specified = false; }
                if keep_alive_specified { keep_alive_specified = false; }
                if full_chain_specified {}
                if request_url_specified {}
                if request_uri_specified {}
                if user_agent_specified {}
                if accept_specified {}
                if timeout_millis_specified {}
                if method_specified {}
                if content_data_specified {}
                if content_file_specified {}
                if keep_alive_specified {}

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
                                bmw_http::ConfigOption::FullChainCertFile(file) => {
                                        config.full_chain = Some(file.to_string());

                                        if full_chain_specified {
                                                error = Some("FullCHainCertFile was specified more than once!".to_string());
                                        }

                                        full_chain_specified = true;
                                        if full_chain_specified {}
                                },
                                bmw_http::ConfigOption::TimeoutMillis(millis) => {
                                        config.timeout_millis = millis;

                                        if timeout_millis_specified {

                                        }

                                        timeout_millis_specified = true;
                                        if timeout_millis_specified {}
                                },
                                bmw_http::ConfigOption::ContentData(data) => {
                                        config.content_data = Some(data.to_vec());

                                        if content_data_specified {

                                        }

                                        content_data_specified = true;
                                        if content_data_specified {}
                                },
                                bmw_http::ConfigOption::ContentFile(file) => {
                                        config.content_file = Some(file.to_string());

                                        if content_file_specified {

                                        }

                                        content_file_specified = true;
                                        if content_file_specified {}
                                },
                                bmw_http::ConfigOption::Method(method) => {
                                        config.method = method;

                                        if method_specified {

                                        }

                                        method_specified = true;
                                        if method_specified {}
                                },
                                bmw_http::ConfigOption::Version(version) => {
                                        config.version = version;

                                        if version_specified {

                                        }

                                        version_specified = true;
                                        if version_specified {}
                                },
                                bmw_http::ConfigOption::KeepAlive(keep_alive) => {
                                        config.keep_alive = keep_alive;

                                        if keep_alive_specified {

                                        }

                                        keep_alive_specified = true;

                                        if keep_alive_specified {}
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
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
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
					let handler = &Box::pin(
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
							std::thread::sleep(std::time::Duration::from_millis(timeout_millis));
							match tx.send(Err(bmw_err::err!(
								bmw_err::ErrKind::IO,
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

					match http_client.send(&$request, handler.clone()) {
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
							match request.timeout_millis() != 0 {
								true => {
                                                                        err_vec.push(
                                                                            bmw_err::err!(
                                                                                bmw_err::ErrKind::IllegalState,
                                                                                "cannot set timeout for a request that is not synchronous")
                                                                            );
                                                                }
								false => match http_client.send(&request, handler.clone()) {
									Ok(_) => {}
									Err(e) => {
										err_vec.push(e);
									}
								},
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
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
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
                        match request.timeout_millis() != 0 {
                                true => {
                                        err_vec.push(
                                                bmw_err::err!(
                                                        bmw_err::ErrKind::IllegalState,
                                                        "cannot set timeout for a request that is not synchronous"
                                                )
                                        );
                                }
                                false => match $connection.send(&request, handler.clone()) {
				        Ok(_) => {}
				        Err(e) => {
					        err_vec.push(e);
				        }
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
                let mut full_chain_cert_file_specified = false;
                let mut error: Option<String> = None;

                // to supress compiler warnings
                if config.port == 0 { config.port = 0; }
                if error.is_some() { error = None; }
                if host_specified { host_specified = false; }
                if port_specified { port_specified = false; }
                if tls_specified { tls_specified = false; }
                if full_chain_cert_file_specified { full_chain_cert_file_specified = false; }
                if host_specified {}
                if port_specified {}
                if tls_specified {}
                if full_chain_cert_file_specified {}

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
                                bmw_http::ConfigOption::FullChainCertFile(file) => {
                                        config.full_chain_cert_file = Some(file.to_string());

                                        if full_chain_cert_file_specified {
                                                error = Some("FullChainCertFile was specified more than once!".to_string());
                                        }

                                        full_chain_cert_file_specified = true;
                                        if full_chain_cert_file_specified {}
                                }
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

#[macro_export]
macro_rules! http_connection_close {
	($connection:expr) => {{
		match $connection.close() {
			Ok(_) => Ok(()),
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IO,
				format!("connection.close generated error: {}", e)
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
	use std::io::{Read, Write};
	use std::thread::{current, sleep};
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

		http_connection_close!(connection)?;
		http_client_stop!()?;
		tear_down_test_dir(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_http_client_macros_init() -> Result<(), Error> {
		let test_dir = ".test_http_client_macros_init.bmw";
		const EVH_THREADS: usize = 3;
		const MAX_HANDLES_PER_THREAD: usize = 5;

		setup_test_dir(test_dir)?;
		http_client_init!(
			BaseDir(test_dir),
			Threads(EVH_THREADS),
			MaxHandlesPerThread(MAX_HANDLES_PER_THREAD)
		)?;

		{
			let container = HTTP_CLIENT_CONTAINER.read()?;
			let http_client = (*container).get(&current().id()).unwrap();
			let config = http_client.config();
			assert_eq!(config.base_dir, test_dir);
			assert_eq!(config.evh_threads, EVH_THREADS);
			assert_eq!(config.evh_max_handles_per_thread, MAX_HANDLES_PER_THREAD);
		}

		http_client_stop!()?;
		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
