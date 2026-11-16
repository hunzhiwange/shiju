//! 验证 AI SDK 消息、工具和响应内容之间的双向转换。

use serde_json::json;

use crate::app::agent::provider::provider;

fn compute_request_info(
    model: &provider::Model,
    messages: &serde_json::Value,
) -> Result<super::AisdkRequestInfo, crate::session::llm::types::Error> {
    super::compute_request_info(model, messages, &json!({}), false)
}

#[path = "../convert_test_support.rs"]
mod fixtures;
use fixtures::test_model;

#[test]
fn native_tool_search_request_capability_requires_responses_and_explicit_metadata() {
    for (adapter, endpoint, enabled, expected) in [
        ("openai-responses", "responses", true, true),
        ("openai-compatible", "chat/completions", true, false),
        ("openai-responses", "responses", false, false),
        ("anthropic", "messages", true, false),
    ] {
        let mut model = test_model(
            "fixture",
            adapter,
            &format!("http://localhost/v1/{endpoint}"),
            "gpt-fixture",
            "GPT fixture",
        );
        model.capabilities.native_tool_search = enabled;
        let info = compute_request_info(&model, &json!([])).unwrap();
        let capabilities = provider::resolve_native_search_capabilities(
            &model.capabilities,
            adapter,
            info.path_override.as_deref().unwrap_or("chat/completions"),
        );
        assert_eq!(capabilities.native_tool_search, expected, "{adapter}/{endpoint}/{enabled}");
    }
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
fn native_tool_search_history_namespaced_schema_is_preserved_and_budgeted() {
    let model = test_model(
        "fixture",
        "openai-responses",
        "https://example.test/v1/responses",
        "fixture",
        "Fixture",
    );
    let call = history_search_item("tool_search_call", "search");
    let mut output: serde_json::Value = serde_json::from_str(
        history_search_item("tool_search_output", "search")["content"].as_str().unwrap(),
    )
    .unwrap();
    output["tools"] = json!([{"type":"namespace", "name":"ledger", "description":"trusted at search time", "tools":[{"type":"function", "name":"read", "description":"x".repeat(20000), "parameters":{"type":"object","properties":{}}}]}]);
    let history = vec![call, json!({"role":"native", "content":output.to_string()})];
    let info = compute_request_info(&model, &json!(history)).unwrap();
    let aisdk::core::Message::ToolSearchOutput(result) = &info.messages[1] else {
        panic!("dedicated result")
    };
    let tools = serde_json::to_value(&result.tools).unwrap();
    assert_eq!(tools[0]["name"], "ledger");
    assert_eq!(tools[0]["tools"][0]["name"], "read");
    assert_eq!(result.call_id.as_deref(), Some("search"));
    let usage = crate::session::processor::context_governance::context_usage_tokens(
        &history,
        &[],
        &[],
        None,
    );
    assert!(usage > 4000, "all historical schemas must count toward compaction");
    assert!(
        super::native_tool_id(&std::collections::HashMap::new(), Some("ledger"), "read").is_err(),
        "old schemas cannot authorize the current catalog"
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
