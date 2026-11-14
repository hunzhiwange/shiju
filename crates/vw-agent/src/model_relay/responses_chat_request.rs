//! Resident Responses request differences; the session converter remains strict.
use super::responses::{self, ChatTools};
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::collections::HashSet;

pub(super) fn convert(mut body: Value) -> Result<(Value, ChatTools), StatusCode> {
    let search = super::tool_namespace::normalize(&mut body)?;
    let effort = responses::codex_chat_effort(&mut body)?;
    if let Some(items) = body["input"].as_array_mut() {
        super::responses_history::preserve_invalid(items);
        let mut pending = HashSet::new();
        let mut deferred = Vec::new();
        let mut normalized = Vec::new();
        for mut item in std::mem::take(items) {
            match item["type"].as_str() {
                Some("function_call" | "custom_tool_call") => {
                    let id = item["call_id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .ok_or(StatusCode::BAD_REQUEST)?;
                    if !pending.insert(id.to_owned()) {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
                Some("function_call_output" | "custom_tool_call_output") => {
                    let id = item["call_id"].as_str().unwrap_or("");
                    if !pending.remove(id) {
                        let marker = if id.is_empty() {
                            "[tool_result without a matching tool_call]".to_owned()
                        } else {
                            format!("[tool_result without a matching tool_call] (call_id: {id})")
                        };
                        let output = match item.get("output") {
                            Some(Value::String(text)) => text.clone(),
                            Some(value) => value.to_string(),
                            None => String::new(),
                        };
                        item = json!({"role":"user","content":format!("{marker}\n{output}")});
                        deferred.push(item);
                        continue;
                    }
                }
                Some("reasoning") => {}
                _ => {
                    // Never put a user turn inside an unfinished parallel tool round.
                    if !pending.is_empty()
                        && item["role"] == "assistant"
                        && item["content"].as_str().is_some_and(|text| {
                            text.starts_with(
                                "[Historical tool call with invalid arguments; context only,",
                            )
                        })
                    {
                        deferred.push(item);
                        continue;
                    }
                    if !pending.is_empty() {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }
                    normalized.append(&mut deferred);
                }
            }
            normalized.push(item);
            if pending.is_empty() {
                normalized.append(&mut deferred);
            }
        }
        if !pending.is_empty() {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        normalized.append(&mut deferred);
        *items = normalized;
    }
    let (mut chat, mut tools) = responses::to_chat(&body, true)?;
    if search {
        tools.mark_search();
    }
    if let Some(effort) = effort {
        chat["reasoning_effort"] = effort.into();
    }
    Ok((chat, tools))
}
