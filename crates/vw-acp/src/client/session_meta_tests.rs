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
