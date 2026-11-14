//! Resident Responses history differences on top of the shared request encoder.
use super::{messages::object_keys, responses::ChatTools};
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::collections::HashSet;
use vw_shared::provider::ModelProtocol;

const BAD: StatusCode = StatusCode::BAD_REQUEST;
const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

pub(super) fn convert(
    mut body: Value,
    default_max_tokens: u64,
) -> Result<(Value, ChatTools), StatusCode> {
    // Controls, schema validation and wire tool identities have one owner.
    let search = super::tool_namespace::normalize(&mut body)?;
    let effort = super::responses::codex_chat_effort(&mut body)?;
    let input = body.get("input").cloned().ok_or(BAD)?;
    body["input"] = "control validation".into();
    if body.get("max_output_tokens").is_none() {
        body["max_output_tokens"] = default_max_tokens.into();
    }
    let (mut chat, mut tools) = super::responses::to_chat(&body, true)?;
    if search {
        tools.mark_search();
    }
    if let Some(effort) = effort {
        chat["reasoning_effort"] = effort.into();
    }
    let mut out = super::protocol_bridge::request(ModelProtocol::Messages, &chat)?;
    let mut messages = Vec::new();
    let mut system =
        out["system"].as_str().map(|text| text.trim().to_owned()).into_iter().collect::<Vec<_>>();
    let items = match input {
        Value::String(text) => vec![json!({"role":"user","content":text})],
        Value::Array(items) => items,
        _ => return Err(BAD),
    };
    for item in items {
        match item["type"].as_str() {
            None | Some("message") => {
                object_keys(
                    &item,
                    &[
                        "type",
                        "id",
                        "role",
                        "content",
                        "status",
                        "phase",
                        "internal_chat_message_metadata_passthrough",
                    ],
                )?;
                let role = item["role"].as_str().ok_or(BAD)?;
                let parts = content(&item["content"])?;
                if matches!(role, "system" | "developer") {
                    for part in parts {
                        if part["type"] != "text" {
                            return Err(UNSUPPORTED);
                        }
                        system.push(part["text"].as_str().ok_or(BAD)?.trim().into());
                    }
                } else if matches!(role, "user" | "assistant") {
                    for part in parts {
                        push(&mut messages, role, part);
                    }
                } else {
                    return Err(UNSUPPORTED);
                }
            }
            Some("input_text" | "input_image" | "input_file") => {
                for part in content(&json!([item]))? {
                    push(&mut messages, "user", part);
                }
            }
            Some("function_call" | "custom_tool_call") => {
                if item["status"] == "incomplete" {
                    continue;
                }
                let call = tools.replay(&item)?;
                let args: Value =
                    serde_json::from_str(call["function"]["arguments"].as_str().ok_or(BAD)?)
                        .map_err(|_| BAD)?;
                if !args.is_object() {
                    return Err(BAD);
                }
                push(
                    &mut messages,
                    "assistant",
                    json!({"type":"tool_use", "id":call["id"],
                    "name":call["function"]["name"], "input":args}),
                );
            }
            Some("function_call_output" | "custom_tool_call_output") => {
                object_keys(&item, &["type", "id", "call_id", "output", "status"])?;
                let id = item["call_id"].as_str().filter(|s| !s.is_empty()).ok_or(BAD)?;
                let parts = if let Some(plan) =
                    super::tool_media::plan_chat_tool_output_media(item["output"].clone())
                {
                    let mut parts = vec![json!({"type":"text","text":plan.tool_content})];
                    for part in plan.media_parts {
                        let input = match part["type"].as_str() {
                            Some("image_url") => {
                                json!({"type":"input_image","image_url":part["image_url"]["url"]})
                            }
                            Some("file") => {
                                let mut file = part["file"].clone();
                                file["type"] = "input_file".into();
                                file
                            }
                            _ => return Err(UNSUPPORTED),
                        };
                        parts.extend(content(&json!([input]))?);
                    }
                    parts
                } else {
                    content(&item["output"])?
                };
                push(
                    &mut messages,
                    "user",
                    json!({"type":"tool_result", "tool_use_id":id,"content":parts}),
                );
            }
            Some("compaction" | "compaction_summary" | "context_compaction") => {
                object_keys(&item, &["type", "id", "encrypted_content"])?;
                if item.get("encrypted_content").is_some_and(|v| !v.is_null() && !v.is_string()) {
                    return Err(BAD);
                }
                if let Some(text) = vw_agent_apps::routing::compaction_replay::replay_item(&item) {
                    push(&mut messages, "user", json!({"type":"text", "text":text}));
                }
            }
            Some("reasoning") => {
                if let Some(block) = item["encrypted_content"]
                    .as_str()
                    .and_then(super::reasoning_bridge::decode_anthropic_thinking_block)
                {
                    push(&mut messages, "assistant", block);
                }
                // Foreign ciphertext is never decoded as an Anthropic block.
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    messages = complete_turns(messages);
    if let Some(last) = messages.last_mut().filter(|m| m["role"] == "assistant") {
        if let Some(parts) = last["content"].as_array_mut() {
            if let Some(part) = parts.last_mut().filter(|p| p["type"] == "text") {
                part["text"] = part["text"].as_str().ok_or(BAD)?.trim_end().into();
            }
            parts.retain(|p| {
                p["type"] != "text" || p["text"].as_str().is_some_and(|s| !s.is_empty())
            });
        }
    }
    messages.retain(|m| m["content"].as_array().is_some_and(|p| !p.is_empty()));
    if messages.is_empty() {
        return Err(BAD);
    }
    if messages[0]["role"] != "user" {
        messages.insert(0, json!({"role":"user","content":[{"type":"text","text":"(continuing the conversation)"}]}));
    }
    out["messages"] = messages.into();
    out.as_object_mut().ok_or(BAD)?.remove("system");
    system.retain(|s| !s.trim().is_empty());
    if !system.is_empty() {
        out["system"] = system.join("\n\n").into();
    }
    Ok((out, tools))
}

fn push(messages: &mut Vec<Value>, role: &str, block: Value) {
    if let Some(last) = messages.last_mut().filter(|m| m["role"] == role) {
        let parts = last["content"].as_array_mut().expect("constructed content array");
        if block["type"] == "tool_result" {
            let index =
                parts.iter().position(|p| p["type"] != "tool_result").unwrap_or(parts.len());
            parts.insert(index, block);
        } else {
            parts.push(block);
        }
    } else {
        messages.push(json!({"role":role,"content":[block]}));
    }
}

fn ids(message: &Value, kind: &str, field: &str) -> Vec<String> {
    message["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["type"] == kind)
        .map(|p| p[field].as_str().unwrap_or("").to_owned())
        .collect()
}

fn complete_turns(messages: Vec<Value>) -> Vec<Value> {
    let mut result = Vec::new();
    let mut iter = messages.into_iter().peekable();
    while let Some(mut message) = iter.next() {
        let calls = ids(&message, "tool_use", "id");
        if message["role"] == "assistant" && !calls.is_empty() {
            let paired = iter.peek().filter(|m| m["role"] == "user");
            let outputs = paired.map(|m| ids(m, "tool_result", "tool_use_id")).unwrap_or_default();
            let call_set: HashSet<_> = calls.iter().collect();
            let output_set: HashSet<_> = outputs.iter().collect();
            if !calls.iter().any(String::is_empty)
                && call_set.len() == calls.len()
                && output_set.len() == outputs.len()
                && call_set == output_set
            {
                result.push(message);
                result.push(iter.next().expect("paired user"));
            }
            // Otherwise drop the whole assistant turn; the next user is sanitized below.
            continue;
        }
        if message["role"] == "user" {
            message["content"]
                .as_array_mut()
                .expect("constructed content array")
                .retain(|p| p["type"] != "tool_result");
        }
        if message["content"].as_array().is_some_and(|p| !p.is_empty()) {
            result.push(message);
        }
    }
    result
}

pub(super) fn content(value: &Value) -> Result<Vec<Value>, StatusCode> {
    if let Some(text) = value.as_str() {
        return Ok(if text.trim().is_empty() {
            vec![]
        } else {
            vec![json!({"type":"text","text":text})]
        });
    }
    let mut parts = Vec::new();
    for part in value.as_array().ok_or(BAD)? {
        let block = match part["type"].as_str() {
            Some("input_text" | "output_text") => {
                object_keys(part, &["type", "text", "annotations", "logprobs"])?;
                for field in ["annotations", "logprobs"] {
                    if part.get(field).is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty())) {
                        return Err(UNSUPPORTED);
                    }
                }
                let text = part["text"].as_str().ok_or(BAD)?;
                if text.trim().is_empty() {
                    continue;
                }
                json!({"type":"text","text":text})
            }
            Some("input_image") => {
                object_keys(part, &["type", "image_url", "detail"])?;
                if part.get("detail").is_some_and(|v| !matches!(v.as_str(), Some("auto"))) {
                    return Err(UNSUPPORTED);
                }
                let url = part["image_url"]
                    .as_str()
                    .or_else(|| part["image_url"]["url"].as_str())
                    .ok_or(BAD)?;
                json!({"type":"image","source":source(url, true)?})
            }
            Some("input_file") => {
                object_keys(part, &["type", "file_url", "file_data", "filename"])?;
                if part.get("file_url").is_some() && part.get("file_data").is_some() {
                    return Err(BAD);
                }
                let url =
                    part["file_url"].as_str().or_else(|| part["file_data"].as_str()).ok_or(BAD)?;
                let mut block = json!({"type":"document","source":source(url, false)?});
                if let Some(name) = part.get("filename") {
                    block["title"] = name.as_str().ok_or(BAD)?.into();
                }
                block
            }
            _ => return Err(UNSUPPORTED),
        };
        parts.push(block);
    }
    Ok(parts)
}

fn source(url: &str, image: bool) -> Result<Value, StatusCode> {
    if let Some(data) = url.strip_prefix("data:") {
        let (meta, data) = data.split_once(',').ok_or(BAD)?;
        let mime = meta.strip_suffix(";base64").ok_or(UNSUPPORTED)?;
        let supported = if image {
            matches!(mime, "image/png" | "image/jpeg" | "image/webp" | "image/gif")
        } else {
            mime == "application/pdf"
        };
        if data.is_empty() || !supported {
            return Err(UNSUPPORTED);
        }
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| BAD)?;
        return Ok(json!({"type":"base64","media_type":mime,"data":data}));
    }
    let parsed = reqwest::Url::parse(url).map_err(|_| BAD)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(UNSUPPORTED);
    }
    Ok(json!({"type":"url","url":url}))
}
