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
