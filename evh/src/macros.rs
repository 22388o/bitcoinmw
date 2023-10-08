// Copyright (c) 2022, 37 Miners, LLC
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

/// Build an [`crate::EventHandler`] instance. See module level documentation for examples.
/// Optionally, an [`crate::EventHandlerConfig`] may be specified. If none is specified,
/// the default values are used.
#[macro_export]
macro_rules! eventhandler {
	() => {{
		let config = bmw_evh::EventHandlerConfig::default();
		bmw_evh::Builder::build_evh(config)
	}};
	($config:expr) => {{
		bmw_evh::Builder::build_evh($config)
	}};
}

#[cfg(test)]
mod test {
	use crate as bmw_evh;
	use bmw_err::*;
	use bmw_evh::*;

	#[test]
	fn test_evh_macros() -> Result<(), Error> {
		let mut evh = eventhandler!()?;
		evh.start()?;
		evh.set_on_read(move |_, _, _| Ok(()))?;
		evh.set_on_accept(move |_, _| Ok(()))?;
		evh.set_on_close(move |_, _| Ok(()))?;
		evh.set_on_panic(move |_, _| Ok(()))?;
		evh.set_housekeeper(move |_| Ok(()))?;

		let config = EventHandlerConfig {
			threads: 3,
			..Default::default()
		};
		let mut evh2 = eventhandler!(config)?;
		evh2.start()?;
		evh2.set_on_read(move |_, _, _| Ok(()))?;
		evh2.set_on_accept(move |_, _| Ok(()))?;
		evh2.set_on_close(move |_, _| Ok(()))?;
		evh2.set_on_panic(move |_, _| Ok(()))?;
		evh2.set_housekeeper(move |_| Ok(()))?;

		Ok(())
	}
}
