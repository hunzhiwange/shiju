//! Lossless same-protocol Chat boundary. Returned tool calls are data, never executed here.
use crate::provider::provider::Model;
use axum::http::StatusCode;
use serde_json::Value;

pub(super) fn prepare(model: &Model, bound_model: &str, bytes: &[u8]) -> Result<Value, StatusCode> {
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let object = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    if object.get("model").and_then(Value::as_str) != Some(bound_model) {
        return Err(StatusCode::FORBIDDEN);
    }
    // Same-protocol unknown vendor fields remain intact; control-plane overrides never cross.
    if [
        "base_url",
        "api_base",
        "api_key",
        "authorization",
        "headers",
        "url",
        "provider",
        "tenant",
        "session",
        "generation",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let messages =
        object.get("messages").and_then(Value::as_array).ok_or(StatusCode::BAD_REQUEST)?;
    let tools_requested =
        object.get("tools").and_then(Value::as_array).is_some_and(|v| !v.is_empty())
            || object.contains_key("functions")
            || object.contains_key("tool_choice")
            || object.contains_key("function_call")
            || messages.iter().any(|m| {
                m.get("tool_calls").is_some()
                    || m.get("function_call").is_some()
                    || matches!(m["role"].as_str(), Some("tool" | "function"))
            });
    if tools_requested && !model.capabilities.toolcall {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if let Some(tools) = object.get("tools") {
        let tools = tools.as_array().ok_or(StatusCode::BAD_REQUEST)?;
        if tools.iter().any(|tool| tool["type"] != "function") {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }
    if object.contains_key("temperature") && !model.capabilities.temperature
        || (object.contains_key("reasoning_effort") || object.contains_key("reasoning"))
            && !model.capabilities.reasoning
        || object.contains_key("audio")
        || object.get("modalities").is_some_and(|v| v != &serde_json::json!(["text"]))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    for message in messages {
        if message.as_object().is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if let Some(parts) = message.get("content").and_then(Value::as_array) {
            for part in parts {
                let supported = match part["type"].as_str() {
                    Some("text") => model.capabilities.input.text,
                    Some("image_url") => model.capabilities.input.image,
                    Some("input_audio") => model.capabilities.input.audio,
                    // Signatures, files and provider hosted tools require explicit adapters.
                    _ => false,
                };
                if !supported {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
        }
    }
    for key in ["max_tokens", "max_completion_tokens"] {
        if let Some(value) = object.get(key) {
            let count = value.as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST)?;
            if model.limit.output > 0 && count > model.limit.output {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
    }
    object.insert("model".into(), model.api.id.clone().into());
    Ok(body)
}
