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

#[macro_export]
macro_rules! evh {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                use bmw_evh2::EvhBuilder;

                let v: Vec<ConfigOption> = vec![$($config)*];
                EvhBuilder::build_evh(v)
        }};
}

#[macro_export]
macro_rules! evh_oro {
        ($($config:tt)*) => {{
                #[allow(unused_imports)]
                use bmw_conf::ConfigOption::*;
                use bmw_conf::ConfigOption;
                use bmw_evh2::EvhBuilder;

                let v: Vec<ConfigOption> = vec![$($config)*];
                match EvhBuilder::build_evh(v) {
                        Ok(mut evh) => {

                                evh.set_on_accept(move |_connection, _ctx| -> Result<(), Error> {
                                        Ok(())
                                })?;

                                evh.set_on_close(move |_connection, _ctx| -> Result<(), Error> {
                                        Ok(())
                                })?;

                                evh.set_on_housekeeper(move |_ctx| -> Result<(), Error> {
                                        Ok(())
                                })?;

                                evh.set_on_panic(move |_ctx, _e| -> Result<(), Error> {
                                        Ok(())
                                })?;

                                Ok(evh)},
                        Err(e) => {
                                let text = format!("build_evh resulted in error: {}", e);
                                Err(err!(ErrKind::Configuration, text))
                        }
                }

        }};
}
