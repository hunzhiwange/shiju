//! Hosted search stays on the upstream; this module never executes tools.
use axum::http::StatusCode;
use serde_json::{Value, json};

#[derive(Clone, Default)]
pub(crate) struct Search {
    pub(crate) name: Option<String>,
    pub(crate) max: Option<u64>,
}

pub(crate) fn request(body: &Value) -> Result<(Value, Option<Value>, Search), StatusCode> {
    let mut normalized = body.clone();
    let mut search = Search::default();
    let mut definition = None;
    if let Some(tools) = normalized.get_mut("tools") {
        let tools = tools.as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
        let mut functions = Vec::new();
        for tool in tools.iter() {
            let kind = tool["type"].as_str().unwrap_or("");
            if kind != "web_search" && !kind.starts_with("web_search_") {
                functions.push(tool.clone());
                continue;
            }
            if definition.is_some() || !matches!(kind, "web_search" | "web_search_20250305") {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            super::messages::object_keys(
                tool,
                &[
                    "type",
                    "name",
                    "max_uses",
                    "allowed_domains",
                    "blocked_domains",
                    "user_location",
                ],
            )?;
            search.name = Some(
                tool["name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or(StatusCode::BAD_REQUEST)?
                    .into(),
            );
            search.max = tool
                .get("max_uses")
                .filter(|v| !v.is_null())
                .map(|v| v.as_u64().filter(|n| *n > 0).ok_or(StatusCode::BAD_REQUEST))
                .transpose()?;
            let mut target = json!({"type":"web_search"});
            for field in ["allowed_domains", "blocked_domains"] {
                if let Some(domains) = tool.get(field) {
                    if !domains.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    if target.get("filters").is_none() {
                        target["filters"] = json!({});
                    }
                    target["filters"][field] = domains.clone();
                }
            }
            if let Some(location) = tool.get("user_location") {
                super::messages::object_keys(
                    location,
                    &["type", "city", "region", "country", "timezone"],
                )?;
                if location["type"] != "approximate" {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                for field in ["city", "region", "country", "timezone"] {
                    if location.get(field).is_some_and(|v| !v.is_string()) {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
                target["user_location"] = location.clone();
            }
            definition = Some(target);
        }
        *tools = functions;
    }
    if search.name.as_deref().is_some_and(|name| {
        body["tool_choice"]["type"] == "tool" && body["tool_choice"]["name"] == name
    }) {
        // Keep validation of the parallel control in the ordinary request owner.
        super::messages::object_keys(
            &body["tool_choice"],
            &["type", "name", "disable_parallel_tool_use"],
        )?;
        normalized["tool_choice"]["type"] = json!("any");
        normalized["tool_choice"].as_object_mut().unwrap().remove("name");
    }
    Ok((normalized, definition, search))
}

impl Search {
    pub(crate) fn blocks(&self, item: &Value, ordinal: u64) -> Result<Vec<Value>, StatusCode> {
        let invalid = StatusCode::BAD_GATEWAY;
        let name = self.name.as_ref().ok_or(invalid)?;
        let id = item["id"].as_str().filter(|s| !s.is_empty()).ok_or(invalid)?;
        let status = item["status"].as_str().ok_or(invalid)?;
        if !matches!(status, "completed" | "failed" | "incomplete") {
            return Err(invalid);
        }
        let exceeded = self.max.is_some_and(|max| ordinal > max);
        let failed = status != "completed" || item.get("error").is_some_and(|v| !v.is_null());
        let content = if exceeded || failed {
            let signal = item["error"].to_string().to_ascii_lowercase();
            let code = if exceeded || signal.contains("max_uses") {
                "max_uses_exceeded"
            } else if signal.contains("rate_limit") || signal.contains("too_many_requests") {
                "too_many_requests"
            } else if signal.contains("query_too_long") {
                "query_too_long"
            } else if signal.contains("request_too_large") {
                "request_too_large"
            } else if signal.contains("invalid") {
                "invalid_tool_input"
            } else {
                "unavailable"
            };
            json!({"type":"web_search_tool_result_error","error_code":code})
        } else {
            let mut results = Vec::new();
            if let Some(sources) = item["action"].get("sources") {
                for source in sources.as_array().ok_or(invalid)? {
                    let url = source["url"].as_str().ok_or(invalid)?;
                    results.push(json!({"type":"web_search_result","url":url,"title":source["title"].as_str().unwrap_or(url),"encrypted_content":source.get("encrypted_content").cloned().unwrap_or(json!("")),"page_age":source.get("page_age").cloned().unwrap_or(Value::Null)}));
                }
            }
            json!(results)
        };
        Ok(vec![
            json!({"type":"server_tool_use","id":id,"name":name,"input":item.get("action").cloned().unwrap_or_else(|| json!({}))}),
            json!({"type":"web_search_tool_result","tool_use_id":id,"content":content}),
        ])
    }
}
