//! Resident-only Chat JSON adaptation. Reuses the strict tool/result converter.
use axum::http::StatusCode;
use serde_json::{Value, json};

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;

pub(crate) fn from_chat(body: &Value, model: &str) -> Result<Value, StatusCode> {
    if !body.is_object() {
        return Err(INVALID);
    }
    let mut body = body.clone();
    if !body["choices"].is_array() && body["data"]["choices"].is_array() {
        let mut outer = body.as_object_mut().ok_or(INVALID)?.clone();
        body = outer.remove("data").ok_or(INVALID)?;
        for (key, value) in outer {
            if key != "choices" && !value.is_null() {
                body.as_object_mut().ok_or(INVALID)?.insert(key, value);
            }
        }
    }
    if body.get("error").is_some() {
        return Err(INVALID);
    }
    let choices = body["choices"].as_array_mut().ok_or(INVALID)?;
    if choices.len() != 1 {
        return Err(INVALID);
    }
    let choice = &mut choices[0];
    if !choice.is_object() || !choice["message"].is_object() {
        return Err(INVALID);
    }
    let message = &mut choice["message"];
    let mut blocks = Vec::new();
    match &message["content"] {
        Value::Null => {}
        Value::String(text) => push_text(&mut blocks, text),
        Value::Array(parts) => {
            for part in parts {
                let key = match part["type"].as_str() {
                    Some("text" | "output_text") => "text",
                    Some("refusal") => "refusal",
                    _ => return Err(INVALID),
                };
                super::messages::object_keys(part, &["type", key]).map_err(|_| INVALID)?;
                push_text(&mut blocks, part[key].as_str().ok_or(INVALID)?);
            }
        }
        _ => return Err(INVALID),
    }
    if let Some(refusal) = message.get("refusal").filter(|v| !v.is_null()) {
        push_text(&mut blocks, refusal.as_str().ok_or(INVALID)?);
    }
    message["content"] = Value::Null;
    message.as_object_mut().ok_or(INVALID)?.remove("refusal");
    if let Some(legacy) = message.as_object_mut().ok_or(INVALID)?.remove("function_call") {
        if message.get("tool_calls").is_some_and(|v| !v.is_null()) {
            return Err(INVALID);
        }
        super::messages::object_keys(&legacy, &["id", "name", "arguments"]).map_err(|_| INVALID)?;
        let arguments = match &legacy["arguments"] {
            Value::String(s) => s.clone(),
            Value::Object(_) => legacy["arguments"].to_string(),
            _ => return Err(INVALID),
        };
        message["tool_calls"] = json!([{"id":legacy["id"],"type":"function","function":{"name":legacy["name"],"arguments":arguments}}]);
    }
    if let Some(calls) = message.get("tool_calls").filter(|v| !v.is_null()) {
        for call in calls.as_array().ok_or(INVALID)? {
            if call["id"].as_str().is_none_or(str::is_empty)
                || call["function"]["name"].as_str().is_none_or(str::is_empty)
            {
                return Err(INVALID);
            }
        }
    }
    choice["finish_reason"] = match choice["finish_reason"].as_str() {
        Some("content_filter") => json!("stop"),
        Some("function_call") => json!("tool_calls"),
        Some("stop" | "length" | "tool_calls") => choice["finish_reason"].clone(),
        _ => return Err(INVALID),
    };
    let mut result = super::messages::from_chat(&body, model).map_err(|_| INVALID)?;
    blocks.extend(result["content"].as_array().ok_or(INVALID)?.iter().cloned());
    result["content"] = blocks.into();
    let usage = &body["usage"];
    let cached = first_nonzero(
        usage,
        &[
            "/cache_read_input_tokens",
            "/prompt_tokens_details/cached_tokens",
            "/prompt_cache_hit_tokens",
        ],
    )?;
    let creation = first_present(
        usage,
        &[
            "/cache_creation_input_tokens",
            "/prompt_tokens_details/cache_write_tokens",
            "/input_tokens_details/cache_write_tokens",
        ],
    )?;
    let fresh = usage["prompt_tokens"]
        .as_u64()
        .ok_or(INVALID)?
        .checked_sub(cached)
        .and_then(|n| n.checked_sub(creation))
        .ok_or(INVALID)?;
    result["usage"]["input_tokens"] = fresh.into();
    if cached > 0 {
        result["usage"]["cache_read_input_tokens"] = cached.into();
    }
    if creation > 0 {
        result["usage"]["cache_creation_input_tokens"] = creation.into();
    }
    Ok(result)
}

fn push_text(blocks: &mut Vec<Value>, text: &str) {
    if !text.is_empty() {
        blocks.push(json!({"type":"text","text":text}));
    }
}
fn first_nonzero(usage: &Value, paths: &[&str]) -> Result<u64, StatusCode> {
    for path in paths {
        if let Some(value) = usage.pointer(path) {
            let count = value.as_u64().ok_or(INVALID)?;
            if count > 0 {
                return Ok(count);
            }
        }
    }
    Ok(0)
}
fn first_present(usage: &Value, paths: &[&str]) -> Result<u64, StatusCode> {
    for path in paths {
        if let Some(value) = usage.pointer(path) {
            return value.as_u64().ok_or(INVALID);
        }
    }
    Ok(0)
}
