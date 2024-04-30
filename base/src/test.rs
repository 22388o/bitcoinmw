// Copyright (c) 2023-2024, The BitcoinMW Developers
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

#[cfg(test)]
mod test {
	#[cfg(unix)]
	use bmw_deps::nix::errno::Errno;

	use crate::{
		cbreak, deserialize, err, err_only, kind, map_err, serialize, try_into, CoreErrorKind,
		Error, ErrorKind, Reader, Serializable, Writer,
	};
	use bmw_deps::rand;
	use bmw_deps::url::{ParseError, Url};
	use std::alloc::{Layout, LayoutError};
	use std::ffi::OsString;
	use std::fmt::Debug;
	use std::net::{AddrParseError, IpAddr};
	use std::num::ParseIntError;
	use std::num::TryFromIntError;
	use std::str::from_utf8;
	use std::str::ParseBoolError;
	use std::sync::mpsc::sync_channel;
	use std::sync::{Arc, Mutex, RwLock};
	use std::thread::spawn;
	use std::time::{Duration, SystemTime, SystemTimeError};

	// function to return a parse error
	fn ret_err() -> Result<(), Error> {
		err!(CoreErrorKind::Parse, "this is a test {}", 1)
	}

	// function to return a different parse error
	fn ret_err2() -> Result<(), Error> {
		err!(CoreErrorKind::Parse, "this is a test")
	}

	// function to generate a parse int error
	fn ret_err3() -> Result<usize, ParseIntError> {
		"".parse::<usize>()
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		// assert that the first function returns an err
		assert!(ret_err().is_err());

		// do a conversion and check the kind
		let err: Error = ret_err().unwrap_err();
		let kind = err.kind();

		assert_eq!(kind, &kind!(CoreErrorKind::Parse, "this is a test 1"));
		assert_ne!(kind, &kind!(CoreErrorKind::Parse, "this is a test 2"));

		// likewise with ret_err2
		let err: Error = ret_err2().unwrap_err();
		assert_eq!(err.kind(), &kind!(CoreErrorKind::Parse, "this is a test"));

		Ok(())
	}

	#[test]
	fn test_map_err() -> Result<(), Error> {
		// test that the map_err macro properly maps errors to the correct kinds
		let e = map_err!(ret_err3(), CoreErrorKind::Parse, "1").unwrap_err();
		let exp_text = "1: cannot parse integer from empty string";
		assert_eq!(e.kind(), &kind!(CoreErrorKind::Parse, exp_text));

		let e = map_err!(ret_err3(), CoreErrorKind::Parse).unwrap_err();
		let exp_text = "cannot parse integer from empty string";
		assert_eq!(e.kind(), &kind!(CoreErrorKind::Parse, exp_text));

		Ok(())
	}

	#[test]
	fn test_cbreak() -> Result<(), Error> {
		// test the conditional break macro
		let mut count = 0;
		loop {
			count += 1;
			cbreak!(count == 10);
		}
		assert_eq!(count, 10);
		Ok(())
	}

	#[test]
	fn test_try_into() -> Result<(), Error> {
		// test the try_into macro

		// with a u64
		let x: u64 = try_into!(100u32)?;
		assert_eq!(x, 100u64);

		// u32
		let x: u32 = try_into!(100u64)?;
		assert_eq!(x, 100u32);

		// now gernate an error u64::MAX always fails being converted to u32
		let x: Result<u32, Error> = try_into!(u64::MAX);
		let exp_text = "out of range integral type conversion attempted";
		assert_eq!(
			x.unwrap_err().kind(),
			&kind!(CoreErrorKind::TryInto, exp_text)
		);
		Ok(())
	}

	// type that can be used to generate an error
	#[derive(Debug, PartialEq)]
	struct SerErr {
		exp: u8,
		empty: u8,
	}

	impl Serializable for SerErr {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			// read data but return an error unless a specific value is set
			reader.expect_u8(99)?;
			reader.read_empty_bytes(1)?;
			Ok(Self { exp: 99, empty: 0 })
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			// write is regular with no errors
			writer.write_u8(self.exp)?;
			writer.write_u8(self.empty)?;
			Ok(())
		}
	}

	// helper function that serializes and deserializes a Serializable and tests them for
	// equality
	fn ser_helper<S: Serializable + Debug + PartialEq>(ser_out: S) -> Result<(), Error> {
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: S = deserialize(&mut &v[..])?;
		assert_eq!(ser_in, ser_out);
		Ok(())
	}

	// struct with all types
	#[derive(Debug, PartialEq)]
	struct SerAll {
		a: u8,
		b: i8,
		c: u16,
		d: i16,
		e: u32,
		f: i32,
		g: u64,
		h: i64,
		i: u128,
		j: i128,
		k: usize,
		l: bool,
		m: f64,
		n: char,
		v: Vec<u8>,
		o: Option<u8>,
	}

	// read/write with some added data to exercise all functions in the interface
	impl Serializable for SerAll {
		fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
			let a = reader.read_u8()?;
			let b = reader.read_i8()?;
			let c = reader.read_u16()?;
			let d = reader.read_i16()?;
			let e = reader.read_u32()?;
			let f = reader.read_i32()?;
			let g = reader.read_u64()?;
			let h = reader.read_i64()?;
			let i = reader.read_u128()?;
			let j = reader.read_i128()?;
			let k = reader.read_usize()?;
			let l = bool::read(reader)?;
			let m = f64::read(reader)?;
			let n = char::read(reader)?;
			let v = Vec::read(reader)?;
			let o = Option::read(reader)?;
			reader.expect_u8(100)?;
			assert_eq!(reader.read_u64()?, 4);
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_u8()?;
			reader.read_empty_bytes(10)?;

			let ret = Self {
				a,
				b,
				c,
				d,
				e,
				f,
				g,
				h,
				i,
				j,
				k,
				l,
				m,
				n,
				v,
				o,
			};

			Ok(ret)
		}
		fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
			writer.write_u8(self.a)?;
			writer.write_i8(self.b)?;
			writer.write_u16(self.c)?;
			writer.write_i16(self.d)?;
			writer.write_u32(self.e)?;
			writer.write_i32(self.f)?;
			writer.write_u64(self.g)?;
			writer.write_i64(self.h)?;
			writer.write_u128(self.i)?;
			writer.write_i128(self.j)?;
			writer.write_usize(self.k)?;
			bool::write(&self.l, writer)?;
			f64::write(&self.m, writer)?;
			char::write(&self.n, writer)?;
			Vec::write(&self.v, writer)?;
			Option::write(&self.o, writer)?;
			writer.write_u8(100)?;
			writer.write_bytes([1, 2, 3, 4])?;
			writer.write_empty_bytes(10)?;
			Ok(())
		}
	}

	#[test]
	fn test_ser() -> Result<(), Error> {
		// create a SerAll with random values
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: false,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: Some(rand::random()),
		};

		// test it
		ser_helper(ser_out)?;

		// create again with some other options
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: true,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: None,
		};

		// test it
		ser_helper(ser_out)?;

		// test with ()
		ser_helper(())?;
		// test with a tuple
		ser_helper((rand::random::<u32>(), rand::random::<i128>()))?;

		// test with a string
		ser_helper(("hi there".to_string(), 123))?;

		// test an array
		let x = [3u8; 8];
		ser_helper(x)?;

		// test an error
		let ser_out = SerErr { exp: 100, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		// test with the values that do not generate an error
		let ser_out = SerErr { exp: 99, empty: 0 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_ok());

		// generate an error again
		let ser_out = SerErr { exp: 99, empty: 1 };
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_out)?;
		let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
		assert!(ser_in.is_err());

		// test a vec of strings
		let v = vec!["test1".to_string(), "a".to_string(), "okokok".to_string()];
		ser_helper(v)?;

		// test a ref to a string (read is an error beacuse we can't return a reference
		// from read).
		let s = "abc".to_string();
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &&s)?;
		let s1: Result<&String, Error> = deserialize(&mut &v[..]);
		assert!(s1.is_err());

		Ok(())
	}

	// return the os string
	fn get_os_string() -> Result<(), Error> {
		Err(OsString::new().into())
	}

	// hide invalid utf8 error by wrapping this in this fn
	#[allow(invalid_from_utf8)]
	fn get_utf8() -> Result<String, Error> {
		Ok(from_utf8(&[0xC0])?.to_string())
	}

	// test to generate all the From conversions we implement and then convert them into our
	// Error struct and compare the kinds to make sure it's what we expect
	#[test]
	fn test_error_conversions() -> Result<(), Error> {
		let err1 = err_only!(CoreErrorKind::Parse, "test");
		let err2: Error = CoreErrorKind::Parse("test".to_string()).into();
		assert_eq!(err1, err2);

		assert!(err1.cause().is_none());
		assert!(err1.backtrace().is_some());

		assert_eq!(err1.inner(), err2.inner());

		let kind: Box<dyn ErrorKind> = Box::new(CoreErrorKind::Misc("".to_string()));
		let err: Error = kind.into();
		assert_eq!(err.kind(), &kind!(CoreErrorKind::Misc, ""));

		let ioe = std::io::Error::new(std::io::ErrorKind::NotFound, "uh oh");
		let ioe: Error = ioe.into();
		assert_eq!(ioe, err_only!(CoreErrorKind::IO, "uh oh"));

		let err: Error = "".parse::<usize>().unwrap_err().into();
		assert_eq!(
			err,
			err_only!(
				CoreErrorKind::Parse,
				"cannot parse integer from empty string"
			)
		);

		let err1: Error = get_os_string().unwrap_err();
		let err2: Result<OsString, Error> = err!(CoreErrorKind::OsString, "\"\"");
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Error = get_utf8().unwrap_err();
		let err2: Result<String, Error> = err!(
			CoreErrorKind::Utf8,
			"invalid utf-8 sequence of 1 bytes from index 0"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Result<u32, TryFromIntError> = u64::MAX.try_into();
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(
			CoreErrorKind::TryFrom,
			"out of range integral type conversion attempted"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Result<i32, ParseIntError> = i32::from_str_radix("a12", 10);
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(CoreErrorKind::Parse, "invalid digit found in string");
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Result<Url, ParseError> = Url::parse("http://[:::1]");
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(CoreErrorKind::Parse, "invalid IPv6 address");
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let mutex = Arc::new(Mutex::new(1));
		// poison the mutex
		let c_mutex = Arc::clone(&mutex);
		let _ = spawn(move || {
			let mut data = c_mutex.lock().unwrap();
			*data = 2;
			panic!();
		})
		.join();

		let err1 = mutex.lock().unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(
			CoreErrorKind::Poison,
			"poisoned lock: another task failed inside"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let rwlock = Arc::new(RwLock::new(1));
		// poison the rwlock
		let c_rwlock = Arc::clone(&rwlock);
		let _ = spawn(move || {
			let mut data = c_rwlock.write().unwrap();
			*data = 2;
			panic!();
		})
		.join();

		let err1 = rwlock.write().unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(
			CoreErrorKind::Poison,
			"poisoned lock: another task failed inside"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let rwlock = Arc::new(RwLock::new(1));
		// poison the rwlock
		let c_rwlock = Arc::clone(&rwlock);
		let _ = spawn(move || {
			let mut data = c_rwlock.write().unwrap();
			*data = 2;
			panic!();
		})
		.join();

		let err1 = rwlock.read().unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<u32, Error> = err!(
			CoreErrorKind::Poison,
			"poisoned lock: another task failed inside"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let x = Arc::new(RwLock::new(false));
		let x_clone = x.clone();
		let (tx_outer, rx_outer) = sync_channel::<()>(1);
		{
			let (_tx, rx) = sync_channel::<()>(1);

			let _ = spawn(move || {
				let err1 = rx.recv().unwrap_err();
				let err1: Error = err1.into();
				let err2: Result<u32, Error> =
					err!(CoreErrorKind::IllegalState, "receiving on a closed channel");
				let err2 = err2.unwrap_err();
				assert_eq!(err1.kind(), err2.kind());
				let mut guard = x.write().unwrap();
				*guard = true;
				let _ = tx_outer.send(());
			});
		}

		rx_outer.recv()?;
		let guard = x_clone.read()?;
		assert_eq!(*guard, true);

		let (tx_outer, rx_outer) = sync_channel::<()>(1);
		let (tx_outer_outer, rx_outer_outer) = sync_channel::<()>(1);
		let x = Arc::new(RwLock::new(false));
		let x_clone = x.clone();

		{
			let (tx, _rx) = sync_channel::<()>(1);
			let _ = spawn(move || {
				rx_outer.recv().unwrap();
				let err1 = tx.send(());
				let err1 = err1.unwrap_err();
				let err1: Error = err1.into();
				let err2: Result<u32, Error> =
					err!(CoreErrorKind::IllegalState, "sending on a closed channel");
				let err2 = err2.unwrap_err();
				assert_eq!(err1.kind(), err2.kind());
				let mut guard = x.write().unwrap();
				*guard = true;
				tx_outer_outer.send(()).unwrap();
			});
		}
		tx_outer.send(())?;
		rx_outer_outer.recv()?;

		let guard = x_clone.read()?;
		assert_eq!(*guard, true);

		let now = SystemTime::now();
		let err1: Result<Duration, SystemTimeError> = now
			.checked_add(Duration::from_millis(1_000_000))
			.unwrap()
			.duration_since(now.checked_add(Duration::from_millis(2_000_000)).unwrap());

		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(
			CoreErrorKind::SystemTime,
			"second time provided was later than self"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Result<IpAddr, AddrParseError> = "1234".parse();
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(CoreErrorKind::Parse, "invalid IP address syntax");
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let err1: Result<Layout, LayoutError> = Layout::from_size_align(7, 7);
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(
			CoreErrorKind::Alloc,
			"layout error: invalid parameters to Layout::from_size_align"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let bytes = vec![0, 159];
		let value = String::from_utf8(bytes);
		let err1 = value.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(
			CoreErrorKind::Utf8,
			"invalid utf-8 sequence of 1 bytes from index 1"
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		let x: Result<bool, ParseBoolError> = "x".parse();
		let err1 = x.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(
			CoreErrorKind::Parse,
			"provided string was not `true` or `false`",
		);
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());

		assert!(format!("{}", err2).len() > 0);

		Ok(())
	}

	// unix specific test of nix crate
	#[test]
	#[cfg(unix)]
	fn test_nix() -> Result<(), Error> {
		let err1: Result<(), Errno> = Err(Errno::EIO);
		let err1 = err1.unwrap_err();
		let err1: Error = err1.into();
		let err2: Result<Duration, Error> = err!(CoreErrorKind::Errno, "EIO: I/O error");
		let err2 = err2.unwrap_err();
		assert_eq!(err1.kind(), err2.kind());
		Ok(())
	}
}
