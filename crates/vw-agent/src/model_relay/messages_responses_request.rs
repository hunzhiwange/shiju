//! Resident Messages request projection; session relay validation stays unchanged.
use axum::http::StatusCode;
use serde_json::Value;
use vw_shared::provider::ModelProtocol;

pub(crate) fn to_responses(body: &Value) -> Result<Value, StatusCode> {
    let mut chat = super::messages_chat_request::to_chat(body)?;
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
    // Preserve the caller's bound; silently increasing a probe budget changes
    // request semantics. Provider-specific minimums belong to its contract.
    Ok(output)
}
