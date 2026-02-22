use super::traits::{Channel, ChannelMessage};
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Lark/Feishu base URL
const LARK_BASE_URL: &str = "https://open.larksuite.com";
const FEISHU_BASE_URL: &str = "https://open.feishu.cn";

/// Connection modes
const MODE_WEBHOOK: &str = "webhook";
const MODE_POLLING: &str = "polling";
const MODE_LONG_CONNECTION: &str = "long_connection";

/// Lark/Feishu channel — supports webhook, polling, and long connection modes
pub struct LarkChannel {
    app_id: String,
    app_secret: String,
    encrypt_key: Option<String>,
    verification_token: Option<String>,
    allowed_users: Vec<String>,
    use_feishu: bool,
    connection_mode: String,
    poll_interval: Duration,
    tenant_access_token: Arc<Mutex<Option<String>>>,
    token_expiry: Arc<Mutex<Option<Instant>>>,
    seen_messages: Arc<Mutex<HashSet<String>>>,
    client: reqwest::Client,
}

impl LarkChannel {
    pub fn new(
        app_id: String,
        app_secret: String,
        encrypt_key: Option<String>,
        verification_token: Option<String>,
        allowed_users: Vec<String>,
        use_feishu: bool,
    ) -> Self {
        Self {
            app_id,
            app_secret,
            encrypt_key,
            verification_token,
            allowed_users,
            use_feishu,
            connection_mode: MODE_WEBHOOK.to_string(),
            poll_interval: Duration::from_secs(5),
            tenant_access_token: Arc::new(Mutex::new(None)),
            token_expiry: Arc::new(Mutex::new(None)),
            seen_messages: Arc::new(Mutex::new(HashSet::new())),
            client: reqwest::Client::new(),
        }
    }

    /// Set connection mode
    pub fn with_connection_mode(mut self, mode: String, interval_secs: u64) -> Self {
        self.connection_mode = mode;
        self.poll_interval = Duration::from_secs(interval_secs);
        self
    }

    /// Check if using webhook mode
    fn is_webhook_mode(&self) -> bool {
        self.connection_mode == MODE_WEBHOOK
    }

    /// Check if using polling mode
    fn is_polling_mode(&self) -> bool {
        self.connection_mode == MODE_POLLING
    }

    /// Check if using long connection mode
    fn is_long_connection_mode(&self) -> bool {
        self.connection_mode == MODE_LONG_CONNECTION
    }

    fn base_url(&self) -> &str {
        if self.use_feishu {
            FEISHU_BASE_URL
        } else {
            LARK_BASE_URL
        }
    }

    /// Check if a user is allowed (supports user_id, open_id, union_id)
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// Check if a message has already been processed (deduplication)
    async fn is_message_processed(&self, message_id: &str) -> bool {
        let mut seen = self.seen_messages.lock().await;
        if seen.contains(message_id) {
            tracing::debug!("Lark: skipping duplicate message {}", message_id);
            true
        } else {
            seen.insert(message_id.to_string());

            // Periodically clean up old message IDs to prevent unbounded growth
            // Keep only the most recent 1000 messages
            if seen.len() > 1000 {
                // Remove oldest entries (first 100)
                let old_ids: Vec<_> = seen.iter().take(100).cloned().collect();
                for id in old_ids {
                    seen.remove(&id);
                }
                tracing::debug!(
                    "Lark: cleaned up old message IDs, current count: {}",
                    seen.len()
                );
            }

            false
        }
    }

    /// Get or refresh tenant access token (2 hour expiry)
    pub async fn get_tenant_access_token(&self) -> Option<String> {
        // Check if we have a valid cached token
        {
            let token_guard = self.tenant_access_token.lock().await;
            let expiry_guard = self.token_expiry.lock().await;

            if let (Some(token), Some(expiry)) = (&*token_guard, *expiry_guard) {
                if expiry > Instant::now() + Duration::from_secs(300) {
                    // Token is valid for at least 5 more minutes
                    return Some(token.clone());
                }
            }
        } // Drop locks before making HTTP request

        // Need to refresh token
        let url = format!(
            "{}/open-apis/auth/v3/tenant_access_token/internal",
            self.base_url()
        );

        let body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret
        });

        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to get tenant access token: {e}");
                return None;
            }
        };

        if !resp.status().is_success() {
            tracing::error!("Tenant access token API returned error: {}", resp.status());
            return None;
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to parse token response: {e}");
                return None;
            }
        };

        let token = data
            .get("tenant_access_token")
            .and_then(|t| t.as_str())
            .map(String::from);

        let expire = data.get("expire").and_then(|e| e.as_i64()).unwrap_or(7200); // Default 2 hours

        if let Some(ref token_str) = token {
            let expiry = Instant::now() + Duration::from_secs(expire as u64 - 300); // Refresh 5 min early

            *self.tenant_access_token.lock().await = Some(token_str.clone());
            *self.token_expiry.lock().await = Some(expiry);

            tracing::debug!("Refreshed tenant access token, expires in {}s", expire);
        }

        token
    }

    /// Parse incoming Lark/Feishu webhook event and extract messages
    /// Supports both "text" and "post" message types (like NullClaw)
    pub async fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        // Check event type
        let event_type = payload
            .get("header")
            .and_then(|h| h.get("event_type"))
            .and_then(|t| t.as_str());

        if event_type != Some("im.message.receive_v1") {
            return messages;
        }

        let event = match payload.get("event") {
            Some(e) => e,
            None => return messages,
        };

        // Extract sender open_id (NOTE: Feishu uses "open_id", not "user_id")
        let sender_open_id = event
            .get("sender")
            .and_then(|s| s.get("sender_id"))
            .and_then(|id| id.get("open_id"))
            .and_then(|u| u.as_str());

        let sender_open_id = match sender_open_id {
            Some(s) => s,
            None => {
                tracing::warn!("Missing open_id in sender event");
                return messages;
            }
        };

        // Check authorization
        if !self.is_user_allowed(sender_open_id) {
            tracing::warn!(
                "Lark: ignoring message from unauthorized user: {sender_open_id}. \
                Add to allowed_users in config.toml."
            );
            return messages;
        }

        // Extract message content
        let message_obj = match event.get("message") {
            Some(m) => m,
            None => return messages,
        };

        // Get chat_id for sending replies (prefer chat_id over open_id)
        let chat_id = message_obj
            .get("chat_id")
            .and_then(|c| c.as_str())
            .unwrap_or(sender_open_id);

        let message_id = message_obj
            .get("message_id")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Check for duplicate message (deduplication)
        if self.is_message_processed(&message_id).await {
            tracing::info!("Lark: skipping duplicate message_id {}", message_id);
            return messages;
        }

        let msg_type = message_obj
            .get("message_type")
            .and_then(|t| t.as_str())
            .unwrap_or("text");

        let content_str = message_obj
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("{}");

        // Parse content based on message type
        let text = match msg_type {
            "text" => {
                // Parse content JSON (Lark stores content as JSON string)
                let content_json: serde_json::Value = match serde_json::from_str(content_str) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!("Failed to parse message content JSON: {e}");
                        return messages;
                    }
                };
                content_json
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            "post" => {
                // Parse post content (rich text)
                self.parse_post_content(content_str).unwrap_or_default()
            }
            _ => {
                tracing::debug!("Unsupported message type: {}", msg_type);
                return messages;
            }
        };

        // Strip @_user_N placeholders injected by Feishu in group chats
        let text = self.strip_at_placeholders(&text);

        // Trim whitespace
        let text = text.trim();
        if text.is_empty() {
            return messages;
        }

        // Get timestamp (Lark timestamps are in milliseconds)
        let timestamp = if let Some(create_time_str) =
            message_obj.get("create_time").and_then(|t| t.as_str())
        {
            create_time_str
                .parse::<u64>()
                .ok()
                .map(|ms| ms / 1000)
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };

        messages.push(ChannelMessage {
            id: message_id,
            sender: chat_id.to_string(), // Use chat_id for sending replies
            content: text.to_string(),
            channel: self.name().to_string(),
            timestamp,
        });

        messages
    }

    /// Parse a Lark "post" rich-text message to plain text.
    /// Post format: {"zh_cn": {"title": "...", "content": [[{"tag": "text", "text": "..."}]]}}
    /// Returns None when content cannot be parsed or yields no usable text.
    /// Matches NullClaw's parsePostContent implementation.
    fn parse_post_content(&self, post_json: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(post_json).ok()?;

        // Try locale keys: zh_cn, en_us, or first object value
        let locale_obj = if let Some(l) = parsed.get("zh_cn").and_then(|v| v.as_object()) {
            l
        } else if let Some(l) = parsed.get("en_us").and_then(|v| v.as_object()) {
            l
        } else {
            // Try first object value (like NullClaw)
            let mut iter = parsed.as_object()?.values();
            match iter.next() {
                Some(v) => v.as_object()?,
                None => return None,
            }
        };

        let mut result = String::new();

        // Title
        if let Some(title_val) = locale_obj.get("title") {
            if let Some(title) = title_val.as_str() {
                if !title.is_empty() {
                    result.push_str(title);
                    result.push_str("\n\n");
                }
            }
        }

        // Content paragraphs: [[{tag, text}, ...], ...]
        if let Some(content_val) = locale_obj.get("content") {
            if let Some(content) = content_val.as_array() {
                for para in content {
                    if let Some(para_array) = para.as_array() {
                        for el in para_array {
                            if let Some(el_obj) = el.as_object() {
                                let tag_val = el_obj.get("tag");
                                let tag = tag_val.and_then(|t| t.as_str()).unwrap_or("");

                                if tag == "text" {
                                    if let Some(text_val) = el_obj.get("text") {
                                        if let Some(text) = text_val.as_str() {
                                            result.push_str(text);
                                        }
                                    }
                                } else if tag == "a" {
                                    // Link: prefer text, fallback to href
                                    let mut link_text = None;
                                    if let Some(text_val) = el_obj.get("text") {
                                        if let Some(t) = text_val.as_str() {
                                            if !t.is_empty() {
                                                link_text = Some(t);
                                            }
                                        }
                                    }
                                    if link_text.is_none() {
                                        if let Some(href_val) = el_obj.get("href") {
                                            if let Some(h) = href_val.as_str() {
                                                link_text = Some(h);
                                            }
                                        }
                                    }
                                    if let Some(lt) = link_text {
                                        result.push_str(lt);
                                    }
                                } else if tag == "at" {
                                    result.push('@');
                                    let mut name = None;
                                    if let Some(name_val) = el_obj.get("user_name") {
                                        name = name_val.as_str();
                                    }
                                    if name.is_none() {
                                        if let Some(uid_val) = el_obj.get("user_id") {
                                            name = uid_val.as_str();
                                        }
                                    }
                                    if let Some(n) = name {
                                        result.push_str(n);
                                    } else {
                                        result.push_str("user");
                                    }
                                }
                            }
                        }
                        result.push('\n');
                    }
                }
            }
        }

        let trimmed = result.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Remove `@_user_N` placeholder tokens injected by Feishu in group chats.
    /// Patterns like "@_user_1", "@_user_2" are replaced with empty string.
    /// Matches NullClaw's stripAtPlaceholders implementation.
    fn strip_at_placeholders(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut i = 0;
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();

        while i < len {
            if chars[i] == '@' && i + 1 < len {
                // Check for "_user_" prefix after '@'
                let remaining = &text[i + 1..];
                if remaining.starts_with("_user_") {
                    // Skip past "@_user_"
                    let mut skip = 1 + "_user_".len(); // '@' + "_user_"
                                                       // Skip digits
                    while i + skip < len && chars[i + skip].is_ascii_digit() {
                        skip += 1;
                    }
                    // Skip trailing space
                    if i + skip < len && chars[i + skip] == ' ' {
                        skip += 1;
                    }
                    i += skip;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Decrypt Lark/Feishu webhook payload (if encrypt_key is configured)
    pub fn decrypt_webhook_payload(&self, encrypt: &str) -> Option<String> {
        use aes::cipher::{BlockDecryptMut, KeyIvInit};
        use base64::Engine;
        use cbc::Decryptor;

        let key_str = &self.encrypt_key.as_ref()?;

        tracing::debug!("Decrypting with encrypt_key length: {}", key_str.len());

        // Feishu uses AES-256-CBC
        // Key: Base64-decoded encrypt_key (should be 32 bytes)
        // IV: MD5 of empty string or first 16 bytes of key
        let cipher_key = URL_SAFE.decode(key_str).ok()?;

        tracing::debug!("Decoded key length: {} bytes", cipher_key.len());

        // Use first 16 bytes of key as IV (common Feishu approach)
        let iv = &cipher_key[..16.min(cipher_key.len())];

        let ciphertext = URL_SAFE.decode(encrypt).ok()?;

        tracing::debug!("Ciphertext length: {} bytes", ciphertext.len());

        type Aes256CbcDec = Decryptor<aes::Aes256>;

        let mut buf = ciphertext;
        let ct_len = buf.len();

        if ct_len % 16 != 0 || ct_len == 0 {
            tracing::warn!("Invalid ciphertext length: {}", ct_len);
            return None;
        }

        if cipher_key.len() < 32 {
            tracing::warn!("Key too short: {} bytes (need 32)", cipher_key.len());
            return None;
        }

        let mut cipher_key_array = [0u8; 32];
        cipher_key_array.copy_from_slice(&cipher_key[..32]);

        let mut iv_array = [0u8; 16];
        iv_array.copy_from_slice(iv);

        tracing::debug!("Key (first 8 bytes): {:02x?}", &cipher_key_array[..8]);
        tracing::debug!("IV: {:02x?}", &iv_array);

        let decryptor = Aes256CbcDec::new(&cipher_key_array.into(), (&iv_array).into());

        match decryptor.decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut buf) {
            Ok(decrypted) => match String::from_utf8(decrypted.to_vec()) {
                Ok(s) => {
                    tracing::debug!("Decrypted successfully: {} chars", s.len());
                    Some(s)
                }
                Err(e) => {
                    tracing::warn!("Decrypted data is not valid UTF-8: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Decryption failed: {}", e);
                None
            }
        }
    }

    /// Verify webhook URL challenge token
    pub fn verify_challenge(&self, token: &str) -> bool {
        match &self.verification_token {
            Some(t) if t == token => true,
            None => true, // If no token configured, accept all
            _ => false,
        }
    }

    /// Polling mode implementation
    async fn listen_polling(
        &self,
        tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Lark/Feishu polling mode started (interval: {}s)",
            self.poll_interval.as_secs()
        );

        let mut cursor = String::new();

        loop {
            // Get fresh tenant access token
            let token = match self.get_tenant_access_token().await {
                Some(t) => t,
                None => {
                    tracing::warn!("Failed to get tenant access token, retrying...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Poll for events using HTTP long polling
            let url = format!("{}/open-apis/event/v4/listener", self.base_url());

            let mut body = serde_json::json!({
                "page_size": 50
            });

            if !cursor.is_empty() {
                body["cursor"] = serde_json::Value::String(cursor.clone());
            }

            let resp = match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .json(&body)
                .timeout(Duration::from_secs(30))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Lark poll error: {e}");
                    tokio::time::sleep(self.poll_interval).await;
                    continue;
                }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_default();
                tracing::warn!("Lark poll failed ({status}): {error_text}");

                if status.as_u16() == 404 {
                    tracing::info!("Event subscription not configured. Please enable message events in Feishu developer console.");
                }

                tokio::time::sleep(self.poll_interval).await;
                continue;
            }

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Failed to parse poll response: {e}");
                    tokio::time::sleep(self.poll_interval).await;
                    continue;
                }
            };

            // Process events
            if let Some(events) = data.get("events").and_then(|e| e.as_array()) {
                for event in events {
                    if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
                        if event_type == "im.message.receive_v1" {
                            let parsed = self.parse_webhook_payload(event).await;
                            for msg in parsed {
                                if tx.send(msg).await.is_err() {
                                    tracing::error!("Failed to send message to channel");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            // Update cursor for next poll
            if let Some(new_cursor) = data.get("cursor").and_then(|c| c.as_str()) {
                cursor = new_cursor.to_string();
            }

            // Check if we have more data
            let has_more = data
                .get("has_more")
                .and_then(|h| h.as_bool())
                .unwrap_or(false);

            if !has_more {
                tokio::time::sleep(self.poll_interval).await;
            }
        }
    }

    /// Long connection mode implementation (WebSocket)
    async fn listen_long_connection(
        &self,
        tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> anyhow::Result<()> {
        use crate::lark_ws::{EventHandler, LarkWebSocketClient};

        tracing::info!("Lark/Feishu WebSocket long connection mode starting...");

        // Create event handler that forwards messages to the channel
        struct ChannelEventHandler {
            tx: tokio::sync::mpsc::Sender<ChannelMessage>,
            allowed_users: Vec<String>,
            lark_channel: LarkChannel,
            seen_messages: Arc<Mutex<HashSet<String>>>,
        }

        impl EventHandler for ChannelEventHandler {
            fn handle_event(&self, event_json: &str) -> anyhow::Result<()> {
                tracing::info!("Lark WebSocket event received: {}", event_json);

                // Parse the event JSON
                let event: serde_json::Value = serde_json::from_str(event_json)
                    .map_err(|e| anyhow::anyhow!("Failed to parse event JSON: {}", e))?;

                // Extract event data - handle both wrapped and unwrapped formats
                // Webhook format: { "schema": "2.0", "header": {...}, "event": {...} }
                // Long connection format: { ... } (direct event data)
                let event_data = if event.get("event").is_some() {
                    tracing::info!("Event format: wrapped (webhook-style)");
                    event.get("event")
                } else if event.get("header").is_some() {
                    tracing::info!("Event format: webhook with header");
                    event.get("event").or_else(|| Some(&event))
                } else {
                    tracing::info!("Event format: unwrapped (direct event data)");
                    // Assume the entire JSON is the event data
                    Some(&event)
                }
                .ok_or_else(|| anyhow::anyhow!("Missing event data"))?;

                // Check event type from header
                let event_type = event
                    .get("header")
                    .and_then(|h| h.get("event_type"))
                    .and_then(|t| t.as_str());

                // Only process message events, skip others (read receipts, etc.)
                if event_type != Some("im.message.receive_v1") {
                    tracing::debug!("Skipping non-message event: {:?}", event_type);
                    return Ok(());
                }

                tracing::info!(
                    "Event data: {}",
                    serde_json::to_string(event_data).unwrap_or_default()
                );

                // NOTE: Use open_id instead of user_id (matches NullClaw)
                let sender_open_id = event_data
                    .get("sender")
                    .and_then(|s| s.get("sender_id"))
                    .and_then(|id| id.get("open_id"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing sender.open_id"))?;

                // Check authorization
                if !self
                    .allowed_users
                    .iter()
                    .any(|u| u == "*" || u == sender_open_id)
                {
                    tracing::warn!(
                        "Ignoring message from unauthorized user: {}",
                        sender_open_id
                    );
                    return Ok(());
                }

                // Extract message content
                let message_obj = event_data
                    .get("message")
                    .ok_or_else(|| anyhow::anyhow!("Missing message field"))?;

                // Get chat_id for sending replies
                let chat_id = message_obj
                    .get("chat_id")
                    .and_then(|c| c.as_str())
                    .unwrap_or(sender_open_id);

                let message_id = message_obj
                    .get("message_id")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| Uuid::new_v4().to_string());

                // Check for duplicate message (deduplication)
                // Use a non-blocking attempt to acquire the async mutex to avoid
                // blocking the current thread inside the async runtime. If the
                // lock is not immediately available, skip deduplication for this
                // event to prevent a runtime panic.
                {
                    if let Ok(mut seen) = self.seen_messages.try_lock() {
                        if seen.contains(&message_id) {
                            tracing::info!(
                                "Lark WebSocket: skipping duplicate message_id {}",
                                message_id
                            );
                            return Ok(());
                        }
                        seen.insert(message_id.clone());

                        // Periodically clean up old message IDs to prevent unbounded growth
                        if seen.len() > 1000 {
                            let old_ids: Vec<_> = seen.iter().take(100).cloned().collect();
                            for id in old_ids {
                                seen.remove(&id);
                            }
                            tracing::debug!(
                                "Lark WebSocket: cleaned up old message IDs, current count: {}",
                                seen.len()
                            );
                        }
                    } else {
                        tracing::warn!("Lark WebSocket: seen_messages lock busy; skipping deduplication for this event");
                    }
                }

                let msg_type = message_obj
                    .get("message_type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("text");

                let content_str = message_obj
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("{}");

                // Parse content based on message type
                let text = match msg_type {
                    "text" => {
                        let content_json: serde_json::Value = serde_json::from_str(content_str)
                            .map_err(|e| anyhow::anyhow!("Failed to parse content JSON: {}", e))?;
                        content_json
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string()
                    }
                    "post" => self
                        .lark_channel
                        .parse_post_content(content_str)
                        .unwrap_or_default(),
                    _ => {
                        return Ok(()); // Skip unsupported message types
                    }
                };

                // Strip @_user_N placeholders and trim
                let text = self.lark_channel.strip_at_placeholders(&text);
                let text = text.trim();
                if text.is_empty() {
                    return Ok(());
                }

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let channel_message = ChannelMessage {
                    id: message_id,
                    sender: chat_id.to_string(), // Use chat_id for sending replies
                    content: text.to_string(),
                    channel: "lark".to_string(),
                    timestamp,
                };

                // Send to channel (non-blocking)
                if let Err(e) = self.tx.try_send(channel_message) {
                    tracing::error!("Failed to send message to channel: {}", e);
                }

                Ok(())
            }
        }

        // Create WebSocket client
        let handler = Box::new(ChannelEventHandler {
            tx: tx.clone(),
            allowed_users: self.allowed_users.clone(),
            lark_channel: LarkChannel::new(
                self.app_id.clone(),
                self.app_secret.clone(),
                None,
                None,
                self.allowed_users.clone(),
                self.use_feishu,
            ),
            seen_messages: Arc::clone(&self.seen_messages),
        });

        let ws_client = LarkWebSocketClient::new(
            self.app_id.clone(),
            self.app_secret.clone(),
            self.use_feishu,
        )
        .with_event_handler(handler);

        // Start the WebSocket connection (this will auto-reconnect)
        ws_client
            .start()
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket client error: {}", e))?;

        Ok(())
    }
}

/// Escape JSON string for embedding in JSON (similar to NullClaw's appendJsonStringW)
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c < ' ' => result.push_str(&format!("\\u{:04x}", c as u32)),
            _ => result.push(c),
        }
    }
    result
}

/// Convert text message to Lark/Feishu text content format
/// Matches NullClaw's simple text message format
fn text_to_lark_content(text: &str) -> String {
    // Build inner content JSON: {"text":"..."}
    format!("{{\"text\":\"{}\"}}", escape_json_string(text))
}

#[async_trait]
impl Channel for LarkChannel {
    fn name(&self) -> &str {
        "lark"
    }

    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
        let token = self
            .get_tenant_access_token()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get tenant access token"))?;

        // Use chat_id as receive_id_type (matches NullClaw's approach)
        let url = format!(
            "{}/open-apis/im/v1/messages?receive_id_type=chat_id",
            self.base_url()
        );

        // Build inner content JSON: {"text":"..."}
        let content_json = text_to_lark_content(message);

        // Build outer body JSON (manually to escape the inner JSON string)
        let body = format!(
            r#"{{"receive_id":"{}","msg_type":"text","content":"{}"}}"#,
            escape_json_string(recipient),
            escape_json_string(&content_json)
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Lark send message failed ({status}): {error_body}");
        }

        tracing::debug!("Lark message sent to {}", recipient);
        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        match self.connection_mode.as_str() {
            MODE_WEBHOOK => {
                // Webhook mode - listen is a no-op, events come via HTTP
                tracing::info!("Lark/Feishu webhook mode - waiting for events via HTTP");
                #[allow(clippy::empty_loop)]
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            }
            MODE_POLLING => LarkChannel::listen_polling(self, tx).await,
            MODE_LONG_CONNECTION => LarkChannel::listen_long_connection(self, tx).await,
            _ => {
                tracing::warn!(
                    "Unknown connection mode '{}', defaulting to webhook",
                    self.connection_mode
                );
                #[allow(clippy::empty_loop)]
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        let _token = match self.get_tenant_access_token().await {
            Some(t) => t,
            None => return false,
        };

        let url = format!(
            "{}/open-apis/auth/v3/tenant_access_token/internal",
            self.base_url()
        );

        tokio::time::timeout(Duration::from_secs(5), self.client.get(&url).send())
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|resp| resp.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lark_channel_name() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec!["*".into()],
            false,
        );
        assert_eq!(ch.name(), "lark");
    }

    #[test]
    fn lark_base_url_international() {
        let ch = LarkChannel::new("cli_xxx".into(), "secret".into(), None, None, vec![], false);
        assert_eq!(ch.base_url(), LARK_BASE_URL);
    }

    #[test]
    fn lark_base_url_chinese() {
        let ch = LarkChannel::new("cli_xxx".into(), "secret".into(), None, None, vec![], true);
        assert_eq!(ch.base_url(), FEISHU_BASE_URL);
    }

    #[test]
    fn lark_user_allowed_wildcard() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec!["*".into()],
            false,
        );
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn lark_user_allowed_specific() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec!["ou_xxx".into(), "on_yyy".into()],
            false,
        );
        assert!(ch.is_user_allowed("ou_xxx"));
        assert!(ch.is_user_allowed("on_yyy"));
        assert!(!ch.is_user_allowed("ou_zzz"));
    }

    #[tokio::test]
    async fn lark_get_tenant_access_token_returns_cached() {
        let ch = LarkChannel::new(
            "fake_app_id".into(),
            "fake_secret".into(),
            None,
            None,
            vec![],
            false,
        );

        // First call - will attempt to fetch (and fail with fake credentials)
        let result1 = ch.get_tenant_access_token().await;
        assert!(result1.is_none());

        // Manually set a token to test caching
        let token = "test_token".to_string();
        let expiry = Instant::now() + Duration::from_secs(3600);
        *ch.tenant_access_token.lock().await = Some(token.clone());
        *ch.token_expiry.lock().await = Some(expiry);

        // Second call - should return cached token
        let result2 = ch.get_tenant_access_token().await;
        assert_eq!(result2, Some(token));
    }

    #[tokio::test]
    async fn lark_send_builds_correct_request() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec!["*".into()],
            false,
        );

        // Set a fake token with valid expiry to avoid API call
        *ch.tenant_access_token.lock().await = Some("fake_token".into());
        *ch.token_expiry.lock().await = Some(Instant::now() + Duration::from_secs(3600));

        let result = ch.send("Hello, world!", "ou_xxx").await;

        // Should fail with network error, not a panic
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Network or HTTP errors should contain these keywords
        assert!(
            err.contains("error")
                || err.contains("failed")
                || err.contains("connect")
                || err.contains("http")
                || err.contains("reqwest")
        );
    }

    #[tokio::test]
    async fn lark_parse_webhook_extracts_text_message() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            Some("encrypt_key".into()),
            Some("verify_token".into()),
            vec!["ou_xxx".into()],
            false,
        );

        let payload = serde_json::json!({
            "schema": "2.0",
            "header": {
                "event_id": "evn_xxx",
                "timestamp": "1.7000000000000000000000000000E+18",
                "event_type": "im.message.receive_v1",
                "tenant_key": "xxx",
                "app_id": "cli_xxx"
            },
            "event": {
                "sender": {
                    "sender_id": {
                        "open_id": "ou_xxx"
                    }
                },
                "message": {
                    "message_id": "om_xxx",
                    "chat_type": "p2p",
                    "chat_id": "oc_xxx",
                    "content": "{\"text\":\"Hello, bot!\"}"
                }
            }
        });

        let messages = ch.parse_webhook_payload(&payload).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "oc_xxx"); // Use chat_id for sending replies
        assert_eq!(messages[0].content, "Hello, bot!");
    }

    #[tokio::test]
    async fn lark_parse_webhook_filters_unauthorized_users() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec!["ou_yyy".into()], // Different user
            false,
        );

        let payload = serde_json::json!({
            "schema": "2.0",
            "header": {
                "event_id": "evn_xxx",
                "timestamp": "1.7000000000000000000000000000E+18",
                "event_type": "im.message.receive_v1",
                "tenant_key": "xxx",
                "app_id": "cli_xxx"
            },
            "event": {
                "sender": {
                    "sender_id": {
                        "open_id": "ou_xxx"
                    }
                },
                "message": {
                    "message_id": "om_xxx",
                    "chat_type": "p2p",
                    "chat_id": "oc_xxx",
                    "content": "{\"text\":\"Hello!\"}"
                }
            }
        });

        let messages = ch.parse_webhook_payload(&payload).await;
        assert_eq!(messages.len(), 0); // Unauthorized user filtered out
    }

    #[tokio::test]
    async fn lark_health_check_returns_false_without_credentials() {
        let ch = LarkChannel::new(
            "invalid".into(),
            "invalid".into(),
            None,
            None,
            vec![],
            false,
        );

        // Should not panic, should return false (API will fail)
        let result = ch.health_check().await;
        assert!(!result);
    }
}
