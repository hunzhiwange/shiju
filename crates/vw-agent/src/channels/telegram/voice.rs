//! Telegram 语音消息处理模块
//!
//! 本模块负责处理来自 Telegram 频道的语音和音频消息，包括：
//! - 解析语音消息元数据（文件ID、时长、MIME类型等）
//! - 推断语音文件名和扩展名
//! - 下载语音文件并进行转录
//! - 将转录后的文本转换为标准的通道消息格式
//!
//! 支持的音频格式包括：
//! - FLAC、MP3、MP4/M4A、OGG、OPUS、WAV、WEBM 等

use super::TelegramChannel;
use crate::app::agent::channels::traits::ChannelMessage;
use crate::app::agent::channels::transcription;

/// 语音消息元数据结构体
///
/// 存储从 Telegram 消息中提取的语音或音频文件的元数据信息，
/// 用于后续的文件下载、格式推断和转录处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceMetadata {
    /// Telegram 服务器上的文件唯一标识符
    pub(super) file_id: String,

    /// 语音或音频的时长（秒）
    pub(super) duration_secs: u64,

    /// 原始文件名提示（可选）
    /// 可能由发送方提供，也可能为空
    pub(super) file_name_hint: Option<String>,

    /// MIME 类型提示（可选）
    /// 例如 "audio/ogg"、"audio/mpeg" 等
    pub(super) mime_type_hint: Option<String>,

    /// 是否为语音笔记（voice note）
    /// true 表示 Telegram 语音消息，false 表示普通音频文件
    pub(super) voice_note: bool,
}

impl TelegramChannel {
    /// 从 Telegram 消息 JSON 中解析语音元数据
    ///
    /// 该方法会尝试从消息中提取 `voice` 或 `audio` 字段，
    /// 并构造包含文件ID、时长、文件名和MIME类型的元数据结构。
    ///
    /// # 参数
    ///
    /// * `message` - Telegram 消息的 JSON 值对象，预期包含 `voice` 或 `audio` 字段
    ///
    /// # 返回值
    ///
    /// 如果成功解析到语音或音频信息，返回 `Some(VoiceMetadata)`；
    /// 如果消息中不包含语音或音频数据，返回 `None`。
    ///
    /// # 解析优先级
    ///
    /// 1. 优先检查 `voice` 字段（语音消息）
    /// 2. 如果不存在，则检查 `audio` 字段（音频文件）
    pub(super) fn parse_voice_metadata(message: &serde_json::Value) -> Option<VoiceMetadata> {
        // 尝试获取 voice 或 audio 字段，优先处理语音消息
        let (voice, voice_note) = if let Some(voice) = message.get("voice") {
            (voice, true)
        } else {
            (message.get("audio")?, false)
        };

        // 提取必需的字段：file_id
        let file_id = voice.get("file_id")?.as_str()?.to_string();

        // 提取可选字段：duration（默认为0）
        let duration_secs = voice.get("duration").and_then(serde_json::Value::as_u64).unwrap_or(0);

        // 提取可选字段：file_name（过滤空字符串）
        let file_name_hint = voice
            .get("file_name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .filter(|name| !name.trim().is_empty());

        // 提取可选字段：mime_type（过滤空字符串）
        let mime_type_hint = voice
            .get("mime_type")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .filter(|mime| !mime.trim().is_empty());

        Some(VoiceMetadata { file_id, duration_secs, file_name_hint, mime_type_hint, voice_note })
    }

    /// 根据 MIME 类型推断音频文件扩展名
    ///
    /// 该方法将常见的音频 MIME 类型映射到对应的文件扩展名，
    /// 用于在文件名缺失或需要补充时推断正确的文件扩展名。
    ///
    /// # 参数
    ///
    /// * `mime_type` - 音频文件的 MIME 类型字符串（不区分大小写，自动 trim）
    ///
    /// # 返回值
    ///
    /// 如果是支持的 MIME 类型，返回对应的扩展名（如 "mp3"、"ogg"）；
    /// 如果不支持的类型，返回 `None`。
    ///
    /// # 支持的 MIME 类型
    ///
    /// | MIME 类型 | 扩展名 |
    /// |-----------|--------|
    /// | audio/flac, audio/x-flac | flac |
    /// | audio/mpeg | mp3 |
    /// | audio/mp4 | mp4 |
    /// | audio/x-m4a | m4a |
    /// | audio/ogg, application/ogg | ogg |
    /// | audio/opus | opus |
    /// | audio/wav, audio/x-wav, audio/wave | wav |
    /// | audio/webm | webm |
    pub(super) fn extension_from_audio_mime_type(mime_type: &str) -> Option<&'static str> {
        // 标准化处理：去除空白并转换为小写
        match mime_type.trim().to_ascii_lowercase().as_str() {
            "audio/flac" | "audio/x-flac" => Some("flac"),
            "audio/mpeg" => Some("mp3"),
            "audio/mp4" => Some("mp4"),
            "audio/x-m4a" => Some("m4a"),
            "audio/ogg" | "application/ogg" => Some("ogg"),
            "audio/opus" => Some("opus"),
            "audio/wav" | "audio/x-wav" | "audio/wave" => Some("wav"),
            "audio/webm" => Some("webm"),
            _ => None,
        }
    }

    /// 检查文件名是否包含有效的文件扩展名
    ///
    /// 通过路径解析检查文件名是否存在非空扩展名。
    ///
    /// # 参数
    ///
    /// * `name` - 要检查的文件名或路径
    ///
    /// # 返回值
    ///
    /// 如果文件名包含有效的（非空）扩展名，返回 `true`；否则返回 `false`。
    pub(super) fn has_file_extension(name: &str) -> bool {
        std::path::Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| !ext.trim().is_empty())
    }

    /// 推断语音文件的完整文件名
    ///
    /// 该方法综合多种信息来源推断最合适的文件名：
    /// 1. 首先尝试使用文件路径中的名称（如果包含扩展名）
    /// 2. 其次尝试使用元数据中的文件名提示（如果包含扩展名）
    /// 3. 最后根据基础名称和MIME类型构造文件名
    ///
    /// # 参数
    ///
    /// * `file_path` - Telegram 服务器返回的文件路径
    /// * `metadata` - 语音消息的元数据
    ///
    /// # 返回值
    ///
    /// 返回推断的完整文件名（包含扩展名）。
    ///
    /// # 默认扩展名策略
    ///
    /// - 语音消息（voice_note = true）默认使用 `.ogg` 扩展名
    /// - 普通音频（voice_note = false）默认使用 `.mp3` 扩展名
    pub(super) fn infer_voice_filename(file_path: &str, metadata: &VoiceMetadata) -> String {
        // 尝试从文件路径中提取基础名称
        let basename = file_path.rsplit('/').next().unwrap_or("").trim();

        // 如果基础名称已包含扩展名，直接使用
        if !basename.is_empty() && Self::has_file_extension(basename) {
            return basename.to_string();
        }

        // 检查元数据中的文件名提示是否包含扩展名
        if let Some(hint) =
            metadata.file_name_hint.as_deref().map(str::trim).filter(|name| !name.is_empty())
        {
            if Self::has_file_extension(hint) {
                return hint.to_string();
            }
        }

        // 确定默认的基础名称前缀
        let default_stem = if metadata.voice_note { "voice" } else { "audio" };

        // 确定最终的基础名称（移除末尾的点）
        let stem = if basename.is_empty() {
            metadata
                .file_name_hint
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(default_stem)
        } else {
            basename
        }
        .trim_end_matches('.');

        // 尝试根据 MIME 类型推断扩展名
        if let Some(extension) =
            metadata.mime_type_hint.as_deref().and_then(Self::extension_from_audio_mime_type)
        {
            return format!("{stem}.{extension}");
        }

        // 使用默认扩展名
        if metadata.voice_note { format!("{stem}.ogg") } else { format!("{stem}.mp3") }
    }

    /// 尝试解析 Telegram 更新中的语音消息并转录
    ///
    /// 该方法是处理语音消息的主入口，负责：
    /// 1. 检查转录配置是否启用
    /// 2. 解析语音消息元数据
    /// 3. 验证消息时长限制
    /// 4. 检查用户权限
    /// 5. 下载语音文件
    /// 6. 调用转录服务
    /// 7. 缓存转录结果
    /// 8. 构造标准消息格式
    ///
    /// # 参数
    ///
    /// * `update` - Telegram 更新对象的 JSON 值
    ///
    /// # 返回值
    ///
    /// 如果成功处理并转录语音消息，返回 `Some(ChannelMessage)`；
    /// 如果不满足处理条件或处理失败，返回 `None`。
    ///
    /// # 处理条件
    ///
    /// - 转录功能必须启用（transcription 配置存在）
    /// - 消息必须包含 `voice` 或 `audio` 字段
    /// - 时长不能超过配置的最大限制
    /// - 发送用户必须在允许列表中（如果配置了白名单）
    /// - 群组消息需要满足提及条件（如果配置了 mention_only）
    ///
    /// # 缓存机制
    ///
    /// 转录结果会被缓存，最多保留100条记录，超过后清空缓存。
    pub(super) async fn try_parse_voice_message(
        &self,
        update: &serde_json::Value,
    ) -> Option<ChannelMessage> {
        // 检查转录配置是否启用
        let config = match self.transcription.as_ref() {
            Some(c) => c,
            None => {
                // 如果收到语音消息但转录未启用，记录调试日志
                if let Some(message) = update.get("message") {
                    if message.get("voice").is_some() || message.get("audio").is_some() {
                        tracing::debug!(
                            "Received voice/audio message but transcription is disabled. \
                             Set [transcription].enabled = true to enable voice transcription."
                        );
                    }
                }
                return None;
            }
        };

        // 获取消息对象
        let message = update.get("message")?;

        // 解析语音元数据
        let metadata = Self::parse_voice_metadata(message)?;

        // 检查时长限制
        if metadata.duration_secs > config.max_duration_secs {
            tracing::info!(
                "Skipping voice message: duration {}s exceeds limit {}s",
                metadata.duration_secs,
                config.max_duration_secs
            );
            return None;
        }

        // 提取发送者信息
        let (username, sender_id, sender_identity) = Self::extract_sender_info(message);

        // 构建身份标识列表用于权限检查
        let mut identities = vec![username.as_str()];
        if let Some(id) = sender_id.as_deref() {
            identities.push(id);
        }

        // 验证用户权限
        if !self.is_any_user_allowed(identities.iter().copied()) {
            let allowed_users = self
                .allowed_users
                .read()
                .map(|u| u.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            tracing::debug!(?allowed_users, %sender_identity, "Skipping voice message from unauthorized user");
            return None;
        }

        // 检查群组消息的提及条件
        let is_group = Self::is_group_message(message);
        let allow_sender_without_mention =
            is_group && self.is_group_sender_trigger_enabled(sender_id.as_deref());
        if self.mention_only && is_group && !allow_sender_without_mention {
            return None;
        }

        // 提取聊天 ID
        let chat_id = message
            .get("chat")
            .and_then(|chat| chat.get("id"))
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string())?;

        // 提取消息 ID
        let message_id = message.get("message_id").and_then(serde_json::Value::as_i64).unwrap_or(0);

        // 提取话题线程 ID（如果存在）
        let thread_id = message
            .get("message_thread_id")
            .and_then(serde_json::Value::as_i64)
            .map(|id| id.to_string());

        // 构建回复目标（话题消息需要包含 thread_id）
        let reply_target = if let Some(ref tid) = thread_id {
            format!("{}:{}", chat_id, tid)
        } else {
            chat_id.clone()
        };

        // 获取文件路径
        let file_path = match self.get_file_path(&metadata.file_id).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to get voice file path: {e}");
                return None;
            }
        };

        // 推断文件名
        let file_name = Self::infer_voice_filename(&file_path, &metadata);

        // 下载语音文件
        let audio_data = match self.download_file(&file_path).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to download voice file: {e}");
                return None;
            }
        };

        // 调用转录服务
        let text: String =
            match transcription::transcribe_audio(audio_data, &file_name, config).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("Voice transcription failed: {e}");
                    return None;
                }
            };

        // 检查转录结果是否为空
        if text.trim().is_empty() {
            tracing::info!("Voice transcription returned empty text, skipping");
            return None;
        }

        // 缓存转录结果（限制缓存大小为100）
        {
            let mut cache = self.voice_transcriptions.lock();
            if cache.len() >= 100 {
                cache.clear();
            }
            cache.insert(format!("{chat_id}:{message_id}"), text.clone());
        }

        // 记录成功日志
        let text_len = text.len();
        tracing::info!(text_len, %sender_identity, %chat_id, "Voice message transcribed successfully");

        // 构建消息内容（包含可能的引用上下文）
        let content = if let Some(quote) = self.extract_reply_context(message) {
            format!("{quote}\n\n[Voice] {text}")
        } else {
            format!("[Voice] {text}")
        };

        // 返回标准的通道消息
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
