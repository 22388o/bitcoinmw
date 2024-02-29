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

use bmw_deps::colored::Colorize;
use bmw_deps::itertools::Itertools;
use bmw_deps::num_format::{Locale, ToFormattedString};
use bmw_deps::rand::random;
use bmw_err::*;
use bmw_evh::*;
use bmw_log::LogConfigOption::*;
use bmw_log::*;
use bmw_util::*;
use clap::{load_yaml, App, ArgMatches};
use std::collections::HashMap;
use std::net::TcpStream;
use std::process::exit;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
#[cfg(windows)]
use std::os::windows::io::IntoRawSocket;

const SPACER: &str =
	"----------------------------------------------------------------------------------------------------";

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Clone)]
struct GlobalStats {
	messages: usize,
	lat_sum: u128,
	histo_data: Vec<u64>,
}

impl GlobalStats {
	fn new() -> Self {
		Self {
			messages: 0,
			lat_sum: 0,
			histo_data: vec![],
		}
	}
}

fn print_configs(configs: HashMap<String, String>) -> Result<(), Error> {
	let mut max_len = 0;
	for (k, _v) in &configs {
		if k.len() > max_len {
			max_len = k.len();
		}
	}

	info_plain!(SPACER)?;
	for (k, v) in configs.iter().sorted() {
		let mut spaces = ":".to_string();
		for _ in k.len()..max_len {
			spaces = format!("{} ", spaces).to_string();
		}
		info!("{} '{}'", format!("{}{}", k.yellow(), spaces), v)?;
	}
	info_plain!(SPACER)?;
	Ok(())
}

fn run_eventhandler(
	args: ArgMatches,
	start: Instant,
	ready_notifier: Option<SyncSender<u8>>,
) -> Result<(), Error> {
	let threads: usize = match args.is_present("threads") {
		true => args.value_of("threads").unwrap().parse()?,
		false => 1,
	};
	let port = match args.is_present("port") {
		true => args.value_of("port").unwrap().parse()?,
		false => 8081,
	};
	let read_slab_count = match args.is_present("read_slab_count") {
		true => args.value_of("read_slab_count").unwrap().parse()?,
		false => 20,
	};
	let reuse_port = args.is_present("reuse_port");

	let tls = args.is_present("tls");

	let max_handles_per_thread = match args.is_present("max_handles_per_thread") {
		true => args.value_of("max_handles_per_thread").unwrap().parse()?,
		false => 300,
	};

	let host = match args.is_present("host") {
		true => args.value_of("host").unwrap(),
		false => "127.0.0.1",
	};

	let debug = args.is_present("debug");

	let mut configs = HashMap::new();
	configs.insert("debug".to_string(), debug.to_string());
	configs.insert("port".to_string(), port.to_string());
	configs.insert("host".to_string(), host.to_string());
	configs.insert("reuse_port".to_string(), reuse_port.to_string());
	configs.insert(
		"max_handles_per_thread".to_string(),
		max_handles_per_thread.to_formatted_string(&Locale::en),
	);
	configs.insert(
		"threads".to_string(),
		threads.to_formatted_string(&Locale::en),
	);

	configs.insert(
		"read_slab_count".to_string(),
		read_slab_count.to_formatted_string(&Locale::en),
	);
	configs.insert("tls".to_string(), tls.to_string());
	print_configs(configs)?;

	set_log_option!(LogConfigOption::Level(true))?;

	let addr = &format!("{}:{}", host, port)[..];
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
		let tid = conn_data.tid();
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
			info!(
				"[tid={}] Wrote back {} bytes on connection {}",
				tid, byte_count, id
			)?;
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
		tls_config: match tls {
			true => Some(TlsServerConfig {
				certificates_file: "./resources/cert.pem".to_string(),
				private_key_file: "./resources/key.pem".to_string(),
			}),
			false => None,
		},
		handles,
		is_reuse_port: reuse_port,
	};
	evh.add_server(sc, Box::new(""))?;

	info!(
		"{}",
		format!("Server started in {} ms.", start.elapsed().as_millis()).cyan()
	)?;

	match ready_notifier {
		Some(ready_notifier) => ready_notifier.send(1)?,
		None => {}
	}

	std::thread::park();

	Ok(())
}

fn run_client(args: ArgMatches, start: Instant) -> Result<(), Error> {
	let debug = match args.is_present("debug") {
		true => true,
		_ => false,
	};

	let histo_delta_micros = match args.is_present("histo_delta_micros") {
		true => args.value_of("histo_delta_micros").unwrap().parse()?,
		false => 10,
	};

	let tls = args.is_present("tls");

	let port = match args.is_present("port") {
		true => args.value_of("port").unwrap().parse()?,
		false => 8081,
	};
	let itt: usize = match args.is_present("itt") {
		true => args.value_of("itt").unwrap().parse()?,
		false => 1,
	};
	let count: usize = match args.is_present("count") {
		true => args.value_of("count").unwrap().parse()?,
		false => 1,
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

	let read_slab_count = match args.is_present("read_slab_count") {
		true => args.value_of("read_slab_count").unwrap().parse()?,
		false => 20,
	};

	let sleep_time = match args.is_present("sleep") {
		true => args.value_of("sleep").unwrap().parse()?,
		false => 0,
	};

	let host = match args.is_present("host") {
		true => args.value_of("host").unwrap(),
		false => "127.0.0.1",
	};

	let histo = args.is_present("histo");

	let mut configs = HashMap::new();
	configs.insert("host".to_string(), host.to_string());
	configs.insert("count".to_string(), count.to_formatted_string(&Locale::en));
	configs.insert(
		"clients".to_string(),
		clients.to_formatted_string(&Locale::en),
	);
	configs.insert("min".to_string(), min.to_formatted_string(&Locale::en));
	configs.insert("max".to_string(), max.to_formatted_string(&Locale::en));
	configs.insert(
		"sleep".to_string(),
		sleep_time.to_formatted_string(&Locale::en),
	);
	configs.insert(
		"histo_delta_micros".to_string(),
		histo_delta_micros.to_formatted_string(&Locale::en),
	);
	configs.insert(
		"iterations".to_string(),
		itt.to_formatted_string(&Locale::en),
	);
	configs.insert("debug".to_string(), debug.to_string());
	configs.insert("port".to_string(), port.to_string());
	configs.insert(
		"reconns".to_string(),
		reconns.to_formatted_string(&Locale::en),
	);
	configs.insert(
		"max_handles_per_thread".to_string(),
		max_handles_per_thread.to_formatted_string(&Locale::en),
	);
	configs.insert(
		"threads".to_string(),
		threads.to_formatted_string(&Locale::en),
	);

	configs.insert(
		"read_slab_count".to_string(),
		read_slab_count.to_formatted_string(&Locale::en),
	);
	configs.insert("tls".to_string(), tls.to_string());
	configs.insert("histo".to_string(), histo.to_string());
	print_configs(configs)?;

	let addr = format!("{}:{}", host, port);
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
				histo_delta_micros,
				tls,
			);
			match res {
				Ok(_) => {}
				Err(e) => error!("run_thread generated error: {}", e)?,
			}
			Ok(())
		})?);
	}

	set_log_option!(LogConfigOption::Level(true))?;

	info!(
		"{}",
		format!("Client started in {} ms.", start.elapsed().as_millis()).cyan()
	)?;

	let state_clone = state.clone();
	let mut messages_last = 0;
	let mut lat_sum_last = 0;
	spawn(move || -> Result<(), Error> {
		loop {
			sleep(Duration::from_millis(3000));
			info_plain!(SPACER)?;

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
				"{} of {} messages received. [{:.2}% complete]",
				messages.to_formatted_string(&Locale::en),
				(clients * count * itt * threads * reconns).to_formatted_string(&Locale::en),
				((100.0 * messages as f64) / (clients * count * itt * threads * reconns) as f64)
			)?;

			let incremental_messages = messages.saturating_sub(messages_last);
			let incremental_qps = (incremental_messages as f64 / 3_000_000_000.0) * 1_000_000_000.0;
			let incremental_latsum = lat_sum.saturating_sub(lat_sum_last);

			let avg_incremental_lat = if incremental_messages > 0 {
				incremental_latsum / incremental_messages as u128
			} else {
				0
			};

			info!(
				"incremental_messages=[{}],elapsed_time=[3.00s]",
				incremental_messages.to_formatted_string(&Locale::en),
			)?;
			info!(
				"incremental_mps=[{}],incremental_avg_latency=[{:.2}µs]",
				(incremental_qps.round() as u64).to_formatted_string(&Locale::en),
				((avg_incremental_lat as f64) / 1_000.0)
			)?;

			info!(
				"total_messages=[{}],elapsed_time=[{:.2}s]",
				messages.to_formatted_string(&Locale::en),
				seconds,
			)?;
			info!(
				"total_mps=[{}],total_avg_latency=[{:.2}µs]",
				(qps.round() as u64).to_formatted_string(&Locale::en),
				((avg_lat as f64) / 1_000.0)
			)?;

			messages_last = messages;
			lat_sum_last = lat_sum;
		}
	});

	for i in 0..completions.len() {
		block_on!(completions[i]);
	}

	info_plain!(SPACER)?;
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

	let avg_lat = if messages > 0 {
		lat_sum / messages as u128
	} else {
		0
	};
	let seconds = (elapsed_nanos as f64) / 1_000_000_000.0;

	if messages == count * itt * threads * clients * reconns {
		info!("{}", "Perf test completed successfully!".cyan())?;
	} else {
		error!(
			"{}",
			format!(
				"Perf test failed! Expected {} messages. Received {}.",
				count * itt * threads * clients * reconns,
				messages
			)
			.red()
		)?;
	}

	info!(
		"total_messages=[{}],elapsed_time=[{:.2}s]",
		messages.to_formatted_string(&Locale::en),
		seconds,
	)?;

	info!(
		"messages_per_second=[{}],average_latency=[{:.2}µs]",
		(qps.round() as u64).to_formatted_string(&Locale::en),
		((avg_lat as f64) / 1_000.0),
	)?;

	if messages != itt * threads * clients * reconns * count {
		exit(-1);
	}

	let state = state.rlock()?;
	let guard = state.guard();
	if histo {
		print_histo((**guard).histo_data.clone(), histo_delta_micros)?;
	}

	Ok(())
}

fn print_histo(data: Vec<u64>, delta_micros: usize) -> Result<(), Error> {
	let mut sum = 0;
	let len = data.len();
	for i in 0..len {
		sum += data[i];
	}
	if sum == 0 {
		sum += 1;
	}
	info_plain!("{}", SPACER)?;
	info_plain!("Latency Histogram")?;
	info_plain!("{}", SPACER)?;

	let first_digit_len = format!("{}", len * (delta_micros - 1)).len();
	let last_digit_len = format!("{}", len * delta_micros).len();

	let mut start = 0;
	for i in 0..len {
		if data[i] > 0 {
			let percent = (data[i] as f64 / sum as f64) * 100 as f64;
			let percent_rounded = percent.round() as usize;
			let mut bar = "".to_string();
			for _ in 0..percent_rounded {
				bar = format!("{}{}", bar, "=");
			}
			bar = format!("{}>", bar);

			let mut start_str = format!("{}µs", start);
			for _ in start_str.len()..(first_digit_len + 3) {
				start_str = format!("{}{}", start_str, " ");
			}
			let mut end_str = format!("{}µs", start + delta_micros);
			for _ in end_str.len()..(last_digit_len + 3) {
				end_str = format!("{}{}", end_str, " ");
			}
			info_plain!(
				"{}{} {}",
				format!("[{} - {}]", start_str, end_str),
				bar.cyan(),
				format!(
					"{} ({:.2}%)",
					data[i].to_formatted_string(&Locale::en),
					percent
				)
			)?;
		}
		start += delta_micros;
	}
	info_plain!("{}", SPACER)?;

	/*
	info_plain!("====================>")?;
	info_plain!("==============================>")?;
	info_plain!("========================================>")?;
	info_plain!("==================================================>")?;
	info_plain!("============================================================>")?;
	info_plain!("======================================================================>")?;
	info_plain!(
		"================================================================================>"
	)?;
	info_plain!("==========================================================================================>")?;
	info_plain!("====================================================================================================>")?;
		*/

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
	histo_delta_micros: usize,
	tls: bool,
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
				update_histo_vec(
					&mut (**guard).histo_data,
					try_into!(diff / 1000)?,
					try_into!(histo_delta_micros)?,
				)?;
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
				tls_config: match tls {
					true => Some(TlsClientConfig {
						sni_host: "localhost".to_string(),
						trusted_cert_full_chain_file: Some("./resources/cert.pem".to_string()),
					}),
					false => None,
				},
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

fn update_histo_vec(
	histo_data: &mut Vec<u64>,
	diff: u64,
	histo_delta_micros: u64,
) -> Result<(), Error> {
	let bucket = try_into!(diff / histo_delta_micros)?;
	if histo_data.len() <= bucket {
		histo_data.resize(bucket + 1, 0);
	}
	histo_data[bucket] += 1;

	Ok(())
}

fn main() -> Result<(), Error> {
    z
	let start = Instant::now();

	global_slab_allocator!()?;
	log_init!(LogConfig {
		show_bt: ShowBt(false),
		line_num: LineNum(false),
		level: Level(false),
		..Default::default()
	})?;

	let yml = load_yaml!("evh_perf.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	let client = args.is_present("client");
	let eventhandler = args.is_present("eventhandler");

	if client && eventhandler {
		info!(
			"{}",
			format!("evh_perf Client/{}", built_info::PKG_VERSION).green()
		)?;

		let (tx, rx) = sync_channel(1);
		spawn(move || -> Result<(), Error> {
			let yml = load_yaml!("evh_perf.yml");
			let args = App::from_yaml(yml)
				.version(built_info::PKG_VERSION)
				.get_matches();

			run_eventhandler(args, start, Some(tx))?;
			Ok(())
		});

		rx.recv()?;
		let start = Instant::now();
		run_client(args, start)?;
	} else if client && args.is_present("reuse_port") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--reuse_port must only be used with the -e option")?;
		exit(-1);
	} else if client {
		info!(
			"{}",
			format!("evh_perf Client/{}", built_info::PKG_VERSION).green()
		)?;
		run_client(args, start)?;
	} else if eventhandler && args.is_present("histo_delta_micros") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--histo_delta_micros must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("histo") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--histo must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("reconns") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--reconns must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("clients") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--clients must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("count") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--count must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("itt") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--itt must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("max") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--min must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("min") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--min must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler && args.is_present("sleep") {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("--sleep must only be used with the -c option")?;
		exit(-1);
	} else if eventhandler {
		info!(
			"{}",
			format!("evh_perf EventHandler/{}", built_info::PKG_VERSION).green()
		)?;
		run_eventhandler(args, start, None)?;
	} else {
		set_log_option!(LogConfigOption::Level(true))?;
		error!("-c or -e option must be specified. run evh_perf --help for full details.")?;
		exit(-1);
	}

	Ok(())
}
