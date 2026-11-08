// 验证最近项目搜索和删除确认状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::{App, state::RecentProjectMeta};

fn app() -> App {
    App::new().0
}

fn recent_project_meta(path: &str, name: &str) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: name.to_string(),
        task_board_settings: None,
        session_auto_refresh: false,
        session_refresh_interval_seconds: 120,
        icon: Some("folder".to_string()),
        icon_color: Some("blue".to_string()),
        worktree_start_command: Some("prepare".to_string()),
    }
}

#[test]
fn search_updates_query_and_dismisses_previous_row_actions() {
    let mut app = app();
    app.project_tools_menu_path = Some("/workspace/project-a".into());
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectQueryChanged("project-b".into()));

    assert_eq!(app.recent_project_query, "project-b");
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.recent_project_delete_confirm_idx.is_none());
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn delete_request_and_cancel_only_change_confirmation_state() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteRequested(7));

    assert_eq!(app.recent_project_delete_confirm_idx, Some(7));
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteCanceled);

    assert_eq!(app.recent_project_delete_confirm_idx, None);
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
}

#[test]
fn delete_request_allows_current_project() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.project_path = Some("/workspace/project-a".to_string());

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteRequested(0));

    assert_eq!(app.recent_project_delete_confirm_idx, Some(0));
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert!(app.active_toast.is_none());
}

#[test]
fn delete_confirmed_current_project_selects_first_remaining_project() {
    let mut app = app();
    app.recent_projects =
        vec!["/workspace/project-a".to_string(), "/workspace/project-b".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string(), "Project B".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-b", "Project B")];
    app.project_path = Some("/workspace/project-a".to_string());
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert_eq!(app.recent_projects, vec!["/workspace/project-b"]);
    assert_eq!(app.recent_projects_edits, vec!["Project B"]);
    assert_eq!(app.project_path.as_deref(), Some("/workspace/project-b"));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
    assert!(app.active_toast.is_none());
}

#[test]
fn delete_confirmed_removes_associated_state_and_rebuilds_display_names() {
    let mut app = app();
    app.recent_projects = vec![
        "/workspace/deleted".to_string(),
        "/workspace/from-meta".to_string(),
        "/workspace/from-path".to_string(),
        "/".to_string(),
    ];
    app.recent_projects_edits = vec![
        "Deleted".to_string(),
        "Stale meta".to_string(),
        "Stale path".to_string(),
        "Stale root".to_string(),
    ];
    app.recent_projects_meta = vec![
        recent_project_meta("/workspace/deleted", "Deleted"),
        recent_project_meta("/workspace/from-meta", "Metadata name"),
    ];
    app.project_worktree_enabled.insert("/workspace/deleted".to_string(), true);
    app.project_worktree_enabled.insert("/workspace/from-meta".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert_eq!(app.recent_projects, vec!["/workspace/from-meta", "/workspace/from-path", "/"]);
    assert_eq!(app.recent_projects_edits, vec!["Metadata name", "from-path", "/"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/from-meta");
    assert!(!app.project_worktree_enabled.contains_key("/workspace/deleted"));
    assert_eq!(app.project_worktree_enabled.get("/workspace/from-meta"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn worktree_toggles_patch_one_project_in_click_order() {
    use iced::futures::StreamExt;
    use std::time::Duration;

    crate::app::config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let path = "/workspace/project-a";
        let enable =
            update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.into(), true));
        let disable =
            update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.into(), false));
        assert_eq!(disable.units(), 0);
        assert!(!app.project_worktree_enabled.contains_key(path));
        assert!(requests.try_recv().is_err());

        let idle = wait_for_worktree_saves(&mut app).expect("pending saves");
        assert!(!*idle.borrow());
        let mut task = enable;
        for value in [json!(true), Value::Null] {
            responses.send(Some("null".into())).unwrap();
            let mut stream = iced_runtime::task::into_stream(task).expect("background save");
            let Some(iced_runtime::Action::Output(Message::Settings(saved))) =
                runtime.block_on(stream.next())
            else {
                panic!("expected worktree save completion");
            };
            assert!(matches!(saved, SettingsMessage::ProjectWorktreeSaved(Ok(()))));
            let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
            assert!(request.starts_with("PATCH "));
            let (_, body) = request.split_once("\r\n\r\n").unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(body).unwrap(),
                json!({"app_ui": {"system_settings": {"project_worktree_enabled": {path: value}}}})
            );
            task = update(&mut app, saved);
        }
        assert_eq!(task.units(), 0);
        assert!(!app.project_worktree_save_running);
        assert!(*idle.borrow());
    });
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn delete_confirmed_handles_missing_edit_for_existing_project() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/deleted".to_string()];
    app.recent_projects_edits.clear();
    app.recent_projects_meta = vec![recent_project_meta("/workspace/deleted", "Deleted")];

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert!(app.recent_projects.is_empty());
    assert!(app.recent_projects_edits.is_empty());
    assert!(app.recent_projects_meta.is_empty());
    assert_eq!(app.recent_project_delete_confirm_idx, None);
}

#[test]
fn delete_confirmed_rejects_out_of_bounds_index_but_closes_confirmation() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-a", "Project A")];
    app.project_worktree_enabled.insert("/workspace/project-a".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(4);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(4));

    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert_eq!(app.recent_projects_edits, vec!["Project A"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/project-a");
    assert_eq!(app.project_worktree_enabled.get("/workspace/project-a"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
}

#[test]
fn worktree_toggle_inserts_enabled_entry_and_removes_disabled_entry() {
    let mut app = app();
    let path = "/workspace/project-a";

    let _task =
        update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.to_string(), true));

    assert_eq!(app.project_worktree_enabled.get(path), Some(&true));

    let _task =
        update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.to_string(), false));

    assert!(!app.project_worktree_enabled.contains_key(path));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_settings_message_leaves_project_state_unchanged() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-a", "Project A")];
    app.project_worktree_enabled.insert("/workspace/project-a".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert_eq!(app.recent_projects_edits, vec!["Project A"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/project-a");
    assert_eq!(app.project_worktree_enabled.get("/workspace/project-a"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, Some(0));
}
