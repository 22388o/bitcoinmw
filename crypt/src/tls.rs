// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::types::{TlsClientCertVerifier, TlsServerCertVerifier};
use bmw_deps::ed25519_dalek::PublicKey;
use bmw_deps::rustls::client::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use bmw_deps::rustls::internal::msgs::base::PayloadU16;
use bmw_deps::rustls::internal::msgs::handshake::DigitallySignedStruct;
use bmw_deps::rustls::server::{ClientCertVerified, ClientCertVerifier};
use bmw_deps::rustls::{Certificate, Error, ServerName, SignatureScheme};
use bmw_deps::x509_signature::{self, parse_certificate, X509Certificate};
use bmw_err::ErrorKind;
use bmw_log::*;
use std::time::SystemTime;

info!();

#[allow(deprecated)]
impl ClientCertVerifier for TlsClientCertVerifier {
	fn client_auth_root_subjects(&self) -> Option<Vec<PayloadU16>> {
		Some(vec![])
	}
	fn verify_client_cert(
		&self,
		cert: &Certificate,
		_: &[Certificate],
		_: SystemTime,
	) -> Result<ClientCertVerified, bmw_deps::rustls::Error> {
		let cert = get_cert(cert).map_err(|_e| Error::InvalidCertificateSignature)?;
		let pubkey = cert.subject_public_key_info().key();
		let pubkey = match PublicKey::from_bytes(pubkey) {
			Ok(p) => p,
			Err(e) => {
				return Err(Error::InvalidCertificateData(format!(
					"invalid pubkey: {}",
					e
				)));
			}
		};
		let mut lock = self.found_pubkey.clone();
		let mut lock = lock.wlock().unwrap();
		**lock.guard() = Some(pubkey);

		Ok(ClientCertVerified::assertion())
	}

	fn verify_tls12_signature(
		&self,
		message: &[u8],
		cert: &Certificate,
		dss: &DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, Error> {
		let cert = get_cert(cert).map_err(|_e| Error::InvalidCertificateSignature)?;

		let scheme = convert_scheme(dss.scheme)?;
		let signature = dss.sig.0.as_ref();

		cert.check_signature(scheme, message, signature)
			.map(|_| HandshakeSignatureValid::assertion())
			.map_err(|_| Error::InvalidCertificateSignature)
	}
	fn verify_tls13_signature(
		&self,
		message: &[u8],
		cert: &Certificate,
		dss: &DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, Error> {
		let cert = get_cert(cert).map_err(|_e| Error::InvalidCertificateSignature)?;
		let scheme = convert_scheme(dss.scheme)?;
		let signature = dss.sig.0.as_ref();

		cert.check_tls13_signature(scheme, message, signature)
			.map(|_| HandshakeSignatureValid::assertion())
			.map_err(|_| Error::InvalidCertificateSignature)
	}
}

#[allow(deprecated)]
impl ServerCertVerifier for TlsServerCertVerifier {
	fn verify_server_cert(
		&self,
		end_entity: &Certificate,
		_intermediates: &[Certificate],
		_server_name: &ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: SystemTime,
	) -> Result<ServerCertVerified, Error> {
		let cert = get_cert(end_entity)
			.map_err(|e| Error::InvalidCertificateData(format!("InvalidCertificateData: {}", e)))?;
		let pubkey = cert.subject_public_key_info().key();
		let expected = self.expected_pubkey.as_bytes();

		if pubkey != expected {
			return Err(Error::InvalidCertificateData(format!(
				"InvalidCertificateData: Expected Pubkey: {:?}, Found Pubkey: {:?}",
				expected, pubkey
			)));
		}

		Ok(ServerCertVerified::assertion())
	}

	fn verify_tls12_signature(
		&self,
		message: &[u8],
		cert: &Certificate,
		dss: &DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, Error> {
		let cert = get_cert(cert).map_err(|_e| Error::InvalidCertificateSignature)?;
		let scheme = convert_scheme(dss.scheme)?;
		let signature = dss.sig.0.as_ref();

		cert.check_signature(scheme, message, signature)
			.map(|_| HandshakeSignatureValid::assertion())
			.map_err(|_| Error::InvalidCertificateSignature)
	}
	fn verify_tls13_signature(
		&self,
		message: &[u8],
		cert: &Certificate,
		dss: &DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, Error> {
		let cert = get_cert(cert).map_err(|_e| Error::InvalidCertificateSignature)?;
		let scheme = convert_scheme(dss.scheme)?;
		let signature = dss.sig.0.as_ref();

		cert.check_tls13_signature(scheme, message, signature)
			.map(|_| HandshakeSignatureValid::assertion())
			.map_err(|_| Error::InvalidCertificateSignature)
	}
}

fn get_cert(c: &Certificate) -> Result<X509Certificate, bmw_err::Error> {
	parse_certificate(c.as_ref())
		.map_err(|e| ErrorKind::Rustls(format!("Cert error: {:?}", e)).into())
}

/// Convert from the signature scheme type used in `rustls` to the one used in
/// `x509_signature`.
fn convert_scheme(scheme: SignatureScheme) -> Result<x509_signature::SignatureScheme, Error> {
	use bmw_deps::rustls::SignatureScheme as R;
	use x509_signature::SignatureScheme as X;
	// Yes, we do allow PKCS1 here.  That's fine in practice when PKCS1 is only
	// used (as in TLS 1.2) for signatures; the attacks against correctly
	// implemented PKCS1 make sense only when it's used for
	// encryption.
	Ok(match scheme {
		R::RSA_PKCS1_SHA256 => X::RSA_PKCS1_SHA256,
		R::ECDSA_NISTP256_SHA256 => X::ECDSA_NISTP256_SHA256,
		R::RSA_PKCS1_SHA384 => X::RSA_PKCS1_SHA384,
		R::ECDSA_NISTP384_SHA384 => X::ECDSA_NISTP384_SHA384,
		R::RSA_PKCS1_SHA512 => X::RSA_PKCS1_SHA512,
		R::RSA_PSS_SHA256 => X::RSA_PSS_SHA256,
		R::RSA_PSS_SHA384 => X::RSA_PSS_SHA384,
		R::RSA_PSS_SHA512 => X::RSA_PSS_SHA512,
		R::ED25519 => X::ED25519,
		R::ED448 => X::ED448,
		R::RSA_PKCS1_SHA1 | R::ECDSA_SHA1_Legacy | R::ECDSA_NISTP521_SHA512 => {
			// The `x509-signature` crate doesn't support these, nor should it really.
			return Err(Error::PeerIncompatibleError(format!(
				"Unsupported signature scheme {:?}",
				scheme
			)));
		}
		R::Unknown(_) => {
			return Err(Error::PeerIncompatibleError(format!(
				"Unrecognized signature scheme {:?}",
				scheme
			)))
		}
	})
}
