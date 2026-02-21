//! Frame types and constants for Lark WebSocket protocol

use crate::lark_ws::proto::Frame;

/// Frame type (method field)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Control = 0,
    Data = 1,
}

impl FrameType {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(FrameType::Control),
            1 => Some(FrameType::Data),
            _ => None,
        }
    }
}

/// Message type (from "type" header)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Event,
    Card,
    Ping,
    Pong,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Event => "event",
            MessageType::Card => "card",
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "event" => Some(MessageType::Event),
            "card" => Some(MessageType::Card),
            "ping" => Some(MessageType::Ping),
            "pong" => Some(MessageType::Pong),
            _ => None,
        }
    }
}

/// Header name constants
pub mod header {
    pub const TIMESTAMP: &str = "timestamp";
    pub const TYPE: &str = "type";
    pub const MESSAGE_ID: &str = "message_id";
    pub const SUM: &str = "sum";
    pub const SEQ: &str = "seq";
    pub const TRACE_ID: &str = "trace_id";
    pub const INSTANCE_ID: &str = "instance_id";
    pub const BIZ_RT: &str = "biz_rt";
}

/// Extension trait for easy header access
pub trait HeadersExt {
    fn get_header_str(&self, key: &str) -> Option<String>;
    fn get_header_u64(&self, key: &str) -> Option<u64>;
    fn message_type(&self) -> Option<MessageType>;
    fn is_control_frame(&self) -> bool;
    fn is_data_frame(&self) -> bool;
}

impl HeadersExt for Frame {
    fn get_header_str(&self, key: &str) -> Option<String> {
        self.get_header(key).map(|v| v.clone())
    }

    fn get_header_u64(&self, key: &str) -> Option<u64> {
        self.get_header(key).and_then(|v| v.parse::<u64>().ok())
    }

    fn message_type(&self) -> Option<MessageType> {
        self.get_header(header::TYPE)
            .and_then(|t| MessageType::from_str(t))
    }

    fn is_control_frame(&self) -> bool {
        self.method == FrameType::Control as i32
    }

    fn is_data_frame(&self) -> bool {
        self.method == FrameType::Data as i32
    }
}

/// Create a new ping frame
pub fn new_ping_frame(service_id: i32) -> Frame {
    let mut frame = Frame::new(service_id, FrameType::Control as i32);
    frame.set_header(
        header::TYPE.to_string(),
        MessageType::Ping.as_str().to_string(),
    );
    frame
}

/// Create a new response frame for an event
pub fn new_response_frame(
    request_frame: &Frame,
    status_code: i32,
    data: Option<Vec<u8>>,
    biz_rt_ms: Option<u64>,
) -> Frame {
    let mut response = Frame::new(request_frame.service, FrameType::Data as i32);
    response.seq_id = request_frame.seq_id;
    response.log_id = request_frame.log_id;

    // Copy headers from request
    for header in &request_frame.headers {
        response.headers.push(header.clone());
    }

    // Add business processing time
    if let Some(rt) = biz_rt_ms {
        response.set_header(header::BIZ_RT.to_string(), rt.to_string());
    }

    // Build response payload
    let payload_data = if let Some(d) = data { d } else { vec![] };

    let response_obj = if payload_data.is_empty() {
        serde_json::json!({
            "code": status_code,
            "data": serde_json::Value::Null
        })
    } else {
        serde_json::json!({
            "code": status_code,
            "data": payload_data
        })
    };

    response.payload = serde_json::to_vec(&response_obj).unwrap_or_default();
    response.set_header(
        header::TYPE.to_string(),
        MessageType::Event.as_str().to_string(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::Event.as_str(), "event");
        assert_eq!(MessageType::from_str("event"), Some(MessageType::Event));
        assert_eq!(MessageType::from_str("unknown"), None);
    }

    #[test]
    fn test_ping_frame() {
        let frame = new_ping_frame(123);
        assert_eq!(frame.service, 123);
        assert_eq!(frame.method, FrameType::Control as i32);
        assert_eq!(frame.message_type(), Some(MessageType::Ping));
    }

    #[test]
    fn test_response_frame() {
        let mut request = Frame::new(123, FrameType::Data as i32);
        request.seq_id = 456;
        request.log_id = 789;
        request.set_header(header::MESSAGE_ID.to_string(), "msg_123".to_string());

        let response = new_response_frame(&request, 200, Some(b"response".to_vec()), Some(100));

        assert_eq!(response.seq_id, 456);
        assert_eq!(response.log_id, 789);
        assert_eq!(response.service, 123);
        assert_eq!(
            response.get_header_str(header::MESSAGE_ID),
            Some("msg_123".to_string())
        );
        assert_eq!(
            response.get_header_str(header::BIZ_RT),
            Some("100".to_string())
        );
    }
}
