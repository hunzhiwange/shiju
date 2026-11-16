//! Historical native search replay conversion; budget integration remains in host tests.

use model_types as provider;
use serde_json::json;

#[path = "convert_test_support.rs"]
mod fixtures;
use fixtures::test_model;

fn compute_request_info(
    model: &provider::Model,
    messages: &serde_json::Value,
) -> Result<super::AisdkRequestInfo, crate::Error> {
    super::compute_request_info(model, messages, &json!({}), false)
}

fn history_search_item(kind: &str, id: &str) -> serde_json::Value {
    let item = if kind == "tool_search_call" {
        json!({"type":kind,"call_id":id,"status":"completed","execution":"client","arguments":{"query":"ledger"}})
    } else {
        json!({"type":kind,"call_id":id,"status":"completed","execution":"client","tools":[]})
    };
    json!({"role":"native","content":item.to_string()})
}

#[test]
fn native_tool_search_history_compaction_keeps_call_output_pair() {
    let model = test_model(
        "fixture",
        "openai-responses",
        "https://example.test/v1/responses",
        "fixture",
        "Fixture",
    );
    let call = history_search_item("tool_search_call", "search");
    let output = history_search_item("tool_search_output", "search");
    let info =
        compute_request_info(&model, &json!([call.clone(), output.clone(), call, output])).unwrap();
    assert_eq!(info.messages.len(), 2, "identical replay must not inject duplicate declarations");
    assert!(
        matches!(&info.messages[1], aisdk::core::Message::ToolSearchOutput(out) if out.call_id.as_deref() == Some("search"))
    );
}

#[test]
fn native_tool_search_history_orphaned_items_are_removed() {
    let model = test_model(
        "fixture",
        "openai-responses",
        "https://example.test/v1/responses",
        "fixture",
        "Fixture",
    );
    for item in [
        history_search_item("tool_search_call", "search"),
        history_search_item("tool_search_output", "search"),
    ] {
        assert!(compute_request_info(&model, &json!([item])).unwrap().messages.is_empty());
    }
}

#[test]
fn native_tool_search_history_conflicting_identity_is_rejected() {
    let model = test_model(
        "fixture",
        "openai-responses",
        "https://example.test/v1/responses",
        "fixture",
        "Fixture",
    );
    let call = history_search_item("tool_search_call", "search");
    let mut altered: serde_json::Value =
        serde_json::from_str(call["content"].as_str().unwrap()).unwrap();
    altered["arguments"]["query"] = json!("other");
    let altered = json!({"role":"native","content":altered.to_string()});
    assert!(
        compute_request_info(
            &model,
            &json!([call, history_search_item("tool_search_output", "search"), altered])
        )
        .is_err()
    );
}

#[test]
fn native_tool_search_history_function_namespace_survives_request_assembly() {
    let model = test_model(
        "fixture",
        "openai-responses",
        "https://example.test/v1/responses",
        "fixture",
        "Fixture",
    );
    let message = json!({"role":"assistant", "tool_calls":[{"id":"function-call", "namespace":"ledger", "type":"function", "function":{"name":"read", "arguments":"{}"}}]});
    let info = compute_request_info(&model, &json!([message])).unwrap();
    let aisdk::core::Message::Assistant(assistant) = &info.messages[0] else { panic!("assistant") };
    let aisdk::core::language_model::LanguageModelResponseContentType::ToolCall(call) =
        &assistant.content
    else {
        panic!("function call")
    };
    assert_eq!(call.tool.namespace.as_deref(), Some("ledger"));
    assert_eq!(call.tool.id, "function-call");
}
