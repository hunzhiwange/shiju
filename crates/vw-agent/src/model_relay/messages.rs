//! Messages ↔ Chat text/tool boundary. No response references or tool execution state.
use axum::http::StatusCode;
use serde_json::{Value, json};

const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
pub(super) fn object_keys(value: &Value, keys: &[&str]) -> Result<(), StatusCode> {
    let object = value.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(UNSUPPORTED);
    }
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, StatusCode> {
    value[key].as_str().ok_or(StatusCode::BAD_REQUEST)
}
// Anthropic cache hints are optional performance annotations. Validate their
// complete shape before dropping them on Chat; never drop inference state.
fn cache_hint(block: &Value) -> Result<(), StatusCode> {
    if let Some(cache) = block.get("cache_control") {
        object_keys(cache, &["type", "ttl", "scope"])?;
        if cache["type"] != "ephemeral"
            || cache.get("ttl").is_some_and(|ttl| ttl != "5m" && ttl != "1h")
            || cache.get("scope").is_some_and(|scope| scope != "global")
        {
            return Err(UNSUPPORTED);
        }
    }
    Ok(())
}
fn text(value: &Value) -> Result<String, StatusCode> {
    if let Some(s) = value.as_str() {
        return Ok(s.into());
    }
    let blocks = value.as_array().ok_or(StatusCode::BAD_REQUEST)?;
    let mut result = String::new();
    for block in blocks {
        object_keys(block, &["type", "text", "cache_control"])?;
        cache_hint(block)?;
        if block["type"] != "text" {
            return Err(UNSUPPORTED);
        }
        result.push_str(string(block, "text")?);
    }
    Ok(result)
}

pub(super) fn to_chat(body: &Value) -> Result<Value, StatusCode> {
    object_keys(
        body,
        &[
            "model",
            "messages",
            "system",
            "max_tokens",
            "stream",
            "temperature",
            "top_p",
            "tools",
            "tool_choice",
            "metadata",
        ],
    )?;
    if let Some(metadata) = body.get("metadata") {
        object_keys(metadata, &["user_id"])?;
        if metadata.get("user_id").is_some_and(|value| !value.is_string()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        // Do not disclose a native Claude account/device/session identifier to
        // the selected provider. This analytics hint has no model semantics.
    }
    let mut output = json!({"model":body["model"],"messages":[],"max_tokens":body["max_tokens"]});
    if body["max_tokens"].as_u64().is_none_or(|n| n == 0) {
        return Err(StatusCode::BAD_REQUEST);
    }
    for key in ["stream", "temperature", "top_p"] {
        if let Some(value) = body.get(key) {
            output[key] = value.clone();
        }
    }
    if let Some(stream) = body.get("stream") {
        if !stream.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if stream == true {
            output["stream_options"] = json!({"include_usage":true});
        }
    }
    let mut messages = Vec::new();
    if let Some(system) = body.get("system") {
        messages.push(json!({"role":"system","content":text(system)?}));
    }
    for message in body["messages"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
        object_keys(message, &["role", "content"])?;
        let role = string(message, "role")?;
        if !matches!(role, "user" | "assistant") {
            return Err(UNSUPPORTED);
        }
        if let Some(content) = message["content"].as_str() {
            messages.push(json!({"role":role,"content":content}));
            continue;
        }
        let blocks = message["content"].as_array().ok_or(StatusCode::BAD_REQUEST)?;
        let mut content = String::new();
        let mut calls = Vec::new();
        for block in blocks {
            cache_hint(block)?;
            match block["type"].as_str() {
                Some("text") => {
                    object_keys(block, &["type", "text", "cache_control"])?;
                    if !calls.is_empty() {
                        return Err(UNSUPPORTED);
                    }
                    content.push_str(string(block, "text")?);
                }
                Some("tool_use") if role == "assistant" => {
                    object_keys(block, &["type", "id", "name", "input", "cache_control"])?;
                    if !block["input"].is_object() {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    calls.push(json!({"id":string(block,"id")?,"type":"function","function":{"name":string(block,"name")?,"arguments":block["input"].to_string()}}));
                }
                Some("tool_result") if role == "user" => {
                    object_keys(
                        block,
                        &["type", "tool_use_id", "content", "is_error", "cache_control"],
                    )?;
                    let mut result = text(&block["content"])?;
                    if let Some(is_error) = block.get("is_error")
                        && is_error.as_bool().ok_or(StatusCode::BAD_REQUEST)?
                    {
                        // Chat has no tool error flag. Preserve it in the correlated result.
                        result.insert_str(0, "Tool execution failed:\n");
                    }
                    if !content.is_empty() {
                        messages
                            .push(json!({"role":"user","content":std::mem::take(&mut content)}));
                    }
                    messages.push(json!({"role":"tool","tool_call_id":string(block,"tool_use_id")?,"content":result}));
                }
                _ => return Err(UNSUPPORTED),
            }
        }
        if !content.is_empty() || !calls.is_empty() || blocks.is_empty() {
            let mut message = json!({"role":role,"content":content});
            if !calls.is_empty() {
                message["tool_calls"] = calls.into();
            }
            messages.push(message);
        }
    }
    if messages.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    output["messages"] = messages.into();
    if let Some(tools) = body.get("tools") {
        let tools = tools.as_array().ok_or(StatusCode::BAD_REQUEST)?;
        let mut converted = Vec::new();
        for tool in tools {
            object_keys(tool, &["name", "description", "input_schema"])?;
            if !tool["input_schema"].is_object() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let mut function =
                json!({"name":string(tool,"name")?,"parameters":tool["input_schema"]});
            if let Some(description) = tool.get("description") {
                function["description"] = description.clone();
            }
            converted.push(json!({"type":"function","function":function}));
        }
        output["tools"] = converted.into();
    }
    if let Some(choice) = body.get("tool_choice") {
        object_keys(choice, &["type", "name", "disable_parallel_tool_use"])?;
        output["tool_choice"] = match string(choice, "type")? {
            "auto" => json!("auto"),
            "any" => json!("required"),
            "none" => json!("none"),
            "tool" => json!({"type":"function","function":{"name":string(choice,"name")?}}),
            _ => return Err(UNSUPPORTED),
        };
        if let Some(disabled) = choice.get("disable_parallel_tool_use") {
            output["parallel_tool_calls"] =
                json!(!disabled.as_bool().ok_or(StatusCode::BAD_REQUEST)?);
        }
    }
    Ok(output)
}

pub(super) fn finish(value: &Value) -> Result<&'static str, StatusCode> {
    match value.as_str() {
        Some("stop") => Ok("end_turn"),
        Some("length") => Ok("max_tokens"),
        Some("tool_calls") => Ok("tool_use"),
        _ => Err(UNSUPPORTED),
    }
}
pub(super) fn from_chat(body: &Value, model: &str) -> Result<Value, StatusCode> {
    let choices = body["choices"].as_array().ok_or(StatusCode::BAD_GATEWAY)?;
    if choices.len() != 1 {
        return Err(UNSUPPORTED);
    }
    let choice = &choices[0];
    let message = &choice["message"];
    object_keys(message, &["role", "content", "tool_calls", "refusal"])?;
    if message["role"] != "assistant" || !message["refusal"].is_null() {
        return Err(UNSUPPORTED);
    }
    let mut content = Vec::new();
    if let Some(s) = message["content"].as_str() {
        if !s.is_empty() {
            content.push(json!({"type":"text","text":s}));
        }
    } else if !message["content"].is_null() {
        return Err(UNSUPPORTED);
    }
    if let Some(calls) = message.get("tool_calls") {
        for call in calls.as_array().ok_or(StatusCode::BAD_GATEWAY)? {
            object_keys(call, &["id", "type", "function"])?;
            object_keys(&call["function"], &["name", "arguments"])?;
            if call["type"] != "function" {
                return Err(UNSUPPORTED);
            }
            let input: Value = serde_json::from_str(string(&call["function"], "arguments")?)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
            if !input.is_object() {
                return Err(UNSUPPORTED);
            }
            content.push(json!({"type":"tool_use","id":string(call,"id")?,"name":string(&call["function"],"name")?,"input":input}));
        }
    }
    let usage = &body["usage"];
    Ok(
        json!({"id":string(body,"id")?,"type":"message","role":"assistant","model":model,"content":content,"stop_reason":finish(&choice["finish_reason"])? ,"stop_sequence":null,"usage":{"input_tokens":usage["prompt_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?,"output_tokens":usage["completion_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?}}),
    )
}

pub(super) fn error(status: StatusCode) -> axum::response::Response {
    use axum::response::IntoResponse;
    let kind = match status.as_u16() {
        401 => "authentication_error",
        403 => "permission_error",
        429 => "rate_limit_error",
        400 | 413 | 422 => "invalid_request_error",
        _ => "api_error",
    };
    (status, axum::Json(json!({"type":"error","error":{"type":kind,"message": if status == UNSUPPORTED {"Unsupported Messages capability"} else {"Messages relay request failed"}}}))).into_response()
}

/// Same-protocol requests retain native blocks, signatures, cache controls and vendor options.
pub(super) fn native(
    model: &crate::provider::provider::Model,
    bound: &str,
    bytes: &[u8],
) -> Result<Value, StatusCode> {
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let object = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if object.get("model").and_then(Value::as_str) != Some(bound) {
        return Err(StatusCode::FORBIDDEN);
    }
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
    let max = body["max_tokens"].as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST)?;
    if model.limit.output > 0 && max > model.limit.output
        || body.get("temperature").is_some() && !model.capabilities.temperature
        || body.get("thinking").is_some() && !model.capabilities.reasoning
        || body.get("tools").is_some() && !model.capabilities.toolcall
    {
        return Err(UNSUPPORTED);
    }
    let messages = body["messages"].as_array().ok_or(StatusCode::BAD_REQUEST)?;
    for message in messages {
        if !matches!(message["role"].as_str(), Some("user" | "assistant")) {
            return Err(StatusCode::BAD_REQUEST);
        }
        if message["content"].is_string() {
            if !model.capabilities.input.text {
                return Err(UNSUPPORTED);
            }
        } else {
            for block in message["content"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
                let supported = match block["type"].as_str() {
                    Some("text") => model.capabilities.input.text,
                    Some("image") => model.capabilities.input.image,
                    Some("thinking" | "redacted_thinking") => model.capabilities.reasoning,
                    Some(
                        "tool_use" | "tool_result" | "server_tool_use" | "web_search_tool_result",
                    ) => model.capabilities.toolcall,
                    _ => false,
                };
                if !supported {
                    return Err(UNSUPPORTED);
                }
            }
        }
    }
    body["model"] = model.api.id.clone().into();
    Ok(body)
}
