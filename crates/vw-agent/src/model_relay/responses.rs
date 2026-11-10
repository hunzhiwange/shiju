//! Responses text/function boundary. References are opaque and scoped to the owning relay.
use super::{RelayIdentity, messages::object_keys};
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::{http::StatusCode, response::IntoResponse};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc};

#[path = "responses_chat_tools.rs"]
mod chat_tools;
pub(super) use chat_tools::ChatTools;
#[path = "responses_codex_chat.rs"]
mod codex_chat;
pub(super) use codex_chat::{codex_chat_effort, configure_codex_chat};

const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
pub(super) fn response_id() -> String {
    format!("resp_{}", uuid::Uuid::new_v4().simple())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, StatusCode> {
    value[key].as_str().filter(|v| !v.is_empty()).ok_or(StatusCode::BAD_REQUEST)
}
fn arguments(value: &Value) -> Result<(), StatusCode> {
    let parsed: Value =
        serde_json::from_str(string(value, "arguments")?).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !parsed.is_object() {
        return Err(UNSUPPORTED);
    }
    Ok(())
}
fn content(value: &Value, role: &str) -> Result<String, StatusCode> {
    if let Some(text) = value.as_str() {
        return Ok(text.into());
    }
    let mut text = String::new();
    for part in value.as_array().ok_or(StatusCode::BAD_REQUEST)? {
        object_keys(part, &["type", "text", "annotations", "logprobs"])?;
        if part["type"] != "input_text" && !(role == "assistant" && part["type"] == "output_text") {
            return Err(UNSUPPORTED);
        }
        for field in ["annotations", "logprobs"] {
            if part.get(field).is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty())) {
                return Err(UNSUPPORTED);
            }
        }
        text.push_str(part["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?);
    }
    Ok(text)
}
fn chat_content(value: &Value, role: &str, codex: bool) -> Result<Value, StatusCode> {
    if codex
        && role == "user"
        && let Some(parts) = value.as_array()
        && parts.iter().any(|part| part["type"] == "input_image")
    {
        let mut chat = Vec::new();
        for part in parts {
            if part["type"] == "input_image" {
                object_keys(part, &["type", "image_url", "detail"])?;
                let mut image = json!({"url": string(part, "image_url")?});
                if let Some(detail) = part.get("detail") {
                    if !matches!(detail.as_str(), Some("auto" | "low" | "high")) {
                        return Err(UNSUPPORTED);
                    }
                    image["detail"] = detail.clone();
                }
                chat.push(json!({"type":"image_url","image_url":image}));
            } else {
                chat.push(json!({"type":"text","text":content(&json!([part]), role)?}));
            }
        }
        return Ok(chat.into());
    }
    Ok(content(value, role)?.into())
}

pub(super) fn to_chat(body: &Value, codex: bool) -> Result<(Value, ChatTools), StatusCode> {
    object_keys(
        body,
        &[
            "model",
            "input",
            "instructions",
            "max_output_tokens",
            "stream",
            "temperature",
            "top_p",
            "tools",
            "tool_choice",
            "parallel_tool_calls",
            "store",
            "previous_response_id",
            "text",
        ],
    )?;
    if body.get("previous_response_id").is_some_and(|v| !v.is_null())
        || body.get("store").is_some_and(|v| v != false)
    {
        return Err(UNSUPPORTED);
    }
    let mut output = json!({"model":body["model"],"messages":[]});
    for key in ["stream", "temperature", "top_p", "parallel_tool_calls"] {
        if let Some(value) = body.get(key) {
            output[key] = value.clone();
        }
    }
    for key in ["stream", "parallel_tool_calls"] {
        if output.get(key).is_some_and(|v| !v.is_boolean()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if output["stream"] == true {
        output["stream_options"] = json!({"include_usage":true});
    }
    if let Some(max) = body.get("max_output_tokens") {
        output["max_tokens"] = max.clone();
    }
    if let Some(text) = body.get("text") {
        object_keys(text, if codex { &["format", "verbosity"] } else { &["format"] })?;
        if let Some(format) = text.get("format") {
            match string(format, "type")? {
                "text" => {
                    object_keys(format, &["type"])?;
                }
                "json_object" => {
                    object_keys(format, &["type"])?;
                    output["response_format"] = format.clone();
                }
                "json_schema" => {
                    object_keys(format, &["type", "name", "description", "schema", "strict"])?;
                    if !format["schema"].is_object() {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    let mut schema = format.clone();
                    schema.as_object_mut().unwrap().remove("type");
                    output["response_format"] = json!({"type":"json_schema","json_schema":schema});
                }
                _ => return Err(UNSUPPORTED),
            }
        } else if !codex {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let mut tools = ChatTools::default();
    if let Some(definitions) = body.get("tools") {
        output["tools"] = tools.definitions(definitions)?;
    }
    let mut messages = Vec::new();
    if let Some(instructions) = body.get("instructions").filter(|v| !v.is_null()) {
        messages.push(json!({"role":"system","content":instructions.as_str().ok_or(StatusCode::BAD_REQUEST)?}));
    }
    if let Some(input) = body["input"].as_str() {
        messages.push(json!({"role":"user","content":input}));
    } else {
        for item in body["input"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
            match item["type"].as_str() {
                None | Some("message") => {
                    object_keys(item, &["type", "id", "role", "content", "status", "phase"])?;
                    let role = string(item, "role")?;
                    if !matches!(role, "system" | "developer" | "user" | "assistant") {
                        return Err(UNSUPPORTED);
                    }
                    let content = chat_content(&item["content"], role, codex)?;
                    let role = if codex && role == "developer" { "system" } else { role };
                    if codex
                        && role == "assistant"
                        && let Some(last) = messages.last_mut().filter(|m| {
                            m["role"] == "assistant"
                                && m["content"].is_null()
                                && m.get("tool_calls").is_none()
                        })
                    {
                        last["content"] = content;
                    } else {
                        messages.push(json!({"role":role,"content":content}));
                    }
                }
                Some("reasoning") if codex => {
                    // Only plain reasoning produced by Chat can be replayed to Chat.
                    object_keys(
                        item,
                        &["type", "id", "summary", "status", "content", "encrypted_content"],
                    )?;
                    if !item["content"].is_null() || !item["encrypted_content"].is_null() {
                        return Err(UNSUPPORTED);
                    }
                    let mut reasoning = String::new();
                    for part in item["summary"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
                        object_keys(part, &["type", "text"])?;
                        if part["type"] != "summary_text" {
                            return Err(UNSUPPORTED);
                        }
                        reasoning.push_str(string(part, "text")?);
                    }
                    if let Some(last) = messages.last_mut().filter(|m| m["role"] == "assistant") {
                        if let Some(Value::String(previous)) = last.get_mut("reasoning_content") {
                            // The Responses stream can split reasoning around the assistant text.
                            previous.push_str(&reasoning);
                        } else {
                            last["reasoning_content"] = reasoning.into();
                        }
                    } else {
                        messages.push(json!({"role":"assistant","content":null,"reasoning_content":reasoning}));
                    }
                }
                Some("function_call" | "custom_tool_call") => {
                    let call = tools.replay(item)?;
                    if let Some(last) = messages.last_mut().filter(|m| m["role"] == "assistant") {
                        if last.get("tool_calls").is_none() {
                            last["tool_calls"] = json!([]);
                        }
                        last["tool_calls"]
                            .as_array_mut()
                            .ok_or(StatusCode::BAD_REQUEST)?
                            .push(call);
                    } else {
                        messages
                            .push(json!({"role":"assistant","content":null,"tool_calls":[call]}));
                    }
                }
                Some("function_call_output" | "custom_tool_call_output") => {
                    object_keys(item, &["type", "id", "call_id", "output", "status"])?;
                    let output = if codex {
                        content(&item["output"], "assistant")?
                    } else {
                        item["output"].as_str().ok_or(UNSUPPORTED)?.into()
                    };
                    messages.push(json!({"role":"tool","tool_call_id":string(item,"call_id")?,"content":output}));
                }
                _ => return Err(UNSUPPORTED),
            }
        }
    }
    if messages.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    output["messages"] = messages.into();
    if let Some(choice) = body.get("tool_choice") {
        output["tool_choice"] = if let Some(choice) = choice.as_str() {
            if !matches!(choice, "auto" | "none" | "required") {
                return Err(UNSUPPORTED);
            }
            json!(choice)
        } else {
            tools.choice(choice)?
        };
    }
    if codex && output["tools"].as_array().is_none_or(Vec::is_empty) {
        let object = output.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
        if object.get("tool_choice").is_some_and(|choice| choice != "auto" && choice != "none") {
            return Err(UNSUPPORTED);
        }
        for field in ["tools", "tool_choice", "parallel_tool_calls"] {
            object.remove(field);
        }
    }
    Ok((output, tools))
}

pub(super) fn from_chat(
    body: &Value,
    model: &str,
    id: &str,
    tools: &ChatTools,
    codex: bool,
) -> Result<Value, StatusCode> {
    // Preserve the strict Chat validator for all other response fields.
    let mut normalized;
    let mut reasoning = None;
    let body = if codex && let Some(value) = body["choices"][0]["message"].get("reasoning_content")
    {
        reasoning = if value.is_null() {
            None
        } else {
            Some(value.as_str().ok_or(StatusCode::BAD_GATEWAY)?)
        };
        normalized = body.clone();
        normalized["choices"][0]["message"]
            .as_object_mut()
            .ok_or(StatusCode::BAD_GATEWAY)?
            .remove("reasoning_content");
        &normalized
    } else {
        body
    };
    let message = super::messages::from_chat(body, model)?;
    let status = if message["stop_reason"] == "max_tokens" { "incomplete" } else { "completed" };
    let mut output = Vec::new();
    let mut call_ids = HashSet::new();
    for block in message["content"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
        let index = output.len();
        output.push(match block["type"].as_str(){
            Some("text")=>json!({"type":"message","id":format!("{id}_msg_{index}"),"role":"assistant","status":status,"content":[{"type":"output_text","text":block["text"],"annotations":[],"logprobs":[]}]}),
            Some("tool_use") if codex && status == "incomplete" => continue,
            Some("tool_use")=>{
                if !call_ids.insert(string(block, "id")?) {
                    return Err(StatusCode::BAD_GATEWAY);
                }
                // Keep the original JSON argument string, including formatting and number precision.
                let call=body["choices"][0]["message"]["tool_calls"].as_array().and_then(|calls|calls.iter().find(|call|call["id"]==block["id"])).ok_or(StatusCode::BAD_GATEWAY)?;
                tools.output(block, call, format!("{id}_fc_{index}"), status)?
            },
            _=>return Err(UNSUPPORTED)
        });
    }
    if let Some(reasoning) = reasoning.filter(|text| !text.is_empty()) {
        let index = usize::from(output.first().is_some_and(|item| item["type"] == "message"));
        output.insert(index, reasoning_item(&format!("{id}_rs"), reasoning));
    }
    let input = message["usage"]["input_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
    let tokens = message["usage"]["output_tokens"].as_u64().ok_or(StatusCode::BAD_GATEWAY)?;
    let mut usage = json!({"input_tokens":input,"output_tokens":tokens,"total_tokens":input.checked_add(tokens).ok_or(StatusCode::BAD_GATEWAY)?});
    for (source, target) in [
        ("prompt_tokens_details", "input_tokens_details"),
        ("completion_tokens_details", "output_tokens_details"),
    ] {
        if let Some(details) = body["usage"].get(source) {
            usage[target] = details.clone();
        }
    }
    Ok(
        json!({"id":id,"object":"response","created_at":chrono::Utc::now().timestamp(),"model":model,"status":status,"output":output,"error":null,"incomplete_details":if status=="incomplete" {json!({"reason":"max_output_tokens"})}else{Value::Null},"usage":usage,"store":false}),
    )
}

pub(super) fn reasoning_item(id: &str, text: &str) -> Value {
    json!({"type":"reasoning","id":id,"summary":[{"type":"summary_text","text":text}]})
}

pub(super) fn is_native(model: &crate::provider::provider::Model) -> bool {
    model.api.adapter == "openai-responses"
        || matches!(model.api.adapter.as_str(), "openai" | "openai-compatible")
            && reqwest::Url::parse(&model.api.url)
                .is_ok_and(|url| url.path().trim_end_matches('/').ends_with("/responses"))
}
// IDs on fully materialized inputs are client bookkeeping, not server-side lookups.
// Opaque reasoning, citations, hosted items and item_reference still require ownership.
fn is_inline_message_content(value: &Value, role: &str) -> bool {
    if value.is_string() {
        return true;
    }
    let Some(parts) = value.as_array() else { return false };
    parts.iter().all(|part| {
        if part["type"] == "input_image" {
            object_keys(part, &["type", "image_url", "detail"]).is_ok()
                && part["image_url"].as_str().is_some_and(|url| {
                    reqwest::Url::parse(url)
                        .is_ok_and(|url| matches!(url.scheme(), "data" | "http" | "https"))
                })
                && part.get("detail").is_none_or(|v| v.is_null() || v.is_string())
        } else {
            object_keys(part, &["type", "text", "annotations"]).is_ok()
                && (part["type"] == "input_text"
                    || (role == "assistant" && part["type"] == "output_text"))
                && part["text"].is_string()
                && part.get("annotations").is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
        }
    })
}
fn is_complete_inline_item(item: &Value) -> bool {
    if ["status", "phase", "namespace"]
        .iter()
        .any(|key| item.get(*key).is_some_and(|v| !v.is_null() && !v.is_string()))
    {
        return false;
    }
    match item["type"].as_str() {
        None | Some("message") => {
            object_keys(item, &["type", "id", "role", "content", "status", "phase"]).is_ok()
                && item["role"].as_str().is_some_and(|role| {
                    matches!(role, "system" | "developer" | "user" | "assistant")
                        && is_inline_message_content(&item["content"], role)
                })
        }
        Some("function_call") => {
            object_keys(
                item,
                &["type", "id", "call_id", "name", "arguments", "status", "namespace"],
            )
            .is_ok()
                && string(item, "call_id").is_ok()
                && string(item, "name").is_ok()
                && arguments(item).is_ok()
        }
        Some("custom_tool_call") => {
            object_keys(item, &["type", "id", "call_id", "name", "input", "status", "namespace"])
                .is_ok()
                && string(item, "call_id").is_ok()
                && string(item, "name").is_ok()
                && item["input"].is_string()
        }
        Some("function_call_output" | "custom_tool_call_output") => {
            object_keys(item, &["type", "id", "call_id", "output", "status"]).is_ok()
                && string(item, "call_id").is_ok()
                && is_inline_message_content(&item["output"], "assistant")
        }
        _ => false,
    }
}

pub(super) fn native(
    model: &crate::provider::provider::Model,
    identity: &RelayIdentity,
    bytes: &[u8],
    references: &References,
) -> Result<Value, StatusCode> {
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let object = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if object.get("model").and_then(Value::as_str) != Some(identity.model.as_str()) {
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
    // Conversation/item references provide alternate upstream state lookup paths, so reject them.
    if body.get("conversation").is_some_and(|v| !v.is_null())
        || body.get("background").is_some_and(|v| v != false)
    {
        return Err(UNSUPPORTED);
    }
    if body.get("stream").is_some_and(|v| !v.is_boolean())
        || body.get("store").is_some_and(|v| !v.is_boolean())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.get("temperature").is_some() && !model.capabilities.temperature
        || body.get("reasoning").is_some() && !model.capabilities.reasoning
        || body.get("tools").is_some() && !model.capabilities.toolcall
    {
        return Err(UNSUPPORTED);
    }
    if let Some(max) = body.get("max_output_tokens") {
        let max = max.as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST)?;
        if model.limit.output > 0 && max > model.limit.output {
            return Err(UNSUPPORTED);
        }
    }
    let scope = ReferenceScope::Session(identity.clone());
    if let Some(items) = body["input"].as_array_mut() {
        for item in items {
            if let Some(id) = item.get("id") {
                let id = id.as_str().ok_or(StatusCode::BAD_REQUEST)?;
                if !id.is_empty() && is_complete_inline_item(item) {
                    item.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?.remove("id");
                } else {
                    references.check_item(&scope, id)?;
                }
            }
            match item["type"].as_str() {
                Some("item_reference") if !item["id"].is_string() => {
                    return Err(StatusCode::BAD_REQUEST);
                }
                None if !item["role"].is_string() => return Err(UNSUPPORTED),
                Some("reasoning") if !model.capabilities.reasoning => return Err(UNSUPPORTED),
                Some(
                    "function_call"
                    | "function_call_output"
                    | "custom_tool_call"
                    | "custom_tool_call_output",
                ) if !model.capabilities.toolcall => {
                    return Err(UNSUPPORTED);
                }
                _ => {}
            }
            if item["content"].is_string() && !model.capabilities.input.text {
                return Err(UNSUPPORTED);
            }
            if let Some(parts) = item["content"].as_array() {
                for part in parts {
                    let supported = match part["type"].as_str() {
                        Some("input_text" | "output_text") => model.capabilities.input.text,
                        Some("input_image") => model.capabilities.input.image,
                        _ => false,
                    };
                    if !supported {
                        return Err(UNSUPPORTED);
                    }
                }
            }
        }
    } else if !body["input"].is_string() {
        return Err(StatusCode::BAD_REQUEST);
    } else if !model.capabilities.input.text {
        return Err(UNSUPPORTED);
    }
    if let Some(previous) = body.get("previous_response_id").filter(|v| !v.is_null()) {
        body["previous_response_id"] =
            references.resolve(&scope, previous.as_str().ok_or(StatusCode::BAD_REQUEST)?)?.into();
    }
    body["model"] = model.api.id.clone().into();
    Ok(body)
}

pub(super) fn native_output(
    mut body: Value,
    identity: &RelayIdentity,
    id: &str,
    references: &Arc<References>,
    remember: bool,
) -> Result<Value, StatusCode> {
    if body["object"] != "response"
        || !matches!(
            body["status"].as_str(),
            Some("queued" | "in_progress" | "completed" | "incomplete")
        )
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let upstream = string(&body, "id")?.to_owned();
    let scope = ReferenceScope::Session(identity.clone());
    if let Some(previous) = body.get("previous_response_id").filter(|v| !v.is_null()) {
        body["previous_response_id"] =
            references.public(&scope, previous.as_str().ok_or(StatusCode::BAD_GATEWAY)?)?.into();
    }
    if remember {
        for item in body["output"].as_array().ok_or(StatusCode::BAD_GATEWAY)? {
            if let Some(item_id) = item.get("id") {
                let item_id = item_id.as_str().ok_or(StatusCode::BAD_GATEWAY)?;
                references.remember_item(&scope, item_id)?;
            }
        }
        references.remember(&scope, id, &upstream)?;
    }
    body["id"] = id.into();
    body["model"] = identity.model.clone().into();
    Ok(body)
}
pub(super) fn error(status: StatusCode) -> axum::response::Response {
    let kind = match status.as_u16() {
        400 | 413 | 422 => "invalid_request_error",
        401 => "authentication_error",
        403 => "permission_error",
        429 => "rate_limit_error",
        _ => "api_error",
    };
    (status,axum::Json(json!({"error":{"type":kind,"code":if status==UNSUPPORTED{"unsupported_capability"}else{"relay_error"},"param":null,"message":if status==UNSUPPORTED{"Unsupported Responses capability"}else{"Responses relay request failed"}}}))).into_response()
}
