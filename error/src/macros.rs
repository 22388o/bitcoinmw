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

/// Build the specified [`crate::ErrorKind`] and convert it into an [`crate::Error`]. The desired
/// [`crate::ErrorKind`] is specified using the [`crate::ErrKind`] name enum.
///
/// Example:
///
///```
/// use bmw_err::{Error, ErrorKind, ErrKind, err};
///
/// fn main() -> Result<(), Error> {
///     show_err_kind(false)?;
///     Ok(())
/// }
///
/// fn show_err_kind(do_error: bool) -> Result<(), Error> {
///     let e = err!(ErrKind::Configuration, "invalid parameter name");
///
///     if do_error {
///         return Err(e);
///     }
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! err {
	($kind:expr, $msg:expr, $($param:tt)*) => {{
                use bmw_err::err;
                let msg = &format!($msg, $($param)*)[..];
                err!($kind, msg)
        }};
	($kind:expr, $m:expr) => {{
                #[allow(unused_imports)]
                use bmw_err::{ErrKind, ErrorKind, Error, impl_err};
                use bmw_err::ErrKind::*;
		match $kind {
                        IO => impl_err!(IO, $m),
                        Log=> impl_err!(Log, $m),
                        Utf8 => impl_err!(Utf8, $m),
                        ArrayIndexOutOfBounds => impl_err!(ArrayIndexOutOfBounds, $m),
                        Configuration => impl_err!(Configuration, $m),
                        Poison => impl_err!(Poison, $m),
                        CorruptedData => impl_err!(CorruptedData, $m),
                        Timeout => impl_err!(Timeout, $m),
                        CapacityExceeded => impl_err!(CapacityExceeded, $m),
                        UnexpectedEof=> impl_err!(UnexpectedEof, $m),
                        IllegalArgument => impl_err!(IllegalArgument, $m),
                        Misc => impl_err!(Misc, $m),
                        IllegalState => impl_err!(IllegalState, $m),
                        Overflow => impl_err!(Overflow, $m),
                        Test => impl_err!(Test, $m),
                        ThreadPanic => impl_err!(ThreadPanic, $m),
                        Alloc => impl_err!(Alloc, $m),
                        OperationNotSupported => impl_err!(OperationNotSupported, $m),
                        SystemTime => impl_err!(SystemTime, $m),
                        Errno => impl_err!(Errno, $m),
                        Rustls => impl_err!(Rustls, $m),
                        Crypt => impl_err!(Crypt, $m),
                        Http => impl_err!(Http, $m),
                        Http404 => impl_err!(Http404, $m),
                        Http403 => impl_err!(Http403, $m),
                        Http400 => impl_err!(Http400, $m),
                        Rustlet => impl_err!(Rustlet, $m),
		}
	}};
}

/// Map the specified error into the [`crate::ErrKind`] enum name from this crate.
/// Optionally specify an additional message to be included in the error.
///
/// Example:
///
///```
/// use bmw_err::{Error, ErrorKind, ErrKind, map_err};
/// use std::fs::File;
/// use std::io::Write;
///
/// fn main() -> Result<(), Error> {
///     assert!(show_map_err().is_err());
///     Ok(())
/// }
///
/// fn show_map_err() -> Result<(), Error> {
///     let file = map_err!(File::open("/path/to/something"), ErrKind::IO, "file open failed")?;
///     println!("file_type={:?}", file.metadata()?.file_type());
///
///     let mut x = map_err!(File::open("/invalid/log/path.log"), ErrKind::Log)?;
///     x.write(b"test")?;
///
///     Ok(())
/// }
///```
#[macro_export]
macro_rules! map_err {
	($in_err:expr, $kind:expr) => {{
		use bmw_err::map_err;
		map_err!($in_err, $kind, "")
	}};
	($in_err:expr, $kind:expr, $m:expr) => {{
		use bmw_err::ErrKind::*;
		#[allow(unused_imports)]
		use bmw_err::{impl_map_err, ErrKind, Error, ErrorKind};
		$in_err.map_err(|e| -> Error {
			let k = $kind;
			match k {
				Configuration => impl_map_err!(Configuration, $m, e),
				IO => impl_map_err!(IO, $m, e),
				Log => impl_map_err!(Log, $m, e),
				UnexpectedEof => impl_map_err!(UnexpectedEof, $m, e),
				Utf8 => impl_map_err!(Utf8, $m, e),
				ArrayIndexOutOfBounds => impl_map_err!(ArrayIndexOutOfBounds, $m, e),
				Timeout => impl_map_err!(Timeout, $m, e),
				CapacityExceeded => impl_map_err!(CapacityExceeded, $m, e),
				IllegalArgument => impl_map_err!(IllegalArgument, $m, e),
				Poison => impl_map_err!(Poison, $m, e),
				Misc => impl_map_err!(Misc, $m, e),
				CorruptedData => impl_map_err!(CorruptedData, $m, e),
				IllegalState => impl_map_err!(IllegalState, $m, e),
				Test => impl_map_err!(Test, $m, e),
				Overflow => impl_map_err!(Overflow, $m, e),
				ThreadPanic => impl_map_err!(ThreadPanic, $m, e),
				Alloc => impl_map_err!(Alloc, $m, e),
				OperationNotSupported => impl_map_err!(OperationNotSupported, $m, e),
				SystemTime => impl_map_err!(SystemTime, $m, e),
				Errno => impl_map_err!(Errno, $m, e),
				Rustls => impl_map_err!(Rustls, $m, e),
				Crypt => impl_map_err!(Crypt, $m, e),
				Http => impl_map_err!(Http, $m, e),
				Http404 => impl_map_err!(Http404, $m, e),
				Http403 => impl_map_err!(Http403, $m, e),
				Http400 => impl_map_err!(Http400, $m, e),
				Rustlet => impl_map_err!(Rustlet, $m, e),
			}
		})
	}};
}

/// Macro to do a conditional break
#[macro_export]
macro_rules! cbreak {
	($cond:expr) => {{
		if $cond {
			break;
		}
	}};
}

/// Macro to map the try_from error into an appropriate error.
#[macro_export]
macro_rules! try_into {
	($v:expr) => {{
		use bmw_err::{map_err, ErrKind};
		use std::convert::TryInto;
		map_err!($v.try_into(), ErrKind::Misc, "TryInto Error")
	}};
}

// helper to do err
#[doc(hidden)]
#[macro_export]
macro_rules! impl_err {
	($error_kind:ident, $msg:expr) => {{
		let error: Error = ErrorKind::$error_kind($msg.to_string()).into();
		error
	}};
}

// helper to do mapping
#[doc(hidden)]
#[macro_export]
macro_rules! impl_map_err {
	($error_kind:ident, $msg:expr, $e:expr) => {
		ErrorKind::$error_kind(format!("{}: {}", $msg, $e)).into()
	};
}
