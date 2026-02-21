# Lark/Feishu 消息重复问题修复说明

## 问题描述

### 1. 消息重复回复
机器人会重复发送相同的回复内容，例如：
```
我看到了之前的讨论。EvoMap 听起来确实很有意思...
我看到了之前的讨论。EvoMap 听起来确实很有意思...
```

### 2. 延迟回复
在某些情况下，机器人的回复会有 2-3 小时的延迟。

## 根本原因分析

### 消息重复回复的原因

1. **缺少消息去重机制**
   - Lark channel 原本没有实现消息去重（不像 EmailChannel 有 `seen_messages` HashSet）
   - WebSocket 重连、网络波动或服务端重复推送事件时，同一条 `message_id` 可能被多次处理

2. **配置混乱**
   - 用户配置中同时设置了 `poll_mode = true` 和 `connection_mode = "long_connection"`
   - 虽然代码只使用 `connection_mode`，但这种混乱可能导致理解上的偏差

### 延迟回复的原因

1. **长连接模式的重连机制**
   - WebSocket 断线后会等待 120 秒再重连
   - 如果网络不稳定或服务端配置问题，可能导致消息长时间延迟

2. **可能的服务端配置问题**
   - 长连接模式需要应用已发布
   - 需要在飞书开放平台正确启用长连接功能

## 修复方案

### 代码修复（已实现）

1. **添加消息去重机制**
   - 在 `LarkChannel` 结构体中添加 `seen_messages: Arc<Mutex<HashSet<String>>>`
   - 实现了 `is_message_processed()` 方法来检查和记录已处理的消息
   - 在 `parse_webhook_payload()` 中添加去重检查
   - 在长连接模式的事件处理器中也添加去重检查

2. **自动清理机制**
   - 当已处理消息数超过 1000 条时，自动清理最旧的 100 条
   - 防止内存无限增长

### 配置建议

#### 当前正确配置示例
```toml
[channels_config.lark]
app_id = "cli_a91f80f12ae1dcee"
app_secret = "ayIMGpepynJ7AxIsQRJrdcxY1EuAY1G6"
allowed_users = ["*"]
use_feishu = true

# 使用长连接模式（推荐，实时性好）
connection_mode = "long_connection"

# 轮询间隔（仅在 polling 模式下使用）
poll_interval_secs = 5

# 加密和验证（可选，webhook 模式推荐）
encrypt_key = "cT91dY7QKWrkQ0W6G8btVgsaUrszRf5p"
verification_token = "G6VH3VqLRURXUoro0lX15bvvHMOOVCAe"
```

#### 移除的配置项
- ❌ `poll_mode = true` - 此配置已废弃，请使用 `connection_mode`
- ✅ `connection_mode = "long_connection"` - 新的统一配置方式

#### 三种连接模式对比

| 模式 | 配置值 | 优点 | 缺点 | 适用场景 |
|------|--------|------|------|----------|
| **Webhook** | `"webhook"` | 实时性最好，无需轮询 | 需要公网 URL | 有稳定公网地址 |
| **轮询** | `"polling"` | 无需公网 URL，配置简单 | 实时性差，有延迟 | 测试环境，无公网 |
| **长连接** | `"long_connection"` | 实时性好，无需公网 URL | 需要应用已发布 | 生产环境推荐 |

## 如何选择连接模式

### 1. 长连接模式（推荐）
```toml
connection_mode = "long_connection"
```
- ✅ 实时性最好，无需轮询
- ✅ 无需公网 URL
- ⚠️ 需要应用已发布（在飞书开放平台）
- ⚠️ 需要启用长连接事件订阅

**前提条件**：
1. 在飞书开放平台发布应用
2. 在事件订阅中启用"长连接"功能
3. 配置需要接收的事件类型（如 `im.message.receive_v1`）

### 2. 轮询模式（备选）
```toml
connection_mode = "polling"
poll_interval_secs = 5  # 5秒轮询一次
```
- ✅ 无需公网 URL
- ✅ 无需发布应用
- ❌ 有轮询延迟（至少 5 秒）
- ❌ 频繁请求消耗配额

**适用场景**：
- 开发测试环境
- 无法发布应用的场景
- 对实时性要求不高

### 3. Webhook 模式（传统）
```toml
connection_mode = "webhook"
```
- ✅ 实时性最好
- ❌ 需要公网 URL（需要 cloudflared 等隧道工具）
- ❌ 配置相对复杂

**适用场景**：
- 有稳定公网地址
- 已有 HTTP 服务器（如使用 `zeroclaw gateway`）

## 延迟问题的排查步骤

如果仍然遇到延迟回复问题，请按以下步骤排查：

1. **检查日志**
   ```bash
   # 查看是否有 WebSocket 连接错误
   grep -i "websocket\|reconnect\|error" ~/.zeroclaw/zeroclaw.log
   ```

2. **验证配置**
   ```bash
   # 检查连接模式配置
   grep -A 10 '\[channels_config.lark\]' ~/.zeroclaw/config.toml
   ```

3. **测试连接**
   ```bash
   # 运行健康检查
   zeroclaw channel doctor
   ```

4. **切换模式测试**
   - 如果长连接有问题，尝试切换到轮询模式：
     ```toml
     connection_mode = "polling"
     poll_interval_secs = 10
     ```

5. **检查飞书开放平台配置**
   - 确认应用已发布
   - 确认长连接功能已启用
   - 确认事件订阅已配置

## 技术实现细节

### 消息去重实现

```rust
// 检查消息是否已处理
async fn is_message_processed(&self, message_id: &str) -> bool {
    let mut seen = self.seen_messages.lock().await;
    if seen.contains(message_id) {
        tracing::debug!("Lark: skipping duplicate message {}", message_id);
        true
    } else {
        seen.insert(message_id.to_string());

        // 自动清理：保留最近 1000 条
        if seen.len() > 1000 {
            let old_ids: Vec<_> = seen.iter().take(100).cloned().collect();
            for id in old_ids {
                seen.remove(&id);
            }
        }

        false
    }
}
```

### 长连接模式去重

在 WebSocket 事件处理器中使用阻塞锁（因为是同步回调）：

```rust
// 检查重复消息（在同步上下文中）
{
    let mut seen = self.seen_messages.blocking_lock();
    if seen.contains(&message_id) {
        tracing::info!("Lark WebSocket: skipping duplicate message_id {}", message_id);
        return Ok(());
    }
    seen.insert(message_id.clone());
    // ... 清理逻辑
}
```

## 验证修复

### 测试步骤

1. **构建并运行**
   ```bash
   cargo build --release
   ./target/release/zeroclaw channel start
   ```

2. **发送测试消息**
   - 向机器人发送相同的消息多次
   - 验证机器人只回复一次

3. **检查日志**
   ```bash
   # 应该能看到 "skipping duplicate message" 的日志
   tail -f ~/.zeroclaw/zeroclaw.log | grep "duplicate"
   ```

### 预期结果

- ✅ 每条消息只回复一次
- ✅ 日志中显示重复消息被跳过
- ✅ 内存使用稳定（seen_messages 不会无限增长）

## 相关文件

- [src/channels/lark.rs](src/channels/lark.rs) - Lark channel 实现
- [src/config/schema.rs](src/config/schema.rs) - 配置结构定义
- [src/lark_ws/client.rs](src/lark_ws/client.rs) - WebSocket 客户端实现
- [src/main.rs](src/main.rs) - 状态命令显示

## 更新日志

- 2025-02-21: 添加消息去重机制，修复重复回复问题
- 2025-02-21: 文档化配置建议，移除废弃的 `poll_mode` 配置
- 2025-02-21: 修复 `zeroclaw status` 命令不显示 Lark/Feishu 配置状态的问题
