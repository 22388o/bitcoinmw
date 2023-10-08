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

//! This crate includes the [`crate::Serializable`] trait, the [`crate::Reader`] trait and the
//! [`crate::Writer`] trait. They are separated from the bmw_util crate so that the util crate
//! does not have to be a dependency of bmw_derive and can therefore use the Serializable
//! proc_macro that is included in that crate. The Serializable trait is the key to several of the
//! data structures in the bmw_util crate. It allows a specific way for data to be serialized so
//! that it can be stored in various forms. The Reader and Writer traits are abstractions
//! for reading and writing serializable data structures. The Serializable macro is implemented for
//! several data structures in this crate as well.

use bmw_err::{err, ErrKind, Error};

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
		match self {
			Some(s) => {
				writer.write_u8(1)?;
				s.write(writer)?;
			}
			None => {
				writer.write_u8(0)?;
			}
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
		let e = err!(ErrKind::OperationNotSupported, fmt);
		return Err(e);
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

/// Writer trait used to serializing data.
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
