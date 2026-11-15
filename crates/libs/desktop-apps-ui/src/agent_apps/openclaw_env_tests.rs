use super::*;

#[test]
fn openclaw_env_editor_retains_failed_draft() {
    let draft = r#"{"vars":{"标签":"测试","TOKEN":"secret"},"shellEnv":{"enabled":true}}"#;
    let mut state = State { editor: text_editor::Content::with_text(draft), loaded: true, busy: true, ..State::default() };
    assert!(parse(draft).is_ok());
    for status in [400, 403, 409, 422] {
        state.finish_save(Err(safe_error(OpenClawSettingsError::Http(status))));
        assert_eq!(state.editor.text().trim_end(), draft);
        assert!(state.error.is_some());
        assert!(state.loaded);
        assert!(!state.busy);
    }
    assert!(!format!("{:?}", Event::Edit(text_editor::Action::SelectAll)).contains("secret"));
}

#[test]
fn openclaw_env_editor_retains_failed_draft_rejects_non_objects() {
    for draft in ["", "  ", "[]", "null", "42", "\"secret\"", "{broken"] {
        assert!(parse(draft).is_err());
    }
    assert!(parse("{}").is_ok());
}

#[test]
fn openclaw_env_editor_retains_failed_draft_success_keeps_warnings() {
    let mut state = State::default();
    state.finish_save(Ok(OpenClawWriteOutcome {
        backup_path: None,
        warnings: vec![OpenClawHealthWarning { code: "restart".into(), message: "需要重启".into(), path: None }],
    }));
    assert_eq!(state.warnings[0].code, "restart");
    assert!(safe_error(OpenClawSettingsError::Http(403)).contains("不可访问"));
    state.invalidate();
    assert!(state.editor.text().trim().is_empty());
    assert!(!state.loaded);
}
