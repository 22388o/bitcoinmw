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
	/// write a u8 to the stream
	fn write_u8(&mut self, n: u8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n])
	}

	/// write an i8 to the stream
	fn write_i8(&mut self, n: i8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n as u8])
	}

	/// write a u16 to the stream
	fn write_u16(&mut self, n: u16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i16 to the stream
	fn write_i16(&mut self, n: i16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u32 to the stream
	fn write_u32(&mut self, n: u32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i32 to the stream
	fn write_i32(&mut self, n: i32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u64 to the stream
	fn write_u64(&mut self, n: u64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i128 to the stream
	fn write_i128(&mut self, n: i128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u128 to the stream
	fn write_u128(&mut self, n: u128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i64 to the stream
	fn write_i64(&mut self, n: i64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a usize to the stream
	fn write_usize(&mut self, n: usize) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write `bytes` to the stream and specify the length so that variable length data may be
	/// written to the stream
	fn write_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.write_u64(bytes.as_ref().len() as u64)?;
		self.write_fixed_bytes(bytes)
	}

	/// write `bytes` to the stream
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error>;

	/// write `length` empty (0u8) bytes to a stream.
	fn write_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			self.write_u8(0)?;
		}
		Ok(())
	}
}

/// Reader trait used for deserializing data.
pub trait Reader {
	/// read a u8 from the reader and return the value
	fn read_u8(&mut self) -> Result<u8, Error>;
	/// read an i8 from the reader and return the value
	fn read_i8(&mut self) -> Result<i8, Error>;
	/// read an i16 from the reader and return the value
	fn read_i16(&mut self) -> Result<i16, Error>;
	/// read a u16 from the reader and return the value
	fn read_u16(&mut self) -> Result<u16, Error>;
	/// read a u32 from the reader and return the value
	fn read_u32(&mut self) -> Result<u32, Error>;
	/// read a u64 from the reader and return the value
	fn read_u64(&mut self) -> Result<u64, Error>;
	/// read a u128 from the reader and return the value
	fn read_u128(&mut self) -> Result<u128, Error>;
	/// read an i128 from the reader and return the value
	fn read_i128(&mut self) -> Result<i128, Error>;
	/// read an i32 from the reader and return the value
	fn read_i32(&mut self) -> Result<i32, Error>;
	/// read an i64 from the reader and return the value
	fn read_i64(&mut self) -> Result<i64, Error>;
	/// read a fixed length of bytes from the reader store them in `buf`
	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error>;
	/// read usize from the Reader.
	fn read_usize(&mut self) -> Result<usize, Error>;
	/// expect a specific byte, otherwise return an error
	fn expect_u8(&mut self, val: u8) -> Result<u8, Error>;

	/// Read bytes, expect them all to be 0u8. Otherwise, reutrn an error.
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
///
/// # Examples
///
///```
/// use bmw_err::*;
/// use bmw_ser::*;
/// use std::fmt::Debug;
///
/// #[derive(Debug, PartialEq)]
/// struct SerEx {
///     a: u8,
///     b: u128,
/// }
///
/// impl Serializable for SerEx {
///     fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         let a = reader.read_u8()?;
///         let b = reader.read_u128()?;
///
///         let ret = Self {
///             a: a,
///             b,
///         };
///
///         Ok(ret)
///     }
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         writer.write_u8(self.a)?;
///         writer.write_u128(self.b)?;
///         Ok(())
///     }
/// }
///
/// // helper function that serializes and deserializes a Serializable and tests them for
/// // equality
/// fn ser_helper<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: S = deserialize(&mut &v[..])?;
///     assert_eq!(ser_in, ser_out);
///     Ok(())
/// }
///
/// fn main() -> Result<(), Error> {
///     let v = SerEx {
///         a: 100,
///         b: 1_000,
///     };
///     ser_helper(v)?;
///     Ok(())
/// }
///```
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
