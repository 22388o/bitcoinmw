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
//

// These are local constants used in the logging crate

// newline as byte array
pub(crate) const NEWLINE: &[u8] = &['\n' as u8];
// the default max length for the file location of a logged line
pub(crate) const DEFAULT_LINE_NUM_DATA_MAX_LEN: u64 = 30;
// the minimum value for MaxAgeMillis
pub(crate) const MINIMUM_MAX_AGE_MILLIS: u64 = 1_000;
// the minimum value for MaxSizeBytes
pub(crate) const MINIMUM_MAX_SIZE_BYTES: u64 = 50;
// the minimum value for LineNumDataMaxLen
pub(crate) const MINIMUM_LNDML: u64 = 10;
