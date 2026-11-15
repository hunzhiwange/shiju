//! Telegram 通道实现模块
//!
//! 本模块为 `TelegramChannel` 提供 `Channel` trait 的具体实现，
//! 实现 Telegram Bot API 的核心消息收发功能。
//!
//! # 主要功能
//!
//! - **消息发送**：支持普通消息和草稿消息的发送
//! - **流式消息**：支持可更新的草稿消息（基于流模式）
//! - **审批提示**：发送带内联按钮的工具调用审批请求
//! - **消息监听**：通过轮询机制接收 Telegram 更新
//! - **打字状态**：管理"正在输入"的聊天动作指示器
//! - **健康检查**：验证 Bot Token 和 API 连接状态
//!
//! # 架构说明
//!
//! 本实现采用委托模式，将具体逻辑委托给 `TelegramChannel` 的辅助方法，
//! 保持 trait 实现简洁，同时便于测试和维护。

use super::TelegramChannel;
use channel_types::{Channel, ChannelMessage, SendMessage};
use agent_config::runtime::StreamMode;
use async_trait::async_trait;

/// 为 TelegramChannel 实现 Channel trait
///
/// 该实现提供了与 Telegram Bot API 交互的完整通道功能，
/// 支持异步操作和跨平台（包括 WASM）运行。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for TelegramChannel {
    /// 返回通道的标识名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"telegram"`，用于在系统中标识此通道类型。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(config);
    /// assert_eq!(channel.name(), "telegram");
    /// ```
    fn name(&self) -> &str {
        "telegram"
    }

    /// 检查是否支持草稿消息更新
    ///
    /// 草稿更新功能允许在消息完成前持续编辑已发送的消息内容，
    /// 这对于流式输出场景非常有用。
    ///
    /// # 返回值
    ///
    /// - `true`：如果流模式未关闭（`StreamMode::Off` 之外的模式）
    /// - `false`：如果流模式已关闭
    ///
    /// # 说明
    ///
    /// Telegram Bot API 支持编辑已发送消息，因此当启用流模式时，
    /// 可以通过不断更新消息内容来展示流式输出效果。
    fn supports_draft_updates(&self) -> bool {
        self.stream_mode != StreamMode::Off
            || self.allowed_users.read().is_ok_and(|users| users.is_empty())
    }

    /// 发送草稿消息
    ///
    /// 发送一条可后续更新的消息，用于流式输出场景。
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息内容，包含接收者和文本等
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(message_id))`：成功发送，返回消息 ID 用于后续更新
    /// - `Ok(None)`：成功发送但无消息 ID（某些情况下可能发生）
    /// - `Err(e)`：发送失败
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `send_draft_impl` 方法。
    async fn send_draft(&self, message: &SendMessage) -> anyhow::Result<Option<String>> {
        self.send_draft_impl(message).await
    }

    /// 更新已发送的草稿消息
    ///
    /// 修改之前发送的草稿消息的文本内容，用于流式输出更新。
    ///
    /// # 参数
    ///
    /// - `recipient`：消息接收者（聊天 ID 或用户名）
    /// - `message_id`：要更新的消息 ID
    /// - `text`：新的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(new_message_id))`：更新成功，可能返回新的消息 ID
    /// - `Ok(None)`：更新成功但无新消息 ID
    /// - `Err(e)`：更新失败
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `update_draft_impl` 方法。
    async fn update_draft(
        &self,
        recipient: &str,
        message_id: &str,
        text: &str,
    ) -> anyhow::Result<Option<String>> {
        self.update_draft_impl(recipient, message_id, text).await
    }

    /// 完成草稿消息
    ///
    /// 将草稿消息标记为最终版本，表示流式输出已完成。
    /// 调用后不应再更新该消息。
    ///
    /// # 参数
    ///
    /// - `recipient`：消息接收者
    /// - `message_id`：要完成的消息 ID
    /// - `text`：最终的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：成功完成
    /// - `Err(e)`：操作失败
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `finalize_draft_impl` 方法。
    async fn finalize_draft(
        &self,
        recipient: &str,
        message_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        self.finalize_draft_impl(recipient, message_id, text).await
    }

    /// 取消草稿消息
    ///
    /// 取消并删除之前发送的草稿消息，通常用于错误处理或用户中断场景。
    ///
    /// # 参数
    ///
    /// - `recipient`：消息接收者
    /// - `message_id`：要取消的消息 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：成功取消
    /// - `Err(e)`：操作失败
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `cancel_draft_impl` 方法。
    async fn cancel_draft(&self, recipient: &str, message_id: &str) -> anyhow::Result<()> {
        self.cancel_draft_impl(recipient, message_id).await
    }

    /// 发送最终消息
    ///
    /// 发送一条普通消息，不提供更新能力。适用于一次性消息发送。
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：发送成功
    /// - `Err(e)`：发送失败
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `send_outbound` 方法。
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.send_outbound(message).await
    }

    /// 发送工具调用审批提示
    ///
    /// 发送一条带有"批准/拒绝"内联按钮的消息，用于请求用户审批工具调用。
    /// 用户点击按钮后，Telegram 会发送回调查询，由监听器处理。
    ///
    /// # 参数
    ///
    /// - `recipient`：审批请求的接收者（聊天 ID，可能包含线程 ID）
    /// - `request_id`：审批请求的唯一标识符，用于匹配回调响应
    /// - `tool_name`：待审批的工具名称
    /// - `arguments`：工具调用的参数（JSON 格式）
    /// - `thread_ts`：可选的线程/话题 ID，用于在特定话题中发送消息
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：审批提示发送成功
    /// - `Err(e)`：发送失败（如网络错误、API 错误等）
    ///
    /// # 实现细节
    ///
    /// 1. 解析接收者字符串，提取聊天 ID 和可选的线程 ID
    /// 2. 如果参数过长（>260 字符），进行截断并添加省略号
    /// 3. 构建包含审批信息的消息文本
    /// 4. 创建带两个按钮的内联键盘："Approve" 和 "Deny"
    /// 5. 按钮的回调数据格式为 `zcapr:yes:{request_id}` 或 `zcapr:no:{request_id}`
    /// 6. 如果有线程 ID，添加到请求体中以支持话题消息
    ///
    /// # 错误处理
    ///
    /// 如果 Telegram API 返回非成功状态码，会提取错误信息并进行净化处理，
    /// 避免泄露敏感信息（如 Bot Token）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// channel.send_approval_prompt(
    ///     "123456789",
    ///     "req-123",
    ///     "delete_file",
    ///     &serde_json::json!({"path": "/tmp/test.txt"}),
    ///     None,
    /// ).await?;
    /// ```
    async fn send_approval_prompt(
        &self,
        recipient: &str,
        request_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        thread_ts: Option<String>,
    ) -> anyhow::Result<()> {
        // 解析接收者目标，提取聊天 ID 和可能的线程 ID
        let (chat_id, parsed_thread_id) = Self::parse_reply_target(recipient);
        // 优先使用解析出的线程 ID，如果没有则使用参数提供的线程 ID
        let thread_id = parsed_thread_id.or(thread_ts);

        // 将参数转换为字符串用于显示
        let raw_args = arguments.to_string();
        // 如果参数过长，截断到 260 字符并添加省略号，避免消息过长
        let args_preview =
            if raw_args.len() > 260 { format!("{}...", &raw_args[..260]) } else { raw_args };

        // 构建消息请求体，包含文本和内联键盘按钮
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            // 构建审批提示消息文本
            "text": format!(
                "Approval required for tool `{tool_name}`.\nRequest ID: `{request_id}`\nArgs: `{args_preview}`",
            ),
            // 内联键盘提供批准和拒绝按钮
            "reply_markup": {
                "inline_keyboard": [[
                    {
                        "text": "Approve",
                        // 批准按钮的回调数据格式：zcapr:yes:{request_id}
                        "callback_data": format!("zcapr:yes:{request_id}")
                    },
                    {
                        "text": "Deny",
                        // 拒绝按钮的回调数据格式：zcapr:no:{request_id}
                        "callback_data": format!("zcapr:no:{request_id}")
                    }
                ]]
            }
        });

        // 如果有线程 ID（话题/群组中的特定话题），添加到请求体中
        if let Some(thread_id) = thread_id {
            body["message_thread_id"] = serde_json::Value::String(thread_id);
        }

        // 发送 API 请求到 Telegram sendMessage 端点
        let response =
            self.http_client().post(self.api_url("sendMessage")).json(&body).send().await?;

        // 检查响应状态，失败时提取并净化错误信息
        if !response.status().is_success() {
            let status = response.status();
            let err = response.text().await.unwrap_or_default();
            // 净化错误信息，移除可能包含的敏感数据（如 Bot Token）
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram approval prompt failed ({status}): {sanitized}");
        }

        Ok(())
    }

    /// 启动消息监听
    ///
    /// 开始监听来自 Telegram 的消息和更新，通过轮询机制实现。
    /// 接收到的消息会通过通道发送给调用者。
    ///
    /// # 参数
    ///
    /// - `tx`：多生产者单消费者通道的发送端，用于将接收到的消息传递给消费者
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：监听正常启动（通常会持续运行直到发生错误）
    /// - `Err(e)`：启动或运行过程中发生错误
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `listen_impl` 方法。
    ///
    /// # 说明
    ///
    /// 该方法通常会阻塞运行，持续轮询 Telegram API 获取新更新。
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        self.listen_impl(tx).await
    }

    /// 执行健康检查
    ///
    /// 验证 Telegram Bot 的连接状态和配置有效性。
    /// 通常通过调用 `getMe` API 端点来验证 Bot Token 是否有效。
    ///
    /// # 返回值
    ///
    /// - `true`：健康检查通过，Bot 配置正确且 API 可达
    /// - `false`：健康检查失败，Bot 不可用或配置错误
    ///
    /// # 委托
    ///
    /// 实际逻辑委托给 `health_check_impl` 方法。
    async fn health_check(&self) -> bool {
        self.health_check_impl().await
    }

    /// 启动"正在输入"状态指示器
    ///
    /// 在指定聊天中显示"正在输入"的聊天动作，告知用户 Bot 正在处理消息。
    /// 该方法会启动一个后台任务，定期发送输入状态（每 4 秒一次），
    /// 直到调用 `stop_typing` 为止。
    ///
    /// # 参数
    ///
    /// - `recipient`：目标聊天的 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：成功启动输入状态指示器
    /// - `Err(e)`：启动失败
    ///
    /// # 实现细节
    ///
    /// 1. 先停止任何现有的输入状态任务，避免冲突
    /// 2. 创建新的异步任务，循环发送 `typing` 聊天动作
    /// 3. Telegram 的输入状态有效期约 5 秒，因此每 4 秒发送一次以保持连续
    /// 4. 将任务句柄保存到 `typing_handle` 字段，便于后续停止
    ///
    /// # 注意
    ///
    /// - 输入状态需要定期刷新，否则会自动消失
    /// - 多次调用会停止之前的任务并启动新任务
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()> {
        // 先停止现有的输入状态任务，避免同时运行多个任务
        self.stop_typing(recipient).await?;

        // 获取 HTTP 客户端和 API URL 的副本，用于异步任务
        let client = self.http_client();
        let url = self.api_url("sendChatAction");
        let chat_id = recipient.to_string();

        // 启动后台任务，定期发送输入状态
        let handle = tokio::spawn(async move {
            loop {
                // 构建聊天动作请求体
                let body = serde_json::json!({
                    "chat_id": &chat_id,
                    "action": "typing" // "typing" 表示正在输入
                });
                // 发送请求，忽略错误（失败不影响主流程）
                let _ = client.post(&url).json(&body).send().await;
                // 等待 4 秒后再次发送
                // Telegram 的输入状态持续约 5 秒，4 秒刷新一次确保连续性
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            }
        });

        // 保存任务句柄，便于后续停止
        let mut guard = self.typing_handle.lock();
        *guard = Some(handle);

        Ok(())
    }

    /// 停止"正在输入"状态指示器
    ///
    /// 停止之前通过 `start_typing` 启动的输入状态任务。
    /// 如果没有运行中的任务，该方法是空操作。
    ///
    /// # 参数
    ///
    /// - `_recipient`：目标聊天的 ID（未使用，但保持接口一致性）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：操作完成（无论是否有任务被停止）
    /// - `Err(e)`：操作失败（如锁获取失败）
    ///
    /// # 实现细节
    ///
    /// 1. 获取 `typing_handle` 的互斥锁
    /// 2. 如果存在运行中的任务句柄，取出并中止该任务
    /// 3. 清空句柄字段
    ///
    /// # 说明
    ///
    /// 即使任务被中止，Telegram 聊天中的"正在输入"状态也会在约 5 秒后自动消失。
    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        // 获取互斥锁并停止现有的输入状态任务
        let mut guard = self.typing_handle.lock();
        // 如果有运行中的任务，取出句柄并中止任务
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        Ok(())
    }
}
