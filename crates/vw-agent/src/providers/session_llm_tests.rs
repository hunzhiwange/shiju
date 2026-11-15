//! Cross-host native-history integration retained with its runtime owner.
use super::SessionLlmProvider;
use model_types::ChatMessage;

#[test]
fn native_tool_search_core_loop_gateway_keeps_typed_history() {
    let item = serde_json::json!({"type":"tool_search_output", "call_id":"search-id", "status":"completed", "execution":"client", "tools":[]});
    let projected = SessionLlmProvider::build_history_messages(&[ChatMessage {
        role: "native".into(),
        content: item.to_string(),
    }]);
    let request = SessionLlmProvider::gateway_chat_request(projected, "fixture-model");
    let item: crate::session::llm::native_tools::NativeHistoryItem =
        serde_json::from_str(request.messages[0]["content"].as_str().unwrap()).unwrap();
    assert!(
        matches!(item.into_message(), aisdk::core::Message::ToolSearchOutput(output) if output.call_id.as_deref() == Some("search-id") && output.tools.is_empty())
    );
}
