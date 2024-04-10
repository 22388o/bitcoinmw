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

// Http config option defaults
pub(crate) const DEFAULT_HTTP_ACCEPT: &str = "*/*";
pub(crate) const DEFAULT_HTTP_VERSION: &str = "HTTP/1.1";
pub(crate) const DEFAULT_HTTP_METHOD: &str = "GET";
pub(crate) const DEFAULT_HTTP_CONNECTION_TYPE: &str = "close";
pub(crate) const DEFAULT_HTTP_TIMEOUT_MILLIS: u64 = 0;
pub(crate) const DEFAULT_HTTP_REQUEST_URI: &str = "";
pub(crate) const DEFAULT_HTTP_REQUEST_URL: &str = "";

// Http version strings
pub(crate) const HTTP_VERSION_11: &str = "HTTP/1.1";
pub(crate) const HTTP_VERSION_10: &str = "HTTP/1.0";
pub(crate) const HTTP_VERSION_20: &str = "HTTP/2.0";

// Http method strings
pub(crate) const HTTP_METHOD_GET: &str = "GET";
pub(crate) const HTTP_METHOD_POST: &str = "POST";
pub(crate) const HTTP_METHOD_HEAD: &str = "HEAD";
pub(crate) const HTTP_METHOD_PUT: &str = "PUT";
pub(crate) const HTTP_METHOD_DELETE: &str = "DELETE";
pub(crate) const HTTP_METHOD_OPTIONS: &str = "OPTIONS";
pub(crate) const HTTP_METHOD_CONNECT: &str = "CONNECT";
pub(crate) const HTTP_METHOD_TRACE: &str = "TRACE";
pub(crate) const HTTP_METHOD_PATCH: &str = "PATCH";

// Http connection types
pub(crate) const HTTP_CONNECTION_TYPE_CLOSE: &str = "close";
pub(crate) const HTTP_CONNECTION_TYPE_KEEP_ALIVE: &str = "keep-alive";

// Http client search trie pattern ids
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_TERMINATION: usize = 0;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_HEADER: usize = 1;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_CONTENT_LENGTH: usize = 2;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_SERVER: usize = 3;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_TRANSFER_ENCODING: usize = 4;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_GET: usize = 5;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_POST: usize = 6;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_HEAD: usize = 7;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_PUT: usize = 8;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_DELETE: usize = 9;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_OPTIONS: usize = 10;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_CONNECT: usize = 11;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_TRACE: usize = 12;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_PATCH: usize = 13;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_HTTP11: usize = 14;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_HTTP10: usize = 15;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_CONNECTION_KEEP_ALIVE: usize = 16;
pub(crate) const HTTP_SEARCH_TRIE_PATTERN_CONNECTION_CLOSE: usize = 17;

// Http Server defaults
pub(crate) const HTTP_SERVER_DEFAULT_PORT: u16 = 8080;
pub(crate) const HTTP_SERVER_DEFAULT_ADDR: &str = "127.0.0.1";
pub(crate) const HTTP_SERVER_DEFAULT_BASE_DIR: &str = "~/.bmw/www";
pub(crate) const HTTP_SERVER_DEFAULT_LISTEN_QUEUE_SIZE: usize = 1_000;

pub(crate) const HTTP_SERVER_FILE_BUFFER_SIZE: usize = 1_000;
pub(crate) const HTTP_SERVER_DEFAULT_EVH_SLAB_COUNT: usize = 10_000;
pub(crate) const HTTP_SERVER_DEFAULT_EVH_SLAB_SIZE: usize = 512;

pub(crate) const HTTP_CLIENT_DEFAULT_BASE_DIR: &str = "~/.bmw";
pub(crate) const HTTP_CLIENT_DEFAULT_EVH_SLAB_SIZE: usize = 512;
pub(crate) const HTTP_CLIENT_DEFAULT_EVH_SLAB_COUNT: usize = 10_000;
pub(crate) const HTTP_CLIENT_MAX_MATCHES: usize = 1_000;

pub(crate) const HTTP_SERVER_404_CONTENT: &str =
	"A 404 (not found) error occurred. See server logs for further details.\n";
pub(crate) const HTTP_SERVER_403_CONTENT: &str =
	"A 403 (forbidden) error occurred. See server logs for further details.\n";
pub(crate) const HTTP_SERVER_400_CONTENT: &str =
	"A 400 (bad request) error occurred. See server logs for further details.\n";
