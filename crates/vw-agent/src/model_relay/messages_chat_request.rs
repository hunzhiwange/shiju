//! Resident source adaptations; the session relay keeps its strict contract.
use axum::http::StatusCode;
use serde_json::{Value, json};

pub(crate) fn to_chat(body: &Value) -> Result<Value, StatusCode> {
    // Validate the original blocks before normalizing source annotations.
    let mut normalized = body.clone();
    let media = super::tool_media::messages(&mut normalized);
    if let Some(choice) = body.get("tool_choice").and_then(Value::as_str) {
        if !matches!(choice, "auto" | "any" | "none") {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        normalized["tool_choice"] = json!({"type":choice});
    }
    let mut chat = super::messages::to_chat(&normalized)?;
    if let Some(system) = body.get("system") {
        let parts: Vec<&str> = if let Some(text) = system.as_str() {
            vec![text]
        } else {
            system
                .as_array()
                .ok_or(StatusCode::BAD_REQUEST)?
                .iter()
                .map(|block| block["text"].as_str().ok_or(StatusCode::BAD_REQUEST))
                .collect::<Result<_, _>>()?
        };
        let content = parts
            .into_iter()
            .map(strip_billing)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let messages = chat["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
        if content.is_empty() {
            messages.remove(0);
        } else {
            messages[0]["content"] = content.into();
        }
    }
    if let Some(tools) = chat.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools {
            clean_schema(&mut tool["function"]["parameters"], true);
        }
    }
    super::tool_media::attach(&mut chat, media);
    Ok(chat)
}

fn strip_billing(text: &str) -> &str {
    if !text.starts_with("x-anthropic-billing-header:") {
        return text;
    }
    let Some(end) = text.find(['\r', '\n']) else {
        return "";
    };
    let rest = &text[end..];
    let rest =
        rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix(['\r', '\n'])).unwrap_or(rest);
    rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix(['\r', '\n'])).unwrap_or(rest)
}

fn clean_schema(schema: &mut Value, root: bool) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    if root && !object.contains_key("type") {
        object.insert("type".into(), json!("object"));
        object.entry("properties").or_insert_with(|| json!({}));
    }
    if object.get("format").and_then(Value::as_str) == Some("uri") {
        object.remove("format");
    }
    if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
        for value in properties.values_mut() {
            clean_schema(value, false);
        }
    }
    if let Some(items) = object.get_mut("items") {
        clean_schema(items, false);
    }
}
