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

#[class {
    pub cat as catmapped, dog_send;
    pub(crate) dog_box;
    pub(crate) bwrp, monkey_box;

    pub monkey_sync_box as monmon;

    module "bmw_int::test_class";
    const x: usize = usize::MAX - 10;
    const vvv: Vec<u16> = vec![1,2,3];
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

    [dog]
    fn x(&mut self, x: Vec<usize>) -> Result<(), Error>;

    [cat]
    fn meow(&mut self) -> Result<(), Error>;

    [dog]
    fn bark(&mut self) -> Result<(), Error>;

    [monkey]
    fn debug(&mut self);

}]
pub impl Animal {
	fn builder(&self) -> Result<Self, Error> {
		Ok(Self {
			t: "aaa".to_string(),
			y: 10,
			b: false,
		})
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

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_animal() -> Result<(), Error> {
		let mut cat = catmapped!()?;
		cat.abc();
		cat.abc();
		cat.abc();
		Ok(())
	}
}
