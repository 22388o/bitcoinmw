// Copyright (c) 2022, 37 Miners, LLC
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

use crate::constants::*;
use crate::types::{ChannelImpl, Info, TlsClientCertVerifier, TlsServerCertVerifier};
use crate::{Cell, Channel, ChannelDirection, ChannelState, Peer};
use bmw_deps::ed25519_dalek::{PublicKey, SecretKey};
use bmw_deps::openssl::pkey::Id;
use bmw_deps::openssl::pkey::PKey;
use bmw_deps::pem;
use bmw_deps::rcgen::{
	date_time_ymd, Certificate, CertificateParams, DistinguishedName, KeyPair, PKCS_ED25519,
};
use bmw_deps::rustls::client::ClientConfig;
use bmw_deps::rustls::server::ServerConfig;
use bmw_deps::rustls::{
	Certificate as RustlsCertificate, ClientConnection, PrivateKey as RustlsPrivateKey,
	RootCertStore, ServerConnection,
};
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::io::{Read, Write};
use std::sync::Arc;

info!();

impl ChannelState {
	pub fn new() -> Self {
		Self {
			bytes_to_write: 0,
			cells: vec![],
			has_closed: false,
			in_buf: vec![],
			offset: 0,
		}
	}
}

impl Channel for ChannelImpl {
	fn direction(&self) -> ChannelDirection {
		match self.tls_client {
			Some(_) => ChannelDirection::Outbound,
			_ => match self.tls_server {
				Some(_) => ChannelDirection::Inbound,
				_ => ChannelDirection::NotConnected,
			},
		}
	}

	fn is_verified(&self) -> bool {
		self.verified
	}

	fn read_crypt(&mut self, rd: &mut dyn Read) -> Result<usize, Error> {
		match &mut self.tls_client {
			Some(client) => map_err!(client.read_tls(rd), ErrKind::Rustls),
			_ => match &mut self.tls_server {
				Some(server) => map_err!(server.read_tls(rd), ErrKind::Rustls),
				_ => Err(err!(ErrKind::Crypt, "channel not connected")),
			},
		}
	}

	fn write_crypt(&mut self, wr: &mut dyn Write) -> Result<usize, Error> {
		match &mut self.tls_client {
			Some(client) => map_err!(client.write_tls(wr), ErrKind::Rustls),
			_ => match &mut self.tls_server {
				Some(server) => map_err!(server.write_tls(wr), ErrKind::Rustls),
				_ => Err(err!(ErrKind::Crypt, "channel not connected")),
			},
		}
	}

	fn send_cell(&mut self, cell: Cell) -> Result<(), Error> {
		let mut cell_bytes = [0u8; CELL_LEN];
		cell.to_bytes(&mut cell_bytes)?;
		match &mut self.tls_client {
			Some(client) => client.writer().write(&cell_bytes),
			_ => match &mut self.tls_server {
				Some(server) => server.writer().write(&cell_bytes),
				_ => return Err(err!(ErrKind::Crypt, "channel not connected")),
			},
		}?;
		Ok(())
	}

	fn process_new_packets(&mut self, state: &mut ChannelState) -> Result<(), Error> {
		let io_state = match &mut self.tls_client {
			Some(client) => map_err!(
				client.process_new_packets(),
				ErrKind::Rustls,
				"process_new_packets generated error"
			),
			None => match &mut self.tls_server {
				Some(server) => map_err!(
					server.process_new_packets(),
					ErrKind::Rustls,
					"process_new_packets generated error"
				),
				None => return Err(err!(ErrKind::Crypt, "channel not connected")),
			},
		}?;

		let len = io_state.plaintext_bytes_to_read();
		if len > 0 {
			self.process_cells(len, state)?;
		}
		state.has_closed = io_state.peer_has_closed();
		state.bytes_to_write = io_state.tls_bytes_to_write();

		Ok(())
	}
	fn connect(&mut self, peer: &Peer, secret: &SecretKey) -> Result<(), Error> {
		debug!("connecting to peer: {:?}", peer)?;

		// localhost is used because we don't validate hostnames.
		self.tls_client = Some(ClientConnection::new(
			self.make_client_config(secret, peer.pubkey.clone())?,
			"localhost".try_into()?,
		)?);
		Ok(())
	}

	fn accept(&mut self, secret: SecretKey) -> Result<(), Error> {
		debug!("accepting a connection")?;
		self.tls_server = Some(ServerConnection::new(self.make_server_config(secret)?)?);
		Ok(())
	}

	fn start(&mut self) -> Result<(), Error> {
		let info = Cell::Info(Info {
			local_peer: self.local_peer.clone(),
		});
		let mut cell = [0u8; CELL_LEN];
		info.to_bytes(&mut cell)?;

		match &mut self.tls_server {
			Some(server) => {
				// server
				debug!("start server")?;
				server.writer().write(&cell)?;
				Ok(())
			}
			None => match &mut self.tls_client {
				Some(client) => {
					// client
					client.writer().write(&cell)?;
					Ok(())
				}
				None => Err(err!(
					ErrKind::IllegalState,
					"start must be called after connect or accept"
				)),
			},
		}
	}
}

impl ChannelImpl {
	pub fn new(local_peer: Peer) -> Self {
		Self {
			tls_client: None,
			tls_server: None,
			remote_peer: None,
			local_peer,
			verified: false,
			tls_client_verifier: Arc::new(TlsClientCertVerifier {
				found_pubkey: lock_box!(None).unwrap(),
			}),
		}
	}

	fn process_cells(&mut self, len: usize, state: &mut ChannelState) -> Result<(), Error> {
		let mut reader = match &mut self.tls_client {
			Some(client) => client.reader(),
			_ => match &mut self.tls_server {
				Some(server) => server.reader(),
				_ => return Err(err!(ErrKind::IllegalState, "channel not connected")),
			},
		};

		debug!("reading len = {}", len)?;
		state.in_buf.resize(len, 0u8);
		reader.read_exact(&mut state.in_buf[state.offset..state.offset + len])?;
		state.offset += len;

		loop {
			if state.offset < CELL_LEN {
				debug!("Not enough data to verify. Current len = {}", state.offset)?;
				// return ok and try again later when we have a full cell
				return Ok(());
			}

			let cell = Cell::from_bytes(&state.in_buf[0..CELL_LEN])?;
			debug!("cell = {:?}", cell)?;

			let mut info_peer: Option<Peer> = None;

			match &cell {
				Cell::Info(info) => {
					info_peer = Some(info.local_peer.clone());
				}
				Cell::Padding(_padding) => {
					if !self.verified {
						return Err(err!(
							ErrKind::IllegalState,
							"only Info is allowed when not verified"
						));
					}
				}
			}

			if !self.verified {
				// check for remote peer
				if self.tls_server.is_some() {
					if info_peer.is_none() {
						return Err(err!(ErrKind::IllegalState, "info data not found"));
					}
					let lock = Arc::get_mut(&mut self.tls_client_verifier);
					let lock = lock.as_ref();
					if lock.is_some() {
						let lock = &*lock.unwrap();
						let pubkey = lock.found_pubkey.rlock()?;
						let guard = pubkey.guard();
						debug!("pubkey={:?}", **guard)?;
						if (**guard).as_ref().unwrap() != &info_peer.as_ref().unwrap().pubkey {
							return Err(err!(
								ErrKind::IllegalState,
								"tls pubkey did not match the pubkey sent by peer"
							));
						}
						self.remote_peer = info_peer.clone();
						self.verified = true;
					}
					debug!("server verified")?;
				} else if self.tls_client.is_some() {
					// we can't get here unless the pubkey matches our expectations due to the client verifier
					self.verified = true;
					self.remote_peer = info_peer.clone();
					debug!("client verified")?;
				}

				if self.verified {
					debug!("channel verified")?;
				}
			} else {
				state.cells.push(cell);
			}

			state.in_buf.drain(0..CELL_LEN);
			state.offset -= CELL_LEN;
		}
	}

	// make a client config that only verifies the signatures and not hostname.
	// Identity is verified after tls tunnel is created.
	fn make_client_config(
		&self,
		secret: &SecretKey,
		expected_pubkey: PublicKey,
	) -> Result<Arc<ClientConfig>, Error> {
		let mut params: CertificateParams = Default::default();
		params.not_before = date_time_ymd(2021, 05, 19);
		params.not_after = date_time_ymd(4096, 01, 01);
		params.distinguished_name = DistinguishedName::new();
		params.alg = &PKCS_ED25519;
		let pkey = PKey::private_key_from_raw_bytes(&secret.to_bytes(), Id::ED25519)?;
		let private_key = pkey.private_key_to_pem_pkcs8()?;
		let key_pair_pem = String::from_utf8(private_key.clone())?;
		let key_pair = KeyPair::from_pem(&key_pair_pem)?;
		params.key_pair = Some(key_pair);
		let private_key_der = pkey.private_key_to_der()?;
		let cert = Certificate::from_params(params)?;
		let pem_serialized = cert.serialize_pem()?;
		let der_serialized = pem::parse(&pem_serialized)?.contents;

		let root_store = RootCertStore::empty();

		let mut config = ClientConfig::builder()
			.with_safe_default_cipher_suites()
			.with_safe_default_kx_groups()
			.with_safe_default_protocol_versions()?
			.with_root_certificates(root_store)
			.with_single_cert(
				vec![RustlsCertificate(der_serialized)],
				RustlsPrivateKey(private_key_der),
			)?;

		config
			.dangerous()
			.set_certificate_verifier(Arc::new(TlsServerCertVerifier { expected_pubkey }));

		let ret = Arc::new(config);
		Ok(ret)
	}

	// make a server config
	fn make_server_config(&self, secret: SecretKey) -> Result<Arc<ServerConfig>, Error> {
		let mut params: CertificateParams = Default::default();
		params.not_before = date_time_ymd(2021, 05, 19);
		params.not_after = date_time_ymd(4096, 01, 01);
		params.distinguished_name = DistinguishedName::new();
		params.alg = &PKCS_ED25519;
		let pkey = PKey::private_key_from_raw_bytes(&secret.to_bytes(), Id::ED25519)?;
		let private_key = pkey.private_key_to_pem_pkcs8()?;
		let key_pair_pem = String::from_utf8(private_key.clone())?;
		let key_pair = KeyPair::from_pem(&key_pair_pem)?;
		params.key_pair = Some(key_pair);
		let private_key_der = pkey.private_key_to_der()?;
		let cert = Certificate::from_params(params)?;
		let pem_serialized = cert.serialize_pem()?;
		let der_serialized = pem::parse(&pem_serialized)?.contents;
		let config = ServerConfig::builder()
			.with_safe_defaults()
			.with_client_cert_verifier(self.tls_client_verifier.clone())
			.with_single_cert(
				vec![RustlsCertificate(der_serialized)],
				RustlsPrivateKey(private_key_der),
			)?;

		let ret = Arc::new(config);
		Ok(ret)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::types::RngCompatExt;
	use bmw_deps::ed25519_dalek::Keypair;
	use bmw_deps::rand::thread_rng;
	use std::io::{Read, Write};

	#[test]
	fn test_crypt_channel_tls_msgs() -> Result<(), Error> {
		let mut rng = thread_rng().rng_compat();
		let server_keypair = Keypair::generate(&mut rng);
		let client_keypair = Keypair::generate(&mut rng);
		let secret_client = client_keypair.secret;
		let secret_server = server_keypair.secret;
		let server_pubkey = server_keypair.public;
		let client_pubkey = client_keypair.public;

		let mut server = ChannelImpl::new(Peer {
			sockaddr: "127.0.0.1:1234".parse()?,
			pubkey: server_pubkey,
			nickname: "test1".to_string(),
		});
		let mut client = ChannelImpl::new(Peer {
			sockaddr: "127.0.0.1:4567".parse()?,
			pubkey: client_pubkey,
			nickname: "test2".to_string(),
		});

		let peer = Peer {
			sockaddr: "127.0.0.1:1234".parse()?,
			pubkey: server_pubkey,
			nickname: "test1".to_string(),
		};

		client.connect(&peer, &secret_client)?;
		server.accept(secret_server)?;

		let mut tls_client = client.tls_client.unwrap();
		let mut tls_server = server.tls_server.unwrap();

		let msg = b"abcd1234567890";
		// write message to the client
		tls_client.writer().write(msg)?;
		let io_state = tls_client.process_new_packets()?;
		info!("io_state={:?}", io_state)?;
		let mut buf = vec![];
		tls_client.write_tls(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// read the bytes into the server
		tls_server.read_tls(&mut &buf[..])?;
		let io_state = tls_server.process_new_packets()?;
		info!("io_state={:?}", io_state)?;
		let mut buf = vec![];
		tls_server.write_tls(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// read the bytes into the client
		tls_client.read_tls(&mut &buf[..])?;
		let io_state = tls_client.process_new_packets()?;
		info!("io_state={:?}", io_state)?;
		let mut buf = vec![];
		tls_client.write_tls(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// write the bytes into the server
		tls_server.read_tls(&mut &buf[..])?;
		let io_state = tls_server.process_new_packets()?;
		info!("io_state={:?}", io_state)?;

		let mut buf = vec![];
		buf.resize(msg.len(), 0u8);
		tls_server.reader().read(&mut buf)?;
		assert_eq!(&buf[..], msg);

		Ok(())
	}

	#[test]
	fn test_crypt_channel_with_start() -> Result<(), Error> {
		let mut rng = thread_rng().rng_compat();
		let server_keypair = Keypair::generate(&mut rng);
		let client_keypair = Keypair::generate(&mut rng);
		let secret_client = client_keypair.secret;
		let secret_server = server_keypair.secret;
		let server_pubkey = server_keypair.public;
		let client_pubkey = client_keypair.public;

		info!("server_pubkey={:?}", server_pubkey.as_bytes())?;
		info!("client_pubkey={:?}", client_pubkey.as_bytes())?;

		let mut server = ChannelImpl::new(Peer {
			sockaddr: "127.0.0.1:1234".parse()?,
			pubkey: server_pubkey,
			nickname: "test1".to_string(),
		});
		let mut client = ChannelImpl::new(Peer {
			sockaddr: "127.0.0.1:4567".parse()?,
			pubkey: client_pubkey,
			nickname: "test2".to_string(),
		});

		let peer = Peer {
			sockaddr: "127.0.0.1:1234".parse()?,
			pubkey: server_pubkey,
			nickname: "test1".to_string(),
		};

		client.connect(&peer, &secret_client)?;
		server.accept(secret_server)?;

		client.start()?;
		server.start()?;

		assert!(!client.is_verified());
		assert!(!server.is_verified());

		let mut client_state = ChannelState::new();
		let mut server_state = ChannelState::new();

		client.process_new_packets(&mut client_state)?;
		info!("client_state={:?}", client_state)?;
		let mut buf = vec![];
		client.write_crypt(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// read the bytes into the server
		server.read_crypt(&mut &buf[..])?;
		server.process_new_packets(&mut server_state)?;
		info!("server_state={:?}", server_state)?;
		let mut buf = vec![];
		server.write_crypt(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// read the bytes into the client
		client.read_crypt(&mut &buf[..])?;
		client.process_new_packets(&mut client_state)?;
		info!("client_state={:?}", client_state)?;
		let mut buf = vec![];
		client.write_crypt(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// write the bytes into the server
		server.read_crypt(&mut &buf[..])?;
		server.process_new_packets(&mut server_state)?;
		info!("server_state={:?}", server_state)?;
		let mut buf = vec![];
		server.write_crypt(&mut buf)?;
		info!("buf.len() = {}", buf.len())?;

		// write the bytes into the client
		client.read_crypt(&mut &buf[..])?;
		client.process_new_packets(&mut client_state)?;
		info!("client_state={:?}", client_state)?;

		assert!(client.is_verified());
		assert!(server.is_verified());

		Ok(())
	}
}
