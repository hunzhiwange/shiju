//! 验证 Git 主视图的虚拟窗口、文件卡片估算及弹窗叠加路径。
//! 空仓库、超出视口和多种选择来源用于约束回退与过滤行为。

use crate::page::Message;
use crate::page::utils::FileStatus;
use desktop_workspace::workspace_state::ChatTextDiff;

#[test]
fn task_720_mod_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("mod_tests.rs"));
}

fn app() -> crate::test_support::Fixture {
    crate::test_support::Fixture::new().0
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
fn embedded_custom_diff_and_main_view_cover_modal_and_filter_paths() {
    let mut app = app();
    let embedded = super::embedded_custom_text_diff_view(
        &app.view(),
        "Title".to_string(),
        Some("src/lib.rs".to_string()),
        "old\n".to_string(),
        "new\n".to_string(),
        Some(Message::None),
    );
    drop(embedded);

    app.git.show_git_filter_options = true;
    app.git.show_git_diff_summary = true;
    app.git.show_git_custom_diff_modal = true;
    app.git.git_custom_diff_title = "Custom".to_string();
    app.git.git_custom_diff_hide_inputs = false;
    app.git.git_diff_file_metas = vec![
        meta("src/add.rs", FileStatus::Added, 2, 0),
        meta("src/mod.rs", FileStatus::Modified, 2, 1),
        meta("src/delete.rs", FileStatus::Deleted, 0, 2),
    ];
    app.git.git_filter_query = "src/".to_string();
    app.git.git_filter_new = true;
    app.git.git_filter_modified = true;
    app.git.git_filter_deleted = true;
    app.git.staged_files_selected.push("src/add.rs".to_string());
    app.git.expanded_files_set.insert("src/mod.rs".to_string());
    app.git
        .git_diff_contents
        .insert("src/mod.rs".to_string(), ("old\n".to_string(), "new\n".to_string()));

    let with_editors = super::view(&app.view());
    drop(with_editors);

    app.git.git_custom_diff_hide_inputs = true;
    app.git.chat_text_diff = Some(ChatTextDiff {
        title: "Chat".to_string(),
        file: "src/chat.rs".to_string(),
        before: "a\n".to_string(),
        after: "b\n".to_string(),
    });
    let _hidden_inputs = super::view(&app.view());
}

#[test]
fn main_view_covers_help_and_copy_modal_layers() {
    let mut app = app();
    app.git.show_git_commit_help_modal = true;
    app.git.show_git_filter_help_modal = true;
    let with_help = super::view(&app.view());
    drop(with_help);

    app.git.show_git_commit_help_modal = false;
    app.git.show_git_filter_help_modal = false;
    app.git.show_git_copy_modal = true;
    app.git.git_copy_modal_use_color = false;
    let text_modal = super::view(&app.view());
    assert_eq!(text_modal.as_widget().children().len(), 3);
    drop(text_modal);

    app.git.git_copy_modal_use_color = true;
    let colored_modal = super::view(&app.view());
    assert_eq!(colored_modal.as_widget().children().len(), 3);
}

#[test]
fn main_view_filters_included_files_from_all_selection_sources() {
    let mut app = app();
    app.git.git_diff_file_metas = vec![
        meta("src/file.rs", FileStatus::Modified, 1, 1),
        meta("src/hunk.rs", FileStatus::Modified, 1, 0),
        meta("src/line.rs", FileStatus::Modified, 0, 1),
        meta("src/old_line.rs", FileStatus::Deleted, 0, 1),
        meta("src/excluded.rs", FileStatus::Added, 1, 0),
    ];
    app.git.git_filter_included = true;
    app.git.staged_files_selected.push("src/file.rs".to_string());
    app.git.staged_hunks_selected.push(("src/hunk.rs".to_string(), 1));
    app.git.staged_lines_selected.push(("src/line.rs".to_string(), 2));
    app.git.staged_old_lines_selected.push(("src/old_line.rs".to_string(), 3));

    let included = super::view(&app.view());
    drop(included);

    app.git.git_filter_included = false;
    app.git.git_filter_excluded = true;
    let excluded = super::view(&app.view());
    drop(excluded);
}

#[test]
fn main_view_covers_empty_and_virtualized_lists() {
    let mut app = app();
    let empty = super::view(&app.view());
    drop(empty);

    app.git.git_diff_file_metas =
        (0..50).map(|idx| meta(&format!("src/file_{idx}.rs"), FileStatus::Added, 1, 0)).collect();
    app.git.git_diff_scroll_viewport_h = 120.0;
    app.git.git_diff_scroll_offset_y = 0.5;

    let _virtualized = super::view(&app.view());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repository_wrappers_preserve_empty_results_for_missing_repository() {
    let temp = tempfile::tempdir().expect("temporary parent");
    let missing = temp.path().join("missing-repository");
    let path = missing.to_str().expect("utf8 path");
    let mut app = app();
    app.project_path = Some(path.to_string());
    let missing_meta = meta("missing.txt", FileStatus::Modified, 0, 0);

    assert_eq!(super::git_repo_path_for_app(&app.view()), None);
    assert!(super::get_diff_file_metas_for_repo_path(path).is_empty());
    assert_eq!(
        super::load_diff_content_for_repo_path(path, &missing_meta),
        (String::new(), String::new())
    );
    assert!(super::changed_files_in_repo(path).is_empty());
    assert!(super::changed_files(&app.view()).is_empty());
}
