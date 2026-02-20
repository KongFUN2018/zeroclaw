# 飞书/Feishu 长连接模式使用指南

飞书长连接模式是飞书开放平台推荐的实时事件订阅方式，无需公网 IP 或域名即可接收事件。

## 优势

- ✅ **无需公网 IP** - 本地开发环境即可接收事件
- ✅ **降低接入成本** - 开发周期从 1 周降到 5 分钟
- ✅ **无需内网穿透** - 测试阶段更便捷
- ✅ **简化安全处理** - 只在建连时鉴权，后续事件无需解密
- ✅ **实时性强** - 消息延迟从分钟级降至毫秒级

## 前置条件

⚠️ **重要**: 长连接模式目前主要支持**国内版飞书开放平台**，国际版可能不支持。

## 配置步骤

### 1. 飞书开放平台配置

1. 登录飞书开放平台: https://open.feishu.cn
2. 创建应用或选择现有应用
3. 进入「事件与回调」→「事件配置」
4. **开启长连接模式** ⚠️
5. **发布应用版本** ⚠️ (必须，否则无法使用长连接)
6. 订阅所需事件（至少订阅 `im.message.receive_v1`）

### 2. 获取凭证

从飞书开放平台获取：
- `App ID`: 应用凭证
- `App Secret`: 应用密钥

### 3. 配置 ZeroClaw

编辑 `~/.zeroclaw/config.toml`:

```toml
[channels_config.lark]
app_id = "cli_xxxxxxxxxxxxx"              # 你的 App ID
app_secret = "xxxxxxxxxxxxxxxxxxxx"        # 你的 App Secret
encrypt_key = ""                           # 可选：加密密钥
verification_token = ""                    # 可选：验证令牌
allowed_users = ["ou_xxx", "on_yyy"]      # 允许的用户 ID
use_feishu = true                          # 使用国内版飞书
connection_mode = "long_connection"       # 启用长连接模式
```

### 4. 启动 ZeroClaw

```bash
zeroclaw daemon
```

## 验证连接

使用调试模式查看详细日志：

```bash
RUST_LOG=debug zeroclaw daemon
```

成功的日志应该显示：

```
INFO Attempting to establish WebSocket connection...
INFO Got WebSocket endpoint: wss://ws-open.feishu.cn/...
INFO WebSocket connected (device_id: xxx, service_id: xxx)
INFO Waiting for messages from WebSocket...
```

## 常见问题

### 连接失败

如果看到 `WebSocket connection failed` 错误，检查：

1. ✅ 飞书后台是否已开启长连接模式
2. ✅ 应用是否已发布版本
3. ✅ app_id 和 app_secret 是否正确
4. ✅ 网络连接是否正常
5. ✅ 使用的是国内版飞书 (`use_feishu = true`)

### 没有收到消息

1. 确认已订阅 `im.message.receive_v1` 事件
2. 检查 `allowed_users` 配置是否包含你的用户 ID
3. 向机器人发送测试消息
4. 查看日志是否有事件接收记录

### 如何获取用户 ID

1. 启动 ZeroClaw daemon
2. 向机器人发送一条消息
3. 查看日志，会显示类似：
   ```
   WARN Lark: ignoring message from unauthorized user: ou_xxxxxxxxx
   ```
4. 将 `ou_xxxxxxxxx` 添加到 `allowed_users` 配置中

## 模式对比

| 特性 | Webhook 模式 | Polling 模式 | 长连接模式 |
|------|-------------|-------------|-----------|
| 需要公网 IP | ✅ 是 | ❌ 否 | ❌ 否 |
| 实时性 | 高 | 低 | 高 |
| 资源消耗 | 低 | 高 | 中 |
| 配置复杂度 | 高 | 低 | 低 |
| 推荐程度 | - | - | ⭐ 推荐 |

## 技术细节

长连接模式实现基于：
- **WebSocket 协议**: 建立持久双向连接
- **JSON 格式**: 事件以 JSON 格式传输
- **自动重连**: 连接断开时自动重连（可配置）
- **心跳机制**: 定期发送 ping/pong 保持连接

## 相关文档

- [飞书开放平台 - 事件订阅](https://open.feishu.cn/document/server-docs/event-subscription-guide/overview)
- [配置示例](docs/config.lark.example)
- [代码实现](src/lark_ws/)
