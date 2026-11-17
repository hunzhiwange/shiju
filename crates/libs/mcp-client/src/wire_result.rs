//! Preserve protocol payloads before decoding older typed wire models.
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

pub const WIRE_RESULT: &str = "vibewindow/internal-wire-result";

/// This carrier is replaced at every transport decode, so a server cannot forge its outer layer.
/// It is consumed before any tool result leaves the MCP adapter, never used as authority.
pub fn preserve_mcp_wire_result(message: &mut Value) {
    // rmcp 0.8 has only the form request type; retain URL params in the raw request slot.
    if message["method"] == "elicitation/create"
        && message.pointer("/params/mode").and_then(Value::as_str) == Some("url")
    {
        message["params"]["requestedSchema"] = serde_json::json!({"type":"object","properties":{}});
    }

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
    // SSE is decoded again from text through rmcp's untagged ServerResult.
    // With arbitrary_precision, numeric annotations can pass from_value but
    // make that decoder fall through to EmptyResult. Keep the complete raw
    // payload in the carrier for plugin-mcp's validation and restoration.
    if !matches!(
        serde_json::from_str::<rmcp::model::ServerJsonRpcMessage>(&message.to_string()),
        Ok(rmcp::model::ServerJsonRpcMessage::Response(response))
            if matches!(response.result, rmcp::model::ServerResult::CallToolResult(_))
    ) {
        message["result"]["content"] = json!([{"type":"text","text":""}]);
    }
}
