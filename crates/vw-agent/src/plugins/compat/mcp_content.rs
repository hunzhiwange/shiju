//! Preserve the wire result before rmcp's older content types can discard metadata.
use crate::tools::{ToolCallResult, ToolCallTelemetry};
use anyhow::{Context, Result, bail};
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_api_types::tools::ToolResultContentDto;

const WIRE_RESULT: &str = "vibewindow/internal-wire-result";

/// This carrier is replaced at every transport decode, so a server cannot forge its outer layer.
/// It is consumed before any tool result leaves the MCP adapter, never used as authority.
pub(crate) fn preserve_mcp_wire_result(message: &mut Value) {
    if message.get("id").is_none() || message.get("method").is_some() {
        return;
    }
    let Some(result) = message.get_mut("result") else { return };
    if !result.is_object()
        || !(result.get("content").is_some()
            || result.get("structuredContent").is_some()
            || result.get("isError").is_some())
    {
        return;
    }
    let raw = result.take();
    // Keep ordinary peer callers compatible. Empty/malformed results still reach validation below
    // rather than rmcp interpreting them as a different untagged response variant.
    let mut typed = match serde_json::from_value::<CallToolResult>(raw.clone()) {
        Ok(result) => json!({
            "content": result.content,
            "structuredContent": result.structured_content,
            "isError": result.is_error,
        }),
        Err(_) => json!({"content":[{"type":"text","text":""}]}),
    };
    typed["_meta"] = json!({ WIRE_RESULT: raw });
    *result = typed;
}

pub(crate) fn normalize_mcp_tool_result(
    server: &str,
    tool: &str,
    mut typed: CallToolResult,
) -> Result<ToolCallResult> {
    let raw = match typed.meta.as_mut().and_then(|meta| meta.0.remove(WIRE_RESULT)) {
        Some(raw) => raw,
        None => serde_json::to_value(typed).context("failed to encode MCP tool result")?,
    };
    let object = raw.as_object().context("MCP tool result must be an object")?;
    let empty = Vec::new();
    let content = match object.get("content") {
        Some(Value::Array(content)) => content,
        None if object.contains_key("structuredContent") => &empty,
        _ => bail!("MCP tool result content must be an array"),
    };
    for block in content {
        validate_content(block)?;
    }
    let failed = match object.get("isError") {
        Some(Value::Bool(value)) => *value,
        None | Some(Value::Null) => false,
        _ => bail!("MCP tool result isError must be a boolean"),
    };
    if object.get("_meta").is_some_and(|meta| !meta.is_null() && !meta.is_object()) {
        bail!("MCP tool result _meta must be an object");
    }
    let structured = object.get("structuredContent");
    let model_result = model_result(content, structured);
    let mut data = json!({
        "success": !failed,
        "server": server,
        "tool": tool,
        "is_error": failed,
        "content": content,
        "structured_content": structured.unwrap_or(&Value::Null),
        "_meta": object.get("_meta").unwrap_or(&Value::Null),
        "mcp_result": raw,
    });
    if failed {
        let text = text_content(content);
        data["error"] = json!(if text.is_empty() {
            match &model_result {
                Value::String(text) => text.clone(),
                other => serde_json::to_string_pretty(other)?,
            }
        } else {
            text
        });
    }
    let content_blocks = content
        .iter()
        .map(|block| {
            if block.get("type").and_then(Value::as_str) == Some("text")
                && block.as_object().is_some_and(|object| object.len() == 2)
            {
                ToolResultContentDto::Text { text: block["text"].as_str().unwrap().to_owned() }
            } else {
                ToolResultContentDto::Json { value: block.clone() }
            }
        })
        .collect();
    let attributes = BTreeMap::from([
        ("server".to_owned(), json!(server)),
        ("tool".to_owned(), json!(tool)),
        ("content_count".to_owned(), json!(content.len())),
        ("is_error".to_owned(), json!(failed)),
    ]);
    Ok(ToolCallResult {
        data,
        model_result,
        content_blocks,
        telemetry: Some(ToolCallTelemetry {
            success: !failed,
            attributes,
            ..ToolCallTelemetry::default()
        }),
        ..ToolCallResult::default()
    })
}

fn validate_meta(object: &serde_json::Map<String, Value>) -> Result<()> {
    if object.get("_meta").is_some_and(|meta| !meta.is_null() && !meta.is_object()) {
        bail!("MCP content _meta must be an object");
    }
    Ok(())
}

fn string_fields(
    object: &serde_json::Map<String, Value>,
    required: &[&str],
    optional: &[&str],
) -> Result<()> {
    if required.iter().any(|key| !object.get(*key).is_some_and(Value::is_string))
        || optional
            .iter()
            .any(|key| object.get(*key).is_some_and(|value| !value.is_null() && !value.is_string()))
    {
        bail!("invalid MCP content string field");
    }
    Ok(())
}

pub(super) fn validate_content(block: &Value) -> Result<()> {
    let object = block.as_object().context("MCP content block must be an object")?;
    validate_meta(object)?;
    match object.get("type").and_then(Value::as_str) {
        Some("text") => string_fields(object, &["text"], &[])?,
        Some("image" | "audio") => string_fields(object, &["data", "mimeType"], &[])?,
        Some("resource") => {
            let resource = object
                .get("resource")
                .and_then(Value::as_object)
                .context("MCP embedded resource must be an object")?;
            validate_meta(resource)?;
            string_fields(resource, &["uri"], &["mimeType"])?;
            match (resource.get("text"), resource.get("blob")) {
                (Some(Value::String(_)), None) | (None, Some(Value::String(_))) => {}
                _ => bail!("MCP embedded resource requires one text or blob payload"),
            }
        }
        Some("resource_link") => {
            string_fields(object, &["uri", "name"], &["title", "description", "mimeType"])?;
            if object.get("size").is_some_and(|size| !size.is_null() && size.as_u64().is_none()) {
                bail!("MCP resource link size must be a non-negative integer");
            }
        }
        _ => bail!("unsupported MCP tool content block type"),
    }
    if let Some(annotations) = object.get("annotations").filter(|value| !value.is_null()) {
        let annotations = annotations.as_object().context("MCP annotations must be an object")?;
        if let Some(audience) = annotations.get("audience").filter(|value| !value.is_null()) {
            let audience = audience.as_array().context("MCP audience must be an array")?;
            if audience.iter().any(|role| !matches!(role.as_str(), Some("user" | "assistant"))) {
                bail!("invalid MCP content audience");
            }
        }
        if let Some(priority) = annotations.get("priority").filter(|value| !value.is_null()) {
            if !priority.as_f64().is_some_and(|value| (0.0..=1.0).contains(&value)) {
                bail!("MCP content priority must be between zero and one");
            }
        }
        string_fields(annotations, &[], &["lastModified"])?;
    }
    Ok(())
}

fn text_content(content: &[Value]) -> String {
    content
        .iter()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn model_result(content: &[Value], structured: Option<&Value>) -> Value {
    if content.is_empty() {
        return match structured {
            // ToolCallResult uses JSON null for "unset"; keep an explicit null result from
            // falling back to the host-only data/metadata in default_model_result().
            Some(Value::Null) => json!({"structuredContent":null}),
            Some(value) => value.clone(),
            None => json!([]),
        };
    }
    if structured.is_none() && content.iter().all(|block| block["type"] == "text") {
        return json!(text_content(content));
    }
    let visible = content
        .iter()
        .cloned()
        .map(|mut block| {
            // Protocol metadata is host-side data, not model instructions or content.
            if let Some(object) = block.as_object_mut() {
                object.remove("_meta");
            }
            if let Some(resource) = block.get_mut("resource").and_then(Value::as_object_mut) {
                resource.remove("_meta");
            }
            block
        })
        .collect::<Vec<_>>();
    match structured {
        Some(structured) => json!({"content":visible,"structuredContent":structured}),
        None => json!(visible),
    }
}
