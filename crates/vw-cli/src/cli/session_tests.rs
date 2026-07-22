use super::session::{
    GitWorkspaceStatus, build_project_info, collect_git_workspace_status, collect_modified_files,
    initial_cli_session_title_for_input, parse_git_status_porcelain_path,
};

struct TestDir {
    path: std::path::PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("vw-cli-session-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create session test directory");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn initial_title_defaults_for_empty_or_whitespace_input() {
    assert_eq!(initial_cli_session_title_for_input(""), "CLI session");
    assert_eq!(initial_cli_session_title_for_input(" \t\n\r"), "CLI session");
}

#[test]
fn initial_title_collapses_mixed_whitespace() {
    assert_eq!(
        initial_cli_session_title_for_input("  inspect\tthis\n\nproject  "),
        "inspect this project"
    );
}

#[test]
fn initial_title_preserves_unicode_content_within_limit() {
    assert_eq!(
        initial_cli_session_title_for_input("  分析\u{3000}这个 Rust 项目  "),
        "分析 这个 Rust 项目"
    );
}

#[test]
fn initial_title_keeps_exactly_fifty_characters_without_ellipsis() {
    let input = "界".repeat(50);

    assert_eq!(initial_cli_session_title_for_input(&input), input);
}

#[test]
fn initial_title_truncates_by_character_and_appends_ellipsis() {
    let input = format!("{}尾", "界".repeat(50));

    assert_eq!(initial_cli_session_title_for_input(&input), format!("{}...", "界".repeat(50)));
}

#[test]
fn initial_title_removes_trailing_separator_at_truncation_boundary() {
    let input = format!("{} remaining", "a".repeat(49));

    assert_eq!(initial_cli_session_title_for_input(&input), format!("{}...", "a".repeat(49)));
}

#[test]
fn initial_title_applies_limit_after_whitespace_normalization() {
    let input = format!("  {}\n\t尾  ", "a".repeat(49));

    assert_eq!(initial_cli_session_title_for_input(&input), format!("{}...", "a".repeat(49)));
}

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
    assert!(GitWorkspaceStatus::ReadyClean.modified_files().is_empty());
    assert!(GitWorkspaceStatus::Unavailable.modified_files().is_empty());
}

#[test]
fn porcelain_path_parses_tracked_untracked_and_spaced_paths() {
    assert_eq!(parse_git_status_porcelain_path(" M src/lib.rs"), Some("src/lib.rs".to_string()));
    assert_eq!(parse_git_status_porcelain_path("?? notes.txt"), Some("notes.txt".to_string()));
    assert_eq!(
        parse_git_status_porcelain_path("A  docs/design notes.md"),
        Some("docs/design notes.md".to_string())
    );
}

#[test]
fn porcelain_path_uses_rename_destination() {
    assert_eq!(
        parse_git_status_porcelain_path("R  old name.rs -> new name.rs"),
        Some("new name.rs".to_string())
    );
}

#[test]
fn porcelain_path_rejects_missing_or_empty_path() {
    assert_eq!(parse_git_status_porcelain_path(""), None);
    assert_eq!(parse_git_status_porcelain_path("?"), None);
    assert_eq!(parse_git_status_porcelain_path("??"), None);
    assert_eq!(parse_git_status_porcelain_path(" M"), None);
    assert_eq!(parse_git_status_porcelain_path(" M   "), None);
}

#[test]
fn porcelain_path_preserves_non_rename_path_whitespace_inside_filename() {
    assert_eq!(
        parse_git_status_porcelain_path(" M docs/two  spaces.md"),
        Some("docs/two  spaces.md".to_string())
    );
}

#[test]
fn collect_git_workspace_status_reports_unavailable_for_missing_directory() {
    let missing =
        std::env::temp_dir().join(format!("vw-cli-missing-worktree-{}", std::process::id()));

    assert_eq!(collect_git_workspace_status(&missing), GitWorkspaceStatus::Unavailable);
    assert!(collect_modified_files(&missing).is_empty());
}

#[test]
fn build_project_info_includes_path_version_and_no_branch_for_non_git_directory() {
    let test_dir = TestDir::new();

    let info = build_project_info(test_dir.path());

    assert!(info.starts_with(&test_dir.path().display().to_string()));
    assert!(info.contains("VibeWindow"));
    assert!(!info.contains(":main"));
}
