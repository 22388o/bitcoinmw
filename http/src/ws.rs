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

use crate::constants::*;
use crate::types::{
	FrameHeader, FrameType, WebSocketClientImpl, WebSocketConnectionImpl, WebSocketConnectionState,
	WebSocketMessage, WebSocketMessageType,
};
use crate::{
	HttpConfig, HttpInstance, WebSocketClient, WebSocketClientConfig, WebSocketConnection,
	WebSocketConnectionConfig, WebSocketData, WebSocketHandle, WebSocketHandler,
};
use bmw_deps::base64;
use bmw_deps::byteorder::{BigEndian, ByteOrder};
use bmw_deps::rand_core::{OsRng, RngCore};
use bmw_deps::sha1::{Digest, Sha1};
use bmw_err::*;
use bmw_evh::{
	tcp_stream_to_handle, AttachmentHolder, Builder, ClientConnection, ConnData, ConnectionData,
	EventHandler, EventHandlerConfig, EventHandlerData, ThreadContext, TlsClientConfig,
	WriteHandle, READ_SLAB_DATA_SIZE,
};
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::net::TcpStream;
use std::str::from_utf8;

info!();

fn websocket_message_to_vec(ws: &WebSocketMessage, mask: bool) -> Result<Vec<u8>, Error> {
	let mut ret: Vec<u8> = vec![];
	ret.resize(2, 0u8);

	// always set fin bit for now. No fragmentation.
	ret[0] = match ws.mtype {
		WebSocketMessageType::Text => 0x80 | 0x1,
		WebSocketMessageType::Binary => 0x80 | 0x2,
		WebSocketMessageType::Close => 0x80 | 0x8,
		WebSocketMessageType::Ping => 0x80 | 0x9,
		WebSocketMessageType::Pong => 0x80 | 0xA,
		_ => 0x80 | 0x1, // should not happen
	};

	ret[1] = if mask { 0x80 } else { 0x00 };

	let mut masking_bytes = [0u8; 4];
	let payload_len = ws.payload.len();
	let start_content = if payload_len < 126 {
		ret[1] |= payload_len as u8;
		if mask {
			ret.resize(6 + payload_len, 0u8);
			BigEndian::write_u32(&mut ret[2..6], OsRng.next_u32());
			masking_bytes.clone_from_slice(&ret[2..6]);
			6
		} else {
			ret.resize(2 + payload_len, 0u8);
			2
		}
	} else if payload_len <= u16::MAX.into() {
		ret[1] |= 126;
		if mask {
			ret.resize(8 + payload_len, 0u8);
		} else {
			ret.resize(4 + payload_len, 0u8);
		}
		BigEndian::write_u16(&mut ret[2..4], payload_len.try_into()?);
		if mask {
			BigEndian::write_u32(&mut ret[4..8], OsRng.next_u32());
			masking_bytes.clone_from_slice(&ret[4..8]);
			8
		} else {
			4
		}
	} else {
		ret[1] |= 127;
		if mask {
			ret.resize(14 + payload_len, 0u8);
		} else {
			ret.resize(10 + payload_len, 0u8);
		}
		BigEndian::write_u64(&mut ret[2..10], payload_len.try_into()?);
		if mask {
			BigEndian::write_u32(&mut ret[10..14], OsRng.next_u32());
			masking_bytes.clone_from_slice(&ret[10..14]);
			14
		} else {
			10
		}
	};

	ret[start_content..].clone_from_slice(&ws.payload);

	if mask {
		let mut i = 0;
		let ret_len = ret.len();
		loop {
			if i + start_content >= ret_len {
				break;
			}

			let j = i % 4;
			ret[i + start_content] = ret[i + start_content] ^ masking_bytes[j];
			i += 1;
		}
	}

	Ok(ret)
}

impl WebSocketHandle {
	fn new(write_handle: WriteHandle) -> Self {
		Self { write_handle }
	}
	pub fn send(&mut self, message: &WebSocketMessage) -> Result<(), Error> {
		self.write_handle
			.write(&websocket_message_to_vec(message, false)?)
	}
	pub fn send_masked(&mut self, message: &WebSocketMessage) -> Result<(), Error> {
		self.write_handle
			.write(&websocket_message_to_vec(message, true)?)
	}
	pub fn close(&mut self) -> Result<(), Error> {
		self.write_handle.close()
	}
}

pub(crate) fn send_websocket_handshake_response(
	wh: &mut WriteHandle,
	key: String,
	mut sha1: Sha1,
	proto: Option<String>,
	proto_required: bool,
) -> Result<(), Error> {
	let hash = format!("{}{}", key, WEBSOCKET_GUID);
	sha1.update(hash.as_bytes());
	let msg = format!(
		"HTTP/1.1 101 Switching Protocols\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n{}\
Sec-WebSocket-Accept: {}\r\n\r\n",
		match proto {
			Some(proto) => format!("Sec-WebSocket-Protocol: {}\r\n", proto),
			None =>
				if proto_required {
					"Sec-WebSocket-Protocol: nomatches\r\n".to_string()
				} else {
					"".to_string()
				},
		},
		base64::encode(&sha1.finalize()[..]),
	);
	let response = msg.as_bytes();
	wh.write(response)?;
	Ok(())
}

pub(crate) fn process_websocket_data(
	req: &[u8],
	conn_data: &mut ConnectionData,
	instance: &HttpInstance,
	config: &HttpConfig,
	websocket_data: &WebSocketData,
	thread_context: &mut ThreadContext,
) -> Result<usize, Error> {
	debug!("proc data: {:?}", req)?;
	let (messages, termination_point) = build_messages(req)?;
	let mut ws_handle = WebSocketHandle {
		write_handle: conn_data.write_handle(),
	};
	for message in &messages {
		match instance.websocket_handler {
			Some(ws_handler) => ws_handler(
				message,
				config,
				instance,
				&mut ws_handle,
				websocket_data,
				thread_context,
			)?,
			None => {
				warn!("got websocket request but no handler was specified!")?;
			}
		}
	}
	Ok(termination_point)
}

fn get_frame_header_info(buffer: &[u8]) -> Result<Option<FrameHeader>, Error> {
	let len = buffer.len();
	let start_content;
	if len < 2 {
		// not enough to even start parsing
		debug!("return none 1, len = {}", len)?;
		return Ok(None);
	}

	// get basic bits from the first byte
	let fin = (buffer[0] & FIN_BIT) != 0;
	let op1 = (buffer[0] & OP_CODE_MASK1) != 0;
	let op2 = (buffer[0] & OP_CODE_MASK2) != 0;
	let op3 = (buffer[0] & OP_CODE_MASK3) != 0;
	let op4 = (buffer[0] & OP_CODE_MASK4) != 0;

	debug!(
		"fin={},op1={},op2={},op3={},op4={}",
		fin, op1, op2, op3, op4
	)?;

	// get type based on op_codes
	let ftype = if !op1 && !op2 && !op3 && !op4 {
		FrameType::Continuation
	} else if !op1 && !op2 && !op3 && op4 {
		FrameType::Text
	} else if !op1 && !op2 && op3 && !op4 {
		FrameType::Binary
	} else if op1 && !op2 && !op3 && !op4 {
		FrameType::Close
	} else if op1 && !op2 && !op3 && op4 {
		FrameType::Ping
	} else if op1 && !op2 && op3 && !op4 {
		FrameType::Pong
	} else {
		// other op codes not supported
		return Err(err!(ErrKind::IllegalArgument, "invalid websocket opcode"));
	};

	// get bit indicating masking.
	let mask = (buffer[1] & MASK_BIT) != 0;
	// get 7 bit size, then 16 bit, then 64. See rfc:
	// https://datatracker.ietf.org/doc/html/rfc6455
	let first_payload_bits = buffer[1] & !MASK_BIT;

	let payload_len: usize = if first_payload_bits == 126 {
		if len < 4 {
			debug!("return none 2")?;
			return Ok(None);
		}
		BigEndian::read_u16(&buffer[2..4]).try_into()?
	} else if first_payload_bits == 127 {
		if len < 10 {
			debug!("return none 3")?;
			return Ok(None);
		}
		BigEndian::read_u64(&buffer[2..10]).try_into()?
	} else {
		let payload_len: usize = first_payload_bits.into();
		payload_len
	}
	.into();

	let masking_key = if !mask {
		if first_payload_bits == 126 {
			start_content = 4;
			if len < 4 + payload_len {
				debug!("return none 4")?;
				return Ok(None);
			}
		} else if first_payload_bits == 127 {
			start_content = 10;
			if len < 10 + payload_len {
				debug!("return none 5")?;
				return Ok(None);
			}
		} else {
			start_content = 2;
			if len < 2 + payload_len {
				debug!("return none 6")?;
				return Ok(None);
			}
		}
		0
	} else if first_payload_bits == 126 {
		start_content = 8;
		if len < 8 + payload_len {
			debug!("return none 7")?;
			return Ok(None);
		}
		BigEndian::read_u32(&buffer[4..8])
	} else if first_payload_bits == 127 {
		start_content = 14;
		if len < 14 + payload_len {
			debug!("return none 8: payload_len={},len={}", payload_len, len)?;
			return Ok(None);
		}
		BigEndian::read_u32(&buffer[10..14])
	} else {
		start_content = 6;
		if len < 6 + payload_len {
			debug!("return none 9")?;
			return Ok(None);
		}
		BigEndian::read_u32(&buffer[2..6])
	};

	Ok(Some(FrameHeader {
		ftype,
		mask,
		fin,
		payload_len,
		masking_key,
		start_content,
	}))
}

pub fn build_messages(buffer: &[u8]) -> Result<(Vec<WebSocketMessage>, usize), Error> {
	let mut ret = vec![];
	let mut headers = vec![];
	let mut offset = 0;

	loop {
		let header = get_frame_header_info(&mut buffer[offset..].to_vec())?;

		debug!("found a header: {:?}", header)?;

		match header {
			Some(header) => {
				let fin = header.fin;
				let payload_len = header.payload_len;
				let start_content = header.start_content;

				let end_content = start_content + payload_len;

				if fin {
					// add this header to our framed headers
					headers.push((header, offset));
					// process the existing frames
					let message = build_message(headers, buffer)?;
					ret.push(message);
					headers = vec![];
				} else {
					match header.ftype {
						FrameType::Ping => {
							return Err(err!(
								ErrKind::IllegalArgument,
								format!("frametype '{:?}' must set fin = true.", header.ftype)
							));
						}
						_ => {}
					}
					// add this header to our framed headers
					headers.push((header, offset));
				}

				offset += end_content;
			}
			None => {
				debug!("not enough data. Returning none")?;
				// we don't have enough data to continue, so break
				break;
			}
		}
	}
	Ok((ret, offset))
}

fn build_message(
	frames: Vec<(FrameHeader, usize)>,
	buffer: &[u8],
) -> Result<WebSocketMessage, Error> {
	// append each frame of the message content into a single message for processing
	let mut payload = vec![];

	let mut masking_bytes = [0u8; 4];
	let mut mtype = WebSocketMessageType::Text;
	let mut itt = 0;

	for (header, offset) in frames {
		let start = header.start_content + offset;
		let end = header.payload_len + start;
		let data = &buffer[start..end];
		let mut i = 0;

		BigEndian::write_u32(&mut masking_bytes, header.masking_key);
		for d in data {
			let j = i % 4;
			let nbyte = d ^ masking_bytes[j];
			payload.push(nbyte);
			i += 1;
		}

		// take the type of the first frame
		if itt == 0 {
			mtype = match header.ftype {
				FrameType::Text => WebSocketMessageType::Text,
				FrameType::Binary => WebSocketMessageType::Binary,
				FrameType::Ping => WebSocketMessageType::Ping,
				FrameType::Pong => WebSocketMessageType::Pong,
				FrameType::Close => WebSocketMessageType::Close,
				_ => WebSocketMessageType::Text,
			};
		}
		itt += 1;
	}

	Ok(WebSocketMessage { mtype, payload })
}

impl Default for WebSocketClientConfig {
	fn default() -> Self {
		let evhc = EventHandlerConfig::default();
		Self {
			debug: false,
			evh_max_handles_per_thread: evhc.max_handles_per_thread,
			evh_threads: evhc.threads,
			evh_sync_channel_size: evhc.sync_channel_size,
			evh_write_queue_size: evhc.write_queue_size,
			evh_nhandles_queue_size: evhc.nhandles_queue_size,
			evh_max_events_in: evhc.max_events_in,
			evh_housekeeping_frequency_millis: evhc.housekeeping_frequency_millis,
			evh_read_slab_count: evhc.read_slab_count,
			evh_max_events: evhc.max_events,
		}
	}
}

impl Default for WebSocketConnectionConfig {
	fn default() -> Self {
		Self {
			tls: false,
			host: "127.0.0.1".to_string(),
			port: 8080,
			masked: false,
			full_chain_cert_file: None,
			protocols: vec![],
			path: "/".to_string(),
		}
	}
}

impl WebSocketClient for WebSocketClientImpl {
	fn connect(
		&mut self,
		config: &WebSocketConnectionConfig,
		handler: WebSocketHandler,
	) -> Result<Box<dyn WebSocketConnection + Send + Sync>, Error> {
		let host = config.host.clone();
		let port = config.port;
		let addr = format!("{}:{}", host, port);
		let tcp_stream = TcpStream::connect(addr.clone())?;
		tcp_stream.set_nonblocking(true)?;

		let client_connection = ClientConnection {
			handle: tcp_stream_to_handle(tcp_stream)?,
			tls_config: if config.tls {
				Some(TlsClientConfig {
					sni_host: host,
					trusted_cert_full_chain_file: config.full_chain_cert_file.clone(),
				})
			} else {
				None
			},
		};

		let websocket_connection_state = WebSocketConnectionState::new(handler);
		let mut wh = self
			.controller
			.add_client(client_connection, Box::new(websocket_connection_state))?;
		let wsci = WebSocketConnectionImpl::new(config.clone(), wh.clone())?;

		let mut bytes = [0u8; 16];
		random_bytes(&mut bytes);
		let key = base64::encode(bytes);

		wh.write(
			format!(
				"GET {} HTTP/1.1\r\nHost: {}\r\nSec-WebSocket-Key: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
				config.path, addr, key
			)
			.as_bytes(),
		)?;

		Ok(Box::new(wsci))
	}

	fn stop(&mut self) -> Result<(), Error> {
		self.controller.stop()
	}
}

impl WebSocketClientImpl {
	pub(crate) fn new(config: &WebSocketClientConfig) -> Result<Self, Error> {
		let evh_config = EventHandlerConfig {
			threads: config.evh_threads,
			max_handles_per_thread: config.evh_max_handles_per_thread,
			sync_channel_size: config.evh_sync_channel_size,
			write_queue_size: config.evh_write_queue_size,
			nhandles_queue_size: config.evh_nhandles_queue_size,
			max_events_in: config.evh_max_events_in,
			housekeeping_frequency_millis: config.evh_housekeeping_frequency_millis,
			max_events: config.evh_max_events,
			read_slab_count: config.evh_read_slab_count,
		};
		let mut evh = Builder::build_evh(evh_config)?;
		let event_handler_data = evh.event_handler_data()?;

		let config = config.clone();
		let config2 = config.clone();
		let config3 = config.clone();
		let config4 = config.clone();
		let config5 = config.clone();
		evh.set_on_read(move |conn_data, ctx, attach| {
			Self::process_on_read(&config2, conn_data, ctx, attach)
		})?;
		evh.set_on_accept(move |conn_data, ctx| Self::process_on_accept(&config3, conn_data, ctx))?;
		evh.set_on_close(move |conn_data, ctx| Self::process_on_close(&config4, conn_data, ctx))?;
		evh.set_on_panic(move |ctx, e| Self::process_on_panic(ctx, e))?;
		evh.set_housekeeper(move |ctx| {
			Self::process_housekeeper(config5.clone(), ctx, event_handler_data.clone())
		})?;

		evh.start()?;

		Ok(Self {
			controller: evh.event_handler_controller()?,
		})
	}

	fn process_on_read(
		config: &WebSocketClientConfig,
		conn_data: &mut ConnectionData,
		ctx: &ThreadContext,
		attachment: Option<AttachmentHolder>,
	) -> Result<(), Error> {
		match attachment {
			Some(mut attachment) => {
				let mut state = attachment.attachment.wlock()?;
				let guard = state.guard();
				match (**guard).downcast_mut::<WebSocketConnectionState>() {
					Some(state) => Self::process_on_read_w_state(config, conn_data, ctx, state),
					None => Err(err!(
						ErrKind::Http,
						"Websocket client did not have the correct attachment"
					)),
				}
			}
			None => Err(err!(
				ErrKind::Http,
				"Websocket client did not have attachment"
			)),
		}
	}

	fn process_on_read_w_state(
		config: &WebSocketClientConfig,
		conn_data: &mut ConnectionData,
		ctx: &ThreadContext,
		state: &mut WebSocketConnectionState,
	) -> Result<(), Error> {
		let first_slab = conn_data.first_slab();
		let last_slab = conn_data.last_slab();
		let slab_offset = conn_data.slab_offset();
		let slab_start = 0;

		let mut wh = conn_data.write_handle();

		conn_data.borrow_slab_allocator(move |sa| {
			let mut slab_id = first_slab;

			loop {
				if slab_id >= u32::MAX {
					break;
				}

				let slab = sa.get(slab_id.try_into()?)?;
				let slab_bytes = slab.get();
				let offset = if slab_id == last_slab {
					slab_offset as usize
				} else {
					READ_SLAB_DATA_SIZE
				};

				let start = if slab_id == first_slab { slab_start } else { 0 };
				state.buffer.extend(&slab_bytes[start..offset]);

				if slab_id == last_slab {
					break;
				}

				slab_id = u32::from_be_bytes(try_into!(
					slab_bytes[READ_SLAB_DATA_SIZE..READ_SLAB_DATA_SIZE + 4]
				)?);
			}

			if !state.switched {
				Self::process_non_switched(config, &mut wh, ctx, state)?;
				Self::process_switched(config, &mut wh, ctx, state)?;
			} else {
				debug!("already switched")?;
				Self::process_switched(config, &mut wh, ctx, state)?;
			}
			Ok(())
		})?;

		conn_data.clear_through(last_slab)?;

		Ok(())
	}

	fn process_non_switched(
		_config: &WebSocketClientConfig,
		_wh: &mut WriteHandle,
		_ctx: &ThreadContext,
		state: &mut WebSocketConnectionState,
	) -> Result<(), Error> {
		let buffer_len = state.buffer.len();
		let res = from_utf8(&state.buffer)?;
		let mut end_point = 0;
		for i in 3..buffer_len {
			if state.buffer[i - 3] == b'\r'
				&& state.buffer[i - 2] == b'\n'
				&& state.buffer[i - 1] == b'\r'
				&& state.buffer[i] == b'\n'
			{
				// end the switch
				end_point = i + 1;
			}
		}
		if end_point > 0 {
			debug!("res='{}'", &res[0..end_point])?;
			state.buffer = state.buffer.drain(end_point..).collect();
			state.buffer.shrink_to_fit();
			state.switched = true;
		}
		Ok(())
	}

	fn process_switched(
		_config: &WebSocketClientConfig,
		wh: &mut WriteHandle,
		_ctx: &ThreadContext,
		state: &mut WebSocketConnectionState,
	) -> Result<(), Error> {
		let messages = build_messages(&state.buffer)?;
		let mut wsh = WebSocketHandle::new(wh.clone());
		for message in messages.0 {
			debug!("Got a message: {:?}", message)?;
			match (state.handler)(&message, &mut wsh) {
				Ok(_) => {}
				Err(e) => warn!("websocket handler generated error: {}", e)?,
			}
		}
		state.buffer = state.buffer.drain(messages.1..).collect();
		Ok(())
	}

	fn process_on_accept(
		_config: &WebSocketClientConfig,
		_conn_data: &mut ConnectionData,
		_ctx: &ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_close(
		_config: &WebSocketClientConfig,
		_conn_data: &mut ConnectionData,
		_ctx: &ThreadContext,
	) -> Result<(), Error> {
		Ok(())
	}

	fn process_on_panic(_ctx: &ThreadContext, _e: Box<dyn Any + Send>) -> Result<(), Error> {
		Ok(())
	}

	fn process_housekeeper(
		_config: WebSocketClientConfig,
		_ctx: &ThreadContext,
		_evh_data: Array<Box<dyn LockBox<EventHandlerData>>>,
	) -> Result<(), Error> {
		Ok(())
	}
}

impl WebSocketConnectionState {
	fn new(handler: WebSocketHandler) -> Self {
		Self {
			switched: false,
			buffer: vec![],
			handler,
		}
	}
}

impl WebSocketConnection for WebSocketConnectionImpl {
	fn send(&mut self, message: &WebSocketMessage) -> Result<(), Error> {
		let msg = websocket_message_to_vec(message, true)?;
		self.wh.write(&msg)?;
		Ok(())
	}
	fn close(&mut self) -> Result<(), Error> {
		let message = WebSocketMessage {
			mtype: WebSocketMessageType::Close,
			payload: vec![],
		};
		let msg = websocket_message_to_vec(&message, true)?;
		self.wh.write(&msg)?;
		self.wh.close()
	}
}

impl WebSocketConnectionImpl {
	pub(crate) fn new(_config: WebSocketConnectionConfig, wh: WriteHandle) -> Result<Self, Error> {
		Ok(Self { wh })
	}
}
