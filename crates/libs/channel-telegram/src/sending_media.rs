//! # Telegram 媒体发送模块
//!
//! 本模块为 `TelegramChannel` 提供各类媒体消息的发送能力，包括：
//! - 通过本地文件或内存字节发送文档、图片、视频、音频、语音
//! - 通过远程 URL 发送上述媒体类型（由 Telegram 服务端拉取）
//!
//! 所有方法均为异步，使用 multipart 上传本地或内存数据，使用 JSON body 发送 URL 型媒体。
//!
//! ## 设计要点
//! - 统一使用 `reqwest::multipart` 构建表单；对 URL 发送则使用 JSON body
//! - 可选参数统一为 `Option`，在内部按需添加到请求体
//! - 失败时调用 `sanitize_telegram_error` 清洗错误文本，避免泄密后再报错
//! - 成功后统一记录 `tracing::info` 日志
//!
//! ## 使用约定
//! - `chat_id`：接收方聊天或频道 ID（字符串形式）
//! - `thread_id`：可选话题/主题 ID，用于超级群组的话题功能
//! - `file_path`：本地文件路径；方法内部读取并上传
//! - `file_bytes`/`file_name`：内存中的文件内容与文件名
//! - `url`：远程媒体 URL，由 Telegram 拉取并发送
//! - `caption`：可选的媒体说明文字（标题）

use super::TelegramChannel;
use reqwest::multipart::{Form, Part};
use std::path::Path;

impl TelegramChannel {
    /// 通过远程 URL 发送媒体消息的通用实现。
    ///
    /// 该方法为私有辅助方法，供各类 `send_*_by_url` 复用，构造统一的 JSON body 并发送。
    ///
    /// # 参数
    /// - `method`：Telegram Bot API 方法名（如 `"sendVideo"`）
    /// - `media_field`：请求体中媒体字段名（如 `"video"`）
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID，用于超级群组话题
    /// - `url`：远程媒体 URL
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息（已清洗敏感字段）
    pub(super) async fn send_media_by_url(
        &self,
        method: &str,
        media_field: &str,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 构造基础 JSON body，包含 chat_id 与媒体字段
        let mut body = serde_json::json!({
            "chat_id": chat_id,
        });
        body[media_field] = serde_json::Value::String(url.to_string());

        // 可选：添加话题 ID（超级群组话题功能）
        if let Some(tid) = thread_id {
            body["message_thread_id"] = serde_json::Value::String(tid.to_string());
        }

        // 可选：添加说明文字
        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }

        // 发送请求并校验响应状态
        let resp = self
            .http_client()
            .post(self.api_url(method))
            .timeout(std::time::Duration::from_secs(120))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram {method} by URL failed: {sanitized}");
        }

        // 记录成功日志
        tracing::info!("Telegram {method} sent to {chat_id}: {url}");
        Ok(())
    }

    /// 通过本地文件发送文档。
    ///
    /// 读取本地文件并通过 multipart/form-data 上传到 Telegram。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_path`：本地文档文件路径
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息（文件读取失败或 Telegram 接口错误）
    pub async fn send_document(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 提取文件名，若无法获取则使用默认值 "file"
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

        // 异步读取文件内容并构造 multipart Part
        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        // 构造基础表单，包含 chat_id 与 document 字段
        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("document", part);

        // 可选：添加话题 ID
        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        // 可选：添加说明文字
        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        // 发送 multipart 请求并校验响应
        let resp = self
            .http_client()
            .post(self.api_url("sendDocument"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendDocument failed: {sanitized}");
        }

        tracing::info!("Telegram document sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过内存字节发送文档。
    ///
    /// 将内存中的文件数据通过 multipart/form-data 上传到 Telegram，适合无需落盘的动态生成文件。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_bytes`：文件内容字节数组
    /// - `file_name`：上传时使用的文件名
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息（Telegram 接口错误）
    pub async fn send_document_bytes(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_bytes: Vec<u8>,
        file_name: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("document", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendDocument"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendDocument failed: {sanitized}");
        }

        tracing::info!("Telegram document sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过本地文件发送图片。
    ///
    /// 读取本地图片文件并通过 multipart/form-data 上传到 Telegram。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_path`：本地图片文件路径
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_photo(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 默认文件名为 "photo.jpg"
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("photo.jpg");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("photo", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendPhoto"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendPhoto failed: {sanitized}");
        }

        tracing::info!("Telegram photo sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过内存字节发送图片。
    ///
    /// 将内存中的图片数据上传到 Telegram，无需落盘。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_bytes`：图片内容字节数组
    /// - `file_name`：上传时使用的文件名
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_photo_bytes(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_bytes: Vec<u8>,
        file_name: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("photo", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendPhoto"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendPhoto failed: {sanitized}");
        }

        tracing::info!("Telegram photo sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过本地文件发送视频。
    ///
    /// 读取本地视频文件并通过 multipart/form-data 上传到 Telegram。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_path`：本地视频文件路径
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_video(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 默认文件名为 "video.mp4"
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("video.mp4");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("video", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendVideo"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendVideo failed: {sanitized}");
        }

        tracing::info!("Telegram video sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过本地文件发送音频。
    ///
    /// 读取本地音频文件并通过 multipart/form-data 上传到 Telegram。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_path`：本地音频文件路径
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_audio(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 默认文件名为 "audio.mp3"
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.mp3");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("audio", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendAudio"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendAudio failed: {sanitized}");
        }

        tracing::info!("Telegram audio sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过本地文件发送语音。
    ///
    /// 读取本地语音文件并通过 multipart/form-data 上传到 Telegram。
    /// 语音消息通常为 `.ogg` 格式，Telegram 会在客户端展示为语音消息。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `file_path`：本地语音文件路径
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_voice(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 默认文件名为 "voice.ogg"
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("voice.ogg");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new().text("chat_id", chat_id.to_string()).part("voice", part);

        if let Some(tid) = thread_id {
            form = form.text("message_thread_id", tid.to_string());
        }

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendVoice"))
            .timeout(std::time::Duration::from_secs(120))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendVoice failed: {sanitized}");
        }

        tracing::info!("Telegram voice sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// 通过远程 URL 发送文档。
    ///
    /// Telegram 服务端将从给定 URL 下载文档并发送，适用于公开可访问的文件链接。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `url`：文档文件的公开 URL
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_document_by_url(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        // 构造 JSON body，直接包含 document 字段为 URL
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "document": url
        });

        if let Some(tid) = thread_id {
            body["message_thread_id"] = serde_json::Value::String(tid.to_string());
        }

        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendDocument"))
            .timeout(std::time::Duration::from_secs(120))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendDocument by URL failed: {sanitized}");
        }

        tracing::info!("Telegram document (URL) sent to {chat_id}: {url}");
        Ok(())
    }

    /// 通过远程 URL 发送图片。
    ///
    /// Telegram 服务端将从给定 URL 下载图片并发送。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `url`：图片文件的公开 URL
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_photo_by_url(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": url
        });

        if let Some(tid) = thread_id {
            body["message_thread_id"] = serde_json::Value::String(tid.to_string());
        }

        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }

        let resp = self
            .http_client()
            .post(self.api_url("sendPhoto"))
            .timeout(std::time::Duration::from_secs(120))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            let sanitized = Self::sanitize_telegram_error(&err);
            anyhow::bail!("Telegram sendPhoto by URL failed: {sanitized}");
        }

        tracing::info!("Telegram photo (URL) sent to {chat_id}: {url}");
        Ok(())
    }

    /// 通过远程 URL 发送视频。
    ///
    /// 复用 `send_media_by_url` 发送视频，Telegram 将从 URL 下载并播放。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `url`：视频文件的公开 URL
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_video_by_url(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        self.send_media_by_url("sendVideo", "video", chat_id, thread_id, url, caption).await
    }

    /// 通过远程 URL 发送音频。
    ///
    /// 复用 `send_media_by_url` 发送音频文件。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `url`：音频文件的公开 URL
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_audio_by_url(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        self.send_media_by_url("sendAudio", "audio", chat_id, thread_id, url, caption).await
    }

    /// 通过远程 URL 发送语音。
    ///
    /// 复用 `send_media_by_url` 发送语音消息。
    ///
    /// # 参数
    /// - `chat_id`：目标聊天 ID
    /// - `thread_id`：可选话题 ID
    /// - `url`：语音文件的公开 URL（通常为 `.ogg` 格式）
    /// - `caption`：可选说明文字
    ///
    /// # 返回
    /// - 成功时返回 `Ok(())`
    /// - 失败时返回错误信息
    pub async fn send_voice_by_url(
        &self,
        chat_id: &str,
        thread_id: Option<&str>,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        self.send_media_by_url("sendVoice", "voice", chat_id, thread_id, url, caption).await
    }
}
