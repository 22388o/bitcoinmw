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

pub(crate) const BUFFER_SIZE: usize = 412;

pub(crate) const THREAD_POOL_DEFAULT_MIN_SIZE: usize = 1;
pub(crate) const THREAD_POOL_DEFAULT_MAX_SIZE: usize = 1;
pub(crate) const THREAD_POOL_DEFAULT_SYNC_CHANNEL_SIZE: usize = 10;

pub(crate) const HASH_DEFAULT_MAX_ENTRIES: usize = 1_000;
pub(crate) const HASH_DEFAULT_MAX_LOAD_FACTOR: f64 = 0.7;
pub(crate) const HASH_DEFAULT_SLAB_SIZE: usize = 514;
pub(crate) const HASH_DEFAULT_SLAB_COUNT: usize = 1_000;
