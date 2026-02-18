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
}

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
}
