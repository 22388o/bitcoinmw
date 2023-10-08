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

use crate::crypt::crypt1::{InboundClientLayer, OutboundClientLayer, RelayCellBody};
use crate::SecretBytes;
use bmw_deps::aes::Aes256Ctr;
use bmw_deps::arrayref::array_ref;
use bmw_deps::generic_array::GenericArray;
use bmw_deps::sha3::Sha3_256;
use bmw_deps::subtle::ConstantTimeEq;
use bmw_err::*;
use std::fmt::{Debug, Display, Formatter};

/// A KeyGenerator is returned by a handshake, and used to generate
/// session keys for the protocol.
///
/// Typically, it wraps a KDF function, and some seed key material.
///
/// It can only be used once.
pub trait KeyGenerator {
	/// Consume the key
	fn expand(self, keylen: usize) -> Result<SecretBytes, Error>;
}

/// Return true if two slices are equal.  Performs its operation in constant
/// time, but returns a bool instead of a subtle::Choice.
pub fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
	let choice = a.ct_eq(b);
	choice.unwrap_u8() == 1
}

/// Hops on the circuit.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct HopNum(u8);

impl From<HopNum> for u8 {
	fn from(hop: HopNum) -> u8 {
		hop.0
	}
}

impl From<u8> for HopNum {
	fn from(v: u8) -> HopNum {
		HopNum(v)
	}
}

impl From<HopNum> for usize {
	fn from(hop: HopNum) -> usize {
		hop.0 as usize
	}
}

impl Display for HopNum {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		Display::fmt(&self.0, f)
	}
}

/// A client's view of the cryptographic state for an entire
/// constructed circuit, as used for sending cells.
pub struct OutboundClientCrypt {
	/// Vector of layers, one for each hop on the circuit, ordered from the
	/// closest hop to the farthest.
	layers: Vec<Box<dyn OutboundClientLayer + Send>>,
}

impl Debug for OutboundClientCrypt {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "[outboundclientcrypt,layers={}]", self.layers.len())
	}
}

unsafe impl Sync for OutboundClientCrypt {}

/// A client's view of the cryptographic state for an entire
/// constructed circuit, as used for receiving cells.
pub struct InboundClientCrypt {
	/// Vector of layers, one for each hop on the circuit, ordered from the
	/// closest hop to the farthest.
	layers: Vec<Box<dyn InboundClientLayer + Send>>,
}

impl Debug for InboundClientCrypt {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "[inboundclientcrypt,layers={}]", self.layers.len())
	}
}

unsafe impl Sync for InboundClientCrypt {}

impl OutboundClientCrypt {
	/// Return a new (empty) OutboundClientCrypt.
	pub fn new() -> Self {
		OutboundClientCrypt { layers: Vec::new() }
	}
	/// Prepare a cell body to sent away from the client.
	///
	/// The cell is prepared for the `hop`th hop, and then encrypted with
	/// the appropriate keys.
	///
	/// On success, returns a reference to tag that should be expected
	/// for an authenticated SENDME sent in response to this cell.
	pub fn encrypt(&mut self, cell: &mut RelayCellBody, hop: HopNum) -> Result<&[u8; 20], Error> {
		let hop: usize = hop.into();
		if hop >= self.layers.len() {
			return Err(err!(ErrKind::Crypt, "No such hop"));
		}

		let mut layers = self.layers.iter_mut().take(hop + 1).rev();
		let first_layer = layers.next().ok_or(err!(ErrKind::Crypt, "No such hop"))?;
		let tag = first_layer.originate_for(cell);
		for layer in layers {
			layer.encrypt_outbound(cell);
		}
		Ok(tag.try_into()?)
	}

	/// Add a new layer to this OutboundClientCrypt
	pub fn add_layer(&mut self, layer: Box<dyn OutboundClientLayer + Send>) {
		assert!(self.layers.len() < u8::MAX as usize);
		self.layers.push(layer);
	}

	/// Return the number of layers configured on this OutboundClientCrypt.
	pub fn n_layers(&self) -> usize {
		self.layers.len()
	}
}

impl InboundClientCrypt {
	/// Return a new (empty) InboundClientCrypt.
	pub fn new() -> Self {
		InboundClientCrypt { layers: Vec::new() }
	}
	/// Decrypt an incoming cell that is coming to the client.
	///
	/// On success, return which hop was the originator of the cell.
	pub fn decrypt(&mut self, cell: &mut RelayCellBody) -> Result<(HopNum, &[u8]), Error> {
		for (hopnum, layer) in self.layers.iter_mut().enumerate() {
			if let Some(tag) = layer.decrypt_inbound(cell) {
				assert!(hopnum <= u8::MAX as usize);
				return Ok(((hopnum as u8).into(), tag));
			}
		}
		Err(err!(ErrKind::Crypt, "BadCellAuth"))
	}
	/// Add a new layer to this InboundClientCrypt
	pub fn add_layer(&mut self, layer: Box<dyn InboundClientLayer + Send>) {
		assert!(self.layers.len() < u8::MAX as usize);
		self.layers.push(layer);
	}

	/// Return the number of layers configured on this InboundClientCrypt.
	pub fn n_layers(&self) -> usize {
		self.layers.len()
	}
}

pub type Crypt1RelayCrypto = crypt1::CryptStatePair<Aes256Ctr, Sha3_256>;

pub mod crypt1 {
	use super::*;
	use bmw_deps::cipher::{NewCipher, StreamCipher};
	use bmw_deps::digest::Digest;
	use bmw_deps::typenum::Unsigned;
	use std::convert::TryInto;

	/// A CryptState is part of a RelayCrypt or a ClientLayer.
	pub struct CryptState<SC: StreamCipher, D: Digest + Clone> {
		/// Stream cipher for en/decrypting cell bodies.
		cipher: SC,
		/// Digest for authenticating cells to/from this hop.
		digest: D,
		/// Most recent digest value generated by this crypto.
		last_digest_val: GenericArray<u8, D::OutputSize>,
	}

	/// A pair of CryptStates, one for the forward (away from client)
	/// direction, and one for the reverse (towards client) direction.
	pub struct CryptStatePair<SC: StreamCipher, D: Digest + Clone> {
		/// State for en/decrypting cells sent away from the client.
		fwd: CryptState<SC, D>,
		/// State for en/decrypting cells sent towards the client.
		back: CryptState<SC, D>,
	}

	impl<SC: StreamCipher + NewCipher, D: Digest + Clone> CryptInit for CryptStatePair<SC, D> {
		fn seed_len() -> usize {
			SC::KeySize::to_usize() * 2 + D::OutputSize::to_usize() * 2
		}
		fn initialize(seed: &[u8]) -> Result<Self, Error> {
			if seed.len() != Self::seed_len() {
				return Err(err!(
					ErrKind::Crypt,
					format!("seed length {} was invalid", seed.len())
				));
			}
			let keylen = SC::KeySize::to_usize();
			let dlen = D::OutputSize::to_usize();
			let fdinit = &seed[0..dlen];
			let bdinit = &seed[dlen..dlen * 2];
			let fckey = &seed[dlen * 2..dlen * 2 + keylen];
			let bckey = &seed[dlen * 2 + keylen..dlen * 2 + keylen * 2];
			let fwd = CryptState {
				cipher: SC::new(fckey.try_into().expect("Wrong length"), &Default::default()),
				digest: D::new().chain_update(fdinit),
				last_digest_val: GenericArray::default(),
			};
			let back = CryptState {
				cipher: SC::new(bckey.try_into().expect("Wrong length"), &Default::default()),
				digest: D::new().chain_update(bdinit),
				last_digest_val: GenericArray::default(),
			};
			Ok(CryptStatePair { fwd, back })
		}
	}

	impl<SC, D> ClientLayer<CryptState<SC, D>, CryptState<SC, D>> for CryptStatePair<SC, D>
	where
		SC: StreamCipher,
		D: Digest + Clone,
	{
		fn split(self) -> (CryptState<SC, D>, CryptState<SC, D>) {
			(self.fwd, self.back)
		}
	}

	impl<SC: StreamCipher, D: Digest + Clone> RelayCrypt for CryptStatePair<SC, D> {
		fn originate(&mut self, cell: &mut RelayCellBody) {
			let mut d_ignored = GenericArray::default();
			cell.set_digest(&mut self.back.digest, &mut d_ignored);
		}
		fn encrypt_inbound(&mut self, cell: &mut RelayCellBody) {
			self.back.cipher.apply_keystream(cell.as_mut());
		}
		fn decrypt_outbound(&mut self, cell: &mut RelayCellBody) -> bool {
			self.fwd.cipher.apply_keystream(cell.as_mut());
			let mut d_ignored = GenericArray::default();
			cell.recognized(&mut self.fwd.digest, &mut d_ignored)
		}
	}

	impl<SC: StreamCipher, D: Digest + Clone> OutboundClientLayer for CryptState<SC, D> {
		fn originate_for(&mut self, cell: &mut RelayCellBody) -> &[u8] {
			cell.set_digest(&mut self.digest, &mut self.last_digest_val);
			self.encrypt_outbound(cell);
			&self.last_digest_val
		}
		fn encrypt_outbound(&mut self, cell: &mut RelayCellBody) {
			self.cipher.apply_keystream(&mut cell.0[..]);
		}
	}

	impl<SC: StreamCipher, D: Digest + Clone> InboundClientLayer for CryptState<SC, D> {
		fn decrypt_inbound(&mut self, cell: &mut RelayCellBody) -> Option<&[u8]> {
			self.cipher.apply_keystream(&mut cell.0[..]);
			if cell.recognized(&mut self.digest, &mut self.last_digest_val) {
				Some(&self.last_digest_val)
			} else {
				None
			}
		}
	}

	pub const CELL_DATA_LEN: usize = 509;
	pub type RawCellBody = [u8; CELL_DATA_LEN];

	/// Type for the body of a relay cell.
	#[derive(Clone)]
	pub struct RelayCellBody(pub RawCellBody);

	impl From<RawCellBody> for RelayCellBody {
		fn from(body: RawCellBody) -> Self {
			RelayCellBody(body)
		}
	}
	impl From<RelayCellBody> for RawCellBody {
		fn from(cell: RelayCellBody) -> Self {
			cell.0
		}
	}
	impl AsRef<[u8]> for RelayCellBody {
		fn as_ref(&self) -> &[u8] {
			&self.0[..]
		}
	}
	impl AsMut<[u8]> for RelayCellBody {
		fn as_mut(&mut self) -> &mut [u8] {
			&mut self.0[..]
		}
	}

	/// A paired object containing an inbound client layer and an outbound
	/// client layer.
	pub trait ClientLayer<F, B>
	where
		F: OutboundClientLayer,
		B: InboundClientLayer,
	{
		/// Consume this ClientLayer and return a paired forward and reverse
		/// crypto layer.
		fn split(self) -> (F, B);
	}

	pub trait CryptInit: Sized {
		/// Return the number of bytes that this state will require.
		fn seed_len() -> usize;
		/// Construct this state from a seed of the appropriate length.
		fn initialize(seed: &[u8]) -> Result<Self, Error>;
		/// Initialize this object from a key generator.
		fn construct<K: KeyGenerator>(keygen: K) -> Result<Self, Error> {
			let seed = keygen.expand(Self::seed_len())?;
			Self::initialize(&seed)
		}
	}

	/// A client's view of the crypto state shared with a single relay, as
	/// used for outbound cells.
	pub trait OutboundClientLayer {
		/// Prepare a RelayCellBody to be sent to the relay at this layer, and
		/// encrypt it.
		///
		/// Return the authentication tag.
		fn originate_for(&mut self, cell: &mut RelayCellBody) -> &[u8];
		/// Encrypt a RelayCellBody to be decrypted by this layer.
		fn encrypt_outbound(&mut self, cell: &mut RelayCellBody);
	}
	pub trait RelayCrypt {
		/// Prepare a RelayCellBody to be sent towards the client.
		fn originate(&mut self, cell: &mut RelayCellBody);
		/// Encrypt a RelayCellBody that is moving towards the client.
		fn encrypt_inbound(&mut self, cell: &mut RelayCellBody);
		/// Decrypt a RelayCellBody that is moving towards
		/// the client.  Return true if it is addressed to us.
		fn decrypt_outbound(&mut self, cell: &mut RelayCellBody) -> bool;
	}

	/// A client's view of the crypto state shared with a single relay, as
	/// used for inbound cells.
	pub trait InboundClientLayer {
		/// Decrypt a CellBody that passed through this layer.
		/// Return an authentication tag if this layer is the originator.
		fn decrypt_inbound(&mut self, cell: &mut RelayCellBody) -> Option<&[u8]>;
	}

	impl RelayCellBody {
		/// Prepare a cell body by setting its digest and recognized field.
		fn set_digest<D: Digest + Clone>(
			&mut self,
			d: &mut D,
			used_digest: &mut GenericArray<u8, D::OutputSize>,
		) {
			self.0[1] = 0;
			self.0[2] = 0;
			self.0[5] = 0;
			self.0[6] = 0;
			self.0[7] = 0;
			self.0[8] = 0;

			d.update(&self.0[..]);
			// TODO(nickm) can we avoid this clone?  Probably not.
			*used_digest = d.clone().finalize();
			self.0[5..9].copy_from_slice(&used_digest[0..4]);
		}
		/// Check a cell to see whether its recognized field is set.
		fn recognized<D: Digest + Clone>(
			&self,
			d: &mut D,
			rcvd: &mut GenericArray<u8, D::OutputSize>,
		) -> bool {
			// Validate 'Recognized' field
			let recognized = u16::from_be_bytes(*array_ref![self.0, 1, 2]);
			if recognized != 0 {
				return false;
			}

			// Now also validate the 'Digest' field:
			let mut dtmp = d.clone();
			// Add bytes up to the 'Digest' field
			dtmp.update(&self.0[..5]);
			// Add zeroes where the 'Digest' field is
			dtmp.update([0_u8; 4]);
			// Add the rest of the bytes
			dtmp.update(&self.0[9..]);
			// Clone the digest before finalize destroys it because we will use
			// it in the future
			let dtmp_clone = dtmp.clone();
			let result = dtmp.finalize();

			if bytes_eq(&self.0[5..9], &result[0..4]) {
				// Copy useful things out of this cell (we keep running digest)
				*d = dtmp_clone;
				*rcvd = result;
				return true;
			}

			false
		}
	}
}
