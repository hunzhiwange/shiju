//! Discord 通道模块
//!
//! 本模块实现了 Discord 平台的通道集成，通过 Discord Gateway WebSocket 协议实现实时消息收发。
//! 支持文本消息、附件处理、表情反应以及"正在输入"状态等功能。
//!
//! # 主要功能
//!
//! - **实时消息监听**：通过 Gateway WebSocket 连接接收消息事件
//! - **消息发送**：支持发送文本消息和文件附件（最多 10 个文件）
//! - **表情反应**：支持为消息添加/移除表情反应
//! - **输入状态**：支持显示"正在输入"状态提示
//! - **权限控制**：支持用户白名单、频道过滤和提及触发
//! - **附件处理**：支持处理传入附件和传出本地文件附件
//! - **语音转写**：可选支持语音消息的转录功能
//!
//! # 架构说明
//!
//! 模块采用 submodule 化设计，各功能职责分离：
//! - `attachments`: 处理传入的附件（图片、音频等）
//! - `attachments_outgoing`: 处理传出附件的解析、分类和路径验证
//! - `client`: Discord HTTP API 客户端封装
//! - `content`: 消息内容的规范化处理
//! - `gateway`: Gateway WebSocket 连接管理
//! - `ids`: Discord ID 处理工具
//! - `message_split`: 消息分片（应对 Discord 长度限制）
//! - `permissions`: 权限和触发条件检查
//! - `reactions`: 表情反应相关功能
//! - `typing`: "正在输入"状态管理
//!
//! # 使用示例
//!
//! ```no_run
//! use vibewindow::app::agent::channels::discord::DiscordChannel;
//! use vibewindow::app::agent::channels::traits::Channel;
//!
//! // 创建 Discord 通道实例
//! let channel = DiscordChannel::new(
//!     "bot_token".to_string(),
//!     Some("guild_id".to_string()),
//!     vec!["user_id_1".to_string()],
//!     false,
//!     true,
//! );
//!
//! // 发送消息
//! // channel.send(&message).await?;
//! ```

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::path::PathBuf;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

mod attachments;
mod attachments_outgoing;
mod client;
mod content;
mod gateway;
mod ids;
mod message_split;
mod permissions;
mod reactions;
mod typing;

#[cfg(test)]
#[path = "attachments_outgoing_tests.rs"]
mod attachments_outgoing_tests;
#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

use attachments::process_attachments;
use attachments_outgoing::{
    classify_outgoing_attachments, parse_attachment_markers, resolve_local_attachment_path,
    with_inline_attachment_urls,
};
use client::{
    discord_messages_url, send_discord_message_json_to_url, send_discord_message_with_files_to_url,
};
use content::{normalize_group_reply_allowed_sender_ids, normalize_incoming_content};
use gateway::{connect_gateway, fetch_gateway_url, read_gateway_hello, send_identify};
use message_split::split_message_for_discord;
use permissions::{is_group_sender_trigger_enabled, is_user_allowed};
use reactions::{discord_reaction_url_with_base, random_discord_ack_reaction};
use typing::{TypingHandles, new_typing_handles, start_typing, stop_typing};

/// Discord 通道实现
///
/// 通过 Discord Gateway WebSocket 连接实现实时消息收发的通道结构体。
/// 支持文本消息、附件、表情反应和输入状态等完整的 Discord 消息交互功能。
///
/// # 字段说明
///
/// - `bot_token`: Discord Bot 令牌，用于 API 认证
/// - `guild_id`: 可选的服务器 ID，用于过滤消息来源
/// - `allowed_users`: 允许与之交互的用户 ID 白名单
/// - `listen_to_bots`: 是否监听其他 Bot 发送的消息
/// - `mention_only`: 群组消息中是否要求必须提及 Bot 才触发
/// - `group_reply_allowed_sender_ids`: 群组中无需提及即可触发的发送者 ID 列表
/// - `workspace_dir`: 工作目录，用于验证本地附件路径
/// - `typing_handles`: "正在输入"状态管理的句柄集合
pub struct DiscordChannel {
    /// Discord Bot 令牌，用于所有 API 调用的身份认证
    bot_token: String,
    /// 服务器 ID 过滤器，设置为 Some 时仅处理该服务器的消息
    guild_id: Option<String>,
    /// 允许与之交互的用户 ID 白名单，空列表表示允许所有用户
    allowed_users: Vec<String>,
    /// 是否监听其他 Bot 发送的消息，通常为 false 以避免 Bot 间无限循环
    listen_to_bots: bool,
    /// 群组消息中是否要求必须 @提及 Bot 才触发响应
    mention_only: bool,
    /// 群组中无需提及即可触发响应的特权发送者 ID 列表
    group_reply_allowed_sender_ids: Vec<String>,
    /// 工作目录路径，用于验证本地文件附件的路径安全性
    workspace_dir: Option<PathBuf>,
    /// "正在输入"状态管理所需的异步任务句柄集合
    typing_handles: TypingHandles,
    api_base_url: String,
}

impl DiscordChannel {
    /// 创建新的 Discord 通道实例
    ///
    /// # 参数
    ///
    /// - `bot_token`: Discord Bot 令牌，从 Discord 开发者门户获取
    /// - `guild_id`: 可选的服务器 ID，用于限制消息来源范围
    /// - `allowed_users`: 允许与之交互的用户 ID 列表，空列表表示无限制
    /// - `listen_to_bots`: 是否监听其他 Bot 的消息（通常应设为 false）
    /// - `mention_only`: 群组消息中是否要求必须提及 Bot 才响应
    ///
    /// # 返回值
    ///
    /// 返回配置了基本参数的 `DiscordChannel` 实例，
    /// 可通过链式调用 `with_*` 方法进一步配置
    ///
    /// # 示例
    ///
    /// ```no_run
    /// let channel = DiscordChannel::new(
    ///     "your_bot_token".to_string(),
    ///     Some("123456789".to_string()),
    ///     vec!["111222333".to_string()],
    ///     false,
    ///     true,
    /// );
    /// ```
    pub fn new(
        bot_token: String,
        guild_id: Option<String>,
        allowed_users: Vec<String>,
        listen_to_bots: bool,
        mention_only: bool,
    ) -> Self {
        Self {
            bot_token,
            guild_id,
            allowed_users,
            listen_to_bots,
            mention_only,
            group_reply_allowed_sender_ids: Vec::new(),
            workspace_dir: None,
            typing_handles: new_typing_handles(),
            api_base_url: "https://discord.com/api/v10".to_string(),
        }
    }

    /// 配置群组中无需提及即可触发的发送者 ID 列表
    ///
    /// 当 `mention_only` 为 true 时，群组消息通常需要 @提及 Bot 才会触发。
    /// 此方法可指定一组特权用户 ID，这些用户的消息无需提及即可触发响应。
    ///
    /// # 参数
    ///
    /// - `sender_ids`: 特权发送者的 Discord 用户 ID 列表
    ///
    /// # 返回值
    ///
    /// 返回配置后的 `DiscordChannel` 实例（支持链式调用）
    ///
    /// # 示例
    ///
    /// ```no_run
    /// let channel = DiscordChannel::new(token, guild, users, false, true)
    ///     .with_group_reply_allowed_senders(vec!["admin_id".to_string()]);
    /// ```
    pub fn with_group_reply_allowed_senders(mut self, sender_ids: Vec<String>) -> Self {
        self.group_reply_allowed_sender_ids = normalize_group_reply_allowed_sender_ids(sender_ids);
        self
    }

    /// 配置语音转写功能
    ///
    /// 当启用语音转写时，收到的音频附件将被自动转录为文本。
    /// 这对于处理语音消息或音频文件非常有用。
    ///
    /// # 参数
    ///
    /// - `config`: 转写配置，包含启用标志和提供商设置
    ///
    /// # 返回值
    ///
    /// 返回配置后的 `DiscordChannel` 实例（支持链式调用）
    ///
    /// # 注意
    ///
    /// 配置工作目录，用于验证本地附件路径
    ///
    /// 设置工作目录后，传出消息中的本地文件附件路径将被限制在此目录内，
    /// 防止路径遍历攻击。
    ///
    /// # 参数
    ///
    /// - `dir`: 工作目录的绝对路径
    ///
    /// # 返回值
    ///
    /// 返回配置后的 `DiscordChannel` 实例（支持链式调用）
    ///
    /// # 安全说明
    ///
    /// 强烈建议设置工作目录，以防止恶意用户通过附件路径访问系统敏感文件
    pub fn with_workspace_dir(mut self, dir: PathBuf) -> Self {
        self.workspace_dir = Some(dir);
        self
    }

    fn with_api_base(mut self, api_base_url: String) -> Self {
        self.api_base_url = api_base_url.trim_end_matches('/').to_string();
        self
    }

    fn discord_api_base_url(&self) -> &str {
        &self.api_base_url
    }

    /// 创建配置了代理设置的 HTTP 客户端
    ///
    /// 使用全局运行时代理配置创建一个新的 HTTP 客户端实例，
    /// 用于调用 Discord REST API。
    ///
    /// # 返回值
    ///
    /// 返回配置了代理设置的 `reqwest::Client` 实例
    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("channel.discord")
    }
}

// 以下是 Channel trait 的具体实现

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for DiscordChannel {
    /// 返回通道名称标识
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 "discord"，用于日志和调试标识
    fn name(&self) -> &str {
        "discord"
    }

    /// 发送消息到 Discord
    ///
    /// 支持发送纯文本消息和附带文件附件的消息。消息内容会被自动分片以符合
    /// Discord 的单条消息长度限制。附件通过特殊的标记语法识别和处理。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息，包含收件人和内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 消息发送成功
    /// - `Err(e)`: 发送失败，包含错误详情
    ///
    /// # 附件处理流程
    ///
    /// 1. 解析消息内容中的附件标记（如 `[file:path]`、`[image:url]` 等）
    /// 2. 将附件分类为本地文件和远程 URL
    /// 3. 验证本地文件路径是否在工作目录内
    /// 4. 第一条消息附带最多 10 个本地文件上传
    /// 5. 后续消息分片发送剩余文本内容
    ///
    /// # Discord 限制
    ///
    /// - 单条消息最多附带 10 个文件
    /// - 单条消息文本长度有限制（自动分片处理）
    ///
    /// # 错误处理
    ///
    /// - 本地文件路径验证失败时记录警告并将标记作为普通文本发送
    /// - 无法解析的附件标记会被保留为原样文本
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let raw_content = super::strip_tool_call_tags(&message.content);
        let (cleaned_content, parsed_attachments) = parse_attachment_markers(&raw_content);
        let (local_attachment_targets, remote_urls, mut unresolved_markers) =
            classify_outgoing_attachments(&parsed_attachments);
        let mut local_files = Vec::new();

        // 遍历所有本地附件目标，验证路径并收集有效的文件路径
        for attachment in &local_attachment_targets {
            let target = attachment.target.trim();
            match resolve_local_attachment_path(self.workspace_dir.as_ref(), target) {
                Ok(path) => local_files.push(path),
                Err(error) => {
                    tracing::warn!(
                        target,
                        error = %error,
                        "discord: local attachment rejected by workspace policy"
                    );
                    // 路径验证失败的附件标记会被保留，稍后作为普通文本发送
                    unresolved_markers.push(format!(
                        "[{}:{}]",
                        attachment.kind.marker_name(),
                        target
                    ));
                }
            }
        }

        // 如果存在无法解析的附件标记，记录警告
        if !unresolved_markers.is_empty() {
            tracing::warn!(
                unresolved = ?unresolved_markers,
                "discord: unresolved attachment markers were sent as plain text"
            );
        }

        // Discord 单条消息最多接受 10 个文件附件
        if local_files.len() > 10 {
            tracing::warn!(
                count = local_files.len(),
                "discord: truncating local attachment upload list to 10 files"
            );
            local_files.truncate(10);
        }

        // 将远程 URL 和未解析的标记内联到消息内容中
        let content =
            with_inline_attachment_urls(&cleaned_content, &remote_urls, &unresolved_markers);
        // 根据 Discord 消息长度限制分片
        let chunks = split_message_for_discord(&content);
        let client = self.http_client();
        let url = discord_messages_url(self.discord_api_base_url(), &message.recipient);

        // 逐片发送消息，第一条附带文件
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 && !local_files.is_empty() {
                // 第一条消息：附带文件上传
                send_discord_message_with_files_to_url(
                    &client,
                    &self.bot_token,
                    &url,
                    chunk,
                    &local_files,
                )
                .await?;
            } else {
                // 后续消息：仅发送文本
                send_discord_message_json_to_url(&client, &self.bot_token, &url, chunk).await?;
            }

            // 消息分片之间添加短暂延迟，避免触发 Discord 速率限制
            if i < chunks.len() - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Ok(())
    }

    /// 监听 Discord Gateway 消息事件
    ///
    /// 建立 WebSocket 连接到 Discord Gateway，持续监听并处理消息事件。
    /// 满足过滤条件的消息会被转换为 `ChannelMessage` 并通过通道发送。
    ///
    /// # 参数
    ///
    /// - `tx`: 消息发送通道，通过此通道将接收到的消息传递给消费者
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: WebSocket 连接正常关闭
    /// - `Err(e)`: 连接或处理过程中发生错误
    ///
    /// # Gateway 协议处理
    ///
    /// 1. 获取 Gateway URL 并建立 WebSocket 连接
    /// 2. 接收 Hello 事件，获取心跳间隔
    /// 3. 发送 Identify 进行身份认证
    /// 4. 启动独立的心跳定时器任务
    /// 5. 在主循环中处理：
    ///    - 心跳请求（定时触发和服务器主动请求）
    ///    - 重连请求（Op 7）
    ///    - 会话失效（Op 9）
    ///    - 消息创建事件（MESSAGE_CREATE）
    ///
    /// # 消息过滤逻辑
    ///
    /// - 跳过自己发送的消息
    /// - 跳过其他 Bot 的消息（除非 `listen_to_bots` 为 true）
    /// - 验证发送者在 `allowed_users` 白名单中
    /// - 验证消息来源服务器（如果设置了 `guild_id`）
    /// - 群组消息中验证提及条件或发送者特权
    ///
    /// # 附件处理
    ///
    /// 收到的附件（图片、音频等）会被自动处理：
    /// - 图片附件：生成文本描述
    /// - 音频附件：如果启用了转写，会进行语音转文字
    ///
    /// # ACK 反应
    ///
    /// 每条被处理的消息都会收到一个随机的表情反应作为确认标记
    #[allow(clippy::too_many_lines)]
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let bot_user_id = ids::bot_user_id_from_token(&self.bot_token).unwrap_or_default();

        // 步骤 1：获取 Gateway WebSocket URL
        let gw_url =
            fetch_gateway_url(&self.http_client(), &self.bot_token, self.discord_api_base_url())
                .await?;
        let ws_url = format!("{gw_url}/?v=10&encoding=json");
        tracing::info!("Discord: connecting to gateway...");

        // 步骤 2：建立 WebSocket 连接
        let (mut write, mut read) = connect_gateway(&ws_url).await?;

        // 步骤 3：接收 Hello 事件，获取心跳间隔配置
        let heartbeat_interval = read_gateway_hello(&mut read).await?;

        // 步骤 4：发送 Identify 进行身份认证
        send_identify(&mut write, &self.bot_token).await?;

        tracing::info!("Discord: connected and identified");

        // 跟踪最后的序列号，用于心跳和会话恢复
        // 仅在下面的 select! 循环中访问，因此使用普通的 i64 类型即可
        let mut sequence: i64 = -1;

        // 步骤 5：启动心跳定时器任务
        // 定时器发送 tick 信号，实际的心跳包在 select! 循环中组装
        let (hb_tx, mut hb_rx) = tokio::sync::mpsc::channel::<()>(1);
        let hb_interval = heartbeat_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(hb_interval));
            loop {
                interval.tick().await;
                if hb_tx.send(()).await.is_err() {
                    break;
                }
            }
        });

        let guild_filter = self.guild_id.clone();

        // 步骤 6：主事件循环
        loop {
            tokio::select! {
                // 处理定时心跳
                _ = hb_rx.recv() => {
                    let d = if sequence >= 0 { json!(sequence) } else { json!(null) };
                    let hb = json!({"op": 1, "d": d});
                    if write.send(Message::Text(hb.to_string().into())).await.is_err() {
                        break;
                    }
                }
                // 处理 WebSocket 消息
                msg = read.next() => {
                    let msg = match msg {
                        Some(Ok(Message::Text(t))) => t,
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => continue,
                    };

                    // 解析 JSON 事件
                    let event: serde_json::Value = match serde_json::from_str(msg.as_ref()) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    // 从所有事件中跟踪序列号（用于心跳和恢复）
                    if let Some(s) = event.get("s").and_then(serde_json::Value::as_i64) {
                        sequence = s;
                    }

                    let op = event.get("op").and_then(serde_json::Value::as_u64).unwrap_or(0);

                    match op {
                        // Op 1：服务器请求立即发送心跳
                        1 => {
                            let d = if sequence >= 0 { json!(sequence) } else { json!(null) };
                            let hb = json!({"op": 1, "d": d});
                            if write.send(Message::Text(hb.to_string().into())).await.is_err() {
                                break;
                            }
                            continue;
                        }
                        // Op 7：服务器请求重新连接
                        7 => {
                            tracing::warn!("Discord: received Reconnect (op 7), closing for restart");
                            break;
                        }
                        // Op 9：会话无效，需要重新识别
                        9 => {
                            tracing::warn!("Discord: received Invalid Session (op 9), closing for restart");
                            break;
                        }
                        _ => {}
                    }

                    // 仅处理 MESSAGE_CREATE 事件（opcode 0，类型 "MESSAGE_CREATE"）
                    let event_type = event.get("t").and_then(|t| t.as_str()).unwrap_or("");
                    if event_type != "MESSAGE_CREATE" {
                        continue;
                    }

                    let Some(d) = event.get("d") else {
                        continue;
                    };

                    // 过滤 1：跳过自己发送的消息
                    let author_id = d.get("author").and_then(|a| a.get("id")).and_then(|i| i.as_str()).unwrap_or("");
                    if author_id == bot_user_id {
                        continue;
                    }

                    // 过滤 2：跳过其他 Bot 的消息（除非明确允许）
                    if !self.listen_to_bots
                        && d
                            .get("author")
                            .and_then(|a| a.get("bot"))
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                    {
                        continue;
                    }

                    // 过滤 3：验证发送者权限
                    if !is_user_allowed(&self.allowed_users, author_id) {
                        tracing::warn!("Discord: ignoring message from unauthorized user: {author_id}");
                        continue;
                    }

                    // 过滤 4：验证消息来源服务器
                    if let Some(ref gid) = guild_filter {
                        let msg_guild = d.get("guild_id").and_then(serde_json::Value::as_str);
                        // 私聊消息没有 guild_id，直接放行；群组消息需要匹配过滤器
                        if let Some(g) = msg_guild {
                            if g != gid {
                                continue;
                            }
                        }
                    }

                    // 提取并规范化消息内容
                    let content = d.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let is_group_message = d.get("guild_id").is_some();
                    let allow_sender_without_mention =
                        is_group_message
                            && is_group_sender_trigger_enabled(
                                &self.group_reply_allowed_sender_ids,
                                author_id,
                            );
                    let require_mention =
                        self.mention_only && is_group_message && !allow_sender_without_mention;
                    // 规范化内容（移除提及、处理回复等）
                    let Some(clean_content) =
                        normalize_incoming_content(content, require_mention, &bot_user_id)
                    else {
                        continue;
                    };

                    // 处理附件：图片、音频等
                    let attachment_text = {
                        let atts = d
                            .get("attachments")
                            .and_then(|a| a.as_array())
                            .cloned()
                            .unwrap_or_default();
                        process_attachments(&atts, &self.http_client()).await
                    };
                    // 合并消息内容和附件文本
                    let final_content = if attachment_text.is_empty() {
                        clean_content
                    } else {
                        format!("{clean_content}\n\n[Attachments]\n{attachment_text}")
                    };

                    // 提取消息和频道 ID
                    let message_id = d.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let channel_id = d
                        .get("channel_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();

                    // 为消息添加 ACK 表情反应（异步执行，不阻塞主流程）
                    if !message_id.is_empty() && !channel_id.is_empty() {
                        let reaction_channel = DiscordChannel::new(
                            self.bot_token.clone(),
                            self.guild_id.clone(),
                            self.allowed_users.clone(),
                            self.listen_to_bots,
                            self.mention_only,
                        )
                        .with_api_base(self.api_base_url.clone());
                        let reaction_channel_id = channel_id.clone();
                        let reaction_message_id = message_id.to_string();
                        let reaction_emoji = random_discord_ack_reaction().to_string();
                        tokio::spawn(async move {
                            if let Err(err) = reaction_channel
                                .add_reaction(
                                    &reaction_channel_id,
                                    &reaction_message_id,
                                    &reaction_emoji,
                                )
                                .await
                            {
                                tracing::debug!(
                                    "Discord: failed to add ACK reaction for message {reaction_message_id}: {err}"
                                );
                            }
                        });
                    }

                    // 构建 ChannelMessage 并发送给消费者
                    let channel_msg = ChannelMessage {
                        instance_id: "default".to_string(),
                        id: if message_id.is_empty() {
                            Uuid::new_v4().to_string()
                        } else {
                            format!("discord_{message_id}")
                        },
                        sender: author_id.to_string(),
                        // 私聊回复目标是用户 ID，群组回复目标是频道 ID
                        reply_target: if channel_id.is_empty() {
                            author_id.to_string()
                        } else {
                            channel_id.clone()
                        },
                        content: final_content,
                        channel: "discord".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        thread_ts: None,
                    };

                    // 发送消息到通道，如果消费者已关闭则退出循环
                    if tx.send(channel_msg).await.is_err() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行健康检查
    ///
    /// 通过调用 Discord API 的 `/users/@me` 端点验证 Bot 令牌的有效性。
    /// 这是验证 Bot 是否正常工作的最简单方式。
    ///
    /// # 返回值
    ///
    /// - `true`: Bot 令牌有效，API 调用成功
    /// - `false`: Bot 令牌无效或网络请求失败
    ///
    /// # 示例
    ///
    /// ```no_run
    /// let is_healthy = channel.health_check().await;
    /// if !is_healthy {
    ///     eprintln!("Discord Bot 令牌无效或网络异常");
    /// }
    /// ```
    async fn health_check(&self) -> bool {
        self.http_client()
            .get(format!("{}/users/@me", self.discord_api_base_url()))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// 启动"正在输入"状态显示
    ///
    /// 调用 Discord API 在指定频道显示"正在输入..."状态。
    /// 该状态会在后台持续发送，直到调用 `stop_typing` 为止。
    ///
    /// # 参数
    ///
    /// - `recipient`: 频道 ID 或用户 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功启动输入状态显示
    /// - `Err(e)`: 启动失败
    ///
    /// # 注意
    ///
    /// - 输入状态会自动每 10 秒刷新一次
    /// - 10 秒后 Discord 会自动清除状态，需要持续刷新
    /// - 对于长回复建议在开始生成前调用，完成后调用 `stop_typing`
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        start_typing(
            &self.typing_handles,
            self.http_client(),
            self.bot_token.clone(),
            self.discord_api_base_url().to_string(),
            recipient.to_string(),
        )
        .await
    }

    /// 停止"正在输入"状态显示
    ///
    /// 取消指定频道/用户的后台输入状态刷新任务。
    ///
    /// # 参数
    ///
    /// - `recipient`: 频道 ID 或用户 ID（需与 `start_typing` 使用相同的值）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功停止输入状态显示
    /// - `Err(e)`: 停止失败（通常可以忽略）
    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()> {
        stop_typing(&self.typing_handles, recipient).await
    }

    /// 为消息添加表情反应
    ///
    /// 调用 Discord API 为指定消息添加一个表情反应。
    /// 常用于确认收到消息或表达对消息的态度。
    ///
    /// # 参数
    ///
    /// - `channel_id`: 消息所在的频道 ID
    /// - `message_id`: 要添加反应的消息 ID
    /// - `emoji`: 表情符号（可以是 Unicode 表情或自定义表情格式 `name:id`）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 反应添加成功
    /// - `Err(e)`: 添加失败，包含状态码和错误信息
    ///
    /// # 错误处理
    ///
    /// 常见失败原因：
    /// - 消息不存在
    /// - 没有在该频道添加反应的权限
    /// - 表情格式无效
    /// - 已经添加过相同的反应（幂等操作，通常不应报错）
    ///
    /// # 示例
    ///
    /// ```no_run
    /// // 添加一个简单的 Unicode 表情
    /// channel.add_reaction("channel_id", "message_id", "👍").await?;
    ///
    /// // 添加自定义表情（格式：name:id）
    /// channel.add_reaction("channel_id", "message_id", "customemoji:123456789").await?;
    /// ```
    async fn add_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        let url = discord_reaction_url_with_base(
            self.discord_api_base_url(),
            channel_id,
            message_id,
            emoji,
        );

        let resp = self
            .http_client()
            .put(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Length", "0")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            // 读取错误响应体以便调试
            let err = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            // 清理错误信息中的敏感内容
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("Discord add reaction failed ({status}): {sanitized}");
        }

        Ok(())
    }

    /// 移除消息上的表情反应
    ///
    /// 调用 Discord API 移除指定消息上的表情反应。
    /// 仅能移除 Bot 自己添加的反应。
    ///
    /// # 参数
    ///
    /// - `channel_id`: 消息所在的频道 ID
    /// - `message_id`: 要移除反应的消息 ID
    /// - `emoji`: 要移除的表情符号
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 反应移除成功
    /// - `Err(e)`: 移除失败，包含状态码和错误信息
    ///
    /// # 注意
    ///
    /// - 只能移除 Bot 自己添加的反应
    /// - 如果反应不存在，API 仍会返回成功
    ///
    /// # 示例
    ///
    /// ```no_run
    /// channel.remove_reaction("channel_id", "message_id", "👍").await?;
    /// ```
    async fn remove_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        let url = discord_reaction_url_with_base(
            self.discord_api_base_url(),
            channel_id,
            message_id,
            emoji,
        );

        let resp = self
            .http_client()
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
            anyhow::bail!("Discord remove reaction failed ({status}): {sanitized}");
        }

        Ok(())
    }
}

/// 单元测试和集成测试
///
/// 测试按职责拆分在同目录下的 `tests/` 目录中，
/// 包含 Discord 通道各项功能的测试用例。
#[cfg(test)]
mod tests;
