use super::git_workspace::GitWorkspaceStatus;

#[test]
fn git_workspace_status_defaults_to_unavailable() {
    assert_eq!(GitWorkspaceStatus::default(), GitWorkspaceStatus::Unavailable);
}

#[test]
fn git_workspace_status_exposes_dirty_files_in_original_order() {
    let files = vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()];
    let status = GitWorkspaceStatus::ReadyDirty(files.clone());

    assert_eq!(status.modified_files(), files.as_slice());
}

#[test]
fn git_workspace_status_returns_empty_files_for_non_dirty_states() {
    assert_eq!(GitWorkspaceStatus::ReadyClean.modified_files(), &[] as &[String]);
    assert_eq!(GitWorkspaceStatus::Unavailable.modified_files(), &[] as &[String]);
}

