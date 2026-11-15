//! Telegram 文本发送模块。
//!
//! 本模块负责把长文本拆分为 Telegram API 可接受的片段，并优先以 HTML
//! parse mode 发送。若格式化发送失败，会回退到纯文本发送，确保内容尽量送达，
//! 同时对失败响应脱敏后再返回错误。

use super::TelegramChannel;
use super::message_utils::split_message_for_telegram;
use std::time::Duration;

impl TelegramChannel {
    /// 按 Telegram 长度限制分片发送文本消息。
    ///
    /// # 参数
    /// - `message`: 原始消息文本。
    /// - `chat_id`: 目标 Telegram 聊天 ID。
    /// - `thread_id`: 可选的话题 ID，用于超级群组话题。
    ///
    /// # 返回值
    /// 所有片段发送成功时返回 `Ok(())`。
    ///
    /// # 错误
    /// HTML 发送失败后会尝试纯文本回退；如果纯文本也失败，则返回包含脱敏响应的错误。
    pub(super) async fn send_text_chunks(
        &self,
        message: &str,
        chat_id: &str,
        thread_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let chunks = split_message_for_telegram(message);

        for (index, chunk) in chunks.iter().enumerate() {
            let text = if chunks.len() > 1 {
                // 给分片加连续性提示，避免用户在 Telegram 中把中间片段误认为完整回复。
                if index == 0 {
                    format!("{chunk}\n\n(continues...)")
                } else if index == chunks.len() - 1 {
                    format!("(continued)\n\n{chunk}")
                } else {
                    format!("(continued)\n\n{chunk}\n\n(continues...)")
                }
            } else {
                chunk.to_string()
            };

            let mut markdown_body = serde_json::json!({
                "chat_id": chat_id,
                "text": Self::markdown_to_telegram_html(&text),
                "parse_mode": "HTML"
            });

            if let Some(tid) = thread_id {
                markdown_body["message_thread_id"] = serde_json::Value::String(tid.to_string());
            }

            let markdown_resp = self
                .http_client()
                .post(self.api_url("sendMessage"))
                .json(&markdown_body)
                .send()
                .await?;

            if markdown_resp.status().is_success() {
                if index < chunks.len() - 1 {
                    // 轻微节流可降低连续长消息触发 Telegram 速率限制的概率。
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                continue;
            }

            let markdown_status = markdown_resp.status();
            let markdown_err = markdown_resp.text().await.unwrap_or_default();
            tracing::warn!(
                status = ?markdown_status,
                "Telegram sendMessage with Markdown failed; retrying without parse_mode"
            );

            // HTML parse mode 失败通常来自 Telegram 解析限制，纯文本回退能保留内容本身。
            let mut plain_body = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            });

            if let Some(tid) = thread_id {
                plain_body["message_thread_id"] = serde_json::Value::String(tid.to_string());
            }
            let plain_resp = self
                .http_client()
                .post(self.api_url("sendMessage"))
                .json(&plain_body)
                .send()
                .await?;

            if !plain_resp.status().is_success() {
                let plain_status = plain_resp.status();
                let plain_err = plain_resp.text().await.unwrap_or_default();
                let sanitized_markdown_err = Self::sanitize_telegram_error(&markdown_err);
                let sanitized_plain_err = Self::sanitize_telegram_error(&plain_err);
                anyhow::bail!(
                    "Telegram sendMessage failed (markdown {}: {}; plain {}: {})",
                    markdown_status,
                    sanitized_markdown_err,
                    plain_status,
                    sanitized_plain_err
                );
            }

            if index < chunks.len() - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }
}
