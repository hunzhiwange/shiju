//! Resident-only repair after a self-identified rejection. Never used by session relay.
use axum::http::StatusCode;
use serde_json::{Value, json};

/// `Err` means the current encrypted task cannot safely be replaced.
pub(crate) fn repair(
    body: &Value,
    status: StatusCode,
    error: &[u8],
    url: &reqwest::Url,
) -> Result<Option<Value>, ()> {
    let text = String::from_utf8_lossy(error);
    let lower = text.to_lowercase();
    let messages = url.path().trim_end_matches('/').ends_with("/messages");
    let responses = url.path().trim_end_matches('/').ends_with("/responses");
    let signature = status == StatusCode::BAD_REQUEST && messages
        && ((lower.contains("signature") && lower.contains("thinking") && lower.contains("invalid"))
            || (lower.contains("thought signature") && (lower.contains("not valid") || lower.contains("invalid")))
            || lower.contains("must start with a thinking block")
            || (lower.contains("expected") && lower.contains("thinking") && lower.contains("found") && lower.contains("tool_use"))
            || (lower.contains("signature") && (lower.contains("field required") || lower.contains("extra inputs are not permitted")))
            || (lower.contains("thinking") && lower.contains("cannot be modified")));
    let payload: Value = serde_json::from_slice(error).unwrap_or(Value::Null);
    let message = payload.pointer("/error/message").or_else(|| payload.get("message"))
        .or_else(|| payload.get("error")).or_else(|| payload.get("detail"))
        .and_then(Value::as_str).unwrap_or(&text);
    let foreign = message.contains("created under a different") && message.contains("resource");
    let opaque = responses && ((matches!(status.as_u16(), 400 | 422)
        && (payload.pointer("/error/code").and_then(Value::as_str) == Some("invalid_encrypted_content")
            || (message.starts_with("The encrypted content") && message.contains("could not be verified"))
            || (message.contains("was not issued to this caller") && (message.contains("reasoning") || message.contains("encrypted_content")))
            || foreign
            || (message.contains("not found") && message.contains("Items are not persisted when"))
            || (message.contains("Invalid 'input[") && message.contains("Expected an ID that begins with"))))
        || (status == StatusCode::BAD_GATEWAY && message == "Encrypted function output content could not be decrypted or decoded."));
    if !signature && !opaque { return Ok(None); }
    let mut fixed = body.clone();
    if signature {
        if let Some(items) = fixed["messages"].as_array_mut() {
            for item in items {
                if let Some(parts) = item["content"].as_array_mut() {
                    parts.retain(|p| !matches!(p["type"].as_str(), Some("thinking" | "redacted_thinking")));
                    for part in parts {
                        if let Some(object) = part.as_object_mut() { object.remove("signature"); }
                    }
                }
            }
        }
        // Removing historical thinking makes the tool continuation incompatible with enabled thinking.
        if fixed["messages"].as_array().is_some_and(|items| items.iter().any(|item|
            item["content"].as_array().is_some_and(|parts| parts.iter().any(|p| p["type"] == "tool_use")))) {
            fixed.as_object_mut().unwrap().remove("thinking");
        }
    }
    if opaque {
        let third_party = !matches!(url.host_str(), Some("api.openai.com" | "chatgpt.com" | "chat.openai.com"));
        if let Some(items) = fixed["input"].as_array_mut() {
            let current = items.iter().rposition(|item| !matches!(item["type"].as_str(), Some("compaction_trigger" | "additional_tools")));
            if current.is_some_and(|i| items[i]["type"] == "agent_message" && items[i]["content"].as_array()
                .is_some_and(|parts| parts.iter().any(encrypted))) { return Err(()); }
            items.retain(|item| item["type"] != "reasoning");
            for item in items {
                let kind = item["type"].as_str().unwrap_or("").to_owned();
                if matches!(kind.as_str(), "compaction" | "context_compaction") && (third_party || message.contains("compaction")) {
                    let replay = vw_agent_apps::routing::compaction_replay::replay_text(item["encrypted_content"].as_str().unwrap_or(""));
                    if let Some(text) = replay {
                        *item = json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]});
                    }
                    continue;
                }
                let field = match kind.as_str() { "agent_message" => "content", "function_call_output" | "custom_tool_call_output" => "output", _ => "" };
                if let Some(parts) = item.get_mut(field).and_then(Value::as_array_mut) {
                    for part in parts { if encrypted(part) { *part = json!({"type":"input_text","text":"[encrypted content omitted]"}); } }
                }
                let prefix = match kind.as_str() { "message" => Some("msg_"), "function_call" => Some("fc_"), _ => None };
                if kind != "item_reference" && (foreign || prefix.is_some_and(|p| item["id"].as_str().is_some_and(|id| !id.starts_with(p)))) {
                    if let Some(object) = item.as_object_mut() { object.remove("id"); }
                }
            }
        }
    }
    Ok((fixed != *body).then_some(fixed))
}
fn encrypted(part: &Value) -> bool {
    part["type"] == "encrypted_content" && part["encrypted_content"].as_str().is_some_and(|s| !s.is_empty())
}
