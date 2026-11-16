//! 快捷提交消息的生成准备与请求身份隔离。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickMessageGitCommitAction {
    StageAllAndCommit,
    CommitSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickMessageHandoffState {
    WaitingForGitCommitMessage {
        request_id: u64,
        source_session_id: String,
        project_path: String,
        action: QuickMessageGitCommitAction,
    },
}

pub enum GitCommitPreparation {
    Warning(&'static str),
    Send(String),
}
pub fn prepare_git_commit(
    project_path: &str,
    action: &QuickMessageGitCommitAction,
    commit_in_progress: bool,
    requesting: bool,
    handoff_pending: bool,
    task_mode: bool,
    files: &[String],
    hunks: &[(String, usize)],
    lines: &[(String, usize)],
    old_lines: &[(String, usize)],
) -> GitCommitPreparation {
    let warning = if commit_in_progress {
        Some("Git 提交正在进行中")
    } else if requesting {
        Some("请等待当前回复完成后再生成 Git 提交信息")
    } else if handoff_pending {
        Some("另一个快捷动作正在进行中")
    } else if task_mode {
        Some("请先切换到聊天模式，再生成 Git 提交信息")
    } else if matches!(action, QuickMessageGitCommitAction::CommitSelected)
        && files.is_empty()
        && hunks.is_empty()
        && lines.is_empty()
        && old_lines.is_empty()
    {
        Some("请先选择要提交的 Git 变更")
    } else {
        None
    };
    if let Some(message) = warning {
        return GitCommitPreparation::Warning(message);
    }
    let target = match action {
        QuickMessageGitCommitAction::StageAllAndCommit => {
            "当前工作树中的全部 Git 变更。请只读检查这些变更，不要执行暂存或提交。".to_string()
        }
        QuickMessageGitCommitAction::CommitSelected => format!(
            "用户在 Git 面板中选中的变更。请只读检查仓库差异，不要执行暂存或提交。\n\
             选中范围：\n{}",
            super::session::git_commit_selection_context(files, hunks, lines, old_lines)
        ),
    };
    GitCommitPreparation::Send(super::session::git_commit_generation_prompt(project_path, &target))
}

pub fn clear_for_request(
    state: &mut Option<QuickMessageHandoffState>,
    request_id: u64,
) -> GitHandoffEffect {
    if matches!(state, Some(QuickMessageHandoffState::WaitingForGitCommitMessage { request_id: pending_id, .. }) if *pending_id == request_id)
    {
        *state = None;
        GitHandoffEffect::ClearCommitInProgress
    } else {
        GitHandoffEffect::None
    }
}

pub enum GitHandoffEffect {
    None,
    Waiting(QuickMessageHandoffState),
    ClearCommitInProgress,
}

pub fn begin_handoff(
    request_id: Option<u64>,
    source_session_id: String,
    project_path: String,
    action: QuickMessageGitCommitAction,
) -> GitHandoffEffect {
    match request_id {
        None => GitHandoffEffect::None,
        Some(request_id) => {
            GitHandoffEffect::Waiting(QuickMessageHandoffState::WaitingForGitCommitMessage {
                request_id,
                source_session_id,
                project_path,
                action,
            })
        }
    }
}
