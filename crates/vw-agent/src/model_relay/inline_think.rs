//! Resident Chat delta normalization after bounded UTF-8 SSE framing.
use axum::http::StatusCode;
use serde_json::Value;
use vw_agent_apps::routing::inline_think::InlineThinkSplitter;

pub(super) fn normalize(
    splitter: &mut InlineThinkSplitter,
    chunk: &mut Value,
) -> Result<(), StatusCode> {
    let choices = chunk["choices"].as_array_mut().ok_or(StatusCode::BAD_GATEWAY)?;
    if choices.len() > 1 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    for choice in choices {
        if choice["index"] != 0 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let boundary = !choice["finish_reason"].is_null();
        let delta = choice["delta"].as_object_mut().ok_or(StatusCode::BAD_GATEWAY)?;
        let content = match delta.get("content") {
            Some(Value::String(text)) => text.as_str(),
            None | Some(Value::Null) => "",
            _ => return Err(StatusCode::BAD_GATEWAY),
        };
        let (thought, text) = splitter.push(content);
        let mut thought = thought.unwrap_or_default();
        let mut text = text.unwrap_or_default();
        if boundary || delta.get("tool_calls").is_some_and(|v| !v.is_null()) {
            let (tail_thought, tail_text) = splitter.flush();
            thought.push_str(&tail_thought.unwrap_or_default());
            text.push_str(&tail_text.unwrap_or_default());
        }
        if !thought.is_empty() {
            let existing = match delta.get("reasoning_content") {
                Some(Value::String(value)) => value.as_str(),
                None | Some(Value::Null) => "",
                _ => return Err(StatusCode::BAD_GATEWAY),
            };
            delta.insert("reasoning_content".into(), format!("{existing}{thought}").into());
        }
        if delta.contains_key("content") || !text.is_empty() {
            delta.insert("content".into(), text.into());
        }
    }
    Ok(())
}
