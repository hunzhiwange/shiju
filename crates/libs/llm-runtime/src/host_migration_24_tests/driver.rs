//! Native tool search dependency contracts retained outside the driver migration.

use aisdk::core::Message;
use serde_json::json;

#[test]
fn native_tool_search_dependency_locked_dependency_exposes_native_contract() {
    use aisdk::core::{NativeToolSpec, ToolSearchCall, ToolSearchExecution, ToolSearchOutput};
    let declaration = NativeToolSpec::ToolSearch {
        execution: ToolSearchExecution::Client,
        description: "Find tools".into(),
        parameters: serde_json::from_value(json!({"type": "object"})).unwrap(),
    };
    assert_eq!(serde_json::to_value(declaration).unwrap()["type"], "tool_search");
    let call: ToolSearchCall = serde_json::from_value(json!({
        "call_id": "search_1", "execution": "client", "arguments": {"query": "read"}
    }))
    .unwrap();
    assert_eq!(call.arguments["query"], "read");
    let output: ToolSearchOutput = serde_json::from_value(json!({
        "call_id": "search_1", "status": "completed", "execution": "client", "tools": []
    }))
    .unwrap();
    assert_eq!(output.call_id, call.call_id);
    assert!(matches!(Message::ToolSearchOutput(output), Message::ToolSearchOutput(_)));
}

#[test]
fn native_tool_search_dependency_rejects_unknown_execution_and_ordinary_function_results() {
    use aisdk::core::{ToolSearchExecution, ToolSearchOutput};
    assert!(serde_json::from_value::<ToolSearchExecution>(json!("unknown")).is_err());
    assert!(
        serde_json::from_value::<ToolSearchOutput>(json!({
            "call_id": "search_1", "output": "[]"
        }))
        .is_err()
    );
}
