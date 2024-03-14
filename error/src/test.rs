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
	use crate as bmw_err;
	use crate::{err, map_err, ErrKind, Error, ErrorKind};
	use bmw_deps::rustls::pki_types::{InvalidDnsNameError, ServerName};
	use bmw_deps::substring::Substring;
	use bmw_deps::url::ParseError;
	use bmw_deps::webpki::Error as WebpkiError;
	use std::alloc::Layout;
	use std::convert::TryInto;
	use std::ffi::OsString;
	use std::fs::File;
	use std::net::{AddrParseError, IpAddr};
	use std::num::TryFromIntError;
	use std::string::FromUtf8Error;
	use std::sync::mpsc::channel;
	use std::sync::{Arc, Mutex, RwLock};
	use std::time::{Duration, SystemTime, SystemTimeError};

	fn test_kind(k: ErrKind, s: &str, error: Error) -> Result<(), Error> {
		let err: bmw_err::Error = err!(k, s);
		let err_kind = err.kind();
		println!("error.kind={:?},err_kind={:?}", error.kind(), err_kind);
		assert_eq!(error.kind(), err_kind);
		Ok(())
	}
	#[test]
	fn test_ekinds() -> Result<(), crate::Error> {
		let s = "s";
		let ss = "s".to_string();

		test_kind(ErrKind::Http, s, ErrorKind::Http(ss.clone()).into())?;
		test_kind(ErrKind::Crypt, s, ErrorKind::Crypt(ss.clone()).into())?;
		test_kind(ErrKind::Rustls, s, ErrorKind::Rustls(ss.clone()).into())?;
		test_kind(ErrKind::Errno, s, ErrorKind::Errno(ss.clone()).into())?;
		test_kind(
			ErrKind::SystemTime,
			s,
			ErrorKind::SystemTime(ss.clone()).into(),
		)?;
		test_kind(
			ErrKind::OperationNotSupported,
			s,
			ErrorKind::OperationNotSupported(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Alloc, s, ErrorKind::Alloc(ss.clone()).into())?;
		test_kind(
			ErrKind::ThreadPanic,
			s,
			ErrorKind::ThreadPanic(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Overflow, s, ErrorKind::Overflow(ss.clone()).into())?;
		test_kind(ErrKind::Test, s, ErrorKind::Test(ss.clone()).into())?;
		test_kind(
			ErrKind::IllegalState,
			s,
			ErrorKind::IllegalState(ss.clone()).into(),
		)?;
		test_kind(
			ErrKind::CorruptedData,
			s,
			ErrorKind::CorruptedData(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Misc, s, ErrorKind::Misc(ss.clone()).into())?;
		test_kind(ErrKind::Poison, s, ErrorKind::Poison(ss.clone()).into())?;
		test_kind(
			ErrKind::IllegalArgument,
			s,
			ErrorKind::IllegalArgument(ss.clone()).into(),
		)?;
		test_kind(
			ErrKind::CapacityExceeded,
			s,
			ErrorKind::CapacityExceeded(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Timeout, s, ErrorKind::Timeout(ss.clone()).into())?;
		test_kind(
			ErrKind::ArrayIndexOutOfBounds,
			s,
			ErrorKind::ArrayIndexOutOfBounds(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Utf8, s, ErrorKind::Utf8(ss.clone()).into())?;
		test_kind(
			ErrKind::UnexpectedEof,
			s,
			ErrorKind::UnexpectedEof(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Log, s, ErrorKind::Log(ss.clone()).into())?;
		test_kind(ErrKind::IO, s, ErrorKind::IO(ss.clone()).into())?;
		test_kind(
			ErrKind::Configuration,
			s,
			ErrorKind::Configuration(ss.clone()).into(),
		)?;
		test_kind(ErrKind::Rustlet, s, ErrorKind::Rustlet(ss.clone()).into())?;

		Ok(())
	}

	fn test_map(err_kind: ErrKind, _error_kind: ErrorKind) -> Result<(), Error> {
		let map: Result<usize, Error> = map_err!((-1).try_into(), err_kind);
		let kind = map.unwrap_err().kind();
		assert_eq!(kind, _error_kind);
		Ok(())
	}

	#[test]
	fn test_map_err() -> Result<(), crate::Error> {
		let res = map_err!(
			File::open("/path/to/nothing"),
			bmw_err::ErrKind::Log,
			"another msg"
		);

		assert!(matches!(
			res.as_ref().unwrap_err().kind(),
			crate::ErrorKind::Log(_),
		));

		let res = map_err!(File::open("/path/to/nothing"), bmw_err::ErrKind::IO);
		assert!(matches!(
			res.as_ref().unwrap_err().kind(),
			crate::ErrorKind::IO(_),
		));

		let x: Result<i32, TryFromIntError> = u64::MAX.try_into();
		let map = map_err!(x, ErrKind::Misc);
		assert!(matches!(map.unwrap_err().kind(), crate::ErrorKind::Misc(_)));

		let map = map_err!(x, ErrKind::Poison);
		let kind = map.unwrap_err().kind();
		let _poison = crate::ErrorKind::Poison("".to_string());
		assert!(matches!(kind, _poison));

		let map = map_err!(x, ErrKind::IllegalArgument);
		let kind = map.unwrap_err().kind();
		let _arg = crate::ErrorKind::IllegalArgument("".to_string());
		assert!(matches!(kind, _arg));

		let s = ": out of range integral type conversion attempted".to_string();
		test_map(ErrKind::Http, ErrorKind::Http(s.clone()).into())?;
		test_map(
			ErrKind::Configuration,
			ErrorKind::Configuration(s.clone()).into(),
		)?;
		test_map(ErrKind::IO, ErrorKind::IO(s.clone()).into())?;
		test_map(ErrKind::Log, ErrorKind::Log(s.clone()).into())?;
		test_map(
			ErrKind::UnexpectedEof,
			ErrorKind::UnexpectedEof(s.clone()).into(),
		)?;
		test_map(ErrKind::Utf8, ErrorKind::Utf8(s.clone()).into())?;
		test_map(
			ErrKind::ArrayIndexOutOfBounds,
			ErrorKind::ArrayIndexOutOfBounds(s.clone()).into(),
		)?;
		test_map(ErrKind::Timeout, ErrorKind::Timeout(s.clone()).into())?;
		test_map(
			ErrKind::CapacityExceeded,
			ErrorKind::CapacityExceeded(s.clone()).into(),
		)?;
		test_map(
			ErrKind::IllegalArgument,
			ErrorKind::IllegalArgument(s.clone()).into(),
		)?;
		test_map(ErrKind::Poison, ErrorKind::Poison(s.clone()).into())?;
		test_map(ErrKind::Misc, ErrorKind::Misc(s.clone()).into())?;
		test_map(
			ErrKind::CorruptedData,
			ErrorKind::CorruptedData(s.clone()).into(),
		)?;
		test_map(
			ErrKind::IllegalState,
			ErrorKind::IllegalState(s.clone()).into(),
		)?;
		test_map(ErrKind::Test, ErrorKind::Test(s.clone()).into())?;
		test_map(ErrKind::Overflow, ErrorKind::Overflow(s.clone()).into())?;
		test_map(
			ErrKind::ThreadPanic,
			ErrorKind::ThreadPanic(s.clone()).into(),
		)?;
		test_map(ErrKind::Alloc, ErrorKind::Alloc(s.clone()).into())?;
		test_map(
			ErrKind::OperationNotSupported,
			ErrorKind::OperationNotSupported(s.clone()).into(),
		)?;
		test_map(ErrKind::SystemTime, ErrorKind::SystemTime(s.clone()).into())?;
		test_map(ErrKind::Errno, ErrorKind::Errno(s.clone()).into())?;
		test_map(ErrKind::Rustls, ErrorKind::Rustls(s.clone()).into())?;
		test_map(ErrKind::Crypt, ErrorKind::Crypt(s.clone()).into())?;
		test_map(ErrKind::Rustlet, ErrorKind::Rustlet(s.clone()).into())?;

		Ok(())
	}

	fn get_os_string() -> Result<(), Error> {
		Err(OsString::new().into())
	}

	fn check_error<T: Sized, Q>(r: Result<T, Q>, ematch: Error) -> Result<(), Error>
	where
		crate::Error: From<Q>,
	{
		if let Err(r) = r {
			let e: Error = r.into();

			// Some errors are slightly different on different platforms. So, we check
			// the first 10 characters which is specified in the ErrorKind generally.
			assert_eq!(
				e.to_string().substring(0, 10),
				ematch.to_string().substring(0, 10)
			);
			assert_eq!(
				e.kind().to_string().substring(0, 10),
				ematch.to_string().substring(0, 10)
			);
			assert!(e.cause().is_none());
			assert!(e.backtrace().is_some());
			assert_eq!(
				e.inner().substring(0, 10),
				ematch.to_string().substring(0, 10),
			);
			println!("e.backtrace()={:?}", e.backtrace());
			println!("e={}", e);
		}
		Ok(())
	}

	#[allow(invalid_from_utf8)]
	fn get_utf8() -> Result<String, Error> {
		Ok(std::str::from_utf8(&[0xC0])?.to_string())
	}

	#[test]
	fn test_errors() -> Result<(), Error> {
		check_error(
			std::fs::File::open("/no/path/here"),
			ErrorKind::IO("No such file or directory (os error 2)".to_string()).into(),
		)?;

		check_error(get_os_string(), ErrorKind::Misc("".to_string()).into())?;

		let x: Result<u32, _> = u64::MAX.try_into();
		check_error(x, ErrorKind::Misc(format!("TryFromIntError..")).into())?;

		let x: Result<u32, _> = "abc".parse();
		check_error(x, ErrorKind::Misc(format!("ParseIntError..")).into())?;
		check_error(get_utf8(), ErrorKind::Utf8(format!("Utf8 Error..")).into())?;

		Ok(())
	}

	#[test]
	fn test_other_errors() -> Result<(), Error> {
		let mutex = Arc::new(Mutex::new(0));
		let mutex_clone = mutex.clone();
		let lock = Arc::new(RwLock::new(0));
		let lock_clone = lock.clone();
		let _ = std::thread::spawn(move || -> Result<u32, Error> {
			let _mutex = mutex_clone.lock();
			let _x = lock.write();
			let y: Option<u32> = None;
			Ok(y.unwrap())
		})
		.join();

		check_error(
			lock_clone.write(),
			ErrorKind::Poison(format!("Poison..")).into(),
		)?;

		check_error(
			lock_clone.read(),
			ErrorKind::Poison(format!("Poison..")).into(),
		)?;

		check_error(mutex.lock(), ErrorKind::Poison(format!("Poison..")).into())?;

		let x = err!(ErrKind::Poison, "");
		let y = err!(ErrKind::IllegalArgument, "");
		let z = err!(ErrKind::Poison, "");

		assert_ne!(x, y);
		assert_eq!(x, z);

		let (tx, rx) = channel();

		std::thread::spawn(move || -> Result<(), Error> {
			tx.send(1)?;
			Ok(())
		});

		assert!(rx.recv().is_ok());
		let err = rx.recv();
		assert!(err.is_err());
		check_error(
			err,
			ErrorKind::IllegalState(format!("IllegalState..")).into(),
		)?;
		let tx = {
			let (tx, _rx) = channel();
			tx
		};

		let err = tx.send(1);
		check_error(
			err,
			ErrorKind::IllegalState(format!("IllegalState..")).into(),
		)?;

		let err = Layout::from_size_align(7, 7);
		check_error(err, ErrorKind::Alloc(format!("LayoutError..")).into())?;

		let now = SystemTime::now();
		let err: Result<Duration, SystemTimeError> = now
			.checked_add(Duration::from_millis(1_000_000))
			.unwrap()
			.duration_since(now.checked_add(Duration::from_millis(2_000_000)).unwrap());
		check_error(
			err,
			ErrorKind::SystemTime("System time error".into()).into(),
		)?;

		let err: Result<ServerName, InvalidDnsNameError> = "a*$&@@!aa".try_into();
		assert!(err.is_err());
		check_error(err, ErrorKind::Rustls("rustls error: ".to_string()).into())?;

		let err: Result<IpAddr, AddrParseError> = "127.0.0.1:8080".parse();
		assert!(err.is_err());
		check_error(
			err,
			ErrorKind::Misc("addr parse error: ".to_string()).into(),
		)?;

		let bytes: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 255];
		let err: Result<String, FromUtf8Error> = String::from_utf8(bytes.to_vec());
		assert!(err.is_err());
		check_error(err, ErrorKind::Misc("utf8 error: ".to_string()).into())?;

		let err: Result<String, WebpkiError> = Err(WebpkiError::BadDerTime);
		check_error(err, ErrorKind::Misc("webpkiError: ".to_string()).into())?;

		let err: Result<(), bmw_deps::rustls::Error> = Err(bmw_deps::rustls::Error::AlertReceived(
			bmw_deps::rustls::AlertDescription::CloseNotify,
		));
		check_error(err, ErrorKind::Rustls("rustls error: ".to_string()).into())?;

		let err: Result<(), bmw_deps::nix::errno::Errno> = Err(bmw_deps::nix::errno::Errno::EIO);
		check_error(err, ErrorKind::Errno("Errno error: ".to_string()).into())?;

		let err: Result<bmw_deps::url::Url, ParseError> = bmw_deps::url::Url::parse("http://*&^%$");
		check_error(err, ErrorKind::Misc("url::ParseError: ".to_string()).into())?;

		Ok(())
	}

	#[test]
	fn test_param() -> Result<(), Error> {
		let e = err!(ErrKind::Misc, "this is a test {} {}", 1, 2);
		let s = "Miscellaneous Error: this is a test 1 2".to_string();
		assert_eq!(&(e.to_string())[0..s.len()], &s.to_string()[0..s.len()]);
		Ok(())
	}
}
