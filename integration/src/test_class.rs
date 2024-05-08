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

use bmw_core::*;

#[class{
        clone http_server;
	no_send;
	pub http_server_box;
	const instance: Vec<Instance> = vec![];
	const threads: usize = 1;

	[http_server]
	fn show(&self);
}]
impl Server {
	fn builder(&self) -> Result<Self, Error> {
		Ok(Self {})
	}
}

impl Server {
	fn show(&self) {
		let mut i = 0;
		for instance in self.constants().get_instance() {
			println!("instance[{}] = {:?}", i, instance);
			i += 1;
		}
	}
}

#[allow(unused_macros)]
macro_rules! instance {
        ($($params:tt)*) => {{
            configure_box!(Instance, InstanceOptions, vec![$($params)*])
        }};
}

#[derive(Clone, Configurable, Debug)]
struct Instance {
	port: u16,
	address: String,
}

impl Default for Instance {
	fn default() -> Self {
		Self {
			port: 1234,
			address: "127.0.0.1".to_string(),
		}
	}
}

/*
#[class{
	no_send;
	clone http_instance;
	pub(crate) http_instance_box;
	const port: u16 = 8000;

	[http_instance]
	fn get_port(&self) -> u16;
}]
impl Instance {
	fn builder(&self) -> Result<Self, Error> {
		Ok(Self {})
	}
}

impl Instance {
	fn get_port(&self) -> u16 {
		*self.constants().get_port()
	}
}

impl Configurable for Server {
	fn set_u8(&mut self, name: &str, value: u8) {}
	fn set_u16(&mut self, name: &str, value: u16) {
		self._hidden_const_struct.set_u16(name, value)
	}
	fn set_u32(&mut self, name: &str, value: u32) {}
	fn set_u64(&mut self, name: &str, value: u64) {}
	fn set_u128(&mut self, name: &str, value: u128) {}
	fn set_usize(&mut self, name: &str, value: usize) {
		self._hidden_const_struct.set_usize(name, value)
	}
	fn set_string(&mut self, _: &str, _: std::string::String) {}
	fn set_bool(&mut self, _: &str, _: bool) {}
	fn set_configurable(&mut self, name: &str, value: &dyn bmw_core::Configurable) {
		self._hidden_const_struct.set_configurable(name, value)
	}

	fn allow_dupes(&self) -> HashSet<std::string::String> {
		HashSet::new()
	}
	fn required(&self) -> Vec<std::string::String> {
		vec![]
	}
	fn get_usize_params(&self) -> Vec<(std::string::String, usize)> {
		self._hidden_const_struct.get_usize_params()
	}
	fn get_u8_params(&self) -> Vec<(std::string::String, u8)> {
		vec![]
	}
	fn get_u16_params(&self) -> Vec<(std::string::String, u16)> {
		self._hidden_const_struct.get_u16_params()
	}
	fn get_u32_params(&self) -> Vec<(std::string::String, u32)> {
		vec![]
	}
	fn get_u64_params(&self) -> Vec<(std::string::String, u64)> {
		vec![]
	}
	fn get_u128_params(&self) -> Vec<(std::string::String, u128)> {
		vec![]
	}
	fn get_bool_params(&self) -> Vec<(std::string::String, bool)> {
		vec![]
	}
	fn get_string_params(&self) -> Vec<(std::string::String, std::string::String)> {
		vec![]
	}
	fn get_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		std::boxed::Box<(dyn bmw_core::Configurable + 'static)>,
	)> {
		self._hidden_const_struct.get_configurable_params()
	}
	fn get_vec_usize_params(&self) -> Vec<(std::string::String, Vec<usize>)> {
		vec![]
	}
	fn get_vec_u8_params(&self) -> Vec<(std::string::String, Vec<u8>)> {
		vec![]
	}
	fn get_vec_u16_params(&self) -> Vec<(std::string::String, Vec<u16>)> {
		vec![]
	}
	fn get_vec_u32_params(&self) -> Vec<(std::string::String, Vec<u32>)> {
		vec![]
	}
	fn get_vec_u64_params(&self) -> Vec<(std::string::String, Vec<u64>)> {
		vec![]
	}
	fn get_vec_u128_params(&self) -> Vec<(std::string::String, Vec<u128>)> {
		vec![]
	}
	fn get_vec_bool_params(&self) -> Vec<(std::string::String, Vec<bool>)> {
		vec![]
	}
	fn get_vec_string_params(&self) -> Vec<(std::string::String, Vec<std::string::String>)> {
		vec![]
	}

	fn get_vec_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		Vec<std::boxed::Box<(dyn bmw_core::Configurable + 'static)>>,
	)> {
		vec![]
	}
}

impl Configurable for Instance {
	fn set_u8(&mut self, name: &str, value: u8) {}
	fn set_u16(&mut self, name: &str, value: u16) {
		self._hidden_const_struct.set_u16(name, value)
	}
	fn set_u32(&mut self, name: &str, value: u32) {}
	fn set_u64(&mut self, name: &str, value: u64) {}
	fn set_u128(&mut self, name: &str, value: u128) {}
	fn set_usize(&mut self, name: &str, value: usize) {}
	fn set_string(&mut self, _: &str, _: std::string::String) {}
	fn set_bool(&mut self, _: &str, _: bool) {}
	fn set_configurable(&mut self, _: &str, _: &dyn bmw_core::Configurable) {}
	fn allow_dupes(&self) -> HashSet<std::string::String> {
		HashSet::new()
	}
	fn required(&self) -> Vec<std::string::String> {
		vec![]
	}
	fn get_usize_params(&self) -> Vec<(std::string::String, usize)> {
		vec![]
	}
	fn get_u8_params(&self) -> Vec<(std::string::String, u8)> {
		vec![]
	}
	fn get_u16_params(&self) -> Vec<(std::string::String, u16)> {
		self._hidden_const_struct.get_u16_params()
	}
	fn get_u32_params(&self) -> Vec<(std::string::String, u32)> {
		vec![]
	}
	fn get_u64_params(&self) -> Vec<(std::string::String, u64)> {
		vec![]
	}
	fn get_u128_params(&self) -> Vec<(std::string::String, u128)> {
		vec![]
	}
	fn get_bool_params(&self) -> Vec<(std::string::String, bool)> {
		vec![]
	}
	fn get_string_params(&self) -> Vec<(std::string::String, std::string::String)> {
		vec![]
	}
	fn get_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		std::boxed::Box<(dyn bmw_core::Configurable + 'static)>,
	)> {
		vec![]
	}
	fn get_vec_usize_params(&self) -> Vec<(std::string::String, Vec<usize>)> {
		vec![]
	}
	fn get_vec_u8_params(&self) -> Vec<(std::string::String, Vec<u8>)> {
		vec![]
	}
	fn get_vec_u16_params(&self) -> Vec<(std::string::String, Vec<u16>)> {
		vec![]
	}
	fn get_vec_u32_params(&self) -> Vec<(std::string::String, Vec<u32>)> {
		vec![]
	}
	fn get_vec_u64_params(&self) -> Vec<(std::string::String, Vec<u64>)> {
		vec![]
	}
	fn get_vec_u128_params(&self) -> Vec<(std::string::String, Vec<u128>)> {
		vec![]
	}
	fn get_vec_bool_params(&self) -> Vec<(std::string::String, Vec<bool>)> {
		vec![]
	}
	fn get_vec_string_params(&self) -> Vec<(std::string::String, Vec<std::string::String>)> {
		vec![]
	}

	fn get_vec_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		Vec<std::boxed::Box<(dyn bmw_core::Configurable + 'static)>>,
	)> {
		vec![]
	}
}

impl Configurable for Box<dyn HttpInstance> {
	fn set_u8(&mut self, name: &str, value: u8) {}
	fn set_u16(&mut self, name: &str, value: u16) {
		self.configurable_mut().set_u16(name, value)
	}
	fn set_u32(&mut self, name: &str, value: u32) {}
	fn set_u64(&mut self, name: &str, value: u64) {}
	fn set_u128(&mut self, name: &str, value: u128) {}
	fn set_usize(&mut self, name: &str, value: usize) {}
	fn set_string(&mut self, _: &str, _: std::string::String) {}
	fn set_bool(&mut self, _: &str, _: bool) {}
	fn set_configurable(&mut self, _: &str, _: &dyn bmw_core::Configurable) {}
	fn allow_dupes(&self) -> HashSet<std::string::String> {
		HashSet::new()
	}
	fn required(&self) -> Vec<std::string::String> {
		vec![]
	}
	fn get_usize_params(&self) -> Vec<(std::string::String, usize)> {
		vec![]
	}
	fn get_u8_params(&self) -> Vec<(std::string::String, u8)> {
		vec![]
	}
	fn get_u16_params(&self) -> Vec<(std::string::String, u16)> {
		self.configurable().get_u16_params()
	}
	fn get_u32_params(&self) -> Vec<(std::string::String, u32)> {
		vec![]
	}
	fn get_u64_params(&self) -> Vec<(std::string::String, u64)> {
		vec![]
	}
	fn get_u128_params(&self) -> Vec<(std::string::String, u128)> {
		vec![]
	}
	fn get_bool_params(&self) -> Vec<(std::string::String, bool)> {
		vec![]
	}
	fn get_string_params(&self) -> Vec<(std::string::String, std::string::String)> {
		vec![]
	}
	fn get_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		std::boxed::Box<(dyn bmw_core::Configurable + 'static)>,
	)> {
		vec![]
	}
	fn get_vec_usize_params(&self) -> Vec<(std::string::String, Vec<usize>)> {
		vec![]
	}
	fn get_vec_u8_params(&self) -> Vec<(std::string::String, Vec<u8>)> {
		vec![]
	}
	fn get_vec_u16_params(&self) -> Vec<(std::string::String, Vec<u16>)> {
		vec![]
	}
	fn get_vec_u32_params(&self) -> Vec<(std::string::String, Vec<u32>)> {
		vec![]
	}
	fn get_vec_u64_params(&self) -> Vec<(std::string::String, Vec<u64>)> {
		vec![]
	}
	fn get_vec_u128_params(&self) -> Vec<(std::string::String, Vec<u128>)> {
		vec![]
	}
	fn get_vec_bool_params(&self) -> Vec<(std::string::String, Vec<bool>)> {
		vec![]
	}
	fn get_vec_string_params(&self) -> Vec<(std::string::String, Vec<std::string::String>)> {
		vec![]
	}

	fn get_vec_configurable_params(
		&self,
	) -> Vec<(
		std::string::String,
		Vec<std::boxed::Box<(dyn bmw_core::Configurable + 'static)>>,
	)> {
		vec![]
	}
}

fn _vvv<X>(_i: usize) -> X
where
	X: Send + Sync + 'static,
{
	todo!()
}

#[class{
		//no_sync;
		no_send;
	var y: Option<&'a usize>;
		var z: Option<A>;

		[test_abc_1]
		fn unimp(&self);

	[test_abc_1]
	fn x(&mut self, v: A);

}]
impl<'a, A> TestLifetimes<'a, A>
where
	A: 'a,
{
	fn builder(&self) -> Result<Self, Error> {
		Ok(Self { y: None, z: None })
	}
}

impl<'a, A> TestLifetimes<'a, A>
where
	A: 'a,
{
	fn x(&mut self, _v: A) {}
	fn unimp(&self) {}
}

#[derive(Clone)]
struct Abc {
	x: usize,
}

#[class {
	clone cat;
	pub cat as catmapped, dog_send, cat_box;
	pub(crate) dog_box;
	pub(crate) bwrp, monkey_box;

	pub monkey_sync_box as monmon;

	module "bmw_int::test_class";
	const x: usize = usize::MAX - 10;
	const vvv: Vec<u16> = vec![1,2,3];
	var x: Option<B>;
	var a: Abc;
	var t: String;
	const p: Vec<usize> = vec![];
	const v123: usize = 0;
	var y: usize;
	var b: bool;

	[cat, dog, monkey, bwrp]
	fn speak(&self, x: usize, v: Option<Vec<(usize, Box<dyn std::fmt::Debug + Send + Sync + '_>,)>>)
	-> Result<(), Error>;

	[cat]
	fn ok(&mut self);

	[cat]
	fn abc(&mut self);

	[dog, cat]
	fn x(&mut self, x: B) -> Result<(), Error>;

	[cat]
	fn meow(&mut self) -> Result<(), Error>;

	[dog]
	fn bark(&mut self) -> Result<(), Error>;

	[monkey]
	fn debug(&mut self);

}]
pub impl<B> Animal<B>
where
	B: Clone + Send + Sync + 'static,
{
	fn builder(&self) -> Result<Self, Error> {
		let a = Abc { x: 0 };
		println!("a.x={}", a.x);
		Ok(Self {
			t: "aaa".to_string(),
			y: 10,
			b: false,
			a,
			x: None,
		})
	}
}

impl<B> Animal<B>
where
	B: Clone + Send + Sync + 'static,
{
	fn x(&mut self, x: B) -> Result<(), Error> {
		*self.vars().get_mut_x() = Some(x);
		Ok(())
	}
	fn speak(
		&self,
		x: usize,
		v: Option<Vec<(usize, Box<dyn std::fmt::Debug + Send + Sync + '_>)>>,
	) -> Result<(), Error> {
		println!("hi: {:?} {:?}", x, v);
		Ok(())
	}

	fn meow(&mut self) -> Result<(), Error> {
		println!("meow, v123: {}", self.constants().get_v123());
		Ok(())
	}

	fn bark(&mut self) -> Result<(), Error> {
		todo!()
	}

	fn ok(&mut self) {
		println!("test");
	}

	fn abc(&mut self) {
		println!("in abc2: {}", self.vars().get_y());
		*self.vars().get_mut_y() += 1;
	}
}

#[class{
	pub test_box;
	clone test;
	var v: usize;

	[test]
	fn test_run(&mut self);

}]
impl XClone {
	fn builder(&self) -> Result<Self, Error> {
		Ok(Self { v: 0 })
	}
}

impl XClone {
	fn test_run(&mut self) {
		let x = self.vars().get_mut_v();
		println!("x={}", *x);
		*x += 1;
	}
}

fn test1(count: usize) {
	println!(
		"test1is_recursive={},count={}",
		bmw_core::is_recursive(),
		count
	);
	if count != 0 {
		test2(count - 1);
		test1(count - 1);
	}
}

fn test2(count: usize) {
	println!("test2is_recursive={},count={}", is_recursive(), count);
}
*/

/*
enum Test1<'a> {
	Test(&'a dyn Configurable),
}
*/

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_animal() -> Result<(), Error> {
		/*
		let mut cat = cat_box!()?;
		cat.abc();
		cat.abc();
		cat.abc();
		cat.x(0usize)?;

		let mut cat_clone = cat.clone();

		cat_clone.abc();
		cat_clone.abc();
		cat.abc();

		let mut x1 = test_box!()?;
		let mut x2 = x1.clone();
		x2.test_run();
		x2.test_run();
		x1.test_run();

		let mut m = test_abc_1_box!()?;
		m.x(0usize);

		test1(3);

		println!("test1(0)");
		test1(0);

		m.unimp();
			*/

		//let mut http = http_server_box!(Threads(100), HttpInstance(&instance))?;
		//http.x();
		//
		/*
		let test_conf = Box::new(TestConf {
			port: 9999,
			address: "0.0.0.0".to_string(),
		});
				*/

		let http = http_server_box!(
			Threads(100),
			Instance(instance!(Port(1113))?),
			Instance(instance!(Port(1233), Address("0.0.0.0"))?),
			Instance(instance!()?),
		)?;
		http.show();

		Ok(())
	}
}
