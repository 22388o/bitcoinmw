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

use crate::TraitType;
use bmw_err::*;

impl TryFrom<String> for TraitType {
	type Error = Error;
	fn try_from(v: String) -> Result<Self, Error> {
		Ok(if v == "IMPL" {
			TraitType::Impl
		} else if v == "DYN" {
			TraitType::Dyn
		} else if v == "IMPL_SEND" {
			TraitType::ImplSend
		} else if v == "IMPL_SYNC" {
			TraitType::ImplSync
		} else if v == "DYN_SEND" {
			TraitType::DynSend
		} else if v == "DYN_SYNC" {
			TraitType::DynSync
		} else {
			return Err(err!(ErrKind::Parse, "'{}' is not a valid TraitType", v));
		})
	}
}
