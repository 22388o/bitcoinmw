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

// configuration defaults
pub(crate) const EVH_DEFAULT_THREADS: usize = 4;
pub(crate) const EVH_DEFAULT_TIMEOUT: u16 = 1_000;
pub(crate) const EVH_DEFAULT_IN_EVENTS_SIZE: usize = 1_000;
pub(crate) const EVH_DEFAULT_READ_SLAB_COUNT: usize = 1_000;
pub(crate) const EVH_DEFAULT_READ_SLAB_SIZE: usize = 512;
pub(crate) const EVH_DEFAULT_HOUSEKEEPING_FREQUENCY_MILLIS: usize = 10_000; // 10 seconds
pub(crate) const EVH_DEFAULT_STATS_UPDATE_MILLIS: usize = 5_000; // 5 seconds
pub(crate) const EVH_DEFAULT_OUT_OF_SLABS_MESSAGE: &str = "";

// slice max size for ret handles
pub(crate) const MAX_RET_HANDLES: usize = 100;

// write state flags
pub(crate) const WRITE_STATE_FLAG_PENDING: u8 = 0x1 << 0;
pub(crate) const WRITE_STATE_FLAG_CLOSE: u8 = 0x1 << 1;
pub(crate) const WRITE_STATE_FLAG_TRIGGER_ON_READ: u8 = 0x1 << 2;

// errno().0 values
pub(crate) const EAGAIN: i32 = 11;
pub(crate) const ETEMPUNAVAILABLE: i32 = 35;
pub(crate) const WINNONBLOCKING: i32 = 10035;

// true to avoid the warning on while true loops
pub(crate) const TRUE: bool = true;
