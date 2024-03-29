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

/// The [`crate::evh!`] macro builds a [`crate::EventHandler`] implementation. It is returned as a
/// `Box<dyn EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> + Send + Sync>`. If an
/// error occurs, a [`bmw_err::Error`] is rerurned.
///
/// # Input Parameters
/// * EvhThreads ([`prim@usize`]) (optional) - The number of threads for this
/// [`crate::EventHandler`]. The default value is 4.
/// * EvhReadSlabSize ([`prim@usize`]) (optional) - The size of the slabs, in bytes, in the read slab
/// allocator. The read slab allocator is where request data is temporarily stored before it can be
/// processed and cleared. Each thread has it's own dedicated slab allocator. The default value is 512.
/// * EvhReadSlabCount ([`prim@usize`]) (optional) - The count of the slabs, in the read slab
/// allocator. The read slab allocator is where request data is temporarily stored before it can be
/// processed and cleared. Each thread has it's own dedicated slab allocator. The default value is
/// 1_000.
/// * EvhTimeout ([`prim@u16`]) (optional) - The time, in milliseconds that the event handler will
/// wait to get events, if none occur. This value is important for things like the house keeping
/// frequency and stats configurations because if no events occur and this value is too great, a
/// house keeper or stats action will not occur until this timeout occurs. If too small a number is
/// used, too many needless events occur, if too big, the periodic tasks will not occur until later
/// than desired. The default value is set to 1_000 (1 second).
/// * EvhHouseKeeperFrequencyMillis ([`prim@usize`]) (optional) - The frequency, in milliseconds,
/// at which the housekeeper callback is called. The housekeeper closure is set by the
/// [`crate::EventHandler::set_on_housekeeper`] function. It is important to note that each thread
/// executes it's own housekeeper function. The default value is 10_000 (10 seconds).
/// * EvhStatsUpdateMillis ([`prim@usize`]) - The frequency, in milliseconds, at which the stats
/// data is returned. The stats data may be retreived by calling the
/// [`crate::EventHandler::wait_for_stats`] function. The default value is 5_000 (5 seconds).
/// * Debug ([`bool`]) - If this parameter is set to true, additional debugging information will be
/// logged. This parameter must NOT be set in a production configuration.
///
/// # Returns
/// A `Ok(Box<dyn EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> + Send + Sync>)`
/// is returned on success. A [`bmw_err::Error`] is returned on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhReadSlabCount is 0.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhReadSlabSize is less than 25.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhTimeout is 0.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhHouseKeeperFrequencyMillis is 0.
///
/// # See also
/// See the [`crate`] documentation as well for the background information and motivation for this
/// crate.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_test::*;
/// use bmw_evh2::*;
/// use bmw_log::*;
/// use std::str::from_utf8;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     // create an evh with the specified configuration. The unspecified values are
///     //the defaults mentioned above.
///     let mut evh = evh!(
///         EvhTimeout(100),
///         EvhThreads(1),
///         EvhReadSlabSize(100),
///         EvhStatsUpdateMillis(5000),
///         Debug(true)
///     )?;
///
///     // set the on read handler
///     evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
///         let mut buf = [0u8; 1024];
///         let mut data: Vec<u8> = vec![];
///
///         // loop through each of the available chunks clone them to local
///         // variable and append to a vec.
///         loop {
///             let len = ctx.clone_next_chunk(connection, &mut buf)?;
///
///             if len == 0 {
///                 break;
///             }
///
///             data.extend(&buf[0..len]);
///         }
///
///         // convert returned data to a utf8 string
///         let dstring = from_utf8(&data)?;
///         info!("data[{}]='{}'", connection.id(), dstring)?;
///
///         // get a write handle
///         let mut wh = connection.write_handle()?;
///
///         // echo
///         wh.write(dstring.as_bytes())?;
///
///         // clear all chunks from this connection. Note that partial
///         // clearing is possible with the ctx.clear_through function
///         // or no data can be cleared at all in which case it can
///         // be accessed on a subsequent request. When the connection
///         // is closed, all data is cleared automatically.
///         ctx.clear_all(connection)?;
///
///         Ok(())
///     })?;
///
///     // set the handler to be executed when a new connection is accepted
///     evh.set_on_accept(move |connection, ctx| -> Result<(), Error> {
///         Ok(())
///     })?;
///
///     // set the handler to be executed when a connection is closed
///     evh.set_on_close(move |connection, ctx| -> Result<(), Error> {
///         Ok(())
///     })?;
///
///     // set a housekeeper handler to be executed on a per thread basis
///     // at the configured frequency
///     evh.set_on_housekeeper(move |ctx| -> Result<(), Error> {
///         Ok(())
///     })?;
///
///     // set a handler to be executed if there is a thread panic during the
///     // execution of a handler. Note that the on_read handler can tollerate
///     // thread panics. Behavior is undefined if any of the other handlers
///     // thread panic
///     evh.set_on_panic(move |id, e| -> Result<(), Error> {
///         Ok(())
///     })?;
///
///     evh.start()?;
///
///     Ok(())
/// }
///
///```
///
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

/// The [`crate::evh_oro!`] macro builds a [`crate::EventHandler`] implementation. This is a simplified
/// version of the [`crate::evh!`] macro. It allows the user to not have to specify the handlers
/// other than the [`crate::EventHandler::set_on_read`]. Hence, it is called `evh_oro` (evh on read
/// only). This macro is useful for testing, but also for simple use cases where only on read
/// processing is necessary. It can allow the user to avoid specifying the unused boilerplate code
/// that is required since all of the handlers must be specified with the [`crate::evh!`] macro.
/// The result is returned as a
/// `Box<dyn EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> + Send + Sync>`. If an
/// error occurs, a [`bmw_err::Error`] is rerurned.
///
/// # Input Parameters
/// * EvhThreads ([`prim@usize`]) (optional) - The number of threads for this
/// [`crate::EventHandler`]. The default value is 4.
/// * EvhReadSlabSize ([`prim@usize`]) (optional) - The size of the slabs, in bytes, in the read slab
/// allocator. The read slab allocator is where request data is temporarily stored before it can be
/// processed and cleared. Each thread has it's own dedicated slab allocator. The default value is 512.
/// * EvhReadSlabCount ([`prim@usize`]) (optional) - The count of the slabs, in the read slab
/// allocator. The read slab allocator is where request data is temporarily stored before it can be
/// processed and cleared. Each thread has it's own dedicated slab allocator. The default value is
/// 1_000.
/// * EvhTimeout ([`prim@u16`]) (optional) - The time, in milliseconds that the event handler will
/// wait to get events, if none occur. This value is important for things like the house keeping
/// frequency and stats configurations because if no events occur and this value is too great, a
/// house keeper or stats action will not occur until this timeout occurs. If too small a number is
/// used, too many needless events occur, if too big, the periodic tasks will not occur until later
/// than desired. The default value is set to 1_000 (1 second).
/// * EvhHouseKeeperFrequencyMillis ([`prim@usize`]) (optional) - The frequency, in milliseconds,
/// at which the housekeeper callback is called. The housekeeper closure is set by the
/// [`crate::EventHandler::set_on_housekeeper`] function. It is important to note that each thread
/// executes it's own housekeeper function. The default value is 10_000 (10 seconds).
/// * EvhStatsUpdateMillis ([`prim@usize`]) - The frequency, in milliseconds, at which the stats
/// data is returned. The stats data may be retreived by calling the
/// [`crate::EventHandler::wait_for_stats`] function. The default value is 5_000 (5 seconds).
/// * Debug ([`bool`]) - If this parameter is set to true, additional debugging information will be
/// logged. This parameter must NOT be set in a production configuration.
///
/// # Returns
/// A `Ok(Box<dyn EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> + Send + Sync>)`
/// is returned on success. A [`bmw_err::Error`] is returned on failure.
///
/// # Errors
/// * [`bmw_err::ErrKind::Configuration`] - If any values are specified other than the allowed
/// values mentioned above or if there are any duplicate parameters specified.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhReadSlabCount is 0.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhReadSlabSize is less than 25.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhTimeout is 0.
/// * [`bmw_err::ErrKind::Configuration`] - If EvhHouseKeeperFrequencyMillis is 0.
///
/// # See also
/// See the [`crate`] documentation as well for the background information and motivation for this
/// crate.
///
/// # Examples
///```
/// use bmw_err::*;
/// use bmw_test::*;
/// use bmw_evh2::*;
/// use bmw_log::*;
/// use std::str::from_utf8;
///
/// info!();
///
/// fn main() -> Result<(), Error> {
///     // create an evh with the specified configuration. The unspecified values are
///     //the defaults mentioned above.
///     let mut evh = evh_oro!(
///         EvhTimeout(100),
///         EvhThreads(1),
///         EvhReadSlabSize(100),
///         EvhStatsUpdateMillis(5000),
///         Debug(true)
///     )?;
///
///     // set the on read handler
///     evh.set_on_read(move |connection, ctx| -> Result<(), Error> {
///         let mut buf = [0u8; 1024];
///         let mut data: Vec<u8> = vec![];
///
///         // loop through each of the available chunks clone them to local
///         // variable and append to a vec.
///         loop {
///             let len = ctx.clone_next_chunk(connection, &mut buf)?;
///
///             if len == 0 {
///                 break;
///             }
///
///             data.extend(&buf[0..len]);
///         }
///
///         // convert returned data to a utf8 string
///         let dstring = from_utf8(&data)?;
///         info!("data[{}]='{}'", connection.id(), dstring)?;
///
///         // get a write handle
///         let mut wh = connection.write_handle()?;
///
///         // echo
///         wh.write(dstring.as_bytes())?;
///
///         // clear all chunks from this connection. Note that partial
///         // clearing is possible with the ctx.clear_through function
///         // or no data can be cleared at all in which case it can
///         // be accessed on a subsequent request. When the connection
///         // is closed, all data is cleared automatically.
///         ctx.clear_all(connection)?;
///
///         Ok(())
///     })?;
///
///     // no other handlers are necessary
///
///     evh.start()?;
///
///     Ok(())
/// }
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
