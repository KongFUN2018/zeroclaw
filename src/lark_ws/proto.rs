//! Protocol Buffers definitions for Lark WebSocket frames
//!
//! This module provides the binary encoding/decoding for the Lark WebSocket protocol.
//! Based on the official pbbp2.proto specification.

use bytes::{Buf, BufMut, BytesMut};

/// Header represents a key-value pair in frame headers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub key: String,
    pub value: String,
}

impl Header {
    /// Encode header to protobuf format
    pub fn encode(&self, buf: &mut BytesMut) {
        // Tag for field 1 (key): 0x0A (field 1, wire type 2)
        buf.put_u8(0x0A);
        encode_string(self.key.as_bytes(), buf);

        // Tag for field 2 (value): 0x12 (field 2, wire type 2)
        buf.put_u8(0x12);
        encode_string(self.value.as_bytes(), buf);
    }

    /// Decode header from protobuf format
    pub fn decode(buf: &mut BytesMut) -> Result<Option<Self>, String> {
        let mut key = None;
        let mut value = None;

        // Read all fields until buffer is empty
        // Note: This is called with a pre-sized buffer containing just this header
        while buf.has_remaining() {
            let tag = decode_varint(buf)?;
            let field_num = tag >> 3;
            let wire_type = tag & 0x07;

            match field_num {
                1 => {
                    // key field
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for key: {}", wire_type));
                    }
                    let len = decode_varint(buf)? as usize;
                    if buf.remaining() < len {
                        return Err("Buffer underflow reading key".to_string());
                    }
                    let mut bytes = vec![0u8; len];
                    buf.copy_to_slice(&mut bytes);
                    key = Some(String::from_utf8(bytes)
                        .map_err(|e| format!("Invalid UTF-8 for key: {}", e))?);
                }
                2 => {
                    // value field
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for value: {}", wire_type));
                    }
                    let len = decode_varint(buf)? as usize;
                    if buf.remaining() < len {
                        return Err("Buffer underflow reading value".to_string());
                    }
                    let mut bytes = vec![0u8; len];
                    buf.copy_to_slice(&mut bytes);
                    value = Some(String::from_utf8(bytes)
                        .map_err(|e| format!("Invalid UTF-8 for value: {}", e))?);
                }
                _ => {
                    // Unknown field, skip it
                    if wire_type == 2 {
                        let len = decode_varint(buf)? as usize;
                        if buf.remaining() < len {
                            return Err("Buffer underflow skipping field".to_string());
                        }
                        buf.advance(len);
                    } else if wire_type == 0 {
                        decode_varint(buf)?;
                    } else {
                        return Err(format!("Unknown wire type: {}", wire_type));
                    }
                }
            }
        }

        match (key, value) {
            (Some(k), Some(v)) => Ok(Some(Header { key: k, value: v })),
            _ => Err("Invalid header: missing key or value".to_string()),
        }
    }
}

/// Frame represents a WebSocket message frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub seq_id: u64,
    pub log_id: u64,
    pub service: i32,
    pub method: i32,
    pub headers: Vec<Header>,
    pub payload_encoding: String,
    pub payload_type: String,
    pub payload: Vec<u8>,
    pub log_id_new: String,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            seq_id: 0,
            log_id: 0,
            service: 0,
            method: 0,
            headers: Vec::new(),
            payload_encoding: String::new(),
            payload_type: String::new(),
            payload: Vec::new(),
            log_id_new: String::new(),
        }
    }
}

impl Frame {
    /// Create a new frame
    pub fn new(service: i32, method: i32) -> Self {
        Self {
            service,
            method,
            ..Default::default()
        }
    }

    /// Encode frame to protobuf format
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut buf = BytesMut::new();

        // Field 1: seq_id (varint)
        buf.put_u8(0x08);
        encode_varint(self.seq_id, &mut buf);

        // Field 2: log_id (varint)
        buf.put_u8(0x10);
        encode_varint(self.log_id, &mut buf);

        // Field 3: service (varint)
        buf.put_u8(0x18);
        encode_varint(self.service as u64, &mut buf);

        // Field 4: method (varint)
        buf.put_u8(0x20);
        encode_varint(self.method as u64, &mut buf);

        // Field 5: headers (repeated)
        for header in &self.headers {
            buf.put_u8(0x2A);
            let header_data = {
                let mut tmp = BytesMut::new();
                header.encode(&mut tmp);
                tmp.to_vec()
            };
            encode_varint(header_data.len() as u64, &mut buf);
            buf.put_slice(&header_data);
        }

        // Field 6: payload_encoding (string, optional)
        if !self.payload_encoding.is_empty() {
            buf.put_u8(0x32);
            encode_string(self.payload_encoding.as_bytes(), &mut buf);
        }

        // Field 7: payload_type (string, optional)
        if !self.payload_type.is_empty() {
            buf.put_u8(0x3A);
            encode_string(self.payload_type.as_bytes(), &mut buf);
        }

        // Field 8: payload (bytes, optional)
        if !self.payload.is_empty() {
            buf.put_u8(0x42);
            encode_varint(self.payload.len() as u64, &mut buf);
            buf.put_slice(&self.payload);
        }

        // Field 9: log_id_new (string, optional)
        if !self.log_id_new.is_empty() {
            buf.put_u8(0x4A);
            encode_string(self.log_id_new.as_bytes(), &mut buf);
        }

        Ok(buf.to_vec())
    }

    /// Decode frame from protobuf format
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        let mut buf = BytesMut::from(data);
        let mut frame = Frame::default();

        while buf.has_remaining() {
            let tag = decode_varint(&mut buf)?;
            let field_num = tag >> 3;
            let wire_type = tag & 0x07;

            match field_num {
                1 => {
                    // seq_id
                    if wire_type != 0 {
                        return Err(format!("Invalid wire type for seq_id: {}", wire_type));
                    }
                    frame.seq_id = decode_varint(&mut buf)?;
                }
                2 => {
                    // log_id
                    if wire_type != 0 {
                        return Err(format!("Invalid wire type for log_id: {}", wire_type));
                    }
                    frame.log_id = decode_varint(&mut buf)?;
                }
                3 => {
                    // service
                    if wire_type != 0 {
                        return Err(format!("Invalid wire type for service: {}", wire_type));
                    }
                    frame.service = decode_varint(&mut buf)? as i32;
                }
                4 => {
                    // method
                    if wire_type != 0 {
                        return Err(format!("Invalid wire type for method: {}", wire_type));
                    }
                    frame.method = decode_varint(&mut buf)? as i32;
                }
                5 => {
                    // headers
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for headers: {}", wire_type));
                    }
                    let len = decode_varint(&mut buf)? as usize;
                    if buf.remaining() < len {
                        return Err("Buffer underflow reading header".to_string());
                    }
                    let mut header_buf = buf.split_to(len);
                    // Header::decode returns Result<Option<Header>>
                    match Header::decode(&mut header_buf) {
                        Ok(Some(header)) => frame.headers.push(header),
                        Ok(None) => {}  // Skip empty/incomplete headers
                        Err(e) => {
                            return Err(format!("Failed to decode header: {}", e));
                        }
                    }
                }
                6 => {
                    // payload_encoding
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for payload_encoding: {}", wire_type));
                    }
                    frame.payload_encoding = decode_string(&mut buf)?;
                }
                7 => {
                    // payload_type
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for payload_type: {}", wire_type));
                    }
                    frame.payload_type = decode_string(&mut buf)?;
                }
                8 => {
                    // payload
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for payload: {}", wire_type));
                    }
                    let len = decode_varint(&mut buf)? as usize;
                    if buf.remaining() < len {
                        return Err("Buffer underflow reading payload".to_string());
                    }
                    frame.payload = buf.split_to(len).to_vec();
                }
                9 => {
                    // log_id_new
                    if wire_type != 2 {
                        return Err(format!("Invalid wire type for log_id_new: {}", wire_type));
                    }
                    frame.log_id_new = decode_string(&mut buf)?;
                }
                _ => {
                    // Unknown field, skip it
                    if wire_type == 2 {
                        let len = decode_varint(&mut buf)? as usize;
                        if buf.remaining() < len {
                            return Err("Buffer underflow skipping field".to_string());
                        }
                        buf.advance(len);
                    } else if wire_type == 0 {
                        decode_varint(&mut buf)?;
                    } else {
                        return Err(format!("Unknown wire type: {}", wire_type));
                    }
                }
            }
        }

        Ok(frame)
    }

    /// Get header value by key
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.iter()
            .find(|h| h.key == key)
            .map(|h| &h.value)
    }

    /// Add or update a header
    pub fn set_header(&mut self, key: String, value: String) {
        if let Some(header) = self.headers.iter_mut().find(|h| h.key == key) {
            header.value = value;
        } else {
            self.headers.push(Header { key, value });
        }
    }
}

/// Encode a varint
fn encode_varint(mut value: u64, buf: &mut BytesMut) {
    while value >= 0x80 {
        buf.put_u8((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    buf.put_u8(value as u8);
}

/// Decode a varint
fn decode_varint<B: Buf>(buf: &mut B) -> Result<u64, String> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        if !buf.has_remaining() {
            return Err("Unexpected end of buffer".to_string());
        }

        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 64 {
            return Err("Varint too large".to_string());
        }
    }

    Ok(result)
}

/// Encode a string (length-delimited)
fn encode_string(bytes: &[u8], buf: &mut BytesMut) {
    encode_varint(bytes.len() as u64, buf);
    buf.put_slice(bytes);
}

/// Decode a string (length-delimited)
fn decode_string<B: Buf>(buf: &mut B) -> Result<String, String> {
    let len = decode_varint(buf)? as usize;
    if buf.remaining() < len {
        return Err("Buffer underflow reading string".to_string());
    }

    let mut bytes = vec![0u8; len];
    buf.copy_to_slice(&mut bytes);

    String::from_utf8(bytes)
        .map_err(|e| format!("Invalid UTF-8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_encode_decode() {
        let mut buf = BytesMut::new();

        // Test single byte
        encode_varint(127, &mut buf);
        assert_eq!(decode_varint(&mut buf.as_ref()).unwrap(), 127);

        // Test multi byte
        buf.clear();
        encode_varint(300, &mut buf);
        assert_eq!(decode_varint(&mut buf.as_ref()).unwrap(), 300);

        // Test large value
        buf.clear();
        encode_varint(0xFFFFFFFF, &mut buf);
        assert_eq!(decode_varint(&mut buf.as_ref()).unwrap(), 0xFFFFFFFF);
    }

    #[test]
    fn test_frame_encode_decode() {
        let frame = Frame {
            seq_id: 123,
            log_id: 456,
            service: 1,
            method: 0,
            headers: vec![
                Header {
                    key: "type".to_string(),
                    value: "event".to_string(),
                },
                Header {
                    key: "message_id".to_string(),
                    value: "msg_123".to_string(),
                },
            ],
            payload: b"hello world".to_vec(),
            ..Default::default()
        };

        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();

        assert_eq!(decoded.seq_id, frame.seq_id);
        assert_eq!(decoded.log_id, frame.log_id);
        assert_eq!(decoded.service, frame.service);
        assert_eq!(decoded.method, frame.method);
        assert_eq!(decoded.headers.len(), frame.headers.len());
        assert_eq!(decoded.payload, frame.payload);
    }

    #[test]
    fn test_header_encode_decode() {
        let header = Header {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        };

        let mut buf = BytesMut::new();
        header.encode(&mut buf);

        let decoded = Header::decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.key, header.key);
        assert_eq!(decoded.value, header.value);
    }
}
