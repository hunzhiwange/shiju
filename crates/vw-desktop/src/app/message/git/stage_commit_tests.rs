// 验证暂存选择、提交表单及变更行筛选的消息处理。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::message::git::GitMessage;
use crate::app::state::ConventionalCommitType;

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_rejects_blank_message_without_starting_commit() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_message = "   ".to_string();
    app.git_commit_description.clear();
    app.git_commit_type = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_all_reuses_commit_validation() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_message = "   ".to_string();
    app.git_commit_description.clear();
    app.git_commit_type = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitAll);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn stage_all_requires_project_context_and_reports_finished_states() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::StageAll);
    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关暂存"));

    app.project_path = Some("/repo".to_string());
    app.error_message = None;
    let task = super::stage_commit::update(&mut app, GitMessage::StageAll);
    assert!(app.git_commit_in_progress);
    assert!(task.units() > 0);

    let _task = super::stage_commit::update(&mut app, GitMessage::StageAllFinished(Ok(())));
    assert!(!app.git_commit_in_progress);
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("暂存完成")
    );

    app.git_commit_in_progress = true;
    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::StageAllFinished(Err("failed".to_string())),
    );
    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("暂存失败: failed")
    );
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_while_in_progress_preserves_existing_state() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.error_message = Some("existing error".to_string());

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.error_message.as_deref(), Some("existing error"));
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_without_project_path_restores_idle_state_and_reports_error() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;
    app.git_commit_message = "summary".to_string();

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关提交"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("未找到项目路径，无法通过网关提交")
    );
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_with_valid_context_enters_in_progress_state() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/repo".to_string());
    app.git_commit_message = "summary".to_string();
    app.git_commit_in_progress = false;
    app.error_message = None;

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert!(app.error_message.is_none());
}

// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_finished_success_resets_form_and_reports_success() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git".to_string();
    app.git_commit_description = "details".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];
    app.staged_hunks_selected = vec![("a.rs".to_string(), 1)];
    app.staged_lines_selected = vec![("a.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 3)];

    let _task = super::stage_commit::update(&mut app, GitMessage::CommitSelectedFinished(Ok(())));

    assert!(!app.git_commit_in_progress);
    assert!(app.git_commit_message.is_empty());
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(app.git_commit_scope.is_empty());
    assert!(app.git_commit_description.is_empty());
    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("提交完成")
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 宿主契约：核对 App 字段投影及 update 的消息/任务接线。
#[test]
fn commit_selected_finished_error_keeps_form_and_reports_error() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];

    let _task = super::stage_commit::update(
        &mut app,
        GitMessage::CommitSelectedFinished(Err("failed".to_string())),
    );

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.staged_files_selected, vec!["a.rs"]);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("提交失败: failed")
    );
}
