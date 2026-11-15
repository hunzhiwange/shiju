//! Telegram 渠道草稿消息功能实现
//!
//! 本模块为 TelegramChannel 提供草稿消息的创建、更新、定稿和取消功能。
//! 草稿消息机制允许在流式响应模式下实时展示正在生成的消息内容，
//! 提供更好的用户体验。
//!
//! # 主要功能
//!
//! - **发送草稿**: 创建初始草稿消息，用于后续实时更新
//! - **更新草稿**: 按频率限制实时更新草稿内容
//! - **定稿草稿**: 将草稿转换为最终消息，处理附件和格式转换
//! - **取消草稿**: 删除未完成的草稿消息
//!
//! # 流式模式
//!
//! 草稿功能仅在流式模式启用时生效（`StreamMode::On`）。
//! 在非流式模式下，相关方法会直接返回 `None`。

use super::TelegramChannel;
use super::message_utils::TELEGRAM_MAX_MESSAGE_LENGTH;
use channel_types::SendMessage;

impl TelegramChannel {
    /// 发送草稿消息的内部实现
    ///
    /// 在流式模式下创建一条初始草稿消息，该消息的 ID 将用于后续的更新和最终定稿。
    /// 如果流式模式未启用，则直接返回 `None`。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息对象，包含收件人和内容
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(message_id))`: 成功创建草稿，返回 Telegram 消息 ID
    /// - `Ok(None)`: 流式模式未启用或无需创建草稿
    /// - `Err(e)`: 发送失败，返回错误信息
    ///
    /// # 错误处理
    ///
    /// 当 Telegram API 返回非成功状态码时，会自动净化错误信息并返回错误。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let message = SendMessage {
    ///     recipient: "chat_123".to_string(),
    ///     content: "正在生成...".to_string(),
    ///     ..Default::default()
    /// };
    /// let msg_id = channel.send_draft_impl(&message).await?;
    /// ```
    pub(super) async fn send_draft_impl(
        &self,
        message: &SendMessage,
    ) -> anyhow::Result<Option<String>> {
        // 检查流式模式是否启用，未启用则直接返回
        if self.stream_mode == crate::app::agent::config::StreamMode::Off {
            return Ok(None);
        }

        // 解析收件人信息，提取 chat_id 和 thread_id（话题/线程ID）
        let (chat_id, thread_id) = Self::parse_reply_target(&message.recipient);

        // 构建初始文本内容，空内容时显示省略号
        let initial_text =
            if message.content.is_empty() { "...".to_string() } else { message.content.clone() };

        // 构建请求体，包含基本的 chat_id 和文本内容
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": initial_text,
        });

        // 如果存在话题ID，添加到请求体中（用于 Telegram 群组话题功能）
        if let Some(tid) = thread_id {
            body["message_thread_id"] = serde_json::Value::String(tid.to_string());
        }

        // 发送创建消息请求到 Telegram API
        let resp = self.client.post(self.api_url("sendMessage")).json(&body).send().await?;

        // 检查响应状态，失败时返回净化后的错误信息
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendMessage (draft) failed: {sanitized}");
        }

        // 从响应中提取消息 ID
        let resp_json: serde_json::Value = resp.json().await?;
        let message_id = resp_json
            .get("result")
            .and_then(|r| r.get("message_id"))
            .and_then(|id| id.as_i64())
            .map(|id| id.to_string());

        // 记录最后一次编辑时间，用于更新频率控制
        self.last_draft_edit.lock().insert(chat_id.to_string(), std::time::Instant::now());

        Ok(message_id)
    }

    /// 更新草稿消息的内部实现
    ///
    /// 按照配置的更新频率限制，更新已存在的草稿消息内容。
    /// 该方法实现了频率控制机制，避免过于频繁地调用 Telegram API。
    ///
    /// # 参数
    ///
    /// - `recipient`: 消息接收者（包含 chat_id 和可选的 thread_id）
    /// - `message_id`: 要更新的草稿消息 ID
    /// - `text`: 新的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(_))`: 更新成功（当前实现总是返回 None）
    /// - `Ok(None)`: 更新被跳过（频率限制、消息ID无效等）
    /// - `Err(e)`: 更新过程中发生错误
    ///
    /// # 频率控制
    ///
    /// 通过 `draft_update_interval_ms` 配置项控制最小更新间隔。
    /// 如果距离上次更新时间不足该间隔，本次更新将被跳过。
    ///
    /// # 文本截断
    ///
    /// 当文本长度超过 Telegram 最大消息长度限制时，
    /// 会按 UTF-8 字符边界安全截断，避免截断多字节字符。
    pub(super) async fn update_draft_impl(
        &self,
        recipient: &str,
        message_id: &str,
        text: &str,
    ) -> anyhow::Result<Option<String>> {
        let (chat_id, _) = Self::parse_reply_target(recipient);

        // 检查频率限制：如果距离上次更新时间过短，跳过本次更新
        {
            let last_edits = self.last_draft_edit.lock();
            if let Some(last_time) = last_edits.get(&chat_id) {
                let elapsed = u64::try_from(last_time.elapsed().as_millis()).unwrap_or(u64::MAX);
                if elapsed < self.draft_update_interval_ms {
                    return Ok(None);
                }
            }
        }

        // 处理文本长度超限：按 UTF-8 字符边界安全截断
        let display_text = if text.len() > TELEGRAM_MAX_MESSAGE_LENGTH {
            let mut end = 0;
            // 遍历字符索引，找到最后一个不超过长度限制的字符边界
            for (idx, ch) in text.char_indices() {
                let next = idx + ch.len_utf8();
                if next > TELEGRAM_MAX_MESSAGE_LENGTH {
                    break;
                }
                end = next;
            }
            &text[..end]
        } else {
            text
        };

        // 解析消息 ID，无效时记录警告并跳过更新
        let message_id_parsed = match message_id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("Invalid Telegram message_id '{message_id}': {e}");
                return Ok(None);
            }
        };

        // 构建编辑消息的请求体
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id_parsed,
            "text": display_text,
        });

        // 发送编辑请求到 Telegram API
        let resp = self.client.post(self.api_url("editMessageText")).json(&body).send().await?;

        // 更新成功：记录本次更新时间
        if resp.status().is_success() {
            self.last_draft_edit.lock().insert(chat_id.clone(), std::time::Instant::now());
        } else {
            // 更新失败：记录调试日志（不中断流程，因为草稿更新失败不致命）
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            let sanitized = Self::sanitize_telegram_error(&err);
            tracing::debug!("Telegram editMessageText failed ({status}): {sanitized}");
        }

        Ok(None)
    }

    /// 定稿草稿消息的内部实现
    ///
    /// 将草稿消息转换为最终形式，处理以下任务：
    /// 1. 移除工具调用标签（tool call tags）
    /// 2. 解析和处理附件标记
    /// 3. 将 Markdown 转换为 Telegram HTML 格式
    /// 4. 处理长消息的分块发送
    ///
    /// # 参数
    ///
    /// - `recipient`: 消息接收者（包含 chat_id 和可选的 thread_id）
    /// - `message_id`: 草稿消息的 ID
    /// - `text`: 最终的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 定稿成功
    /// - `Err(e)`: 定稿过程中发生错误
    ///
    /// # 处理逻辑
    ///
    /// 1. **包含附件**: 删除草稿消息，重新发送文本和附件
    /// 2. **文本超长**: 删除草稿消息，分块发送完整文本
    /// 3. **正常情况**: 尝试编辑草稿消息，失败时降级为发送新消息
    ///
    /// # 降级策略
    ///
    /// 当使用 HTML 格式编辑失败时，会尝试纯文本格式。
    /// 如果所有编辑尝试都失败，最终会降级为发送新消息。
    pub(super) async fn finalize_draft_impl(
        &self,
        recipient: &str,
        message_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        // 移除工具调用标签（如 <tool_call_start> 等内部标记）
        let text = &super::tool_tags::strip_tool_call_tags(text);

        // 解析收件人信息
        let (chat_id, thread_id) = Self::parse_reply_target(recipient);

        // 清除该聊天的时间记录（草稿即将定稿）
        self.last_draft_edit.lock().remove(&chat_id);

        // 解析附件标记，分离纯文本和附件列表
        let (text_without_markers, attachments) =
            super::attachments::parse_attachment_markers(text);

        // 解析消息 ID
        let msg_id = match message_id.parse::<i64>() {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!("Invalid Telegram message_id '{message_id}': {e}");
                None
            }
        };

        // 情况1: 存在附件 - 需要删除草稿并重新发送
        if !attachments.is_empty() {
            // 删除原有的草稿消息
            if let Some(id) = msg_id {
                let _ = self
                    .client
                    .post(self.api_url("deleteMessage"))
                    .json(&serde_json::json!({
                        "chat_id": chat_id,
                        "message_id": id,
                    }))
                    .send()
                    .await;
            }

            // 发送文本内容（如果有）
            if !text_without_markers.is_empty() {
                self.send_text_chunks(&text_without_markers, &chat_id, thread_id.as_deref())
                    .await?;
            }

            // 逐个发送附件
            for attachment in &attachments {
                self.send_attachment(&chat_id, thread_id.as_deref(), attachment).await?;
            }

            return Ok(());
        }

        // 情况2: 文本超长 - 删除草稿并分块发送
        if text.len() > TELEGRAM_MAX_MESSAGE_LENGTH {
            if let Some(id) = msg_id {
                let _ = self
                    .client
                    .post(self.api_url("deleteMessage"))
                    .json(&serde_json::json!({
                        "chat_id": chat_id,
                        "message_id": id,
                    }))
                    .send()
                    .await;
            }

            return self.send_text_chunks(text, &chat_id, thread_id.as_deref()).await;
        }

        // 情况3: 消息ID无效 - 直接发送新消息
        let Some(id) = msg_id else {
            return self.send_text_chunks(text, &chat_id, thread_id.as_deref()).await;
        };

        // 情况4: 尝试编辑草稿消息为最终版本（使用 HTML 格式）
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": id,
            "text": Self::markdown_to_telegram_html(text),
            "parse_mode": "HTML",
        });

        let resp = self.client.post(self.api_url("editMessageText")).json(&body).send().await?;

        // HTML 格式编辑成功
        if resp.status().is_success() {
            return Ok(());
        }

        // HTML 格式失败，降级为纯文本格式再试一次
        let plain_body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": id,
            "text": text,
        });

        let resp =
            self.client.post(self.api_url("editMessageText")).json(&plain_body).send().await?;

        // 纯文本格式成功
        if resp.status().is_success() {
            return Ok(());
        }

        // 所有编辑尝试都失败，降级为发送新消息
        tracing::warn!("Telegram finalize_draft edit failed; falling back to sendMessage");
        self.send_text_chunks(text, &chat_id, thread_id.as_deref()).await
    }

    /// 取消草稿消息的内部实现
    ///
    /// 删除指定的草稿消息，清理相关的状态记录。
    /// 通常在流式生成被中断或取消时调用。
    ///
    /// # 参数
    ///
    /// - `recipient`: 消息接收者（包含 chat_id）
    /// - `message_id`: 要取消的草稿消息 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 取消成功或消息 ID 无效（静默处理）
    /// - `Err(e)`: 删除过程中发生严重错误
    ///
    /// # 错误处理策略
    ///
    /// - 消息 ID 解析失败：静默处理，返回 `Ok(())`
    /// - 删除 API 调用失败：记录调试日志，仍返回 `Ok(())`
    ///
    /// 这种宽松的错误处理策略是因为取消草稿是非关键操作，
    /// 即使失败也不应影响主流程的执行。
    pub(super) async fn cancel_draft_impl(
        &self,
        recipient: &str,
        message_id: &str,
    ) -> anyhow::Result<()> {
        // 解析收件人信息
        let (chat_id, _) = Self::parse_reply_target(recipient);

        // 清除该聊天的草稿编辑时间记录
        self.last_draft_edit.lock().remove(&chat_id);

        // 解析消息 ID，无效时静默返回（非关键错误）
        let message_id = match message_id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                tracing::debug!("Invalid Telegram draft message_id '{message_id}': {e}");
                return Ok(());
            }
        };

        // 调用 Telegram API 删除消息
        let response = self
            .client
            .post(self.api_url("deleteMessage"))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id,
            }))
            .send()
            .await?;

        // 删除失败时记录调试日志（不中断流程）
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let sanitized = Self::sanitize_telegram_error(&body);
            tracing::debug!("Telegram deleteMessage failed ({status}): {sanitized}");
        }

        Ok(())
    }
}
