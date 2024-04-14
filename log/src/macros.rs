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

#[macro_export]
macro_rules! trace {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                trace!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! trace_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! trace_all {
        () => {
                 #[doc(hidden)]
                 const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Trace;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Trace, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                trace_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                debug!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! debug_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Debug;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Debug, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                debug_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                info!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                info_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! info_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Info;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Info, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                info_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                warn!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! warn_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Warn;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Warn, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                warn_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                error!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                error_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! error_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Error;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Error, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                error_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Standard)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_plain {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::Plain)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_plain!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! fatal_all {
        () => {
                #[doc(hidden)]
                const BMW_GLOBAL_LOG_LEVEL: bmw_log::LogLevel = bmw_log::LogLevel::Fatal;
        };
        ($line:expr) => {{
                use bmw_log::*;
                GlobalLogFunctions::log(LogLevel::Fatal, $line, BMW_GLOBAL_LOG_LEVEL, LoggingType::All)
        }};
        ($line:expr,$($values:tt)*) => {
                fatal_all!(&format!($line, $($values)*)[..])
        };
}

#[macro_export]
macro_rules! log_init {
        ($($config:tt)*) => {{
                use bmw_log::GlobalLogFunctions;
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                GlobalLogFunctions::init(v)
        }};
}

#[macro_export]
macro_rules! set_log_option {
	($option:expr) => {{
		use bmw_log::GlobalLogFunctions;
		use bmw_log::LogConfigOptions::*;
		GlobalLogFunctions::set_log_option($option)
	}};
}

#[macro_export]
macro_rules! log_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::rotate()
	}};
}

#[macro_export]
macro_rules! need_rotate {
	() => {{
		use bmw_log::GlobalLogFunctions;
		GlobalLogFunctions::need_rotate()
	}};
}

#[macro_export]
macro_rules! logger {
        ($($config:tt)*) => {{
                use bmw_log::LogConfigOptions::*;
                let v: Vec<LogConfigOptions> = vec![$($config)*];
                LogBuilder::build_log(v)
        }};
}
