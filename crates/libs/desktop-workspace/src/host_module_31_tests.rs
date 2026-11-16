#[test]
fn project_preferences_keep_tree_lookup_in_sync_without_resetting_live_state() {
    let mut state = crate::project_state::State::<()>::new(
        &serde_json::json!({}),
        vec![],
        vec![],
        Default::default(),
    );
    state.project_id = Some("active-project".into());
    state.project_sessions_loading.insert("/active".into());
    state.apply_preferences(&serde_json::json!({
        "file_tree_expanded": ["src", 7, "src", "docs"],
        "new_session_last_directory": "/next"
    }));
    assert_eq!(state.file_tree_expanded, ["src", "src", "docs"]);
    assert_eq!(state.file_tree_expanded_set.len(), 2);
    assert_eq!(state.new_session_last_directory.as_deref(), Some("/next"));
    assert_eq!(state.project_id.as_deref(), Some("active-project"));
    assert!(state.project_sessions_loading.contains("/active"));
    state.apply_preferences(&serde_json::json!({"new_session_last_directory": null}));
    assert!(state.new_session_last_directory.is_none());
    assert_eq!(state.file_tree_expanded_set.len(), 2);
}
