#[test]
fn diff_file_expansion_helpers_keep_order_and_set_in_sync() {
    let mut git = crate::state::State::new(false, false);

    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &git.expanded_files_set,
        "src/lib.rs"
    ));

    crate::message_git::diff::ensure_diff_file_expanded(
        &mut git.expanded_files_set,
        &mut git.expanded_files,
        "src/lib.rs".to_string(),
    );
    crate::message_git::diff::ensure_diff_file_expanded(
        &mut git.expanded_files_set,
        &mut git.expanded_files,
        "src/lib.rs".to_string(),
    );
    assert_eq!(git.expanded_files, ["src/lib.rs"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "src/lib.rs"));

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "src/lib.rs".to_string(),
    );
    assert!(git.expanded_files.is_empty());
    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &git.expanded_files_set,
        "src/lib.rs"
    ));

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "src/main.rs".to_string(),
    );
    crate::message_git::diff::replace_expanded_files(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        vec!["README.md".to_string(), "src/main.rs".to_string()],
    );
    assert_eq!(git.expanded_files, ["README.md", "src/main.rs"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "README.md"));

    crate::message_git::diff::set_single_expanded_file(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "Cargo.toml".to_string(),
    );
    assert_eq!(git.expanded_files, ["Cargo.toml"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "Cargo.toml"));
    assert!(!crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "README.md"));

    crate::message_git::diff::clear_expanded_files(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
    );
    assert!(git.expanded_files.is_empty());
    assert!(git.expanded_files_set.is_empty());
}

#[test]
fn expansion_toggles_clear_membership_when_ordered_entries_are_absent() {
    let mut state = crate::state::State::new(false, false);
    state.expanded_files.clear();
    state.expanded_files_set.clear();

    state.expanded_files_set.insert("src/lib.rs".to_string());

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut state.expanded_files,
        &mut state.expanded_files_set,
        "src/lib.rs".to_string(),
    );

    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &state.expanded_files_set,
        "src/lib.rs"
    ));
    assert!(state.expanded_files.is_empty());
}
