// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::constants::*;
use crate::types::{HttpContext, HttpServerImpl};
use crate::{
	HttpConfig, HttpHeaders, HttpInstance, HttpInstanceType, HttpRequestType, HttpServer,
	PlainConfig,
};
use bmw_deps::chrono::Utc;
use bmw_deps::dirs;
use bmw_err::*;
use bmw_evh::{
	create_listeners, AttachmentHolder, Builder, ConnData, ConnectionData, EventHandler,
	EventHandlerConfig, ServerConnection, ThreadContext, READ_SLAB_DATA_SIZE,
};
use bmw_log::*;
use bmw_util::*;
use std::any::{type_name, Any};
use std::fs::{File, Metadata};
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;

info!();

const ERROR_CONTENT: &str = "Error ERROR_CODE: \"ERROR_MESSAGE\" occurred.";

impl Default for HttpConfig {
	fn default() -> Self {
		Self {
			evh_config: EventHandlerConfig::default(),
			instances: vec![HttpInstance {
				..Default::default()
			}],
		}
	}
}

impl Default for HttpInstance {
	fn default() -> Self {
		Self {
			port: 8080,
			addr: "127.0.0.1".to_string(),
			listen_queue_size: 100,
			http_dir: "~/.bmw/www".to_string(),
			instance_type: HttpInstanceType::Plain(PlainConfig {
				domainnames: vec![],
			}),
			default_file: vec!["index.html".to_string(), "index.htm".to_string()],
			error_404file: "error.html".to_string(),
			error_400file: "error.html".to_string(),
		}
	}
}

impl HttpHeaders<'_> {
	pub fn path(&self) -> Result<String, Error> {
		if self.start_uri > 0 && self.end_uri > self.start_uri {
			let path = std::str::from_utf8(&self.req[self.start_uri..self.end_uri])?.to_string();
			Ok(path)
		} else {
			Err(err!(ErrKind::Http, "no path"))
		}
	}
}

impl HttpServerImpl {
	pub(crate) fn new(config: HttpConfig) -> Result<HttpServerImpl, Error> {
		Ok(Self { config })
	}

	fn build_ctx<'a>(ctx: &'a mut ThreadContext) -> Result<&'a mut HttpContext, Error> {
		match ctx.user_data.downcast_ref::<HttpContext>() {
			Some(_) => {}
			None => {
				ctx.user_data = Box::new(Self::build_http_context()?);
			}
		}

		Ok(ctx.user_data.downcast_mut::<HttpContext>().unwrap())
	}

	fn build_http_context() -> Result<HttpContext, Error> {
		debug!("build ctx")?;
		global_slab_allocator!(SlabSize(128), SlabCount(1_000))?;

		let suffix_tree = Box::new(suffix_tree!(
			list![
				bmw_util::Builder::build_pattern(
					"\r\n\r\n",
					true,
					true,
					true,
					SUFFIX_TREE_TERMINATE_HEADERS_ID
				),
				pattern!(Regex("^GET .* "), Id(SUFFIX_TREE_GET_ID))?,
				pattern!(Regex("^POST .* "), Id(SUFFIX_TREE_POST_ID))?,
				pattern!(Regex("^HEAD .* "), Id(SUFFIX_TREE_HEAD_ID))?,
				pattern!(Regex("^GET .*\n"), Id(SUFFIX_TREE_GET_ID))?,
				pattern!(Regex("^POST .*\n"), Id(SUFFIX_TREE_POST_ID))?,
				pattern!(Regex("^HEAD .*\n"), Id(SUFFIX_TREE_HEAD_ID))?,
				pattern!(Regex("^GET .*\r"), Id(SUFFIX_TREE_GET_ID))?,
				pattern!(Regex("^POST .*\r"), Id(SUFFIX_TREE_POST_ID))?,
				pattern!(Regex("^HEAD .*\r"), Id(SUFFIX_TREE_HEAD_ID))?,
				pattern!(Regex("\r\n.*: "), Id(SUFFIX_TREE_HEADER))?
			],
			TerminationLength(100_000),
			MaxWildcardLength(100)
		)?);
		let matches = [bmw_util::Builder::build_match_default(); 1_000];
		let offset = 0;
		Ok(HttpContext {
			suffix_tree,
			matches,
			offset,
		})
	}

	fn process_error(
		_config: &HttpConfig,
		_path: String,
		conn_data: &mut ConnectionData,
		instance: &HttpInstance,
		code: u16,
		message: &str,
	) -> Result<(), Error> {
		let slash = if instance.http_dir.ends_with("/") {
			""
		} else {
			"/"
		};
		let fpath = if code == 404 {
			format!("{}{}{}", instance.http_dir, slash, instance.error_404file)
		} else {
			format!("{}{}{}", instance.http_dir, slash, instance.error_400file)
		};

		debug!("error page location: {}", fpath)?;
		let metadata = std::fs::metadata(fpath.clone());

		match metadata {
			Ok(metadata) => {
				Self::stream_file(fpath, metadata, conn_data, code, message)?;
			}
			Err(_) => {
				let dt = Utc::now();

				let error_content = ERROR_CONTENT
					.replace("ERROR_MESSAGE", message)
					.replace("ERROR_CODE", &format!("{}", code));

				let res = dt
					.format(
						&format!(
							"HTTP/1.1 {} {}\r\n\
Date: %a, %d %h %C%y %H:%M:%S GMT\r\n\
Content-Length: {}\r\n\r\n{}\n",
							code,
							message,
							error_content.len(),
							error_content
						)
						.to_string(),
					)
					.to_string();

				conn_data.write_handle().write(&res.as_bytes()[..])?;
			}
		}

		Ok(())
	}

	fn process_file(
		config: &HttpConfig,
		path: String,
		conn_data: &mut ConnectionData,
		instance: &HttpInstance,
	) -> Result<(), Error> {
		let fpath = format!("{}{}", instance.http_dir, path);
		let metadata = std::fs::metadata(fpath.clone());

		let metadata = match metadata {
			Ok(metadata) => metadata,
			Err(_e) => {
				debug!("404path={},dir={}", fpath, instance.http_dir)?;
				Self::process_error(config, path, conn_data, instance, 404, "Not Found")?;
				return Ok(());
			}
		};

		let (fpath, metadata) = if metadata.is_dir() {
			let mut fpath_ret: Option<String> = None;
			let mut metadata_ret: Option<Metadata> = None;
			let slash = if fpath.ends_with("/") { "" } else { "/" };

			for default_file in instance.default_file.clone() {
				let fpath_res = format!("{}{}{}", fpath, slash, default_file);
				let metadata_res = std::fs::metadata(fpath_res.clone());
				match metadata_res {
					Ok(metadata) => {
						fpath_ret = Some(fpath_res);
						metadata_ret = Some(metadata);
						break;
					}
					Err(_e) => {
						// not found, continue in loop to try next path
					}
				};
			}

			if fpath_ret.is_some() && metadata_ret.is_some() {
				(fpath_ret.unwrap(), metadata_ret.unwrap())
			} else {
				Self::process_error(config, path, conn_data, instance, 404, "Not Found")?;
				return Ok(());
			}
		} else {
			(fpath, metadata)
		};

		debug!("path={},dir={}", fpath, instance.http_dir)?;

		Self::stream_file(fpath, metadata, conn_data, 200, "OK")?;

		Ok(())
	}

	fn stream_file(
		fpath: String,
		metadata: Metadata,
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
	) -> Result<(), Error> {
		let file = File::open(fpath)?;
		let mut buf_reader = BufReader::new(file);

		let dt = Utc::now();
		let res = dt
			.format(
				&format!(
					"HTTP/1.1 {} {}\r\n\
Date: %a, %d %h %C%y %H:%M:%S GMT\r\n\
Content-Length: ",
					code, message
				)
				.to_string(),
			)
			.to_string();

		let res = format!("{}{}\r\n\r\n", res, metadata.len());

		debug!("writing {}", res)?;
		conn_data.write_handle().write(&res.as_bytes()[..])?;

		loop {
			let mut buf = vec![0u8; 100];
			let len = buf_reader.read(&mut buf)?;
			conn_data.write_handle().write(&buf[0..len])?;
			if len == 0 {
				break;
			}
		}

		Ok(())
	}

	fn build_headers<'a>(
		req: &'a Vec<u8>,
		start: usize,
		mut matches: [bmw_util::Match; 1_000],
		suffix_tree: &mut Box<dyn SuffixTree + Send + Sync>,
		slab_offset: usize,
	) -> Result<HttpHeaders<'a>, Error> {
		let mut termination_point = 0;
		let count = suffix_tree.tmatch(&req[start..], &mut matches)?;

		debug!(
			"count={},slab_offset={},start={}",
			count, slab_offset, start
		)?;

		let mut start_uri = 0;
		let mut end_uri = 0;
		let mut http_request_type = HttpRequestType::UNKNOWN;

		debug!("count={}", count)?;
		for i in 0..count {
			debug!("c[{}]={:?}", i, matches[i])?;
			let end = matches[i].end();
			let start = matches[i].start();
			let id = matches[i].id();

			if id == SUFFIX_TREE_TERMINATE_HEADERS_ID {
				debug!("found term end={}, slab_off={}", end, slab_offset)?;

				if end_uri == 0 {
					match req.windows(1).position(|window| window == " ".as_bytes()) {
						Some(c) => {
							start_uri = c + 1;
							end_uri = end;
							for j in start_uri..end {
								if req[j] == '\r' as u8
									|| req[j] == '\n' as u8 || req[j] == ' ' as u8
								{
									end_uri = j;
									break;
								}
							}
						}
						None => {}
					}
				}

				if end == slab_offset.into() {
					termination_point = end;
				}
			} else if id == SUFFIX_TREE_GET_ID
				|| id == SUFFIX_TREE_POST_ID
				|| id == SUFFIX_TREE_HEAD_ID
			{
				debug!("id is GET/POST = {}", id)?;
				if id == SUFFIX_TREE_GET_ID {
					start_uri = start + 4;
					http_request_type = HttpRequestType::GET;
				} else {
					if id == SUFFIX_TREE_POST_ID {
						http_request_type = HttpRequestType::POST;
					} else if id == SUFFIX_TREE_HEAD_ID {
						http_request_type = HttpRequestType::HEAD;
					}
					start_uri = start + 5;
				}
				for i in start_uri..end {
					if req[start + i] == ' ' as u8 {
						end_uri = start + i;
						break;
					}
				}
			}
		}

		if termination_point != 0 && start_uri == 0 {
			Err(err!(ErrKind::Http, "URI not specified".to_string()))
		} else if termination_point != 0 && http_request_type == HttpRequestType::UNKNOWN {
			Err(err!(ErrKind::Http, "Unknown http request type".to_string()))
		} else {
			Ok(HttpHeaders {
				termination_point,
				start,
				req,
				start_uri,
				end_uri,
				http_request_type,
			})
		}
	}

	fn type_of<T>(_: T) -> &'static str {
		type_name::<T>()
	}

	fn header_error(
		config: &HttpConfig,
		path: String,
		conn_data: &mut ConnectionData,
		instance: &HttpInstance,
		err: Error,
	) -> Result<(), Error> {
		debug!("Err: {:?}", err.inner())?;
		let err_text = err.inner();
		if err_text == "http_error: Unknown http request type" {
			Self::process_error(
				config,
				path,
				conn_data,
				instance,
				501,
				"Unknown Request Type",
			)?;
		} else {
			Self::process_error(config, path, conn_data, instance, 400, "Bad Request")?;
		}
		conn_data.write_handle().close()?;
		Ok(())
	}

	fn process_on_read(
		config: &HttpConfig,
		conn_data: &mut ConnectionData,
		ctx: &mut ThreadContext,
		attachment: Option<AttachmentHolder>,
	) -> Result<(), Error> {
		let attachment = match attachment {
			Some(attachment) => attachment,
			None => return Err(err!(ErrKind::Http, "no instance found for this request1")),
		};
		debug!(
			"atttypename={:?},type={}",
			attachment.clone(),
			Self::type_of(attachment.clone())
		)?;
		let attachment = attachment.attachment.downcast_ref::<HttpInstance>();

		let attachment = match attachment {
			Some(attachment) => attachment,
			None => {
				return Err(err!(ErrKind::Http, "no instance found for this request2"));
			}
		};
		debug!("conn_data.tid={},att={:?}", conn_data.tid(), attachment)?;
		let ctx = Self::build_ctx(ctx)?;
		debug!("on read slab_offset = {}", conn_data.slab_offset())?;
		let first_slab = conn_data.first_slab();
		let last_slab = conn_data.last_slab();
		let slab_offset = conn_data.slab_offset();
		debug!("firstslab={},last_slab={}", first_slab, last_slab)?;
		let (req, slab_id_vec, slab_count) = conn_data.borrow_slab_allocator(move |sa| {
			let mut slab_id_vec = vec![];
			let mut slab_id = first_slab;
			let mut ret: Vec<u8> = vec![];
			let mut slab_count = 0;
			loop {
				slab_count += 1;
				slab_id_vec.push(slab_id);
				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = if slab_id == last_slab {
					slab_offset as usize
				} else {
					READ_SLAB_DATA_SIZE
				};

				let slab_bytes_data = &slab_bytes[0..offset];
				debug!("read bytes = {:?}", slab_bytes_data)?;
				ret.extend(slab_bytes_data);

				if slab_id == last_slab {
					break;
				}
				slab_id = u32::from_be_bytes(try_into!(
					slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
				)?);
			}
			Ok((ret, slab_id_vec, slab_count))
		})?;

		let mut start = 0;
		let mut last_term = 0;
		let mut termination_sum = 0;

		loop {
			debug!("slab_count={}", slab_count)?;
			let headers = Self::build_headers(
				&req,
				start,
				ctx.matches,
				&mut ctx.suffix_tree,
				(slab_offset as usize + (slab_count - 1) * READ_SLAB_DATA_SIZE).into(),
			);

			let headers = match headers {
				Ok(headers) => headers,
				Err(e) => {
					Self::header_error(config, "".to_string(), conn_data, attachment, e)?;
					return Ok(());
				}
			};

			debug!("Request type = {:?}", headers.http_request_type)?;

			termination_sum += headers.termination_point;

			debug!("term point = {}", headers.termination_point)?;
			if headers.termination_point == 0 {
				break;
			} else {
				let path = match headers.path() {
					Ok(path) => path,
					Err(e) => {
						Self::header_error(config, "".to_string(), conn_data, attachment, e)?;
						return Ok(());
					}
				};
				start = headers.termination_point;
				last_term = headers.termination_point;
				Self::process_file(config, path, conn_data, attachment)?;
			}

			debug!("start={}", headers.start)?;
		}

		debug!("last term = {}", last_term)?;

		if termination_sum != req.len() {
			ctx.offset = termination_sum % READ_SLAB_DATA_SIZE;
		}
		let del_slab = slab_id_vec[termination_sum / READ_SLAB_DATA_SIZE];

		if termination_sum > 0 {
			conn_data.clear_through(del_slab)?;
		}

		Ok(())
	}

	fn process_on_accept(
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_close(
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_panic(_ctx: &mut ThreadContext, _e: Box<dyn Any + Send>) -> Result<(), Error> {
		Ok(())
	}

	fn process_housekeeper(_ctx: &mut ThreadContext) -> Result<(), Error> {
		Ok(())
	}
}

impl HttpServer for HttpServerImpl {
	fn start(&mut self) -> Result<(), Error> {
		if self.config.instances.len() == 0 {
			return Err(err!(
				ErrKind::IllegalArgument,
				"At least one instance must be specified"
			));
		}

		let home_dir = match dirs::home_dir() {
			Some(p) => p,
			None => PathBuf::new(),
		}
		.as_path()
		.display()
		.to_string();

		for i in 0..self.config.instances.len() {
			self.config.instances[i].http_dir =
				self.config.instances[i].http_dir.replace("~", &home_dir);
		}

		let mut evh = Builder::build_evh(self.config.evh_config.clone())?;
		let config = &self.config;
		let config = config.clone();

		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config, conn_data, ctx, attach)
		})?;
		evh.set_on_accept(move |conn_data, ctx| Self::process_on_accept(conn_data, ctx))?;
		evh.set_on_close(move |conn_data, ctx| Self::process_on_close(conn_data, ctx))?;
		evh.set_on_panic(move |ctx, e| Self::process_on_panic(ctx, e))?;
		evh.set_housekeeper(move |ctx| Self::process_housekeeper(ctx))?;

		evh.start()?;

		for instance in &self.config.instances {
			let port = instance.port;
			let addr = &instance.addr;

			debug!("creating listener for {}", port)?;
			let addr = format!("{}:{}", addr, port);
			let handles = create_listeners(self.config.evh_config.threads, &addr, 10, false)?;

			let sc = ServerConnection {
				tls_config: vec![],
				handles,
				is_reuse_port: false,
			};

			evh.add_server(sc, Box::new(instance.clone()))?;
		}

		Ok(())
	}

	fn stop(&mut self) -> Result<(), Error> {
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::types::HttpServerImpl;
	use crate::{HttpConfig, HttpInstance, HttpServer};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::port::pick_free_port;
	use bmw_test::testdir::{setup_test_dir, tear_down_test_dir};
	use std::fs::File;
	use std::io::Read;
	use std::io::Write;
	use std::net::TcpStream;
	use std::str::from_utf8;

	debug!();

	#[test]
	fn test_http_slow_requests() -> Result<(), Error> {
		let port = pick_free_port()?;
		let test_dir = ".test_http_slow_requests.bmw";
		setup_test_dir(test_dir)?;
		let mut file = File::create(format!("{}/abc.html", test_dir))?;
		file.write_all(b"Hello, world!")?;

		let mut file = File::create(format!("{}/def1.html", test_dir))?;
		file.write_all(b"Hello, world2!")?;
		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				http_dir: test_dir.to_string(),
				..Default::default()
			}],
			..Default::default()
		};
		let mut http = HttpServerImpl::new(config)?;
		http.start()?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let addr = &format!("127.0.0.1:{}", port)[..];
		info!("addr={}", addr)?;
		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));

		client.write(b"GET /abc.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		client.write(b"\r\n\r\n")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let mut buf = [0; 128];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 89);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		client.write(b"POST /def1.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		let mut buf = [0; 128];
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 90);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		tear_down_test_dir(test_dir)?;

		Ok(())
	}

	#[test]
	fn test_http_server_basic() -> Result<(), Error> {
		let test_dir = ".test_http_server_basic.bmw";
		setup_test_dir(test_dir)?;
		let mut file = File::create(format!("{}/foo.html", test_dir))?;
		file.write_all(b"Hello, world!")?;
		let port = pick_free_port()?;
		info!("port={}", port)?;
		let config = HttpConfig {
			instances: vec![HttpInstance {
				port,
				http_dir: test_dir.to_string(),
				..Default::default()
			}],
			..Default::default()
		};
		let mut http = HttpServerImpl::new(config)?;
		http.start()?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let addr = &format!("127.0.0.1:{}", port)[..];
		info!("addr={}", addr)?;
		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));

		client.write(b"GET /foo.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let mut buf = [0; 128];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 89);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		client.write(b"POST /foo.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let mut buf = [0; 128];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 89);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
