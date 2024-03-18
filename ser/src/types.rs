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

use bmw_err::{err, Error};
use std::io::{Read, Write};

/// Writer trait used for serializing data.
pub trait Writer {
	fn write_u8(&mut self, n: u8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n])
	}

	fn write_i8(&mut self, n: i8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n as u8])
	}

	fn write_u16(&mut self, n: u16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_i16(&mut self, n: i16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_u32(&mut self, n: u32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_i32(&mut self, n: i32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_u64(&mut self, n: u64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_i128(&mut self, n: i128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_u128(&mut self, n: u128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_i64(&mut self, n: i64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_usize(&mut self, n: usize) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	fn write_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.write_u64(bytes.as_ref().len() as u64)?;
		self.write_fixed_bytes(bytes)
	}

	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error>;

	fn write_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			self.write_u8(0)?;
		}
		Ok(())
	}
}

/// Reader trait used for deserializing data.
pub trait Reader {
	fn read_u8(&mut self) -> Result<u8, Error>;
	fn read_i8(&mut self) -> Result<i8, Error>;
	fn read_i16(&mut self) -> Result<i16, Error>;
	fn read_u16(&mut self) -> Result<u16, Error>;
	fn read_u32(&mut self) -> Result<u32, Error>;
	fn read_u64(&mut self) -> Result<u64, Error>;
	fn read_u128(&mut self) -> Result<u128, Error>;
	fn read_i128(&mut self) -> Result<i128, Error>;
	fn read_i32(&mut self) -> Result<i32, Error>;
	fn read_i64(&mut self) -> Result<i64, Error>;
	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error>;
	fn read_usize(&mut self) -> Result<usize, Error>;
	fn expect_u8(&mut self, val: u8) -> Result<u8, Error>;

	fn read_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			if self.read_u8()? != 0u8 {
				return Err(err!(ErrKind::CorruptedData, "expected 0u8"));
			}
		}
		Ok(())
	}
}

/// This is the trait used by all data structures to serialize and deserialize data.
/// Anything stored in them must implement this trait. Commonly needed implementations
/// are built in the ser module in this crate. These include Vec, String, integer types among
/// other things.
pub trait Serializable {
	/// read data from the reader and build the underlying type represented by that
	/// data.
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error>
	where
		Self: Sized;
	/// write data to the writer representing the underlying type.
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error>;
}

/// Utility wrapper for an underlying byte Writer. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc.
pub struct BinWriter<'a> {
	pub(crate) sink: &'a mut dyn Write,
}

/// Utility wrapper for an underlying byte Reader. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc.
pub struct BinReader<'a, R: Read> {
	pub(crate) source: &'a mut R,
}
