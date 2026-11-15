use super::RecentProjectMeta;
fn recent_meta(path: &str) -> crate::project_message::edit_meta::RecentProjectMeta {
    crate::project_message::edit_meta::RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: crate::project_message::edit_meta::default_recent_project_session_auto_refresh(),
        session_refresh_interval_seconds:
            crate::project_message::edit_meta::default_recent_project_session_refresh_interval_seconds(),
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

#[test]
fn recent_project_dir_name_trims_whitespace_and_trailing_separators() {
    assert_eq!(super::recent_project_dir_name("  /repo/alpha///  "), Some("alpha"));
    assert_eq!(super::recent_project_dir_name("C:\\repo\\beta\\"), Some("beta"));
}

#[test]
fn recent_project_dir_name_rejects_empty_and_separator_only_paths() {
    assert_eq!(super::recent_project_dir_name("   "), None);
    assert_eq!(super::recent_project_dir_name("///"), None);
    assert_eq!(super::recent_project_dir_name("\\\\"), None);
}

#[test]
fn is_visible_recent_project_path_rejects_hidden_last_segment() {
    assert!(!super::is_visible_recent_project_path(
        "/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT"
    ));
}

#[test]
fn is_visible_recent_project_path_rejects_windows_hidden_last_segment() {
    assert!(!super::is_visible_recent_project_path("C:\\Users\\me\\.tmp0fBHwT\\"));
}

#[test]
fn is_visible_recent_project_path_accepts_visible_project_under_hidden_parent() {
    assert!(super::is_visible_recent_project_path("/Users/me/.local/vibe-window"));
}

#[test]
fn is_visible_recent_project_path_rejects_empty_and_dot_names() {
    assert!(!super::is_visible_recent_project_path(""));
    assert!(!super::is_visible_recent_project_path("/repo/."));
    assert!(!super::is_visible_recent_project_path("/repo/.."));
}

#[test]
fn parse_recent_projects_filters_hidden_projects() {
    let content = r#"[
        "/Users/me/vibe-window",
        "/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT",
        "/Users/me/vibe-window"
    ]"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/Users/me/vibe-window".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_legacy_metadata_array() {
    let content = serde_json::to_string(&vec![
        recent_meta(" /repo/alpha "),
        recent_meta("/repo/beta"),
        recent_meta("/repo/alpha"),
    ])
    .expect("recent project metadata should serialize");

    assert_eq!(
        super::parse_recent_projects(&content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_mixed_array_entries() {
    let content = r#"[
        "/repo/alpha",
        {"path": " /repo/beta "},
        {"path": 7},
        false,
        "/repo/.hidden",
        "/repo/alpha"
    ]"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_nested_recent_projects_entries() {
    let content = r#"{
        "recent_projects": [
            {"path": "/repo/alpha"},
            "/repo/beta",
            null,
            {"name": "missing path"}
        ]
    }"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_rejects_malformed_or_unrelated_objects() {
    assert_eq!(super::parse_recent_projects("not json"), None);
    assert_eq!(super::parse_recent_projects(r#"{"projects": []}"#), None);
    assert_eq!(super::parse_recent_projects(r#"{"recent_projects": {}}"#), None);
}

#[test]
fn parse_recent_projects_returns_empty_list_for_valid_scalar_json() {
    assert_eq!(super::parse_recent_projects("null"), Some(vec![]));
    assert_eq!(super::parse_recent_projects("42"), Some(vec![]));
}

#[test]
fn normalize_recent_projects_trims_filters_and_preserves_first_seen_order() {
    let normalized = super::normalize_recent_projects(vec![
        " /repo/beta ".to_string(),
        "".to_string(),
        "/repo/.hidden".to_string(),
        "/repo/alpha".to_string(),
        "/repo/beta".to_string(),
    ]);

    assert_eq!(normalized, vec!["/repo/beta".to_string(), "/repo/alpha".to_string()]);
}

#[test]
fn normalize_recent_projects_preserves_more_than_recent_list_storage_limit() {
    let recent = (0..12).map(|index| format!("/repo/project-{index}")).collect::<Vec<_>>();

    let normalized = super::normalize_recent_projects(recent.clone());

    assert_eq!(normalized, recent);
}

#[test]
fn normalize_recent_projects_meta_filters_hidden_projects() {
    let metas = vec![
        recent_meta("/Users/me/vibe-window"),
        recent_meta("/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT"),
        recent_meta(" /Users/me/vibe-window "),
    ];

    let normalized = super::normalize_recent_projects_meta(metas);

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].path, "/Users/me/vibe-window");
}

#[test]
fn normalize_recent_projects_meta_keeps_first_metadata_for_duplicate_path() {
    let mut first = recent_meta(" /repo/alpha ");
    first.name = "First".to_string();
    let mut duplicate = recent_meta("/repo/alpha");
    duplicate.name = "Second".to_string();

    let normalized = super::normalize_recent_projects_meta(vec![first, duplicate]);

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].path, "/repo/alpha");
    assert_eq!(normalized[0].name, "First");
}

#[test]
fn ensure_recent_project_keeps_existing_project_order() {
    let storage = tempfile::tempdir().unwrap();
    let dirs = directories::ProjectDirs::from_path(storage.path().to_path_buf());
    let mut recent =
        vec!["/repo/alpha".to_string(), "/repo/beta".to_string(), "/repo/gamma".to_string()];

    super::ensure_recent_project(dirs, &mut recent, "/repo/beta".to_string());

    assert_eq!(
        recent,
        vec!["/repo/alpha".to_string(), "/repo/beta".to_string(), "/repo/gamma".to_string()]
    );
}

#[test]
fn ensure_recent_project_adds_new_project_to_front() {
    let storage = tempfile::tempdir().unwrap();
    let dirs = directories::ProjectDirs::from_path(storage.path().to_path_buf());
    let mut recent = vec!["/repo/alpha".to_string(), "/repo/beta".to_string()];

    super::ensure_recent_project(dirs, &mut recent, "/repo/gamma".to_string());

    assert_eq!(
        recent,
        vec!["/repo/gamma".to_string(), "/repo/alpha".to_string(), "/repo/beta".to_string()]
    );
}

#[test]
fn push_recent_project_moves_existing_to_front_and_truncates_to_storage_limit() {
    let storage = tempfile::tempdir().unwrap();
    let dirs = directories::ProjectDirs::from_path(storage.path().to_path_buf());
    let mut recent = (0..11).map(|index| format!("/repo/project-{index}")).collect::<Vec<_>>();

    super::push_recent_project(dirs, &mut recent, "/repo/project-5".to_string());

    assert_eq!(recent.len(), 10);
    assert_eq!(recent.first().map(String::as_str), Some("/repo/project-5"));
    assert_eq!(recent.iter().filter(|path| *path == "/repo/project-5").count(), 1);
    assert!(!recent.contains(&"/repo/project-10".to_string()));
}

#[test]
fn refresh_branches_clears_project_branch_snapshot() {
    let project_path = Some("/repo/alpha".to_string());
    let mut selected_branch = Some("feature/static-tests".to_string());
    let mut branches = vec!["main".to_string(), "feature/static-tests".to_string()];
    let mut project_updated_at_ms = Some(123);

    super::refresh_branches(project_path.as_deref(), &mut selected_branch, &mut branches, &mut project_updated_at_ms);

    assert_eq!(selected_branch, None);
    assert!(branches.is_empty());
    assert_eq!(project_updated_at_ms, None);
}

#[test]
fn refresh_branches_leaves_snapshot_unchanged_without_project() {
    let project_path: Option<String> = None;
    let mut selected_branch = Some("main".to_string());
    let mut branches = vec!["main".to_string()];
    let mut project_updated_at_ms = Some(456);

    super::refresh_branches(project_path.as_deref(), &mut selected_branch, &mut branches, &mut project_updated_at_ms);

    assert_eq!(selected_branch.as_deref(), Some("main"));
    assert_eq!(branches, vec!["main".to_string()]);
    assert_eq!(project_updated_at_ms, Some(456));
}
