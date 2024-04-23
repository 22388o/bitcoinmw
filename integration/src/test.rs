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

use bmw_base::*;
use bmw_derive::*;
use std::collections::HashSet;
use std::pin::Pin;

#[class {
        const y: usize = 1;
        const z: u8 = 10;
        var x: i32;
        var v: usize;

        // test
        // ok ok ok
        fn builder(&const_values) -> Result<Self, Error> {
                Ok(Self { x: -100, v: *const_values.get_y() })
        }

        [dog, test]
        fn bark(&mut self) -> Result<String, Error> {
                *self.get_mut_x() += 1;
                println!("x={}", self.get_x());
                Ok("woof".to_string())
        }

        [cat, test]
        fn meow(&mut self, v1: usize) -> Result<String, Error> {
                self.other();
                Ok("meow".to_string())
        }

        fn other(&self) {
            let value: u8 = *self.get_z();
            println!("v+1={}", value+1);
        }
}]
impl Animal2 {}
// impl Animal2 <OnRead> where OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,{}

#[allow(dead_code)]
struct Animal2Var<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	x: i32,
	v: usize,
	bbb: HashSet<String>,
	on_read: Option<Pin<Box<OnRead>>>,
}

#[allow(dead_code)]
struct Animal2Const {
	y: usize,
	z: u8,
}

#[allow(dead_code)]
struct Animal2<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	hidden_var_struct: Animal2Var<OnRead>,
	hidden_const_struct: Animal2Const,
}

pub trait Dog {
	fn bark(&mut self) -> Result<String, Error>;
}

#[allow(dead_code)]
impl Animal2Const {
	fn get_y(&self) -> &usize {
		&self.y
	}

	fn get_z(&self) -> &u8 {
		&self.z
	}
}

#[allow(dead_code)]
impl<OnRead> Animal2Var<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	fn builder(const_values: &Animal2Const) -> Result<Self, Error> {
		Ok(Self {
			x: -100,
			v: *const_values.get_y(),
			bbb: HashSet::new(),
			on_read: None,
		})
	}

	fn get_x(&self) -> &i32 {
		&self.x
	}

	fn get_mut_x(&mut self) -> &mut i32 {
		&mut self.x
	}

	fn get_v(&self) -> &usize {
		&self.v
	}

	fn get_mut_v(&mut self) -> &mut usize {
		&mut self.v
	}
}

#[allow(dead_code)]
impl<OnRead> Animal2<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	fn builder(const_values: Animal2Const) -> Result<Self, Error> {
		Ok(Self {
			hidden_var_struct: Animal2Var::builder(&const_values)?,
			hidden_const_struct: const_values,
		})
	}

	fn bark(&mut self) -> Result<String, Error> {
		*self.get_mut_x() += 1;
		println!("x={}", self.get_x());
		Ok("woof".to_string())
	}

	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error> {
		let x = &mut self.hidden_var_struct.on_read;
		*x = Some(Box::pin(on_read));
		Ok(())
	}

	fn other(&self) {
		let value: u8 = *self.get_z();
		println!("v+1={}", value + 1);
	}

	fn get_x(&self) -> &i32 {
		self.hidden_var_struct.get_x()
	}

	fn get_mut_x(&mut self) -> &mut i32 {
		self.hidden_var_struct.get_mut_x()
	}

	fn get_v(&self) -> &usize {
		self.hidden_var_struct.get_v()
	}

	fn get_mut_v(&mut self) -> &mut usize {
		self.hidden_var_struct.get_mut_v()
	}

	fn get_y(&self) -> &usize {
		self.hidden_const_struct.get_y()
	}

	fn get_z(&self) -> &u8 {
		self.hidden_const_struct.get_z()
	}
}

#[allow(dead_code)]
impl<OnRead> Dog for Animal2<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	fn bark(&mut self) -> Result<String, Error> {
		Animal2::bark(self)
	}
}

#[allow(dead_code)]
impl<OnRead> Dog for &mut Animal2<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	fn bark(&mut self) -> Result<String, Error> {
		Animal2::bark(self)
	}
}

struct X<OnRead>
where
	OnRead: FnMut() -> Result<(), Error> + Send + 'static + Clone + Sync + Unpin,
{
	on_read: Option<Pin<Box<OnRead>>>,
}

#[cfg(test)]
mod test {
	use bmw_base::*;
	use bmw_deps::failure::Fail;
	use bmw_derive::*;

	#[derive(ErrorKind, Debug, Fail)]
	enum IntErrorKind {
		/// Test the integration of errors
		#[fail(display = "integration error: {}", _0)]
		Integration(String),
	}

	fn ret_err() -> Result<(), Error> {
		err!(IntErrorKind::Integration, "this is a test {}", 1)
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		assert!(ret_err().is_err());

		let err: Error = ret_err().unwrap_err();
		let kind = err.kind();

		assert_eq!(kind, &kind!(IntErrorKind::Integration, "this is a test 1"));

		Ok(())
	}

	#[object]
	impl Animal {
		#[config(y: usize = 1)]
		#[config(z: u8 = 10)]
		#[field(x: i32)]
		#[builder]
		fn builder10(config: AnimalConfig) -> Result<Self, Error> {
			Ok(Self { config, x: -100 })
		}

		#[method(dog, test)]
		fn bark(&mut self) -> Result<String, Error> {
			if self.config.y == 1 {
				Ok("WOOF!".to_string())
			} else {
				Ok("woof!".to_string())
			}
		}

		#[method(dog, cat)]
		fn wag(&self) -> Result<i32, Error> {
			Ok(self.x + 1)
		}

		#[method(cat)]
		fn ret10(&self) -> Result<u32, Error> {
			Ok(10)
		}

		#[method(test)]
		fn as_dog(&mut self) -> Result<Box<dyn Dog + '_>, Error> {
			let dog: Box<dyn Dog> = Box::new(self);
			Ok(dog)
		}
	}

	#[test]
	fn test_object() -> Result<(), Error> {
		let mut dog = dog!(Y(2))?;
		assert_eq!(dog.bark()?, "woof!");
		let mut dog = dog!(Y(1))?;
		assert_eq!(dog.bark()?, "WOOF!");
		let mut dog = dog!(Y(10))?;
		assert_eq!(dog.bark()?, "woof!");
		assert_eq!(dog.wag()?, -99);
		let mut test = test!(Y(20))?;
		assert_eq!(test.bark()?, "woof!");

		let mut test = test!(Y(1))?;
		assert_eq!(test.bark()?, "WOOF!");

		let mut test2 = test.as_dog()?;
		assert_eq!(test2.bark()?, "WOOF!");

		let cat = cat!()?;
		assert_eq!(cat.ret10(), Ok(10));

		Ok(())
	}
}
