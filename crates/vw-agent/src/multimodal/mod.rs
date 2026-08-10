//! 多模态消息预处理与图片引用归一化。
//!
//! 该模块负责从用户消息中识别 `[IMAGE:...]` 标记，并在进入具体模型提供商前把本地文件、
//! 远程图片或 data URI 统一为受限的 base64 data URI。所有路径都会执行数量、大小和 MIME
//! 白名单校验，避免在模型请求阶段隐式扩大文件读取或网络下载能力。

use crate::app::agent::config::{MultimodalConfig, build_runtime_proxy_client_with_timeouts};
use crate::app::agent::providers::ChatMessage;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;
use std::path::Path;

pub mod analysis;
pub mod assets;
pub mod context;
mod image_compression;
pub mod ocr;

use image_compression::{compress_image_bytes, per_image_output_budget};

const IMAGE_MARKER_PREFIX: &str = "[IMAGE:";
/// Internal gateway option used by the OCR model request to avoid recursively
/// preprocessing the image it is already sending to the vision model.
pub(crate) const DISABLE_OCR_OPTION: &str = "disable_multimodal_ocr";
const ALLOWED_IMAGE_MIME_TYPES: &[&str] =
    &["image/png", "image/jpeg", "image/webp", "image/gif", "image/bmp"];

/// 发送给模型提供商前的消息集合。
///
/// `messages` 保留原有聊天顺序；当输入中存在图片标记时，用户消息内容会被改写为文本加归一化
/// 图片标记。`contains_images` 让调用方可以选择支持图片的提供商路径。
#[derive(Debug, Clone)]
pub struct PreparedMessages {
    /// 已归一化的聊天消息列表。
    pub messages: Vec<ChatMessage>,
    /// 是否至少包含一个有效图片引用。
    pub contains_images: bool,
}

/// 多模态预处理阶段可能返回的显式错误。
///
/// 这些错误在真正调用模型前发生，用于把不支持或不安全的输入阻断在本地边界内。
#[derive(Debug, thiserror::Error)]
pub enum MultimodalError {
    /// 图片数量超过配置上限。
    #[error("multimodal image limit exceeded: max_images={max_images}, found={found}")]
    TooManyImages { max_images: usize, found: usize },

    /// 图片字节数超过配置上限。
    #[error(
        "multimodal image size limit exceeded for '{input}': {size_bytes} bytes > {max_bytes} bytes"
    )]
    ImageTooLarge { input: String, size_bytes: usize, max_bytes: usize },

    /// 图片 MIME 类型不在允许列表中。
    #[error("multimodal image MIME type is not allowed for '{input}': {mime}")]
    UnsupportedMime { input: String, mime: String },

    /// 配置禁止下载远程图片。
    #[error("multimodal remote image fetch is disabled for '{input}'")]
    RemoteFetchDisabled { input: String },

    /// 本地图片路径不存在或不是普通文件。
    #[error("multimodal image source not found or unreadable: '{input}'")]
    ImageSourceNotFound { input: String },

    /// 图片标记语法或 data URI 格式无效。
    #[error("invalid multimodal image marker '{input}': {reason}")]
    InvalidMarker { input: String, reason: String },

    /// 远程图片下载失败。
    #[error("failed to download remote image '{input}': {reason}")]
    RemoteFetchFailed { input: String, reason: String },

    /// 本地图片读取失败。
    #[error("failed to read local image '{input}': {reason}")]
    LocalReadFailed { input: String, reason: String },

    /// OCR 模型引用缺失或不是完整的 `provider/model`。
    #[error("OCR model reference must use a non-empty provider/model value")]
    InvalidOcrModelReference,

    /// OCR 模型目录查询失败。
    #[error("OCR model is unavailable: '{model}'")]
    OcrModelUnavailable { model: String },

    /// OCR 模型不支持图片输入。
    #[error("OCR model does not support image input: '{model}'")]
    OcrModelDoesNotSupportImages { model: String },

    /// OCR 模型返回了空文本。
    #[error("OCR model returned empty text: '{model}'")]
    EmptyOcrOutput { model: String },

    /// OCR 模型调用失败。上游错误不进入消息，避免泄露图片或 OCR 原文。
    #[error("OCR extraction failed for model: '{model}'")]
    OcrExtractionFailed { model: String },

    /// 图片无法在请求输出预算内完成压缩，或 GIF 超出预算且不支持重编码。
    #[error("multimodal image compression failed for '{input}': {reason}")]
    CompressionFailed { input: String, reason: String },
}

/// 从消息正文中提取图片标记并返回清理后的文本。
///
/// # 参数
///
/// - `content`: 可能包含 `[IMAGE:...]` 标记的原始消息正文。
///
/// # 返回值
///
/// 返回 `(cleaned_text, image_refs)`：`cleaned_text` 是移除有效图片标记后的文本，
/// `image_refs` 是按出现顺序提取出的图片引用。
///
/// # 错误处理
///
/// 本函数不返回错误；无法闭合或空内容的标记会保留在文本里，由后续正常文本路径处理。
pub fn parse_image_markers(content: &str) -> (String, Vec<String>) {
    let mut refs = Vec::new();
    let mut cleaned = String::with_capacity(content.len());
    let mut cursor = 0usize;

    while let Some(rel_start) = content[cursor..].find(IMAGE_MARKER_PREFIX) {
        let start = cursor + rel_start;
        cleaned.push_str(&content[cursor..start]);

        let marker_start = start + IMAGE_MARKER_PREFIX.len();
        let Some(rel_end) = content[marker_start..].find(']') else {
            cleaned.push_str(&content[start..]);
            cursor = content.len();
            break;
        };

        let end = marker_start + rel_end;
        let candidate = content[marker_start..end].trim();

        if candidate.is_empty() {
            cleaned.push_str(&content[start..=end]);
        } else {
            // 空标记保留为正文，非空标记才进入图片管线，避免把用户普通文本误删。
            refs.push(candidate.to_string());
        }

        cursor = end + 1;
    }

    if cursor < content.len() {
        cleaned.push_str(&content[cursor..]);
    }

    (cleaned.trim().to_string(), refs)
}

/// 统计用户消息中的图片标记数量。
///
/// # 参数
///
/// - `messages`: 待检查的聊天消息。
///
/// # 返回值
///
/// 返回所有 `role == "user"` 消息中有效图片标记的数量。
pub fn count_image_markers(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .filter(|m| m.role == "user")
        .map(|m| parse_image_markers(&m.content).1.len())
        .sum()
}

/// 判断消息集合中是否包含图片标记。
///
/// # 参数
///
/// - `messages`: 待检查的聊天消息。
///
/// # 返回值
///
/// 存在至少一个用户图片标记时返回 `true`。
pub fn contains_image_markers(messages: &[ChatMessage]) -> bool {
    count_image_markers(messages) > 0
}

/// 提取 Ollama 图片字段可接受的 payload。
///
/// # 参数
///
/// - `image_ref`: data URI 或已归一化的图片引用。
///
/// # 返回值
///
/// data URI 返回逗号后的 base64 payload；普通引用返回去除空白后的字符串。空 payload 返回 `None`。
pub fn extract_ollama_image_payload(image_ref: &str) -> Option<String> {
    if image_ref.starts_with("data:") {
        let comma_idx = image_ref.find(',')?;
        let (_, payload) = image_ref.split_at(comma_idx + 1);
        let payload = payload.trim();
        if payload.is_empty() { None } else { Some(payload.to_string()) }
    } else {
        Some(image_ref.trim().to_string()).filter(|value| !value.is_empty())
    }
}

/// 根据多模态配置为模型提供商准备消息。
///
/// # 参数
///
/// - `messages`: 原始聊天消息。
/// - `config`: 多模态能力、远程下载和大小限制配置。
///
/// # 返回值
///
/// 返回已归一化的 `PreparedMessages`，调用方可据此选择图片模型路径。
///
/// # 错误
///
/// 当图片数量超限、大小超限、MIME 类型不受支持、远程下载被禁用或读取失败时返回错误。
pub async fn prepare_messages_for_provider(
    messages: &[ChatMessage],
    config: &MultimodalConfig,
) -> anyhow::Result<PreparedMessages> {
    let (max_images, max_image_size_mb) = config.effective_limits();
    let max_bytes = max_image_size_mb.saturating_mul(1024 * 1024);

    let found_images = count_image_markers(messages);
    if found_images > max_images {
        // 数量限制先于任何 IO 执行，避免超量输入触发额外文件读取或网络请求。
        return Err(MultimodalError::TooManyImages { max_images, found: found_images }.into());
    }

    if found_images == 0 {
        return Ok(PreparedMessages { messages: messages.to_vec(), contains_images: false });
    }

    let remote_client = build_runtime_proxy_client_with_timeouts("provider.ollama", 30, 10);
    let output_budget = per_image_output_budget(found_images);

    let mut normalized_messages = Vec::with_capacity(messages.len());
    for message in messages {
        if message.role != "user" {
            normalized_messages.push(message.clone());
            continue;
        }

        let (cleaned_text, refs) = parse_image_markers(&message.content);
        if refs.is_empty() {
            normalized_messages.push(message.clone());
            continue;
        }

        let mut normalized_refs = Vec::with_capacity(refs.len());
        for reference in refs {
            let data_uri = normalize_image_reference(
                &reference,
                config,
                max_bytes,
                output_budget,
                &remote_client,
            )
            .await?;
            normalized_refs.push(data_uri);
        }

        let content = compose_multimodal_message(&cleaned_text, &normalized_refs);
        normalized_messages.push(ChatMessage { role: message.role.clone(), content });
    }

    Ok(PreparedMessages { messages: normalized_messages, contains_images: true })
}

/// 归一化 OpenAI 兼容 JSON 消息数组中的用户图片标记。
///
/// 仅处理 `role=user` 且 `content` 为字符串、并包含 `[IMAGE:...]` 的消息：把本地路径、
/// 远程 URL 或 data URI 统一为受限 data URI。数组形态的多模态 content、以及非用户消息
/// 保持原样，避免破坏 tool_calls / image_url parts 等已结构化字段。
pub async fn prepare_openai_json_messages_for_provider(
    messages: serde_json::Value,
    config: &MultimodalConfig,
) -> anyhow::Result<serde_json::Value> {
    let serde_json::Value::Array(mut arr) = messages else {
        return Ok(messages);
    };

    let mut chat = Vec::new();
    let mut mapped_indices = Vec::new();
    for (index, message) in arr.iter().enumerate() {
        let Some(obj) = message.as_object() else {
            continue;
        };
        if obj.get("role").and_then(serde_json::Value::as_str) != Some("user") {
            continue;
        }
        let Some(content) = obj.get("content").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if parse_image_markers(content).1.is_empty() {
            continue;
        }
        chat.push(ChatMessage::user(content));
        mapped_indices.push(index);
    }

    if chat.is_empty() {
        return Ok(serde_json::Value::Array(arr));
    }

    let prepared = prepare_messages_for_provider(&chat, config).await?;
    for (chat_index, &message_index) in mapped_indices.iter().enumerate() {
        let Some(prepared_message) = prepared.messages.get(chat_index) else {
            continue;
        };
        if let Some(obj) = arr.get_mut(message_index).and_then(serde_json::Value::as_object_mut) {
            obj.insert(
                "content".to_string(),
                serde_json::Value::String(prepared_message.content.clone()),
            );
        }
    }

    Ok(serde_json::Value::Array(arr))
}

fn compose_multimodal_message(text: &str, data_uris: &[String]) -> String {
    let mut content = String::new();
    let trimmed = text.trim();

    if !trimmed.is_empty() {
        content.push_str(trimmed);
        content.push_str("\n\n");
    }

    for (index, data_uri) in data_uris.iter().enumerate() {
        if index > 0 {
            content.push('\n');
        }
        content.push_str(IMAGE_MARKER_PREFIX);
        content.push_str(data_uri);
        content.push(']');
    }

    content
}

async fn normalize_image_reference(
    source: &str,
    config: &MultimodalConfig,
    max_bytes: usize,
    output_budget: usize,
    remote_client: &Client,
) -> anyhow::Result<String> {
    if source.starts_with("data:") {
        return normalize_data_uri(source, max_bytes, output_budget);
    }

    if source.starts_with("http://") || source.starts_with("https://") {
        if !config.allow_remote_fetch {
            return Err(MultimodalError::RemoteFetchDisabled { input: source.to_string() }.into());
        }

        return normalize_remote_image(source, max_bytes, output_budget, remote_client).await;
    }

    normalize_local_image(source, max_bytes, output_budget).await
}

fn normalize_data_uri(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
) -> anyhow::Result<String> {
    let Some(comma_idx) = source.find(',') else {
        return Err(MultimodalError::InvalidMarker {
            input: source.to_string(),
            reason: "expected data URI payload".to_string(),
        }
        .into());
    };

    let header = &source[..comma_idx];
    let payload = source[comma_idx + 1..].trim();

    if !header.contains(";base64") {
        return Err(MultimodalError::InvalidMarker {
            input: source.to_string(),
            reason: "only base64 data URIs are supported".to_string(),
        }
        .into());
    }

    let mime = header
        .trim_start_matches("data:")
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    validate_mime(source, &mime)?;

    let decoded = STANDARD.decode(payload).map_err(|error| MultimodalError::InvalidMarker {
        input: source.to_string(),
        reason: format!("invalid base64 payload: {error}"),
    })?;

    // 先解码再按实际字节数校验，避免压缩后的文本长度绕过图片大小限制。
    validate_size(source, decoded.len(), max_bytes)?;

    to_data_uri(source, &mime, &decoded, output_budget)
}

async fn normalize_remote_image(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
    remote_client: &Client,
) -> anyhow::Result<String> {
    let response = remote_client.get(source).send().await.map_err(|error| {
        MultimodalError::RemoteFetchFailed { input: source.to_string(), reason: error.to_string() }
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(MultimodalError::RemoteFetchFailed {
            input: source.to_string(),
            reason: format!("HTTP {status}"),
        }
        .into());
    }

    if let Some(content_length) = response.content_length() {
        let content_length = usize::try_from(content_length).unwrap_or(usize::MAX);
        // Content-Length 是提前拒绝的大门；后续仍会按真实下载字节数再校验一次。
        validate_size(source, content_length, max_bytes)?;
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);

    let bytes = response.bytes().await.map_err(|error| MultimodalError::RemoteFetchFailed {
        input: source.to_string(),
        reason: error.to_string(),
    })?;

    validate_size(source, bytes.len(), max_bytes)?;

    let mime = detect_mime(None, bytes.as_ref(), content_type.as_deref()).ok_or_else(|| {
        MultimodalError::UnsupportedMime { input: source.to_string(), mime: "unknown".to_string() }
    })?;

    validate_mime(source, &mime)?;

    to_data_uri(source, &mime, bytes.as_ref(), output_budget)
}

#[cfg(target_arch = "wasm32")]
async fn normalize_local_image(
    source: &str,
    _max_bytes: usize,
    _output_budget: usize,
) -> anyhow::Result<String> {
    Err(MultimodalError::LocalReadFailed {
        input: source.to_string(),
        reason: "Local file access is not supported on WASM".to_string(),
    }
    .into())
}

#[cfg(not(target_arch = "wasm32"))]
async fn normalize_local_image(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
) -> anyhow::Result<String> {
    let path = Path::new(source);
    if !path.exists() || !path.is_file() {
        return Err(MultimodalError::ImageSourceNotFound { input: source.to_string() }.into());
    }

    let metadata = tokio::fs::metadata(path).await.map_err(|error| {
        MultimodalError::LocalReadFailed { input: source.to_string(), reason: error.to_string() }
    })?;

    validate_size(source, usize::try_from(metadata.len()).unwrap_or(usize::MAX), max_bytes)?;

    let bytes = tokio::fs::read(path).await.map_err(|error| MultimodalError::LocalReadFailed {
        input: source.to_string(),
        reason: error.to_string(),
    })?;

    validate_size(source, bytes.len(), max_bytes)?;

    let mime = detect_mime(Some(path), &bytes, None).ok_or_else(|| {
        MultimodalError::UnsupportedMime { input: source.to_string(), mime: "unknown".to_string() }
    })?;

    validate_mime(source, &mime)?;

    to_data_uri(source, &mime, &bytes, output_budget)
}

fn to_data_uri(
    source: &str,
    mime: &str,
    bytes: &[u8],
    output_budget: usize,
) -> anyhow::Result<String> {
    let materialized = materialize_image_bytes(source, mime, bytes, output_budget)?;
    Ok(materialized.model_input_data_uri())
}

/// 源大小校验后的原图与压缩模型输入。
#[derive(Debug, Clone)]
pub struct MaterializedImage {
    /// 原始图片引用（路径、URL 或 data URI）。
    pub source: String,
    /// 源校验后的原始字节。
    pub original_bytes: Vec<u8>,
    /// 原始 MIME。
    pub original_mime: String,
    /// 压缩后的模型输入字节。
    pub model_input_bytes: Vec<u8>,
    /// 模型输入 MIME。
    pub model_input_mime: String,
}

impl MaterializedImage {
    /// 模型输入对应的 data URI。
    pub fn model_input_data_uri(&self) -> String {
        format!(
            "data:{};base64,{}",
            self.model_input_mime,
            STANDARD.encode(&self.model_input_bytes)
        )
    }
}

/// 加载并压缩单张图片引用，返回原图与模型输入变体。
pub async fn materialize_image_reference(
    source: &str,
    config: &MultimodalConfig,
    image_count_for_budget: usize,
) -> anyhow::Result<MaterializedImage> {
    let (_, max_image_size_mb) = config.effective_limits();
    let max_bytes = max_image_size_mb.saturating_mul(1024 * 1024);
    let output_budget = per_image_output_budget(image_count_for_budget.max(1));
    let remote_client = build_runtime_proxy_client_with_timeouts("provider.ollama", 30, 10);
    materialize_image_reference_with_budget(source, config, max_bytes, output_budget, &remote_client)
        .await
}

async fn materialize_image_reference_with_budget(
    source: &str,
    config: &MultimodalConfig,
    max_bytes: usize,
    output_budget: usize,
    remote_client: &Client,
) -> anyhow::Result<MaterializedImage> {
    if source.starts_with("data:") {
        return materialize_data_uri(source, max_bytes, output_budget);
    }
    if source.starts_with("http://") || source.starts_with("https://") {
        if !config.allow_remote_fetch {
            return Err(MultimodalError::RemoteFetchDisabled { input: source.to_string() }.into());
        }
        return materialize_remote_image(source, max_bytes, output_budget, remote_client).await;
    }
    materialize_local_image(source, max_bytes, output_budget).await
}

fn materialize_image_bytes(
    source: &str,
    mime: &str,
    bytes: &[u8],
    output_budget: usize,
) -> anyhow::Result<MaterializedImage> {
    let (output_mime, output_bytes) =
        compress_image_bytes(bytes, mime, output_budget).map_err(|reason| {
            MultimodalError::CompressionFailed { input: source.to_string(), reason }
        })?;
    Ok(MaterializedImage {
        source: source.to_string(),
        original_bytes: bytes.to_vec(),
        original_mime: mime.to_ascii_lowercase(),
        model_input_bytes: output_bytes,
        model_input_mime: output_mime,
    })
}

fn materialize_data_uri(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
) -> anyhow::Result<MaterializedImage> {
    let Some(comma_idx) = source.find(',') else {
        return Err(MultimodalError::InvalidMarker {
            input: source.to_string(),
            reason: "expected data URI payload".to_string(),
        }
        .into());
    };
    let header = &source[..comma_idx];
    let payload = source[comma_idx + 1..].trim();
    if !header.contains(";base64") {
        return Err(MultimodalError::InvalidMarker {
            input: source.to_string(),
            reason: "only base64 data URIs are supported".to_string(),
        }
        .into());
    }
    let mime = header
        .trim_start_matches("data:")
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    validate_mime(source, &mime)?;
    let decoded = STANDARD.decode(payload).map_err(|error| MultimodalError::InvalidMarker {
        input: source.to_string(),
        reason: format!("invalid base64 payload: {error}"),
    })?;
    validate_size(source, decoded.len(), max_bytes)?;
    materialize_image_bytes(source, &mime, &decoded, output_budget)
}

async fn materialize_remote_image(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
    remote_client: &Client,
) -> anyhow::Result<MaterializedImage> {
    let response = remote_client.get(source).send().await.map_err(|error| {
        MultimodalError::RemoteFetchFailed { input: source.to_string(), reason: error.to_string() }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(MultimodalError::RemoteFetchFailed {
            input: source.to_string(),
            reason: format!("HTTP {status}"),
        }
        .into());
    }
    if let Some(content_length) = response.content_length() {
        let content_length = usize::try_from(content_length).unwrap_or(usize::MAX);
        validate_size(source, content_length, max_bytes)?;
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let bytes = response.bytes().await.map_err(|error| MultimodalError::RemoteFetchFailed {
        input: source.to_string(),
        reason: error.to_string(),
    })?;
    validate_size(source, bytes.len(), max_bytes)?;
    let mime = detect_mime(None, bytes.as_ref(), content_type.as_deref()).ok_or_else(|| {
        MultimodalError::UnsupportedMime { input: source.to_string(), mime: "unknown".to_string() }
    })?;
    validate_mime(source, &mime)?;
    materialize_image_bytes(source, &mime, bytes.as_ref(), output_budget)
}

#[cfg(target_arch = "wasm32")]
async fn materialize_local_image(
    source: &str,
    _max_bytes: usize,
    _output_budget: usize,
) -> anyhow::Result<MaterializedImage> {
    Err(MultimodalError::LocalReadFailed {
        input: source.to_string(),
        reason: "Local file access is not supported on WASM".to_string(),
    }
    .into())
}

#[cfg(not(target_arch = "wasm32"))]
async fn materialize_local_image(
    source: &str,
    max_bytes: usize,
    output_budget: usize,
) -> anyhow::Result<MaterializedImage> {
    let path = Path::new(source);
    if !path.exists() || !path.is_file() {
        return Err(MultimodalError::ImageSourceNotFound { input: source.to_string() }.into());
    }
    let metadata = tokio::fs::metadata(path).await.map_err(|error| {
        MultimodalError::LocalReadFailed { input: source.to_string(), reason: error.to_string() }
    })?;
    validate_size(source, usize::try_from(metadata.len()).unwrap_or(usize::MAX), max_bytes)?;
    let bytes = tokio::fs::read(path).await.map_err(|error| MultimodalError::LocalReadFailed {
        input: source.to_string(),
        reason: error.to_string(),
    })?;
    validate_size(source, bytes.len(), max_bytes)?;
    let mime = detect_mime(Some(path), &bytes, None).ok_or_else(|| {
        MultimodalError::UnsupportedMime { input: source.to_string(), mime: "unknown".to_string() }
    })?;
    validate_mime(source, &mime)?;
    materialize_image_bytes(source, &mime, &bytes, output_budget)
}

fn validate_size(source: &str, size_bytes: usize, max_bytes: usize) -> anyhow::Result<()> {
    if size_bytes > max_bytes {
        return Err(MultimodalError::ImageTooLarge {
            input: source.to_string(),
            size_bytes,
            max_bytes,
        }
        .into());
    }

    Ok(())
}

fn validate_mime(source: &str, mime: &str) -> anyhow::Result<()> {
    if ALLOWED_IMAGE_MIME_TYPES.contains(&mime) {
        return Ok(());
    }

    // MIME 白名单保持默认拒绝，避免 SVG/HTML 等可携带脚本或外链的格式进入模型请求。
    Err(MultimodalError::UnsupportedMime { input: source.to_string(), mime: mime.to_string() }
        .into())
}

fn detect_mime(
    path: Option<&Path>,
    bytes: &[u8],
    header_content_type: Option<&str>,
) -> Option<String> {
    if let Some(header_mime) = header_content_type.and_then(normalize_content_type) {
        return Some(header_mime);
    }

    if let Some(path) = path {
        if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
            if let Some(mime) = mime_from_extension(ext) {
                return Some(mime.to_string());
            }
        }
    }

    mime_from_magic(bytes).map(ToString::to_string)
}

fn normalize_content_type(content_type: &str) -> Option<String> {
    let mime = content_type.split(';').next()?.trim().to_ascii_lowercase();
    if mime.is_empty() { None } else { Some(mime) }
}

fn mime_from_extension(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

fn mime_from_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']) {
        return Some("image/png");
    }

    if bytes.len() >= 3 && bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }

    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return Some("image/gif");
    }

    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }

    if bytes.len() >= 2 && bytes.starts_with(b"BM") {
        return Some("image/bmp");
    }

    None
}

#[cfg(test)]
#[path = "analysis_tests.rs"]
mod analysis_tests;

#[cfg(test)]
#[path = "assets_tests.rs"]
mod assets_tests;

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;

#[cfg(test)]
#[path = "image_compression_tests.rs"]
mod image_compression_tests;

#[cfg(test)]
#[path = "ocr_tests.rs"]
mod ocr_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
