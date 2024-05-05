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

	#[class{
                pub cat;
                pub(crate) dog_box;

                module "bmw_int::test_class";
                const x: usize = usize::MAX - 10;
                const vvv: Vec<u16> = vec![1,2,3];
                var t: String;
                const p: Vec<usize> = vec![];
                const v123: configurable = 0;
                var y: usize;
                var b: Box<dyn Any + Send + Sync + '_>;

                [cat, dog, monkey]
                fn speak(&self, x: usize, v: Option<Vec<(usize, Box<dyn std::fmt::Debug + Send + Sync + '_>,)>>)
                     -> Result<(), Error>;

                [dog]
                fn x(&mut self, x: Vec<usize>) -> Result<(), Error>;

                [cat]
                fn meow(&mut self) -> Result<(), Error>;

                [dog]
                fn bark(&mut self) -> Result<(), Error>;

                [monkey]
                fn debug(&mut self);

        }]
	pub(crate) impl<A, B> Animal<A, B: 'test>
	where
		A: std::fmt::Debug,
	{
		fn builder(&self) -> Result<Self, Error> {
			Ok(Self { y: 1 })
		}
	}

	impl Animal {
		fn speak(
			&self,
			x: usize,
			v: Option<Vec<(usize, Box<dyn std::fmt::Debug + Send + Sync + '_>)>>,
		) -> Result<(), Error> {
			println!("hi: {:?} {:?}", x, v);
			Ok(())
		}

		fn meow(&mut self) -> Result<(), Error> {
			todo!()
		}

		fn bark(&mut self) -> Result<(), Error> {
			todo!()
		}
	}

	struct Animal {}

	struct TestClass<A> {
		a: A,
	}

	impl<A> TestClass<A> where A: std::fmt::Debug {}

	#[test]
	fn test_animal() -> Result<(), Error> {
		// let animal = cat!(X(5))?;
		let _animal = Animal {};
		Ok(())
	}
}
