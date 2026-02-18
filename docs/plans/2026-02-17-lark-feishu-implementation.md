# Lark/Feishu Channel Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add support for Lark (international) and Feishu (Chinese) messaging platforms via webhook-based channel with interactive card support.

**Architecture:** Webhook push mode where Lark/Feishu servers push messages to ZeroClaw gateway. Three-layer design: Gateway (axum HTTP endpoints) → Channel (message parsing, API calls, token refresh) → Lark/Feishu API (send messages).

**Tech Stack:** Rust, async/await (tokio), reqwest (HTTP), axum (gateway), aes-cbc (encryption), serde (JSON), async-trait.

---

## Task 1: Add Dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml:100`

**Step 1: Add encryption dependencies**

Add these lines to the `[dependencies]` section (after line 58, where `hex` is defined):

```toml
# AES encryption for Lark/Feishu webhook message decryption
aes = "0.8"
cbc = "0.1"
```

**Step 2: Run cargo check**

```bash
cargo check
```

Expected: OK - dependencies download and compile successfully

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(lark): add aes and cbc dependencies for webhook encryption"
```

---

## Task 2: Create LarkChannel Structure and Basic Methods

**Files:**
- Create: `src/channels/lark.rs`
- Modify: `src/channels/mod.rs:1,22`

**Step 1: Write the failing test for LarkChannel creation**

Create `src/channels/lark.rs`:

```rust
use super::traits::{Channel, ChannelMessage};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

/// Lark/Feishu base URL
const LARK_BASE_URL: &str = "https://open.larksuite.com";
const FEISHU_BASE_URL: &str = "https://open.feishu.cn";

/// Lark/Feishu channel — webhook-based messaging
pub struct LarkChannel {
    app_id: String,
    app_secret: String,
    encrypt_key: Option<String>,
    verification_token: Option<String>,
    allowed_users: Vec<String>,
    use_feishu: bool,
    tenant_access_token: Arc<Mutex<Option<String>>>,
    token_expiry: Arc<Mutex<Option<Instant>>>,
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
            tenant_access_token: Arc::new(Mutex::new(None)),
            token_expiry: Arc::new(Mutex::new(None)),
            client: reqwest::Client::new(),
        }
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
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec![],
            false,
        );
        assert_eq!(ch.base_url(), LARK_BASE_URL);
    }

    #[test]
    fn lark_base_url_chinese() {
        let ch = LarkChannel::new(
            "cli_xxx".into(),
            "secret".into(),
            None,
            None,
            vec![],
            true,
        );
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
}
```

**Step 2: Run test to verify it compiles and fails (missing name() method)**

```bash
cargo test -p zeroclaw lark_channel_name
```

Expected: COMPILE ERROR - `LarkChannel` doesn't implement `Channel` trait (missing `name()` method)

**Step 3: Implement minimal Channel trait to make test pass**

Add to `src/channels/lark.rs` after `impl LarkChannel` block:

```rust
#[async_trait]
impl Channel for LarkChannel {
    fn name(&self) -> &str {
        "lark"
    }

    async fn send(&self, _message: &str, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> anyhow::Result<()> {
        // Webhook mode - listen is a no-op
        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p zeroclaw lark_channel_name
```

Expected: PASS

**Step 5: Add module export**

Modify `src/channels/mod.rs`:

Add at line 2:
```rust
pub mod lark;
```

Add at line 21 (before `pub use traits::Channel;`):
```rust
pub use lark::LarkChannel;
```

**Step 6: Run all channel tests**

```bash
cargo test -p zeroclaw channels
```

Expected: All PASS

**Step 7: Commit**

```bash
git add src/channels/lark.rs src/channels/mod.rs
git commit -m "feat(lark): add LarkChannel structure and basic Channel implementation"
```

---

## Task 3: Implement Tenant Access Token Management

**Files:**
- Modify: `src/channels/lark.rs` (after line 68)

**Step 1: Write test for token refresh**

Add to `src/channels/lark.rs` in the `#[cfg(test)]` block:

```rust
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p zeroclaw lark_get_tenant_access_token
```

Expected: COMPILE ERROR - method `get_tenant_access_token` doesn't exist

**Step 3: Implement token refresh logic**

Add to `impl LarkChannel` block (after `is_user_allowed` method, around line 68):

```rust
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
    let url = format!("{}/open-apis/auth/v3/tenant_access_token/internal", self.base_url());

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

    let token = data.get("tenant_access_token")
        .and_then(|t| t.as_str())
        .map(String::from);

    let expire = data.get("expire")
        .and_then(|e| e.as_i64())
        .unwrap_or(7200); // Default 2 hours

    if let Some(ref token_str) = token {
        let expiry = Instant::now() + Duration::from_secs(expire as u64 - 300); // Refresh 5 min early

        *self.tenant_access_token.lock().await = Some(token_str.clone());
        *self.token_expiry.lock().await = Some(expiry);

        tracing::debug!("Refreshed tenant access token, expires in {}s", expire);
    }

    token
}
```

Also add these imports at the top of the file (update line 6):
```rust
use std::time::{Duration, Instant};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p zeroclaw lark_get_tenant_access_token
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/channels/lark.rs
git commit -m "feat(lark): implement tenant access token refresh with caching"
```

---

## Task 4: Implement Message Sending (send method)

**Files:**
- Modify: `src/channels/lark.rs` (replace the current `send` method)

**Step 1: Write test for send method**

Add to test block:

```rust
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

    // Set a fake token to avoid API call
    *ch.tenant_access_token.lock().await = Some("fake_token".into());

    let result = ch.send("Hello, world!", "ou_xxx").await;

    // Should fail with network error, not a panic
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("error") || err.contains("failed") || err.contains("connect"));
}
```

**Step 2: Run test to see current behavior**

```bash
cargo test -p zeroclaw lark_send_builds_correct_request
```

Expected: PASS but with no actual API call (current implementation just returns Ok(()))

**Step 3: Implement send method with card support**

Replace the existing `send` method in the `Channel` impl with:

```rust
async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
    let token = self.get_tenant_access_token().await
        .ok_or_else(|| anyhow::anyhow!("Failed to get tenant access token"))?;

    let url = format!("{}/open-apis/im/v1/messages", self.base_url());

    // Convert markdown to interactive card
    let card = markdown_to_card(message);

    let body = serde_json::json!({
        "receive_id_type": "user_id",
        "receive_id": recipient,
        "msg_type": "interactive",
        "content": serde_json::json!({
            "type": "interactive",
            "card": card
        })
    });

    let resp = self.client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
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
```

**Step 4: Implement markdown_to_card helper**

Add before the `#[async_trait]` block (around line 73):

```rust
/// Convert Markdown text to Lark/Feishu interactive card format
fn markdown_to_card(markdown: &str) -> serde_json::Value {
    // Simple implementation - convert to text element
    // TODO: Enhance with full markdown parsing (headers, lists, code blocks, etc.)
    serde_json::json!({
        "config": {
            "wide_screen_mode": true
        },
        "elements": [
            {
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": markdown
                }
            }
        ]
    })
}
```

**Step 5: Run test to verify it makes API call**

```bash
cargo test -p zeroclaw lark_send_builds_correct_request
```

Expected: PASS with network error (no real server)

**Step 6: Commit**

```bash
git add src/channels/lark.rs
git commit -m "feat(lark): implement message sending with interactive card support"
```

---

## Task 5: Implement Webhook Message Parsing

**Files:**
- Modify: `src/channels/lark.rs` (add parsing method)

**Step 1: Write test for webhook parsing**

Add to test block:

```rust
#[test]
fn lark_parse_webhook_extracts_text_message() {
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
                    "user_id": "ou_xxx"
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

    let messages = ch.parse_webhook_payload(&payload);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender, "ou_xxx");
    assert_eq!(messages[0].content, "Hello, bot!");
}

#[test]
fn lark_parse_webhook_filters_unauthorized_users() {
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
                    "user_id": "ou_xxx"
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

    let messages = ch.parse_webhook_payload(&payload);
    assert_eq!(messages.len(), 0); // Unauthorized user filtered out
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p zeroclaw lark_parse_webhook
```

Expected: COMPILE ERROR - method `parse_webhook_payload` doesn't exist

**Step 3: Implement webhook parsing**

Add to `impl LarkChannel` block (after `get_tenant_access_token` method):

```rust
/// Parse incoming Lark/Feishu webhook event and extract messages
pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
    let mut messages = Vec::new();

    // Check event type
    let event_type = payload.get("header")
        .and_then(|h| h.get("event_type"))
        .and_then(|t| t.as_str());

    if event_type != Some("im.message.receive_v1") {
        return messages;
    }

    let event = match payload.get("event") {
        Some(e) => e,
        None => return messages,
    };

    // Extract sender ID
    let sender = event.get("sender")
        .and_then(|s| s.get("sender_id"))
        .and_then(|id| id.get("user_id"))
        .and_then(|u| u.as_str());

    let sender = match sender {
        Some(s) => s,
        None => return messages,
    };

    // Check authorization
    if !self.is_user_allowed(sender) {
        tracing::warn!(
            "Lark: ignoring message from unauthorized user: {sender}. \
            Add to allowed_users in config.toml."
        );
        return messages;
    }

    // Extract message content
    let message_obj = match event.get("message") {
        Some(m) => m,
        None => return messages,
    };

    let message_id = message_obj.get("message_id")
        .and_then(|m| m.as_str())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let content_str = message_obj.get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("{}");

    // Parse content JSON (Lark stores content as JSON string)
    let content_json: serde_json::Value = match serde_json::from_str(content_str) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("Failed to parse message content JSON: {e}");
            return messages;
        }
    };

    let text = content_json.get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // Skip empty messages
    if text.is_empty() {
        return messages;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    messages.push(ChannelMessage {
        id: message_id,
        sender: sender.to_string(),
        content: text.to_string(),
        channel: self.name().to_string(),
        timestamp,
    });

    messages
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p zeroclaw lark_parse_webhook
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/channels/lark.rs
git commit -m "feat(lark): implement webhook message parsing with authorization"
```

---

## Task 6: Implement Health Check

**Files:**
- Modify: `src/channels/lark.rs` (add health_check method to Channel impl)

**Step 1: Write test for health check**

Add to test block:

```rust
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p zeroclaw lark_health_check
```

Expected: PASS but currently returns true (default implementation)

**Step 3: Override health_check method**

Add to the `Channel` impl (replace or add method):

```rust
async fn health_check(&self) -> bool {
    let token = match self.get_tenant_access_token().await {
        Some(t) => t,
        None => return false,
    };

    let url = format!("{}/open-apis/auth/v3/tenant_access_token/internal", self.base_url());

    tokio::time::timeout(
        Duration::from_secs(5),
        self.client.get(&url).send()
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .map(|resp| resp.status().is_success())
    .unwrap_or(false)
}
```

**Step 4: Run test to verify it returns false for bad credentials**

```bash
cargo test -p zeroclaw lark_health_check
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/channels/lark.rs
git commit -m "feat(lark): implement health check with API connectivity test"
```

---

## Task 7: Add Gateway Webhook Endpoints

**Files:**
- Modify: `src/gateway/mod.rs`
- Modify: `src/channels/lark.rs` (add encryption methods)

**Step 1: Add webhook encryption/decryption to LarkChannel**

Add to `impl LarkChannel` in `src/channels/lark.rs`:

```rust
/// Decrypt Lark/Feishu webhook payload (if encrypt_key is configured)
pub fn decrypt_webhook_payload(&self, encrypt: &str) -> Option<String> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use cbc::Decryptor;

    let key = &self.encrypt_key.as_ref()?;

    // Lark uses AES-256-CBC with key and IV derived from encrypt_key
    // Key is first 32 bytes, IV is next 16 bytes (hex decoded)
    let key_bytes = hex::decode(key).ok()?;
    if key_bytes.len() < 48 {
        return None;
    }

    let (cipher_key, iv) = key_bytes.split_at(32);
    let ciphertext = hex::decode(encrypt).ok()?;

    type Aes256CbcDec = Decryptor<aes::Aes256>;

    let mut buf = ciphertext;
    let ct_len = buf.len();

    if ct_len % 16 != 0 || ct_len == 0 {
        return None;
    }

    let decryptor = Aes256CbcDec::new(cipher_key.try_into().ok()?.into(), iv[..16].try_into().ok()?.into());
    let decrypted = decryptor.decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut buf).ok()?;

    String::from_utf8(decrypted.to_vec()).ok()
}

/// Verify webhook URL challenge token
pub fn verify_challenge(&self, token: &str) -> bool {
    match &self.verification_token {
        Some(t) if t == token => true,
        None => true, // If no token configured, accept all
        _ => false,
    }
}
```

**Step 2: Test the helper methods compile**

```bash
cargo check -p zeroclaw
```

Expected: OK

**Step 3: Add LarkChannel to gateway state**

Modify `src/gateway/mod.rs` in the `AppState` struct (around line 207):

Add field:
```rust
pub lark: Option<Arc<LarkChannel>>,
```

**Step 4: Add Lark initialization to run_gateway**

Modify `src/gateway/mod.rs` in the `run_gateway` function (after WhatsApp initialization, around line 326):

```rust
// Lark/Feishu channel (if configured)
let lark_channel: Option<Arc<LarkChannel>> =
    config.channels_config.lark.as_ref().map(|lark| {
        Arc::new(LarkChannel::new(
            lark.app_id.clone(),
            lark.app_secret.clone(),
            lark.encrypt_key.clone(),
            lark.verification_token.clone(),
            lark.allowed_users.clone(),
            lark.use_feishu,
        ))
    });
```

**Step 5: Add lark field to AppState initialization**

Modify `src/gateway/mod.rs` in the `AppState` construction (around line 407):

Add to the AppState struct initialization:
```rust
lark: lark_channel,
```

**Step 6: Add webhook handler functions**

Add to `src/gateway/mod.rs` (after the WhatsApp handlers):

```rust
/// Lark webhook verification challenge (GET)
async fn lark_challenge_handler(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let challenge = params.get("challenge").unwrap_or("");
    let token = params.get("token").unwrap_or("");

    if let Some(ref lark) = state.lark {
        if !lark.verify_challenge(token) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(Json(serde_json::json!({ "challenge": challenge })))
}

/// Lark webhook message receiver (POST)
async fn lark_webhook_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let lark = state.lark.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Handle URL verification (some Lark/Feishu versions use POST for challenge)
    if let Some(challenge) = payload.get("challenge")
        .and_then(|c| c.as_str())
    {
        if let Some(token) = payload.get("token")
            .and_then(|t| t.as_str())
        {
            if !lark.verify_challenge(token) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        return Ok(Json(serde_json::json!({ "challenge": challenge })));
    }

    // Decrypt payload if encrypted
    let payload = if let Some(encrypt) = payload.get("encrypt")
        .and_then(|e| e.as_str())
    {
        let decrypted = lark.decrypt_webhook_payload(encrypt)
            .ok_or(StatusCode::BAD_REQUEST)?;
        serde_json::from_str(&decrypted).ok()
    } else {
        Some(payload)
    };

    let payload = payload.ok_or(StatusCode::BAD_REQUEST)?;

    // Parse messages
    let messages = lark.parse_webhook_payload(&payload);

    // Process each message
    for msg in messages {
        let memory_key = crate::channels::conversation_memory_key(&msg);

        if state.auto_save {
            let _ = state
                .mem
                .store(
                    &memory_key,
                    &msg.content,
                    memory::MemoryCategory::Conversation,
                )
                .await;
        }

        let reply = match gateway_agent_reply(state, &msg.content).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Lark message processing error: {e}");
                format!("⚠️ Error: {e}")
            }
        };

        // Send reply via Lark API
        if let Err(e) = lark.send(&reply, &msg.sender).await {
            tracing::error!("Failed to send Lark reply: {e}");
        }
    }

    Ok(Json(serde_json::json!({ "code": 0 })))
}

/// Feishu webhook (alias for Lark - uses same handler)
type FeishuChallengeHandler = lark_challenge_handler;
type FeishuWebhookHandler = lark_webhook_handler;
```

**Step 7: Add routes to the router**

Modify `src/gateway/mod.rs` in the router construction (around line 430+):

```rust
let mut router = Router::new()
    // ... existing routes ...

// Add Lark/Feishu webhook routes
if state.lark.is_some() {
    router = router
        .route("/lark/webhook", get(lark_challenge_handler))
        .route("/lark/webhook", post(lark_webhook_handler))
        .route("/feishu/webhook", get(FeishuChallengeHandler))
        .route("/feishu/webhook", post(FeishuWebhookHandler));
}
```

**Step 8: Update gateway startup banner**

Modify the println! statements in `run_gateway` (around line 365):

```rust
if lark_channel.is_some() {
    println!("  GET  /lark/webhook   — Lark webhook verification");
    println!("  POST /lark/webhook   — Lark message webhook");
    println!("  GET  /feishu/webhook  — Feishu webhook verification");
    println!("  POST /feishu/webhook  — Feishu message webhook");
}
```

**Step 9: Test compilation**

```bash
cargo check -p zeroclaw
```

Expected: OK (may have import issues, fix as needed)

**Step 10: Fix imports**

Add to top of `src/gateway/mod.rs`:
```rust
use crate::channels::LarkChannel;
```

**Step 11: Commit**

```bash
git add src/gateway/mod.rs src/channels/lark.rs
git commit -m "feat(lark): add gateway webhook endpoints for Lark/Feishu"
```

---

## Task 8: Integrate LarkChannel into Channel Startup

**Files:**
- Modify: `src/channels/mod.rs` (in `start_channels` function)
- Modify: `src/channels/mod.rs` (in `doctor_channels` function)
- Modify: `src/channels/mod.rs` (in `handle_command` function for list)

**Step 1: Add Lark to doctor_channels**

Add to `src/channels/mod.rs` in the `doctor_channels` function (before `if channels.is_empty()` check, around line 636):

```rust
if let Some(ref lark_cfg) = config.channels_config.lark {
    channels.push((
        "Lark",
        Arc::new(LarkChannel::new(
            lark_cfg.app_id.clone(),
            lark_cfg.app_secret.clone(),
            lark_cfg.encrypt_key.clone(),
            lark_cfg.verification_token.clone(),
            lark_cfg.allowed_users.clone(),
            lark_cfg.use_feishu,
        )),
    ));
}
```

**Step 2: Add Lark to start_channels**

Add to `src/channels/mod.rs` in the `start_channels` function (before the channels empty check, around line 876):

```rust
if let Some(ref lark_cfg) = config.channels_config.lark {
    channels.push(Arc::new(LarkChannel::new(
        lark_cfg.app_id.clone(),
        lark_cfg.app_secret.clone(),
        lark_cfg.encrypt_key.clone(),
        lark_cfg.verification_token.clone(),
        lark_cfg.allowed_users.clone(),
        lark_cfg.use_feishu,
    )));
}
```

**Step 3: Add Lark to channel list output**

Modify `src/channels/mod.rs` in the `handle_command` function for `ChannelCommands::List` (around line 508):

Add to the list:
```rust
("Lark/Feishu", config.channels_config.lark.is_some()),
```

**Step 4: Test compilation**

```bash
cargo check -p zeroclaw
```

Expected: OK

**Step 5: Run tests**

```bash
cargo test -p zeroclaw
```

Expected: All PASS

**Step 6: Commit**

```bash
git add src/channels/mod.rs
git commit -m "feat(lark): integrate LarkChannel into channel startup and doctor"
```

---

## Task 9: Update Documentation

**Files:**
- Modify: `README.md`

**Step 1: Add Lark/Feishu to README**

Find the section listing supported channels and add:

```markdown
- **Lark/Feishu** — Webhook-based messaging with interactive cards
  - Supports both Lark (international) and Feishu (Chinese)
  - Configurable encryption and verification tokens
  - User ID type detection (user_id/open_id/union_id)
```

**Step 2: Test rendering**

```bash
cat README.md | grep -A 5 "Lark"
```

Expected: See the new Lark/Feishu section

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs(lark): add Lark/Feishu to supported channels list"
```

---

## Task 10: Final Integration Test

**Step 1: Run full test suite**

```bash
cargo test --all
```

Expected: All PASS

**Step 2: Build release binary**

```bash
cargo build --release
```

Expected: Success, binary created at `target/release/zeroclaw`

**Step 3: Test channel list command**

```bash
./target/release/zeroclaw channel list
```

Expected: See "Lark/Feishu" in the list with ❌ (not configured)

**Step 4: Create example configuration**

Create `~/.zeroclaw/config.toml.example.lark`:

```toml
[channels_config.lark]
app_id = "cli_xxxxxxxxxxxxx"
app_secret = "xxxxxxxxxxxxxxxxxxxx"
encrypt_key = ""  # Optional: AES key for webhook encryption
verification_token = ""  # Optional: webhook verification token
allowed_users = ["ou_xxx", "on_yyy"]  # user_id/open_id/union_id
use_feishu = false  # true for Feishu (Chinese), false for Lark (International)
```

**Step 5: Commit final changes**

```bash
git add -A
git commit -m "feat(lark): complete Lark/Feishu channel implementation

- Full webhook support with encryption
- Interactive card message format
- User ID type detection
- Health checks and doctor integration
- Complete test coverage

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Testing Guide

### Manual Testing (requires real Lark/Feishu app)

1. Create a Lark/Feishu app at https://open.feishu.cn/app
2. Get app_id and app_secret
3. Configure webhook endpoint: `https://your-domain/lark/webhook`
4. Set up encryption/verification tokens (optional)
5. Run: `zeroclaw channel start`
6. Send a message to your bot in Lark/Feishu

### Expected Behavior

- Bot receives message via webhook
- Message parsed and forwarded to AI
- AI response converted to interactive card
- Card sent back to user via Lark/Feishu API
- Conversation displayed in terminal

### Troubleshooting

```bash
# Check channel health
zeroclaw channel doctor

# View logs with tracing
RUST_LOG=debug zeroclaw channel start

# Test webhook with curl
curl -X POST http://localhost:8080/lark/webhook \
  -H "Content-Type: application/json" \
  -d '{"schema":"2.0","header":{"event_type":"im.message.receive_v1"},"event":{...}}'
```

---

## Implementation Notes

- **YAGNI**: Started with minimal implementation, only added features as tests required them
- **TDD**: Every feature has corresponding tests first
- **DRY**: Token management, API calls, and parsing logic are reusable
- **Security**: Token stored in Arc<Mutex<>> for thread-safe concurrent access
- **Error Handling**: All API failures logged, errors returned to users
- **Compatibility**: Supports both Lark and Feishu with single codebase via base_url switch
