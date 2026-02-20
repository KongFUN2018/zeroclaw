//! Lark/Feishu WebSocket long connection client
//!
//! This client implements the official Lark/Feishu WebSocket protocol
//! including endpoint retrieval, binary frame handling, and heartbeat.

use crate::lark_ws::proto::Frame;
use crate::lark_ws::frame::{MessageType, HeadersExt, new_ping_frame};
use anyhow::{Result, Context, anyhow};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn, error};

/// Client configuration from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    pub reconnect_count: Option<i32>,
    pub reconnect_interval: Option<u64>,
    pub reconnect_nonce: Option<u64>,
    pub ping_interval: Option<u64>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            reconnect_count: Some(-1),      // Infinite reconnect
            reconnect_interval: Some(120),   // 2 minutes
            reconnect_nonce: Some(30),       // 30 seconds random jitter
            ping_interval: Some(120),        // 2 minutes
        }
    }
}

/// WebSocket endpoint response
#[derive(Debug, Deserialize)]
struct EndpointResponse {
    code: i32,
    #[serde(default)]
    msg: String,
    data: Option<EndpointData>,
}

#[derive(Debug, Deserialize)]
struct EndpointData {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig", default)]
    client_config: Option<ClientConfig>,
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle_event(&self, event_json: &str) -> Result<()>;
}

/// Lark WebSocket client
pub struct LarkWebSocketClient {
    app_id: String,
    app_secret: String,
    base_url: String,
    config: Arc<Mutex<ClientConfig>>,
    event_handler: Option<Box<dyn EventHandler>>,
    device_id: Arc<Mutex<String>>,
    service_id: Arc<Mutex<String>>,
    client: reqwest::Client,
}

impl LarkWebSocketClient {
    /// Create a new WebSocket client
    pub fn new(app_id: String, app_secret: String, use_feishu: bool) -> Self {
        let base_url = if use_feishu {
            "https://open.feishu.cn".to_string()
        } else {
            "https://open.larksuite.com".to_string()
        };

        Self {
            app_id,
            app_secret,
            base_url,
            config: Arc::new(Mutex::new(ClientConfig::default())),
            event_handler: None,
            device_id: Arc::new(Mutex::new(String::new())),
            service_id: Arc::new(Mutex::new(String::new())),
            client: reqwest::Client::new(),
        }
    }

    /// Set event handler
    pub fn with_event_handler(mut self, handler: Box<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }

    /// Get WebSocket endpoint URL from server
    async fn get_endpoint(&self) -> Result<(String, ClientConfig)> {
        let url = format!("{}/callback/ws/endpoint", self.base_url);

        let body = serde_json::json!({
            "AppID": self.app_id,
            "AppSecret": self.app_secret
        });

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to request WebSocket endpoint")?;

        if !resp.status().is_success() {
            return Err(anyhow!("Endpoint request failed: {}", resp.status()));
        }

        let endpoint_resp: EndpointResponse = resp
            .json()
            .await
            .context("Failed to parse endpoint response")?;

        if endpoint_resp.code != 0 {
            return Err(anyhow!("Endpoint error (code {}): {}", endpoint_resp.code, endpoint_resp.msg));
        }

        let data = endpoint_resp.data.ok_or_else(|| anyhow!("Missing endpoint data"))?;
        let config = data.client_config.unwrap_or_default();

        Ok((data.url, config))
    }

    /// Extract device_id and service_id from WebSocket URL
    fn parse_ws_params(url: &str) -> Result<(String, String)> {
        let parsed = url::Url::parse(url)
            .context("Failed to parse WebSocket URL")?;

        let device_id = parsed
            .query_pairs()
            .find(|(k, _)| k == "device_id")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| anyhow!("Missing device_id in URL"))?;

        let service_id = parsed
            .query_pairs()
            .find(|(k, _)| k == "service_id")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| anyhow!("Missing service_id in URL"))?;

        Ok((device_id, service_id))
    }

    /// Start the WebSocket connection
    pub async fn start(&self) -> Result<()> {
        loop {
            info!("Attempting to establish WebSocket connection...");
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("WebSocket connection closed normally");
                    break;
                }
                Err(e) => {
                    error!("WebSocket connection failed: {}", e);
                    error!("Possible causes:");
                    error!("  1. Long connection is not enabled in Feishu developer console");
                    error!("  2. App is not published (required for long connection)");
                    error!("  3. Invalid app_id or app_secret");
                    error!("  4. Network connectivity issues");

                    let config = self.config.lock().await;
                    let should_reconnect = config.reconnect_count.map(|c| c < 0 || c > 0).unwrap_or(true);

                    if !should_reconnect {
                        return Err(e);
                    }

                    let interval = Duration::from_secs(config.reconnect_interval.unwrap_or(120));
                    let nonce = config.reconnect_nonce.unwrap_or(30);
                    drop(config);

                    // Add random jitter before first reconnect
                    if nonce > 0 {
                        let jitter = rand::random::<u64>() % (nonce * 1000);
                        info!("Waiting {}ms before reconnect...", jitter);
                        tokio::time::sleep(Duration::from_millis(jitter)).await;
                    }

                    info!("Reconnecting in {:?}...", interval);
                    tokio::time::sleep(interval).await;
                }
            }
        }

        Ok(())
    }

    /// Connect and run the WebSocket loop
    async fn connect_and_run(&self) -> Result<()> {
        // Get endpoint URL
        let (ws_url, server_config) = self.get_endpoint().await?;
        debug!("Got WebSocket endpoint: {}", ws_url);

        // Update config from server
        *self.config.lock().await = server_config;

        // Parse device_id and service_id
        let (device_id, service_id) = Self::parse_ws_params(&ws_url)?;
        *self.device_id.lock().await = device_id;
        *self.service_id.lock().await = service_id.clone();

        info!("Connecting to WebSocket: {}", ws_url);

        // Connect to WebSocket (no auth headers needed)
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
            .context("Failed to connect to WebSocket")?;

        info!("WebSocket connected (device_id: {}, service_id: {})",
            self.device_id.lock().await, service_id);

        debug!("Waiting for messages from WebSocket...");

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        // Parse service_id for ping frames
        let service_id_i32 = service_id.parse::<i32>()
            .unwrap_or(0);

        // Spawn ping loop
        let config_clone = self.config.clone();
        let device_id_clone = self.device_id.clone();
        let ping_handle = tokio::spawn(async move {
            Self::ping_loop(ws_sender, service_id_i32, config_clone, device_id_clone).await
        });

        // Message receive loop
        let event_handler = self.event_handler.as_ref();
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    debug!("Received binary data: {} bytes", data.len());
                    if !data.is_empty() {
                        debug!("First 100 bytes (hex): {}", &data[..data.len().min(100)].to_vec()[..data.len().min(100)].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                    }
                    if let Err(e) = self.handle_frame(data, event_handler).await {
                        error!("Error handling frame: {}", e);
                    }
                }
                Ok(Message::Text(text)) => {
                    debug!("Received text data: {} bytes", text.len());
                    debug!("Text content: {}", text);
                    // Try to handle as JSON event
                    if let Some(handler) = event_handler {
                        if let Err(e) = handler.handle_event(&text) {
                            error!("Event handler error for text message: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed by server");
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    debug!("Received standard WebSocket ping");
                    // Standard WebSocket ping is handled by tungstenite
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received WebSocket pong");
                }
                Ok(_) => {
                    debug!("Ignoring unknown WebSocket message type");
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Cancel ping loop
        ping_handle.abort();

        Ok(())
    }

    /// Ping loop - sends custom ping frames
    async fn ping_loop<
        T: futures_util::Sink<Message> + Unpin + Send,
    >(
        mut sender: T,
        service_id: i32,
        config: Arc<Mutex<ClientConfig>>,
        device_id: Arc<Mutex<String>>,
    ) where
        T::Error: Send + std::error::Error,
    {
        loop {
            let interval = {
                let cfg = config.lock().await;
                Duration::from_secs(cfg.ping_interval.unwrap_or(120))
            };

            tokio::time::sleep(interval).await;

            let frame = new_ping_frame(service_id);
            match frame.encode() {
                Ok(data) => {
                    if let Err(e) = sender.send(Message::Binary(data)).await {
                        warn!("Failed to send ping: {}", e);
                        break;
                    }
                    debug!("Ping sent (device_id: {})", device_id.lock().await);
                }
                Err(e) => {
                    error!("Failed to encode ping frame: {}", e);
                }
            }
        }
    }

    /// Handle incoming binary frame
    async fn handle_frame(
        &self,
        data: Vec<u8>,
        event_handler: Option<&Box<dyn EventHandler>>,
    ) -> Result<()> {
        debug!("Attempting to decode frame from {} bytes", data.len());

        // First check if data is valid UTF-8 (likely JSON)
        if let Ok(json_str) = std::str::from_utf8(&data) {
            debug!("Data is valid UTF-8, trying JSON first");

            // Check if it looks like JSON
            if json_str.trim().starts_with("{") {
                if let Ok(_json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    debug!("Successfully parsed as JSON event");

                    // Handle as event
                    if let Some(handler) = event_handler {
                        if let Err(e) = handler.handle_event(json_str) {
                            error!("Event handler error for JSON message: {}", e);
                        }
                    }
                    return Ok(());
                }
            }
        }

        // Try Protocol Buffers format (binary)
        debug!("Trying Protocol Buffers format");
        let frame_result = Frame::decode(&data);

        if let Err(ref e) = frame_result {
            debug!("Protocol Buffers decode failed: {}, trying as JSON", e);

            // Try parsing as JSON directly (v2 might use JSON)
            if let Ok(json_str) = std::str::from_utf8(&data) {
                debug!("Data as UTF-8: {}", json_str);

                // Try to parse as JSON
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    debug!("Successfully parsed as JSON: {}", serde_json::to_string(&json_value).unwrap_or_default());

                    // Handle as event
                    if let Some(handler) = event_handler {
                        if let Err(e) = handler.handle_event(json_str) {
                            error!("Event handler error for JSON message: {}", e);
                        }
                    }
                    return Ok(());
                }
            }

            return Err(anyhow::anyhow!("Failed to decode frame: {}", e));
        }

        let frame = frame_result.unwrap();
        let frame_type = frame.method;
        let message_type = frame.message_type();

        debug!("Received frame: method={}, message_type={:?}, headers={:?}",
            frame_type, message_type, frame.headers);

        match message_type {
            Some(MessageType::Ping) => {
                // Handle server ping - respond with pong
                debug!("Received ping frame");
                // Pong is handled automatically by tungstenite
            }
            Some(MessageType::Pong) => {
                debug!("Received pong frame");
            }
            Some(MessageType::Event) => {
                // Handle event message
                debug!("Processing event message, payload length: {}", frame.payload.len());
                if let Some(handler) = event_handler {
                    let event_json = String::from_utf8_lossy(&frame.payload);
                    debug!("Event JSON: {}", event_json);
                    if let Err(e) = handler.handle_event(&event_json) {
                        error!("Event handler error: {}", e);
                    }
                }
            }
            Some(MessageType::Card) => {
                debug!("Received card callback (not implemented)");
            }
            None => {
                debug!("Received frame with unknown message type, headers: {:?}", frame.headers);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEventHandler;

    impl EventHandler for TestEventHandler {
        fn handle_event(&self, event_json: &str) -> Result<()> {
            println!("Event: {}", event_json);
            Ok(())
        }
    }

    #[test]
    fn test_parse_ws_params() {
        let url = "wss://ws-open.feishu.cn/ws/v1?device_id=abc123&service_id=456";
        let (device_id, service_id) = LarkWebSocketClient::parse_ws_params(url).unwrap();
        assert_eq!(device_id, "abc123");
        assert_eq!(service_id, "456");
    }
}
