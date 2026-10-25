//! Signal 消息通道模块
//!
//! 本模块实现了基于 [signal-cli](https://github.com/AsamK/signal-cli) 守护进程的 Signal 消息通道。
//! 通过 signal-cli 原生的 JSON-RPC 和 SSE（Server-Sent Events）API 与 Signal 网络通信。
//!
//! # 架构概述
//!
//! - **监听**：通过 SSE 连接到 `/api/v1/events` 端点接收实时消息
//! - **发送**：通过 JSON-RPC 调用 `/api/v1/rpc` 端点发送消息
//! - **健康检查**：通过 `/api/v1/check` 端点检查服务状态
//!
//! # 前置条件
//!
//! 需要先启动 signal-cli 守护进程：
//! ```bash
//! signal-cli daemon --http 127.0.0.1:8080
//! ```
//!
//! # 支持的消息类型
//!
//! - 私聊消息（DM）
//! - 群组消息
//! - 可选：忽略附件消息、忽略 Story 消息

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

/// 群组目标前缀标识符
///
/// 当消息接收者以此前缀开头时，表示目标为群组而非个人用户。
/// 格式：`group:<群组ID>`
const GROUP_TARGET_PREFIX: &str = "group:";

/// 消息接收者目标类型
///
/// 区分消息是发送给个人用户还是群组。
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecipientTarget {
    /// 直接发送给个人用户（电话号码或 UUID）
    Direct(String),
    /// 发送给群组（群组 ID）
    Group(String),
}

/// Signal 消息通道
///
/// 通过 signal-cli 守护进程的 HTTP API 实现 Signal 消息的收发功能。
/// 连接到运行中的 `signal-cli daemon --http <host:port>`。
///
/// # 配置说明
///
/// - `http_url`：signal-cli HTTP 服务地址
/// - `account`：Signal 账号（E.164 格式电话号码）
/// - `group_id`：可选的群组 ID 过滤，设为 `Some("dm")` 仅接收私聊
/// - `allowed_from`：允许发送消息的号码列表，`["*"]` 表示允许所有
/// - `ignore_attachments`：是否忽略仅包含附件的消息
/// - `ignore_stories`：是否忽略 Story 消息
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::channels::signal::SignalChannel;
///
/// let channel = SignalChannel::new(
///     "http://127.0.0.1:8080".to_string(),
///     "+1234567890".to_string(),
///     None,
///     vec!["*".to_string()],
///     false,
///     true,
/// );
/// ```
#[derive(Clone)]
pub struct SignalChannel {
    /// signal-cli HTTP 服务的基础 URL
    http_url: String,
    /// Signal 账号（E.164 格式电话号码）
    account: String,
    /// 可选的群组 ID 过滤，`Some("dm")` 表示仅接收私聊
    group_id: Option<String>,
    /// 允许发送消息的号码列表，`["*"]` 表示允许所有发送者
    allowed_from: Vec<String>,
    /// 是否忽略仅包含附件的消息
    ignore_attachments: bool,
    /// 是否忽略 Story 消息
    ignore_stories: bool,
}

// ── signal-cli SSE 事件 JSON 结构体 ────────────────────────────

/// SSE 信封包装器
///
/// signal-cli 通过 SSE 发送的消息外层结构。
#[derive(Debug, Deserialize)]
struct SseEnvelope {
    /// 消息信封，包含实际的消内容
    #[serde(default)]
    envelope: Option<Envelope>,
}

/// Signal 消息信封
///
/// 包含消息的元数据和内容。
#[derive(Debug, Deserialize)]
struct Envelope {
    /// 发送者显示名称（可能为空）
    #[serde(default)]
    source: Option<String>,
    /// 发送者电话号码（E.164 格式，优先使用）
    #[serde(rename = "sourceNumber", default)]
    source_number: Option<String>,
    /// 数据消息（普通文本消息）
    #[serde(rename = "dataMessage", default)]
    data_message: Option<DataMessage>,
    /// Story 消息（当 ignore_stories 为 true 时会被忽略）
    #[serde(rename = "storyMessage", default)]
    story_message: Option<serde_json::Value>,
    /// 消息时间戳（毫秒级 Unix 时间戳）
    #[serde(default)]
    timestamp: Option<u64>,
}

/// Signal 数据消息
///
/// 包含实际的消息文本和相关信息。
#[derive(Debug, Deserialize)]
struct DataMessage {
    /// 消息文本内容
    #[serde(default)]
    message: Option<String>,
    /// 消息时间戳（毫秒级 Unix 时间戳）
    #[serde(default)]
    timestamp: Option<u64>,
    /// 群组信息（如果是群组消息）
    #[serde(rename = "groupInfo", default)]
    group_info: Option<GroupInfo>,
    /// 附件列表
    #[serde(default)]
    attachments: Option<Vec<serde_json::Value>>,
}

/// Signal 群组信息
#[derive(Debug, Deserialize)]
struct GroupInfo {
    /// 群组 ID
    #[serde(rename = "groupId", default)]
    group_id: Option<String>,
}

impl SignalChannel {
    /// 创建新的 Signal 通道实例
    ///
    /// # 参数
    ///
    /// - `http_url`：signal-cli HTTP 服务地址（如 `http://127.0.0.1:8080`）
    /// - `account`：Signal 账号（E.164 格式，如 `+1234567890`）
    /// - `group_id`：可选的群组 ID 过滤
    ///   - `None`：接收所有消息
    ///   - `Some("dm")`：仅接收私聊消息
    ///   - `Some(group_id)`：仅接收指定群组的消息
    /// - `allowed_from`：允许发送消息的号码列表
    ///   - `["*"]`：允许所有发送者
    ///   - `["+1111111111", "+2222222222"]`：仅允许指定号码
    /// - `ignore_attachments`：是否忽略仅包含附件的消息
    /// - `ignore_stories`：是否忽略 Story 消息
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `SignalChannel` 实例
    pub fn new(
        http_url: String,
        account: String,
        group_id: Option<String>,
        allowed_from: Vec<String>,
        ignore_attachments: bool,
        ignore_stories: bool,
    ) -> Self {
        // 移除 URL 末尾的斜杠以保持一致性
        let http_url = http_url.trim_end_matches('/').to_string();
        Self { http_url, account, group_id, allowed_from, ignore_attachments, ignore_stories }
    }

    /// 创建并配置 HTTP 客户端
    ///
    /// 配置连接超时和应用代理设置。
    fn http_client(&self) -> Client {
        let builder = Client::builder();
        // 非 WASM 目标设置连接超时
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.connect_timeout(Duration::from_secs(10));

        // 应用运行时代理配置
        let builder =
            crate::app::agent::config::apply_runtime_proxy_to_builder(builder, "channel.signal");
        builder.build().expect("Signal HTTP client should build")
    }

    /// 从信封中提取有效发送者
    ///
    /// 优先使用 `sourceNumber`（E.164 格式），若不存在则回退到 `source`。
    ///
    /// # 参数
    ///
    /// - `envelope`：消息信封引用
    ///
    /// # 返回值
    ///
    /// 返回发送者标识符（电话号码或显示名称），若两者都不存在则返回 `None`
    fn sender(envelope: &Envelope) -> Option<String> {
        envelope.source_number.as_deref().or(envelope.source.as_deref()).map(String::from)
    }

    /// 检查发送者是否在允许列表中
    ///
    /// # 参数
    ///
    /// - `sender`：发送者标识符
    ///
    /// # 返回值
    ///
    /// 如果 `allowed_from` 包含 `"*"` 或包含该发送者，返回 `true`
    fn is_sender_allowed(&self, sender: &str) -> bool {
        // 通配符 "*" 表示允许所有发送者
        if self.allowed_from.iter().any(|u| u == "*") {
            return true;
        }
        // 检查发送者是否在允许列表中
        self.allowed_from.iter().any(|u| u == sender)
    }

    /// 检查字符串是否为有效的 E.164 电话号码格式
    ///
    /// E.164 格式：以 `+` 开头，后跟 2-15 位数字。
    ///
    /// # 参数
    ///
    /// - `recipient`：待检查的字符串
    ///
    /// # 返回值
    ///
    /// 如果符合 E.164 格式返回 `true`
    fn is_e164(recipient: &str) -> bool {
        let Some(number) = recipient.strip_prefix('+') else {
            return false;
        };
        // E.164 号码长度为 2-15 位数字
        (2..=15).contains(&number.len()) && number.chars().all(|c| c.is_ascii_digit())
    }

    /// 检查字符串是否为有效的 UUID
    ///
    /// signal-cli 使用 UUID 标识选择退出共享电话号码的隐私保护用户。
    ///
    /// # 参数
    ///
    /// - `s`：待检查的字符串
    ///
    /// # 返回值
    ///
    /// 如果是有效的 UUID 格式返回 `true`
    fn is_uuid(s: &str) -> bool {
        Uuid::parse_str(s).is_ok()
    }

    /// 解析接收者目标类型
    ///
    /// 根据接收者字符串格式判断是个人用户还是群组。
    ///
    /// # 参数
    ///
    /// - `recipient`：接收者标识符
    ///
    /// # 返回值
    ///
    /// - `RecipientTarget::Direct`：个人用户（E.164 号码或 UUID）
    /// - `RecipientTarget::Group`：群组（以 `group:` 前缀开头或非个人格式）
    fn parse_recipient_target(recipient: &str) -> RecipientTarget {
        // 检查是否为群组前缀格式
        if let Some(group_id) = recipient.strip_prefix(GROUP_TARGET_PREFIX) {
            return RecipientTarget::Group(group_id.to_string());
        }

        // 检查是否为个人用户格式（E.164 或 UUID）
        if Self::is_e164(recipient) || Self::is_uuid(recipient) {
            RecipientTarget::Direct(recipient.to_string())
        } else {
            // 其他格式视为群组 ID
            RecipientTarget::Group(recipient.to_string())
        }
    }

    /// 检查消息是否匹配配置的群组过滤条件
    ///
    /// 如果未配置 `group_id`（None），则接受所有私聊和群组消息。
    /// 使用 `"dm"` 作为 group_id 可以仅过滤私聊消息。
    ///
    /// # 参数
    ///
    /// - `data_msg`：数据消息引用
    ///
    /// # 返回值
    ///
    /// 如果消息匹配配置的群组条件返回 `true`
    fn matches_group(&self, data_msg: &DataMessage) -> bool {
        // 未配置 group_id 时接受所有消息
        let Some(ref expected) = self.group_id else {
            return true;
        };
        // 检查消息的群组 ID 是否匹配
        match data_msg.group_info.as_ref().and_then(|g| g.group_id.as_deref()) {
            Some(gid) => gid == expected.as_str(),
            // 私聊消息（无群组信息）：仅当配置为 "dm" 时匹配
            None => expected.eq_ignore_ascii_case("dm"),
        }
    }

    /// 确定回复目标
    ///
    /// 根据消息来源返回正确的回复目标：
    /// - 群组消息：回复到该群组
    /// - 私聊消息：回复到发送者
    ///
    /// # 参数
    ///
    /// - `data_msg`：数据消息引用
    /// - `sender`：发送者标识符
    ///
    /// # 返回值
    ///
    /// 返回格式化的回复目标字符串（群组带 `group:` 前缀）
    fn reply_target(&self, data_msg: &DataMessage, sender: &str) -> String {
        // 如果消息来自群组，回复到该群组
        if let Some(group_id) = data_msg.group_info.as_ref().and_then(|g| g.group_id.as_deref()) {
            format!("{GROUP_TARGET_PREFIX}{group_id}")
        } else {
            // 私聊消息：回复到发送者
            sender.to_string()
        }
    }

    /// 发送 JSON-RPC 请求到 signal-cli 守护进程
    ///
    /// # 参数
    ///
    /// - `method`：RPC 方法名（如 `send`、`sendTyping`）
    /// - `params`：方法参数（JSON 对象）
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(result))`：请求成功并返回结果
    /// - `Ok(None)`：请求成功但无返回内容（如输入指示器）
    /// - `Err(e)`：请求失败
    ///
    /// # 错误
    ///
    /// - HTTP 请求失败
    /// - JSON 解析失败
    /// - RPC 错误响应
    async fn rpc_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let url = format!("{}/api/v1/rpc", self.http_url);
        // 生成唯一请求 ID
        let id = Uuid::new_v4().to_string();

        // 构建 JSON-RPC 2.0 请求体
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        });

        let mut builder = self.http_client().post(&url);

        // 非 WASM 目标设置请求超时
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(Duration::from_secs(30));
        }

        // 发送请求
        let resp = builder.header("Content-Type", "application/json").json(&body).send().await?;

        // 201 状态码表示成功但无响应体（如输入指示器）
        if resp.status().as_u16() == 201 {
            return Ok(None);
        }

        let text = resp.text().await?;
        // 空响应也视为成功
        if text.is_empty() {
            return Ok(None);
        }

        // 解析响应
        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        // 检查是否有错误响应
        if let Some(err) = parsed.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
            anyhow::bail!("Signal RPC error {code}: {msg}");
        }

        // 返回结果字段
        Ok(parsed.get("result").cloned())
    }

    /// 处理单个 SSE 信封，转换为 ChannelMessage
    ///
    /// 执行以下过滤和处理：
    /// 1. 忽略 Story 消息（如果配置）
    /// 2. 忽略仅附件消息（如果配置）
    /// 3. 验证发送者权限
    /// 4. 验证群组匹配
    /// 5. 提取消息内容和元数据
    ///
    /// # 参数
    ///
    /// - `envelope`：SSE 信封引用
    ///
    /// # 返回值
    ///
    /// 如果消息通过所有过滤条件，返回 `Some(ChannelMessage)`；否则返回 `None`
    fn process_envelope(&self, envelope: &Envelope) -> Option<ChannelMessage> {
        // 跳过 Story 消息（如果配置了忽略）
        if self.ignore_stories && envelope.story_message.is_some() {
            return None;
        }

        // 获取数据消息
        let data_msg = envelope.data_message.as_ref()?;

        // 跳过仅附件消息（如果配置了忽略）
        if self.ignore_attachments {
            let has_attachments = data_msg.attachments.as_ref().is_some_and(|a| !a.is_empty());
            if has_attachments && data_msg.message.is_none() {
                return None;
            }
        }

        // 获取消息文本（非空）
        let text = data_msg.message.as_deref().filter(|t| !t.is_empty())?;
        // 获取发送者
        let sender = Self::sender(envelope)?;

        // 验证发送者权限
        if !self.is_sender_allowed(&sender) {
            return None;
        }

        // 验证群组匹配
        if !self.matches_group(data_msg) {
            return None;
        }

        // 确定回复目标
        let target = self.reply_target(data_msg, &sender);

        // 确定时间戳：优先使用消息时间戳，其次信封时间戳，最后使用当前时间
        let timestamp = data_msg.timestamp.or(envelope.timestamp).unwrap_or_else(|| {
            u64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
            )
            .unwrap_or(u64::MAX)
        });

        // 构建 ChannelMessage
        Some(ChannelMessage {
            instance_id: "default".to_string(),
            id: format!("sig_{timestamp}"),
            sender: sender.clone(),
            reply_target: target,
            content: text.to_string(),
            channel: "signal".to_string(),
            timestamp: timestamp / 1000, // 毫秒 → 秒
            thread_ts: None,
        })
    }
}

/// Channel trait 实现
///
/// 为 SignalChannel 实现 Channel trait，提供消息收发和状态管理能力。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for SignalChannel {
    /// 返回通道名称
    fn name(&self) -> &str {
        "signal"
    }

    /// 发送消息
    ///
    /// 通过 JSON-RPC 的 `send` 方法发送消息到指定接收者。
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息，包含接收者和内容
    ///
    /// # 返回值
    ///
    /// 发送成功返回 `Ok(())`，失败返回错误
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 根据接收者类型构建不同的参数
        let params = match Self::parse_recipient_target(&message.recipient) {
            // 直接发送给个人用户
            RecipientTarget::Direct(number) => serde_json::json!({
                "recipient": [number],
                "message": &message.content,
                "account": &self.account,
            }),
            // 发送给群组
            RecipientTarget::Group(group_id) => serde_json::json!({
                "groupId": group_id,
                "message": &message.content,
                "account": &self.account,
            }),
        };

        // 调用 RPC 发送
        self.rpc_request("send", params).await?;
        Ok(())
    }

    /// 监听消息
    ///
    /// 通过 SSE 连接到 signal-cli 的事件流，接收实时消息。
    /// 包含自动重连机制和指数退避策略。
    ///
    /// # 参数
    ///
    /// - `tx`：消息发送通道，接收到的消息将通过此通道发送
    ///
    /// # 返回值
    ///
    /// 当通道关闭或发生不可恢复错误时返回
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // 构建 SSE 事件流 URL
        let mut url = reqwest::Url::parse(&format!("{}/api/v1/events", self.http_url))?;
        url.query_pairs_mut().append_pair("account", &self.account);

        tracing::info!("Signal channel listening via SSE on {}...", self.http_url);

        // 指数退避重连参数
        let mut retry_delay_secs = 2u64;
        let max_delay_secs = 60u64;

        loop {
            // 尝试建立 SSE 连接
            let resp = self
                .http_client()
                .get(url.clone())
                .header("Accept", "text/event-stream")
                .send()
                .await;

            // 处理连接结果
            let resp = match resp {
                Ok(r) if r.status().is_success() => r,
                // HTTP 错误响应
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    // 清理错误信息中的敏感内容
                    let sanitized = crate::app::agent::providers::sanitize_api_error(&body);
                    tracing::warn!("Signal SSE returned {status}: {sanitized}");
                    // 指数退避等待后重试
                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs)).await;
                    retry_delay_secs = (retry_delay_secs * 2).min(max_delay_secs);
                    continue;
                }
                // 连接错误
                Err(e) => {
                    tracing::warn!("Signal SSE connect error: {e}, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs)).await;
                    retry_delay_secs = (retry_delay_secs * 2).min(max_delay_secs);
                    continue;
                }
            };

            // 连接成功，重置退避延迟
            retry_delay_secs = 2;

            // 获取字节流
            let mut bytes_stream = resp.bytes_stream();
            let mut buffer = String::new();
            let mut current_data = String::new();

            // 处理 SSE 流
            while let Some(chunk) = bytes_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!("Signal SSE chunk error, reconnecting: {e}");
                        break;
                    }
                };

                // 将字节转换为 UTF-8 字符串
                let text = match String::from_utf8(chunk.to_vec()) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::debug!("Signal SSE invalid UTF-8, skipping chunk: {}", e);
                        continue;
                    }
                };

                buffer.push_str(&text);

                // 逐行处理 SSE 数据
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    // 跳过 SSE 注释行（用于 keepalive）
                    if line.starts_with(':') {
                        continue;
                    }

                    if line.is_empty() {
                        // 空行表示事件边界，处理累积的数据
                        if !current_data.is_empty() {
                            match serde_json::from_str::<SseEnvelope>(&current_data) {
                                Ok(sse) => {
                                    if let Some(ref envelope) = sse.envelope {
                                        if let Some(msg) = self.process_envelope(envelope) {
                                            // 发送消息到通道，如果接收端已关闭则退出
                                            if tx.send(msg).await.is_err() {
                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!("Signal SSE parse skip: {e}");
                                }
                            }
                            current_data.clear();
                        }
                    } else if let Some(data) = line.strip_prefix("data:") {
                        // 累积 data 行内容
                        if !current_data.is_empty() {
                            current_data.push('\n');
                        }
                        current_data.push_str(data.trim_start());
                    }
                    // 忽略 "event:", "id:", "retry:" 等其他 SSE 字段
                }
            }

            // 处理流结束前剩余的数据
            if !current_data.is_empty() {
                match serde_json::from_str::<SseEnvelope>(&current_data) {
                    Ok(sse) => {
                        if let Some(ref envelope) = sse.envelope {
                            if let Some(msg) = self.process_envelope(envelope) {
                                let _ = tx.send(msg).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Signal SSE trailing parse skip: {e}");
                    }
                }
            }

            // 流结束，准备重连
            tracing::debug!("Signal SSE stream ended, reconnecting...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    /// 健康检查
    ///
    /// 检查 signal-cli 守护进程是否正常运行。
    ///
    /// # 返回值
    ///
    /// 如果服务正常返回 `true`，否则返回 `false`
    async fn health_check(&self) -> bool {
        let url = format!("{}/api/v1/check", self.http_url);
        let mut builder = self.http_client().get(&url);

        // 非 WASM 目标设置超时
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(Duration::from_secs(10));
        }

        let Ok(resp) = builder.send().await else {
            return false;
        };
        resp.status().is_success()
    }

    /// 发送输入指示器
    ///
    /// 向接收者显示"正在输入..."状态。
    ///
    /// # 参数
    ///
    /// - `recipient`：接收者标识符（个人号码或群组 ID）
    ///
    /// # 返回值
    ///
    /// 发送成功返回 `Ok(())`，失败返回错误
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        // 根据接收者类型构建参数
        let params = match Self::parse_recipient_target(recipient) {
            RecipientTarget::Direct(number) => serde_json::json!({
                "recipient": [number],
                "account": &self.account,
            }),
            RecipientTarget::Group(group_id) => serde_json::json!({
                "groupId": group_id,
                "account": &self.account,
            }),
        };
        self.rpc_request("sendTyping", params).await?;
        Ok(())
    }

    /// 停止输入指示器
    ///
    /// signal-cli 没有显式的停止输入 RPC，
    /// 输入指示器会在约 15 秒后自动过期。
    ///
    /// # 参数
    ///
    /// - `_recipient`：接收者标识符（未使用）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(())`
    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        // signal-cli 没有 stop-typing RPC，输入指示器在客户端约 15 秒后自动过期
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
