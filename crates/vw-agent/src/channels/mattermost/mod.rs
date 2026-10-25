//! Mattermost 通道集成模块
//!
//! 本模块实现了与 Mattermost 团队协作平台的消息通道集成，通过 REST API v4 进行通信。
//! Mattermost 的 API 模式与 Slack 有许多相似之处，但使用了专用的 v4 结构。
//!
//! # 核心功能
//!
//! - **消息轮询**：通过 REST API 定期轮询频道帖子，获取新消息
//! - **消息发送**：支持发送消息到指定频道或帖子线程
//! - **线程回复**：可选择在原始帖子的 root_id 下回复，或直接在频道根级别回复
//! - **提及过滤**：支持仅响应 @ 提及机器人的消息
//! - **输入状态指示**：支持显示"正在输入"的打字指示器
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::channels::mattermost::MattermostChannel;
//! use crate::app::agent::channels::traits::Channel;
//!
//! let channel = MattermostChannel::new(
//!     "https://mm.example.com".to_string(),
//!     "bot_token".to_string(),
//!     Some("channel_id".to_string()),
//!     vec!["*".to_string()],  // 允许所有用户
//!     true,  // 启用线程回复
//!     false, // 不过滤提及
//! );
//!
//! // 检查连接健康状态
//! let is_healthy = channel.health_check().await;
//! ```
//!
//! # 架构说明
//!
//! 本模块遵循 trait 驱动架构，实现了 [`Channel`] trait，可与其他通道实现互换使用。

use super::traits::{Channel, ChannelMessage, SendMessage};
use anyhow::{Result, bail};
use async_trait::async_trait;
use parking_lot::Mutex;

/// Mattermost 通道实现
///
/// 通过 REST API v4 轮询频道帖子，实现与 Mattermost 平台的双向消息通信。
/// Mattermost 的 API 与 Slack 模式兼容，但使用专用的 v4 结构。
///
/// # 配置参数
///
/// - `base_url`：Mattermost 服务器地址（如 `https://mm.example.com`）
/// - `bot_token`：机器人访问令牌
/// - `channel_id`：要监听的频道 ID（可选）
/// - `allowed_users`：允许与机器人交互的用户 ID 列表
/// - `thread_replies`：是否在线程中回复（在原始帖子的 root_id 下回复）
/// - `mention_only`：是否仅响应 @ 提及机器人的消息
pub struct MattermostChannel {
    /// Mattermost 服务器基础 URL（如 https://mm.example.com）
    base_url: String,
    /// 机器人访问令牌，用于 API 认证
    bot_token: String,
    /// 要监听的频道 ID，None 表示不启用监听
    channel_id: Option<String>,
    /// 允许与机器人交互的用户 ID 列表
    /// - 空列表表示拒绝所有用户
    /// - 包含 "*" 表示允许所有用户
    allowed_users: Vec<String>,
    /// 线程回复模式开关
    /// - `true`（默认）：在原始帖子的 root_id 下回复，形成线程
    /// - `false`：直接在频道根级别回复
    thread_replies: bool,
    /// 仅响应提及模式开关
    /// - `true`：仅响应 @ 提及机器人的消息
    /// - `false`：响应所有消息
    mention_only: bool,
    /// 在频道/群组上下文中绕过提及限制的发送者 ID 列表
    /// 这些用户即使没有 @ 提及机器人，也能触发机器人响应
    group_reply_allowed_sender_ids: Vec<String>,
    /// 后台打字指示器循环的任务句柄
    /// 在调用 `stop_typing` 时会被中止
    typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl MattermostChannel {
    /// 创建新的 Mattermost 通道实例
    ///
    /// # 参数
    ///
    /// - `base_url`：Mattermost 服务器地址（如 `https://mm.example.com`）
    /// - `bot_token`：机器人访问令牌
    /// - `channel_id`：要监听的频道 ID，传 `None` 表示不启用监听
    /// - `allowed_users`：允许与机器人交互的用户 ID 列表
    ///   - 空列表：拒绝所有用户
    ///   - 包含 `"*"`：允许所有用户
    /// - `thread_replies`：是否在线程中回复
    /// - `mention_only`：是否仅响应 @ 提及的消息
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `MattermostChannel` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = MattermostChannel::new(
    ///     "https://mm.example.com".to_string(),
    ///     "your_bot_token".to_string(),
    ///     Some("abc123".to_string()),
    ///     vec!["user1".to_string(), "user2".to_string()],
    ///     true,
    ///     false,
    /// );
    /// ```
    pub fn new(
        base_url: String,
        bot_token: String,
        channel_id: Option<String>,
        allowed_users: Vec<String>,
        thread_replies: bool,
        mention_only: bool,
    ) -> Self {
        // 移除 base_url 尾部斜杠，确保路径拼接的一致性
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            base_url,
            bot_token,
            channel_id,
            allowed_users,
            thread_replies,
            mention_only,
            group_reply_allowed_sender_ids: Vec::new(),
            typing_handle: Mutex::new(None),
        }
    }

    /// 配置在频道/群组聊天中绕过提及限制的发送者 ID 列表
    ///
    /// 这些用户即使没有 @ 提及机器人，也能在群组上下文中触发机器人响应。
    /// 这对于管理员或特定角色的用户很有用，允许他们直接与机器人对话而无需每次都提及。
    ///
    /// # 参数
    ///
    /// - `sender_ids`：发送者 ID 列表，支持 `"*"` 表示允许所有用户
    ///
    /// # 返回值
    ///
    /// 返回更新了配置的 `Self`，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = MattermostChannel::new(/* ... */)
    ///     .with_group_reply_allowed_senders(vec![
    ///         "admin_user_id".to_string(),
    ///         "moderator_id".to_string(),
    ///     ]);
    /// ```
    pub fn with_group_reply_allowed_senders(mut self, sender_ids: Vec<String>) -> Self {
        self.group_reply_allowed_sender_ids = normalize_group_reply_allowed_sender_ids(sender_ids);
        self
    }

    /// 创建配置了代理的 HTTP 客户端
    ///
    /// 使用应用全局配置构建支持代理的 reqwest 客户端。
    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("channel.mattermost")
    }

    /// 检查用户 ID 是否在允许列表中
    ///
    /// # 参数
    ///
    /// - `user_id`：要检查的用户 ID
    ///
    /// # 返回值
    ///
    /// - `true`：用户被允许（列表为空时返回 false，列表包含 "*" 时返回 true）
    /// - `false`：用户不被允许
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// 检查群组发送者是否启用了触发权限
    ///
    /// 判断指定用户是否在允许绕过提及限制的列表中。
    ///
    /// # 参数
    ///
    /// - `user_id`：要检查的用户 ID
    ///
    /// # 返回值
    ///
    /// - `true`：用户可以绕过提及限制
    /// - `false`：用户不能绕过提及限制（空 ID 也返回 false）
    fn is_group_sender_trigger_enabled(&self, user_id: &str) -> bool {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return false;
        }
        self.group_reply_allowed_sender_ids.iter().any(|entry| entry == "*" || entry == user_id)
    }

    /// 获取机器人的用户 ID 和用户名
    ///
    /// 通过 API 查询当前认证用户的信息，用于：
    /// - 忽略机器人自己发送的消息
    /// - 检测 @ 提及（通过用户名）
    ///
    /// # 返回值
    ///
    /// 返回元组 `(user_id, username)`，如果查询失败则返回空字符串
    async fn get_bot_identity(&self) -> (String, String) {
        let resp: Option<serde_json::Value> = async {
            self.http_client()
                .get(format!("{}/api/v4/users/me", self.base_url))
                .bearer_auth(&self.bot_token)
                .send()
                .await
                .ok()?
                .json()
                .await
                .ok()
        }
        .await;

        let id = resp
            .as_ref()
            .and_then(|v| v.get("id"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        let username = resp
            .as_ref()
            .and_then(|v| v.get("username"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        (id, username)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for MattermostChannel {
    /// 返回通道名称标识
    ///
    /// # 返回值
    ///
    /// 返回 `"mattermost"` 字符串
    fn name(&self) -> &str {
        "mattermost"
    }

    /// 发送消息到 Mattermost 频道
    ///
    /// 支持发送到频道根级别或帖子线程中。Mattermost 通过 `root_id` 字段实现线程功能。
    /// 如果 `message.recipient` 格式为 `channel_id:root_id`，则回复到指定线程。
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息，包含内容和接收者信息
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：消息发送成功
    /// - `Err`：发送失败，包含错误信息
    ///
    /// # 错误处理
    ///
    /// 如果 API 返回非成功状态码，会返回包含状态码和错误详情的错误信息。
    /// 错误消息中的敏感信息会被净化处理。
    async fn send(&self, message: &SendMessage) -> Result<()> {
        // Mattermost 通过 'root_id' 支持线程功能
        // 如果 recipient 格式为 'channel_id:root_id'，则解析为线程回复
        let (channel_id, root_id) = if let Some((c, r)) = message.recipient.split_once(':') {
            (c, Some(r))
        } else {
            (message.recipient.as_str(), None)
        };

        // 构建请求体，包含频道 ID 和消息内容
        let mut body_map = serde_json::json!({
            "channel_id": channel_id,
            "message": message.content
        });

        // 如果是线程回复，添加 root_id 字段
        if let Some(root) = root_id {
            body_map
                .as_object_mut()
                .unwrap()
                .insert("root_id".to_string(), serde_json::Value::String(root.to_string()));
        }

        // 发送 POST 请求到 Mattermost API
        let resp = self
            .http_client()
            .post(format!("{}/api/v4/posts", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body_map)
            .send()
            .await?;

        // 检查响应状态
        let status = resp.status();
        if !status.is_success() {
            let body =
                resp.text().await.unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            let sanitized = crate::app::agent::providers::sanitize_api_error(&body);
            bail!("Mattermost post failed ({status}): {sanitized}");
        }

        Ok(())
    }

    /// 监听 Mattermost 频道消息
    ///
    /// 启动后台轮询循环，定期查询频道中的新帖子并通过通道发送给订阅者。
    /// 使用 `since` 参数增量获取新消息，避免重复处理。
    ///
    /// # 参数
    ///
    /// - `tx`：消息发送通道，用于将接收到的消息传递给订阅者
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：监听正常结束（通常是发送通道关闭）
    /// - `Err`：初始化失败或遇到不可恢复的错误
    ///
    /// # 工作流程
    ///
    /// 1. 获取机器人身份信息（用于过滤自己的消息和检测提及）
    /// 2. 记录当前时间戳作为起始点
    /// 3. 循环执行：
    ///    - 每 3 秒轮询一次新帖子
    ///    - 按时间顺序处理帖子
    ///    - 更新最后处理时间戳
    ///    - 将有效消息发送到订阅通道
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        // 必须配置 channel_id 才能启用监听
        let channel_id = self
            .channel_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Mattermost channel_id required for listening"))?;

        // 获取机器人身份信息，用于过滤和提及检测
        let (bot_user_id, bot_username) = self.get_bot_identity().await;

        // 初始化最后处理时间戳为当前时间（毫秒级）
        #[allow(clippy::cast_possible_truncation)]
        let mut last_create_at = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()) as i64;

        tracing::info!("Mattermost channel listening on {}...", channel_id);

        loop {
            // 每 3 秒轮询一次新消息
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            // 发送 GET 请求获取新帖子
            let resp = match self
                .http_client()
                .get(format!("{}/api/v4/channels/{}/posts", self.base_url, channel_id))
                .bearer_auth(&self.bot_token)
                .query(&[("since", last_create_at.to_string())])
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Mattermost poll error: {e}");
                    continue;
                }
            };

            // 解析响应 JSON
            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Mattermost parse error: {e}");
                    continue;
                }
            };

            // 处理返回的帖子列表
            if let Some(posts) = data.get("posts").and_then(|p| p.as_object()) {
                // 按创建时间升序排序，确保消息按时间顺序处理
                let mut post_list: Vec<_> = posts.values().collect();
                post_list.sort_by_key(|p| p.get("create_at").and_then(|c| c.as_i64()).unwrap_or(0));

                for post in post_list {
                    // 解析并过滤帖子
                    let msg = self.parse_mattermost_post(
                        post,
                        &bot_user_id,
                        &bot_username,
                        last_create_at,
                        &channel_id,
                    );

                    // 更新最后处理时间戳
                    let create_at =
                        post.get("create_at").and_then(|c| c.as_i64()).unwrap_or(last_create_at);
                    last_create_at = last_create_at.max(create_at);

                    // 发送有效消息到订阅通道
                    if let Some(channel_msg) = msg {
                        if tx.send(channel_msg).await.is_err() {
                            // 发送通道已关闭，退出监听
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// 检查通道健康状态
    ///
    /// 通过调用 `/api/v4/users/me` 端点验证：
    /// - 网络连接正常
    /// - 认证令牌有效
    ///
    /// # 返回值
    ///
    /// - `true`：健康检查通过
    /// - `false`：健康检查失败（网络问题或认证失败）
    async fn health_check(&self) -> bool {
        self.http_client()
            .get(format!("{}/api/v4/users/me", self.base_url))
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// 启动打字指示器
    ///
    /// 在 Mattermost 中显示"正在输入"状态。启动后台任务定期发送打字事件。
    /// Mattermost 的打字事件约 6 秒后过期，因此每 4 秒重新发送一次。
    ///
    /// # 参数
    ///
    /// - `recipient`：接收者标识，格式为 `channel_id` 或 `channel_id:root_id`
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：成功启动打字指示器
    /// - `Err`：启动失败
    ///
    /// # 注意事项
    ///
    /// 调用此方法会自动取消之前的打字指示器任务。
    async fn start_typing(&self, recipient: &str) -> Result<()> {
        // 在启动新的打字循环前，先取消现有的
        self.stop_typing(recipient).await?;

        let client = self.http_client();
        let token = self.bot_token.clone();
        let base_url = self.base_url.clone();

        // 解析 recipient：可能是 "channel_id" 或 "channel_id:root_id"
        let (channel_id, parent_id) = match recipient.split_once(':') {
            Some((channel, parent)) => (channel.to_string(), Some(parent.to_string())),
            None => (recipient.to_string(), None),
        };

        // 启动后台任务定期发送打字事件
        let handle = tokio::spawn(async move {
            let url = format!("{base_url}/api/v4/users/me/typing");
            loop {
                // 构建请求体
                let mut body = serde_json::json!({ "channel_id": channel_id });
                if let Some(ref pid) = parent_id {
                    body.as_object_mut()
                        .unwrap()
                        .insert("parent_id".to_string(), serde_json::json!(pid));
                }

                // 发送打字事件
                if let Ok(r) = client.post(&url).bearer_auth(&token).json(&body).send().await {
                    if !r.status().is_success() {
                        tracing::debug!(status = %r.status(), "Mattermost typing indicator failed");
                    }
                }

                // Mattermost 打字事件约 6 秒后过期，每 4 秒重新发送
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            }
        });

        // 保存任务句柄，以便后续可以中止
        let mut guard = self.typing_handle.lock();
        *guard = Some(handle);

        Ok(())
    }

    /// 停止打字指示器
    ///
    /// 中止后台打字事件发送任务，停止显示"正在输入"状态。
    ///
    /// # 参数
    ///
    /// - `_recipient`：接收者标识（未使用，保持接口一致性）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(())`
    async fn stop_typing(&self, _recipient: &str) -> Result<()> {
        let mut guard = self.typing_handle.lock();
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        Ok(())
    }
}

impl MattermostChannel {
    /// 解析 Mattermost 帖子为通道消息
    ///
    /// 从 Mattermost API 返回的帖子数据中提取并验证消息，转换为统一的通道消息格式。
    ///
    /// # 参数
    ///
    /// - `post`：Mattermost API 返回的帖子 JSON 数据
    /// - `bot_user_id`：机器人的用户 ID，用于过滤自己的消息
    /// - `bot_username`：机器人的用户名，用于检测 @ 提及
    /// - `last_create_at`：最后处理的帖子时间戳，用于增量过滤
    /// - `channel_id`：当前监听的频道 ID
    ///
    /// # 返回值
    ///
    /// - `Some(ChannelMessage)`：有效消息，通过了所有过滤条件
    /// - `None`：消息被过滤（自己发送的、时间太旧、用户未授权、未提及等）
    ///
    /// # 过滤逻辑
    ///
    /// 1. 跳过机器人自己发送的消息
    /// 2. 跳过时间戳早于或等于 `last_create_at` 的消息
    /// 3. 跳过空消息
    /// 4. 跳过未授权用户的消息
    /// 5. 如果启用了 `mention_only`，跳过未 @ 提及机器人的消息
    fn parse_mattermost_post(
        &self,
        post: &serde_json::Value,
        bot_user_id: &str,
        bot_username: &str,
        last_create_at: i64,
        channel_id: &str,
    ) -> Option<ChannelMessage> {
        // 提取帖子的基本字段
        let id = post.get("id").and_then(|i| i.as_str()).unwrap_or("");
        let user_id = post.get("user_id").and_then(|u| u.as_str()).unwrap_or("");
        let text = post.get("message").and_then(|m| m.as_str()).unwrap_or("");
        let create_at = post.get("create_at").and_then(|c| c.as_i64()).unwrap_or(0);
        let root_id = post.get("root_id").and_then(|r| r.as_str()).unwrap_or("");

        // 过滤：跳过自己的消息、旧消息、空消息
        if user_id == bot_user_id || create_at <= last_create_at || text.is_empty() {
            return None;
        }

        // 过滤：检查用户是否在允许列表中
        if !self.is_user_allowed(user_id) {
            tracing::warn!("Mattermost: ignoring message from unauthorized user: {user_id}");
            return None;
        }

        // 判断是否需要提及过滤
        let require_mention = self.mention_only && !self.is_group_sender_trigger_enabled(user_id);

        // 如果启用了 mention_only，过滤未提及的消息并规范化内容
        let content = if require_mention {
            let normalized = normalize_mattermost_content(text, bot_user_id, bot_username, post);
            normalized?
        } else {
            text.to_string()
        };

        // 回复路由逻辑：根据 thread_replies 配置决定回复目标
        //   - 已有线程（root_id 非空）：始终保持在同一线程中
        //   - 顶级帖子 + thread_replies=true：在原始帖子下创建线程回复
        //   - 顶级帖子 + thread_replies=false：直接在频道根级别回复
        let reply_target = if !root_id.is_empty() {
            format!("{}:{}", channel_id, root_id)
        } else if self.thread_replies {
            format!("{}:{}", channel_id, id)
        } else {
            channel_id.to_string()
        };

        Some(ChannelMessage {
            instance_id: "default".to_string(),
            id: format!("mattermost_{id}"),
            sender: user_id.to_string(),
            reply_target,
            content,
            channel: "mattermost".to_string(),
            #[allow(clippy::cast_sign_loss)]
            timestamp: (create_at / 1000) as u64,
            thread_ts: None,
        })
    }
}

/// 检查 Mattermost 帖子是否包含对机器人的 @ 提及
///
/// 检查两个来源：
/// 1. 文本检查：在消息正文中查找 `@bot_username`（不区分大小写）
/// 2. 元数据检查：检查帖子的 `metadata.mentions` 数组中是否包含机器人用户 ID
///
/// # 参数
///
/// - `text`：消息正文
/// - `bot_user_id`：机器人的用户 ID
/// - `bot_username`：机器人的用户名
/// - `post`：完整的帖子 JSON 数据
///
/// # 返回值
///
/// - `true`：消息包含对机器人的提及
/// - `false`：消息不包含提及
fn contains_bot_mention_mm(
    text: &str,
    bot_user_id: &str,
    bot_username: &str,
    post: &serde_json::Value,
) -> bool {
    // 方式 1：文本检查 - @username（不区分大小写，支持单词边界）
    if !find_bot_mention_spans(text, bot_username).is_empty() {
        return true;
    }

    // 方式 2：元数据检查 - Mattermost 可能在 "metadata.mentions" 数组中包含提及的用户 ID
    if !bot_user_id.is_empty() {
        if let Some(mentions) =
            post.get("metadata").and_then(|m| m.get("mentions")).and_then(|m| m.as_array())
        {
            if mentions.iter().any(|m| m.as_str() == Some(bot_user_id)) {
                return true;
            }
        }
    }

    false
}

/// 判断字符是否为有效的 Mattermost 用户名字符
///
/// Mattermost 用户名可以包含字母、数字、下划线、连字符和点。
///
/// # 参数
///
/// - `c`：要检查的字符
///
/// # 返回值
///
/// - `true`：是有效的用户名字符
/// - `false`：不是有效的用户名字符
fn is_mattermost_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
}

/// 在文本中查找所有对机器人的 @ 提及位置
///
/// 查找格式为 `@bot_username` 的提及，支持不区分大小写匹配，
/// 并确保匹配在单词边界处结束（避免部分匹配）。
///
/// # 参数
///
/// - `text`：要搜索的文本
/// - `bot_username`：机器人的用户名
///
/// # 返回值
///
/// 返回所有匹配位置的向量，每个元素为 `(start_index, end_index)` 元组。
/// 如果用户名为空或没有匹配，返回空向量。
///
/// # 示例
///
/// ```ignore
/// let spans = find_bot_mention_spans("@bot hello @bot_name", "bot");
/// // 返回: [(0, 4)]  - 只匹配 "@bot" 后面跟着非用户名字符的情况
/// ```
fn find_bot_mention_spans(text: &str, bot_username: &str) -> Vec<(usize, usize)> {
    if bot_username.is_empty() {
        return Vec::new();
    }

    let mention = format!("@{}", bot_username.to_ascii_lowercase());
    let mention_len = mention.len();
    if mention_len == 0 {
        return Vec::new();
    }

    let mention_bytes = mention.as_bytes();
    let text_bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;

    // 遍历文本，查找提及模式
    while index + mention_len <= text_bytes.len() {
        // 检查当前位置是否匹配提及模式（不区分大小写）
        let is_match = text_bytes[index] == b'@'
            && text_bytes[index..index + mention_len]
                .iter()
                .zip(mention_bytes.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right));

        if is_match {
            let end = index + mention_len;
            // 验证单词边界：确保匹配后不是有效的用户名字符
            let at_boundary =
                text[end..].chars().next().is_none_or(|next| !is_mattermost_username_char(next));
            if at_boundary {
                spans.push((index, end));
                index = end;
                continue;
            }
        }

        // 移动到下一个字符位置
        let step = text[index..].chars().next().map_or(1, char::len_utf8);
        index += step;
    }

    spans
}

/// 当 `mention_only` 模式启用时，规范化 Mattermost 内容
///
/// 检查消息是否提及机器人，如果提及则：
/// 1. 移除 @ 提及文本
/// 2. 修剪空白
/// 3. 返回清理后的内容
///
/// # 参数
///
/// - `text`：原始消息文本
/// - `bot_user_id`：机器人的用户 ID
/// - `bot_username`：机器人的用户名
/// - `post`：完整的帖子 JSON 数据
///
/// # 返回值
///
/// - `None`：消息未提及机器人
/// - `Some(cleaned)`：清理后的消息文本（移除了 @ 提及）
fn normalize_mattermost_content(
    text: &str,
    bot_user_id: &str,
    bot_username: &str,
    post: &serde_json::Value,
) -> Option<String> {
    // 查找文本中的提及位置
    let mention_spans = find_bot_mention_spans(text, bot_username);

    // 检查元数据中是否包含机器人 ID
    let metadata_mentions_bot = !bot_user_id.is_empty()
        && post
            .get("metadata")
            .and_then(|m| m.get("mentions"))
            .and_then(|m| m.as_array())
            .is_some_and(|mentions| mentions.iter().any(|m| m.as_str() == Some(bot_user_id)));

    // 如果既没有文本提及也没有元数据提及，返回 None
    if mention_spans.is_empty() && !metadata_mentions_bot {
        return None;
    }

    // 移除文本中的 @ 提及
    let mut cleaned = text.to_string();
    if !mention_spans.is_empty() {
        let mut result = String::with_capacity(text.len());
        let mut cursor = 0;
        for (start, end) in mention_spans {
            result.push_str(&text[cursor..start]);
            result.push(' '); // 用空格替换提及，避免单词粘连
            cursor = end;
        }
        result.push_str(&text[cursor..]);
        cleaned = result;
    }

    // 修剪空白
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned)
}

/// 规范化群组回复允许发送者 ID 列表
///
/// 对输入的发送者 ID 列表进行清理：
/// 1. 去除每个 ID 的首尾空白
/// 2. 过滤掉空字符串
/// 3. 排序
/// 4. 去重
///
/// # 参数
///
/// - `sender_ids`：原始发送者 ID 列表
///
/// # 返回值
///
/// 返回规范化后的发送者 ID 列表
fn normalize_group_reply_allowed_sender_ids(sender_ids: Vec<String>) -> Vec<String> {
    let mut normalized = sender_ids
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
