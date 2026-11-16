#[test]
fn file_tree_expansion_helpers_toggle_and_deduplicate() {
    let mut state = crate::project_state::State::<()>::new(
        &serde_json::json!({}),
        vec![],
        vec![],
        Default::default(),
    );
    state.file_tree_expanded.clear();
    state.file_tree_expanded_set.clear();

    assert!(!state.file_tree_expanded_set.contains("src"));

    desktop_file_tree::state::ensure_file_tree_dir_expanded(
        &mut state.file_tree_expanded,
        &mut state.file_tree_expanded_set,
        "src".to_string(),
    );
    desktop_file_tree::state::ensure_file_tree_dir_expanded(
        &mut state.file_tree_expanded,
        &mut state.file_tree_expanded_set,
        "src".to_string(),
    );
    assert_eq!(state.file_tree_expanded, ["src"]);
    assert!(state.file_tree_expanded_set.contains("src"));

    desktop_file_tree::state::toggle_file_tree_dir_expanded(
        &mut state.file_tree_expanded,
        &mut state.file_tree_expanded_set,
        "src".to_string(),
    );
    assert!(!state.file_tree_expanded_set.contains("src"));
    assert!(state.file_tree_expanded.is_empty());

    desktop_file_tree::state::toggle_file_tree_dir_expanded(
        &mut state.file_tree_expanded,
        &mut state.file_tree_expanded_set,
        "crates".to_string(),
    );
    assert_eq!(state.file_tree_expanded, ["crates"]);
    assert!(state.file_tree_expanded_set.contains("crates"));
}

#[test]
fn expansion_toggles_clear_membership_when_ordered_entries_are_absent() {
    let mut state = crate::project_state::State::<()>::new(
        &serde_json::json!({}),
        vec![],
        vec![],
        Default::default(),
    );
    state.file_tree_expanded.clear();
    state.file_tree_expanded_set.clear();

    state.file_tree_expanded_set.insert("src".to_string());

    desktop_file_tree::state::toggle_file_tree_dir_expanded(
        &mut state.file_tree_expanded,
        &mut state.file_tree_expanded_set,
        "src".to_string(),
    );

    assert!(!state.file_tree_expanded_set.contains("src"));
    assert!(state.file_tree_expanded.is_empty());
}
