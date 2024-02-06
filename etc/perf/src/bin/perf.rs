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

use bmw_deps::rand::random;
use bmw_err::*;
use bmw_evh::*;
use bmw_log::LogConfigOption::*;
use bmw_log::*;
use bmw_util::*;
use clap::{load_yaml, App, ArgMatches};
use num_format::{Locale, ToFormattedString};
use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::mpsc::sync_channel;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
#[cfg(windows)]
use std::os::windows::io::IntoRawSocket;

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Clone)]
struct GlobalStats {
	messages: usize,
	lat_sum: u128,
}

impl GlobalStats {
	fn new() -> Self {
		Self {
			messages: 0,
			lat_sum: 0,
		}
	}
}

fn run_eventhandler(args: ArgMatches) -> Result<(), Error> {
	let threads: usize = match args.is_present("threads") {
		true => args.value_of("threads").unwrap().parse()?,
		false => 1,
	};
	let port = match args.is_present("port") {
		true => args.value_of("port").unwrap().parse()?,
		false => 8081,
	};
	let read_slab_count = match args.is_present("slabs") {
		true => args.value_of("slabs").unwrap().parse()?,
		false => 20,
	};
	let reuse_port = args.is_present("reuse_port");

	let max_handles_per_thread = match args.is_present("max_handles_per_thread") {
		true => args.value_of("max_handles_per_thread").unwrap().parse()?,
		false => 300,
	};

	let debug = args.is_present("debug");

	info!("Using port: {},reuse_port={}", port, reuse_port)?;
	let addr = &format!("127.0.0.1:{}", port)[..];
	let config = EventHandlerConfig {
		threads,
		housekeeping_frequency_millis: 10_000,
		read_slab_count,
		max_handles_per_thread,
		..Default::default()
	};
	let mut evh = bmw_evh::Builder::build_evh(config)?;

	evh.set_on_read(move |conn_data, _thread_context, _| {
		let first_slab = conn_data.first_slab();
		let last_slab = conn_data.last_slab();
		let slab_offset = conn_data.slab_offset();
		let id = conn_data.get_connection_id();
		let mut wh = conn_data.write_handle();
		let byte_count = conn_data.borrow_slab_allocator(move |sa| {
			let mut slab_id = first_slab;
			let mut byte_count = 0;
			loop {
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = if slab_id != last_slab {
					READ_SLAB_DATA_SIZE
				} else {
					slab_offset as usize
				};

				wh.write(&slab_bytes[0..offset as usize])?;
				byte_count += offset;

				if slab_id == last_slab {
					break;
				}
				slab_id = u32::from_be_bytes(try_into!(
					slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
				)?);
			}
			Ok(byte_count)
		})?;

		conn_data.clear_through(last_slab)?;
		if debug {
			info!("Wrote back {} bytes on connection {}", byte_count, id)?;
		}

		Ok(())
	})?;

	evh.set_on_accept(move |conn_data, _thread_context| {
		debug!(
			"accept a connection handle = {}, tid={}",
			conn_data.get_handle(),
			conn_data.tid()
		)?;
		Ok(())
	})?;
	evh.set_on_close(move |conn_data, _thread_context| {
		if debug {
			info!(
				"on close: {}/{}",
				conn_data.get_handle(),
				conn_data.get_connection_id()
			)?;
		}
		Ok(())
	})?;
	evh.set_on_panic(move |_, _| Ok(()))?;
	evh.set_housekeeper(move |_thread_context| Ok(()))?;

	evh.start()?;
	let handles = create_listeners(threads, addr, 10_000, reuse_port)?;
	debug!("handles.size={},handles={:?}", handles.size(), handles)?;
	let sc = ServerConnection {
		tls_config: None,
		handles,
		is_reuse_port: reuse_port,
	};
	evh.add_server(sc, Box::new(""))?;

	std::thread::park();

	Ok(())
}

fn run_client(args: ArgMatches) -> Result<(), Error> {
	let start = Instant::now();

	let debug = match args.is_present("debug") {
		true => true,
		_ => false,
	};

	let port = match args.is_present("port") {
		true => args.value_of("port").unwrap().parse()?,
		false => 8081,
	};
	let itt: usize = match args.is_present("itt") {
		true => args.value_of("itt").unwrap().parse()?,
		false => 1_000,
	};
	let count: usize = match args.is_present("count") {
		true => args.value_of("count").unwrap().parse()?,
		false => 10,
	};
	let clients: usize = match args.is_present("clients") {
		true => args.value_of("clients").unwrap().parse()?,
		false => 1,
	};
	let threads: usize = match args.is_present("threads") {
		true => args.value_of("threads").unwrap().parse()?,
		false => 1,
	};

	let reconns: usize = match args.is_present("reconns") {
		true => args.value_of("reconns").unwrap().parse()?,
		false => 1,
	};

	let max_handles_per_thread = match args.is_present("max_handles_per_thread") {
		true => args.value_of("max_handles_per_thread").unwrap().parse()?,
		false => 300,
	};

	let min = match args.is_present("min") {
		true => args.value_of("min").unwrap().parse()?,
		false => 3,
	};

	let max = match args.is_present("max") {
		true => args.value_of("max").unwrap().parse()?,
		false => 10,
	};

	let read_slab_count = match args.is_present("slabs") {
		true => args.value_of("slabs").unwrap().parse()?,
		false => 20,
	};

	let sleep_time = match args.is_present("sleep") {
		true => args.value_of("sleep").unwrap().parse()?,
		false => 0,
	};

	info!(
		"iterations={},count={},clients={},threads={},reconns={},port={}",
		itt.to_formatted_string(&Locale::en),
		count.to_formatted_string(&Locale::en),
		clients.to_formatted_string(&Locale::en),
		threads,
		reconns.to_formatted_string(&Locale::en),
		port
	)?;

	let addr = format!("127.0.0.1:{}", port);
	let config = EventHandlerConfig {
		threads: 1,
		housekeeping_frequency_millis: 10_000,
		read_slab_count,
		max_handles_per_thread,
		..Default::default()
	};

	let mut pool = thread_pool!(MinSize(threads), MaxSize(threads))?;
	pool.set_on_panic(move |_, _| Ok(()))?;
	let mut completions = vec![];
	let state = lock_box!(GlobalStats::new())?;
	for _ in 0..threads {
		let addr = addr.clone();
		let config = config.clone();
		let state_clone = state.clone();
		completions.push(execute!(pool, {
			let res = run_thread(
				&config,
				addr,
				itt,
				count,
				clients,
				state_clone,
				debug,
				start,
				reconns,
				max,
				min,
				sleep_time,
			);
			match res {
				Ok(_) => {}
				Err(e) => error!("run_thread generated error: {}", e)?,
			}
			Ok(())
		})?);
	}

	let state_clone = state.clone();
	spawn(move || -> Result<(), Error> {
		loop {
			sleep(Duration::from_millis(3000));
			info_plain!("----------------------------------------------------------------------------------------------------------------------------------------------------")?;

			let elapsed = start.elapsed();
			let elapsed_nanos = elapsed.as_nanos() as f64;

			let (messages, lat_sum) = {
				let state = state_clone.rlock()?;
				let guard = state.guard();
				let messages = (**guard).messages;
				let lat_sum = (**guard).lat_sum;

				(messages, lat_sum)
			};
			let qps = (messages as f64 / elapsed_nanos) * 1_000_000_000.0;

			let avg_lat = if messages > 0 {
				lat_sum / messages as u128
			} else {
				0
			};
			let seconds = (elapsed_nanos as f64) / 1_000_000_000.0;

			info!(
				"Summary for {} of {} messages. [{:.2}% complete]",
				messages.to_formatted_string(&Locale::en),
				(clients * count * itt * threads * reconns).to_formatted_string(&Locale::en),
				((100.0 * messages as f64) / (clients * count * itt * threads * reconns) as f64)
			)?;

			info!(
                            "total_messages=[{}],elapsed_time=[{:.2}s],requests_per_second=[{}],average_latency=[{:.2}µs]",
                            messages.to_formatted_string(&Locale::en),
                            seconds,
                            (qps.round() as u64).to_formatted_string(&Locale::en),
                            ((avg_lat as f64) / 1_000.0)
                        )?;
		}
	});

	for i in 0..completions.len() {
		block_on!(completions[i]);
	}

	info_plain!("----------------------------------------------------------------------------------------------------------------------------------------------------")?;

	let elapsed = start.elapsed();
	let elapsed_nanos = elapsed.as_nanos() as f64;

	let (messages, lat_sum) = {
		let state = state.rlock()?;
		let guard = state.guard();
		let messages = (**guard).messages;
		let lat_sum = (**guard).lat_sum;

		(messages, lat_sum)
	};
	let qps = (messages as f64 / elapsed_nanos) * 1_000_000_000.0;

	let avg_lat = lat_sum / messages as u128;
	let seconds = (elapsed_nanos as f64) / 1_000_000_000.0;

	info!("Perf test complete!")?;

	info!(
		"total_messages=[{}],elapsed_time=[{:.2}s],requests_per_second=[{}],average_latency=[{:.2}µs]",
		messages.to_formatted_string(&Locale::en),
                seconds,
		(qps.round() as u64).to_formatted_string(&Locale::en),
		((avg_lat as f64) / 1_000.0),
	)?;

	Ok(())
}

fn run_thread(
	config: &EventHandlerConfig,
	addr: String,
	itt: usize,
	count: usize,
	clients: usize,
	state: Box<dyn LockBox<GlobalStats>>,
	debug: bool,
	start: Instant,
	reconns: usize,
	max: usize,
	min: usize,
	sleep_time: u64,
) -> Result<(), Error> {
	let mut dictionary = vec![];
	for i in 0..max {
		dictionary.push(('a' as usize + (i % 26)) as u8);
	}
	let state_clone = state.clone();
	let mut evh = bmw_evh::Builder::build_evh(config.clone())?;

	let (tx, rx) = sync_channel(1);
	let sender = lock_box!(tx)?;

	let map: HashMap<u128, Vec<u8>> = HashMap::new();
	let partial_data = lock_box!(map)?;
	let mut recv_count = lock_box!(0usize)?;
	let mut recv_count_clone = recv_count.clone();

	evh.set_on_read(move |conn_data, _thread_context, _| {
		if debug {
			info!("evh on read")?;
		}
		let id = conn_data.get_connection_id();
		let first_slab = conn_data.first_slab();
		let slab_offset = conn_data.slab_offset();
		let last_slab = conn_data.last_slab();
		let mut sender = sender.clone();
		let mut state_clone = state_clone.clone();
		let partial_data = partial_data.clone();
		let mut partial_data_clone = partial_data.clone();
		let res = conn_data.borrow_slab_allocator(move |sa| {
			let mut slab_id = first_slab;
			let mut ret: Vec<u8> = vec![];
			let mut data_extended = false;

			let partial_data = partial_data.rlock()?;
			let guard = partial_data.guard();
			match (**guard).get(&id) {
				Some(data) => {
					if debug {
						info!("extend data with {:?}", data)?;
					}
					ret.extend(data);
					data_extended = true;
				}
				_ => {}
			}

			loop {
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = if slab_id == last_slab {
					slab_offset as usize
				} else {
					READ_SLAB_DATA_SIZE
				};

				debug!("read bytes[{}] = {:?}", offset, &slab_bytes[0..offset])?;
				ret.extend(&slab_bytes[0..offset]);

				if debug && data_extended {
					info!("slab extend data with {:?}", &slab_bytes[0..offset])?;
				}

				if slab_id == last_slab {
					break;
				} else {
					slab_id = u32::from_be_bytes(try_into!(
						slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
					)?);
				}
			}
			Ok(ret)
		})?;

		conn_data.clear_through(last_slab)?;

		if debug {
			info!("evh read {} bytes", res.len())?;
		}

		let res_len = res.len();

		let mut itt = 0;
		let mut inserted = false;
		loop {
			if itt == res_len {
				if debug {
					info!("clean break")?;
				}
				break;
			}

			if itt + 28 > res_len {
				let mut partial_data = partial_data_clone.wlock()?;
				let guard = partial_data.guard();
				(**guard).insert(id, (&res[itt..]).to_vec());
				inserted = true;
				if debug {
					info!(
						"unclean break, itt={},res_len={},append={:?}",
						itt,
						res_len,
						&res[itt..]
					)?;
				}
				break;
			}

			let len = slice_to_usize(&res[itt..itt + 4])?;

			if len + itt + 28 > res_len {
				let mut partial_data = partial_data_clone.wlock()?;
				let guard = partial_data.guard();
				(**guard).insert(id, (&res[itt..]).to_vec());
				inserted = true;
				if debug {
					info!(
						"unclean break, itt={},res_len={},append={:?}",
						itt,
						res_len,
						&res[itt..]
					)?;
				}
				break;
			}

			let id_read = slice_to_u128(&res[itt + 4..itt + 20])?;
			let nanos = slice_to_usize(&res[itt + 20..itt + 28])?;
			let elapsed: usize = try_into!(start.elapsed().as_nanos())?;
			let diff = elapsed.saturating_sub(nanos);

			if debug {
				info!("on_read len={},id={},lat_nanos={}", res_len, id, diff)?;
			}

			if id != id_read {
				let mut state = state_clone.wlock()?;
				let guard = state.guard();
				info!(
					"messages={},lat_sum={}",
					(**guard).messages,
					(**guard).lat_sum
				)?;
				break;
			}
			assert_eq!(id, id_read);

			for i in 29..(28 + len) {
				if res[itt + (i - 1)] == 'z' as u8 {
					assert_eq!(res[itt + i], 'a' as u8);
				} else {
					assert_eq!(res[itt + (i - 1)], res[itt + i] - 1);
				}
			}

			{
				let mut state = state_clone.wlock()?;
				let guard = state.guard();
				(**guard).messages += 1;
				(**guard).lat_sum += diff as u128;
			}

			{
				let mut recv_count = recv_count_clone.wlock()?;
				let guard = recv_count.guard();
				**guard += 1;

				if **guard == clients * count {
					let mut sender = sender.wlock()?;
					let guard = sender.guard();
					(**guard).send(1)?;
				}
			}

			itt += len + 28;
		}

		if !inserted {
			let mut partial_data = partial_data_clone.wlock()?;
			let guard = partial_data.guard();
			(**guard).remove(&id);
		}

		Ok(())
	})?;

	evh.set_on_accept(move |conn_data, _thread_context| {
		debug!(
			"accept a connection handle = {}, tid={}",
			conn_data.get_handle(),
			conn_data.tid()
		)?;
		Ok(())
	})?;
	evh.set_on_close(move |conn_data, _thread_context| {
		debug!(
			"on close: {}/{}",
			conn_data.get_handle(),
			conn_data.get_connection_id()
		)?;
		Ok(())
	})?;
	evh.set_on_panic(move |_, _| Ok(()))?;
	evh.set_housekeeper(move |_thread_context| Ok(()))?;
	evh.start()?;

	for _ in 0..reconns {
		let mut whs = vec![];
		for _ in 0..clients {
			let connection = TcpStream::connect(addr.clone())?;
			connection.set_nonblocking(true)?;
			#[cfg(unix)]
			let connection_handle = connection.into_raw_fd();
			#[cfg(windows)]
			let connection_handle = connection.into_raw_socket().try_into()?;

			let client = ClientConnection {
				handle: connection_handle,
				tls_config: None,
			};
			let wh = evh.add_client(client, Box::new(""))?;
			whs.push(wh);
		}

		for _ in 0..itt {
			{
				let mut recv_count = recv_count.wlock()?;
				let guard = recv_count.guard();
				(**guard) = 0;
			}

			for _ in 0..count {
				for wh in &mut whs {
					let mut buf = vec![];
					let rfloat = random::<f64>();
					let rfloat = rfloat - rfloat.floor();
					let len = min + (rfloat * (max.saturating_sub(min)) as f64).round() as usize;
					buf.resize(len + 28, 0);
					u32_to_slice(len as u32, &mut buf[0..4])?; // length of data
					u128_to_slice(wh.id(), &mut buf[4..20])?; // connection id
					usize_to_slice(try_into!(start.elapsed().as_nanos())?, &mut buf[20..28])?; // start time for request
					buf[28..(28 + len)].copy_from_slice(&dictionary[0..len]); // data
					wh.write(&buf)?;
				}
			}

			rx.recv()?;
		}

		for mut wh in whs {
			wh.close()?;
		}

		if sleep_time > 0 {
			info!("sleeping for {} ms.", sleep_time)?;
			sleep(Duration::from_millis(sleep_time));
		}
	}

	Ok(())
}

fn main() -> Result<(), Error> {
	global_slab_allocator!()?;
	log_init!(LogConfig {
		show_bt: ShowBt(false),
		line_num: LineNum(false),
		..Default::default()
	})?;

	let yml = load_yaml!("perf.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let client = args.is_present("client");
	let eventhandler = args.is_present("eventhandler");

	if client {
		info!("Starting perf client")?;
		run_client(args)?;
	} else if eventhandler {
		info!("Starting eventhandler")?;
		run_eventhandler(args)?;
	}

	Ok(())
}
