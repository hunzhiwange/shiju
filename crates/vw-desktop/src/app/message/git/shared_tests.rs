// 验证 Git 上下文路径解析、瞬态界面清理与差异加载任务。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::state::GitDiffLineRange;
use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };

    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }

    Vec::new()
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
// 宿主契约：核对 App 字段投影及 git_context_path_for_app 的消息/任务接线。
#[test]
fn git_context_path_prefers_trimmed_worktree_and_falls_back_to_project() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/project/root".to_string());
    app.selected_git_worktree_directory = Some("  /project/worktree  ".to_string());

    assert_eq!(
        super::shared::git_context_path_for_app(&app),
        Some("/project/worktree".to_string())
    );

    app.selected_git_worktree_directory = Some(" \n\t ".to_string());

    assert_eq!(super::shared::git_context_path_for_app(&app), Some("/project/root".to_string()));
}

// 宿主契约：核对 App 字段投影及 git_context_path_for_app 的消息/任务接线。
#[test]
fn git_context_path_returns_none_without_any_context() {
    let (mut app, _task) = crate::app::App::new();
    app.selected_git_worktree_directory = None;
    app.active_session_id = None;
    app.sessions.clear();
    app.project_path = None;

    assert_eq!(super::shared::git_context_path_for_app(&app), None);
}

// 宿主契约：核对 App 字段投影及 git_context_path_for_app 的消息/任务接线。
#[test]
fn git_context_path_uses_trimmed_active_session_directory() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![session_info::Info {
        id: "session-1".to_string(),
        slug: "session-1".to_string(),
        project_id: "project-1".to_string(),
        directory: "  /session/root  ".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }];
    app.project_path = Some("/project/root".to_string());

    assert_eq!(super::shared::git_context_path_for_app(&app).as_deref(), Some("/session/root"));
}

// 宿主契约：核对 App 字段投影及 dismiss_preview_transient_ui 的消息/任务接线。
#[test]
fn dismiss_preview_transient_ui_clears_git_file_menu() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_file_menu =
        Some(crate::app::state::GitDiffFileMenuState { file: "src/lib.rs".to_string() });

    super::shared::dismiss_preview_transient_ui(&mut app);

    assert!(app.git_diff_file_menu.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_git_panel_data_task_emits_all_refresh_messages() {
    let messages = collect_task_messages(super::shared::refresh_git_panel_data_task()).await;

    assert_eq!(messages.len(), 3);
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshWorktreeOptions)
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshChangedFiles)
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshDiffFileMetas)
    )));
}

// 宿主契约：核对 App 字段投影及 take_diff_content_task 的消息/任务接线。
#[test]
fn take_diff_content_task_rejects_loaded_loading_and_missing_repo_states() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_contents.insert("loaded.rs".to_string(), (String::new(), String::new()));
    app.git_diff_contents_loading.insert("loading.rs".to_string());
    app.project_path = None;

    assert!(super::shared::take_diff_content_task(&mut app, "loaded.rs".to_string()).is_none());
    assert!(super::shared::take_diff_content_task(&mut app, "loading.rs".to_string()).is_none());
    assert!(super::shared::take_diff_content_task(&mut app, "missing.rs".to_string()).is_none());
}

// 宿主契约：核对 App 字段投影及 load_missing_diff_contents 的消息/任务接线。
#[test]
fn load_missing_diff_contents_without_repo_does_not_mark_files_loading() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;

    let _task = super::shared::load_missing_diff_contents(
        &mut app,
        ["a.rs".to_string(), "b.rs".to_string()],
    );

    assert!(app.git_diff_contents_loading.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
// 宿主契约：核对 App 字段投影及 git_repo_path_for_app 的消息/任务接线。
#[test]
fn git_repo_path_rejects_explicit_non_repository_worktree() {
    let (mut app, _task) = crate::app::App::new();
    app.selected_git_worktree_directory = Some("/path/that/does/not/exist".to_string());
    app.project_path = Some("/fallback/project".to_string());

    assert_eq!(super::shared::git_repo_path_for_app(&app), None);
}

// 宿主契约：核对 App 字段投影及 refresh_git_panel_data_after_repo_mutation_task 的消息/任务接线。
#[test]
fn refresh_after_repo_mutation_clears_stale_loading_guards() {
    let (mut app, _task) = crate::app::App::new();

    app.git_changed_files_loading = true;
    app.git_changed_files_repo_path = Some("/tmp/repo".to_string());
    app.git_diff_file_metas_loading = true;
    app.git_diff_file_metas_repo_path = Some("/tmp/repo".to_string());
    app.git_diff_contents_loading.insert("src/main.rs".to_string());

    let _task = super::shared::refresh_git_panel_data_after_repo_mutation_task(&mut app);

    assert!(!app.git_changed_files_loading);
    assert_eq!(app.git_changed_files_repo_path, None);
    assert!(!app.git_diff_file_metas_loading);
    assert_eq!(app.git_diff_file_metas_repo_path, None);
    assert!(app.git_diff_contents_loading.is_empty());
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

// 宿主契约：核对 App 字段投影及 selected_lines_from_range 的消息/任务接线。
#[test]
fn selected_lines_from_range_returns_empty_without_diff_content() {
    let (app, _task) = crate::app::App::new();
    let range =
        GitDiffLineRange { file: "missing.rs".to_string(), start: 1, end: 3, is_old: false };

    assert!(super::shared::selected_lines_from_range(&app, &range).is_empty());
}
