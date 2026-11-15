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
    let candidates: Vec<_> = body["output"]
        .as_array()
        .ok_or(INVALID)?
        .iter()
        .flat_map(super::search_citations::web_search_results_from_output_item)
        .collect();
    let mut attributed = std::collections::HashSet::new();
    for item in normalized["output"].as_array_mut().ok_or(INVALID)? {
        if item["type"] == "web_search_call"
            && item["status"] == "completed"
            && item.get("error").is_none_or(Value::is_null)
        {
            if item.get("action").is_some_and(|v| !v.is_object())
                || item["action"].get("sources").is_some_and(|v| {
                    !v.as_array().is_some_and(|sources| {
                        sources.iter().all(|source| source["url"].is_string())
                    })
                })
            {
                return Err(INVALID);
            }
            let mut results = super::search_citations::web_search_results_from_action(item);
            super::search_citations::merge_web_search_result_metadata(&mut results, &candidates);
            for result in &results {
                if let Some(url) = result["url"].as_str() {
                    attributed.insert(url.to_owned());
                }
            }
            if item.get("action").is_none() {
                item["action"] = json!({});
            }
            item["action"]["sources"] = json!(results);
        }
    }
    if let Some(item) =
        normalized["output"].as_array_mut().ok_or(INVALID)?.iter_mut().find(|item| {
            item["type"] == "web_search_call"
                && item["status"] == "completed"
                && item.get("error").is_none_or(Value::is_null)
        })
    {
        let sources = item["action"]["sources"].as_array_mut().ok_or(INVALID)?;
        for candidate in candidates {
            if let Some(url) = candidate["url"].as_str() {
                if attributed.insert(url.to_owned()) {
                    sources.push(candidate);
                }
            }
        }
    }
    let mut layout = Vec::new();
    let mut searches = 0;
    for item in normalized["output"].as_array_mut().ok_or(INVALID)? {
        match item["type"].as_str() {
            Some("reasoning") => {
                if let Some(block) =
                    super::reasoning_bridge::anthropic_block_from_openai_reasoning_item(item)
                {
                    layout.push(Some(block));
                }
            }
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
                    let text = super::search_citations::output_text_with_url_citations(part)
                        .unwrap_or_else(|| part["text"].as_str().unwrap_or_default().to_owned());
                    if !part["text"].is_string() {
                        return Err(INVALID);
                    }
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
    normalized["output"]
        .as_array_mut()
        .ok_or(INVALID)?
        .retain(|v| v["type"] != "web_search_call" && v["type"] != "reasoning");
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
