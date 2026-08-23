//! 验证 Git 面板公共布局计算、主题选择与可选弹窗的基础行为。
//! 这些测试从面板入口约束虚拟化边界和缺失项目路径时的安全空结果。

use super::utils::FileStatus;

fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[cfg(not(target_arch = "wasm32"))]
fn meta(
    path: &str,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
) -> super::DiffFileMeta {
    super::DiffFileMeta {
        path: path.to_string(),
        status,
        old_oid: None,
        old_size: 0,
        new_size: 0,
        new_exists: true,
        insertions,
        deletions,
    }
}

#[cfg(target_arch = "wasm32")]
fn meta(
    path: &str,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
) -> super::DiffFileMeta {
    super::DiffFileMeta {
        path: path.to_string(),
        status,
        old_size: 0,
        new_size: 0,
        new_exists: true,
        insertions,
        deletions,
    }
}

#[test]
fn theme_brightness_selects_dark_and_light_diff_palettes() {
    assert!(super::is_dark_theme(&iced::Theme::Dark));
    assert!(!super::is_dark_theme(&iced::Theme::Light));

    let dark = super::get_diff_colors(crate::app::DiffTheme::Monokai);
    let light = super::get_diff_colors(crate::app::DiffTheme::GitHub);
    assert_ne!(dark.1, light.1);
    assert_ne!(dark.2, light.2);
}

#[test]
fn changed_files_without_project_path_is_empty_without_repository_access() {
    let mut app = app();
    app.project_path = None;

    assert!(super::changed_files(&app).is_empty());
}

#[test]
fn optional_git_modals_require_corresponding_state() {
    let mut app = app();
    app.file_to_discard = None;
    assert!(super::diff_comment_modal(&app).is_none());
    assert!(super::discard_file_modal(&app).is_none());

    app.file_to_discard = Some("src/lib.rs".to_string());
    assert!(super::discard_file_modal(&app).is_some());
}

#[test]
fn file_card_height_accounts_for_focus_loading_and_missing_content() {
    let mut app = app();
    let file = meta("src/lib.rs", FileStatus::Modified, 3, 2);

    assert_eq!(
        super::estimate_git_file_card_height(&app, &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
    );

    app.git_focused_file = Some(file.path.clone());
    assert_eq!(
        super::estimate_git_file_card_height(&app, &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_FOCUSED_EXTRA_HEIGHT
    );

    app.expanded_files_set.insert(file.path.clone());
    app.git_diff_contents_loading.insert(file.path.clone());
    assert_eq!(
        super::estimate_git_file_card_height(&app, &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
            + super::GIT_FILE_FOCUSED_EXTRA_HEIGHT
            + super::GIT_FILE_LOADING_PLACEHOLDER_HEIGHT
    );

    app.git_focused_file = None;
    app.git_diff_contents_loading.clear();
    assert_eq!(
        super::estimate_git_file_card_height(&app, &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_LOADING_PLACEHOLDER_HEIGHT
    );
}

#[test]
fn file_card_height_uses_status_specific_line_estimates() {
    let cases = [
        (FileStatus::Added, "old", "one\ntwo", 5, 0, 5),
        (FileStatus::Untracked, "", "one\ntwo\nthree", 0, 0, 3),
        (FileStatus::Deleted, "one\ntwo", "", 0, 4, 4),
        (FileStatus::Modified, "one\ntwo", "three\nfour", 2, 1, 4),
        (FileStatus::Renamed, "one\ntwo", "three\nfour", 1, 1, 4),
        (FileStatus::Unknown, "one", "two", 0, 0, 12),
    ];

    for (index, (status, old, new, insertions, deletions, expected_lines)) in
        cases.into_iter().enumerate()
    {
        let mut app = app();
        let file = meta(&format!("src/file_{index}.rs"), status, insertions, deletions);
        app.expanded_files_set.insert(file.path.clone());
        app.git_diff_contents.insert(file.path.clone(), (old.to_string(), new.to_string()));

        assert_eq!(
            super::estimate_git_file_card_height(&app, &file),
            super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
                + expected_lines as f32 * super::GIT_FILE_LINE_ESTIMATED_HEIGHT
                + 12.0,
            "unexpected estimate for {status:?}"
        );
    }
}

#[test]
fn modified_empty_contents_still_reserve_one_diff_line() {
    let mut app = app();
    let file = meta("empty.txt", FileStatus::Modified, 0, 0);
    app.expanded_files_set.insert(file.path.clone());
    app.git_diff_contents.insert(file.path.clone(), (String::new(), String::new()));

    assert_eq!(
        super::estimate_git_file_card_height(&app, &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_LINE_ESTIMATED_HEIGHT + 12.0
    );
}

#[test]
fn virtual_window_has_exact_buffered_bounds_and_spacers() {
    let mut app = app();
    let heights = vec![10.0; 40];
    app.git_diff_scroll_viewport_h = 10.0;

    app.git_diff_scroll_offset_y = 0.0;
    assert_eq!(super::compute_git_file_virtual_window(&app, &heights), (0, 13, 0.0, 530.0));

    app.git_diff_scroll_offset_y = 0.5;
    assert_eq!(super::compute_git_file_virtual_window(&app, &heights), (7, 33, 130.0, 130.0));

    app.git_diff_scroll_offset_y = 1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app, &heights), (27, 40, 530.0, 0.0));
}

#[test]
fn virtual_window_disables_clipping_for_non_positive_or_oversized_viewports() {
    let mut app = app();
    let heights = vec![10.0, 20.0, 30.0];

    app.git_diff_scroll_viewport_h = -1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app, &heights), (0, 3, 0.0, 0.0));

    app.git_diff_scroll_viewport_h = 1_000.0;
    app.git_diff_scroll_offset_y = 1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app, &heights), (0, 3, 0.0, 0.0));
}

#[test]
fn main_view_constructs_selection_help_and_copy_modal_variants() {
    let mut app = app();
    app.git_diff_file_metas = vec![
        meta("src/file.rs", FileStatus::Modified, 1, 1),
        meta("src/hunk.rs", FileStatus::Added, 2, 0),
        meta("src/line.rs", FileStatus::Deleted, 0, 2),
        meta("src/old_line.rs", FileStatus::Renamed, 1, 1),
    ];
    app.staged_files_selected.push("src/file.rs".to_string());
    app.staged_hunks_selected.push(("src/hunk.rs".to_string(), 0));
    app.staged_lines_selected.push(("src/line.rs".to_string(), 1));
    app.staged_old_lines_selected.push(("src/old_line.rs".to_string(), 1));
    app.git_filter_included = true;
    app.git_filter_query = "SRC/".to_string();
    app.git_focused_file = Some("src/file.rs".to_string());
    app.expanded_files_set.insert("src/file.rs".to_string());
    app.git_diff_contents_loading.insert("src/file.rs".to_string());
    app.show_git_commit_help_modal = true;
    app.show_git_filter_help_modal = true;
    app.show_git_copy_modal = true;
    app.git_copy_modal_use_color = false;
    drop(super::view(&app));

    app.git_filter_included = false;
    app.git_filter_excluded = true;
    app.git_copy_modal_use_color = true;
    drop(super::view(&app));
}

#[test]
fn embedded_diff_constructs_for_both_application_theme_brightnesses() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let mut app = app();
        app.app_theme = theme;

        drop(super::embedded_custom_text_diff_view(
            &app,
            "Comparison".to_string(),
            None,
            "before".to_string(),
            "after".to_string(),
            None,
        ));
    }
}
