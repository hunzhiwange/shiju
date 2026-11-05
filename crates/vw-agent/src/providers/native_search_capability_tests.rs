use crate::provider::provider::resolve_native_search_capabilities;
use serde_json::json;
use vw_shared::provider::state::resolve_model_capabilities;

#[test]
fn native_tool_search_capability_ordinary_toolcall_does_not_enable_native_search() {
    let model = resolve_model_capabilities(&json!({"tool_call": true}), None, false);
    assert!(
        !resolve_native_search_capabilities(&model, "openai-compatible", "responses")
            .native_tool_search
    );
}

#[test]
fn native_tool_search_capability_explicit_declaration_requires_responses_and_toolcall() {
    let mut model = resolve_model_capabilities(&json!({"native_tool_search": true}), None, false);
    assert!(
        resolve_native_search_capabilities(&model, "openai-responses", "responses")
            .native_tool_search
    );
    for path in ["chat/completions", "chat/stream", ""] {
        assert!(
            !resolve_native_search_capabilities(&model, "openai-responses", path)
                .native_tool_search
        );
    }
    for adapter in ["anthropic", "acp", "google", "unknown"] {
        assert!(
            !resolve_native_search_capabilities(&model, adapter, "responses").native_tool_search
        );
    }
    model.toolcall = false;
    assert!(
        !resolve_native_search_capabilities(&model, "openai-responses", "responses")
            .native_tool_search
    );
}

#[test]
fn native_tool_search_capability_unknown_and_disabled_remain_ordinary() {
    for cfg in
        [json!({}), json!({"native_tool_search": false}), json!({"native_tool_search": "true"})]
    {
        let model = resolve_model_capabilities(&cfg, None, false);
        assert!(
            !resolve_native_search_capabilities(&model, "openai-responses", "responses")
                .native_tool_search
        );
    }
}
