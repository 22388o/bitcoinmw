// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::types::CircuitImpl;
use crate::{Builder, Cell, Circuit, CircuitPlan, CircuitState, Peer, Stream};
use bmw_deps::ed25519_dalek::SecretKey;
use bmw_err::*;
use std::io::{Read, Write};

impl CircuitImpl {
	fn new(plan: CircuitPlan, local_peer: Peer, secret_key: SecretKey) -> Result<Self, Error> {
		if plan.hops.len() < 2 {
			return Err(err!(
				ErrKind::IllegalArgument,
				"circuit plan must have at least 2 hops"
			));
		}
		Ok(Self {
			plan,
			channel: None,
			local_peer,
			secret_key,
		})
	}
}

impl Circuit for CircuitImpl {
	fn start(&mut self) -> Result<(), Error> {
		let mut channel = Builder::channel(self.local_peer.clone())?;
		channel.connect(&self.plan.hops[0], &self.secret_key)?;
		channel.start()?;
		self.channel = Some(channel);

		Ok(())
	}
	fn open_stream(&mut self) -> Result<Box<dyn Stream>, Error> {
		todo!()
	}
	fn get_stream(&self, _sid: u16) -> Result<Box<dyn Stream>, Error> {
		todo!()
	}
	fn close_stream(&mut self, _sid: u16) -> Result<(), Error> {
		todo!()
	}
	fn close_circuit(&mut self) -> Result<(), Error> {
		todo!()
	}
	fn read_crypt(&mut self, rd: &mut dyn Read) -> Result<usize, Error> {
		match self.channel.as_mut() {
			Some(channel) => channel.read_crypt(rd),
			None => Err(err!(ErrKind::IllegalState, "circuit not started yet")),
		}
	}
	fn write_crypt(&mut self, wr: &mut dyn Write) -> Result<usize, Error> {
		match self.channel.as_mut() {
			Some(channel) => channel.write_crypt(wr),
			None => Err(err!(ErrKind::IllegalState, "circuit not started yet")),
		}
	}
	fn send_cell(&mut self, _cell: Cell) -> Result<(), Error> {
		Ok(())
	}
	fn process_new_packets(&mut self, _state: &mut CircuitState) -> Result<(), Error> {
		todo!()
	}
}

#[cfg(test)]
mod test {
	use crate::types::{CircuitImpl, RngCompatExt};
	use crate::{Circuit, CircuitPlan, CircuitState, Peer};
	use bmw_deps::ed25519_dalek::Keypair;
	use bmw_deps::ed25519_dalek::SecretKey;
	use bmw_deps::rand::thread_rng;
	use bmw_err::*;
	use std::io::{Read, Write};

	struct RouteSimulator {
		secrets: Vec<SecretKey>,
		peers: Vec<Peer>,
	}

	impl RouteSimulator {
		fn new(secrets: Vec<SecretKey>, peers: Vec<Peer>) -> Self {
			Self { secrets, peers }
		}
		fn start(&mut self) -> Result<(), Error> {
			Ok(())
		}
		fn read_crypt(&mut self, _rd: &mut dyn Read) -> Result<usize, Error> {
			todo!()
		}
		fn write_crypt(&mut self, _wr: &mut dyn Write) -> Result<usize, Error> {
			todo!()
		}

		fn process_new_packets(&mut self, _state: &mut CircuitState) -> Result<(), Error> {
			todo!()
		}
	}

	#[test]
	fn test_crypt_circuit_basic() -> Result<(), Error> {
		let mut rng = thread_rng().rng_compat();
		let local = Keypair::generate(&mut rng);
		let hop1 = Keypair::generate(&mut rng);
		let hop2 = Keypair::generate(&mut rng);
		let local_peer = Peer {
			sockaddr: "127.0.0.1:8080".parse()?,
			pubkey: local.public,
			nickname: "test0".to_string(),
		};
		let peer1 = Peer {
			sockaddr: "127.0.0.1:8081".parse()?,
			pubkey: hop1.public,
			nickname: "test1".to_string(),
		};
		let peer2 = Peer {
			sockaddr: "127.0.0.1:8082".parse()?,
			pubkey: hop2.public,
			nickname: "test2".to_string(),
		};
		let plan = CircuitPlan {
			hops: vec![peer1.clone(), peer2.clone()],
		};
		let mut circuit = CircuitImpl::new(plan, local_peer, local.secret)?;
		circuit.start()?;

		let mut secrets = vec![];
		secrets.push(hop1.secret);
		secrets.push(hop2.secret);
		let mut peers = vec![];
		peers.push(peer1);
		peers.push(peer2);
		let _route_simulator = RouteSimulator::new(secrets, peers);

		Ok(())
	}
}
