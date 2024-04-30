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

pub use crate::types::LogLevel;
use bmw_core::*;

#[class{
    /// Log trait
    public logger;
    clone logger;
    /// The maximum age, in milliseconds, before a log should be rotated.
    const max_age_millis: u64 = u64::MAX;
    /// The maximum size, in bytes, before a log file should be rotated.
    const max_size_bytes: u64 = u64::MAX;
    /// The maximum length, in bytes, of the line number data which, if
    /// enabled, is included in the log line.
    const LineNumDataMaxLen: u16 = 30;
    /// The path of log file. If this value is set to "", file logging is
    /// disabled.
    const LogFilePath: String = "".to_string();
    /// If [`true`], print logged data to standard output.
    const stdout: bool = true;
    /// If set to true, colors are displayed to make the log lines easier to
    /// read. Note that colors are only enabled on stdout.
    const colors: bool = true;
    /// If set to true, timestamps are displayed on each log line.
    const timestamp: bool = true;
    ///  If set to true, log level is displayed on each log line.
    const log_level: bool = true;
    /// If set to true, line number data are displayed on each log line.
    const line_num: bool = true;
    ///  If set to true, a backtrace will be logged when data are logged at
    ///  the [`crate::LogLevel::Error`] or [`crate::LogLevel::Fatal`] level.
    const backtrace: bool = false;
    /// If set to true, log files are automatically rotated. Rotation can
    /// only occur at the time data is logged.
    const auto_rotate: bool = false;
    /// If set to true, log files are immidately deleted upon log rotation.
    /// This option is useful for long running tests where logging to disk
    /// may result in running out of disk space. This value _MUST_ be set
    /// to false in a production environment.
    const delete_rotation: bool = false;
    /// The header to include at the top of all log files.
    const file_header: String = "".to_string();


    fn builder(&const_values) -> Result<Self, Error> {
        Ok(Self {})
    }

    [logger]
    fn log(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn log_all(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn log_plain(&mut self, level: LogLevel, line: &str) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn need_rotate(&self) -> Result<bool, Error> {
        todo!()
    }

    [logger]
    fn rotate(&mut self) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn set_log_level(&mut self, level: LogLevel) {
        todo!()
    }

    [logger]
    fn init(&mut self) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn close(&mut self) -> Result<(), Error> {
        todo!()
    }

    [logger]
    fn set_config_option(&mut self, value: LogConstOptions) -> Result<(), Error> {
        todo!()
    }
}]
impl Log {}
