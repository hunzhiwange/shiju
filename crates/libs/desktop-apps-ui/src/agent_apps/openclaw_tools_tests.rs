use super::*;

#[test]
fn openclaw_tools_editor_shows_unknown_profile_and_preserves_extensions() {
    let draft: OpenClawToolsConfig = serde_json::from_value(serde_json::json!({
        "profile": "future", "allow": ["read", "  "], "deny": ["exec"], "futureOption": {"enabled": true}
    })).unwrap();
    assert!(
        matches!(Profile::from_value(&draft.profile), Profile::Unknown(value) if value == "future")
    );
    assert_eq!(Profile::from_value(&draft.profile).to_string(), "未知值：future");
    let saved = candidate(&draft);
    assert_eq!(saved.profile, draft.profile);
    assert_eq!(saved.extra, draft.extra);
    assert_eq!(saved.allow, vec!["read"]);
    assert_eq!(saved.deny, vec!["exec"]);
    assert_eq!(draft.allow.len(), 2);
}

#[test]
fn openclaw_tools_editor_shows_unknown_profile_distinct_from_unset_and_known() {
    assert!(matches!(Profile::from_value(&None), Profile::Unset));
    assert!(matches!(Profile::from_value(&Some(String::new())), Profile::Unknown(_)));
    for profile in ["minimal", "coding", "messaging", "full"] {
        let projected = Profile::from_value(&Some(profile.into()));
        assert!(!matches!(projected, Profile::Unset | Profile::Unknown(_)));
        assert_eq!(projected.to_string(), profile);
    }
}

#[test]
fn openclaw_tools_editor_shows_unknown_profile_save_failure_keeps_draft_and_warnings() {
    let mut state = State::default();
    state.draft.profile = Some("future".into());
    state.draft.allow.push("read".into());
    state.busy = true;
    state.finish_save(Err("拒绝写入".into()));
    assert!(!state.busy);
    assert_eq!(state.draft.profile.as_deref(), Some("future"));
    assert_eq!(state.draft.allow, vec!["read"]);
    assert_eq!(state.error.as_deref(), Some("拒绝写入"));
    state.finish_save(Ok(OpenClawWriteOutcome {
        backup_path: None,
        warnings: vec![OpenClawHealthWarning {
            code: "unsupported_profile".into(),
            message: "未知配置".into(),
            path: Some("tools.profile".into()),
        }],
    }));
    assert!(state.error.is_none());
    assert_eq!(state.warnings.len(), 1);
    assert_eq!(state.draft.profile.as_deref(), Some("future"));
}
