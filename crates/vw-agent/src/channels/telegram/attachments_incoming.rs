//! Telegram 附件接收处理模块
//!
//! 本模块提供处理 Telegram 消息中附件的相关功能，包括：
//! - 解析消息中的附件元数据（文档、图片等）
//! - 下载附件到本地工作空间
//! - 将附件转换为标准化的通道消息格式
//!
//! ## 支持的附件类型
//!
//! - **文档（Document）**：任意文件，包含文件名、大小等信息
//! - **图片（Photo）**：自动选择最高分辨率的版本
//!
//! ## 主要流程
//!
//! 1. 从 Telegram 更新中提取附件元数据
//! 2. 验证文件大小是否符合限制
//! 3. 检查用户权限和群组提及条件
//! 4. 下载文件并保存到工作空间
//! 5. 构建包含附件信息的标准消息对象

use super::TelegramChannel;
use super::attachments::{
    IncomingAttachment, IncomingAttachmentKind, TELEGRAM_MAX_FILE_DOWNLOAD_BYTES,
    format_attachment_content, resolve_workspace_attachment_output_path,
    sanitize_attachment_filename, sanitize_generated_extension,
};
use crate::app::agent::channels::traits::ChannelMessage;

impl TelegramChannel {
    /// 从 Telegram 消息中解析附件元数据
    ///
    /// 该函数检查消息内容，提取文档或图片的元数据信息。
    /// 对于图片消息，会自动选择最高分辨率的版本。
    ///
    /// # 参数
    ///
    /// * `message` - Telegram 消息的 JSON 对象，包含消息的所有字段
    ///
    /// # 返回值
    ///
    /// 如果消息包含有效附件，返回 `Some(IncomingAttachment)`，包含：
    /// - `file_id`: Telegram 文件标识符
    /// - `file_name`: 原始文件名（仅文档类型有）
    /// - `file_size`: 文件大小（字节）
    /// - `caption`: 附件说明文字
    /// - `kind`: 附件类型（Document 或 Photo）
    ///
    /// 如果消息不包含附件，返回 `None`。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let message = json!({
    ///     "document": {
    ///         "file_id": "AbCdEf123",
    ///         "file_name": "report.pdf",
    ///         "file_size": 1024
    ///     },
    ///     "caption": "月度报告"
    /// });
    ///
    /// let attachment = channel.parse_attachment_metadata(&message);
    /// assert!(attachment.is_some());
    /// ```
    pub(super) fn parse_attachment_metadata(
        message: &serde_json::Value,
    ) -> Option<IncomingAttachment> {
        // 尝试解析文档类型附件
        // 文档类型通常包含用户上传的文件，如 PDF、Word 文档等
        if let Some(doc) = message.get("document") {
            // 提取文件 ID，这是下载文件的必需标识符
            let file_id = doc.get("file_id")?.as_str()?.to_string();
            // 提取文件名，可能不存在（某些客户端上传时不提供）
            let file_name =
                doc.get("file_name").and_then(serde_json::Value::as_str).map(String::from);
            // 提取文件大小，用于后续的大小限制检查
            let file_size = doc.get("file_size").and_then(serde_json::Value::as_u64);
            // 提取附件说明文字（用户发送文件时添加的文字描述）
            let caption =
                message.get("caption").and_then(serde_json::Value::as_str).map(String::from);
            return Some(IncomingAttachment {
                file_id,
                file_name,
                file_size,
                caption,
                kind: IncomingAttachmentKind::Document,
            });
        }

        // 尝试解析图片类型附件
        // Telegram 会提供多个尺寸的图片，我们选择最大（最后）的一个
        if let Some(photos) = message.get("photo").and_then(serde_json::Value::as_array) {
            // 选择最高分辨率的图片版本（数组中最后一个）
            let best = photos.last()?;
            let file_id = best.get("file_id")?.as_str()?.to_string();
            let file_size = best.get("file_size").and_then(serde_json::Value::as_u64);
            let caption =
                message.get("caption").and_then(serde_json::Value::as_str).map(String::from);
            // 图片类型没有原始文件名，后续会根据类型生成默认名称
            return Some(IncomingAttachment {
                file_id,
                file_name: None,
                file_size,
                caption,
                kind: IncomingAttachmentKind::Photo,
            });
        }

        // 消息中不包含任何支持的附件类型
        None
    }

    /// 尝试将 Telegram 更新解析为包含附件的通道消息
    ///
    /// 该函数执行完整的附件消息处理流程，包括权限验证、文件下载、
    /// 本地保存以及消息对象构建。
    ///
    /// # 参数
    ///
    /// * `update` - Telegram Bot API 的更新对象，包含消息内容
    ///
    /// # 返回值
    ///
    /// 如果成功处理附件消息，返回 `Some(ChannelMessage)`，包含：
    /// - `id`: 唯一消息标识符（格式：`telegram_{chat_id}_{message_id}`）
    /// - `sender`: 发送者身份标识
    /// - `reply_target`: 回复目标（可能包含话题 ID）
    /// - `content`: 包含附件信息和说明文字的内容
    /// - `channel`: 固定为 "telegram"
    /// - `timestamp`: Unix 时间戳
    /// - `thread_ts`: 话题 ID（如果存在）
    ///
    /// 在以下情况返回 `None`：
    /// - 更新不包含消息或附件
    /// - 文件大小超过限制
    /// - 发送者未在允许列表中
    /// - 群组消息中未提及机器人（当 mention_only 为 true 时）
    /// - 工作空间目录未配置
    /// - 文件下载或保存失败
    ///
    /// # 错误处理
    ///
    /// 所有可能的错误都会被记录并优雅地返回 None，不会抛出异常。
    /// 这确保了单个附件处理失败不会影响其他消息的处理。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let update = json!({
    ///     "message": {
    ///         "message_id": 123,
    ///         "chat": {"id": 456},
    ///         "document": {"file_id": "AbCdEf"}
    ///     }
    /// });
    ///
    /// if let Some(msg) = channel.try_parse_attachment_message(&update).await {
    ///     println!("收到附件消息: {}", msg.content);
    /// }
    /// ```
    pub(super) async fn try_parse_attachment_message(
        &self,
        update: &serde_json::Value,
    ) -> Option<ChannelMessage> {
        // 从更新对象中提取消息内容
        let message = update.get("message")?;
        // 解析附件元数据，如果不是附件消息则直接返回
        let attachment = Self::parse_attachment_metadata(message)?;

        // 文件大小限制检查
        // 防止下载过大的文件消耗过多带宽和存储空间
        if let Some(size) = attachment.file_size {
            if size > TELEGRAM_MAX_FILE_DOWNLOAD_BYTES {
                tracing::info!(
                    "Skipping attachment: file size {size} bytes exceeds {} MB limit",
                    TELEGRAM_MAX_FILE_DOWNLOAD_BYTES / (1024 * 1024)
                );
                return None;
            }
        }

        // 提取发送者信息（用户名和 ID）
        let (username, sender_id, sender_identity) = Self::extract_sender_info(message);

        // 构建身份标识列表，用于权限检查
        let mut identities = vec![username.as_str()];
        if let Some(id) = sender_id.as_deref() {
            identities.push(id);
        }

        // 权限验证：检查发送者是否在允许列表中
        if !self.is_any_user_allowed(identities.iter().copied()) {
            return None;
        }

        // 检查是否为群组消息
        let is_group = Self::is_group_message(message);
        // 群组消息的提及过滤逻辑
        // 如果配置了 mention_only，则只处理明确提及机器人的消息
        if self.mention_only && is_group {
            let bot_username = self.bot_username.lock();
            if let Some(ref bot_username) = *bot_username {
                // 检查附件说明文字中是否包含机器人用户名
                let text_to_check = attachment.caption.as_deref().unwrap_or("");
                if !Self::contains_bot_mention(text_to_check, bot_username) {
                    return None;
                }
            } else {
                // 无法获取机器人用户名，跳过此消息
                return None;
            }
        }

        // 提取聊天 ID，这是消息的唯一标识符之一
        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string())?;

        // 提取消息 ID，用于构建唯一标识符
        let message_id = message.get("message_id").and_then(serde_json::Value::as_i64).unwrap_or(0);

        // 提取话题 ID（仅群组话题消息存在）
        let thread_id = message
            .get("message_thread_id")
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string());

        // 构建回复目标
        // 对于话题消息，格式为 "chat_id:thread_id"
        // 对于普通消息，仅使用 chat_id
        let reply_target = if let Some(ref tid) = thread_id {
            format!("{}:{}", chat_id, tid)
        } else {
            chat_id.clone()
        };

        // 重复的群组消息提及检查
        // 注意：此处代码与上方逻辑重复，可能是遗留代码
        let is_group = Self::is_group_message(message);
        if self.mention_only && is_group {
            let bot_username = self.bot_username.lock();
            if let Some(ref bot_username) = *bot_username {
                let caption_text = attachment.caption.as_deref().unwrap_or("");
                if !Self::contains_bot_mention(caption_text, bot_username) {
                    return None;
                }
            } else {
                return None;
            }
        }

        // 获取工作空间目录，用于保存下载的文件
        // 如果未配置工作空间目录，无法保存附件
        let workspace = self.workspace_dir.as_ref().or_else(|| {
            tracing::warn!("Cannot save attachment: workspace_dir not configured");
            None
        })?;

        // 通过 Telegram API 获取文件的服务器路径
        // 这是下载文件的必要步骤
        let tg_file_path = match self.get_file_path(&attachment.file_id).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to get attachment file path: {e}");
                return None;
            }
        };

        // 下载文件内容到内存
        let file_data = match self.download_file(&tg_file_path).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to download attachment: {e}");
                return None;
            }
        };

        // 生成本地文件名
        // 对于文档：使用原始文件名（经过安全清理）
        // 对于图片：生成基于聊天和消息 ID 的默认名称
        let local_filename = match &attachment.file_name {
            Some(name) => sanitize_attachment_filename(name)
                .unwrap_or_else(|| format!("attachment_{chat_id}_{message_id}.bin")),
            None => {
                // 对于图片，尝试从 Telegram 文件路径提取扩展名
                let ext =
                    sanitize_generated_extension(tg_file_path.rsplit('.').next().unwrap_or("jpg"));
                format!("photo_{chat_id}_{message_id}.{ext}")
            }
        };

        // 解析完整的本地保存路径
        // 确保路径在工作空间内，防止目录遍历攻击
        let local_path =
            match resolve_workspace_attachment_output_path(workspace, &local_filename).await {
                Ok(path) => path,
                Err(e) => {
                    tracing::warn!(
                        "Failed to resolve attachment output path for {}: {e}",
                        local_filename
                    );
                    return None;
                }
            };

        // 将文件内容写入本地磁盘
        if let Err(e) = tokio::fs::write(&local_path, &file_data).await {
            tracing::warn!("Failed to save attachment to {}: {e}", local_path.display());
            return None;
        }

        // 构建消息内容
        // 包含附件类型、文件名和本地路径的格式化信息
        let mut content = format_attachment_content(attachment.kind, &local_filename, &local_path);

        // 如果附件有说明文字，追加到消息内容中
        if let Some(caption) = &attachment.caption {
            if !caption.is_empty() {
                use std::fmt::Write;
                let _ = write!(content, "\n\n{caption}");
            }
        }

        // 如果消息是回复其他消息，提取引用上下文并添加到内容开头
        if let Some(quote) = self.extract_reply_context(message) {
            content = format!("{quote}\n\n{content}");
        }

        // 构建并返回标准化的通道消息对象
        Some(ChannelMessage {
            instance_id: "default".into(),
            // 唯一消息标识符
            id: format!("telegram_{chat_id}_{message_id}"),
            // 发送者身份
            sender: sender_identity,
            // 回复目标（可能包含话题 ID）
            reply_target,
            // 消息内容
            content,
            // 通道类型标识
            channel: "telegram".to_string(),
            // 消息时间戳（Unix 时间戳，秒）
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            // 话题 ID（用于群组话题消息）
            thread_ts: thread_id,
        })
    }
}
