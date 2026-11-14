//! Resident-only bounded SSE aggregation. Session relay policy is unchanged.
use std::collections::BTreeMap;

use axum::http::StatusCode;
use serde_json::{Value, json};

use super::{generation::GenerationProtocol as P, stream_observer::StreamObserver};

const BAD: StatusCode = StatusCode::BAD_GATEWAY;

/// The caller bounds decoded bytes and owns the request deadline and cancellation.
/// Validate the entire stream, including EOF, before returning any JSON.
pub(crate) fn aggregate(bytes: &[u8], protocol: P, limit: usize) -> Result<Vec<u8>, StatusCode> {
    if bytes.len() > limit {
        return Err(BAD);
    }
    // Resident sources may omit the final blank delimiter, but not a terminal event.
    let mut observer = StreamObserver::new(protocol);
    observer.push(bytes).map_err(|_| BAD)?;
    observer.push(b"\n\n").map_err(|_| BAD)?;
    observer.finish().map_err(|_| BAD)?;
    let mut lines = super::sse_framing::Lines::default();
    let mut parsed = lines.push(bytes, limit).map_err(|_| BAD)?;
    parsed.extend(lines.push(b"\n\n", limit).map_err(|_| BAD)?);
    let mut data = String::new();
    let mut result = Value::Null;
    let mut choices = BTreeMap::<u64, Value>::new();
    let mut tools = BTreeMap::<(u64, u64), Value>::new();
    let mut arguments = BTreeMap::<u64, String>::new();
    for line in parsed {
        if !line.is_empty() {
            if let Some(value) = line.strip_prefix("data:") {
                data.push_str(value.strip_prefix(' ').unwrap_or(value));
                data.push('\n');
            }
            continue;
        }
        if data.is_empty() {
            continue;
        }
        let frame = std::mem::take(&mut data);
        let frame = frame.trim_end_matches('\n');
        if frame == "[DONE]" {
            continue;
        }
        let v: Value = serde_json::from_str(frame).map_err(|_| BAD)?;
        if matches!(v["type"].as_str(), Some("ping" | "heartbeat" | "keepalive")) {
            continue;
        }
        match protocol {
            P::Chat => {
                if result.is_null() {
                    result = json!({"object":"chat.completion","choices":[]});
                }
                for key in ["id", "model", "created", "usage", "system_fingerprint", "service_tier"]
                {
                    if let Some(value) = v.get(key).filter(|v| !v.is_null()) {
                        result[key] = value.clone();
                    }
                }
                for choice in v["choices"].as_array().ok_or(BAD)? {
                    let index = choice["index"].as_u64().ok_or(BAD)?;
                    let out = choices.entry(index).or_insert_with(|| json!({"index":index,"message":{"role":"assistant","content":null},"finish_reason":null}));
                    let delta = choice["delta"].as_object().ok_or(BAD)?;
                    for (key, value) in delta {
                        match key.as_str() {
                            "role" => {}
                            "content" | "refusal" | "reasoning_content" | "reasoning" => {
                                append(&mut out["message"][key], value)?;
                            }
                            "tool_calls" => {
                                for call in value.as_array().ok_or(BAD)? {
                                    let tool_index =
                                        call["index"].as_u64().filter(|i| *i < 1024).ok_or(BAD)?;
                                    let tool = tools.entry((index, tool_index)).or_insert_with(|| json!({"type":"function","function":{"name":"","arguments":""}}));
                                    for field in ["id", "type"] {
                                        if let Some(value) = call.get(field) {
                                            tool[field] = value.clone();
                                        }
                                    }
                                    for field in ["name", "arguments"] {
                                        if let Some(value) = call["function"].get(field) {
                                            append(&mut tool["function"][field], value)?;
                                        }
                                    }
                                }
                            }
                            _ if value.is_null() => {}
                            _ => return Err(BAD), // Never silently drop an unsupported delta.
                        }
                    }
                    if !choice["finish_reason"].is_null() {
                        out["finish_reason"] = choice["finish_reason"].clone();
                    }
                    if let Some(logprobs) = choice.get("logprobs").filter(|v| !v.is_null()) {
                        let target = &mut out["logprobs"];
                        if target.is_null() {
                            *target = json!({});
                        }
                        for (key, values) in logprobs.as_object().ok_or(BAD)? {
                            if values.is_null() {
                                continue;
                            }
                            if target[key].is_null() {
                                target[key] = json!([]);
                            }
                            target[key]
                                .as_array_mut()
                                .ok_or(BAD)?
                                .extend(values.as_array().ok_or(BAD)?.iter().cloned());
                        }
                    }
                }
            }
            P::Responses => {
                if v["type"] == "response.incomplete" {
                    return Err(BAD);
                }
                if v["type"] == "response.completed" {
                    result = v["response"].clone();
                }
            }
            P::Messages => match v["type"].as_str() {
                Some("message_start") => {
                    result = v["message"].clone();
                    result["content"] = json!([]);
                }
                Some("content_block_start") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    let blocks = result["content"].as_array_mut().ok_or(BAD)?;
                    if index != blocks.len() as u64 {
                        return Err(BAD);
                    }
                    blocks.push(v["content_block"].clone());
                }
                Some("content_block_delta") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    let block = result["content"]
                        .as_array_mut()
                        .and_then(|b| b.get_mut(index as usize))
                        .ok_or(BAD)?;
                    let delta = &v["delta"];
                    match delta["type"].as_str() {
                        Some("text_delta") => append(&mut block["text"], &delta["text"])?,
                        Some("thinking_delta") => {
                            append(&mut block["thinking"], &delta["thinking"])?
                        }
                        Some("signature_delta") => {
                            append(&mut block["signature"], &delta["signature"])?
                        }
                        Some("input_json_delta") => arguments
                            .entry(index)
                            .or_default()
                            .push_str(delta["partial_json"].as_str().ok_or(BAD)?),
                        Some("citations_delta") => {
                            if block["citations"].is_null() {
                                block["citations"] = json!([]);
                            }
                            block["citations"]
                                .as_array_mut()
                                .ok_or(BAD)?
                                .push(delta["citation"].clone());
                        }
                        _ => return Err(BAD),
                    }
                }
                Some("content_block_stop") => {
                    let index = v["index"].as_u64().ok_or(BAD)?;
                    if let Some(args) = arguments.remove(&index) {
                        result["content"][index as usize]["input"] =
                            serde_json::from_str::<Value>(&args).map_err(|_| BAD)?;
                    }
                }
                Some("message_delta") => {
                    for (key, value) in v["delta"].as_object().ok_or(BAD)? {
                        result[key] = value.clone();
                    }
                    for (key, value) in v["usage"].as_object().ok_or(BAD)? {
                        result["usage"][key] = value.clone();
                    }
                }
                Some("message_stop") => {}
                _ => return Err(BAD),
            },
            P::Gemini => {
                if result.is_null() {
                    result = json!({"candidates":[]});
                }
                for (key, value) in v.as_object().ok_or(BAD)? {
                    if key != "candidates" {
                        result[key] = value.clone();
                    }
                }
                if let Some(candidates) = v.get("candidates") {
                    for candidate in candidates.as_array().ok_or(BAD)? {
                        let index = candidate.get("index").and_then(Value::as_u64).unwrap_or(0);
                        let out = choices.entry(index).or_insert_with(
                            || json!({"index":index,"content":{"role":"model","parts":[]}}),
                        );
                        for (key, value) in candidate.as_object().ok_or(BAD)? {
                            if key != "content" {
                                out[key] = value.clone();
                            }
                        }
                        if let Some(parts) = candidate["content"].get("parts") {
                            out["content"]["parts"]
                                .as_array_mut()
                                .ok_or(BAD)?
                                .extend(parts.as_array().ok_or(BAD)?.iter().cloned());
                        }
                    }
                }
            }
        }
    }
    for ((index, _), tool) in tools {
        let message = &mut choices.get_mut(&index).ok_or(BAD)?["message"];
        if message["tool_calls"].is_null() {
            message["tool_calls"] = json!([]);
        }
        message["tool_calls"].as_array_mut().ok_or(BAD)?.push(tool);
    }
    match protocol {
        P::Chat => result["choices"] = choices.into_values().collect(),
        P::Gemini => result["candidates"] = choices.into_values().collect(),
        _ => {}
    }
    if !result.is_object() || !arguments.is_empty() {
        return Err(BAD);
    }
    let output = serde_json::to_vec(&result).map_err(|_| BAD)?;
    if output.len() > limit {
        return Err(BAD);
    }
    Ok(output)
}

fn append(target: &mut Value, fragment: &Value) -> Result<(), StatusCode> {
    if fragment.is_null() {
        return Ok(());
    }
    let fragment = fragment.as_str().ok_or(BAD)?;
    if target.is_null() {
        *target = Value::String(String::new());
    }
    target.as_str().ok_or(BAD)?;
    if let Value::String(text) = target {
        text.push_str(fragment);
    }
    Ok(())
}
