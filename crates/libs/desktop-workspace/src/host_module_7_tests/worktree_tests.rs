use crate::project_message::session_worktree::{self, Effect, ErrorField, Event, Picker};
#[derive(Default)]
struct Fixture {
    new_session_picker_project: Option<String>,
    new_session_picker_options: Vec<(String, String)>,
    new_session_last_directory: Option<String>,
    new_session_worktree_name: String,
    new_session_creating: bool,
    new_session_confirm_delete_directory: Option<String>,
    new_session_force_delete_directory: Option<String>,
    new_session_delete_error: Option<String>,
    new_session_confirm_reset_directory: Option<String>,
    new_session_reset_error: Option<String>,
    hovered_recent_project: Option<String>,
    error_message: Option<String>,
    notifications: Vec<Notification>,
}
struct Notification {
    message: String,
}
fn app() -> Fixture {
    Fixture::default()
}
// Interpret only localization and notification boundary effects; all picker transitions use production update.
fn apply(app: &mut Fixture, event: Event) -> Option<Vec<Effect>> {
    let effects = session_worktree::update(
        &mut Picker {
            project: &mut app.new_session_picker_project,
            options: &mut app.new_session_picker_options,
            last_directory: &mut app.new_session_last_directory,
            name: &mut app.new_session_worktree_name,
            creating: &mut app.new_session_creating,
            confirm_delete: &mut app.new_session_confirm_delete_directory,
            force_delete: &mut app.new_session_force_delete_directory,
            delete_error: &mut app.new_session_delete_error,
            confirm_reset: &mut app.new_session_confirm_reset_directory,
            reset_error: &mut app.new_session_reset_error,
        },
        None,
        0,
        event,
    );
    for effect in &effects {
        match effect {
            Effect::ClearHover => app.hovered_recent_project = None,
            Effect::InputError(field, template, args) => {
                let mut message = template.to_string();
                for arg in args {
                    message = message.replacen("%s", arg, 1);
                }
                match field {
                    ErrorField::General => app.error_message = Some(message),
                    ErrorField::Delete => app.new_session_delete_error = Some(message),
                    ErrorField::Reset => app.new_session_reset_error = Some(message),
                }
            }
            Effect::ResetNotification => {
                app.notifications.push(Notification { message: "工作区已重置".into() })
            }
            _ => {}
        }
    }
    Some(effects)
}

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("worktree_tests"));
}

#[test]
fn picker_loaded_ignores_stale_project_and_falls_back_on_error() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/current".to_string());
    app.new_session_picker_options = vec![("keep".to_string(), "保留".to_string())];

    let _ = apply(
        &mut app,
        Event::Loaded {
            project_path: "/tmp/stale".to_string(),
            options: Ok(vec![("stale".to_string(), "过期".to_string())]),
        },
    );
    assert_eq!(app.new_session_picker_options, vec![("keep".to_string(), "保留".to_string())]);

    let _ = apply(
        &mut app,
        Event::Loaded {
            project_path: "/tmp/current".to_string(),
            options: Err("gateway down".to_string()),
        },
    );

    assert_eq!(app.error_message.as_deref(), Some("加载工作区列表失败: gateway down"));
    assert_eq!(
        app.new_session_picker_options,
        vec![("__create_worktree__".to_string(), "创建新的独立工作区".to_string())]
    );
}

#[test]
fn picker_close_and_name_change_update_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_picker_options = vec![("dir".to_string(), "Dir".to_string())];

    let _ = apply(&mut app, Event::Name("feature-login".to_string()));
    assert_eq!(app.new_session_worktree_name, "feature-login");

    let _ = apply(&mut app, Event::Close);

    assert!(app.hovered_recent_project.is_none());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
}

#[test]
fn picking_existing_workspace_ignores_duplicate_while_creation_is_running() {
    let mut app = app();
    app.new_session_creating = true;
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_last_directory = Some("/tmp/previous".to_string());

    let task = apply(
        &mut app,
        Event::Picked {
            project_path: "/tmp/project".to_string(),
            directory: "/tmp/project-wt".to_string(),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/tmp/previous"));
    assert_eq!(app.new_session_picker_project.as_deref(), Some("/tmp/project"));
}

#[test]
fn create_worktree_validates_empty_and_invalid_names_before_async_call() {
    let mut app = app();
    let project = "/tmp/project".to_string();

    app.new_session_worktree_name = "   ".to_string();
    let _ = apply(&mut app, Event::Create(project.clone()));
    assert_eq!(app.error_message.as_deref(), Some("请输入 worktree 英文名称"));

    app.error_message = None;
    app.new_session_worktree_name = "Feature_Login".to_string();
    let _ = apply(&mut app, Event::Create(project.clone()));
    assert_eq!(app.error_message.as_deref(), Some("worktree 名称仅支持小写英文、数字、连字符(-)"));

    app.error_message = None;
    app.new_session_picker_project = Some(project.clone());
    app.new_session_worktree_name = "feature-login".to_string();
    let task = apply(&mut app, Event::Create(project));
    assert!(task.is_some());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_worktree_name.is_empty());
}

#[test]
fn delete_worktree_blocks_primary_and_sets_confirmation_for_secondary() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/project-wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_picker_options =
        vec![(project.clone(), "主工作区".to_string()), (worktree.clone(), "功能".to_string())];

    let _ = apply(&mut app, Event::Delete(project.clone()));
    assert_eq!(app.new_session_delete_error.as_deref(), Some("主工作区禁止删除"));
    assert!(app.new_session_confirm_delete_directory.is_none());

    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("old-dir".to_string());
    app.new_session_confirm_reset_directory = Some("old-reset".to_string());
    let _ = apply(&mut app, Event::Delete(worktree.clone()));

    assert_eq!(app.new_session_confirm_delete_directory, Some(worktree));
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
}

#[test]
fn delete_worktree_cancel_and_missing_confirmation_are_noops() {
    let mut app = app();
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_delete_error = Some("error".to_string());
    app.new_session_reset_error = Some("reset".to_string());

    let _ = apply(&mut app, Event::DeleteCancel);
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_reset_error.is_none());

    let task = apply(&mut app, Event::DeleteConfirmed);
    assert!(task.is_some());
}

#[test]
fn delete_worktree_confirmed_requires_project_and_directory() {
    let mut app = app();
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());

    let task = apply(&mut app, Event::DeleteConfirmed);
    assert!(task.is_some());
    assert_eq!(app.new_session_confirm_delete_directory.as_deref(), Some("/tmp/wt"));

    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_delete_directory = None;
    let task = apply(&mut app, Event::DeleteConfirmed);
    assert!(task.is_some());
    assert!(app.new_session_confirm_delete_directory.is_none());
}

#[test]
fn delete_worktree_confirmed_for_secondary_clears_messages_before_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_force_delete_directory = Some("/tmp/old-force".to_string());
    app.new_session_delete_error = Some("old delete error".to_string());
    app.new_session_confirm_reset_directory = Some("/tmp/old-reset".to_string());
    app.new_session_reset_error = Some("old reset error".to_string());

    let task = apply(&mut app, Event::DeleteConfirmed);

    assert!(task.is_some());
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn delete_worktree_confirmed_blocks_primary_and_result_handles_success_error_and_stale_project() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_confirm_delete_directory = Some(project.clone());

    let _ = apply(&mut app, Event::DeleteConfirmed);
    assert_eq!(app.new_session_delete_error.as_deref(), Some("主工作区禁止删除"));

    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("old".to_string());
    let _ = apply(
        &mut app,
        Event::Deleted {
            project_path: "/tmp/stale".to_string(),
            directory: worktree.clone(),
            result: Ok(()),
        },
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("old"));

    let _ = apply(
        &mut app,
        Event::Deleted {
            project_path: project.clone(),
            directory: worktree.clone(),
            result: Err("dirty worktree".to_string()),
        },
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("dirty worktree"));
    assert_eq!(app.new_session_force_delete_directory, Some(worktree.clone()));

    app.new_session_last_directory = Some(worktree.clone());
    let task = apply(
        &mut app,
        Event::Deleted { project_path: project, directory: worktree, result: Ok(()) },
    );
    assert!(task.is_some());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_last_directory.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn delete_worktree_success_preserves_a_different_last_directory() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_last_directory = Some("/tmp/recent-wt".to_string());
    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("/tmp/deleted-wt".to_string());
    app.new_session_reset_error = Some("old reset".to_string());

    let task = apply(
        &mut app,
        Event::Deleted {
            project_path: project,
            directory: "/tmp/deleted-wt".to_string(),
            result: Ok(()),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/tmp/recent-wt"));
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn force_delete_requires_directory_then_starts_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());

    let _ = apply(&mut app, Event::ForceDeleteConfirmed);
    assert!(app.new_session_force_delete_directory.is_none());

    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());
    let task = apply(&mut app, Event::ForceDeleteConfirmed);
    assert!(task.is_some());
    assert!(app.new_session_force_delete_directory.is_none());
}

#[test]
fn force_delete_requires_an_active_project() {
    let mut app = app();
    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());

    let task = apply(&mut app, Event::ForceDeleteConfirmed);

    assert!(task.is_some());
    assert_eq!(app.new_session_force_delete_directory.as_deref(), Some("/tmp/wt"));
}

#[test]
fn reset_worktree_blocks_primary_confirms_secondary_and_cancels() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_picker_options = vec![(project.clone(), "主工作区".to_string())];

    let _ = apply(&mut app, Event::Reset(project));
    assert_eq!(app.new_session_reset_error.as_deref(), Some("主工作区禁止重置"));
    assert!(app.new_session_confirm_reset_directory.is_none());

    app.new_session_picker_options.clear();
    let _ = apply(&mut app, Event::Reset(worktree.clone()));
    assert_eq!(app.new_session_confirm_reset_directory, Some(worktree));
    assert!(app.new_session_reset_error.is_none());

    let _ = apply(&mut app, Event::ResetCancel);
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn reset_worktree_confirmed_and_result_update_state() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();

    let _ = apply(&mut app, Event::ResetConfirmed);
    assert!(app.new_session_confirm_reset_directory.is_none());

    app.new_session_picker_project = Some(project.clone());
    app.new_session_confirm_reset_directory = Some(project.clone());
    let _ = apply(&mut app, Event::ResetConfirmed);
    assert_eq!(app.new_session_reset_error.as_deref(), Some("主工作区禁止重置"));

    app.new_session_reset_error = Some("old".to_string());
    let _ = apply(
        &mut app,
        Event::ResetDone { project_path: "/tmp/stale".to_string(), result: Ok(()) },
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("old"));

    let _ = apply(
        &mut app,
        Event::ResetDone { project_path: project.clone(), result: Err("reset failed".to_string()) },
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("reset failed"));

    let _ = apply(&mut app, Event::ResetDone { project_path: project, result: Ok(()) });
    assert!(app.new_session_reset_error.is_none());
    assert!(app.notifications.iter().any(|item| item.message == "工作区已重置"));
}

#[test]
fn reset_worktree_confirmed_for_secondary_clears_prompt_before_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_reset_directory = Some("/tmp/wt".to_string());
    app.new_session_reset_error = Some("old".to_string());

    let task = apply(&mut app, Event::ResetConfirmed);

    assert!(task.is_some());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
