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

use crate::public::*;
use bmw_conf2::Configurable;
use bmw_derive::Configurable;
use std::fs::File;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct LogImpl {
	pub(crate) config: LogConfig2,
	pub(crate) log_level: LogLevel,
	pub(crate) cur_size: u64,
	pub(crate) file: Arc<RwLock<Option<File>>>,
	pub(crate) is_init: bool,
	pub(crate) last_rotation: Instant,
}

#[derive(Configurable, Clone)]
pub(crate) struct LogConfig2 {
	pub(crate) max_size_bytes: u64,
	pub(crate) max_age_millis: u64,
	pub(crate) display_colors: bool,
	pub(crate) display_stdout: bool,
	pub(crate) display_timestamp: bool,
	pub(crate) display_log_level: bool,
	pub(crate) display_line_num: bool,
	pub(crate) display_millis: bool,
	pub(crate) display_backtrace: bool,
	pub(crate) log_file_path: String,
	pub(crate) line_num_data_max_len: u64,
	pub(crate) delete_rotation: bool,
	pub(crate) file_header: String,
	pub(crate) auto_rotate: bool,
	pub(crate) debug_process_resolve_frame_error: bool,
	pub(crate) debug_invalid_metadata: bool,
	pub(crate) debug_lineno_is_none: bool,
}
