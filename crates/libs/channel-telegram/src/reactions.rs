//! Telegram 消息反应功能模块
//!
//! 本模块为 [`TelegramChannel`] 提供 Telegram 消息反应（reaction）相关的功能实现。
//! 主要用于在消息上添加确认反应（ACK reaction），以向用户反馈消息已被接收或处理。
//!
//! # 功能概述
//!
//! - 非阻塞式添加消息反应
//! - 随机选择确认表情符号
//! - 自动处理网络错误和 API 错误
//! - 错误信息自动脱敏处理
//!
//! # 使用场景
//!
//! 当代理收到用户消息并开始处理时，可以通过添加表情反应来提供即时反馈，
//! 例如添加一个 👀 或 👍 表情，表示"我已看到您的消息"。

use super::TelegramChannel;
use super::message_utils::{build_telegram_ack_reaction_request, random_telegram_ack_reaction};

/// TelegramChannel 的消息反应扩展实现
///
/// 此实现块为 [`TelegramChannel`] 添加与消息反应相关的方法。
/// 所有方法都是非阻塞的，通过 `tokio::spawn` 在后台异步执行，
/// 不会影响主消息处理流程的性能。
impl TelegramChannel {
    /// 尝试为指定消息添加确认反应（非阻塞）
    ///
    /// 此方法会向 Telegram API 发送请求，在指定的消息上添加一个随机的确认表情反应。
    /// 该操作是完全非阻塞的，会在后台异步执行，不会阻塞调用线程。
    ///
    /// # 参数
    ///
    /// - `chat_id`: Telegram 聊天 ID，可以是用户 ID、群组 ID 或频道用户名
    /// - `message_id`: 目标消息的 ID，用于定位需要添加反应的具体消息
    ///
    /// # 行为说明
    ///
    /// 1. 方法会随机选择一个确认表情（如 👍、👀 等）
    /// 2. 构建符合 Telegram API 规范的请求体
    /// 3. 通过 `tokio::spawn` 在后台异步发送 HTTP 请求
    /// 4. 如果请求失败，会记录警告日志但不影响主流程
    /// 5. 错误信息在记录前会经过脱敏处理，防止泄露敏感数据
    ///
    /// # 错误处理
    ///
    /// - **网络错误**: 记录脱敏后的错误信息并静默返回
    /// - **API 错误**: 记录 HTTP 状态码和脱敏后的响应体
    /// - **不会 panic**: 所有错误都会被优雅处理
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(config);
    /// // 在后台添加确认反应，不阻塞当前任务
    /// channel.try_add_ack_reaction_nonblocking("123456789".to_string(), 42);
    /// // 方法立即返回，反应添加在后台进行
    /// ```
    ///
    /// # 注意事项
    ///
    /// - 此方法是"尽力而为"的，不保证反应一定会被添加成功
    /// - 机器人需要有足够的权限才能在目标聊天中添加反应
    /// - 某些群组可能限制了机器人添加反应的权限
    pub(super) fn try_add_ack_reaction_nonblocking(&self, chat_id: String, message_id: i64) {
        // 获取 HTTP 客户端引用，用于发送 API 请求
        let client = self.http_client();

        // 构建 Telegram Bot API 的完整 URL，使用 setMessageReaction 端点
        let url = self.api_url("setMessageReaction");

        // 随机选择一个确认表情符号，增加交互的趣味性
        let emoji = random_telegram_ack_reaction().to_string();

        // 构建请求体，包含聊天 ID、消息 ID 和表情符号
        let body = build_telegram_ack_reaction_request(&chat_id, message_id, &emoji);

        // 在后台异步任务中执行 HTTP 请求，不阻塞调用线程
        tokio::spawn(async move {
            // 发送 POST 请求到 Telegram API
            let response = match client.post(&url).json(&body).send().await {
                Ok(resp) => resp,
                Err(err) => {
                    // 网络请求失败，脱敏处理错误信息后记录警告
                    let sanitized = TelegramChannel::sanitize_telegram_error(&err.to_string());
                    tracing::warn!(
                        "Telegram: failed to add ACK reaction to chat_id={chat_id}, message_id={message_id}: {sanitized}"
                    );
                    return;
                }
            };

            // 检查 API 响应状态，非成功状态时记录详细错误信息
            if !response.status().is_success() {
                let status = response.status();
                // 尝试读取错误响应体，失败时使用空字符串
                let err_body = response.text().await.unwrap_or_default();
                // 脱敏处理错误响应体，防止日志中泄露敏感信息
                let sanitized = TelegramChannel::sanitize_telegram_error(&err_body);
                tracing::warn!(
                    "Telegram: add ACK reaction failed for chat_id={chat_id}, message_id={message_id}: status={status}, body={sanitized}"
                );
            }
        });
    }
}
