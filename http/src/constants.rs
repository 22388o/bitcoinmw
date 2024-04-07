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

pub(crate) const HTTP_CONNECTION_TYPE_CLOSE: &str = "close";
pub(crate) const HTTP_CONNECTION_TYPE_KEEP_ALIVE: &str = "keep-alive";
