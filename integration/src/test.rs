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
	use bmw_core::*;
	use TestErrorKind::*;

	/// Kinds of errors used in testing
	#[ErrorKind]
	enum TestErrorKind {
		TestAbc,
		TestDef,
		/// ghi error kind
		TestGhi,
		IllegalState,
	}

	fn ret_err() -> Result<(), Error> {
		ret_err!(TestAbc, "11234test abc");
	}

	fn ret_err2() -> Result<(), Error> {
		err!(TestGhi, "12345")
	}

	fn ret_err3() -> Result<(), Error> {
		match ret_err2() {
			Ok(_) => Ok(()),
			Err(e) => err!(IllegalState, "ret_err2 generated error: {}", e),
		}
	}

	#[test]
	fn test_errorkind() -> Result<(), Error> {
		let err1 = ret_err().unwrap_err();
		assert_eq!(
			err1.kind().to_string(),
			"test abc: 11234test abc".to_string()
		);
		let err2 = ret_err2().unwrap_err();
		assert_eq!(err2.kind().to_string(), "ghi error kind: 12345".to_string());

		let err3 = ret_err3().unwrap_err();
		println!("err3='{}'", err3);
		assert_eq!(err3.kind(), kind!(IllegalState));

		Ok(())
	}

	/*
	#[derive(Clone)]
	struct TestGlobalConfig {
		config: TestConfig,
		v3: u64,
	}

	impl Default for TestGlobalConfig {
		fn default() -> Self {
			Self {
				config: TestConfig::default(),
				v3: 101,
			}
		}
	}

	enum TestGlobalConfigOptions {
		Config(Box<dyn Configurable>),
		V3(u64),
	}

	impl Configurable for TestGlobalConfig {
		fn set_usize(&mut self, name: &str, value: usize) {}

		fn set_bool(&mut self, name: &str, value: bool) {}
		fn set_u8(&mut self, name: &str, value: u8) {}
		fn set_u16(&mut self, name: &str, value: u16) {}
		fn set_u32(&mut self, name: &str, value: u32) {}
		fn set_u64(&mut self, name: &str, value: u64) {
			if name == "V3" {
				self.v3 = value;
			}
		}
		fn set_u128(&mut self, name: &str, value: u128) {}
		fn set_string(&mut self, name: &str, value: String) {}
		fn set_configurable(&mut self, name: &str, value: &dyn Configurable) {
			if name == "Config" {
				for param in &value.get_usize_params() {
					self.config.set_usize(&param.0, param.1);
				}
				for param in &value.get_u64_params() {
					self.config.set_u64(&param.0, param.1);
				}
				for param in &value.get_configurable_params() {
					self.config.set_configurable(&param.0, &*param.1);
				}
				for param in &value.get_string_params() {
					self.config.set_string(&param.0, param.1.clone());
				}
			}
		}
		fn allow_dupes(&self) -> HashSet<String> {
			HashSet::new()
		}

		fn required(&self) -> Vec<String> {
			vec![]
		}

		fn get_usize_params(&self) -> Vec<(String, usize)> {
			vec![]
		}

		fn get_vec_usize_params(&self) -> Vec<(String, Vec<usize>)> {
			vec![]
		}

		fn get_u8_params(&self) -> Vec<(String, u8)> {
			vec![]
		}
		fn get_u16_params(&self) -> Vec<(String, u16)> {
			vec![]
		}
		fn get_u32_params(&self) -> Vec<(String, u32)> {
			vec![]
		}
		fn get_u64_params(&self) -> Vec<(String, u64)> {
			vec![("V3".to_string(), self.v3)]
		}
		fn get_u128_params(&self) -> Vec<(String, u128)> {
			vec![]
		}
		fn get_bool_params(&self) -> Vec<(String, bool)> {
			vec![]
		}
		fn get_string_params(&self) -> Vec<(String, String)> {
			vec![]
		}
		fn get_configurable_params(&self) -> Vec<(String, Box<dyn Configurable>)> {
			vec![("Config".to_string(), Box::new(self.config.clone()))]
		}
	}

	impl ConfigurableOptions for TestGlobalConfigOptions {
		fn name(&self) -> &str {
			match self {
				TestGlobalConfigOptions::Config(_) => "Config",
				TestGlobalConfigOptions::V3(_) => "V3",
			}
		}
		fn value_usize(&self) -> Option<usize> {
			None
		}
		fn value_bool(&self) -> Option<bool> {
			None
		}
		fn value_u8(&self) -> Option<u8> {
			None
		}
		fn value_u16(&self) -> Option<u16> {
			None
		}
		fn value_u32(&self) -> Option<u32> {
			None
		}
		fn value_u64(&self) -> Option<u64> {
			match self {
				TestGlobalConfigOptions::V3(v) => Some(*v),
				_ => None,
			}
		}
		fn value_u128(&self) -> Option<u128> {
			None
		}
		fn value_string(&self) -> Option<String> {
			None
		}
		fn value_configurable(&self) -> Option<Box<dyn Configurable>> {
			match self {
				TestGlobalConfigOptions::Config(v) => Some(v.clone()),
				_ => None,
			}
		}
	}

	#[derive(Clone)]
	struct TestConfig {
		v1: usize,
		v2: usize,
	}

	impl Default for TestConfig {
		fn default() -> Self {
			Self { v1: 1, v2: 0 }
		}
	}

	enum TestConfigOptions {
		V1(usize),
		V2(usize),
	}

	impl Configurable for TestConfig {
		fn set_usize(&mut self, name: &str, value: usize) {
			if name == "V2" {
				self.v2 = value;
			} else if name == "V1" {
				self.v1 = value;
			}
		}

		fn set_bool(&mut self, name: &str, value: bool) {}
		fn set_u8(&mut self, name: &str, value: u8) {}
		fn set_u16(&mut self, name: &str, value: u16) {}
		fn set_u32(&mut self, name: &str, value: u32) {}
		fn set_u64(&mut self, name: &str, value: u64) {}
		fn set_u128(&mut self, name: &str, value: u128) {}
		fn set_string(&mut self, name: &str, value: String) {}
		fn set_configurable(&mut self, name: &str, value: &dyn Configurable) {}
		fn allow_dupes(&self) -> HashSet<String> {
			HashSet::new()
		}

		fn required(&self) -> Vec<String> {
			vec![]
		}

		fn get_usize_params(&self) -> Vec<(String, usize)> {
			let mut ret = vec![];
			ret.push(("V1".to_string(), self.v1));
			ret.push(("V2".to_string(), self.v2));

			ret
		}

		fn get_vec_usize_params(&self) -> Vec<(String, Vec<usize>)> {
			vec![]
		}

		fn get_u8_params(&self) -> Vec<(String, u8)> {
			vec![]
		}
		fn get_u16_params(&self) -> Vec<(String, u16)> {
			vec![]
		}
		fn get_u32_params(&self) -> Vec<(String, u32)> {
			vec![]
		}
		fn get_u64_params(&self) -> Vec<(String, u64)> {
			vec![]
		}
		fn get_u128_params(&self) -> Vec<(String, u128)> {
			vec![]
		}
		fn get_bool_params(&self) -> Vec<(String, bool)> {
			vec![]
		}
		fn get_string_params(&self) -> Vec<(String, String)> {
			vec![]
		}
		fn get_configurable_params(&self) -> Vec<(String, Box<dyn Configurable>)> {
			vec![]
		}
	}

	impl ConfigurableOptions for TestConfigOptions {
		fn name(&self) -> &str {
			match self {
				TestConfigOptions::V1(_) => "V1",
				TestConfigOptions::V2(_) => "V2",
			}
		}
		fn value_usize(&self) -> Option<usize> {
			match self {
				TestConfigOptions::V2(v) => Some(*v),
				TestConfigOptions::V1(v) => Some(*v),
				_ => None,
			}
		}
		fn value_bool(&self) -> Option<bool> {
			None
		}
		fn value_u8(&self) -> Option<u8> {
			None
		}
		fn value_u16(&self) -> Option<u16> {
			None
		}
		fn value_u32(&self) -> Option<u32> {
			None
		}
		fn value_u64(&self) -> Option<u64> {
			None
		}
		fn value_u128(&self) -> Option<u128> {
			None
		}
		fn value_string(&self) -> Option<String> {
			None
		}
		fn value_configurable(&self) -> Option<Box<dyn Configurable>> {
			None
		}
	}
		*/

	/*
	/// some configurable stuff
	#[derive(Configurable, Clone)]
	struct MyConfigurable {
		/// threads here
		threads: usize,
		#[required]
		timeout: u64,
		param_names: Vec<String>,
		slab_size: u32,
	}

	impl Default for MyConfigurable {
		fn default() -> Self {
			Self {
				threads: 1,
				timeout: 1_000,
				slab_size: 256,
				param_names: vec![],
			}
		}
	}
		*/

	/*
	#[derive(Configurable, Clone)]
	struct MyConfigurableAdvanced {
		first_value: usize,
		config: MyConfigurable,
	}

	impl Default for MyConfigurableAdvanced {
		fn default() -> Self {
			Self {
				config: MyConfigurable::default(),
				first_value: 99,
			}
		}
	}

	#[derive(Configurable, Clone)]
	struct MyConfigurable {
		threads: usize,
		timeout: usize,
		slab_size: usize,
	}

	impl Default for MyConfigurable {
		fn default() -> Self {
			Self {
				threads: 1,
				timeout: 2,
				slab_size: 3,
			}
		}
	}
		*/

	/*
	#[derive(Configurable, Clone)]
	struct TestVecVec {
		def: usize,
		config: TestVec,
	}

	impl Default for TestVecVec {
		fn default() -> Self {
			Self {
				def: 100,
				config: TestVec::default(),
			}
		}
	}

	#[derive(Configurable, Clone)]
	struct TestVec {
		abc: Vec<usize>,
	}

	impl Default for TestVec {
		fn default() -> Self {
			Self { abc: vec![] }
		}
	}

	#[derive(Configurable, Clone)]
	struct RequiredTest {
		abc: usize,
		def: usize,
		#[required]
		ghi: usize,
	}

	impl Default for RequiredTest {
		fn default() -> Self {
			Self {
				abc: 1,
				def: 2,
				ghi: 3,
			}
		}
	}

	#[derive(Configurable, Clone)]
	struct StringTest {
		s1: Vec<String>,
	}

	impl Default for StringTest {
		fn default() -> Self {
			Self { s1: vec![] }
		}
	}
		*/

	#[derive(Configurable, Clone, Debug)]
	struct Level1 {
		s1: String,
		s2: usize,
		s3: usize,
		mmm: Vec<usize>,
		vv123: u8,
		vv456: Vec<u8>,
	}

	#[derive(Configurable, Clone, Debug)]
	struct Level2 {
		l1: Level1,
	}

	#[derive(Configurable, Clone, Debug)]
	struct Level3 {
		l2: Level2,
		larr: Vec<Level1>,
	}

	#[derive(Configurable, Clone, Debug)]
	struct Level4 {
		conf: Level3,
		name: String,
	}

	impl Default for Level4 {
		fn default() -> Self {
			Self {
				conf: Level3::default(),
				name: "".to_string(),
			}
		}
	}

	impl Default for Level1 {
		fn default() -> Self {
			Self {
				s1: "".to_string(),
				s2: 1,
				s3: 2,
				vv123: 6u8,
				vv456: vec![],
				mmm: vec![],
			}
		}
	}

	impl Default for Level2 {
		fn default() -> Self {
			Self {
				l1: Level1::default(),
			}
		}
	}

	impl Default for Level3 {
		fn default() -> Self {
			Self {
				l2: Level2::default(),
				larr: vec![],
			}
		}
	}
	/*
		#[derive(Configurable, Clone)]
		struct Abc {
			x: usize,
			s: String,
		}
	*/

	#[test]
	fn test_configurable() -> Result<(), Error> {
		let level4 = configure_box!(
			Level4,
			Level4Options,
			vec![
				Name("test"),
				Conf(configure_box!(
					Level3,
					Level3Options,
					vec![Larr(configure_box!(
						Level1,
						Level1Options,
						vec![S1("ok"), Mmm(101)]
					)?)]
				)?)
			]
		)?;
		assert_eq!(level4.name, "test".to_string());
		assert_eq!(level4.conf.larr.len(), 1);
		assert_eq!(level4.conf.larr[0].s1, "ok".to_string());
		assert_eq!(level4.conf.larr[0].mmm, vec![101]);
		/*
		let level1 = configure_box!(Level1, Level1Options, vec![S1("test")])?;
		assert_eq!(level1.vv123, 6u8);
		let empty_vec: Vec<u8> = vec![];
		assert_eq!(level1.vv456, empty_vec);
		let level2 = configure_box!(Level2, Level2Options, vec![L1(level1)])?;

		let larr1 = configure_box!(Level1, Level1Options, vec![S1("testlarr1")])?;
		let larr2 = configure_box!(Level1, Level1Options, vec![S1("testlarr2")])?;
		let larr3 = configure_box!(Level1, Level1Options, vec![S1("testlarr3")])?;
		println!("start level3");
		let level3 = configure!(
			Level3,
			Level3Options,
			vec![L2(level2), Larr(larr1), Larr(larr2), Larr(larr3)]
		)?;

		println!("level3={:?}", level3);

		assert_eq!(level3.l2.l1.s1, "test".to_string());
		assert_eq!(level3.larr.len(), 3);
		assert_eq!(level3.larr[0].s1, "testlarr1".to_string());

		let x = configure!(StringTest, StringTestOptions, vec![])?;

		let empty_vec: Vec<String> = vec![];
		assert_eq!(x.s1, empty_vec);
		let x = configure!(StringTest, StringTestOptions, vec![S1("aaa")])?;
		assert_eq!(x.s1, vec!["aaa".to_string()]);
		assert!(configure!(RequiredTest, RequiredTestOptions, vec![Ghi(1)]).is_ok());
		assert!(configure!(RequiredTest, RequiredTestOptions, vec![Def(1)]).is_err());

		let test_vec = configure!(TestVec, TestVecOptions, vec![Abc(1), Abc(103)])?;
		assert_eq!(test_vec.abc, vec![1, 103]);

		let adv2 = configure!(TestVecVec, TestVecVecOptions, vec![Def(8)])?;

		assert_eq!(adv2.def, 8);
		assert_eq!(adv2.config.abc, vec![]);

		let adv2 = configure!(
			TestVecVec,
			TestVecVecOptions,
			vec![
				Def(8),
				Config(configure_box!(
					TestVec,
					TestVecOptions,
					vec![Abc(12345), Abc(99)]
				)?)
			]
		)?;

		assert_eq!(adv2.def, 8);
		assert_eq!(adv2.config.abc, vec![12345, 99]);

				*/

		let level1 = configure_box!(
			Level1,
			Level1Options,
			vec![Vv123(7u8), S1("test"), Vv456(1u8), Vv456(2u8)]
		)?;
		assert_eq!(level1.vv123, 7u8);
		assert_eq!(level1.vv456, vec![1, 2]);

		/*
		let adv = configure!(
			MyConfigurableAdvanced,
			MyConfigurableAdvancedOptions,
			vec![
				FirstValue(7),
				Config(configure_box!(
					MyConfigurable,
					MyConfigurableOptions,
					vec![Threads(101), SlabSize(30)]
				)?)
			]
		)?;

		assert_eq!(adv.first_value, 7);
		assert_eq!(adv.config.threads, 101);
		assert_eq!(adv.config.slab_size, 30);
		assert_eq!(adv.config.timeout, 2);
			*/

		/*
		let x = configure!(TestConfig, TestConfigOptions, vec![V1(4), V2(100)])?;
		assert_eq!(x.v1, 4);
		assert_eq!(x.v2, 100);
		let x = configure!(TestConfig, TestConfigOptions, vec![])?;
		assert_eq!(x.v1, 1);
		assert_eq!(x.v2, 0);

		let x = configure!(TestGlobalConfig, TestGlobalConfigOptions, vec![])?;

		assert_eq!(x.config.v1, 1);
		assert_eq!(x.config.v2, 0);
		assert_eq!(x.v3, 101);

		let y = configure_box!(TestConfig, TestConfigOptions, vec![V1(1234)])?;
		let x = configure!(
			TestGlobalConfig,
			TestGlobalConfigOptions,
			vec![
				V3(10),
				Config(configure_box!(
					TestConfig,
					TestConfigOptions,
					vec![V1(1234)]
				)?)
			]
		)?;
		assert_eq!(x.v3, 10);
		assert_eq!(x.config.v1, 1234);
		assert_eq!(x.config.v2, 0);
				*/

		/*
		let x = configure!(MyConfigurable, MyConfigurableOptions, vec![])?;
		assert_eq!(x.threads, 1);
		assert_eq!(x.timeout, 2);
		assert_eq!(x.slab_size, 3);

		let x = configure!(
			MyConfigurable,
			MyConfigurableOptions,
			vec![Threads(5), Timeout(6)]
		)?;
		assert_eq!(x.threads, 5);
		assert_eq!(x.timeout, 6);
		assert_eq!(x.slab_size, 3);
				*/

		Ok(())
	}

	/*
	 * let slabs = slab_allocator!(slab_config!(SlabSize(10), SlabCount(100))?, SlabsPerResize(100))?;
	 * let http_client = http_client!(
	 *      Timeout(10_000),
	 *      header!(
	 *          Name("Connection"),
	 *          Value("Keep-Alive")
	 *      )?,
	 *      header!(
	 *          Name("Content-Type"),
	 *          Value("text/html")
	 *      )
	 * )?;
	 */

	/*
	macro_rules! test_config {
		($($param:tt)*) => {{
					let my_vec: Vec<TestEnum> = vec![$($param)*];
					println!("myvec={:?}", my_vec);
				}};
	}

	#[derive(Debug)]
	enum TestEnum {
		SlabConfig((TestEnum2, TestEnum2)),
		SlabsPerResize(usize),
		Threads(u64),
	}

	#[derive(Debug)]
	enum TestEnum2 {
		SlabCount(usize),
		SlabSize(usize),
	}

	use TestEnum::*;
	use TestEnum2::*;

	#[test]
	fn test_enums() -> Result<(), Error> {
		let z = vec![
			SlabConfig((SlabCount(10), SlabSize(100))),
			SlabsPerResize(300),
		];
		test_config!(
			SlabsPerResize(300),
			SlabConfig((SlabCount(10), SlabSize(100))),
			SlabConfig((SlabCount(20), SlabSize(200))),
			Threads(10)
		);
		Ok(())
	}
		*/
}
