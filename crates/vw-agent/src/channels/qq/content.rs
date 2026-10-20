use crate::app::agent::channels::traits::ChannelMessage;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 检查是否为远程媒体 URL
///
/// 判断给定字符串是否为有效的远程媒体 URL（HTTP 或 HTTPS 协议）。
fn is_remote_media_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("https://") || trimmed.starts_with("http://")
}

/// 检查文件名是否为图片格式
///
/// 根据文件扩展名判断文件是否为支持的图片格式。
fn is_image_filename(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
        || lower.ends_with(".heic")
        || lower.ends_with(".heif")
        || lower.ends_with(".svg")
}

/// 从附件中提取图片标记
///
/// 分析附件 JSON 对象，如果附件为图片类型，则生成 `[IMAGE:url]` 格式的标记字符串。
fn extract_image_marker_from_attachment(attachment: &Value) -> Option<String> {
    let url = attachment.get("url").and_then(Value::as_str)?.trim();
    if url.is_empty() {
        return None;
    }

    let content_type =
        attachment.get("content_type").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
    let filename = attachment.get("filename").and_then(Value::as_str).unwrap_or("");
    let is_image = content_type.starts_with("image/") || is_image_filename(filename);

    if !is_image {
        return None;
    }

    Some(format!("[IMAGE:{url}]"))
}

/// 解析图片标记行
///
/// 从文本行中提取 `[IMAGE:url]` 格式的图片标记内容。
fn parse_image_marker_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let marker = trimmed.strip_prefix("[IMAGE:")?.strip_suffix(']')?.trim();
    if marker.is_empty() {
        return None;
    }
    Some(marker)
}

/// 解析待发送的内容，分离文本和图片 URL。
pub(super) fn parse_outgoing_content(content: &str) -> (String, Vec<String>) {
    let mut passthrough_lines = Vec::new();
    let mut image_urls = Vec::new();

    for line in content.lines() {
        if let Some(marker_target) = parse_image_marker_line(line) {
            if is_remote_media_url(marker_target) {
                image_urls.push(marker_target.to_string());
                continue;
            }
        }
        passthrough_lines.push(line);
    }

    (passthrough_lines.join("\n").trim().to_string(), image_urls)
}

/// 组合消息内容（文本 + 附件）。
pub(super) fn compose_message_content(payload: &Value) -> Option<String> {
    let text = payload.get("content").and_then(Value::as_str).unwrap_or("").trim();

    let image_markers: Vec<String> = payload
        .get("attachments")
        .and_then(Value::as_array)
        .map(|attachments| {
            attachments.iter().filter_map(extract_image_marker_from_attachment).collect()
        })
        .unwrap_or_default();

    if text.is_empty() && image_markers.is_empty() {
        return None;
    }

    if text.is_empty() {
        return Some(image_markers.join("\n"));
    }

    if image_markers.is_empty() {
        return Some(text.to_string());
    }

    Some(format!("{text}\n\n{}", image_markers.join("\n")))
}

/// 获取当前 Unix 时间戳（秒）。
fn current_unix_timestamp_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// 构建通道消息对象。
pub(super) fn build_channel_message(
    sender: &str,
    reply_target: String,
    content: String,
    msg_id: &str,
) -> ChannelMessage {
    ChannelMessage {
        id: Uuid::new_v4().to_string(),
        sender: sender.to_string(),
        reply_target,
        content,
        channel: "qq".to_string(),
        timestamp: current_unix_timestamp_secs(),
        thread_ts: (!msg_id.is_empty()).then(|| msg_id.to_string()),
    }
}

/// 应用被动回复字段。
fn apply_passive_reply_fields(body: &mut Map<String, Value>, msg_id: Option<&str>, msg_seq: u64) {
    if let Some(msg_id) = msg_id {
        body.insert("msg_id".to_string(), Value::String(msg_id.to_string()));
        body.insert("msg_seq".to_string(), Value::from(msg_seq));
    }
}

/// 构建文本消息体。
pub(super) fn build_text_message_body(
    content: &str,
    msg_id: Option<&str>,
    msg_seq: u64,
) -> Option<Value> {
    let text = content.trim();
    if text.is_empty() {
        return None;
    }

    let mut body = Map::new();
    body.insert("content".to_string(), Value::String(text.to_string()));
    body.insert("msg_type".to_string(), Value::from(0));
    apply_passive_reply_fields(&mut body, msg_id, msg_seq);

    Some(Value::Object(body))
}

/// 构建 QQ 原生 Markdown 消息体。
pub(super) fn build_markdown_message_body(
    content: &str,
    msg_id: Option<&str>,
    msg_seq: u64,
) -> Option<Value> {
    let text = content.trim();
    if text.is_empty() {
        return None;
    }

    let mut body = Map::new();
    body.insert("markdown".to_string(), json!({"content": text}));
    body.insert("msg_type".to_string(), Value::from(2));
    apply_passive_reply_fields(&mut body, msg_id, msg_seq);
    Some(Value::Object(body))
}

/// QQ 普通文本接口不允许 URL；Markdown 降级时替换链接。
pub(super) fn sanitize_qq_plain_text(content: &str, aggressive: bool) -> String {
    static URL: OnceLock<Regex> = OnceLock::new();
    static BARE_DOMAIN: OnceLock<Regex> = OnceLock::new();
    let pattern = if aggressive {
        BARE_DOMAIN.get_or_init(|| {
            Regex::new(
                r"(?i)https?://[^\s]+|www\.[^\s]+|\b[\w][\w.-]*\.(?:com|cn|org|net|edu|gov|io|co|cc|tv|me|info|biz|app|dev|top|xyz|site|vip|shop|tech|club|pro|live|mobi|asia|wiki)(?:\.[a-z]{2,3})?\b(?:/[^\s]*)?",
            )
            .expect("QQ bare-domain regex must compile")
        })
    } else {
        URL.get_or_init(|| {
            Regex::new(r"(?i)https?://[^\s]+|www\.[^\s]+").expect("QQ URL regex must compile")
        })
    };
    pattern.replace_all(content, "[链接已省略]").into_owned()
}

/// 构建媒体消息体。
pub(super) fn build_media_message_body(
    file_info: &str,
    msg_id: Option<&str>,
    msg_seq: u64,
) -> Value {
    let mut body = Map::new();
    body.insert("content".to_string(), Value::String(" ".to_string()));
    body.insert("msg_type".to_string(), Value::from(7));
    body.insert("media".to_string(), json!({ "file_info": file_info }));
    apply_passive_reply_fields(&mut body, msg_id, msg_seq);
    Value::Object(body)
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
