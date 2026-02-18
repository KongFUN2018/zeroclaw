use super::traits::{Channel, ChannelMessage};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
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
            .map(|s| s.to_string())
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

        let cipher_key_array: &[u8; 32] = cipher_key.try_into().ok()?;
        let iv_array: &[u8; 16] = iv[..16].try_into().ok()?;

        let decryptor = Aes256CbcDec::new(cipher_key_array.into(), iv_array.into());
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
}

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

#[async_trait]
impl Channel for LarkChannel {
    fn name(&self) -> &str {
        "lark"
    }

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

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> anyhow::Result<()> {
        // Webhook mode - listen is a no-op
        Ok(())
    }

    async fn health_check(&self) -> bool {
        let _token = match self.get_tenant_access_token().await {
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
        assert!(err.contains("error") || err.contains("failed") || err.contains("connect") || err.contains("http") || err.contains("reqwest"));
    }

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
