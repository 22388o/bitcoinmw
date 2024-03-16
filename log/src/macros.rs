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

/// a usize conversion macro
#[macro_export]
macro_rules! usize {
	($v:expr) => {{
		use std::convert::TryInto;
		let v: usize = $v.try_into()?;
		v
	}};
}

/// a u64 conversion macro
#[macro_export]
macro_rules! u64 {
	($v:expr) => {{
		use std::convert::TryInto;
		let v: u64 = $v.try_into()?;
		v
	}};
}

#[macro_export]
macro_rules! trace {
	() => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
	};
	($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
	}};
	($line:expr,$($values:tt)*) => {
                trace!(&format!($line, $($values)*)[..])
	};
}

#[macro_export]
macro_rules! trace_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! trace_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                debug!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                info!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                info_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                info_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                warn!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                error!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                error_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                error_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_plain {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_all {
        () => {
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogContainer::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! log_init {
	($($config:tt)*) => {{
		use bmw_log::LogBuilder;
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                let v: Vec<ConfigOption> = vec![$($config)*];
                GlobalLogContainer::init(v)?;
	}};
}
