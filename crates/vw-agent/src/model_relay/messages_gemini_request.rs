//! Resident Messages → Gemini request differences. No credentials or history store.
use axum::http::StatusCode;
use base64::Engine as _;
use serde_json::{Value, json};
use vw_shared::provider::ModelProtocol;

pub(crate) fn convert(body: &Value) -> Result<Value, StatusCode> {
    let mut normalized = body.clone();
    let mut media_messages = Vec::new();
    let messages = normalized["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
    for (index, message) in messages.iter_mut().enumerate() {
        let Some(blocks) = message["content"].as_array() else { continue };
        if !blocks.iter().any(|b| matches!(b["type"].as_str(), Some("image" | "document"))) {
            continue;
        }
        if message["role"] != "user" {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let mut parts = Vec::new();
        for block in blocks {
            if block["type"] == "text" {
                super::messages::object_keys(block, &["type", "text"])?;
                parts.push(json!({"text":block["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?}));
            } else {
                parts.push(media(block)?);
            }
        }
        media_messages.push((index, parts));
        message["content"] = json!("");
    }
    // Use the existing strict owner for text, generation controls and correlated tools.
    let chat = super::messages_chat_request::to_chat(&normalized)?;
    let mut output = super::protocol_bridge::request(ModelProtocol::Gemini, &chat)?;
    // A source user media message contributes exactly one Chat message. Prefix
    // conversion accounts for tool results expanding earlier source messages.
    for (index, parts) in media_messages {
        let mut prefix = normalized.clone();
        prefix["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?.truncate(index + 1);
        let prefix = super::messages_chat_request::to_chat(&prefix)?;
        let count = prefix["messages"].as_array().ok_or(StatusCode::BAD_REQUEST)?
            .iter().filter(|m| m["role"] != "system").count();
        output["contents"][count - 1]["parts"] = parts.into();
    }
    if let Some(tools) = output["tools"].as_array_mut() {
        for tool in tools {
            if let Some(functions) = tool["functionDeclarations"].as_array_mut() {
                for function in functions {
                    *function = vw_agent_apps::routing::gemini_schema::build_gemini_function_declaration(
                        function["name"].as_str().ok_or(StatusCode::BAD_REQUEST)?,
                        function["description"].as_str(),
                        function["parametersJsonSchema"].clone(),
                    );
                }
            }
        }
    }
    super::native_gemini::prepare(&output)
}

fn media(block: &Value) -> Result<Value, StatusCode> {
    super::messages::object_keys(block, &["type", "source"])?;
    let source = &block["source"];
    super::messages::object_keys(source, &["type", "media_type", "data"])?;
    if source["type"] != "base64" {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mime = source["media_type"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let allowed = match block["type"].as_str() {
        Some("image") => matches!(mime, "image/png" | "image/jpeg" | "image/webp" | "image/gif"),
        Some("document") => mime == "application/pdf",
        _ => false,
    };
    if !allowed { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
    let data = source["data"].as_str().filter(|v| !v.is_empty()).ok_or(StatusCode::BAD_REQUEST)?;
    base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(json!({"inlineData":{"mimeType":mime,"data":data}}))
}
