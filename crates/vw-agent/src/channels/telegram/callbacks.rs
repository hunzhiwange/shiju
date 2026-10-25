//! Telegram 频道回调处理模块
//!
//! 本模块提供 Telegram Bot API 的回调查询（Callback Query）处理功能，
//! 主要用于处理用户点击内联键盘按钮后的交互响应。
//!
//! ## 主要功能
//!
//! - **审批回调解析**：解析用户对审批请求的"同意"或"拒绝"响应
//! - **回调应答**：向 Telegram 服务器发送回调查询应答，确认用户操作
//! - **键盘清理**：清除消息的内联键盘按钮，防止重复操作
//!
//! ## 回调数据格式
//!
//! 审批回调数据使用以下前缀格式：
//! - `zcapr:yes:<request_id>` - 同意审批请求
//! - `zcapr:no:<request_id>` - 拒绝审批请求

use super::TelegramChannel;
use crate::app::agent::channels::traits::ChannelMessage;

/// 审批回调"同意"前缀
/// 用于标识用户点击了同意按钮的回调查询
const TELEGRAM_APPROVAL_CALLBACK_APPROVE_PREFIX: &str = "zcapr:yes:";

/// 审批回调"拒绝"前缀
/// 用于标识用户点击了拒绝按钮的回调查询
const TELEGRAM_APPROVAL_CALLBACK_DENY_PREFIX: &str = "zcapr:no:";

impl TelegramChannel {
    /// 解析审批回调命令
    ///
    /// 将 Telegram 回调查询的数据字符串转换为内部的审批命令格式。
    ///
    /// ## 参数
    ///
    /// - `data`: 回调查询中的数据字符串，格式为 `zcapr:yes:<request_id>` 或 `zcapr:no:<request_id>`
    ///
    /// ## 返回值
    ///
    /// 返回 `Some(String)` 包含解析后的命令：
    /// - `/approve-allow <request_id>` - 同意命令
    /// - `/approve-deny <request_id>` - 拒绝命令
    ///
    /// 如果数据格式不匹配或请求 ID 为空，则返回 `None`
    ///
    /// ## 示例
    ///
    /// ```ignore
    /// let cmd = parse_approval_callback_command("zcapr:yes:req_123");
    /// assert_eq!(cmd, Some("/approve-allow req_123".to_string()));
    ///
    /// let cmd = parse_approval_callback_command("zcapr:no:req_456");
    /// assert_eq!(cmd, Some("/approve-deny req_456".to_string()));
    /// ```
    pub(super) fn parse_approval_callback_command(data: &str) -> Option<String> {
        // 尝试匹配"同意"前缀
        if let Some(request_id) = data.strip_prefix(TELEGRAM_APPROVAL_CALLBACK_APPROVE_PREFIX) {
            // 确保请求 ID 非空
            if !request_id.trim().is_empty() {
                return Some(format!("/approve-allow {}", request_id.trim()));
            }
        }

        // 尝试匹配"拒绝"前缀
        if let Some(request_id) = data.strip_prefix(TELEGRAM_APPROVAL_CALLBACK_DENY_PREFIX) {
            // 确保请求 ID 非空
            if !request_id.trim().is_empty() {
                return Some(format!("/approve-deny {}", request_id.trim()));
            }
        }

        None
    }

    /// 非阻塞方式应答回调查询
    ///
    /// 向 Telegram API 发送回调查询应答，通知服务器已处理该回调。
    /// 此方法使用 `tokio::spawn` 异步执行，不会阻塞当前调用线程。
    ///
    /// ## 参数
    ///
    /// - `callback_id`: 回调查询的唯一标识符（由 Telegram 提供）
    /// - `text`: 向用户显示的提示文本（通常为简短的确认消息）
    ///
    /// ## 注意事项
    ///
    /// - 应答是可选的，但建议发送以提供用户反馈
    /// - 如果不发送应答，Telegram 会在短时间内显示加载状态
    /// - 发送应答后，加载状态会消失并显示提示文本
    pub(super) fn answer_callback_query_nonblocking(&self, callback_id: String, text: &str) {
        let client = self.http_client();
        let url = self.api_url("answerCallbackQuery");
        let text = text.to_string();

        // 在独立任务中异步发送应答，不阻塞当前执行流
        tokio::spawn(async move {
            let body = serde_json::json!({
                "callback_query_id": callback_id,
                "text": text,
                "show_alert": false  // 不显示弹窗，仅显示简短提示
            });
            // 忽略发送结果，因为这是非关键的确认操作
            let _ = client.post(&url).json(&body).send().await;
        });
    }

    /// 非阻塞方式清除消息的内联键盘
    ///
    /// 通过编辑消息的回复标记来移除内联键盘按钮。
    /// 通常在用户点击按钮后调用，以防止重复操作。
    ///
    /// ## 参数
    ///
    /// - `chat_id`: 聊天 ID（可以是用户 ID 或群组 ID）
    /// - `message_id`: 要编辑的消息 ID
    /// - `thread_id`: 可选的主题/话题 ID（用于超级群组的主题功能）
    ///
    /// ## 注意事项
    ///
    /// - 此方法使用 `tokio::spawn` 异步执行，不会阻塞
    /// - 清除键盘后，用户无法再次点击按钮
    /// - 如果消息已被删除或键盘已被修改，API 调用可能失败（错误被忽略）
    pub(super) fn clear_callback_inline_keyboard_nonblocking(
        &self,
        chat_id: String,
        message_id: i64,
        thread_id: Option<String>,
    ) {
        let client = self.http_client();
        let url = self.api_url("editMessageReplyMarkup");

        // 在独立任务中异步清除键盘，不阻塞当前执行流
        tokio::spawn(async move {
            // 构建请求体，设置空的内联键盘
            let mut body = serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id,
                "reply_markup": {
                    "inline_keyboard": []  // 空数组表示移除所有键盘按钮
                }
            });

            // 如果指定了主题 ID，添加到请求体中
            if let Some(thread_id) = thread_id {
                body["message_thread_id"] = serde_json::Value::String(thread_id);
            }

            // 忽略发送结果，因为键盘清理失败不影响主要流程
            let _ = client.post(&url).json(&body).send().await;
        });
    }

    /// 尝试解析审批回调查询并转换为通道消息
    ///
    /// 处理 Telegram 更新对象中的回调查询，提取审批决策并转换为
    /// 统一的 `ChannelMessage` 格式，以便代理系统处理。
    ///
    /// ## 参数
    ///
    /// - `update`: Telegram API 的更新对象（JSON 格式）
    ///
    /// ## 返回值
    ///
    /// 返回 `Some(ChannelMessage)` 如果：
    /// - 更新包含有效的回调查询
    /// - 回调数据匹配审批格式
    /// - 发送者在允许列表中
    ///
    /// 返回 `None` 如果：
    /// - 更新不包含回调查询
    /// - 回调数据格式不匹配
    /// - 发送者未被授权
    /// - 解析过程中发生错误
    ///
    /// ## 处理流程
    ///
    /// 1. 从更新中提取回调查询
    /// 2. 解析回调数据为审批命令
    /// 3. 提取聊天 ID、消息 ID 和发送者信息
    /// 4. 验证发送者是否在允许列表中
    /// 5. 发送回调查询应答给用户
    /// 6. 清除消息的内联键盘按钮
    /// 7. 构造并返回 `ChannelMessage`
    pub(super) fn try_parse_approval_callback_query(
        &self,
        update: &serde_json::Value,
    ) -> Option<ChannelMessage> {
        // 从更新中提取回调查询对象
        let callback = update.get("callback_query")?;

        // 提取回调 ID 和数据
        let callback_id = callback.get("id").and_then(serde_json::Value::as_str)?;
        let data = callback.get("data").and_then(serde_json::Value::as_str)?;

        // 解析回调数据为审批命令
        let content = Self::parse_approval_callback_command(data)?;

        // 提取原始消息信息
        let message = callback.get("message")?;

        // 提取聊天 ID
        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string())?;

        // 提取消息 ID，默认为 0（理论上不应发生）
        let message_id = message.get("message_id").and_then(serde_json::Value::as_i64).unwrap_or(0);

        // 提取发送者身份信息（用户名和 ID）
        let (username, sender_id, sender_identity) = Self::extract_sender_info(callback);

        // 构建身份列表用于权限检查
        let mut identities = vec![username.as_str()];
        if let Some(id) = sender_id.as_deref() {
            identities.push(id);
        }

        // 验证发送者是否在允许列表中
        if !self.is_any_user_allowed(identities.iter().copied()) {
            return None;
        }

        // 提取话题/主题 ID（如果存在）
        let thread_id = message
            .get("message_thread_id")
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string());

        // 构建回复目标：如果是话题消息，格式为 "chat_id:thread_id"
        let reply_target = if let Some(ref tid) = thread_id {
            format!("{chat_id}:{tid}")
        } else {
            chat_id.clone()
        };

        // 发送回调应答，通知用户操作已被接收
        self.answer_callback_query_nonblocking(callback_id.to_string(), "Decision received");

        // 清除内联键盘，防止重复操作
        self.clear_callback_inline_keyboard_nonblocking(
            chat_id.clone(),
            message_id,
            thread_id.clone(),
        );

        // 构造并返回通道消息
        Some(ChannelMessage {
            instance_id: "default".into(),
            // 生成唯一的消息 ID，包含聊天、消息和回调信息
            id: format!("telegram_cb_{chat_id}_{message_id}_{callback_id}"),
            sender: sender_identity,
            reply_target,
            content, // 包含解析后的审批命令
            channel: "telegram".to_string(),
            // 获取当前 Unix 时间戳
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            thread_ts: thread_id, // 话题/主题时间戳
        })
    }
}
