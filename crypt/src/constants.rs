// Copyright (c) 2022, 37 Miners, LLC
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

/// Ed25519 certificate type
pub const CERT_TYPE_ED25519: u8 = 0x0;

/// Tls certificate type
pub const CERT_TYPE_TLS: u8 = 0x1;

/// Number of bytes in an RSA id.
pub const RSA_ID_LEN: usize = 20;

///  Number of bytes in an ed25519 id.
pub const ED25519_ID_LEN: usize = 32;

/// Length of a bmw_crypt cell
pub const CELL_LEN: usize = 514;

/// Maximum length of a nickname
pub const MAX_NICKNAME_LEN: usize = 100;

pub const CELL_TYPE_INFO: u8 = 0;
