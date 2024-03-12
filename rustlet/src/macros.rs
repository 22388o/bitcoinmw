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
macro_rules! tls_config {
	( $( $config:expr ),* ) => {{
                let mut error = None;
                let mut cert_file = None;
                let mut privkey_file = None;

                $(
                        match $config {
                                bmw_http::ConfigOption::PrivKeyFile(value) => {
                                        privkey_file = Some(value.to_string());
                                },
                                bmw_http::ConfigOption::FullChainCertFile(value) => {
                                        cert_file = Some(value.to_string());
                                },
                                _ => error = Some(format!("'{:?}' is not allowed for tls_config", $config)),
                        }
                )*

                match error{
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::IllegalArgument, format!("could not create tls_config due to: {}", error))),
                        None => {
                                match cert_file {
                                        Some(cert_file) =>
                                                match privkey_file {
                                                        Some(privkey_file) => {
                                                                Ok(bmw_http::TlsConfigFiles { cert_file, privkey_file })
                                                        },
                                                        None => {
                                                                Err(bmw_err::err!(bmw_err::ErrKind::IllegalArgument, "PrivKeyFile not specified. Could not create TlsConfigFiles."))
                                                        },
                                                },
                                        None => {
                                                Err(bmw_err::err!(bmw_err::ErrKind::IllegalArgument, "FullChainCertFile not specified. Could not create TlsConfigFiles."))
                                        }
                                }
                        },
                }
	}};
}

#[macro_export]
macro_rules! instance {
        ( $( $config:expr ),* ) => {{
                let mut error = None;
                let mut instance = bmw_http::HttpInstance::default();
                let mut base_dir = ".bmw/www";
                let mut tls_config_files = None;
                let mut _port_specified = false;
                let mut _base_dir_specified = false;
                let mut _tls_specified = false;

                $(
                        match $config {
                                bmw_http::ConfigOption::Port(port) => {
                                        instance.port = port;
                                        if _port_specified {
                                                error = Some("Port was specified more than once".to_string());
                                        }
                                        _port_specified = true;
                                },
                                bmw_http::ConfigOption::BaseDir(value) => {
                                        base_dir = value;
                                        if _base_dir_specified {
                                                error = Some("BaseDir was specified more than once".to_string());
                                        }
                                        _base_dir_specified = true;
                                },
                                bmw_http::ConfigOption::TlsServerConfig(value) => {
                                        tls_config_files = Some(value);
                                        if _tls_specified {
                                                error = Some("Tls was specified more than once".to_string());
                                        }
                                        _tls_specified = true;
                                },
                                _ => {
                                     error= Some(format!("'{:?}' is not allowed for instance", $config));
                                },
                        }
                )*

                let mut http_dir_map = HashMap::new();
                http_dir_map.insert("*".to_string(), base_dir.to_string());
                let instance_type = match tls_config_files {
                        Some(tls_config_files) =>
                                bmw_http::HttpInstanceType::Tls(bmw_http::TlsConfig {
                                        http_dir_map,
                                        cert_file: tls_config_files.cert_file,
                                        privkey_file: tls_config_files.privkey_file
                                }),
                        None =>
                                bmw_http::HttpInstanceType::Plain(bmw_http::PlainConfig { http_dir_map }),
                };
                instance.instance_type = instance_type;

                match error{
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::IllegalArgument, format!("could not create instance due to: {}", error))),
                        None => Ok(bmw_http::ConfigOption::Instance(instance)),
                }
        }};
}

#[macro_export]
macro_rules! rustlet_init {
	( $( $config:expr ),* ) => {{
                let mut error = None;
                let mut config = bmw_rustlet::RustletConfig {
                        http_config: bmw_http::HttpConfig {
                                evh_config: bmw_evh::EventHandlerConfig::default(),
                                instances: vec![],
                                ..Default::default()
                        },
                        rustlet_config: bmw_rustlet::RustletContainerConfig::default(),
                };
                $(
                        match $config {
                                bmw_http::ConfigOption::Instance(instance) => {
                                        config.http_config.instances.push(instance);
                                },
                                bmw_http::ConfigOption::Debug(debug) => {
                                        config.http_config.debug = debug;
                                },
                                bmw_http::ConfigOption::BaseDir(base_dir) => {
                                        config.http_config.base_dir = base_dir.to_string();
                                },
                                _ => {
                                        error = Some(format!("'{:?}' is not allowed for rustlet_init", $config));
                                },
                        }
                )*

                match error{
                        Some(error) => Err(bmw_err::err!(bmw_err::ErrKind::IllegalArgument, format!("could not create instance due to: {}", error))),
                        None => {
		                match bmw_rustlet::RustletContainer::init(config) {
			                Ok(_) => Ok(()),
			                Err(e) => Err(bmw_err::err!(
				                bmw_err::ErrKind::IllegalState,
				                format!("could not initialize rustlet container due to: {}", e)
			                )),
		                }
                        }
                }
	}};
}

#[macro_export]
macro_rules! rustlet_start {
	() => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.start()?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to start rustlet container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet_stop {
	() => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.stop()?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to stop rustlet container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! websocket {
	($name:expr, $code:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.add_websocket(
					$name,
					Box::pin(
						move |request: &mut Box<dyn bmw_rustlet::WebSocketRequest>| {
							bmw_rustlet::RUSTLET_CONTEXT.with(|f| {
								*f.borrow_mut() = (None, Some((*request).clone()));
							});
							{
								$code
							}
						},
					),
				)?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to add websocket to container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet {
	($name:expr, $code:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut container) => match container.get_mut(&std::thread::current().id()) {
				Some(container) => Ok(container.add_rustlet(
					$name,
					Box::pin(
						move |request: &mut Box<dyn bmw_rustlet::RustletRequest>,
						      response: &mut Box<dyn bmw_rustlet::RustletResponse>| {
							bmw_rustlet::RUSTLET_CONTEXT.with(|f| {
								*f.borrow_mut() =
									(Some(((*request).clone(), (*response).clone())), None);
							});
							{
								$code
							}
						},
					),
				)?),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to add rustlet to container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! rustlet_mapping {
	($path:expr, $name:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut containers) => match ((*containers).get_mut(&std::thread::current().id())) {
				Some(container) => container.add_rustlet_mapping($path, $name),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain request for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!("could not obtain lock to get request from container: {}", e)
			)),
		}
	}};
}

#[macro_export]
macro_rules! request {
	() => {{
		RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((request, _)) => Ok(request.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::Rustlet,
				"Could not find rustlet context"
			)),
		})
	}};
}

#[macro_export]
macro_rules! response {
	() => {{
		bmw_rustlet::RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).0 {
			Some((_, response)) => Ok(response.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::Rustlet,
				"Could not find rustlet context"
			)),
		})
	}};
}

/// Returns [`crate::WebSocketRequest`].
#[macro_export]
macro_rules! websocket_request {
	() => {{
		RUSTLET_CONTEXT.with(|f| match &(*f.borrow()).1 {
			Some(request) => Ok(request.clone()),
			None => Err(bmw_err::err!(
				bmw_err::ErrKind::Rustlet,
				"Could not find rustlet context"
			)),
		})
	}};
}

/// Three params: name, uri, [protocol list]
#[macro_export]
macro_rules! websocket_mapping {
	($path:expr, $name:expr, $protos:expr) => {{
		match bmw_rustlet::RUSTLET_CONTAINER.write() {
			Ok(mut containers) => match ((*containers).get_mut(&std::thread::current().id())) {
				Some(container) => container.add_websocket_mapping($path, $name, $protos),
				None => Err(bmw_err::err!(
					bmw_err::ErrKind::IllegalState,
					format!("could not obtain container to set a websocket_mapping for given thread")
				)),
			},
			Err(e) => Err(bmw_err::err!(
				bmw_err::ErrKind::IllegalState,
				format!(
					"could not obtain lock to insert websocket mapping from container: {}",
					e
				)
			)),
		}
	}};
}
