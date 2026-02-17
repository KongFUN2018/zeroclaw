# Lark/Feishu Channel Design

**Date**: 2026-02-17
**Status**: Design Approved
**Author**: AI Assistant + User Collaboration

## Overview

Add support for Lark (international) and Feishu (Chinese) messaging platforms to ZeroClaw via a unified `LarkChannel` implementation. This design enables ZeroClaw to receive and send messages through Lark/Feishu using webhook push mode with full interactive card support.

## Requirements

### Functional Requirements
- Receive messages from Lark/Feishu via webhooks
- Send text, rich text, and interactive card messages to users
- Support both 1:1 direct messages and group @mentions
- Handle user ID types: user_id, open_id, union_id
- Support both Lark (international) and Feishu (Chinese) endpoints
- Convert AI Markdown responses to Lark/Feishu interactive cards
- Support image/file message types

### Non-Functional Requirements
- Secure webhook validation and optional message encryption
- Automatic tenant_access_token refresh
- Graceful error handling and retry logic
- Health check capability
- Thread-safe concurrent message handling
- Rate limiting for webhook endpoints

## Architecture

### Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Webhook Access Layer                     │
│  (src/gateway/mod.rs: /lark/webhook, /feishu/webhook)       │
│  - URL verification (verification_token)                    │
│  - Message decryption (encrypt_key, optional)               │
│  - Rate limiting & security checks                          │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                     Message Processing Layer                 │
│           (src/channels/lark.rs: LarkChannel)                │
│  - parse_webhook_payload() - Parse event JSON               │
│  - User ID type detection (user_id/open_id/union_id)        │
│  - Markdown → Card converter                                │
│  - Event type handling (im.message.receive_v1)              │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                       API Call Layer                         │
│                    (reqwest + Lark API)                      │
│  - POST /open-apis/im/v1/messages - Send messages           │
│  - GET /open-apis/user/v4/users/{id} - Get user info        │
│  - tenant_access_token auto-refresh                         │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. User sends message in Lark/Feishu
2. Lark/Feishu server pushes webhook to ZeroClaw gateway
3. Gateway validates and decrypts (if enabled)
4. LarkChannel parses event and extracts message content
5. Message routed to Agent Loop for AI processing
6. AI response converted to Lark/Feishu card format
7. Response sent via Lark/Feishu API to user

## Message Format Design

### Incoming Message Types

1. **Text Messages**
   - Extract from `content.text` field
   - Directly forward to AI

2. **@Bot Messages**
   - Detect via `event.mentions` array
   - Check if current bot_id is mentioned
   - Extract message content

3. **Rich Text Messages**
   - Parse Lark/Feishu rich text format
   - Convert to Markdown for AI processing

4. **Image/File Messages**
   - Extract `file_key` from message
   - Download via API or generate temporary URL
   - Forward to AI for analysis

### Outgoing Message Formats

AI response intelligently formatted based on content:

1. **Plain Text**
   - Simple responses
   - API: `message_type: "text"`

2. **Markdown → Interactive Card** (Primary)
   - AI returns Markdown
   - Convert to Lark/Feishu card elements:
     - Headers → `card.header`
     - Code blocks → `element.pre`
     - Lists → `element.div`
     - Links → `element.a`
     - Interactive buttons (optional)

3. **Images**
   - AI generates images
   - Upload to Lark/Feishu
   - Return image message

4. **Error Messages**
   - Red alert card for errors

## Security & Permissions

### User Authorization

Multi-layer permission checks:
- **User whitelist**: `allowed_users` supports:
  - `"*"` - Allow all users (not recommended for production)
  - `"ou_xxx"` - Specific user_id
  - `"oc_xxx"`, `"on_xxx"` - Mixed open_id/union_id
- **Automatic ID type detection**: Detect by prefix and adapt API calls
- **Logging**: Warn on unauthorized access with setup instructions

### Webhook Security

- **URL verification**: Verify initial webhook setup with `verification_token`
- **Message decryption (optional)**:
  - If `encrypt_key` configured: AES-256-CBC decrypt message body
  - If not configured: Accept unencrypted messages (dev/testing)
- **Signature verification**: Verify request signature to prevent forgery
- **Rate limiting**: Reuse gateway's `SlidingWindowRateLimiter`

### API Security

- **Token auto-refresh**:
  - `tenant_access_token` expires in 2 hours
  - Refresh 5 minutes before expiry
  - Thread-safe with `Mutex`
- **Retry logic**: Exponential backoff on failures (max 3 retries)
- **Sensitive data protection**: Never log `app_secret` or `encrypt_key`

## Error Handling

### Webhook Layer Errors
- JSON parse error: 400 Bad Request + log
- Verification failed: 401 Unauthorized
- Rate limit exceeded: 429 Too Many Requests
- Success: 200 OK (even if ignored, prevent Lark retry)

### Channel Layer Errors
- API call failure: Retry 3x with backoff
- Token refresh failure: Critical error log, enter degraded mode
- Send message failure: Log + send error hint to user

### Application Layer Errors
- AI timeout: Friendly "timeout, please retry" message
- AI error: Return sanitized error details

## Implementation Details

### File Structure

```
src/
├── channels/
│   ├── lark.rs          # New: Lark/Feishu channel implementation
│   └── mod.rs           # Modify: Add mod/export
├── gateway/
│   └── mod.rs           # Modify: Add /lark and /feishu routes
├── config/
│   └── schema.rs        # No change needed (LarkConfig exists)
└── main.rs              # Modify: Add LarkChannel to start_channels()
```

### Core Data Structure

```rust
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
```

### Webhook Integration

Add to `src/gateway/mod.rs`:
```rust
// Lark/Feishu webhook endpoints
router = router.route("/lark/webhook", post(lark_webhook_handler));
router = router.route("/lark/webhook", get(lark_challenge_handler));
router = router.route("/feishu/webhook", post(feishu_webhook_handler));
router = router.route("/feishu/webhook", get(feishu_challenge_handler));
```

### Channel Startup Integration

Add to `start_channels()`:
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

### Dependencies

Add to `Cargo.toml`:
```toml
aes = "0.8"
cbc = "0.1"
hex = "0.4"
```

### Configuration Example

```toml
[channels_config.lark]
app_id = "cli_xxx"
app_secret = "xxx"
encrypt_key = "xxx"              # Optional, for message decryption
verification_token = "xxx"       # Optional, for webhook verification
allowed_users = ["ou_xxx", "on_xxx", "*"]  # Mixed user_id/open_id/union_id
use_feishu = true               # true=Feishu CN, false=Lark International
```

## Health Check

```rust
async fn health_check(&self) -> bool {
    // 1. Check token validity
    let token = self.get_tenant_access_token().await;
    if token.is_none() {
        return false;
    }

    // 2. Test API connectivity
    let url = if self.use_feishu {
        "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal"
    } else {
        "https://open.larksuite.com/open-apis/auth/v3/tenant_access_token/internal"
    };

    self.client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .is_ok()
}
```

## Testing Plan

1. **Unit Tests**
   - Message parsing logic
   - User ID type detection
   - Markdown to card conversion
   - Encryption/decryption

2. **Integration Tests**
   - Webhook endpoint with Lark/Feishu test environment
   - Token refresh logic
   - API call success/failure scenarios

3. **Security Tests**
   - Encryption/decryption correctness
   - Signature verification
   - Rate limiting
   - Unauthorized access handling

## Deployment Considerations

1. **Public Endpoint**: Webhook requires public URL or tunnel (ngrok, frp)
2. **Configuration**: Run `zeroclaw onboard` to set up Lark credentials
3. **Verification**: First webhook setup triggers URL verification
4. **Monitoring**: Use `zeroclaw channel doctor` to check health

## Future Enhancements

1. Support for more interactive card elements (input forms, date pickers)
2. Message threading and conversation history
3. File upload/download handling
4. Sticker/emoji reactions
5. Multi-tenant support

## References

- [Lark Open API Documentation](https://open.larksuite.com/document)
- [Feishu Open API Documentation](https://open.feishu.cn/document)
- [Channel Trait Definition](src/channels/traits.rs)
- [Existing WhatsApp Webhook Implementation](src/channels/whatsapp.rs)
- [Gateway Module](src/gateway/mod.rs)
