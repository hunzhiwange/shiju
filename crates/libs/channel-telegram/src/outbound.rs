//!
//! Telegram 出站消息处理模块
//!
//! 本模块负责处理 Telegram 频道的出站消息发送逻辑，包括：
//! - 附件消息的发送（图片、文档、视频、音频、语音）
//! - 文本消息的发送
//! - URL 形式和本地文件形式附件的区分处理
//!
//! ## 核心功能
//!
//! 1. **附件发送** (`send_attachment`)：根据附件类型和目标地址（URL 或本地路径），
//!    选择合适的方式发送媒体文件到 Telegram。
//!
//! 2. **出站消息处理** (`send_outbound`)：处理通用的出站消息，解析消息内容中的
//!    附件标记，并按顺序发送文本和附件。
//!

use super::TelegramChannel;
use super::attachments::{
    TelegramAttachment, TelegramAttachmentKind, is_http_url, parse_attachment_markers,
    parse_path_only_attachment, resolve_workspace_attachment_path,
};
use channel_types::SendMessage;

impl TelegramChannel {
    ///
    /// 发送附件到 Telegram 聊天
    ///
    /// 根据附件的目标地址（URL 或本地文件路径）和附件类型，
    /// 选择合适的 Telegram API 方法发送媒体内容。
    ///
    /// # 参数
    ///
    /// - `chat_id`: Telegram 聊天 ID，标识消息发送的目标聊天
    /// - `thread_id`: 可选的主题/话题 ID，用于在超级群组的特定话题中发送消息
    /// - `attachment`: 要发送的附件信息，包含类型和目标地址
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误信息
    ///
    /// # 处理逻辑
    ///
    /// 1. **URL 附件**：如果目标是 HTTP/HTTPS URL，直接使用 Telegram 的 URL 发送 API
    ///    - 发送失败时会回退为发送文本链接
    /// 2. **本地文件附件**：如果目标是本地文件路径
    ///    - 需要配置 `workspace_dir`
    ///    - 将相对路径解析为绝对路径后上传文件
    ///
    /// # 错误
    ///
    /// - 当发送本地文件但未配置 `workspace_dir` 时返回错误
    /// - 当文件路径解析失败或文件上传失败时返回错误
    ///
    pub(super) async fn send_attachment(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        attachment: &TelegramAttachment,
    ) -> anyhow::Result<()> {
        let target = attachment.target.trim();

        // 检查目标是否为 HTTP URL，优先使用 URL 方式发送
        if is_http_url(target) {
            // 根据附件类型选择对应的 URL 发送方法
            let result = match attachment.kind {
                TelegramAttachmentKind::Image => {
                    self.send_photo_by_url(chat_id, thread_id, target, None).await
                }
                TelegramAttachmentKind::Document => {
                    self.send_document_by_url(chat_id, thread_id, target, None).await
                }
                TelegramAttachmentKind::Video => {
                    self.send_video_by_url(chat_id, thread_id, target, None).await
                }
                TelegramAttachmentKind::Audio => {
                    self.send_audio_by_url(chat_id, thread_id, target, None).await
                }
                TelegramAttachmentKind::Voice => {
                    self.send_voice_by_url(chat_id, thread_id, target, None).await
                }
            };

            // URL 发送失败时，回退为发送文本链接
            if let Err(e) = result {
                tracing::warn!(
                    url = target,
                    error = %e,
                    "Telegram send media by URL failed; falling back to text link"
                );
                // 获取附件类型标签用于回退文本
                let kind_label = match attachment.kind {
                    TelegramAttachmentKind::Image => "Image",
                    TelegramAttachmentKind::Document => "Document",
                    TelegramAttachmentKind::Video => "Video",
                    TelegramAttachmentKind::Audio => "Audio",
                    TelegramAttachmentKind::Voice => "Voice",
                };
                // 构造回退文本并发送
                let fallback_text = format!("{kind_label}: {target}");
                self.send_text_chunks(&fallback_text, chat_id, thread_id).await?;
            }

            return Ok(());
        }

        // 处理本地文件附件：需要工作区目录配置
        let workspace = self.workspace_dir.as_ref().ok_or_else(|| {
            anyhow::anyhow!("workspace_dir is not configured; local file attachments are disabled")
        })?;
        // 将目标路径解析为工作区内的绝对路径
        let path = resolve_workspace_attachment_path(workspace, target)?;

        // 根据附件类型选择对应的文件上传方法
        match attachment.kind {
            TelegramAttachmentKind::Image => self.send_photo(chat_id, thread_id, &path, None).await,
            TelegramAttachmentKind::Document => {
                self.send_document(chat_id, thread_id, &path, None).await
            }
            TelegramAttachmentKind::Video => self.send_video(chat_id, thread_id, &path, None).await,
            TelegramAttachmentKind::Audio => self.send_audio(chat_id, thread_id, &path, None).await,
            TelegramAttachmentKind::Voice => self.send_voice(chat_id, thread_id, &path, None).await,
        }
    }

    ///
    /// 处理出站消息并发送到 Telegram
    ///
    /// 解析消息内容，提取文本和附件信息，并按正确顺序发送到目标聊天。
    /// 这是 `SendMessage` trait 实现的核心处理逻辑。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息对象，包含内容和接收者信息
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误信息
    ///
    /// # 处理流程
    ///
    /// 1. **清理工具调用标签**：从消息内容中移除工具调用相关的标签
    /// 2. **解析接收者**：从 `recipient` 中提取 `chat_id` 和可选的 `thread_id`
    ///    - 格式：`chat_id:thread_id` 或单独的 `chat_id`
    /// 3. **解析附件标记**：从内容中提取附件标记（如 `[image:path]`）
    /// 4. **发送消息**：
    ///    - 如果存在附件：先发送文本内容，再依次发送各附件
    ///    - 如果内容是纯路径：尝试作为附件发送
    ///    - 否则：作为纯文本发送
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 发送到普通聊天
    /// let message = SendMessage {
    ///     recipient: "123456789".to_string(),
    ///     content: "Hello [image:photo.jpg]".to_string(),
    /// };
    /// channel.send_outbound(&message).await?;
    ///
    /// // 发送到群组话题
    /// let message = SendMessage {
    ///     recipient: "-100123456789:42".to_string(),
    ///     content: "Topic message".to_string(),
    /// };
    /// channel.send_outbound(&message).await?;
    /// ```
    ///
    pub(super) async fn send_outbound(&self, message: &SendMessage) -> anyhow::Result<()> {
        // 移除消息内容中的工具调用标签
        let content = super::tool_tags::strip_tool_call_tags(&message.content);

        // 解析接收者：格式为 "chat_id:thread_id" 或单独的 "chat_id"
        let (chat_id, thread_id) = match message.recipient.split_once(':') {
            Some((chat, thread)) => (chat, Some(thread)),
            None => (message.recipient.as_str(), None),
        };

        // 解析内容中的附件标记
        let (text_without_markers, attachments) = parse_attachment_markers(&content);

        // 如果存在附件标记，先发送文本再发送附件
        if !attachments.is_empty() {
            // 发送去除附件标记后的文本内容（如果有）
            if !text_without_markers.is_empty() {
                self.send_text_chunks(&text_without_markers, chat_id, thread_id).await?;
            }

            // 依次发送每个附件
            for attachment in &attachments {
                self.send_attachment(chat_id, thread_id, attachment).await?;
            }

            return Ok(());
        }

        // 如果没有附件标记，检查内容是否为纯路径形式的附件
        if let Some(attachment) = parse_path_only_attachment(&content) {
            self.send_attachment(chat_id, thread_id, &attachment).await?;
            return Ok(());
        }

        // 默认作为纯文本消息发送
        self.send_text_chunks(&content, chat_id, thread_id).await
    }
}
