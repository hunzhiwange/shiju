//! 原宿主提交入口用例，以局部字段夹具运行真实领域 update。
use crate::git_panel::types::ConventionalCommitType;
use crate::message_git::{
    messages::GitMessage,
    stage_commit::{self, Effect, UpdateInput},
};
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet};
struct Fixture {
    context_expansions: HashMap<(String, usize), (usize, usize)>,
    expanded_files: Vec<String>,
    expanded_files_set: HashSet<String>,
    expanded_hunks: Vec<(String, usize)>,
    git_commit_description: String,
    git_commit_description_editor: text_editor::Content,
    git_commit_in_progress: bool,
    git_commit_message: String,
    git_commit_scope: String,
    git_commit_type: Option<ConventionalCommitType>,
    git_filter_deleted: bool,
    git_filter_excluded: bool,
    git_filter_included: bool,
    git_filter_modified: bool,
    git_filter_new: bool,
    git_filter_query: String,
    git_hovered_file_header: Option<String>,
    git_panel_header_hovered: bool,
    show_git_commit_help_modal: bool,
    show_git_filter_help_modal: bool,
    show_git_filter_options: bool,
    staged_files_selected: Vec<String>,
    staged_hunks_selected: Vec<(String, usize)>,
    staged_lines_selected: Vec<(String, usize)>,
    staged_old_lines_selected: Vec<(String, usize)>,
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
                context_expansions: Default::default(),
                expanded_files: Default::default(),
                expanded_files_set: Default::default(),
                expanded_hunks: Default::default(),
                git_commit_description: Default::default(),
                git_commit_description_editor: text_editor::Content::new(),
                git_commit_in_progress: Default::default(),
                git_commit_message: Default::default(),
                git_commit_scope: Default::default(),
                git_commit_type: Default::default(),
                git_filter_deleted: Default::default(),
                git_filter_excluded: Default::default(),
                git_filter_included: Default::default(),
                git_filter_modified: Default::default(),
                git_filter_new: Default::default(),
                git_filter_query: Default::default(),
                git_hovered_file_header: Default::default(),
                git_panel_header_hovered: Default::default(),
                show_git_commit_help_modal: Default::default(),
                show_git_filter_help_modal: Default::default(),
                show_git_filter_options: Default::default(),
                staged_files_selected: Default::default(),
                staged_hunks_selected: Default::default(),
                staged_lines_selected: Default::default(),
                staged_old_lines_selected: Default::default(),
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
    let context_path = crate::message_git::shared::git_context_path(
        app.selected_git_worktree_directory.as_deref(),
        app.active_session_id.as_deref(),
        app.project_path.as_deref(),
    );
    let effects = stage_commit::update(
        &mut UpdateInput {
            context_expansions: &mut app.context_expansions,
            expanded_files: &mut app.expanded_files,
            expanded_files_set: &mut app.expanded_files_set,
            expanded_hunks: &mut app.expanded_hunks,
            git_commit_description: &mut app.git_commit_description,
            git_commit_description_editor: &mut app.git_commit_description_editor,
            git_commit_in_progress: &mut app.git_commit_in_progress,
            git_commit_message: &mut app.git_commit_message,
            git_commit_scope: &mut app.git_commit_scope,
            git_commit_type: &mut app.git_commit_type,
            git_filter_deleted: &mut app.git_filter_deleted,
            git_filter_excluded: &mut app.git_filter_excluded,
            git_filter_included: &mut app.git_filter_included,
            git_filter_modified: &mut app.git_filter_modified,
            git_filter_new: &mut app.git_filter_new,
            git_filter_query: &mut app.git_filter_query,
            git_hovered_file_header: &mut app.git_hovered_file_header,
            git_panel_header_hovered: &mut app.git_panel_header_hovered,
            show_git_commit_help_modal: &mut app.show_git_commit_help_modal,
            show_git_filter_help_modal: &mut app.show_git_filter_help_modal,
            show_git_filter_options: &mut app.show_git_filter_options,
            staged_files_selected: &mut app.staged_files_selected,
            staged_hunks_selected: &mut app.staged_hunks_selected,
            staged_lines_selected: &mut app.staged_lines_selected,
            staged_old_lines_selected: &mut app.staged_old_lines_selected,
            remote_busy: false,
            context_path,
            repo_path: None,
            animate: true,
        },
        message,
    );
    // Feedback is asserted from the actual returned effects; no App or gateway fixture.
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
    app.git_commit_message = "   ".to_string();
    app.git_commit_description.clear();
    app.git_commit_type = None;

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("提交消息不能为空"));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_all_reuses_commit_validation() {
    let (mut app, _task) = Fixture::new();
    app.git_commit_message = "   ".to_string();
    app.git_commit_description.clear();
    app.git_commit_type = None;

    let _task = update(&mut app, GitMessage::CommitAll);

    assert!(!app.git_commit_in_progress);
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
    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关暂存"));

    app.project_path = Some("/repo".to_string());
    app.error_message = None;
    let task = update(&mut app, GitMessage::StageAll);
    assert!(app.git_commit_in_progress);
    assert!(task.len() > 0);

    let _task = update(&mut app, GitMessage::StageAllFinished(Ok(())));
    assert!(!app.git_commit_in_progress);
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "暂存完成")));

    app.git_commit_in_progress = true;
    let _task = update(&mut app, GitMessage::StageAllFinished(Err("failed".to_string())));
    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "暂存失败: failed")));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_while_in_progress_preserves_existing_state() {
    let (mut app, _task) = Fixture::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.error_message = Some("existing error".to_string());

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.error_message.as_deref(), Some("existing error"));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_without_project_path_restores_idle_state_and_reports_error() {
    let (mut app, _task) = Fixture::new();
    app.project_path = None;
    app.active_session_id = None;
    app.selected_git_worktree_directory = None;
    app.git_commit_message = "summary".to_string();

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.error_message.as_deref(), Some("未找到项目路径，无法通过网关提交"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "未找到项目路径，无法通过网关提交")));
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_with_valid_context_enters_in_progress_state() {
    let (mut app, _task) = Fixture::new();
    app.project_path = Some("/repo".to_string());
    app.git_commit_message = "summary".to_string();
    app.git_commit_in_progress = false;
    app.error_message = None;

    let _task = update(&mut app, GitMessage::CommitSelected);

    assert!(app.git_commit_in_progress);
    assert!(app.error_message.is_none());
}

// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_finished_success_resets_form_and_reports_success() {
    let (mut app, _task) = Fixture::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "git".to_string();
    app.git_commit_description = "details".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];
    app.staged_hunks_selected = vec![("a.rs".to_string(), 1)];
    app.staged_lines_selected = vec![("a.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 3)];

    let _task = update(&mut app, GitMessage::CommitSelectedFinished(Ok(())));

    assert!(!app.git_commit_in_progress);
    assert!(app.git_commit_message.is_empty());
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(app.git_commit_scope.is_empty());
    assert!(app.git_commit_description.is_empty());
    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "提交完成")));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
// 原入口断言：现在通过局部输入调用真实领域更新。
#[test]
fn commit_selected_finished_error_keeps_form_and_reports_error() {
    let (mut app, _task) = Fixture::new();
    app.git_commit_in_progress = true;
    app.git_commit_message = "summary".to_string();
    app.staged_files_selected = vec!["a.rs".to_string()];

    let _task = update(&mut app, GitMessage::CommitSelectedFinished(Err("failed".to_string())));

    assert!(!app.git_commit_in_progress);
    assert_eq!(app.git_commit_message, "summary");
    assert_eq!(app.staged_files_selected, vec!["a.rs"]);
    assert_eq!(app.error_message.as_deref(), Some("failed"));
    assert!(app.effects.iter().any(|effect| matches!(effect, Effect::Notify { message, error } if message.replace("%s", error.as_deref().unwrap_or("")) == "提交失败: failed")));
}
