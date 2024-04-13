// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(test)]
mod test {
	use bmw_conf2::*;
	use bmw_derive::*;
	use bmw_err::*;
	use bmw_log::*;

	debug!();

	#[derive(Configurable)]
	struct MyConfig2 {
		#[required]
		v1: u8,
		pub v2: u8,
		pub(crate) v3: u16,
	}

	impl Default for MyConfig2 {
		fn default() -> Self {
			Self {
				v1: 0,
				v2: 3,
				v3: 100,
			}
		}
	}

	#[test]
	fn test_derive_configurable1() -> Result<(), Error> {
		info!("testing derive configurable")?;

		let my = config!(MyConfig2, MyConfig2_Options, vec![v1(10)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 3);
		assert_eq!(my.v3, 100);

		let my = config!(MyConfig2, MyConfig2_Options, vec![v1(10), v2(20)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 20);
		assert_eq!(my.v3, 100);

		let my = config!(MyConfig2, MyConfig2_Options, vec![v1(10), v3(20)])?;
		assert_eq!(my.v1, 10);
		assert_eq!(my.v2, 3);
		assert_eq!(my.v3, 20);

		assert!(config!(MyConfig2, MyConfig2_Options, vec![v2(10), v3(0)]).is_err());
		Ok(())
	}

	#[derive(Configurable)]
	struct EvhConfig {
		threads: u8,
		#[required]
		port: u16,
		slab_count: u64,
		timeout: u128,
		blob: usize,
		x32: u32,
		server_name: String,
		debug: bool,
		test_vec: Vec<u32>,
		test_vec2: Vec<String>,
		other_ports: Vec<u16>,
		default_not_empty: Vec<u8>,
		header: (String, String),
		headers: Vec<(String, String)>,
	}

	impl Default for EvhConfig {
		fn default() -> Self {
			let threads = 1;
			let port = 8080;
			let slab_count = 100;
			let timeout = 100;
			let blob = 0;
			let x32 = 7;
			let server_name = "myname".to_string();
			let debug = false;
			let test_vec = vec![];
			let test_vec2 = vec![];
			let other_ports = vec![];
			let default_not_empty = vec![1, 2, 3];
			let header = ("".to_string(), "".to_string());
			let headers = vec![];
			Self {
				threads,
				port,
				slab_count,
				timeout,
				blob,
				x32,
				server_name,
				debug,
				test_vec,
				test_vec2,
				other_ports,
				default_not_empty,
				header,
				headers,
			}
		}
	}

	trait Evh {
		fn config(&mut self) -> &mut EvhConfig;
	}

	struct EvhImpl {
		config: EvhConfig,
	}

	impl EvhImpl {
		fn new(config: EvhConfig) -> Self {
			Self { config }
		}
	}

	impl Evh for EvhImpl {
		fn config(&mut self) -> &mut EvhConfig {
			&mut self.config
		}
	}

	macro_rules! evh {
		($($config:tt)*) => {{
			use EvhConfig_Options::*;
                        let options: Vec<EvhConfig_Options> = vec![$($config)*];

			match config!(EvhConfig, EvhConfig_Options, options) {
				Ok(config) => {
                                        let ret: Box<dyn Evh> = Box::new(EvhImpl::new(config));
                                        Ok(ret)
                                },
				Err(e) => Err(err!(ErrKind::Configuration, "config error: {}", e)),
			}
		}};
	}

	#[test]
	fn test_derive_configurable_evh() -> Result<(), Error> {
		let mut evh = evh!(threads(10), port(8081), server_name("abc"), debug(true))?;

		info!("evh.config.port={}", evh.config().port)?;

		assert_eq!(evh.config().port, 8081);

		evh.config().port = 8082;
		assert_eq!(evh.config().port, 8082);
		assert_eq!(evh.config().server_name, "abc".to_string());
		assert_eq!(evh.config().debug, true);

		let mut evh = evh!(
			port(1234),
			x32(u32::MAX),
			timeout(1_000_000_000_000),
			test_vec(7u32),
			test_vec(3u32)
		)?;

		assert_eq!(evh.config().port, 1234);
		assert_eq!(evh.config().x32, u32::MAX);
		assert_eq!(evh.config().timeout, 1_000_000_000_000);
		assert_eq!(evh.config().slab_count, 100);
		assert_eq!(evh.config().server_name, "myname".to_string());
		assert_eq!(evh.config().debug, false);
		assert_eq!(evh.config().test_vec, vec![7u32, 3u32]);

		let mut evh = evh!(
			port(1234),
			test_vec2("hi"),
			test_vec2("there"),
			test_vec2("next"),
			other_ports(90),
			other_ports(100),
			other_ports(110),
		)?;

		assert_eq!(
			evh.config().test_vec2,
			vec!["hi".to_string(), "there".to_string(), "next".to_string()]
		);

		assert_eq!(evh.config().other_ports, vec![90, 100, 110]);

		let mut evh = evh!(port(5555),)?;

		assert_eq!(evh.config().default_not_empty, vec![1, 2, 3]);

		let mut evh = evh!(port(5678), default_not_empty(8))?;

		// note that the 8 is appended to the default. That might not be what users expect,
		// but not a lot of use cases that I can think for having these kind of non-empty
		// default Vecs so we'll not implement a delete for now. Need to document this
		// though.
		assert_eq!(evh.config().default_not_empty, vec![1, 2, 3, 8]);

		let mut evh = evh!(port(4444), header(("abc", "def")),)?;

		assert_eq!(evh.config().header, ("abc".to_string(), "def".to_string()));

		let mut evh = evh!(
			port(6666),
			headers(("xyz", "ghi")),
			headers(("zzz", "aaab")),
			headers(("zzz2", "aaa2")),
		)?;

		assert_eq!(
			evh.config().headers,
			vec![
				("xyz".to_string(), "ghi".to_string()),
				("zzz".to_string(), "aaab".to_string()),
				("zzz2".to_string(), "aaa2".to_string())
			]
		);

		Ok(())
	}
}
