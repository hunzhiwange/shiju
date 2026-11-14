//! Resident Anthropic JSON differences; tool identities retain the shared decoder.
use super::responses::ChatTools;
use axum::http::StatusCode;
use serde_json::{Value, json};
use vw_shared::provider::ModelProtocol;

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;

pub(super) fn convert(
    body: &Value,
    model: &str,
    id: &str,
    tools: &ChatTools,
) -> Result<Value, StatusCode> {
    let reason = match body["stop_reason"].as_str() {
        Some("end_turn" | "stop_sequence" | "tool_use") => None,
        Some("max_tokens" | "model_context_window_exceeded") => Some("max_output_tokens"),
        Some("refusal") => Some("content_filter"),
        // A paused server-tool turn is not a completed client-tool response.
        _ => return Err(INVALID),
    };
    let mut native = body.clone();
    if reason.is_some() {
        native["stop_reason"] = "max_tokens".into();
    }
    let chat = super::protocol_bridge::response(ModelProtocol::Messages, &native)?;
    let mut result = super::responses_chat_result::from_chat(&chat, model, id, tools)?;
    let status = if reason.is_some() { "incomplete" } else { "completed" };
    result["status"] = status.into();
    result["incomplete_details"] = reason.map_or(Value::Null, |reason| json!({"reason":reason}));

    // The shared Chat representation merges text. Restore the native order so text
    // on either side of a tool call remains on the correct side of that call.
    let calls = result["output"].as_array().ok_or(INVALID)?.clone();
    let mut output = Vec::new();
    for block in body["content"].as_array().ok_or(INVALID)? {
        let index = output.len();
        match block["type"].as_str() {
            Some("text") => {
                let text = block["text"].as_str().ok_or(INVALID)?;
                let part = if reason == Some("content_filter") {
                    json!({"type":"refusal","refusal":text})
                } else {
                    json!({"type":"output_text","text":text,"annotations":[],"logprobs":[]})
                };
                output.push(json!({"type":"message","id":format!("{id}_msg_{index}"),
                    "role":"assistant","status":status,"content":[part]}));
            }
            Some("tool_use") => {
                let mut call = calls
                    .iter()
                    .find(|call| call["call_id"] == block["id"])
                    .cloned()
                    .ok_or(INVALID)?;
                call["id"] = format!("{id}_fc_{index}").into();
                call["status"] = status.into();
                output.push(call);
            }
            _ => return Err(INVALID),
        }
    }
    result["output"] = output.into();
    if let Some(write) = body["usage"].get("cache_creation_input_tokens") {
        let write = write.as_u64().ok_or(INVALID)?;
        if !result["usage"]["input_tokens_details"].is_object() {
            result["usage"]["input_tokens_details"] = json!({"cached_tokens":0});
        }
        result["usage"]["input_tokens_details"]["cache_write_tokens"] = write.into();
    }
    if let Some(details) = body["usage"].get("output_tokens_details") {
        let thinking = details["thinking_tokens"].as_u64().ok_or(INVALID)?;
        result["usage"]["output_tokens_details"] = json!({"reasoning_tokens":thinking});
    }
    Ok(result)
}
