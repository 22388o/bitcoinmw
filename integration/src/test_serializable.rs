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
	use bmw_base::*;
	use bmw_deps::rand;
	use bmw_derive::Serializable;
	use std::fmt::Debug;

	#[derive(Serializable, PartialEq, Debug)]
	struct OtherSer {
		a: usize,
		b: String,
	}

	#[derive(Serializable, PartialEq, Debug)]
	struct SerAll {
		a: u8,
		b: i8,
		c: u16,
		d: i16,
		e: u32,
		f: i32,
		g: u64,
		h: i64,
		i: u128,
		j: i128,
		k: usize,
		l: bool,
		m: f64,
		n: char,
		v: Vec<u8>,
		o: Option<u8>,
		s: String,
		x: Vec<String>,
		y: Vec<Option<(String, ())>>,
		z: Option<Vec<OtherSer>>,
	}

	// helper function that serializes and deserializes a Serializable and tests them for
	// equality
	fn ser_helper<S: Serializable + Debug + PartialEq>(ser_in: S) -> Result<(), Error> {
		let mut v: Vec<u8> = vec![];
		serialize(&mut v, &ser_in)?;
		let ser_out: S = deserialize(&mut &v[..])?;
		assert_eq!(ser_out, ser_in);
		Ok(())
	}

	#[test]
	fn test_derive() -> Result<(), Error> {
		// create a SerAll with random values
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: false,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: Some(rand::random()),
			s: "abcdef".to_string(),
			x: vec!["123".to_string(), "456".to_string()],
			y: vec![
				None,
				None,
				None,
				Some(("hi".to_string(), ())),
				Some(("hi2".to_string(), ())),
			],
			z: None,
		};

		// test it
		ser_helper(ser_out)?;

		// create again with some other options
		let rand_u8: u8 = rand::random();
		let rand_ch: char = rand_u8 as char;
		let ser_out = SerAll {
			a: rand::random(),
			b: rand::random(),
			c: rand::random(),
			d: rand::random(),
			e: rand::random(),
			f: rand::random(),
			g: rand::random(),
			h: rand::random(),
			i: rand::random(),
			j: rand::random(),
			k: rand::random(),
			l: true,
			m: rand::random(),
			n: rand_ch,
			v: vec![rand::random(), rand::random(), rand::random()],
			o: None,
			s: "0123456789".to_string(),
			x: vec!["abc".to_string(), "def".to_string()],
			y: vec![],
			z: Some(vec![
				OtherSer {
					a: 4,
					b: "bstr".to_string(),
				},
				OtherSer {
					a: 5,
					b: "cstr".to_string(),
				},
			]),
		};

		// test it
		ser_helper(ser_out)?;
		Ok(())
	}
}
