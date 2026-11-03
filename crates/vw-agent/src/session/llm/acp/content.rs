//! 将已归一化的聊天图片转换为 ACP 内联内容块。
use serde_json::{Value, json};
use vw_acp::PromptInput;

use crate::app::agent::multimodal::parse_image_markers;

use super::AcpReplayStrategy;
use super::replay::build_request_prompt;

/// 先保留图片标记参与历史策略选择，再将选中的图片编码为 ACP image 块。
pub(super) fn build_request_content(
    messages: &Value,
    force_new_session: bool,
    strategy: AcpReplayStrategy,
    recent_count: usize,
) -> Result<PromptInput, String> {
    let mut messages = messages.clone();
    if let Some(items) = messages.as_array_mut() {
        let latest_user = items.iter().rposition(|item| item["role"] == "user");
        for (index, item) in items.iter_mut().enumerate() {
            if item["role"] != "user" {
                continue;
            }
            if let Some(parts) = item["content"].as_array() {
                let mut text = Vec::new();
                for part in parts {
                    match part["type"].as_str() {
                        Some("image_url" | "image") => {
                            let url = part["image_url"]
                                .as_str()
                                .or_else(|| part["image_url"]["url"].as_str())
                                .or_else(|| part["url"].as_str())
                                .ok_or("ACP image content is missing its URL")?;
                            text.push(format!("[IMAGE:{url}]"));
                        }
                        _ => {
                            if let Some(value) = part["text"].as_str() {
                                text.push(value.to_string());
                            }
                        }
                    }
                }
                item["content"] = Value::String(text.join("\n"));
            }
            // 摘要不应截断或携带历史图片的 base64 数据。
            if force_new_session
                && strategy == AcpReplayStrategy::Summary
                && Some(index) != latest_user
            {
                let (text, images) =
                    parse_image_markers(item["content"].as_str().unwrap_or_default());
                item["content"] = Value::String(if images.is_empty() {
                    text
                } else {
                    format!("{text}\n[历史图片附件]")
                });
            }
        }
    }
    let prompt = build_request_prompt(&messages, force_new_session, strategy, recent_count);
    let (text, images) = parse_image_markers(&prompt);
    let mut blocks = Vec::new();
    if !text.is_empty() {
        blocks.push(json!({"type": "text", "text": text}));
    }
    for image in images {
        let (header, data) =
            image.split_once(',').ok_or("ACP images must be normalized to base64 data URIs")?;
        let mime = header
            .strip_prefix("data:")
            .and_then(|h| h.strip_suffix(";base64"))
            .ok_or("ACP images must be normalized to base64 data URIs")?;
        blocks.push(json!({"type": "image", "mimeType": mime, "data": data}));
    }
    vw_acp::parse_prompt_source(&Value::Array(blocks).to_string())
        .map_err(|_| "ACP image content contains invalid MIME type or base64 data".to_string())
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
