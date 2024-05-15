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

use crate::{err, CoreErrorKind, Error};
use std::io::{Read, Write};

/// This is the trait used by all data structures to serialize and deserialize data.
/// Anything stored in them must implement this trait. Commonly needed implementations
/// are built in the ser module in this crate. These include Vec, String, integer types among
/// other things.
///
/// # Examples
///
///```
/// use bmw_base::*;
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
/// fn ser_helper<S: Serializable + Debug + PartialEq>(input: S) -> Result<(), Error> {
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &input)?;
///     let output: S = deserialize(&mut &v[..])?;
///     assert_eq!(output, input);
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
/// to write numbers, byte vectors, hashes, etc by implementing the [`crate::Writer`]
/// trait.
/// # Also see
/// * [`crate::BinReader`]
/// * [`crate::Reader`]
/// * [`crate::Writer`]
pub struct BinWriter<'a> {
	pub(crate) sink: &'a mut dyn Write,
}

/// Utility wrapper for an underlying byte Reader. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc by implementing the [`crate::Reader`]
/// trait.
/// # Also see
/// * [`crate::BinWriter`]
/// * [`crate::Reader`]
/// * [`crate::Writer`]
pub struct BinReader<'a, R: Read> {
	pub(crate) source: &'a mut R,
}

/// Writer trait used for serializing data.
/// # Examples
///```
/// use bmw_base::*;
///
///    // type that can be used to generate an error
///    #[derive(Debug, PartialEq)]
///    struct SerErr {
///         exp: u8,
///         empty: u8,
///     }
///
///     // serializable trait requires both a Reader/Writer.
///     impl Serializable for SerErr {
///         fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         // read data but return an error unless a specific value is set
///         reader.expect_u8(99)?;
///         reader.read_empty_bytes(1)?;
///         Ok(Self { exp: 99, empty: 0 })
///     }
///
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         // write is regular with no errors
///         writer.write_u8(self.exp)?;
///         writer.write_u8(self.empty)?;
///         Ok(())
///     }
///
/// }
///
/// fn main() -> Result<(), Error> {
///     // test an error
///     let ser_out = SerErr { exp: 100, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_err());
///
///     // test with the values that do not generate an error
///     let ser_out = SerErr { exp: 99, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_ok());
///
///     Ok(())
/// }
///```
/// # Also see
/// * [`crate::Reader`]
/// * [`crate::BinReader`]
/// * [`crate::BinWriter`]
pub trait Writer {
	/// write a u8 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u8`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u8(&mut self, n: u8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n])
	}

	/// write an i8 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i8`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i8(&mut self, n: i8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n as u8])
	}

	/// write a u16 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u16`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u16(&mut self, n: u16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i16 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u16`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i16(&mut self, n: i16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u32 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u32`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u32(&mut self, n: u32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i32 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i32`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i32(&mut self, n: i32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u64 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u64`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u64(&mut self, n: u64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i128 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i128`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i128(&mut self, n: i128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u128 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u128`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u128(&mut self, n: u128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i64 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i64`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i64(&mut self, n: i64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a usize to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`usize`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_usize(&mut self, n: usize) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write `bytes` to the stream and specify the length so that variable length data may be
	/// written to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `bytes` - `AsRef<[u8]>` - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.write_u64(bytes.as_ref().len() as u64)?;
		self.write_fixed_bytes(bytes)
	}

	/// write `bytes` to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `bytes` - `AsRef<[u8]>` - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error>;

	/// write `length` empty (0u8) bytes to a stream.
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `length` - [`usize`] - the length of the empty bytes to write
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			self.write_u8(0)?;
		}
		Ok(())
	}
}

/// Reader trait used for deserializing data.
/// # Examples
///```
/// use bmw_base::*;
///
///    // type that can be used to generate an error
///    #[derive(Debug, PartialEq)]
///    struct SerErr {
///         exp: u8,
///         empty: u8,
///     }
///
///     // serializable trait requires both a Reader/Writer.
///     impl Serializable for SerErr {
///         fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         // read data but return an error unless a specific value is set
///         reader.expect_u8(99)?;
///         reader.read_empty_bytes(1)?;
///         Ok(Self { exp: 99, empty: 0 })
///     }
///
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         // write is regular with no errors
///         writer.write_u8(self.exp)?;
///         writer.write_u8(self.empty)?;
///         Ok(())
///     }
///
/// }
///
/// fn main() -> Result<(), Error> {
///     // test an error
///     let ser_out = SerErr { exp: 100, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_err());
///
///     // test with the values that do not generate an error
///     let ser_out = SerErr { exp: 99, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_ok());
///
///     Ok(())
/// }
///```
/// # Also see
/// * [`crate::Writer`]
/// * [`crate::BinReader`]
/// * [`crate::BinWriter`]
pub trait Reader {
	/// read a fixed length of bytes from the reader store them in `buf`
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `buf` - [&mut `[u8]`] - a mutable reference to a byte array to store the returned
	/// data in.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error>;
	/// read a u8 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u8(&mut self) -> Result<u8, Error> {
		let mut b = [0u8; 1];
		self.read_fixed_bytes(&mut b)?;
		Ok(b[0])
	}
	/// read an i8 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i8(&mut self) -> Result<i8, Error> {
		let mut b = [0u8; 1];
		self.read_fixed_bytes(&mut b)?;
		Ok(b[0] as i8)
	}
	/// read an i16 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i16`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i16(&mut self) -> Result<i16, Error> {
		let mut b = [0u8; 2];
		self.read_fixed_bytes(&mut b)?;
		Ok(i16::from_be_bytes(b))
	}
	/// read a u16 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u16`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u16(&mut self) -> Result<u16, Error> {
		let mut b = [0u8; 2];
		self.read_fixed_bytes(&mut b)?;
		Ok(u16::from_be_bytes(b))
	}
	/// read a u32 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u32`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u32(&mut self) -> Result<u32, Error> {
		let mut b = [0u8; 4];
		self.read_fixed_bytes(&mut b)?;
		Ok(u32::from_be_bytes(b))
	}
	/// read a u64 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u64`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u64(&mut self) -> Result<u64, Error> {
		let mut b = [0u8; 8];
		self.read_fixed_bytes(&mut b)?;
		Ok(u64::from_be_bytes(b))
	}
	/// read a u128 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u128`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u128(&mut self) -> Result<u128, Error> {
		let mut b = [0u8; 16];
		self.read_fixed_bytes(&mut b)?;
		Ok(u128::from_be_bytes(b))
	}
	/// read an i128 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i128`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i128(&mut self) -> Result<i128, Error> {
		let mut b = [0u8; 16];
		self.read_fixed_bytes(&mut b)?;
		Ok(i128::from_be_bytes(b))
	}
	/// read an i32 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i32`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i32(&mut self) -> Result<i32, Error> {
		let mut b = [0u8; 4];
		self.read_fixed_bytes(&mut b)?;
		Ok(i32::from_be_bytes(b))
	}
	/// read an i64 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i64`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i64(&mut self) -> Result<i64, Error> {
		let mut b = [0u8; 8];
		self.read_fixed_bytes(&mut b)?;
		Ok(i64::from_be_bytes(b))
	}
	/// read usize from the Reader.
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`usize`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_usize(&mut self) -> Result<usize, Error> {
		let mut b = [0u8; 8];
		self.read_fixed_bytes(&mut b)?;
		Ok(usize::from_be_bytes(b))
	}
	/// expect a specific byte, otherwise return an error
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `val` - [`u8`] - the expected value to be read.
	/// # Return
	/// The [`u8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::CoreErrorKind::CorruptedData`] - if the byte read is not equal to `val`
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn expect_u8(&mut self, val: u8) -> Result<u8, Error> {
		let b = self.read_u8()?;
		if b == val {
			Ok(b)
		} else {
			let fmt = format!("expected: {:?}, received: {:?}", val, b);
			err!(CoreErrorKind::CorruptedData, fmt)
		}
	}

	/// Read bytes, expect them all to be 0u8. Otherwise, reutrn an error.
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `length` - [`usize`] - the length to read.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::CoreErrorKind::CorruptedData`] - if the byte read is not equal to `val`
	/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			if self.read_u8()? != 0u8 {
				return err!(CoreErrorKind::CorruptedData, "expected 0u8");
			}
		}
		Ok(())
	}
}

/// Serializes a [`crate::Serializable`] into any [`std::io::Write`] implementation.
/// # Input Parameters
/// * `sink` - &mut dyn [`Write`] - any implementation of [`Write`].
/// * `thing` - [`crate::Serializable`] - anything that implements the [`crate::Serializable`]
/// trait.
/// # Errors
/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
/// # Return
/// * [`unit`]
/// # Also see
/// * [`crate::deserialize`]
/// * [`crate::Serializable`]
/// # Examples
///```
/// use bmw_base::*;
///
/// fn main() -> Result<(), Error> {
///     let input = "this is a string which implements serializable".to_string();
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &input)?;
///     let output: String = deserialize(&mut &v[..])?;
///     assert_eq!(output, input);
///
///     Ok(())
/// }
///```
pub fn serialize<W: Serializable>(sink: &mut dyn Write, thing: &W) -> Result<(), Error> {
	let mut writer = BinWriter::new(sink);
	thing.write(&mut writer)
}

/// Deserializes a [`crate::Serializable`] from any [`std::io::Read`] implementation.
/// # Input Parameters
/// * `source` - &mut dyn [`Read`] - any implementation of [`Read`].
/// # Errors
/// * [`crate::CoreErrorKind::IO`] - if an i/o error occurs
/// * [`crate::CoreErrorKind::OperationNotSupported`] - if the serialized data was from a data
/// type that did not allow for it to be deserialized.
/// * [`crate::CoreErrorKind::CorruptedData`] - if the data that was serialized was corrupted.
/// # Return
/// * [`Serializable`] - the serialized object.
/// # Also see
/// * [`crate::serialize`]
/// * [`crate::Serializable`]
/// # Examples
///```
/// use bmw_base::*;
///
/// fn main() -> Result<(), Error> {
///     let input = "this is a string which implements serializable".to_string();
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &input)?;
///     let output: String = deserialize(&mut &v[..])?;
///     assert_eq!(output, input);
///
///     Ok(())
/// }
///```
pub fn deserialize<T: Serializable, R: Read>(source: &mut R) -> Result<T, Error> {
	let mut reader = BinReader::new(source);
	T::read(&mut reader)
}

/// implement Serializable for some commonly used types (primative and standard)

macro_rules! impl_int {
	($int:ty, $w_fn:ident, $r_fn:ident) => {
		impl Serializable for $int {
			fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
				writer.$w_fn(*self)
			}
			fn read<R: Reader>(reader: &mut R) -> Result<$int, Error> {
				reader.$r_fn()
			}
		}
	};
}

impl_int!(u8, write_u8, read_u8);
impl_int!(u16, write_u16, read_u16);
impl_int!(u32, write_u32, read_u32);
impl_int!(i32, write_i32, read_i32);
impl_int!(u64, write_u64, read_u64);
impl_int!(i64, write_i64, read_i64);
impl_int!(i8, write_i8, read_i8);
impl_int!(i16, write_i16, read_i16);
impl_int!(u128, write_u128, read_u128);
impl_int!(i128, write_i128, read_i128);
impl_int!(usize, write_usize, read_usize);

impl Serializable for bool {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		if *self {
			writer.write_u8(1)?;
		} else {
			writer.write_u8(0)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<bool, Error> {
		Ok(reader.read_u8()? != 0)
	}
}

impl Serializable for f64 {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_fixed_bytes(self.to_be_bytes())?;
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<f64, Error> {
		let mut b = [0u8; 8];
		reader.read_fixed_bytes(&mut b)?;
		Ok(f64::from_be_bytes(b))
	}
}

impl Serializable for char {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(*self as u8)
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
		Ok(reader.read_u8()? as char)
	}
}

impl Serializable for () {
	fn write<W: Writer>(&self, _writer: &mut W) -> Result<(), Error> {
		Ok(())
	}
	fn read<R: Reader>(_reader: &mut R) -> Result<(), Error> {
		Ok(())
	}
}

impl<A: Serializable, B: Serializable> Serializable for (A, B) {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		Serializable::write(&self.0, writer)?;
		Serializable::write(&self.1, writer)
	}
	fn read<R: Reader>(reader: &mut R) -> Result<(A, B), Error> {
		Ok((Serializable::read(reader)?, Serializable::read(reader)?))
	}
}

impl<S: Serializable> Serializable for Vec<S> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		let len = self.len();
		writer.write_usize(len)?;
		for i in 0..len {
			Serializable::write(&self[i], writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Vec<S>, Error> {
		let len = reader.read_usize()?;
		let mut v = Vec::with_capacity(len);
		for _ in 0..len {
			v.push(Serializable::read(reader)?);
		}
		Ok(v)
	}
}

impl<S: Serializable> Serializable for Option<S> {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_u8(if self.is_some() { 1 } else { 0 })?;
		if self.is_some() {
			// unwrap is ok because we called is_some as a condition
			self.as_ref().unwrap().write(writer)?;
		}
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<Option<S>, Error> {
		Ok(match reader.read_u8()? {
			0 => None,
			_ => Some(S::read(reader)?),
		})
	}
}

impl Serializable for String {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		writer.write_usize(self.len())?;
		writer.write_fixed_bytes(self.as_bytes())?;
		Ok(())
	}
	fn read<R: Reader>(reader: &mut R) -> Result<String, Error> {
		let mut ret = String::new();
		let len = reader.read_usize()?;
		for _ in 0..len {
			ret.push(reader.read_u8()? as char);
		}
		Ok(ret)
	}
}

impl<S> Serializable for &S
where
	S: Serializable,
{
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
		S::write(self, writer)?;
		Ok(())
	}
	fn read<R: Reader>(_reader: &mut R) -> Result<Self, Error> {
		let fmt = "not implemented for reading";
		err!(CoreErrorKind::OperationNotSupported, fmt)
	}
}

macro_rules! impl_arr {
	($count:expr) => {
		impl Serializable for [u8; $count] {
			fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
				writer.write_fixed_bytes(self)?;
				Ok(())
			}
			fn read<R: Reader>(reader: &mut R) -> Result<[u8; $count], Error> {
				let mut r = [0u8; $count];
				reader.read_fixed_bytes(&mut r)?;
				Ok(r)
			}
		}
	};
}

impl_arr!(1);
impl_arr!(2);
impl_arr!(3);
impl_arr!(4);
impl_arr!(5);
impl_arr!(6);
impl_arr!(7);
impl_arr!(8);
impl_arr!(9);
impl_arr!(10);
impl_arr!(11);
impl_arr!(12);
impl_arr!(13);
impl_arr!(14);
impl_arr!(15);
impl_arr!(16);
impl_arr!(17);
impl_arr!(18);
impl_arr!(19);
impl_arr!(20);
impl_arr!(21);
impl_arr!(22);
impl_arr!(23);
impl_arr!(24);
impl_arr!(25);
impl_arr!(26);
impl_arr!(27);
impl_arr!(28);
impl_arr!(29);
impl_arr!(30);
impl_arr!(31);
impl_arr!(32);

impl<'a> BinWriter<'a> {
	/// Wraps a standard Write in a new BinWriter
	pub fn new(sink: &'a mut dyn Write) -> BinWriter<'a> {
		BinWriter { sink }
	}
}

impl<'a> Writer for BinWriter<'a> {
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.sink.write_all(bytes.as_ref())?;
		Ok(())
	}
}

impl<'a, R: Read> BinReader<'a, R> {
	/// Constructor for a new BinReader for the provided source
	pub fn new(source: &'a mut R) -> Self {
		BinReader { source }
	}
}

impl<'a, R: Read> Reader for BinReader<'a, R> {
	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.source.read_exact(buf)?;
		Ok(())
	}
}
