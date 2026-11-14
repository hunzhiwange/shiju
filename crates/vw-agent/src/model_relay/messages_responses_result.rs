//! Resident Responses result differences; reuse the shared text/tool decoder.
use axum::http::StatusCode;
use serde_json::{Value, json};
use vw_shared::provider::ModelProtocol;

const INVALID: StatusCode = StatusCode::BAD_GATEWAY;

pub(crate) fn from_responses(body: &Value, model: &str) -> Result<Value, StatusCode> {
    with_search(body, model, &super::hosted_search::Search::default())
}

pub(crate) fn with_search(
    body: &Value,
    model: &str,
    search: &super::hosted_search::Search,
) -> Result<Value, StatusCode> {
    if body.get("error").is_some_and(|error| !error.is_null()) {
        return Err(INVALID);
    }
    let truncated = match body["status"].as_str() {
        Some("completed") => false,
        Some("incomplete")
            if matches!(
                body["incomplete_details"]["reason"].as_str(),
                Some("max_output_tokens" | "max_tokens")
            ) =>
        {
            true
        }
        // Missing/unknown reasons must not become a successful end_turn.
        _ => return Err(INVALID),
    };
    let mut normalized = body.clone();
    if truncated {
        normalized["incomplete_details"]["reason"] = json!("max_output_tokens");
    }
    let mut layout = Vec::new();
    let mut searches = 0;
    for item in normalized["output"].as_array_mut().ok_or(INVALID)? {
        match item["type"].as_str() {
            Some("message") => {
                if item.get("status").is_some_and(|status| {
                    status != "completed" && !(truncated && status == "incomplete")
                }) {
                    return Err(INVALID);
                }
                for part in item["content"].as_array_mut().ok_or(INVALID)? {
                    if part["type"] == "refusal" {
                        super::messages::object_keys(part, &["type", "refusal"])
                            .map_err(|_| INVALID)?;
                        let text = part["refusal"].as_str().ok_or(INVALID)?.to_owned();
                        *part = json!({"type":"output_text","text":text});
                    }
                    let text = part["text"].as_str().ok_or(INVALID)?;
                    if !text.is_empty() {
                        layout.push(Some(json!({"type":"text","text":text})));
                    }
                }
            }
            Some("web_search_call") => {
                searches += 1;
                for block in search.blocks(item, searches)? {
                    layout.push(Some(block));
                }
            }
            Some("function_call") => {
                // Partial tool arguments cannot safely be returned as executable input.
                if item.get("status").is_some_and(|status| status != "completed") {
                    return Err(INVALID);
                }
                layout.push(None);
            }
            _ => return Err(INVALID),
        }
    }
    normalized["output"].as_array_mut().ok_or(INVALID)?.retain(|v| v["type"] != "web_search_call");
    let mut chat = super::protocol_bridge::response(ModelProtocol::Responses, &normalized)
        .map_err(|_| INVALID)?;
    let usage = &body["usage"];
    // Direct cache counters take precedence, including an explicit zero.
    let cached =
        count(usage, &["/cache_read_input_tokens", "/input_tokens_details/cached_tokens"])?;
    let created = count(
        usage,
        &["/cache_creation_input_tokens", "/input_tokens_details/cache_write_tokens"],
    )?;
    chat["usage"] = json!({
        "prompt_tokens":usage["input_tokens"],
        "completion_tokens":usage["output_tokens"],
        "cache_read_input_tokens":cached,
        "cache_creation_input_tokens":created,
    });
    let mut result = super::messages_chat_result::from_chat(&chat, model)?;
    let mut tools = result["content"]
        .as_array()
        .ok_or(INVALID)?
        .iter()
        .filter(|part| part["type"] == "tool_use")
        .cloned();
    let content = layout
        .into_iter()
        .map(|part| part.or_else(|| tools.next()).ok_or(INVALID))
        .collect::<Result<Vec<_>, _>>()?;
    result["content"] = content.into();
    if searches > 0 {
        result["usage"]["server_tool_use"] = json!({"web_search_requests":searches});
    }
    Ok(result)
}

fn count(usage: &Value, paths: &[&str]) -> Result<u64, StatusCode> {
    for path in paths {
        if let Some(value) = usage.pointer(path) {
            return value.as_u64().ok_or(INVALID);
        }
    }
    Ok(0)
}
