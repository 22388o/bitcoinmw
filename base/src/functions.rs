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

use crate::{BinReader, BinWriter, Error, Serializable};
use std::io::{Read, Write};

/// Serializes a [`crate::Serializable`] into any [`std::io::Write`] implementation.
/// # Input Parameters
/// * `sink` - &mut dyn [`Write`] - any implementation of [`Write`].
/// * `thing` - [`crate::Serializable`] - anything that implements the [`crate::Serializable`]
/// trait.
/// # Errors
/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
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
/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
/// * [`crate::BaseErrorKind::OperationNotSupported`] - if the serialized data was from a data
/// type that did not allow for it to be deserialized.
/// * [`crate::BaseErrorKind::CorruptedData`] - if the data that was serialized was corrupted.
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
