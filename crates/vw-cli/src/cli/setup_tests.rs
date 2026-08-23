//! CLI 启动配置解析与工具清单组装的测试。
//! 确认显式覆盖、已配置值和后备值的优先级，包括显式空字符串的语义。
//! 同时校验核心工具恒存、可选工具独立开关、上下文限制和通道命名。

use super::setup::{
    build_tool_descriptions, compact_context_max_chars, resolve_setup_name, setup_channel_name,
};

fn has_tool(descriptions: &[(&str, &str)], name: &str) -> bool {
    descriptions.iter().any(|(tool_name, _)| *tool_name == name)
}

fn tool_description<'a>(descriptions: &'a [(&str, &str)], name: &str) -> &'a str {
    descriptions
        .iter()
        .find_map(|(tool_name, description)| (*tool_name == name).then_some(*description))
        .expect("tool description should exist")
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
fn resolve_setup_name_preserves_explicit_empty_override() {
    let name = resolve_setup_name(Some(""), Some("configured"), "fallback");

    assert_eq!(name, "");
}

#[test]
fn tool_descriptions_always_include_core_tools() {
    let descriptions = build_tool_descriptions(false, false);

    for name in [
        "shell",
        "file_read",
        "file_write",
        "memory_store",
        "cron_add",
        "screenshot",
        "schedule",
    ] {
        assert!(has_tool(&descriptions, name), "missing core tool description: {name}");
    }
}

#[test]
fn tool_descriptions_include_enabled_optional_tools() {
    let descriptions = build_tool_descriptions(true, true);

    assert!(has_tool(&descriptions, "browser_open"));
    assert!(has_tool(&descriptions, "AgentTool"));
    let agent_tool = tool_description(&descriptions, "AgentTool");
    for expected in [
        "complex",
        "simple",
        "parallel",
        "background",
        "run",
        "status",
        "wait",
        "stop",
        "synthesize",
    ] {
        assert!(agent_tool.contains(expected), "missing '{expected}': {agent_tool}");
    }
    for unsupported in ["action=list", "action=get"] {
        assert!(
            !agent_tool.contains(unsupported),
            "unsupported '{unsupported}' advertised: {agent_tool}"
        );
    }
}

#[test]
fn tool_descriptions_omit_disabled_optional_tools() {
    let descriptions = build_tool_descriptions(false, false);

    assert!(!has_tool(&descriptions, "browser_open"));
    assert!(!has_tool(&descriptions, "AgentTool"));
}

#[test]
fn tool_descriptions_enable_optional_tools_independently() {
    let browser_only = build_tool_descriptions(true, false);
    assert!(has_tool(&browser_only, "browser_open"));
    assert!(!has_tool(&browser_only, "AgentTool"));

    let agent_only = build_tool_descriptions(false, true);
    assert!(!has_tool(&agent_only, "browser_open"));
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
