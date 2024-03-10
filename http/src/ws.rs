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
use crate::types::{FrameHeader, FrameType, WebSocketMessage, WebSocketMessageType};
use crate::{HttpConfig, HttpInstance, WebSocketData, WebSocketHandle};
use bmw_deps::base64;
use bmw_deps::byteorder::{BigEndian, ByteOrder};
use bmw_deps::rand_core::{OsRng, RngCore};
use bmw_deps::sha1::{Digest, Sha1};
use bmw_err::*;
use bmw_evh::{ConnData, ConnectionData, ThreadContext, WriteHandle};
use bmw_log::*;

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
