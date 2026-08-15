//! Telegram 入站消息解析模块。
//!
//! 本模块把 Telegram `getUpdates` 返回的 JSON 消息转换为通道统一的
//! `ChannelMessage`。解析过程中会执行用户允许列表、群组提及要求和回复上下文
//! 抽取，确保只有授权且可定位回复目标的消息进入代理运行时。

use super::TelegramChannel;
use crate::app::agent::channels::traits::ChannelMessage;

impl TelegramChannel {
    /// 解析回复目标中的聊天 ID 和可选话题 ID。
    ///
    /// # 参数
    /// - `reply_target`: 通道内部使用的回复目标，格式为 `chat_id` 或 `chat_id:thread_id`。
    ///
    /// # 返回值
    /// 返回 `(chat_id, thread_id)`，其中 `thread_id` 在普通聊天中为 `None`。
    pub(super) fn parse_reply_target(reply_target: &str) -> (String, Option<String>) {
        if let Some((chat_id, thread_id)) = reply_target.split_once(':') {
            (chat_id.to_string(), Some(thread_id.to_string()))
        } else {
            (reply_target.to_string(), None)
        }
    }

    /// 从 Telegram 更新中提取可添加确认反应的消息目标。
    ///
    /// # 参数
    /// - `update`: Telegram 更新 JSON。
    ///
    /// # 返回值
    /// 成功时返回 `(chat_id, message_id)`；缺少普通消息字段时返回 `None`。
    pub(super) fn extract_update_message_target(
        update: &serde_json::Value,
    ) -> Option<(String, i64)> {
        let message = update.get("message")?;
        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(serde_json::Value::as_i64)?
            .to_string();
        let message_id = message.get("message_id").and_then(serde_json::Value::as_i64)?;
        Some((chat_id, message_id))
    }

    /// 判断消息是否来自群组或超级群组。
    ///
    /// # 参数
    /// - `message`: Telegram 消息 JSON。
    ///
    /// # 返回值
    /// 群组和超级群组返回 `true`，私聊或缺失类型返回 `false`。
    pub(super) fn is_group_message(message: &serde_json::Value) -> bool {
        message
            .get("chat")
            .and_then(|c| c.get("type"))
            .and_then(|t| t.as_str())
            .map(|t| t == "group" || t == "supergroup")
            .unwrap_or(false)
    }

    /// 提取发送者的用户名、数字 ID 和用于展示/隔离会话的身份。
    ///
    /// # 参数
    /// - `message`: Telegram 消息 JSON。
    ///
    /// # 返回值
    /// 返回 `(username, sender_id, sender_identity)`。当用户名缺失时，
    /// `sender_identity` 会回退到数字 ID，再回退到 `"unknown"`。
    pub(super) fn extract_sender_info(
        message: &serde_json::Value,
    ) -> (String, Option<String>, String) {
        let username = message
            .get("from")
            .and_then(|from| from.get("username"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let sender_id = message
            .get("from")
            .and_then(|from| from.get("id"))
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string());
        let sender_identity = if username == "unknown" {
            sender_id.clone().unwrap_or_else(|| "unknown".to_string())
        } else {
            username.clone()
        };
        (username, sender_id, sender_identity)
    }

    /// 提取被回复消息的简要引用上下文。
    ///
    /// # 参数
    /// - `message`: 当前 Telegram 消息 JSON。
    ///
    /// # 返回值
    /// 当前消息是回复时返回 Markdown 风格引用文本，否则返回 `None`。
    ///
    /// # 说明
    /// 媒体类回复会生成类型占位符，让代理知道用户是在回应哪类上文。
    pub(super) fn extract_reply_context(&self, message: &serde_json::Value) -> Option<String> {
        let reply = message.get("reply_to_message")?;

        let reply_sender = reply
            .get("from")
            .and_then(|from| from.get("username"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                reply
                    .get("from")
                    .and_then(|from| from.get("first_name"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or("unknown");

        let reply_text = if let Some(text) = reply.get("text").and_then(serde_json::Value::as_str) {
            text.to_string()
        } else if reply.get("voice").is_some() || reply.get("audio").is_some() {
            "[Voice message]".to_string()
        } else if reply.get("photo").is_some() {
            "[Photo]".to_string()
        } else if reply.get("document").is_some() {
            "[Document]".to_string()
        } else if reply.get("video").is_some() {
            "[Video]".to_string()
        } else if reply.get("sticker").is_some() {
            "[Sticker]".to_string()
        } else {
            "[Message]".to_string()
        };

        let quoted_lines: String =
            reply_text.lines().map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n");

        Some(format!("> @{reply_sender}:\n{quoted_lines}"))
    }

    /// 将 Telegram 文本更新解析为统一通道消息。
    ///
    /// # 参数
    /// - `update`: Telegram 更新 JSON。
    ///
    /// # 返回值
    /// 授权且可处理的文本消息返回 `Some(ChannelMessage)`；非文本消息、未授权用户、
    /// 群组中未提及机器人的消息等返回 `None`。
    ///
    /// # 安全说明
    /// 解析时同时使用 username 和数字 ID 做允许列表匹配，避免仅依赖可变用户名。
    /// 群组 mention-only 模式下会要求明确提及 bot，除非发送者被配置为可免提及触发。
    pub(super) fn parse_update_message(
        &self,
        update: &serde_json::Value,
    ) -> Option<ChannelMessage> {
        let message = update.get("message")?;

        let text = message.get("text").and_then(serde_json::Value::as_str)?;

        let (username, sender_id, sender_identity) = Self::extract_sender_info(message);

        let mut identities = vec![username.as_str()];
        if let Some(id) = sender_id.as_deref() {
            identities.push(id);
        }

        // 默认拒绝未授权用户，避免 Telegram bot 暴露在群组或转发场景时扩大访问面。
        if !self.is_any_user_allowed(identities.iter().copied()) {
            return None;
        }

        let is_group = Self::is_group_message(message);
        let allow_sender_without_mention =
            is_group && self.is_group_sender_trigger_enabled(sender_id.as_deref());

        if self.mention_only && is_group && !allow_sender_without_mention {
            let bot_username = self.bot_username.lock();
            if let Some(ref bot_username) = *bot_username {
                if !Self::contains_bot_mention(text, bot_username) {
                    return None;
                }
            } else {
                return None;
            }
        }

        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string())?;

        let message_id = message.get("message_id").and_then(serde_json::Value::as_i64).unwrap_or(0);

        let thread_id = message
            .get("message_thread_id")
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string());

        let reply_target = if let Some(ref tid) = thread_id {
            format!("{}:{}", chat_id, tid)
        } else {
            chat_id.clone()
        };

        let content = if self.mention_only && is_group && !allow_sender_without_mention {
            let bot_username = self.bot_username.lock();
            let bot_username = bot_username.as_ref()?;
            // 转交给代理前移除 bot 提及，让模型只看到用户真正的问题文本。
            Self::normalize_incoming_content(text, bot_username)?
        } else {
            text.to_string()
        };

        let content = if let Some(quote) = self.extract_reply_context(message) {
            format!("{quote}\n\n{content}")
        } else {
            content
        };

        Some(ChannelMessage {
            id: format!("telegram_{chat_id}_{message_id}"),
            sender: sender_identity,
            reply_target,
            content,
            channel: "telegram".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            thread_ts: thread_id,
        })
    }
}
