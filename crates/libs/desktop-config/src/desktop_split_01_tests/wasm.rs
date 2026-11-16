#![cfg(target_arch = "wasm32")]
fn access() -> crate::ConfigAccess {
    crate::ConfigAccess::new(gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1), None)
}
#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_project_preferences_are_safe_noops() {
    assert_eq!(access().load_project_chat_preferences("/project"), None);
    access().save_project_chat_preferences("/project", "model", true, Some("agent"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_sync_app_config_uses_safe_empty_fallback() {
    assert_eq!(access().load_app_config(), serde_json::json!({}));
    access().save_app_config(&serde_json::json!({"theme":"dark"}));
    access().set_config_field("theme", serde_json::json!("light"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn wasm_tool_content_wrappers_are_safe_empty_noops() {
    assert_eq!(access().load_json_tool_content(), "");
    assert_eq!(access().load_sql_tool_content(), "");
    assert_eq!(access().load_html_tool_content(), "");
    access().save_json_tool_content("{}");
    access().save_sql_tool_content("select 1");
    access().save_html_tool_content("<main></main>");
}
