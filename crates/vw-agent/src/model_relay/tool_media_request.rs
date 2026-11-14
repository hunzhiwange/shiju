use super::tool_media::{
    flush_pending_chat_tool_media, plan_chat_tool_output_media, queue_chat_tool_output_media,
};
use serde_json::{Value, json};

/// Extract only tool-result payloads, leaving all other request validation to
/// the existing resident converter. No paths or URLs are opened here.
pub(super) fn messages(body: &mut Value) -> Vec<(String, Vec<Value>)> {
    let mut plans = Vec::new();
    if let Some(messages) = body["messages"].as_array_mut() {
        for message in messages {
            if message["role"] != "user" {
                continue;
            }
            if let Some(blocks) = message["content"].as_array_mut() {
                for block in blocks {
                    if block["type"] != "tool_result" {
                        continue;
                    }
                    if let Some(plan) = plan_chat_tool_output_media(block["content"].clone()) {
                        plans.push((
                            block["tool_use_id"].as_str().unwrap_or_default().to_owned(),
                            plan.media_parts,
                        ));
                        block["content"] = plan.tool_content.into();
                    }
                }
            }
        }
    }
    plans
}

pub(super) fn responses(body: &mut Value) -> Vec<(String, Vec<Value>)> {
    let mut plans = Vec::new();
    if let Some(items) = body["input"].as_array_mut() {
        for item in items {
            if !matches!(
                item["type"].as_str(),
                Some("function_call_output" | "custom_tool_call_output")
            ) {
                continue;
            }
            if let Some(plan) = plan_chat_tool_output_media(item["output"].clone()) {
                plans.push((
                    item["call_id"].as_str().unwrap_or_default().to_owned(),
                    plan.media_parts,
                ));
                item["output"] = plan.tool_content.into();
            }
        }
    }
    plans
}

/// Flush after the complete contiguous tool-result round, including parallel
/// calls. Inserting a user turn between tool results invalidates Chat history.
pub(super) fn attach(chat: &mut Value, plans: Vec<(String, Vec<Value>)>) {
    let Some(messages) = chat["messages"].as_array_mut() else {
        return;
    };
    let mut output = Vec::new();
    let mut pending = Vec::new();
    for message in std::mem::take(messages) {
        if message["role"] != "tool" {
            flush_pending_chat_tool_media(&mut output, &mut pending);
        }
        for (id, parts) in &plans {
            if message["role"] == "tool" && message["tool_call_id"] == *id {
                queue_chat_tool_output_media(&mut pending, id, parts.clone());
            }
        }
        output.push(message);
    }
    flush_pending_chat_tool_media(&mut output, &mut pending);
    *messages = output;
}

/// Keep the shared text bridge strict. Resident media travels through explicit
/// request-local placeholders and is restored only in protocol content arrays.
pub(super) fn request(
    target: vw_shared::provider::ModelProtocol,
    chat: &Value,
) -> Result<Value, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use vw_shared::provider::ModelProtocol;
    let mut normalized = chat.clone();
    let mut replacements = Vec::new();
    for message in normalized["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)? {
        if message["role"] != "user" {
            continue;
        }
        let Some(parts) = message["content"].as_array() else {
            continue;
        };
        if !parts.iter().any(|part| part["type"] != "text") {
            continue;
        }
        let mut native = Vec::new();
        for part in parts {
            native.push(match part["type"].as_str() {
                Some("text") => {
                    if target == ModelProtocol::Gemini {
                        json!({"text":part["text"]})
                    } else {
                        json!({"type":"input_text","text":part["text"]})
                    }
                }
                Some("image_url") => {
                    json!({"type":"input_image","image_url":part["image_url"]["url"]})
                }
                Some("file") => {
                    let mut file = part["file"].clone();
                    file["type"] = "input_file".into();
                    file
                }
                _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            });
        }
        let native = match target {
            ModelProtocol::Responses => native,
            ModelProtocol::Messages => super::responses_anthropic_request::content(&json!(native))?,
            ModelProtocol::Gemini => {
                let mut parts = Vec::new();
                for part in native {
                    if part.get("text").is_some() {
                        parts.push(part);
                        continue;
                    }
                    let data = part["image_url"]
                        .as_str()
                        .or_else(|| part["file_data"].as_str())
                        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                    let (meta, data) = data
                        .strip_prefix("data:")
                        .and_then(|s| s.split_once(','))
                        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                    let mime =
                        meta.strip_suffix(";base64").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                    if data.is_empty() {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    parts.push(json!({"inlineData":{"mimeType":mime,"data":data}}));
                }
                parts
            }
            ModelProtocol::Chat => return Ok(chat.clone()),
        };
        let marker = format!("shiju_tool_media_{}", uuid::Uuid::new_v4().simple());
        message["content"] = marker.clone().into();
        replacements.push((marker, native));
    }
    let mut output = super::protocol_bridge::request(target, &normalized)?;
    restore(&mut output, &replacements);
    Ok(output)
}

fn restore(value: &mut Value, replacements: &[(String, Vec<Value>)]) {
    match value {
        Value::Array(items) => {
            let mut output = Vec::new();
            for mut item in std::mem::take(items) {
                if let Some((_, parts)) =
                    replacements.iter().find(|(marker, _)| item["text"] == *marker)
                {
                    output.extend(parts.iter().cloned());
                } else {
                    restore(&mut item, replacements);
                    output.push(item);
                }
            }
            *items = output;
        }
        Value::Object(object) => {
            for (key, value) in object {
                if matches!(key.as_str(), "input" | "messages" | "contents" | "content" | "parts") {
                    restore(value, replacements);
                }
            }
        }
        _ => {}
    }
}
