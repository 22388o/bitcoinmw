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
use crate::types::{HttpCacheImpl, HttpContext, HttpServerImpl};
use crate::HttpInstanceType::{Plain, Tls};
use crate::{
	ConnectionType, HttpCache, HttpConfig, HttpHeader, HttpHeaders, HttpInstance, HttpInstanceType,
	HttpRequestType, HttpServer, HttpVersion, PlainConfig,
};
use bmw_deps::chrono::{DateTime, TimeZone, Utc};
use bmw_deps::dirs;
use bmw_deps::rand::random;
use bmw_deps::substring::Substring;
use bmw_err::*;
use bmw_evh::{
	create_listeners, AttachmentHolder, Builder, CloseHandle, ConnData, ConnectionData,
	EventHandler, EventHandlerConfig, EventHandlerData, ServerConnection, ThreadContext,
	TlsServerConfig, WriteHandle, READ_SLAB_DATA_SIZE,
};
use bmw_log::*;
use bmw_util::*;
use std::any::{type_name, Any};
use std::collections::{HashMap, HashSet};
use std::fs::{File, Metadata};
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use std::path::{Component, Path};
use std::str::from_utf8;
use std::time::{SystemTime, UNIX_EPOCH};

info!();

fn rfind_utf8(s: &str, chr: char) -> Option<usize> {
	if let Some(rev_pos) = s.chars().rev().position(|c| c == chr) {
		Some(s.chars().count() - rev_pos - 1)
	} else {
		None
	}
}

impl Default for HttpHeader {
	fn default() -> Self {
		Self {
			start_header_name: 0,
			end_header_name: 0,
			start_header_value: 0,
			end_header_value: 0,
		}
	}
}

impl Default for HttpConfig {
	fn default() -> Self {
		Self {
			evh_config: EventHandlerConfig::default(),
			instances: vec![HttpInstance {
				..Default::default()
			}],
			debug: false,
			cache_slab_count: 10_000,
			idle_timeout: 60_000,
			server_name: "BitcoinMW HTTP Server".to_string(),
			server_version: "0.0.0".to_string(),
			mime_map: vec![
				("html".to_string(), "text/html".to_string()),
				("htm".to_string(), "text/html".to_string()),
				("shtml".to_string(), "text/html".to_string()),
				("txt".to_string(), "text/plain".to_string()),
				("css".to_string(), "text/css".to_string()),
				("xml".to_string(), "text/xml".to_string()),
				("gif".to_string(), "image/gif".to_string()),
				("jpeg".to_string(), "image/jpeg".to_string()),
				("jpg".to_string(), "image/jpeg".to_string()),
				("js".to_string(), "application/javascript".to_string()),
				("atom".to_string(), "application/atom+xml".to_string()),
				("rss".to_string(), "application/rss+xml".to_string()),
				("mml".to_string(), "text/mathml".to_string()),
				(
					"jad".to_string(),
					"text/vnd.sun.j2me.app-descriptor".to_string(),
				),
				("wml".to_string(), "text/vnd.wap.wml".to_string()),
				("htc".to_string(), "text/x-component".to_string()),
				("avif".to_string(), "image/avif".to_string()),
				("png".to_string(), "image/png".to_string()),
				("svg".to_string(), "image/svg+xml".to_string()),
				("svgz".to_string(), "image/svg+xml".to_string()),
				("tif".to_string(), "image/tiff".to_string()),
				("tiff".to_string(), "image/tiff".to_string()),
				("wbmp".to_string(), "image/vnd.wap.wbmp".to_string()),
				("webp".to_string(), "image/webp".to_string()),
				("ico".to_string(), "image/x-icon".to_string()),
				("jng".to_string(), "image/x-jng".to_string()),
				("bmp".to_string(), "image/x-ms-bmp".to_string()),
				("woff".to_string(), "font/woff".to_string()),
				("woff2".to_string(), "font/woff2".to_string()),
				("jar".to_string(), "application/java-archive".to_string()),
				("war".to_string(), "application/java-archive".to_string()),
				("ear".to_string(), "application/java-archive".to_string()),
				("json".to_string(), "application/json".to_string()),
				("hqx".to_string(), "application/mac-binhex40".to_string()),
				("doc".to_string(), "application/msword".to_string()),
				("pdf".to_string(), "application/pdf".to_string()),
				("ps".to_string(), "application/postscript".to_string()),
				("eps".to_string(), "application/postscript".to_string()),
				("ai".to_string(), "application/postscript".to_string()),
				("rtf".to_string(), "application/rtf".to_string()),
				(
					"m3u8".to_string(),
					"application/vnd.apple.mpegurl".to_string(),
				),
				(
					"kml".to_string(),
					"application/vnd.google-earth.kml+xml".to_string(),
				),
				(
					"kmz".to_string(),
					"application/vnd.google-earth.kmz".to_string(),
				),
				("xls".to_string(), "application/vnd.ms-excel".to_string()),
				(
					"eot".to_string(),
					"application/vnd.ms-fontobject".to_string(),
				),
				(
					"ppt".to_string(),
					"application/vnd.ms-powerpoint".to_string(),
				),
				(
					"odg".to_string(),
					"application/vnd.oasis.opendocument.graphics".to_string(),
				),
				(
					"odp".to_string(),
					"application/vnd.oasis.opendocument.presentation".to_string(),
				),
				(
					"ods".to_string(),
					"application/vnd.oasis.opendocument.spreadsheet".to_string(),
				),
				(
					"odt".to_string(),
					"application/vnd.oasis.opendocument.text".to_string(),
				),
				(
					"pptx".to_string(),
					"application/vnd.openxmlformats-officedocument.presentationml.presentation"
						.to_string(),
				),
				(
					"xlsx".to_string(),
					"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
				),
				(
					"docx".to_string(),
					"application/vnd.openxmlformats-officedocument.wordprocessingml.document"
						.to_string(),
				),
				("wmlc".to_string(), "application/vnd.wap.wmlc".to_string()),
				("wasm".to_string(), "application/wasm".to_string()),
				("7z".to_string(), "application/x-7z-compressed".to_string()),
				("cco".to_string(), "application/x-cocoa".to_string()),
				(
					"jardiff".to_string(),
					"application/x-java-archive-diff".to_string(),
				),
				(
					"jnlp".to_string(),
					"application/x-java-jnlp-file".to_string(),
				),
				("run".to_string(), "application/x-makeself".to_string()),
				("pl".to_string(), "application/x-perl".to_string()),
				("pm".to_string(), "application/x-perl".to_string()),
				("prc".to_string(), "application/x-pilot".to_string()),
				("pbd".to_string(), "application/x-pilot".to_string()),
				(
					"rar".to_string(),
					"application/x-rar-compressed".to_string(),
				),
				(
					"rpm".to_string(),
					"application/x-redhat-package-manager".to_string(),
				),
				("sea".to_string(), "application/x-sea".to_string()),
				(
					"swf".to_string(),
					"application/x-shockwave-flash".to_string(),
				),
				("sit".to_string(), "application/x-stuffit".to_string()),
				("tcl".to_string(), "application/x-tcl".to_string()),
				("tk".to_string(), "application/x-tcl".to_string()),
				("der".to_string(), "application/x-x509-ca-cert".to_string()),
				("pem".to_string(), "application/x-x509-ca-cert".to_string()),
				("crt".to_string(), "application/x-x509-ca-cert".to_string()),
				("xpi".to_string(), "application/x-xpinstall".to_string()),
				("xhtml".to_string(), "application/xhtml+xml".to_string()),
				("xspf".to_string(), "application/xspf+xml".to_string()),
				("zip".to_string(), "application/zip".to_string()),
				("bin".to_string(), "application/octet-stream".to_string()),
				("exe".to_string(), "application/octet-stream".to_string()),
				("dll".to_string(), "application/octet-stream".to_string()),
				("deb".to_string(), "application/octet-stream".to_string()),
				("dmg".to_string(), "application/octet-stream".to_string()),
				("iso".to_string(), "application/octet-stream".to_string()),
				("img".to_string(), "application/octet-stream".to_string()),
				("msi".to_string(), "application/octet-stream".to_string()),
				("msp".to_string(), "application/octet-stream".to_string()),
				("msm".to_string(), "application/octet-stream".to_string()),
				("mid".to_string(), "audio/midi".to_string()),
				("midi".to_string(), "audio/midi".to_string()),
				("kar".to_string(), "audio/midi".to_string()),
				("mp3".to_string(), "audio/mpeg".to_string()),
				("ogg".to_string(), "audio/ogg".to_string()),
				("m4a".to_string(), "audio/x-m4a".to_string()),
				("ra".to_string(), "audio/x-realaudio".to_string()),
				("3gpg".to_string(), "video/3gpp".to_string()),
				("3gp".to_string(), "video/mp2t".to_string()),
				("ts".to_string(), "video/mp2t".to_string()),
				("mp4".to_string(), "video/mp4".to_string()),
				("mpeg".to_string(), "video/mpeg".to_string()),
				("mpg".to_string(), "video/mpeg".to_string()),
				("mov".to_string(), "video/quicktime".to_string()),
				("webm".to_string(), "video/webm".to_string()),
				("flv".to_string(), "video/x-flv".to_string()),
				("m4v".to_string(), "video/x-m4v".to_string()),
				("mng".to_string(), "video/x-mng".to_string()),
				("asx".to_string(), "video/x-ms-asf".to_string()),
				("asf".to_string(), "video/x-ms-asf".to_string()),
				("wmv".to_string(), "video/x-ms-wmv".to_string()),
				("avi".to_string(), "video/x-msvideo".to_string()),
			],
		}
	}
}

impl Default for HttpInstance {
	fn default() -> Self {
		let mut http_dir_map = HashMap::new();
		http_dir_map.insert("*".to_string(), "~/.bmw/www".to_string());
		Self {
			port: 8080,
			addr: "127.0.0.1".to_string(),
			listen_queue_size: 100,
			instance_type: HttpInstanceType::Plain(PlainConfig { http_dir_map }),
			default_file: vec!["index.html".to_string(), "index.htm".to_string()],
			error_400file: "error.html".to_string(),
			error_403file: "error.html".to_string(),
			error_404file: "error.html".to_string(),
			callback_extensions: HashSet::new(),
			callback_mappings: HashSet::new(),
			callback: None,
		}
	}
}

impl HttpHeaders<'_> {
	pub fn path(&self) -> Result<String, Error> {
		if self.start_uri > 0 && self.end_uri > self.start_uri {
			let path = std::str::from_utf8(&self.req[self.start_uri..self.end_uri])?.to_string();
			if path.contains("?") {
				let pos = path.chars().position(|c| c == '?').unwrap();
				let path = path.substring(0, pos);
				Ok(path.to_string())
			} else {
				Ok(path)
			}
		} else {
			Err(err!(ErrKind::Http, "no path"))
		}
	}

	pub fn query(&self) -> Result<String, Error> {
		let path = std::str::from_utf8(&self.req[self.start_uri..self.end_uri])?.to_string();
		let query = if path.contains("?") {
			let pos = path.chars().position(|c| c == '?').unwrap();
			path.substring(pos + 1, path.len()).to_string()
		} else {
			"".to_string()
		};
		Ok(query)
	}

	pub fn http_request_type(&self) -> Result<&HttpRequestType, Error> {
		Ok(&self.http_request_type)
	}

	pub fn version(&self) -> Result<&HttpVersion, Error> {
		Ok(&self.version)
	}

	pub fn header_count(&self) -> Result<usize, Error> {
		Ok(self.header_count)
	}

	pub fn header_name(&self, i: usize) -> Result<String, Error> {
		let ret = std::str::from_utf8(
			&self.req[self.headers[i].start_header_name..self.headers[i].end_header_name],
		)?
		.to_string();
		Ok(ret)
	}

	pub fn header_value(&self, i: usize) -> Result<String, Error> {
		let ret = std::str::from_utf8(
			&self.req[self.headers[i].start_header_value..self.headers[i].end_header_value],
		)?
		.to_string();
		Ok(ret)
	}

	pub fn host(&self) -> Result<&String, Error> {
		Ok(&self.host)
	}

	pub fn if_none_match(&self) -> Result<&String, Error> {
		Ok(&self.if_none_match)
	}

	pub fn if_modified_since(&self) -> Result<&String, Error> {
		Ok(&self.if_modified_since)
	}

	pub fn extension(&self) -> Result<String, Error> {
		let path = if self.start_uri > 0 && self.end_uri > self.start_uri {
			let path = std::str::from_utf8(&self.req[self.start_uri..self.end_uri])?.to_string();
			if path.contains("?") {
				let pos = path.chars().position(|c| c == '?').unwrap();
				let path = path.substring(0, pos);
				path.to_string()
			} else {
				path
			}
		} else {
			return Err(err!(ErrKind::Http, "no path"));
		};

		let path_len = path.len();

		Ok(match rfind_utf8(&path, '.') {
			Some(pos) => match pos + 1 < path_len {
				true => path.substring(pos + 1, path_len).to_string(),
				false => "".to_string(),
			},
			None => "".to_string(),
		})
	}

	pub fn connection(&self) -> Result<ConnectionType, Error> {
		Ok(self.connection)
	}

	pub fn range_start(&self) -> Result<usize, Error> {
		Ok(self.range_start)
	}

	pub fn range_end(&self) -> Result<usize, Error> {
		Ok(self.range_end)
	}

	pub fn has_range(&self) -> Result<bool, Error> {
		Ok(self.range_start != 0 || self.range_end != usize::MAX)
	}
}

impl HttpServerImpl {
	pub(crate) fn new(config: &HttpConfig) -> Result<HttpServerImpl, Error> {
		let cache = lock_box!(HttpCacheImpl::new(config)?)?;
		Ok(Self {
			config: config.clone(),
			cache,
		})
	}

	fn build_ctx<'a>(
		ctx: &'a mut ThreadContext,
		config: &HttpConfig,
	) -> Result<&'a mut HttpContext, Error> {
		match ctx.user_data.downcast_ref::<HttpContext>() {
			Some(_) => {}
			None => {
				ctx.user_data = Box::new(Self::build_http_context(config)?);
			}
		}

		Ok(ctx.user_data.downcast_mut::<HttpContext>().unwrap())
	}

	fn build_http_context(config: &HttpConfig) -> Result<HttpContext, Error> {
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
				pattern!(Regex("\r\n.*: "), Id(SUFFIX_TREE_HEADER_ID))?
			],
			TerminationLength(100_000),
			MaxWildcardLength(100)
		)?);
		let matches = [bmw_util::Builder::build_match_default(); 1_000];
		let offset = 0;
		let connections = HashMap::new();
		let mut mime_map = HashMap::new();
		let mut mime_lookup = HashMap::new();
		let mut mime_rev_lookup = HashMap::new();

		for i in 0..config.mime_map.len() {
			mime_lookup.insert(i as u32, config.mime_map[i].1.clone());
			mime_rev_lookup.insert(config.mime_map[i].0.clone(), i as u32);
			mime_map.insert(config.mime_map[i].0.clone(), config.mime_map[i].1.clone());
		}

		Ok(HttpContext {
			suffix_tree,
			matches,
			offset,
			connections,
			mime_map,
			mime_lookup,
			mime_rev_lookup,
			now: 0,
		})
	}

	pub(crate) fn build_response_headers(
		config: &HttpConfig,
		code: u16,
		message: &str,
		content_len: usize,
		file_len: usize,
		content: Option<String>,
		content_type: Option<String>,
		_ctx: &HttpContext,
		headers: &HttpHeaders,
		error: bool,
		last_modified: u64,
		etag: String,
	) -> Result<(bool, String), Error> {
		let dt = Utc::now();
		let mut connection_type = headers.connection()?;
		let version = headers.version()?;
		let mut keep_alive = connection_type == ConnectionType::KeepAlive;
		if version != &HttpVersion::HTTP11 || error {
			keep_alive = false;
			connection_type = ConnectionType::CLOSE;
		}

		let mut range_end = headers.range_end()?;
		if range_end > file_len.saturating_sub(1) {
			range_end = file_len.saturating_sub(1);
		}
		let range_start = headers.range_start()?;
		let res = dt
			.format(&format!(
				"HTTP/{} {} {}\r\n\
Date: %a, %d %h %C%y %H:%M:%S GMT\r\n\
Server: {} {}\r\n{}{}\
Connection: {}\r\n\
Last-Modified: {}\r\n\
ETag: {}\r\n\
Content-Length: {}\r\n\r\n{}",
				match version {
					HttpVersion::HTTP11 => "1.1",
					_ => "1.0",
				},
				code,
				message,
				config.server_name,
				config.server_version,
				match content_type {
					Some(content_type) => format!("Content-Type: {}\r\n", content_type),
					None => "".to_string(),
				},
				match headers.has_range()? && !error {
					true => format!(
						"Content-Range: bytes {}-{}/{}\r\n",
						range_start, range_end, file_len
					),
					false => "Accept-Ranges: bytes\r\n".to_string(),
				},
				match connection_type {
					ConnectionType::KeepAlive => "keep-alive",
					_ => "close",
				},
				DateTime::from_timestamp(try_into!(last_modified / 1000)?, 0)
					.unwrap_or(UNIX_EPOCH.into())
					.format("%a, %d %h %C%y %H:%M:%S GMT"),
				etag,
				content_len,
				match content {
					Some(content) => content,
					None => "".to_string(),
				}
			))
			.to_string();

		Ok((keep_alive, res))
	}

	fn find_http_dir(host: &String, map: &HashMap<String, String>) -> Result<String, Error> {
		match map.get(host) {
			Some(http_dir) => Ok(http_dir.clone()),
			None => match map.get("*") {
				Some(http_dir) => Ok(http_dir.clone()),
				None => Err(err!(ErrKind::Http, "could not find http_dir".to_string())),
			},
		}
	}

	fn http_dir(instance: &HttpInstance, headers: &HttpHeaders) -> Result<String, Error> {
		let mut host = headers.host()?.clone();
		if host.contains(":") {
			let pos = host
				.as_bytes()
				.iter()
				.position(|&s| s == b':')
				.unwrap_or(host.len());
			host = host.clone().substring(0, pos).to_string();
		}
		Ok(match &instance.instance_type {
			Plain(config) => Self::find_http_dir(&host, &config.http_dir_map)?,
			Tls(config) => Self::find_http_dir(&host, &config.http_dir_map)?,
		})
	}

	fn process_error(
		config: &HttpConfig,
		_path: String,
		conn_data: &mut ConnectionData,
		instance: &HttpInstance,
		code: u16,
		message: &str,
		cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
		headers: &HttpHeaders,
		ctx: &HttpContext,
	) -> Result<(), Error> {
		let http_dir = Self::http_dir(instance, headers)?;
		let slash = if http_dir.ends_with("/") { "" } else { "/" };
		let fpath = if code == 404 {
			format!("{}{}{}", http_dir, slash, instance.error_404file)
		} else if code == 403 {
			format!("{}{}{}", http_dir, slash, instance.error_403file)
		} else {
			format!("{}{}{}", http_dir, slash, instance.error_400file)
		};

		debug!("error page location: {}", fpath)?;
		let metadata = std::fs::metadata(fpath.clone());

		match metadata {
			Ok(metadata) => {
				Self::stream_file(
					config,
					fpath,
					metadata.len(),
					conn_data,
					code,
					message,
					cache,
					ctx,
					headers,
					try_into!(metadata.modified()?.duration_since(UNIX_EPOCH)?.as_millis())?,
				)?;
			}
			Err(_) => {
				let error_content = ERROR_CONTENT
					.replace("ERROR_MESSAGE", message)
					.replace("ERROR_CODE", &format!("{}", code));
				let last_modified = try_into!(ctx.now)?;
				let etag = format!("{}-{:01x}", last_modified, error_content.len());

				let (keep_alive, res) = Self::build_response_headers(
					config,
					code,
					message,
					error_content.len(),
					error_content.len(),
					Some(error_content),
					Some("text/html".to_string()),
					ctx,
					headers,
					true,
					last_modified,
					etag,
				)?;

				let mut write_handle = conn_data.write_handle();
				write_handle.write(&res.as_bytes()[..])?;
				if !keep_alive {
					write_handle.close()?;
				}
			}
		}

		Ok(())
	}

	pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
		let ends_with_slash = path.as_ref().to_str().map_or(false, |s| s.ends_with('/'));
		let mut normalized = PathBuf::new();
		for component in path.as_ref().components() {
			match &component {
				Component::ParentDir => {
					if !normalized.pop() {
						normalized.push(component);
					}
				}
				_ => {
					normalized.push(component);
				}
			}
		}
		if ends_with_slash {
			normalized.push("");
		}
		normalized
	}

	fn process_file(
		config: &HttpConfig,
		mut cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
		path: String,
		conn_data: &mut ConnectionData,
		instance: &HttpInstance,
		headers: &HttpHeaders,
		ctx: &HttpContext,
	) -> Result<bool, Error> {
		let http_dir = Self::http_dir(instance, headers)?;
		let fpath = format!("{}{}", http_dir, path);

		let fpath = Self::normalize_path(fpath)
			.into_os_string()
			.into_string()
			.unwrap_or(format!("{}/", http_dir));

		if !fpath.starts_with(&http_dir) || !path.starts_with("/") {
			Self::process_error(
				config,
				path,
				conn_data,
				instance,
				403,
				"Forbidden",
				cache,
				headers,
				ctx,
			)?;
			return Ok(false);
		}

		if Self::try_cache(cache.clone(), &fpath, conn_data, ctx, config, headers)? {
			return Ok(true);
		}

		let metadata = std::fs::metadata(fpath.clone());

		let metadata = match metadata {
			Ok(metadata) => metadata,
			Err(_e) => {
				debug!("404path={},dir={}", fpath, http_dir)?;
				Self::process_error(
					config,
					path,
					conn_data,
					instance,
					404,
					"Not Found",
					cache,
					headers,
					ctx,
				)?;
				return Ok(false);
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
				Self::process_error(
					config,
					path,
					conn_data,
					instance,
					404,
					"Not Found",
					cache,
					headers,
					ctx,
				)?;
				return Ok(false);
			}
		} else {
			(fpath, metadata)
		};

		debug!("path={},dir={}", fpath, http_dir)?;

		let hit: bool;
		{
			{
				let cache = cache.rlock()?;
				hit = (**cache.guard())
					.stream_file(&fpath, conn_data, 200, "OK", ctx, config, headers)?;
			}
			let r = random::<u64>();
			debug!("r={}", r)?;
			if hit && r % 2 == 0 {
				let mut cache = cache.wlock()?;
				(**cache.guard()).bring_to_front(&fpath)?;
			}
		}

		if !hit {
			Self::stream_file(
				config,
				fpath,
				metadata.len(),
				conn_data,
				200,
				"OK",
				cache,
				ctx,
				headers,
				try_into!(metadata.modified()?.duration_since(UNIX_EPOCH)?.as_millis())?,
			)?;
		}

		Ok(hit)
	}

	fn try_cache(
		mut cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
		path: &String,
		conn_data: &mut ConnectionData,
		ctx: &HttpContext,
		config: &HttpConfig,
		headers: &HttpHeaders,
	) -> Result<bool, Error> {
		debug!("try cache: {}", path)?;
		let hit: bool;
		{
			let cache = cache.rlock()?;
			hit =
				(**cache.guard()).stream_file(path, conn_data, 200, "OK", ctx, config, headers)?;
		}
		let r = random::<u64>();
		debug!("cache hit={}", hit)?;

		if hit && r % 2 == 0 {
			let mut cache = cache.wlock()?;
			(**cache.guard()).bring_to_front(path)?;
		}

		Ok(hit)
	}

	fn stream_file(
		config: &HttpConfig,
		fpath: String,
		len: u64,
		conn_data: &mut ConnectionData,
		mut code: u16,
		mut message: &str,
		mut cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
		ctx: &HttpContext,
		headers: &HttpHeaders,
		last_modified: u64,
	) -> Result<(), Error> {
		let file = File::open(fpath.clone())?;
		let mut buf_reader = BufReader::new(file);
		let len_usize: usize = try_into!(len)?;

		let range_start = headers.range_start()?;
		let range_end = headers.range_end()?;
		let range_end_content = if range_end > len_usize {
			len_usize
		} else {
			range_end
		};
		let content_len = range_end_content.saturating_sub(range_start);

		let path_len = fpath.len();
		let extension = match rfind_utf8(&fpath, '.') {
			Some(pos) => match pos + 1 < path_len {
				true => fpath.substring(pos + 1, path_len).to_string(),
				false => "".to_string(),
			},
			None => "".to_string(),
		};

		let etag = format!("{}-{:01x}", last_modified, content_len);
		let modified_since = DateTime::parse_from_rfc2822(headers.if_modified_since()?)
			.unwrap_or(Utc.with_ymd_and_hms(1970, 1, 1, 0, 1, 1).unwrap().into());
		info!("formated if_modified_since = {:?}", modified_since);
		let modified_since = modified_since.timestamp_millis();
		info!(
			"modified since = {:?}, last_mod={:?}",
			modified_since, last_modified
		);

		if &etag == headers.if_none_match()? || last_modified < try_into!(modified_since)? {
			code = 304;
			message = "Not Modified";
		}

		let (keep_alive, res) = Self::build_response_headers(
			config,
			match headers.has_range()? {
				true => 206,
				false => code,
			},
			match headers.has_range()? {
				true => "Partial Content",
				false => message,
			},
			try_into!(content_len)?,
			try_into!(len)?,
			None,
			match ctx.mime_map.get(&extension) {
				Some(mime_type) => Some(mime_type.clone()),
				None => Some("text/plain".to_string()),
			},
			ctx,
			headers,
			false,
			last_modified,
			etag,
		)?;

		debug!("writing {}", res)?;

		let mut write_error = false;
		let mut write_handle = conn_data.write_handle();
		match write_handle.write(&res.as_bytes()[..]) {
			Ok(_) => {}
			Err(_) => write_error = true,
		}

		let mut buf = vec![0u8; CACHE_BUFFER_SIZE];
		let mut i = 0;
		let mut write_to_cache = true;
		let mut term = false;
		let mut len_sum = 0;
		debug!("rangestart={},rangeend={}", range_start, range_end)?;
		let mime_type = ctx.mime_rev_lookup.get(&extension).unwrap_or(&u32::MAX);

		if code != 304 {
			loop {
				let mut blen = 0;
				loop {
					let cur = buf_reader.read(&mut buf[blen..])?;
					if cur <= 0 {
						term = true;
						break;
					}
					blen += cur;

					if blen == CACHE_BUFFER_SIZE {
						break;
					}
				}

				debug!("i={},blen={}", i, blen)?;

				if !write_error {
					Self::range_write(
						range_start,
						range_end,
						&buf,
						len_sum,
						blen,
						&mut write_handle,
					)?;
					len_sum += blen;
				}

				if blen > 0 {
					let mut cache = cache.wlock()?;
					if i == 0 {
						write_to_cache = (**cache.guard()).write_metadata(
							&fpath,
							try_into!(len)?,
							last_modified,
							*mime_type,
						)?;
					}

					if write_to_cache {
						(**cache.guard()).write_block(
							&fpath,
							i,
							&try_into!(buf[0..CACHE_BUFFER_SIZE])?,
						)?;
					}
				}

				if term {
					break;
				}

				i += 1;
			}
		}

		debug!("write_error={}", write_error)?;

		if !keep_alive {
			write_handle.close()?;
		}

		Ok(())
	}

	pub(crate) fn range_write(
		range_start: usize,
		range_end: usize,
		buf: &Vec<u8>,
		len_sum: usize,
		blen: usize,
		write_handle: &mut WriteHandle,
	) -> Result<bool, Error> {
		let mut write_error = false;
		let start = if len_sum >= range_start {
			0
		} else {
			range_start - len_sum
		};
		let end = if range_end < len_sum + blen {
			range_end - len_sum
		} else {
			blen
		};

		debug!(
			"start={},end={},blen={},len_sum={},range_end={}",
			start, end, blen, len_sum, range_end
		)?;

		if start < end {
			match write_handle.write(&buf[start..end]) {
				Ok(_) => {}
				Err(_) => write_error = true,
			}
		}

		Ok(write_error)
	}

	fn build_request_headers<'a>(
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
		let mut version = HttpVersion::UNKNOWN;
		let mut header_count = 0;
		let mut headers = [HttpHeader::default(); 100];
		let mut connection = ConnectionType::KeepAlive;
		let mut range_start = 0;
		let mut range_end = usize::MAX;
		let mut if_none_match = "".to_string();
		let mut if_modified_since = "".to_string();

		debug!("count={}", count)?;
		let mut host = "".to_string();
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

				let mut end_version = 0;
				let mut start_version = 0;
				for i in start_uri..req.len() {
					if req[start + i] == ' ' as u8 {
						end_uri = start + i;
						start_version = start + i + 1;
					}
					if req[start + i] == '\r' as u8 || req[start + i] == '\n' as u8 {
						end_version = start + i;
						break;
					}
				}

				debug!("start_v={},end_v={}", start_version, end_version)?;
				if end_version > start_version && start_version != 0 {
					// try to get version
					let version_str =
						std::str::from_utf8(&req[start_version..end_version]).unwrap_or("");
					if version_str == "HTTP/1.1" {
						version = HttpVersion::HTTP11;
					} else if version_str == "HTTP/1.0" {
						version = HttpVersion::HTTP10;
					} else {
						version = HttpVersion::OTHER;
					}
				}
			} else if id == SUFFIX_TREE_HEADER_ID {
				if header_count < headers.len() {
					headers[header_count].start_header_name = start + 2;
					headers[header_count].end_header_name = end - 2;
					headers[header_count].start_header_value = end;
					headers[header_count].end_header_value = 0;

					for i in end..req.len() {
						if req[i] == '\n' as u8 || req[i] == '\r' as u8 {
							headers[header_count].end_header_value = i;
							break;
						}
					}

					if headers[header_count].end_header_name
						> headers[header_count].start_header_name
						&& headers[header_count].end_header_value
							> headers[header_count].start_header_value
						&& headers[header_count].start_header_value < req.len()
					{
						if &req[headers[header_count].start_header_name
							..headers[header_count].end_header_name]
							== HOST_BYTES
						{
							host = from_utf8(
								&req[headers[header_count].start_header_value
									..headers[header_count].end_header_value],
							)
							.unwrap_or("")
							.to_string();
						} else if &req[headers[header_count].start_header_name
							..headers[header_count].end_header_name]
							== CONNECTION_BYTES
						{
							if &req[headers[header_count].start_header_value
								..headers[header_count].end_header_value]
								!= KEEP_ALIVE_BYTES
							{
								connection = ConnectionType::CLOSE;
							}
						} else if &req[headers[header_count].start_header_name
							..headers[header_count].end_header_name]
							== RANGE_BYTES
						{
							let range_value = from_utf8(
								&req[headers[header_count].start_header_value
									..headers[header_count].end_header_value],
							)
							.unwrap_or("");

							if range_value.starts_with("bytes=") {
								let range_split: Vec<&str> = range_value.split('=').collect();
								let range_split: Vec<&str> = range_split[1].split('-').collect();
								range_start = range_split[0].parse()?;
								range_end = range_split[1].parse()?;
								range_end += 1;
							}
						} else if &req[headers[header_count].start_header_name
							..headers[header_count].end_header_name]
							== IF_NONE_MATCH_BYTES
						{
							if_none_match = from_utf8(
								&req[headers[header_count].start_header_value
									..headers[header_count].end_header_value],
							)
							.unwrap_or("")
							.to_string();
						} else if &req[headers[header_count].start_header_name
							..headers[header_count].end_header_name]
							== IF_MODIFIED_SINCE_BYTES
						{
							if_modified_since = from_utf8(
								&req[headers[header_count].start_header_value
									..headers[header_count].end_header_value],
							)
							.unwrap_or("")
							.to_string();
						}
						header_count += 1;
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
				version,
				headers,
				header_count,
				host,
				connection,
				range_start,
				range_end,
				if_none_match,
				if_modified_since,
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
		cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
		headers: &HttpHeaders,
		ctx: &HttpContext,
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
				cache,
				headers,
				ctx,
			)?;
		} else {
			Self::process_error(
				config,
				path,
				conn_data,
				instance,
				400,
				"Bad Request",
				cache,
				headers,
				ctx,
			)?;
		}
		conn_data.write_handle().close()?;
		Ok(())
	}

	fn update_time(
		now: u128,
		ctx: &mut HttpContext,
		conn_data: &mut ConnectionData,
	) -> Result<(), Error> {
		ctx.connections.insert(
			conn_data.get_connection_id(),
			(
				conn_data.write_handle().write_state()?,
				now,
				conn_data.tid(),
			),
		);
		Ok(())
	}

	fn process_on_read(
		config: &HttpConfig,
		cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
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
		let ctx = Self::build_ctx(ctx, config)?;
		ctx.now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
		Self::update_time(ctx.now, ctx, conn_data)?;

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
			let headers = Self::build_request_headers(
				&req,
				start,
				ctx.matches,
				&mut ctx.suffix_tree,
				(slab_offset as usize + (slab_count - 1) * READ_SLAB_DATA_SIZE).into(),
			);

			let headers = match headers {
				Ok(headers) => headers,
				Err(e) => {
					// build a mock header. only thing that matters is host and we
					// can put something that triggers the default instance.
					let termination_point = 0;
					let start = 0;
					let req = &"Host_".to_string().as_bytes().to_vec();
					let start_uri = 0;
					let end_uri = 0;
					let http_request_type = HttpRequestType::GET;
					let headers = [HttpHeader::default(); 100];
					let header_count = 0;
					let version = HttpVersion::UNKNOWN;
					let host = "".to_string();
					let connection = ConnectionType::KeepAlive;
					let headers = HttpHeaders {
						termination_point,
						start,
						req,
						start_uri,
						end_uri,
						http_request_type,
						version,
						headers,
						header_count,
						host,
						connection,
						range_start: 0,
						range_end: usize::MAX,
						if_none_match: "".to_string(),
						if_modified_since: "".to_string(),
					};
					Self::header_error(
						config,
						"".to_string(),
						conn_data,
						attachment,
						e,
						cache,
						&headers,
						ctx,
					)?;
					return Ok(());
				}
			};

			debug!("Request type = {:?}", headers.http_request_type)?;

			termination_sum += headers.termination_point;

			debug!("term point = {}", headers.termination_point)?;
			if headers.termination_point == 0 {
				break;
			} else {
				if headers.version != HttpVersion::HTTP10 && headers.host()?.len() == 0 {
					Self::header_error(
						config,
						"".to_string(),
						conn_data,
						attachment,
						err!(ErrKind::Http, "Host not specified on HTTP/1.1+"),
						cache.clone(),
						&headers,
						ctx,
					)?;
					return Ok(());
				}
				let path = match headers.path() {
					Ok(path) => path,
					Err(e) => {
						Self::header_error(
							config,
							"".to_string(),
							conn_data,
							attachment,
							e,
							cache,
							&headers,
							ctx,
						)?;
						return Ok(());
					}
				};

				start = headers.termination_point;
				last_term = headers.termination_point;

				let mut is_callback = false;
				match attachment.callback {
					Some(callback) => {
						if attachment.callback_mappings.contains(&path) {
							is_callback = true;
							callback(&headers, &config, &attachment, conn_data)?;
						} else if path.contains(r".") {
							let pos = path.chars().rev().position(|c| c == '.').unwrap();
							let len = path.len();
							let suffix = path.substring(len - pos, len).to_string();
							if attachment.callback_extensions.contains(&suffix) {
								is_callback = true;
								callback(&headers, &config, &attachment, conn_data)?;
							}
						}
					}
					None => {}
				}

				let mut cache_hit = false;
				if !is_callback {
					cache_hit = Self::process_file(
						config,
						cache.clone(),
						path.clone(),
						conn_data,
						attachment,
						&headers,
						ctx,
					)?;
				}

				if config.debug {
					let empty = "".to_string();
					let query = headers.query().unwrap_or("".to_string());
					let extension = headers.extension().unwrap_or(empty);
					let header_count = headers.header_count().unwrap_or(0);
					info!(
						"uri={},query={},extension={},method={:?},version={:?},header_count={},cache_hit={},has_range={},range_start={},range_end={},if_none_match={},if_modified_since={}",
						path,
						query,
                                                extension,
						headers.http_request_type().unwrap_or(&HttpRequestType::GET),
						headers.version().unwrap_or(&HttpVersion::UNKNOWN),
						header_count,
						cache_hit,
                                                headers.has_range().unwrap_or(false),
                                                headers.range_start().unwrap_or(0),
                                                headers.range_end().unwrap_or(usize::MAX),
                                                headers.if_none_match().unwrap_or(&"".to_string()),
                                                headers.if_modified_since().unwrap_or(&"".to_string()),
					)?;
					info!("{}", SEPARATOR_LINE)?;

					for i in 0..header_count {
						info!(
							"   header[{}] = ['{}']",
							headers.header_name(i).unwrap_or("".to_string()),
							headers.header_value(i).unwrap_or("".to_string())
						)?;
					}
					info!("{}", SEPARATOR_LINE)?;
				}
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
		conn_data: &mut ConnectionData,
		ctx: &mut ThreadContext,
		config: &HttpConfig,
	) -> Result<(), Error> {
		let ctx = Self::build_ctx(ctx, config)?;
		ctx.now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
		Self::update_time(ctx.now, ctx, conn_data)?;
		Ok(())
	}

	fn process_on_close(
		_conn_data: &mut ConnectionData,
		_ctx: &mut ThreadContext,
		_config: &HttpConfig,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_panic(_ctx: &mut ThreadContext, _e: Box<dyn Any + Send>) -> Result<(), Error> {
		Ok(())
	}

	fn process_housekeeper(
		ctx: &mut ThreadContext,
		mut event_handler_data: Array<Box<dyn LockBox<EventHandlerData>>>,
		config: &HttpConfig,
	) -> Result<(), Error> {
		let ctx = Self::build_ctx(ctx, config)?;
		let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
		let mut to_remove = vec![];
		for (k, v) in &ctx.connections {
			let diff = now.saturating_sub(v.1);
			if diff >= config.idle_timeout {
				to_remove.push((k.clone(), v.0.clone(), v.2.clone()));
			}
		}

		for mut rem in to_remove {
			ctx.connections.remove(&rem.0);
			let mut ch = CloseHandle::new(&mut rem.1, rem.0, &mut event_handler_data[rem.2]);
			ch.close()?;
		}
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
			let mut nmap = HashMap::new();
			match &mut self.config.instances[i].instance_type {
				Plain(instance_type) => {
					for (hostname, http_dir) in &instance_type.http_dir_map {
						nmap.insert(
							hostname.clone(),
							Self::normalize_path(http_dir.replace("~", &home_dir))
								.into_os_string()
								.into_string()?,
						);
					}
					instance_type.http_dir_map = nmap;
				}
				Tls(instance_type) => {
					for (hostname, http_dir) in &instance_type.http_dir_map {
						nmap.insert(
							hostname.clone(),
							Self::normalize_path(http_dir.replace("~", &home_dir))
								.into_os_string()
								.into_string()?,
						);
					}
					instance_type.http_dir_map = nmap;
				}
			}
		}

		let mut evh = Builder::build_evh(self.config.evh_config.clone())?;
		let event_handler_data = evh.event_handler_data()?;
		let config = &self.config;
		let config = config.clone();
		let config2 = config.clone();
		let config3 = config.clone();
		let config4 = config.clone();
		let cache = self.cache.clone();

		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config, cache.clone(), conn_data, ctx, attach)
		})?;
		evh.set_on_accept(move |conn_data, ctx| Self::process_on_accept(conn_data, ctx, &config2))?;
		evh.set_on_close(move |conn_data, ctx| Self::process_on_close(conn_data, ctx, &config3))?;
		evh.set_on_panic(move |ctx, e| Self::process_on_panic(ctx, e))?;
		evh.set_housekeeper(move |ctx| {
			Self::process_housekeeper(ctx, event_handler_data.clone(), &config4)
		})?;

		evh.start()?;

		for instance in &self.config.instances {
			let port = instance.port;
			let addr = &instance.addr;

			debug!("creating listener for {}", port)?;
			let addr = format!("{}:{}", addr, port);
			let handles = create_listeners(self.config.evh_config.threads, &addr, 10, false)?;

			let tls_config = match &instance.instance_type {
				Plain(_) => None,
				Tls(config) => {
					let certificates_file = config.cert_file.clone();
					let private_key_file = config.privkey_file.clone();
					Some(TlsServerConfig {
						certificates_file,
						private_key_file,
					})
				}
			};

			let sc = ServerConnection {
				tls_config,
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
	use crate::{HttpConfig, HttpInstance, HttpInstanceType, HttpServer, PlainConfig};
	use bmw_err::*;
	use bmw_log::*;
	use bmw_test::port::pick_free_port;
	use bmw_test::testdir::{setup_test_dir, tear_down_test_dir};
	use std::collections::HashMap;
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
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			..Default::default()
		};
		let mut http = HttpServerImpl::new(&config)?;
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
		let mut buf = [0; 512];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 266);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		client.write(b"POST /def1.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		let mut buf = [0; 512];
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 267);

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
				instance_type: HttpInstanceType::Plain(PlainConfig {
					http_dir_map: HashMap::from([("*".to_string(), test_dir.to_string())]),
				}),
				..Default::default()
			}],
			server_version: "test1".to_string(),
			..Default::default()
		};
		let mut http = HttpServerImpl::new(&config)?;
		http.start()?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let addr = &format!("127.0.0.1:{}", port)[..];
		info!("addr={}", addr)?;
		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));

		client.write(b"GET /foo.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let mut buf = [0; 512];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 266);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		let mut client = TcpStream::connect(addr)?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		client.write(b"POST /foo.html HTTP/1.1\r\nHost: localhost\r\nUser-agent: test\r\n\r\n")?;
		std::thread::sleep(std::time::Duration::from_millis(1_000));
		let mut buf = [0; 512];
		let len = client.read(&mut buf)?;
		let data = from_utf8(&buf)?;
		info!("len={}", len)?;
		info!("data='{}'", data)?;
		assert_eq!(len, 266);

		std::thread::sleep(std::time::Duration::from_millis(1_000));

		tear_down_test_dir(test_dir)?;

		Ok(())
	}
}
