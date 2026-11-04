//! 验证会话选项元数据对空值的过滤及有效值的规范化。
//!
//! 用例只构造会话选项，明确区分缺失、空白和有效元数据。

use crate::types::AcpSessionOptions;

use super::session_meta::build_session_options_meta;

// 下列行为用例确认空载荷被省略，模型和工具值按契约保留。
#[test]
fn session_options_meta_omits_missing_options() {
    assert!(build_session_options_meta(None).is_none());
}

#[test]
fn session_options_meta_omits_whitespace_only_options() {
    let meta = build_session_options_meta(Some(&AcpSessionOptions {
        model: Some("  ".to_string()),
        allowed_tools: Some(vec!["".to_string(), " \t ".to_string()]),
        max_turns: None,
    }));

    assert!(meta.is_none());
}

#[test]
fn session_options_meta_includes_trimmed_tools_and_preserves_model() {
    let meta = build_session_options_meta(Some(&AcpSessionOptions {
        model: Some(" claude-sonnet ".to_string()),
        allowed_tools: Some(vec![" Read ".to_string(), "  ".to_string(), "Bash".to_string()]),
        max_turns: Some(0),
    }))
    .expect("meta");

    let options = &meta["claudeCode"]["options"];
    assert_eq!(options["model"], " claude-sonnet ");
    assert_eq!(options["allowedTools"], serde_json::json!(["Read", "Bash"]));
    assert_eq!(options["maxTurns"], 0);
}

#[test]
fn acp_model_source_claude_sdk_settings_override_model_without_exposing_credentials() {
    use super::session_meta::build_launch_session_meta;
    let mut config = crate::types::AcpAgentConfig::default();
    config.env.extend([
        ("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:opaque".into()),
        ("VW_ACP_CLAUDE_SETTINGS".into(), "/private/settings.json".into()),
        ("ANTHROPIC_MODEL".into(), "selected".into()),
        ("ANTHROPIC_AUTH_TOKEN".into(), "never-in-meta".into()),
    ]);
    let options = AcpSessionOptions {
        model: Some("local".into()),
        allowed_tools: Some(vec!["Read".into()]),
        max_turns: None,
    };
    let meta = build_launch_session_meta(Some(&options), &config).unwrap();
    assert_eq!(meta["claudeCode"]["options"]["settings"], "/private/settings.json");
    assert_eq!(meta["claudeCode"]["options"]["model"], "selected");
    assert_eq!(meta["claudeCode"]["options"]["allowedTools"], serde_json::json!(["Read"]));
    assert!(!serde_json::to_string(&meta).unwrap().contains("never-in-meta"));
    config.env.remove("VW_ACP_LAUNCH_REFERENCE");
    let local = build_launch_session_meta(Some(&options), &config).unwrap();
    assert_eq!(local["claudeCode"]["options"]["model"], "local");
    assert!(local["claudeCode"]["options"].get("settings").is_none());
}
