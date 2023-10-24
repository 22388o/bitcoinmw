// Copyright (c) 2023, The BitcoinMW Developers
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

pub(crate) const SUFFIX_TREE_TERMINATE_HEADERS_ID: usize = 0;
pub(crate) const SUFFIX_TREE_GET_ID: usize = 1;
pub(crate) const SUFFIX_TREE_POST_ID: usize = 2;
pub(crate) const SUFFIX_TREE_HEAD_ID: usize = 3;
pub(crate) const SUFFIX_TREE_HEADER_ID: usize = 4;

pub(crate) const CACHE_BUFFER_SIZE: usize = 412;
pub(crate) const CACHE_OVERHEAD_BYTES: usize = 100;
pub(crate) const CACHE_SLAB_SIZE: usize = 512;
pub(crate) const CACHE_BYTES_PER_SLAB: usize = 500;

pub(crate) const SEPARATOR_LINE: &str =
	"--------------------------------------------------------------------------------";

pub(crate) const ERROR_CONTENT: &str = "Error ERROR_CODE: \"ERROR_MESSAGE\" occurred.";
pub(crate) const HOST_BYTES: &[u8] = "Host".as_bytes();
pub(crate) const CONNECTION_BYTES: &[u8] = "Connection".as_bytes();
pub(crate) const KEEP_ALIVE_BYTES: &[u8] = "keep-alive".as_bytes();
pub(crate) const RANGE_BYTES: &[u8] = "Range".as_bytes();
