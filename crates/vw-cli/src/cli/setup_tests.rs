use super::setup::{
    build_tool_descriptions, compact_context_max_chars, composio_credentials, resolve_setup_name,
    setup_channel_name,
};

fn has_tool(descriptions: &[(&str, &str)], name: &str) -> bool {
    descriptions.iter().any(|(tool_name, _)| *tool_name == name)
}

#[test]
fn resolve_setup_name_prefers_override() {
    let name = resolve_setup_name(Some("override"), Some("configured"), "fallback");

    assert_eq!(name, "override");
}

#[test]
fn resolve_setup_name_uses_configured_value_without_override() {
    let name = resolve_setup_name(None, Some("configured"), "fallback");

    assert_eq!(name, "configured");
}

#[test]
fn resolve_setup_name_uses_fallback_without_configured_value() {
    let name = resolve_setup_name(None, None, "fallback");

    assert_eq!(name, "fallback");
}

#[test]
fn composio_credentials_preserve_values_when_enabled() {
    let credentials = composio_credentials(true, Some("test-key"), "test-entity");

    assert_eq!(credentials, (Some("test-key"), Some("test-entity")));
}

#[test]
fn composio_credentials_reject_values_when_disabled() {
    let credentials = composio_credentials(false, Some("test-key"), "test-entity");

    assert_eq!(credentials, (None, None));
}

#[test]
fn composio_credentials_keep_entity_without_optional_api_key() {
    let credentials = composio_credentials(true, None, "test-entity");

    assert_eq!(credentials, (None, Some("test-entity")));
}

#[test]
fn resolve_setup_name_preserves_explicit_empty_override() {
    let name = resolve_setup_name(Some(""), Some("configured"), "fallback");

    assert_eq!(name, "");
}

#[test]
fn tool_descriptions_always_include_core_tools() {
    let descriptions = build_tool_descriptions(false, false, false);

    for name in [
        "shell",
        "file_read",
        "file_write",
        "memory_store",
        "cron_add",
        "screenshot",
        "schedule",
        "model_routing_config",
    ] {
        assert!(has_tool(&descriptions, name), "missing core tool description: {name}");
    }
}

#[test]
fn tool_descriptions_include_enabled_optional_tools() {
    let descriptions = build_tool_descriptions(true, true, true);

    assert!(has_tool(&descriptions, "browser_open"));
    assert!(has_tool(&descriptions, "composio"));
    assert!(has_tool(&descriptions, "AgentTool"));
}

#[test]
fn tool_descriptions_omit_disabled_optional_tools() {
    let descriptions = build_tool_descriptions(false, false, false);

    assert!(!has_tool(&descriptions, "browser_open"));
    assert!(!has_tool(&descriptions, "composio"));
    assert!(!has_tool(&descriptions, "AgentTool"));
}

#[test]
fn tool_descriptions_enable_optional_tools_independently() {
    let browser_only = build_tool_descriptions(true, false, false);
    assert!(has_tool(&browser_only, "browser_open"));
    assert!(!has_tool(&browser_only, "composio"));
    assert!(!has_tool(&browser_only, "AgentTool"));

    let composio_only = build_tool_descriptions(false, true, false);
    assert!(!has_tool(&composio_only, "browser_open"));
    assert!(has_tool(&composio_only, "composio"));
    assert!(!has_tool(&composio_only, "AgentTool"));

    let agent_only = build_tool_descriptions(false, false, true);
    assert!(!has_tool(&agent_only, "browser_open"));
    assert!(!has_tool(&agent_only, "composio"));
    assert!(has_tool(&agent_only, "AgentTool"));
}

#[test]
fn bootstrap_max_chars_limits_compact_context() {
    assert_eq!(compact_context_max_chars(true), Some(6000));
}

#[test]
fn bootstrap_max_chars_leaves_full_context_unlimited() {
    assert_eq!(compact_context_max_chars(false), None);
}

#[test]
fn channel_name_distinguishes_interactive_and_daemon_modes() {
    assert_eq!(setup_channel_name(true), "cli");
    assert_eq!(setup_channel_name(false), "daemon");
}
