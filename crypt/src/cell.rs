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

use crate::constants::*;
use crate::types::{Info, Padding};
use crate::{Cell, Peer};
use bmw_deps::ed25519_dalek::PublicKey;
use bmw_err::*;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

impl Info {
	fn to_bytes(&self, buf: &mut [u8]) -> Result<(), Error> {
		if self.local_peer.nickname.len() > MAX_NICKNAME_LEN {
			return Err(err!(
				ErrKind::IllegalArgument,
				format!(
					"nickname must be less than or equal to {} bytes",
					MAX_NICKNAME_LEN
				)
			));
		}
		buf[0] = CELL_TYPE_INFO;
		buf[17..49].clone_from_slice(self.local_peer.pubkey.as_bytes());
		match self.local_peer.sockaddr {
			SocketAddr::V4(sockaddr) => {
				buf[49] = 0;
				let octets = sockaddr.ip().octets();
				buf[50..54].clone_from_slice(&octets);
				buf[54..56].clone_from_slice(&sockaddr.port().to_be_bytes());
				buf[56] = self.local_peer.nickname.len() as u8;
				buf[57..57 + self.local_peer.nickname.len()]
					.clone_from_slice(self.local_peer.nickname.as_bytes());
			}
			SocketAddr::V6(sockaddr) => {
				buf[49] = 1;
				let segments = sockaddr.ip().segments();
				let mut x = 50;
				for i in 0..8 {
					buf[x..x + 2].clone_from_slice(&segments[i].to_be_bytes());
					x += 2;
				}
				buf[66..68].clone_from_slice(&sockaddr.port().to_be_bytes());
				buf[68] = self.local_peer.nickname.len() as u8;
				buf[69..69 + self.local_peer.nickname.len()]
					.clone_from_slice(self.local_peer.nickname.as_bytes());
			}
		}
		Ok(())
	}

	fn from_bytes(buf: &[u8]) -> Result<Self, Error> {
		let pubkey = PublicKey::from_bytes(&buf[17..49])?;
		let sockaddr;
		let nickname;
		match buf[49] {
			0 => {
				sockaddr = SocketAddr::V4(SocketAddrV4::new(
					Ipv4Addr::new(buf[50], buf[51], buf[52], buf[53]),
					u16::from_be_bytes(buf[54..56].try_into()?),
				));
				let name_len = buf[56] as usize;
				nickname = std::str::from_utf8(&buf[57..57 + name_len])?.to_string();
			}
			1 => {
				sockaddr = SocketAddr::V6(SocketAddrV6::new(
					Ipv6Addr::new(
						u16::from_be_bytes(buf[50..52].try_into()?),
						u16::from_be_bytes(buf[52..54].try_into()?),
						u16::from_be_bytes(buf[54..56].try_into()?),
						u16::from_be_bytes(buf[56..58].try_into()?),
						u16::from_be_bytes(buf[58..60].try_into()?),
						u16::from_be_bytes(buf[60..62].try_into()?),
						u16::from_be_bytes(buf[62..64].try_into()?),
						u16::from_be_bytes(buf[64..66].try_into()?),
					),
					u16::from_be_bytes(buf[66..68].try_into()?),
					0,
					0,
				));
				let name_len = buf[68] as usize;
				nickname = std::str::from_utf8(&buf[68..68 + name_len])?.to_string();
			}
			_ => return Err(err!(ErrKind::CorruptedData, "unexpected sockaddr type")),
		}

		Ok(Self {
			local_peer: Peer {
				nickname,
				pubkey,
				sockaddr,
			},
		})
	}
}

impl Padding {
	fn to_bytes(&self, _buf: &mut [u8]) -> Result<(), Error> {
		Ok(())
	}
	fn from_bytes(_buf: &[u8]) -> Result<Self, Error> {
		Ok(Self {})
	}
}

impl Cell {
	pub(crate) fn to_bytes(&self, buf: &mut [u8]) -> Result<(), Error> {
		if buf.len() != CELL_LEN {
			return Err(err!(
				ErrKind::Crypt,
				format!("buf len must be {} bytes", CELL_LEN)
			));
		}
		match self {
			Cell::Info(info) => info.to_bytes(buf)?,
			Cell::Padding(padding) => padding.to_bytes(buf)?,
		}
		Ok(())
	}

	pub(crate) fn from_bytes(buf: &[u8]) -> Result<Self, Error> {
		if buf.len() != CELL_LEN {
			let fmt = format!(
				"Invalid cell length. Cells must be {} bytes. Input was {} bytes.",
				CELL_LEN,
				buf.len(),
			);
			return Err(err!(ErrKind::Crypt, fmt));
		}

		match buf[0] {
			CELL_TYPE_INFO => Ok(Cell::Info(Info::from_bytes(buf)?)),
			_ => Err(err!(
				ErrKind::CorruptedData,
				"cell contained an unknown CELL_TYPE"
			)),
		}
	}
}
