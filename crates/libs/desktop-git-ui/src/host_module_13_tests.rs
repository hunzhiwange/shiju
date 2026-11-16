//! 提交入口原断言与迁移的整组控制器测试。
use crate::git_panel::types::ConventionalCommitType;
use crate::{
    message_git::{controller, messages::GitMessage, stage_commit::Effect},
    state::State,
};
struct Fixture {
    git: State,
    project_path: Option<String>,
    active_session_id: Option<String>,
    selected_git_worktree_directory: Option<String>,
    error_message: Option<String>,
    effects: Vec<Effect>,
}
impl Fixture {
    fn new() -> (Self, ()) {
        (
            Self {
                git: State::default(),
                project_path: None,
                active_session_id: None,
                selected_git_worktree_directory: None,
                error_message: None,
                effects: Vec::new(),
            },
            (),
        )
    }
}
fn update(app: &mut Fixture, message: GitMessage) -> Vec<Effect> {
    let ctx = controller::Context {
        context_path: crate::message_git::shared::git_context_path(
            app.selected_git_worktree_directory.as_deref(),
            None,
            app.project_path.as_deref(),
        ),
        ..Default::default()
    };
    let mut effects = Vec::new();
    let _ = controller::update_stage(&mut app.git, &ctx, &mut effects, message);
    let effects = effects
        .into_iter()
        .map(|effect| match effect {
            controller::Effect::Commit(effect) => effect,
            _ => unreachable!("stage effects"),
        })
        .collect::<Vec<_>>();
    for effect in &effects {
        match effect {
            Effect::Error(error) => app.error_message = Some(error.clone()),
            Effect::InputError(error) => app.error_message = Some((*error).to_owned()),
            _ => {}
        }
    }
    app.effects = effects.clone();
    effects
}
// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_rejects_blank_message_without_starting_commit() {
    let (mut app, _task) = Fixture::new();
    app.git.git_commit_message = "   ".to_string();
    app.git.git_commit_description.clear();
    app.git.git_commit_type = None;

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_all_reuses_commit_validation() {
    let (mut app, _task) = Fixture::new();
    app.git.git_commit_message = "   ".to_string();
    app.git.git_commit_description.clear();
    app.git.git_commit_type = None;

    let _task = update(&mut app, GitMessage::CommitAll);

    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn stage_all_requires_project_context_and_reports_finished_states() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;

    let _task = update(&mut app, GitMessage::StageAll);
    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关暂存"));

    app.project_path = Some("/repo".to_string());
    app.error_message = None;
    let task = update(&mut app, GitMessage::StageAll);
    assert!(app.git.git_commit_in_progress);
    assert!(task.len() > 0);

    let _task = update(&mut app, GitMessage::StageAllFinished(Ok(())));
    assert!(!app.git.git_commit_in_progress);
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "暂存完成")));

    app.git.git_commit_in_progress = true;
    let _task = update(&mut app, GitMessage::StageAllFinished(Err("failed".to_string())));
    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "暂存失败: failed")));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_while_in_progress_preserves_existing_state() {
    let (mut app, _task) = Fixture::new();
    app.git.git_commit_in_progress = true;
    app.git.git_commit_message = "summary".to_string();
    app.error_message = Some("existing error".to_string());

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(app.git.git_commit_in_progress);
    assert_eq!(app.git.git_commit_message, "summary");
    assert_eq!(app.error_message.as_deref(), Some("existing error"));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_without_project_path_restores_idle_state_and_reports_error() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;
    app.git.git_commit_message = "summary".to_string();

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关提交"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "未找到项目路径，无法通过网关提交")));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_with_valid_context_enters_in_progress_state() {
    let (mut app, _task) = Fixture::new();
    app.project_path = Some("/repo".to_string());
    app.git.git_commit_message = "summary".to_string();
    app.git.git_commit_in_progress = false;
    app.error_message = None;

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(app.git.git_commit_in_progress);
    assert!(app.error_message.is_none());
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_finished_success_resets_form_and_reports_success() {
    let (mut app, _task) = Fixture::new();
    app.git.git_commit_in_progress = true;
    app.git.git_commit_message = "summary".to_string();
    app.git.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git.git_commit_scope = "git".to_string();
    app.git.git_commit_description = "details".to_string();
    app.git.staged_files_selected = vec!["a.rs".to_string()];
    app.git.staged_hunks_selected = vec![("a.rs".to_string(), 1)];
    app.git.staged_lines_selected = vec![("a.rs".to_string(), 2)];
    app.git.staged_old_lines_selected = vec![("a.rs".to_string(), 3)];

    let _task = update(&mut app, GitMessage::CommitSelectedFinished(Ok(())));

    assert!(!app.git.git_commit_in_progress);
    assert!(app.git.git_commit_message.is_empty());
    assert_eq!(app.git.git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(app.git.git_commit_scope.is_empty());
    assert!(app.git.git_commit_description.is_empty());
    assert!(app.git.staged_files_selected.is_empty());
    assert!(app.git.staged_hunks_selected.is_empty());
    assert!(app.git.staged_lines_selected.is_empty());
    assert!(app.git.staged_old_lines_selected.is_empty());
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "提交完成")));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_finished_error_keeps_form_and_reports_error() {
    let (mut app, _task) = Fixture::new();
    app.git.git_commit_in_progress = true;
    app.git.git_commit_message = "summary".to_string();
    app.git.staged_files_selected = vec!["a.rs".to_string()];

    let _task = update(&mut app, GitMessage::CommitSelectedFinished(Err("failed".to_string())));

    assert!(!app.git.git_commit_in_progress);
    assert_eq!(app.git.git_commit_message, "summary");
    assert_eq!(app.git.staged_files_selected, vec!["a.rs"]);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "提交失败: failed")));
}

mod diff_tests;

mod modal_tests;

mod refresh_tests;

mod tests;

mod shared_tests;
