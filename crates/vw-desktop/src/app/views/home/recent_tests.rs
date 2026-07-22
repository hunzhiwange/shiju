#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("recent_tests"));
}

#[test]
fn project_name_from_path_trims_trailing_separator() {
    assert_eq!(super::project_name_from_path(""), "未命名项目");
}

#[test]
fn project_name_from_path_keeps_last_segment_without_trailing_separator() {
    assert_eq!(super::project_name_from_path("/Users/Shared/work/dir/data/codes/rue"), "rue");
}

#[test]
fn project_name_from_path_normal_path_returns_last_segment() {
    assert_eq!(super::project_name_from_path("/home/user/project"), "project");
}

#[test]
fn project_name_from_path_trailing_slash_returns_last_segment() {
    assert_eq!(super::project_name_from_path("/a/b/c/"), "c");
}

#[test]
fn project_name_from_path_trailing_backslash_returns_last_segment() {
    assert_eq!(super::project_name_from_path("C:\\a\\b\\c\\"), "c");
}

#[test]
fn project_name_from_path_single_segment() {
    assert_eq!(super::project_name_from_path("my-project"), "my-project");
}

#[test]
fn project_name_from_path_single_dot_dir() {
    assert_eq!(super::project_name_from_path("."), ".");
}

#[test]
fn project_name_from_path_root_slash() {
    let name = super::project_name_from_path("/");
    assert!(!name.is_empty());
}

#[test]
fn project_name_from_path_whitespace_only() {
    let name = super::project_name_from_path("   ");
    assert!(!name.is_empty());
}

#[test]
fn recent_project_name_uses_edits_when_present() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/path".to_string()];
    app.recent_projects_edits = vec!["Custom Name".to_string()];
    let name = super::recent_project_name(&app, 0, "/test/path");
    assert_eq!(name, "Custom Name");
}

#[test]
fn recent_project_name_skips_empty_edits_and_uses_meta() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/path".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/test/path".to_string(),
        name: "Meta Name".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];
    let name = super::recent_project_name(&app, 0, "/test/path");
    assert_eq!(name, "Meta Name");
}

#[test]
fn recent_project_name_skips_whitespace_edits_and_uses_meta() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/path".to_string()];
    app.recent_projects_edits = vec!["   ".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/test/path".to_string(),
        name: "Meta Name".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];
    let name = super::recent_project_name(&app, 0, "/test/path");
    assert_eq!(name, "Meta Name");
}

#[test]
fn recent_project_name_falls_back_to_path_when_no_meta_match() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/my-project".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    let name = super::recent_project_name(&app, 0, "/test/my-project");
    assert_eq!(name, "my-project");
}

#[test]
fn recent_project_name_falls_back_to_path_when_meta_name_empty() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/path".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/test/path".to_string(),
        name: "".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];
    let name = super::recent_project_name(&app, 0, "/test/path");
    assert!(!name.is_empty());
    assert_eq!(name, "path");
}

#[test]
fn recent_project_name_uses_edits_over_meta() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/path".to_string()];
    app.recent_projects_edits = vec!["Edit Name".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/test/path".to_string(),
        name: "Meta Name".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];
    let name = super::recent_project_name(&app, 0, "/test/path");
    assert_eq!(name, "Edit Name");
}

#[test]
fn recent_project_name_multiple_projects_finds_correct_meta_by_path() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/first".to_string(), "/second".to_string()];
    app.recent_projects_edits = vec!["".to_string(), "".to_string()];
    app.recent_projects_meta = vec![
        crate::app::state::RecentProjectMeta {
            path: "/first".to_string(),
            name: "First".to_string(),
            task_board_settings: None,
            session_auto_refresh: true,
            session_refresh_interval_seconds: 60,
            icon: None,
            icon_color: None,
            worktree_start_command: None,
        },
        crate::app::state::RecentProjectMeta {
            path: "/second".to_string(),
            name: "Second".to_string(),
            task_board_settings: None,
            session_auto_refresh: true,
            session_refresh_interval_seconds: 60,
            icon: None,
            icon_color: None,
            worktree_start_command: None,
        },
    ];
    assert_eq!(super::recent_project_name(&app, 1, "/second"), "Second");
}

#[test]
fn render_body_empty_projects_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec![];
    let _ = super::render_body(&app);
}

#[test]
fn render_body_with_recent_projects_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/project-one".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    let _ = super::render_body(&app);
}

#[test]
fn render_body_with_selected_project_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/active".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    app.project_path = Some("/test/active".to_string());
    let _ = super::render_body(&app);
}

#[test]
fn render_body_with_non_matching_selected_project_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/project".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    app.project_path = Some("/other/project".to_string());
    let _ = super::render_body(&app);
}

#[test]
fn render_body_with_multiple_projects_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/a".to_string(), "/test/b".to_string(), "/test/c".to_string()];
    app.recent_projects_edits = vec!["".to_string(), "".to_string(), "Project C".to_string()];
    let _ = super::render_body(&app);
}

#[test]
fn render_body_with_custom_icon_and_color_does_not_panic() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/test/project".to_string()];
    app.recent_projects_edits = vec!["".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/test/project".to_string(),
        name: "Custom".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: Some("some_icon".to_string()),
        icon_color: Some("#FF5500".to_string()),
        worktree_start_command: None,
    }];
    let _ = super::render_body(&app);
}

#[test]
fn project_name_from_path_trims_surrounding_whitespace_before_extracting_name() {
    assert_eq!(super::project_name_from_path("  /tmp/trimmed-project/  "), "trimmed-project");
}

#[test]
fn recent_project_name_returns_trimmed_edit_name() {
    let mut app = crate::app::App::new().0;
    app.recent_projects_edits = vec!["  Edited Project  ".to_string()];

    assert_eq!(super::recent_project_name(&app, 0, "/tmp/fallback"), "Edited Project");
}

#[test]
fn recent_project_name_returns_trimmed_matching_meta_name_without_edit_slot() {
    let mut app = crate::app::App::new().0;
    app.recent_projects_edits.clear();
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/tmp/meta-project".to_string(),
        name: "  Meta Project  ".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];

    assert_eq!(super::recent_project_name(&app, 4, "/tmp/meta-project"), "Meta Project");
}

#[test]
fn recent_project_name_ignores_meta_for_other_paths_when_edit_slot_is_missing() {
    let mut app = crate::app::App::new().0;
    app.recent_projects_edits.clear();
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/tmp/other-project".to_string(),
        name: "Other Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];

    assert_eq!(super::recent_project_name(&app, 3, "/tmp/current-project"), "current-project");
}

#[test]
fn render_body_ignores_invalid_custom_color_and_blank_icon() {
    let mut app = crate::app::App::new().0;
    app.recent_projects = vec!["/tmp/fallback-accent".to_string()];
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/tmp/fallback-accent".to_string(),
        name: "Fallback Accent".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: Some("   ".to_string()),
        icon_color: Some("not-a-color".to_string()),
        worktree_start_command: None,
    }];

    std::hint::black_box(super::render_body(&app));
}
