//! Image fallback for an explicitly declared capability or a narrow upstream rejection.
use serde_json::{Value, json};

pub const UNSUPPORTED_IMAGE_MARKER: &str = "[Unsupported Image]";

/// Unknown capabilities leave the payload intact; model-name guesses are disabled.
pub fn prepare(body: &mut Value, settings: &Value, model: &str) -> usize {
    let metadata = vw_api_types::model_catalog::ModelMetadataDto::default();
    let capability = crate::models::capabilities::resolve_model_capabilities(
        model, Some(settings), &metadata, false,
    );
    if capability.image_input != vw_api_types::model_catalog::ModelCapabilityState::Unsupported {
        return 0;
    }
    replace(body)
}

pub fn repair(body: &Value, status: u16, error: &[u8]) -> Option<Value> {
    if !unsupported(status, error) { return None; }
    let mut fixed = body.clone();
    (replace(&mut fixed) > 0).then_some(fixed)
}

fn unsupported(status: u16, error: &[u8]) -> bool {
    let is_standard_modality_status = matches!(status, 400 | 415 | 422 | 501);
    if !is_standard_modality_status && status != 404 {
        return false;
    }

    let Ok(payload) = serde_json::from_slice::<Value>(error) else { return false; };
    let Some(message) = payload.pointer("/error/message").or_else(|| payload.get("message")).and_then(Value::as_str) else { return false; };
    let message = message.to_ascii_lowercase();

    // OpenRouter reports routing failure as 404 when every endpoint for a
    // model rejects image input. Keep this narrow: an ordinary missing route
    // must not trigger a body-mutating retry.
    if status == 404 {
        return message.contains("no endpoints found that support image input");
    }

    // 自证性表述：这类短语本身就断言了"仅接受文本"，属于模态拒绝，无需再要求
    // 错误提到 image/media 等字样——火山方舟等网关的报错是
    // "Model only support text input"，全程不出现 image（issue #5025）。
    // 国产网关的英文常缺三单 s，因此带 s / 不带 s 两种形式都要列。
    // GLM-5.2 text endpoint rejects any messages.content[*].type other than
    // `text` with a field-validation error that never mentions image/modality
    // (#6151). The Chinese form "content.type 参数非法，取值范围 ['text']"
    // is itself a text-only assertion.
    const TEXT_ONLY_SELF_EVIDENT_HINTS: &[&str] = &["only support text", "only supports text"];
    if TEXT_ONLY_SELF_EVIDENT_HINTS
        .iter()
        .any(|hint| message.contains(hint))
        || (message.contains("content.type 参数非法") && message.contains("取值范围 ['text']"))
    {
        return true;
    }

    let mentions_image = message.contains("image")
        || message.contains("vision")
        || message.contains("multimodal")
        || message.contains("multi-modal")
        || message.contains("modality")
        || message.contains("modalities")
        || message.contains("media")
        || message.contains("attachment");

    if !mentions_image {
        return false;
    }

    const UNSUPPORTED_HINTS: &[&str] = &[
        "unsupported",
        "not supported",
        "does not support",
        "doesn't support",
        "do not support",
        "don't support",
        "text only",
        "text-only",
        "invalid content type",
        "invalid message content",
        "unknown variant",
        "unknown content type",
        "unrecognized content type",
        "cannot process",
        "cannot handle",
        "can't process",
        "can't handle",
        "unable to process",
    ];

    UNSUPPORTED_HINTS.iter().any(|hint| message.contains(hint))
}


fn replace(body: &mut Value) -> usize {
    let mut count = 0;
    if let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) {
        for message in messages {
            let tool = message.get("role").and_then(Value::as_str) == Some("tool");
            if let Some(content) = message.get_mut("content") {
                count += walk(content, "text", tool, 0);
            }
        }
    }
    if let Some(input) = body.get_mut("input") {
        let items = match input { Value::Array(items) => items.as_mut_slice(), item => std::slice::from_mut(item) };
        for item in items {
            let tool = matches!(item.get("type").and_then(Value::as_str), Some("function_call_output" | "custom_tool_call_output"));
            if tool {
                if let Some(output) = item.get_mut("output") { count += walk(output, "input_text", true, 0); }
            } else { count += walk(item, "input_text", false, 0); }
        }
    }
    if let Some(contents) = body.get_mut("contents").and_then(Value::as_array_mut) {
        for content in contents {
            if let Some(parts) = content.get_mut("parts") { count += walk(parts, "", false, 0); }
        }
    }
    count
}

// Traverse content containers only. Never interpret tool arguments or schemas as media.
fn walk(value: &mut Value, text_type: &str, tool: bool, depth: usize) -> usize {
    if depth >= 32 { return 0; }
    if let Value::Array(items) = value {
        return items.iter_mut().map(|item| walk(item, text_type, tool, depth + 1)).sum();
    }
    if tool && let Some(text) = value.as_str() {
        if text.trim().starts_with("data:image/") {
            *value = Value::String(UNSUPPORTED_IMAGE_MARKER.into());
            return 1;
        }
        if let Ok(mut parsed) = serde_json::from_str::<Value>(text) {
            let count = walk(&mut parsed, text_type, true, depth + 1);
            if count > 0 { *value = Value::String(parsed.to_string()); }
            return count;
        }
    }
    let image = matches!(value.get("type").and_then(Value::as_str), Some("image" | "image_url" | "input_image"))
        || (text_type.is_empty() && ["inlineData", "inline_data", "fileData", "file_data"].iter().any(|key| {
            let payload = &value[*key];
            payload.get("mimeType").or_else(|| payload.get("mime_type")).and_then(Value::as_str).is_some_and(|mime| mime.starts_with("image/"))
        }));
    if image {
        let cache = value.get("cache_control").cloned();
        *value = if text_type.is_empty() { json!({"text": UNSUPPORTED_IMAGE_MARKER}) }
            else { json!({"type": text_type, "text": UNSUPPORTED_IMAGE_MARKER}) };
        if let Some(cache) = cache { value["cache_control"] = cache; }
        return 1;
    }
    if text_type.is_empty() {
        let key = if value.get("functionResponse").is_some() { "functionResponse" } else { "function_response" };
        if let Some(response) = value.get_mut(key) {
            let mut removed = 0;
            if let Some(parts) = response.get_mut("parts").and_then(Value::as_array_mut) {
                parts.retain_mut(|part| {
                    let count = walk(part, "", false, depth + 1);
                    removed += count;
                    count == 0
                });
            }
            if removed > 0 {
                if !response.get("response").is_some_and(Value::is_object) {
                    response["response"] = json!({});
                }
                response["response"]["cc_switch_media"] = json!(UNSUPPORTED_IMAGE_MARKER);
            }
            return removed;
        }
    }
    let nested_tool = tool || value.get("type").and_then(Value::as_str) == Some("tool_result");
    let mut count = 0;
    if let Some(content) = value.get_mut("content") { count += walk(content, text_type, nested_tool, depth + 1); }
    count
}
