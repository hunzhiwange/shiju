//! State-dependent model capability tests retained until model-state migration.

#[test]
fn native_tool_search_capability_ordinary_toolcall_does_not_enable_native_search() {
    let capabilities = super::super::state::default_model_capabilities();
    let mut value = serde_json::to_value(capabilities).unwrap();
    value.as_object_mut().unwrap().remove("native_tool_search");
    let parsed: super::Capabilities = serde_json::from_value(value).unwrap();
    assert!(!parsed.native_tool_search);
}

#[test]
fn native_tool_search_capability_explicit_override_and_inheritance() {
    use super::super::state::resolve_model_capabilities;
    let enabled =
        resolve_model_capabilities(&serde_json::json!({"native_tool_search": true}), None, false);
    assert!(enabled.native_tool_search);
    assert!(
        resolve_model_capabilities(&serde_json::json!({}), Some(&enabled), false)
            .native_tool_search
    );
    for value in [serde_json::json!(false), serde_json::json!("true"), serde_json::Value::Null] {
        assert!(
            !resolve_model_capabilities(
                &serde_json::json!({"native_tool_search": value}),
                Some(&enabled),
                false
            )
            .native_tool_search
        );
    }
}
