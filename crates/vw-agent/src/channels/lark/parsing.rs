//! 飞书/Lark 消息解析模块
//!
//! 本模块提供飞书（Lark）消息的解析和处理功能，主要用于将飞书事件回调中
//! 的各类消息格式转换为统一的通道消息结构。
//!
//! # 主要功能
//!
//! - 解析富文本（post）消息，提取纯文本和 @ 提及信息
//! - 解析图片消息，提取图片标识
//! - 处理群聊消息的响应策略（提及触发、白名单等）
//! - 移除飞书群聊中的 @ 占位符
//!
//! # 支持的消息类型
//!
//! - `text`: 纯文本消息
//! - `post`: 富文本消息
//! - `image`: 图片消息

use super::LarkChannel;
use super::constants::LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT;
use uuid::Uuid;

/// 飞书富文本（post）消息解析结果
///
/// 用于存储从飞书 post 消息中提取的纯文本内容和提及用户的 open_id 列表。
/// 当内容无法解析或没有有效文本时，调用者可以直接跳过该消息，
/// 而不是向代理转发无意义的占位符字符串。
///
/// # 字段说明
///
/// - `text`: 从富文本消息中提取的纯文本内容
/// - `mentioned_open_ids`: 被 @ 提及的用户的 open_id 列表
pub(crate) struct ParsedPostContent {
    /// 从富文本消息中提取的纯文本内容
    pub(crate) text: String,
    /// 被 @ 提及的用户的 open_id 列表，用于判断是否提及了机器人
    pub(crate) mentioned_open_ids: Vec<String>,
}

/// 解析飞书富文本（post）消息内容，提取文本和提及信息
///
/// 该函数将飞书的 post 消息 JSON 结构扁平化为纯文本，
/// 同时提取所有 @ 提及用户的 open_id。
///
/// # 参数
///
/// - `content`: 飞书 post 消息的 JSON 字符串内容
///
/// # 返回值
///
/// - `Some(ParsedPostContent)`: 解析成功，包含文本和提及列表
/// - `None`: 内容无法解析或没有有效文本
///
/// # 解析逻辑
///
/// 1. 首先尝试获取 `zh_cn` 语言区域的内容，然后尝试 `en_us`，
///    最后尝试任意可用的对象
/// 2. 提取标题（如果存在）
/// 3. 遍历所有段落和元素，处理以下标签：
///    - `text`: 直接添加文本内容
///    - `a`: 添加链接文本或 href
///    - `at`: 添加 @ 提及，并记录用户的 open_id
pub(crate) fn parse_post_content_details(content: &str) -> Option<ParsedPostContent> {
    // 解析 JSON 内容
    let parsed = serde_json::from_str::<serde_json::Value>(content).ok()?;

    // 按优先级获取语言区域：zh_cn > en_us > 任意可用对象
    let locale = parsed
        .get("zh_cn")
        .or_else(|| parsed.get("en_us"))
        .or_else(|| parsed.as_object().and_then(|m| m.values().find(|v| v.is_object())))?;

    let mut text = String::new();
    let mut mentioned_open_ids = Vec::new();

    // 提取标题（如果存在且非空）
    if let Some(title) = locale.get("title").and_then(|t| t.as_str()).filter(|s| !s.is_empty()) {
        text.push_str(title);
        text.push_str("\n\n");
    }

    // 遍历所有段落内容
    if let Some(paragraphs) = locale.get("content").and_then(|c| c.as_array()) {
        for para in paragraphs {
            if let Some(elements) = para.as_array() {
                // 处理段落中的每个元素
                for el in elements {
                    match el.get("tag").and_then(|t| t.as_str()).unwrap_or("") {
                        // 纯文本元素：直接添加文本
                        "text" => {
                            if let Some(t) = el.get("text").and_then(|t| t.as_str()) {
                                text.push_str(t);
                            }
                        }
                        // 链接元素：优先使用文本，其次使用 href
                        "a" => {
                            text.push_str(
                                el.get("text")
                                    .and_then(|t| t.as_str())
                                    .filter(|s| !s.is_empty())
                                    .or_else(|| el.get("href").and_then(|h| h.as_str()))
                                    .unwrap_or(""),
                            );
                        }
                        // @ 提及元素：添加 @ 前缀，记录 open_id
                        "at" => {
                            // 获取显示名称，优先使用 user_name，其次使用 user_id
                            let n = el
                                .get("user_name")
                                .and_then(|n| n.as_str())
                                .or_else(|| el.get("user_id").and_then(|i| i.as_str()))
                                .unwrap_or("user");
                            text.push('@');
                            text.push_str(n);
                            // 提取并记录被提及用户的 open_id
                            if let Some(open_id) = el
                                .get("user_id")
                                .and_then(|i| i.as_str())
                                .map(str::trim)
                                .filter(|id| !id.is_empty())
                            {
                                mentioned_open_ids.push(open_id.to_string());
                            }
                        }
                        // 其他标签类型忽略
                        _ => {}
                    }
                }
                // 每个段落结束后添加换行
                text.push('\n');
            }
        }
    }

    // 去除首尾空白，如果结果为空则返回 None
    let result = text.trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(ParsedPostContent { text: result, mentioned_open_ids })
    }
}

/// 解析飞书富文本消息，仅返回纯文本内容
///
/// 这是 `parse_post_content_details` 的简化版本，
/// 仅返回文本内容，不包含提及信息。
///
/// # 参数
///
/// - `content`: 飞书 post 消息的 JSON 字符串内容
///
/// # 返回值
///
/// - `Some(String)`: 解析成功，返回纯文本
/// - `None`: 内容无法解析或没有有效文本
#[allow(dead_code)]
pub(crate) fn parse_post_content(content: &str) -> Option<String> {
    parse_post_content_details(content).map(|details| details.text)
}

/// 从飞书图片消息内容中解析图片标识
///
/// 飞书图片消息的 content 字段是一个 JSON 字符串，
/// 包含 `image_key` 字段用于标识图片。
///
/// # 参数
///
/// - `content`: 飞书图片消息的 JSON 字符串内容
///
/// # 返回值
///
/// - `Some(String)`: 解析成功，返回图片标识
/// - `None`: 解析失败或没有 image_key 字段
pub(crate) fn parse_image_key(content: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|value| value.get("image_key").and_then(|key| key.as_str()).map(str::to_string))
}

/// 移除飞书群聊中的 @ 占位符
///
/// 在飞书群聊中，当用户 @ 某人时，消息文本中会插入类似 `@_user_1` 的占位符。
/// 该函数用于移除这些占位符，使文本更加干净。
///
/// # 参数
///
/// - `text`: 原始消息文本
///
/// # 返回值
///
/// 返回移除所有 `@_user_N` 占位符后的文本
///
/// # 示例
///
/// ```ignore
/// let text = "你好 @_user_1 世界";
/// let result = strip_at_placeholders(text);
/// assert_eq!(result, "你好  世界");
/// ```
pub(crate) fn strip_at_placeholders(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '@' {
            // 获取 @ 之后的所有字符
            let rest: String = chars.clone().map(|(_, c)| c).collect();
            // 检查是否匹配 @_user_ 前缀
            if let Some(after) = rest.strip_prefix("_user_") {
                // 计算需要跳过的字符数："_user_" + 数字部分
                let skip =
                    "_user_".len() + after.chars().take_while(|c| c.is_ascii_digit()).count();
                // 跳过这些字符（包括数字）
                for _ in 0..=skip {
                    chars.next();
                }
                // 如果后面有空格，也一并跳过
                if chars.peek().map(|(_, c)| *c == ' ').unwrap_or(false) {
                    chars.next();
                }
                continue;
            }
        }
        result.push(ch);
    }
    result
}

/// 检查提及信息是否匹配机器人的 open_id
///
/// 飞书的提及信息可能有两种结构：
/// 1. 使用 JSON pointer 路径 `/id/open_id`
/// 2. 直接包含 `open_id` 字段
///
/// # 参数
///
/// - `mention`: 提及信息的 JSON 值
/// - `bot_open_id`: 机器人的 open_id
///
/// # 返回值
///
/// 如果提及的 open_id 与机器人的 open_id 匹配，返回 `true`
fn mention_matches_bot_open_id(mention: &serde_json::Value, bot_open_id: &str) -> bool {
    mention
        .pointer("/id/open_id")
        .or_else(|| mention.pointer("/open_id"))
        .and_then(|v| v.as_str())
        .is_some_and(|value| value == bot_open_id)
}

/// 规范化群聊回复白名单发送者 ID 列表
///
/// 对发送者 ID 列表进行清理、去重和排序：
/// 1. 去除每个条目的首尾空白
/// 2. 过滤掉空字符串
/// 3. 排序并去重
///
/// # 参数
///
/// - `sender_ids`: 原始发送者 ID 列表
///
/// # 返回值
///
/// 返回规范化后的发送者 ID 列表
pub(crate) fn normalize_group_reply_allowed_sender_ids(sender_ids: Vec<String>) -> Vec<String> {
    let mut normalized = sender_ids
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

/// 检查发送者是否在群聊回复白名单中
///
/// 白名单支持两种特殊值：
/// - `"*"`: 匹配所有发送者（允许所有人触发回复）
/// - 具体的 open_id: 仅匹配该特定用户
///
/// # 参数
///
/// - `sender_open_id`: 发送者的 open_id
/// - `allowed_sender_ids`: 允许的发送者 ID 列表
///
/// # 返回值
///
/// 如果发送者在白名单中，返回 `true`
fn sender_has_group_reply_override(sender_open_id: &str, allowed_sender_ids: &[String]) -> bool {
    let sender_open_id = sender_open_id.trim();
    // 空的发送者 ID 不允许通过
    if sender_open_id.is_empty() {
        return false;
    }
    // 检查是否匹配通配符 "*" 或具体的 open_id
    allowed_sender_ids.iter().any(|entry| entry == "*" || entry == sender_open_id)
}

/// 判断是否应该在群聊中响应消息
///
/// 群聊响应策略：
/// 1. 如果发送者在白名单中（`group_reply_allowed_sender_ids`），始终响应
/// 2. 如果 `mention_only` 为 `false`，响应所有消息
/// 3. 如果 `mention_only` 为 `true`，仅当机器人被 @ 提及时才响应
///
/// # 参数
///
/// - `mention_only`: 是否启用仅提及触发模式
/// - `sender_open_id`: 发送者的 open_id
/// - `group_reply_allowed_sender_ids`: 群聊回复白名单
/// - `bot_open_id`: 机器人的 open_id（可选）
/// - `mentions`: 消息中的提及列表（来自飞书事件）
/// - `post_mentioned_open_ids`: post 消息中解析出的提及 open_id 列表
///
/// # 返回值
///
/// 如果应该响应该消息，返回 `true`
///
/// # 示例
///
/// ```ignore
/// // 白名单用户始终触发
/// let result = should_respond_in_group(
///     true,  // mention_only
///     "ou_xxx",
///     &["ou_xxx".to_string()],
///     Some("bot_open_id"),
///     &[],
///     &[],
/// );
/// assert!(result);
/// ```
pub(crate) fn should_respond_in_group(
    mention_only: bool,
    sender_open_id: &str,
    group_reply_allowed_sender_ids: &[String],
    bot_open_id: Option<&str>,
    mentions: &[serde_json::Value],
    post_mentioned_open_ids: &[String],
) -> bool {
    // 白名单发送者始终触发响应
    if sender_has_group_reply_override(sender_open_id, group_reply_allowed_sender_ids) {
        return true;
    }
    // 如果未启用仅提及模式，响应所有消息
    if !mention_only {
        return true;
    }
    // 检查机器人 open_id 是否有效
    let Some(bot_open_id) = bot_open_id.filter(|id| !id.is_empty()) else {
        return false;
    };
    // 如果没有任何提及信息，不响应
    if mentions.is_empty() && post_mentioned_open_ids.is_empty() {
        return false;
    }
    // 检查是否提及了机器人
    mentions.iter().any(|mention| mention_matches_bot_open_id(mention, bot_open_id))
        || post_mentioned_open_ids.iter().any(|id| id.as_str() == bot_open_id)
}

impl LarkChannel {
    /// 解析飞书事件回调载荷，提取消息列表
    ///
    /// 这是同步版本的解析器，对图片消息使用非网络回退文本。
    /// 主要用于不需要异步处理的场景。
    ///
    /// # 参数
    ///
    /// - `payload`: 飞书事件回调的 JSON 载荷
    ///
    /// # 返回值
    ///
    /// 返回解析出的消息列表。如果载荷无效或消息类型不支持，返回空列表。
    ///
    /// # 飞书事件 v2 结构
    ///
    /// ```json
    /// {
    ///   "header": { "event_type": "im.message.receive_v1" },
    ///   "event": {
    ///     "message": { ... },
    ///     "sender": { ... }
    ///   }
    /// }
    /// ```
    ///
    /// # 处理流程
    ///
    /// 1. 验证事件类型为 `im.message.receive_v1`
    /// 2. 提取发送者 open_id 并检查是否在允许列表中
    /// 3. 根据消息类型（text/post/image）解析内容
    /// 4. 对于群聊消息，检查响应策略
    /// 5. 构造并返回消息列表
    pub fn parse_event_payload(&self, payload: &serde_json::Value) -> Vec<super::ChannelMessage> {
        let mut messages = Vec::new();

        // 飞书事件 v2 结构：
        // { "header": { "event_type": "im.message.receive_v1" }, "event": { "message": { ... }, "sender": { ... } } }
        let event_type =
            payload.pointer("/header/event_type").and_then(|e| e.as_str()).unwrap_or("");

        // 仅处理消息接收事件
        if event_type != "im.message.receive_v1" {
            return messages;
        }

        // 获取事件对象
        let event = match payload.get("event") {
            Some(e) => e,
            None => return messages,
        };

        // 提取发送者 open_id
        let open_id =
            event.pointer("/sender/sender_id/open_id").and_then(|s| s.as_str()).unwrap_or("");

        // 发送者 open_id 为空则忽略
        if open_id.is_empty() {
            return messages;
        }

        // 检查发送者是否在允许列表中
        if !self.is_user_allowed(open_id) {
            tracing::warn!("Lark: ignoring message from unauthorized user: {open_id}");
            return messages;
        }

        // 提取消息类型（支持 text/post/image）
        let msg_type =
            event.pointer("/message/message_type").and_then(|t| t.as_str()).unwrap_or("");

        // 提取会话类型（p2p/group）
        let chat_type = event.pointer("/message/chat_type").and_then(|c| c.as_str()).unwrap_or("");

        // 提取消息中的提及列表
        let mentions = event
            .pointer("/message/mentions")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();

        // 提取消息内容字符串
        let content_str = event.pointer("/message/content").and_then(|c| c.as_str()).unwrap_or("");

        // 根据消息类型解析内容
        let (text, post_mentioned_open_ids): (String, Vec<String>) = match msg_type {
            // 纯文本消息：从 JSON 中提取 text 字段
            "text" => {
                let extracted =
                    serde_json::from_str::<serde_json::Value>(content_str).ok().and_then(|v| {
                        v.get("text")
                            .and_then(|t| t.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                    });
                match extracted {
                    Some(t) => (t, Vec::new()),
                    None => return messages,
                }
            }
            // 富文本消息：使用专门的解析函数
            "post" => match parse_post_content_details(content_str) {
                Some(details) => (details.text, details.mentioned_open_ids),
                None => return messages,
            },
            // 图片消息：使用回退文本（同步版本不下载图片）
            "image" => (LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT.to_string(), Vec::new()),
            // 不支持的消息类型
            _ => {
                tracing::debug!("Lark: skipping unsupported message type: {msg_type}");
                return messages;
            }
        };

        // 获取机器人的 open_id
        let bot_open_id = self.resolved_bot_open_id();

        // 对于群聊，检查是否应该响应
        if chat_type == "group"
            && !should_respond_in_group(
                self.mention_only,
                open_id,
                &self.group_reply_allowed_sender_ids,
                bot_open_id.as_deref(),
                &mentions,
                &post_mentioned_open_ids,
            )
        {
            return messages;
        }

        // 解析消息时间戳（飞书时间戳为毫秒）
        let timestamp = event
            .pointer("/message/create_time")
            .and_then(|t| t.as_str())
            .and_then(|t| t.parse::<u64>().ok())
            // 飞书时间戳是毫秒，需要转换为秒
            .map(|ms| ms / 1000)
            .unwrap_or_else(|| {
                // 如果无法解析时间戳，使用当前时间
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });

        // 提取会话 ID（群聊使用 chat_id，私聊使用 open_id）
        let chat_id = event.pointer("/message/chat_id").and_then(|c| c.as_str()).unwrap_or(open_id);

        // 构造消息对象
        messages.push(super::ChannelMessage {
            instance_id: "default".to_string(),
            id: Uuid::new_v4().to_string(),
            sender: open_id.to_string(),
            reply_target: chat_id.to_string(),
            content: text,
            channel: self.channel_name().to_string(),
            timestamp,
            thread_ts: message_thread(event),
        });

        messages
    }

    /// 解析飞书事件回调载荷（异步版本）
    ///
    /// 与 `parse_event_payload` 不同，此异步版本会尝试下载图片
    /// 并将图片内容转换为 `[IMAGE:data:...;base64,...]` 格式的标记。
    /// 主要用于 webhook 运行时路径。
    ///
    /// # 参数
    ///
    /// - `payload`: 飞书事件回调的 JSON 载荷
    ///
    /// # 返回值
    ///
    /// 返回解析出的消息列表。如果载荷无效或消息类型不支持，返回空列表。
    ///
    /// # 异步行为
    ///
    /// 对于图片消息，该函数会：
    /// 1. 解析 `image_key`
    /// 2. 异步下载图片并转换为 base64 标记
    /// 3. 如果下载失败，回退到默认文本
    pub async fn parse_event_payload_async(
        &self,
        payload: &serde_json::Value,
    ) -> Vec<super::ChannelMessage> {
        let mut messages = Vec::new();

        // 提取并验证事件类型
        let event_type =
            payload.pointer("/header/event_type").and_then(|e| e.as_str()).unwrap_or("");
        if event_type != "im.message.receive_v1" {
            return messages;
        }

        // 获取事件对象
        let event = match payload.get("event") {
            Some(e) => e,
            None => return messages,
        };

        // 提取发送者 open_id
        let open_id =
            event.pointer("/sender/sender_id/open_id").and_then(|s| s.as_str()).unwrap_or("");
        if open_id.is_empty() {
            return messages;
        }

        // 检查发送者是否在允许列表中
        if !self.is_user_allowed(open_id) {
            tracing::warn!("Lark: ignoring message from unauthorized user: {open_id}");
            return messages;
        }

        // 提取消息类型和会话类型
        let msg_type =
            event.pointer("/message/message_type").and_then(|t| t.as_str()).unwrap_or("");
        let chat_type = event.pointer("/message/chat_type").and_then(|c| c.as_str()).unwrap_or("");

        // 提取提及列表
        let mentions = event
            .pointer("/message/mentions")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();

        // 提取消息内容
        let content_str = event.pointer("/message/content").and_then(|c| c.as_str()).unwrap_or("");

        // 根据消息类型解析内容
        let (text, post_mentioned_open_ids): (String, Vec<String>) = match msg_type {
            // 纯文本消息
            "text" => {
                let extracted =
                    serde_json::from_str::<serde_json::Value>(content_str).ok().and_then(|v| {
                        v.get("text")
                            .and_then(|t| t.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                    });
                match extracted {
                    Some(t) => (t, Vec::new()),
                    None => return messages,
                }
            }
            // 富文本消息
            "post" => match parse_post_content_details(content_str) {
                Some(details) => (details.text, details.mentioned_open_ids),
                None => return messages,
            },
            // 图片消息：异步下载并转换
            "image" => {
                let text = if let Some(image_key) = parse_image_key(content_str) {
                    // 尝试下载图片并转换为标记
                    match self.fetch_image_marker(&image_key).await {
                        Ok(marker) => marker,
                        Err(error) => {
                            tracing::warn!(
                                "Lark webhook: failed to download image {image_key}: {error}"
                            );
                            LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT.to_string()
                        }
                    }
                } else {
                    tracing::warn!("Lark webhook: image message missing image_key");
                    LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT.to_string()
                };
                (text, Vec::new())
            }
            // 不支持的消息类型
            _ => {
                tracing::debug!("Lark: skipping unsupported message type: {msg_type}");
                return messages;
            }
        };

        // 获取机器人的 open_id
        let bot_open_id = self.resolved_bot_open_id();

        // 对于群聊，检查是否应该响应
        if chat_type == "group"
            && !should_respond_in_group(
                self.mention_only,
                open_id,
                &self.group_reply_allowed_sender_ids,
                bot_open_id.as_deref(),
                &mentions,
                &post_mentioned_open_ids,
            )
        {
            return messages;
        }

        // 解析消息时间戳（毫秒转秒）
        let timestamp = event
            .pointer("/message/create_time")
            .and_then(|t| t.as_str())
            .and_then(|t| t.parse::<u64>().ok())
            .map(|ms| ms / 1000)
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });

        // 提取会话 ID
        let chat_id = event.pointer("/message/chat_id").and_then(|c| c.as_str()).unwrap_or(open_id);

        // 构造消息对象
        messages.push(super::ChannelMessage {
            instance_id: "default".to_string(),
            id: Uuid::new_v4().to_string(),
            sender: open_id.to_string(),
            reply_target: chat_id.to_string(),
            content: text,
            channel: self.channel_name().to_string(),
            timestamp,
            thread_ts: message_thread(event),
        });

        messages
    }
}

#[cfg(test)]
#[path = "parsing_tests.rs"]
mod parsing_tests;

/// 平台主题标识不能与群聊身份或发送者混用。
pub(super) fn message_thread(event: &serde_json::Value) -> Option<String> {
    ["/message/thread_id", "/message/root_id"]
        .into_iter()
        .filter_map(|path| event.pointer(path).and_then(serde_json::Value::as_str))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}
