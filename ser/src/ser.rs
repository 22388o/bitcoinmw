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

use crate::{BinReader, BinWriter, Reader, Serializable, Writer};
use bmw_err::{err, Error};
use std::io::{Read, Write};

/// Serializes a Serializable into any std::io::Write implementation.
pub fn serialize<W: Serializable>(sink: &mut dyn Write, thing: &W) -> Result<(), Error> {
	let mut writer = BinWriter::new(sink);
	thing.write(&mut writer)
}

/// Deserializes a Serializable from any std::io::Read implementation.
pub fn deserialize<T: Serializable, R: Read>(source: &mut R) -> Result<T, Error> {
	let mut reader = BinReader::new(source);
	T::read(&mut reader)
}

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
	fn read_u8(&mut self) -> Result<u8, Error> {
		let mut b = [0u8; 1];
		self.source.read_exact(&mut b)?;
		Ok(b[0])
	}
	fn read_i8(&mut self) -> Result<i8, Error> {
		let mut b = [0u8; 1];
		self.source.read_exact(&mut b)?;
		Ok(b[0] as i8)
	}
	fn read_i16(&mut self) -> Result<i16, Error> {
		let mut b = [0u8; 2];
		self.source.read_exact(&mut b)?;
		Ok(i16::from_be_bytes(b))
	}
	fn read_u16(&mut self) -> Result<u16, Error> {
		let mut b = [0u8; 2];
		self.source.read_exact(&mut b)?;
		Ok(u16::from_be_bytes(b))
	}
	fn read_u32(&mut self) -> Result<u32, Error> {
		let mut b = [0u8; 4];
		self.source.read_exact(&mut b)?;
		Ok(u32::from_be_bytes(b))
	}
	fn read_i32(&mut self) -> Result<i32, Error> {
		let mut b = [0u8; 4];
		self.source.read_exact(&mut b)?;
		Ok(i32::from_be_bytes(b))
	}
	fn read_u64(&mut self) -> Result<u64, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(u64::from_be_bytes(b))
	}
	fn read_i128(&mut self) -> Result<i128, Error> {
		let mut b = [0u8; 16];
		self.source.read_exact(&mut b)?;
		Ok(i128::from_be_bytes(b))
	}
	fn read_usize(&mut self) -> Result<usize, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(usize::from_be_bytes(b))
	}

	fn read_u128(&mut self) -> Result<u128, Error> {
		let mut b = [0u8; 16];
		self.source.read_exact(&mut b)?;
		Ok(u128::from_be_bytes(b))
	}
	fn read_i64(&mut self) -> Result<i64, Error> {
		let mut b = [0u8; 8];
		self.source.read_exact(&mut b)?;
		Ok(i64::from_be_bytes(b))
	}

	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.source.read_exact(buf)?;
		Ok(())
	}

	fn expect_u8(&mut self, val: u8) -> Result<u8, Error> {
		let b = self.read_u8()?;
		if b == val {
			Ok(b)
		} else {
			let fmt = format!("expected: {:?}, received: {:?}", val, b);
			Err(err!(ErrKind::CorruptedData, fmt))
		}
	}
}
