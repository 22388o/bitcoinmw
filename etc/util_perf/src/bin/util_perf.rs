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

use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use clap::{load_yaml, App};
use num_format::{Locale, ToFormattedString};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::time::Instant;

info!();

// include build information
pub mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

struct MonAllocator;

static mut MEM_ALLOCATED: usize = 0;
static mut MEM_DEALLOCATED: usize = 0;
static mut LAST_MEMUSED: usize = 0;
static mut ALLOC_COUNT: usize = 0;
static mut DEALLOC_COUNT: usize = 0;

unsafe impl GlobalAlloc for MonAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		MEM_ALLOCATED += layout.size();
		ALLOC_COUNT += 1;
		System.alloc(layout)
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		MEM_DEALLOCATED += layout.size();
		DEALLOC_COUNT += 1;
		System.dealloc(ptr, layout)
	}
}

#[global_allocator]
static GLOBAL: MonAllocator = MonAllocator;

fn show_mem(start: Instant, msg: &str) -> Result<(), Error> {
	let elapsed = start.elapsed();
	let delta = unsafe { MEM_ALLOCATED as i64 - MEM_DEALLOCATED as i64 };
	let alloc = unsafe { MEM_ALLOCATED };
	let dealloc = unsafe { MEM_DEALLOCATED };
	let alloc_count = unsafe { ALLOC_COUNT };
	let dealloc_count = unsafe { DEALLOC_COUNT };

	info!(
		"{}: alloc: {}, dealloc: {}, alloc_qty: {}, dealloc_qty: {}, delta: {}, elapsed: {:?}",
		msg,
		alloc.to_formatted_string(&Locale::en),
		dealloc.to_formatted_string(&Locale::en),
		alloc_count.to_formatted_string(&Locale::en),
		dealloc_count.to_formatted_string(&Locale::en),
		delta.to_formatted_string(&Locale::en),
		elapsed,
	)?;
	Ok(())
}

fn reset_stats() -> Result<(), Error> {
	unsafe { LAST_MEMUSED = 0 };
	unsafe { MEM_ALLOCATED = 0 };
	unsafe { MEM_DEALLOCATED = 0 };
	unsafe { ALLOC_COUNT = 0 };
	unsafe { DEALLOC_COUNT = 0 };
	Ok(())
}

fn do_arraylist() -> Result<(), Error> {
	info!("testing arraylist")?;
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut arraylist = array_list!(10_000, &0)?;
		show_mem(start, "arraylist init")?;
		reset_stats()?;
		start = Instant::now();
		for i in 0..10_000 {
			arraylist.push(i as u32)?;
		}
		show_mem(start, "arraylist insert")?;
		reset_stats()?;
		start = Instant::now();
		let mut count = 0;
		for _x in arraylist.iter() {
			count += 1;
		}
		assert_eq!(count, 10_000);
		show_mem(start, "arraylist iter")?;
		start = Instant::now();
	}
	show_mem(start, "arraylist drop")?;
	Ok(())
}

fn do_array() -> Result<(), Error> {
	info!("Testing array")?;
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut array = array!(10_000, &0)?;
		show_mem(start, "array init")?;
		reset_stats()?;
		start = Instant::now();
		for i in 0..10_000 {
			array[i] = i as u32;
		}

		show_mem(start, "array insert")?;
		reset_stats()?;
		start = Instant::now();

		let mut count = 0;
		for _x in array.iter() {
			count += 1;
		}
		assert_eq!(count, 10_000);
		show_mem(start, "array iter")?;
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "array drop")?;
	Ok(())
}

fn do_vec_string() -> Result<(), Error> {
	info!("Testing vec string")?;
	let s = "0123456789a".to_string();
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut vec = vec![];
		vec.resize(10_000, "".to_string());
		show_mem(start, "vec init")?;
		reset_stats()?;
		start = Instant::now();
		for i in 0..10_000 {
			vec[i] = s.clone();
		}

		show_mem(start, "vec insert")?;
		reset_stats()?;
		start = Instant::now();

		let mut count = 0;
		for _x in vec.iter() {
			count += 1;
		}
		assert_eq!(count, 10_000);
		show_mem(start, "vec iter")?;
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "vec drop")?;
	Ok(())
}

fn do_array_string() -> Result<(), Error> {
	info!("Testing array string")?;
	let s = "0123456789a".to_string();
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut array = array!(10_000, &"".to_string())?;
		show_mem(start, "array init")?;
		reset_stats()?;
		start = Instant::now();
		for i in 0..10_000 {
			array[i] = s.clone();
		}

		show_mem(start, "array insert")?;
		reset_stats()?;
		start = Instant::now();

		let mut count = 0;
		for _x in array.iter() {
			count += 1;
		}
		assert_eq!(count, 10_000);
		show_mem(start, "array iter")?;
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "array drop")?;
	Ok(())
}

fn do_hashtable() -> Result<(), Error> {
	info!("Testing hashtable")?;
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut hashtable = hashtable!(
			GlobalSlabAllocator(false),
			MaxEntries(10_000),
			SlabSize(100),
			SlabCount(100_000)
		)?;

		show_mem(start, "hashtable init")?;
		reset_stats()?;
		start = Instant::now();

		for i in 0..10_000 {
			hashtable.insert(&(i as u32), &(i as u32))?;
		}
		show_mem(start, "hashtable insert")?;
		reset_stats()?;
		start = Instant::now();

		for i in 0..10_000 {
			assert!(hashtable.get(&i)?.is_some());
		}
		show_mem(start, "hashtable get")?;
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "hashtable drop")?;

	Ok(())
}

fn do_hashmap() -> Result<(), Error> {
	info!("Testing hashmap")?;
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut hashmap = HashMap::new();
		show_mem(start, "hashmap init")?;
		reset_stats()?;
		start = Instant::now();
		let mut v = vec![];
		for i in 0..10_000 {
			v.push(i as u32);
		}

		for i in 0..10_000 {
			hashmap.insert(&v[i], &v[i]);
		}

		show_mem(start, "hashmap insert")?;
		reset_stats()?;
		start = Instant::now();

		for i in 0..10_000 {
			assert!(hashmap.get(&i).is_some());
		}
		show_mem(start, "hashmap get")?;
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "hashmap drop")?;
	Ok(())
}

fn do_vec() -> Result<(), Error> {
	info!("testing vec")?;
	let mut start;
	{
		reset_stats()?;
		start = Instant::now();
		let mut vec = vec![];
		show_mem(start, "vec init")?;
		reset_stats()?;
		start = Instant::now();
		for i in 0..10_000 {
			vec.push(i as u32);
		}
		show_mem(start, "vec insert")?;
		reset_stats()?;
		start = Instant::now();
		let mut count = 0;
		for _x in &vec {
			count += 1;
		}
		show_mem(start, "vec iter")?;
		assert_eq!(count, 10_000);
		reset_stats()?;
		start = Instant::now();
	}
	show_mem(start, "vec drop")?;
	Ok(())
}

fn main() -> Result<(), Error> {
	global_slab_allocator!()?;
	log_init!(DisplayLogLevel(false), DisplayLineNum(false),)?;

	let yml = load_yaml!("util_perf.yml");
	let args = App::from_yaml(yml)
		.version(built_info::PKG_VERSION)
		.get_matches();

	info!("util_perf")?;

	let hashtable = args.is_present("hashtable");
	let arraylist = args.is_present("arraylist");
	let vec = args.is_present("vec");
	let vec_string = args.is_present("vec_string");
	let array = args.is_present("array");
	let hashmap = args.is_present("hashmap");
	let array_string = args.is_present("array_string");

	if hashtable {
		do_hashtable()?;
	}
	if hashmap {
		do_hashmap()?;
	}
	if arraylist {
		do_arraylist()?;
	}
	if vec {
		do_vec()?;
	}
	if array {
		do_array()?;
	}
	if array_string {
		do_array_string()?;
	}
	if vec_string {
		do_vec_string()?;
	}

	Ok(())
}
