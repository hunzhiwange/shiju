//! 手动 worktree 操作的有界日志与接收器生命周期。
use super::execution::{
    TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES, TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS,
    TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS, format_task_log_stream_for_ui,
};
use desktop_task_executor::TaskLogStream;
use runtime_time::now_ms;
pub struct WorktreeActionLogs<'a> {
    pub task_board_worktree_manual_action_kind: &'a mut Option<&'static str>,
    pub task_board_worktree_manual_confirm_kind: &'a mut Option<&'static str>,
    pub task_board_worktree_action_logs: &'a mut Vec<String>,
    pub task_board_worktree_action_logs_visible_until_ms: &'a mut Option<u64>,
    pub task_board_worktree_action_log_rx: &'a mut Option<std::sync::mpsc::Receiver<TaskLogStream>>,
}
pub fn push_worktree_action_log(app: &mut WorktreeActionLogs<'_>, line: String) {
    app.task_board_worktree_action_logs.push(line);
    if app.task_board_worktree_action_logs.len() > TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES {
        let overflow =
            app.task_board_worktree_action_logs.len() - TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES;
        app.task_board_worktree_action_logs.drain(0..overflow);
    }
}
pub fn clear_worktree_action_logs(app: &mut WorktreeActionLogs<'_>) {
    *app.task_board_worktree_manual_action_kind = None;
    *app.task_board_worktree_manual_confirm_kind = None;
    app.task_board_worktree_action_logs.clear();
    *app.task_board_worktree_action_logs_visible_until_ms = None;
    *app.task_board_worktree_action_log_rx = None;
}
pub fn finish_worktree_action_logs(app: &mut WorktreeActionLogs<'_>) {
    *app.task_board_worktree_manual_action_kind = None;
    *app.task_board_worktree_manual_confirm_kind = None;
    *app.task_board_worktree_action_log_rx = None;
    if app.task_board_worktree_action_logs.is_empty() {
        *app.task_board_worktree_action_logs_visible_until_ms = None;
    } else {
        *app.task_board_worktree_action_logs_visible_until_ms =
            Some(now_ms().saturating_add(TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS));
    }
}
pub fn poll_worktree_action_logs(app: &mut WorktreeActionLogs<'_>) {
    let Some(receiver) = app.task_board_worktree_action_log_rx.take() else {
        return;
    };

    loop {
        match receiver.try_recv() {
            Ok(log) => {
                if let Some(line) = format_task_log_stream_for_ui(
                    &log,
                    TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS,
                ) {
                    push_worktree_action_log(app, line);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                *app.task_board_worktree_action_log_rx = Some(receiver);
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                *app.task_board_worktree_action_log_rx = None;
                break;
            }
        }
    }
}
