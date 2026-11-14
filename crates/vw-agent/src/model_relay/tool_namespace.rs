//! Resident-only tool-search differences. Namespace/custom mapping stays in ChatTools.
use axum::http::StatusCode;
use serde_json::{Value, json};

const UNSUPPORTED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

pub(super) fn normalize(body: &mut Value) -> Result<bool, StatusCode> {
    let mut definitions = match body.get("tools") {
        Some(value) => value.as_array().ok_or(StatusCode::BAD_REQUEST)?.clone(),
        None => Vec::new(),
    };
    // Discovered tools are declarations supplied by the client, never executed here.
    if let Some(items) = body["input"].as_array() {
        for item in items {
            if item["type"] == "tool_search_output" {
                if let Some(tools) = item.get("tools") {
                    definitions.extend(tools.as_array().ok_or(StatusCode::BAD_REQUEST)?.clone());
                }
            }
        }
    }
    let mut search = false;
    for tool in &mut definitions {
        if tool["type"] != "tool_search" {
            continue;
        }
        super::messages::object_keys(tool, &["type", "execution", "description", "parameters"])?;
        if search || tool.get("execution").is_some_and(|v| v != "client") {
            return Err(UNSUPPORTED);
        }
        search = true;
        let parameters = tool.get("parameters").cloned().unwrap_or_else(|| {
            json!({
                "type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},
                "required":["query"]
            })
        });
        *tool = json!({"type":"function","name":"tool_search",
            "description":tool.get("description").cloned().unwrap_or_else(|| json!("Search and load client tools and namespaces.")),
            "parameters":parameters});
    }
    // ChatTools rejects duplicate names, including a function named tool_search.
    if !definitions.is_empty() {
        body["tools"] = definitions.into();
    }
    if body["tool_choice"]["type"] == "tool_search" {
        super::messages::object_keys(&body["tool_choice"], &["type"])?;
        if !search {
            return Err(UNSUPPORTED);
        }
        body["tool_choice"] = json!({"type":"function","name":"tool_search"});
    }
    if let Some(items) = body["input"].as_array_mut() {
        for item in items {
            if item["type"] == "tool_search_call" {
                super::messages::object_keys(
                    item,
                    &["type", "id", "call_id", "status", "execution", "arguments"],
                )?;
                if !search || item.get("execution").is_some_and(|v| v != "client") {
                    return Err(UNSUPPORTED);
                }
                let arguments = match &item["arguments"] {
                    Value::String(value) => {
                        serde_json::from_str::<Value>(value).map_err(|_| StatusCode::BAD_REQUEST)?
                    }
                    value => value.clone(),
                };
                if !arguments.is_object() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                item.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?.remove("execution");
                item["type"] = "function_call".into();
                item["name"] = "tool_search".into();
                item["arguments"] = arguments.to_string().into();
            } else if item["type"] == "tool_search_output" {
                super::messages::object_keys(
                    item,
                    &["type", "id", "call_id", "status", "execution", "output", "tools"],
                )?;
                if item.get("execution").is_some_and(|v| v != "client") {
                    return Err(UNSUPPORTED);
                }
                let output = item
                    .get("output")
                    .or_else(|| item.get("tools"))
                    .ok_or(StatusCode::BAD_REQUEST)?;
                let output =
                    output.as_str().map(str::to_owned).unwrap_or_else(|| output.to_string());
                let object = item.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
                object.remove("execution");
                object.remove("tools");
                item["type"] = "function_call_output".into();
                item["output"] = output.into();
            }
        }
    }
    Ok(search)
}
