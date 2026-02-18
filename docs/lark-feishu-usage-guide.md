# Lark/Feishu Channel 使用指南

> ZeroClaw Lark/Feishu 渠道完整使用说明 - 测试、验证和生产部署

---

## 📋 目录

1. [快速开始](#快速开始)
2. [配置说明](#配置说明)
3. [本地测试](#本地测试)
4. [实际使用](#实际使用)
5. [故障排查](#故障排查)
6. [API 参考](#api-参考)

---

## 🚀 快速开始

### 前置条件

- ✅ ZeroClaw 已安装（当前在 `feat/lark-feishu` 分支）
- ✅ Lark 或 Feishu 开发者账号
- ✅ 可选：公网域名或隧道服务（Cloudflare、ngrok、Tailscale）

### 5 分钟快速测试

```bash
# 1. 确认在正确的分支
cd /Users/kongfun/Code/github/zeroclaw/.worktrees/feat-lark-feishu
git branch --show-current  # 应显示 feat/lark-feishu

# 2. 编译 release 版本
cargo build --release

# 3. 验证 Lark/Feishu 在渠道列表中
./target/release/zeroclaw channel list
# 应该看到: ❌ Lark/Feishu

# 4. 测试健康检查（无需配置即可运行）
./target/release/zeroclaw channel doctor
```

---

## ⚙️ 配置说明

### 步骤 1: 创建 Lark/Feishu 应用

#### Lark（国际版）
1. 访问 https://open.larksuite.com/app
2. 点击 "Create App" → "Custom App"
3. 填写应用信息：
   - **App Name**: ZeroClaw Bot
   - **App Description**: AI-powered bot
4. 获取凭证：
   - **App ID**: `cli_xxxxxxxxxxxxx` 格式
   - **App Secret**: 在 "Credentials & Basic Info" 中生成

#### Feishu（中国版）
1. 访问 https://open.feishu.cn/app
2. 点击 "创建自建应用"
3. 填写应用信息
4. 获取凭证（同上）

### 步骤 2: 配置权限

在应用管理后台，添加以下权限：

```
✅ 获取与发送单聊消息 (im:message)
✅ 获取用户基本信息 (contact:user.base:readonly)
✅ 接收事件 (im:message)
```

### 步骤 3: 配置事件订阅

1. 进入 "Events" → "Event Configuration"
2. 添加事件：
   ```
   ✅ im.message.receive_v1 - 接收消息
   ```
3. 配置请求地址：
   ```
   https://your-domain.com/lark/webhook
   或
   https://your-domain.com/feishu/webhook
   ```

### 步骤 4: 配置 ZeroClaw

编辑 `~/.zeroclaw/config.toml`：

```toml
[channels_config.lark]
app_id = "cli_xxxxxxxxxxxxx"              # 从 Lark/Feishu 控制台获取
app_secret = "xxxxxxxxxxxxxxxxxxxx"        # 从 Lark/Feishu 控制台获取
encrypt_key = ""                           # 可选：加密密钥
verification_token = ""                    # 可选：验证令牌
allowed_users = ["ou_xxx", "on_yyy"]      # 允许的用户 ID 列表
use_feishu = false                         # true=Feishu, false=Lark
```

#### 配置项说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `app_id` | string | ✅ | 应用 ID，格式 `cli_*` |
| `app_secret` | string | ✅ | 应用密钥 |
| `encrypt_key` | string | ❌ | AES 加密密钥（hex 格式），生产环境推荐 |
| `verification_token` | string | ❌ | Webhook 验证令牌 |
| `allowed_users` | array | ✅ | 允许使用机器人的用户 ID 列表 |
| `use_feishu` | boolean | ✅ | `true`=Feishu（中国），`false`=Lark（国际） |

#### 用户 ID 获取方式

在 Lark/Feishu 中，用户 ID 有三种格式：

- **user_id**: `ou_xxxxxxxxxxxxxxxx` - 用户唯一标识
- **open_id**: `ou_xxxxxxxxxxxxxxxx` - 应用维度唯一标识
- **union_id**: `on_xxxxxxxxxxxxxxxx` - 企业维度唯一标识

**推荐使用 user_id**，支持所有格式。

#### 通配符支持

```toml
allowed_users = ["*"]  # 允许所有用户（不推荐生产环境）
allowed_users = ["ou_*"]  # ⚠️ 不支持通配符前缀，必须完整 ID
```

---

## 🧪 本地测试

### 测试 1: 验证配置

```bash
# 检查配置是否正确加载
./target/release/zeroclaw channel list
# 应该看到: ✅ Lark/Feishu

# 运行健康检查
./target/release/zeroclaw channel doctor
# 应该显示:
# ✅ Lark  - OK (tenant_access_token 获取成功)
```

### 测试 2: 启动 Gateway（带 Webhook）

```bash
# 方式 1: 本地测试（需要隧道）
./target/release/zeroclaw gateway --host 127.0.0.1 --port 8080

# 应该看到:
# 🦀 ZeroClaw Gateway listening on http://127.0.0.1:8080
#   GET  /lark/webhook   — Lark webhook verification
#   POST /lark/webhook   — Lark message webhook
#   GET  /feishu/webhook  — Feishu webhook verification
#   POST /feishu/webhook  — Feishu message webhook
```

### 测试 3: 配置隧道（本地开发）

#### 使用 Cloudflare Tunnel（推荐）

```bash
# 安装 cloudflared
brew install cloudflared

# 启动隧道
cloudflared tunnel --url http://127.0.0.1:8080

# 会得到公网 URL，如: https://xxx.trycloudflare.com
```

#### 使用 ngrok

```bash
# 安装 ngrok
brew install ngrok

# 启动隧道
ngrok http 8080

# 会得到公网 URL，如: https://abc123.ngrok.io
```

#### 配置 ZeroClaw 内置隧道

编辑 `~/.zeroclaw/config.toml`：

```toml
[tunnel]
provider = "cloudflare"  # 或 "ngrok"
```

然后启动：

```bash
./target/release/zeroclaw gateway
# ZeroClaw 会自动启动隧道并显示公网 URL
```

### 测试 4: Webhook 验证

在 Lark/Feishu 控制台配置 webhook URL：

```
Verification URL:  https://your-domain.com/lark/webhook
                   或
                   https://your-domain.com/feishu/webhook
```

点击 "Verify" - 如果配置正确，会显示 ✅ 验证成功。

### 测试 5: 发送测试消息

1. 在 Lark/Feishu 中找到你的机器人
2. 发送消息: "你好"
3. 机器人应该回复 AI 生成的内容

---

## 🚀 实际使用

### 场景 1: 个人助手

```toml
[channels_config.lark]
app_id = "cli_xxx"
app_secret = "secret"
allowed_users = ["ou_your_user_id"]  # 只允许你自己
use_feishu = false
```

**使用方式**：直接与机器人私聊，提问任何问题。

### 场景 2: 团队助手

```toml
[channels_config.lark]
app_id = "cli_xxx"
app_secret = "secret"
allowed_users = [
    "ou_member_1",
    "ou_member_2",
    "ou_member_3"
]
use_feishu = false
```

**使用方式**：团队成员可以单独与机器人对话，获得 AI 辅助。

### 场景 3: 生产环境（安全配置）

```toml
[channels_config.lark]
app_id = "cli_xxx"
app_secret = "secret"
encrypt_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"  # 64 hex chars
verification_token = "your_random_token_here"
allowed_users = ["ou_xxx"]
use_feishu = false
```

**安全建议**：
- ✅ 始终设置 `encrypt_key`（AES-256 加密）
- ✅ 始终设置 `verification_token`（防止伪造请求）
- ✅ 使用 `allowed_users` 限制访问
- ✅ 定期轮换 `app_secret`

### 启动 Channel Server（持久化）

```bash
# 使用 systemd/supervisor 等工具管理
./target/release/zeroclaw channel start

# 机器人将：
# 1. 自动获取 tenant_access_token
# 2. 监听 webhook 事件
# 3. 处理消息并通过 AI 回复
```

---

## 🔍 故障排查

### 问题 1: Webhook 验证失败

**症状**：Lark 控制台显示 "Verification Failed"

**解决方案**：
```bash
# 检查日志
RUST_LOG=debug ./target/release/zeroclaw gateway

# 查看日志中是否有：
# [DEBUG] Lark challenge received: token=xxx

# 常见原因：
# 1. verification_token 不匹配 → 检查配置
# 2. URL 错误 → 确保是 /lark/webhook 或 /feishu/webhook
# 3. 防火墙阻止 → 检查端口是否开放
```

### 问题 2: 健康检查失败

**症状**：`channel doctor` 显示 Lark 不健康

**解决方案**：
```bash
# 1. 验证 app_id 和 app_secret
cat ~/.zeroclaw/config.toml | grep -A 5 "\[channels_config.lark\]"

# 2. 测试网络连接
curl https://open.larksuite.com/open-apis/auth/v3/tenant_access_token/internal
# 或
curl https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal

# 3. 检查是否选择了正确的平台
use_feishu = false  # Lark
use_feishu = true   # Feishu
```

### 问题 3: 消息发送失败

**症状**：机器人不回复

**解决方案**：
```bash
# 1. 检查日志
RUST_LOG=debug ./target/release/zeroclaw channel start

# 查找错误：
# [ERROR] Failed to send Lark reply: ...

# 2. 验证用户权限
# 检查你的 user_id 是否在 allowed_users 中

# 3. 检查 tenant_access_token
# Token 是否成功获取（查看日志）

# 4. 测试 API 连接
curl -X POST https://open.larksuite.com/open-apis/im/v1/messages \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "receive_id_type": "user_id",
    "receive_id": "ou_xxx",
    "msg_type": "text",
    "content": "{\"text\":\"测试\"}"
  }'
```

### 问题 4: 用户未授权

**症状**：日志显示 "ignoring message from unauthorized user"

**解决方案**：
```bash
# 1. 获取你的 user_id
# 在 Lark/Feishu 中点击头像 → 设置 → 基本信息
# 或发送消息后查看日志

# 2. 更新配置
vim ~/.zeroclaw/config.toml
# 修改 allowed_users

# 3. 重启服务
./target/release/zeroclaw channel start
```

### 问题 5: 加密解密失败

**症状**：webhook 解密错误

**解决方案**：
```toml
# 确认 encrypt_key 格式正确
encrypt_key = "0123456789abcdef..."  # 必须是 64 个 hex 字符（48 字节）

# 如果不需要加密，设置为空
encrypt_key = ""
```

---

## 📚 API 参考

### Webhook 端点

#### GET /lark/webhook
**用途**：Webhook URL 验证

**Query Parameters**:
```
challenge: string - 验证码
token: string - 验证令牌
```

**Response**:
```json
{
  "challenge": "echoed_challenge"
}
```

#### POST /lark/webhook
**用途**：接收消息事件

**Request Body**:
```json
{
  "schema": "2.0",
  "header": {
    "event_id": "evn_xxx",
    "event_type": "im.message.receive_v1",
    ...
  },
  "event": {
    "sender": {
      "sender_id": {
        "user_id": "ou_xxx"
      }
    },
    "message": {
      "message_id": "om_xxx",
      "content": "{\"text\":\"Hello\"}"
    }
  }
}
```

**Response**:
```json
{
  "code": 0
}
```

### 支持的消息格式

Lark/Feishu Channel 支持以下消息格式：

1. **纯文本**：自动转换为交互式卡片
2. **Markdown**：完整支持，转换为 lark_md 格式
3. **代码块**：支持语法高亮
4. **链接**：自动识别并渲染

### 用户 ID 类型

| 类型 | 格式 | 说明 |
|------|------|------|
| user_id | `ou_xxx` | 用户唯一 ID（推荐） |
| open_id | `ou_xxx` | 应用维度 ID |
| union_id | `on_xxx` | 企业维度 ID |

---

## 🎯 最佳实践

### 1. 安全配置

```toml
# ✅ 推荐
[channels_config.lark]
encrypt_key = "64-char-hex-string"
verification_token = "random-token"
allowed_users = ["ou_xxx", "ou_yyy"]

# ❌ 不推荐
[channels_config.lark]
encrypt_key = ""
verification_token = ""
allowed_users = ["*"]
```

### 2. 错误处理

```bash
# 使用环境变量设置日志级别
export RUST_LOG=info
./target/release/zeroclaw channel start

# 调试时使用 debug
export RUST_LOG=debug
```

### 3. 监控

```bash
# 定期检查健康状态
watch -n 60 './target/release/zeroclaw channel doctor'

# 使用 systemd 等工具确保服务持续运行
```

### 4. 备份

```bash
# 备份配置文件
cp ~/.zeroclaw/config.toml ~/.zeroclaw/config.toml.backup

# 记录你的 user_id
echo "My user_id: ou_xxx" >> ~/.zeroclaw/lark-user-info.txt
```

---

## 📞 获取帮助

### 查看日志

```bash
# 启用详细日志
RUST_LOG=debug ./target/release/zeroclaw channel start

# 保存日志到文件
RUST_LOG=debug ./target/release/zeroclaw channel start 2>&1 | tee zeroclaw.log
```

### 检查版本

```bash
./target/release/zeroclaw --version
```

### 查看帮助

```bash
./target/release/zeroclaw channel --help
./target/release/zeroclaw gateway --help
```

---

## 🔗 相关链接

- **Lark 开放平台**: https://open.larksuite.com
- **Feishu 开放平台**: https://open.feishu.cn
- **ZeroClaw 文档**: https://github.com/your-repo/zeroclaw
- **问题反馈**: https://github.com/your-repo/zeroclaw/issues

---

## ✅ 测试清单

在部署到生产环境前，完成以下测试：

- [ ] 配置文件正确加载（`channel list` 显示 Lark/Feishu）
- [ ] 健康检查通过（`channel doctor` 显示 OK）
- [ ] Webhook 验证成功（Lark 控制台显示 ✅）
- [ ] 能接收消息（发送 "hello" 收到回复）
- [ ] 能发送消息（机器人正确回复）
- [ ] 用户授权工作（未授权用户被拒绝）
- [ ] 日志正常（无错误或警告）
- [ ] Token 自动刷新（运行 2 小时后仍然工作）

---

**祝你使用愉快！🎉**

如有问题，请查看日志或提交 Issue。
