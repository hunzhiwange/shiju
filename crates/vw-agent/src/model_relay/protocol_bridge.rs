//! Bounded text/tool conversion through a request-local Chat wire representation.
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use vw_shared::provider::ModelProtocol;

#[path = "protocol_bridge_response.rs"]
mod decode;
#[path = "protocol_bridge_request.rs"]
mod encode;
const INVALID: StatusCode = StatusCode::BAD_REQUEST;
const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
const UPSTREAM: StatusCode = StatusCode::BAD_GATEWAY;
type Result<T> = std::result::Result<T, StatusCode>;

pub(super) fn request(target: ModelProtocol, chat: &Value) -> Result<Value> {
    keys(
        chat,
        &[
            "model",
            "messages",
            "max_tokens",
            "max_completion_tokens",
            "stream",
            "stream_options",
            "temperature",
            "top_p",
            "stop",
            "tools",
            "tool_choice",
            "parallel_tool_calls",
            "response_format",
            "reasoning_effort",
        ],
    )?;
    string(chat, "model")?;
    for field in ["stream", "parallel_tool_calls"] {
        if chat.get(field).is_some_and(|v| !v.is_boolean()) {
            return Err(INVALID);
        }
    }
    if let Some(options) = chat.get("stream_options") {
        keys(options, &["include_usage"])?;
        if options.get("include_usage").is_some_and(|v| !v.is_boolean()) {
            return Err(INVALID);
        }
    }
    for field in ["max_tokens", "max_completion_tokens"] {
        if chat.get(field).is_some_and(|v| v.as_u64().is_none_or(|n| n == 0)) {
            return Err(INVALID);
        }
    }
    if chat.get("max_tokens").is_some() && chat.get("max_completion_tokens").is_some() {
        return Err(INVALID);
    }
    for field in ["temperature", "top_p"] {
        if chat.get(field).is_some_and(|v| !v.is_number()) {
            return Err(INVALID);
        }
    }
    if chat.get("reasoning_effort").is_some_and(|v| v != "none") {
        return Err(UNSUPPORTED);
    }
    validate_history(chat)?;
    match target {
        ModelProtocol::Messages => encode::messages(chat),
        ModelProtocol::Responses => encode::responses(chat),
        ModelProtocol::Gemini => encode::gemini(chat),
        ModelProtocol::Chat => Ok(chat.clone()),
    }
}

pub(super) fn response(target: ModelProtocol, native: &Value) -> Result<Value> {
    if native.get("error").is_some_and(|v| !v.is_null()) {
        return Err(UPSTREAM);
    }
    match target {
        ModelProtocol::Messages => decode::messages(native),
        ModelProtocol::Responses => decode::responses(native),
        ModelProtocol::Gemini => decode::gemini(native),
        ModelProtocol::Chat => Ok(native.clone()),
    }
}
fn keys(value: &Value, allowed: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or(INVALID)?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(UNSUPPORTED);
    }
    Ok(())
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key].as_str().filter(|s| !s.is_empty()).ok_or(INVALID)
}
fn text(value: &Value) -> Result<String> {
    if value.is_null() {
        return Ok(String::new());
    }
    if let Some(s) = value.as_str() {
        return Ok(s.into());
    }
    let mut output = String::new();
    for part in value.as_array().ok_or(INVALID)? {
        keys(part, &["type", "text"])?;
        if part["type"] != "text" {
            return Err(UNSUPPORTED);
        }
        output.push_str(part["text"].as_str().ok_or(INVALID)?);
    }
    Ok(output)
}
fn arguments(call: &Value) -> Result<Value> {
    keys(call, &["id", "type", "function"])?;
    if call["type"] != "function" {
        return Err(UNSUPPORTED);
    }
    keys(&call["function"], &["name", "arguments"])?;
    string(call, "id")?;
    string(&call["function"], "name")?;
    let args: Value =
        serde_json::from_str(string(&call["function"], "arguments")?).map_err(|_| INVALID)?;
    if !args.is_object() {
        return Err(UNSUPPORTED);
    }
    Ok(args)
}
fn validate_history(chat: &Value) -> Result<()> {
    let mut pending = BTreeSet::new();
    let mut used = BTreeSet::new();
    for message in chat["messages"].as_array().filter(|a| !a.is_empty()).ok_or(INVALID)? {
        keys(
            message,
            &["role", "content", "tool_calls", "tool_call_id", "reasoning_content", "refusal"],
        )?;
        for field in ["reasoning_content", "refusal"] {
            if message
                .get(field)
                .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
            {
                return Err(UNSUPPORTED);
            }
        }
        let role = string(message, "role")?;
        text(&message["content"])?;
        if role == "tool" {
            if !pending.remove(string(message, "tool_call_id")?)
                || message.get("tool_calls").is_some()
            {
                return Err(INVALID);
            }
        } else {
            if !pending.is_empty() {
                return Err(INVALID);
            }
            if !matches!(role, "system" | "developer" | "user" | "assistant")
                || message.get("tool_call_id").is_some()
            {
                return Err(UNSUPPORTED);
            }
            if let Some(calls) = message.get("tool_calls").filter(|value| !value.is_null()) {
                if role != "assistant" {
                    return Err(INVALID);
                }
                for call in calls.as_array().ok_or(INVALID)? {
                    arguments(call)?;
                    let id = string(call, "id")?.to_owned();
                    if !used.insert(id.clone()) {
                        return Err(INVALID);
                    }
                    pending.insert(id);
                }
            }
        }
    }
    if !pending.is_empty() {
        return Err(INVALID);
    }
    Ok(())
}
fn definitions(chat: &Value) -> Result<Vec<Value>> {
    let mut output = Vec::new();
    let mut names = BTreeSet::new();
    if let Some(tools) = chat.get("tools") {
        for tool in tools.as_array().ok_or(INVALID)? {
            keys(tool, &["type", "function"])?;
            if tool["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            let f = &tool["function"];
            keys(f, &["name", "description", "parameters", "strict"])?;
            if !names.insert(string(f, "name")?) || !f["parameters"].is_object() {
                return Err(INVALID);
            }
            if f.get("description").is_some_and(|v| !v.is_string())
                || f.get("strict").is_some_and(|v| !v.is_boolean())
            {
                return Err(INVALID);
            }
            output.push(f.clone());
        }
    }
    Ok(output)
}
fn chat_result(
    id: &str,
    model: &str,
    content: String,
    calls: Vec<Value>,
    reason: &str,
    usage: Value,
) -> Result<Value> {
    let mut ids = BTreeSet::new();
    for call in &calls {
        arguments(call).map_err(|_| UPSTREAM)?;
        if !ids.insert(string(call, "id")?) {
            return Err(UPSTREAM);
        }
    }
    let input = usage["prompt_tokens"].as_u64().ok_or(UPSTREAM)?;
    let output = usage["completion_tokens"].as_u64().ok_or(UPSTREAM)?;
    let mut usage = usage;
    usage["total_tokens"] = input.checked_add(output).ok_or(UPSTREAM)?.into();
    let mut message = json!({"role":"assistant","content":content});
    if !calls.is_empty() {
        message["tool_calls"] = calls.into();
    }
    Ok(
        json!({"id":id,"object":"chat.completion","model":model,"choices":[{"index":0,"message":message,"finish_reason":reason}],"usage":usage}),
    )
}
#[cfg(test)]
#[path = "protocol_bridge_tests.rs"]
mod tests;
