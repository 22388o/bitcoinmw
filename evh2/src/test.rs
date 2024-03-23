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

#[cfg(test)]
mod test {
	use crate::EvhBuilder;
	use bmw_conf::ConfigOption;
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::*;

	debug!();

	#[test]
	fn test_evh_basic() -> Result<(), Error> {
		let mut evh = EvhBuilder::build_evh(vec![ConfigOption::Debug(true)])?;

		evh.set_on_read(move |_connection, _ctx| -> Result<(), Error> {
			info!("onRead")?;
			Ok(())
		})?;

		evh.set_on_accept(move |_connection, _ctx| -> Result<(), Error> {
			info!("onAccept")?;
			Ok(())
		})?;

		evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
			info!("onClose")?;
			Ok(())
		})?;

		evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
			info!("onHousekeeper")?;
			Ok(())
		})?;

		evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
			info!("onPanic")?;
			Ok(())
		})?;

		evh.start()?;

		sleep(Duration::from_millis(3_000));

		Ok(())
	}
}
