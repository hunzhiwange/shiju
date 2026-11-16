// 验证工作树选项刷新、过期结果过滤及默认选项回退。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::message::git::GitMessage;
use crate::app::state::GitWorktreeOption;

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn refresh_worktree_options_for_non_git_project_clears_state_without_error() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let project_path = temp.path().to_string_lossy().to_string();
    let (mut app, _task) = crate::app::App::new();
    app.navigation_tabs.project_path = Some(project_path.clone());
    app.git.git_worktree_options = vec![GitWorktreeOption {
        directory: "/old".to_string(),
        label: "old".to_string(),
        branch: Some("main".to_string()),
    }];
    app.git.selected_git_worktree_directory = Some("/old".to_string());
    app.git.git_worktree_options_loading = true;
    app.git.git_worktree_options_project_path = Some(project_path);
    app.git.git_worktree_menu_open = true;

    let _task = super::refresh::update(&mut app, GitMessage::RefreshWorktreeOptions);

    assert!(app.git.git_worktree_options.is_empty());
    assert_eq!(app.git.selected_git_worktree_directory, None);
    assert!(!app.git.git_worktree_options_loading);
    assert_eq!(app.git.git_worktree_options_project_path, None);
    assert!(!app.git.git_worktree_menu_open);
    assert_eq!(app.input_feedback.error_message, None);
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn worktree_options_ready_error_resets_options_and_reports_error() {
    let (mut app, _task) = crate::app::App::new();
    app.git.git_worktree_options_project_path = Some("/project".to_string());
    app.git.git_worktree_options_loading = true;
    app.git.git_worktree_menu_open = true;
    app.git.git_worktree_options = vec![GitWorktreeOption {
        directory: "/old".to_string(),
        label: "old".to_string(),
        branch: None,
    }];

    let _task = super::refresh::update(
        &mut app,
        GitMessage::WorktreeOptionsReady {
            project_path: "/project".to_string(),
            result: Err("boom".to_string()),
        },
    );

    assert!(app.git.git_worktree_options.is_empty());
    assert_eq!(app.git.selected_git_worktree_directory, None);
    assert!(!app.git.git_worktree_menu_open);
    assert_eq!(app.input_feedback.error_message.as_deref(), Some("读取 Git worktree 失败: boom"));
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn changed_files_ready_updates_files_and_clears_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.git.git_changed_files_repo_path = Some("/repo".to_string());
    app.git.git_changed_files_loading = true;
    app.file_manager.file_manager_changes_refreshing = true;
    app.git.git_diff_selected_lines.push(crate::app::state::GitDiffSelectedLine {
        file: "old.rs".to_string(),
        line: 1,
        is_old: false,
        text: "x".to_string(),
    });
    app.git.git_diff_selected_range = Some(crate::app::state::GitDiffLineRange {
        file: "old.rs".to_string(),
        start: 2,
        end: 1,
        is_old: false,
    });
    app.git.git_diff_comment_draft = Some(crate::app::state::GitDiffCommentDraft {
        range: crate::app::state::GitDiffLineRange {
            file: "old.rs".to_string(),
            start: 1,
            end: 1,
            is_old: false,
        },
        editor: iced::widget::text_editor::Content::with_text("draft"),
    });

    let _task = super::refresh::update(
        &mut app,
        GitMessage::ChangedFilesReady {
            repo_path: Some("/repo".to_string()),
            files: vec!["src/main.rs".to_string()],
        },
    );

    assert_eq!(app.git.git_changed_files, vec!["src/main.rs"]);
    assert!(!app.git.git_changed_files_loading);
    assert!(!app.file_manager.file_manager_changes_refreshing);
    assert!(app.git.git_diff_selected_lines.is_empty());
    assert!(app.git.git_diff_selected_range.is_none());
    assert!(app.git.git_diff_comment_draft.is_none());
    assert_eq!(
        app.toast_state.active_toast.as_ref().map(|toast| toast.message.as_str()),
        Some("Git 更改已刷新")
    );
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn changed_files_ready_expands_change_list_dirs_when_showing_changes() {
    let (mut app, _task) = crate::app::App::new();
    app.git.git_changed_files_repo_path = Some("/repo".to_string());
    app.file_manager.file_manager_show_changes = true;
    app.file_manager.file_manager_changes_refreshing = false;

    let _task = super::refresh::update(
        &mut app,
        GitMessage::ChangedFilesReady {
            repo_path: Some("/repo".to_string()),
            files: vec!["src/nested/lib.rs".to_string(), "README.md".to_string()],
        },
    );

    assert!(app.workspace_state.file_tree_expanded.contains(&"chg:src".to_string()));
    assert!(app.workspace_state.file_tree_expanded.contains(&"chg:src/nested".to_string()));
    assert!(!app.workspace_state.file_tree_expanded.contains(&"src".to_string()));
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn changed_files_ready_without_manual_refresh_does_not_create_toast() {
    let (mut app, _task) = crate::app::App::new();
    app.git.git_changed_files_repo_path = Some("/repo".to_string());
    app.file_manager.file_manager_changes_refreshing = false;
    app.toast_state.active_toast = None;

    let _task = super::refresh::update(
        &mut app,
        GitMessage::ChangedFilesReady {
            repo_path: Some("/repo".to_string()),
            files: vec!["README.md".to_string()],
        },
    );

    assert_eq!(app.git.git_changed_files, vec!["README.md"]);
    assert!(app.toast_state.active_toast.is_none());
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
