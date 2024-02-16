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
pub(crate) const SUFFIX_TREE_PUT_ID: usize = 5;
pub(crate) const SUFFIX_TREE_DELETE_ID: usize = 6;
pub(crate) const SUFFIX_TREE_OPTIONS_ID: usize = 7;
pub(crate) const SUFFIX_TREE_CONNECT_ID: usize = 8;
pub(crate) const SUFFIX_TREE_TRACE_ID: usize = 9;
pub(crate) const SUFFIX_TREE_PATCH_ID: usize = 10;

pub(crate) const CACHE_BUFFER_SIZE: usize = 412;
pub(crate) const CACHE_OVERHEAD_BYTES: usize = 100;
pub(crate) const CACHE_SLAB_SIZE: usize = 512;
pub(crate) const CACHE_BYTES_PER_SLAB: usize = 500;

pub(crate) const SEPARATOR_LINE: &str =
	"--------------------------------------------------------------------------------";

pub(crate) const ERROR_CONTENT: &str = "Error ERROR_CODE: \"ERROR_MESSAGE\" occurred.\n";
pub(crate) const TEXT_PLAIN: &str = "text/plain";
pub(crate) const HOST_BYTES: &[u8] = "Host".as_bytes();
pub(crate) const IF_NONE_MATCH_BYTES: &[u8] = "If-None-Match".as_bytes();
pub(crate) const IF_MODIFIED_SINCE_BYTES: &[u8] = "If-Modified-Since".as_bytes();
pub(crate) const CONNECTION_BYTES: &[u8] = "Connection".as_bytes();
pub(crate) const KEEP_ALIVE_BYTES: &[u8] = "keep-alive".as_bytes();
pub(crate) const RANGE_BYTES: &[u8] = "Range".as_bytes();
pub(crate) const UPGRADE_BYTES: &[u8] = "Upgrade".as_bytes();
pub(crate) const WEBSOCKET_BYTES: &[u8] = "websocket".as_bytes();
pub(crate) const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
pub(crate) const SEC_WEBSOCKET_KEY_BYTES: &[u8] = "Sec-WebSocket-Key".as_bytes();
pub(crate) const SEC_WEBSOCKET_PROTOCOL_BYTES: &[u8] = "Sec-WebSocket-Protocol".as_bytes();
pub(crate) const ACCEPT_ENCODING_BYTES: &[u8] = "Accept-Encoding".as_bytes();
pub(crate) const CONTENT_LENGTH_BYTES: &[u8] = "Content-Length".as_bytes();

pub(crate) const FIN_BIT: u8 = 0x1 << 7;
pub(crate) const MASK_BIT: u8 = 0x1 << 7;
pub(crate) const OP_CODE_MASK1: u8 = 0x1 << 3;
pub(crate) const OP_CODE_MASK2: u8 = 0x1 << 2;
pub(crate) const OP_CODE_MASK3: u8 = 0x1 << 1;
pub(crate) const OP_CODE_MASK4: u8 = 0x1 << 0;

pub(crate) const CONTENT_SLAB_DATA_SIZE: usize = 514;
pub(crate) const CONTENT_SLAB_SIZE: usize = 518;
pub(crate) const CONTENT_SLAB_NEXT_OFFSET: usize = 514;
