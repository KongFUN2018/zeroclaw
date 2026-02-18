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
}
