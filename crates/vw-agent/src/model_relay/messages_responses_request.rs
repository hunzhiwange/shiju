//! Resident Messages request projection; session relay validation stays unchanged.
use axum::http::StatusCode;
use serde_json::Value;
use vw_shared::provider::ModelProtocol;

pub(crate) fn to_responses(body: &Value) -> Result<Value, StatusCode> {
    let (normalized, hosted, search) = super::hosted_search::request(body)?;
    let mut chat = super::messages_chat_request::to_chat(&normalized)?;
    // Instructions are request-wide in Responses. Keep source annotation cleanup
    // in the existing Messages owner rather than maintaining a second parser.
    let messages = chat["messages"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?;
    let instructions = if messages.first().is_some_and(|message| message["role"] == "system") {
        Some(messages.remove(0)["content"].clone())
    } else {
        None
    };
    let mut output = super::protocol_bridge::request(ModelProtocol::Responses, &chat)?;
    if let Some(instructions) = instructions {
        output["instructions"] = instructions;
    }
    if let Some(hosted) = hosted {
        output["tools"].as_array_mut().ok_or(StatusCode::BAD_REQUEST)?.push(hosted);
        output["include"] = serde_json::json!(["web_search_call.action.sources"]);
        if let Some(max) = search.max {
            output["max_tool_calls"] = max.into();
        }
        if body["tool_choice"]["type"] == "tool"
            && body["tool_choice"]["name"] == search.name.as_deref().unwrap_or("")
        {
            output["tool_choice"] = serde_json::json!({"type":"web_search"});
        }
    }
    // Preserve the caller's bound; silently increasing a probe budget changes
    // request semantics. Provider-specific minimums belong to its contract.
    Ok(output)
}
