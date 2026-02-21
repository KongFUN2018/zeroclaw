//! Lark/Feishu WebSocket long connection module
//!
//! This module implements the Protocol Buffers-based WebSocket protocol
//! used by Lark/Feishu for real-time event delivery.

pub mod client;
pub mod frame;
pub mod proto;

pub use client::{EventHandler, LarkWebSocketClient};
