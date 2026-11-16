//! 全局状态投影与消息包装。
use crate::app::{App, Message};
use desktop_task_board::control::{self, ControlView};

pub fn build_control_buttons(app: &App) -> iced::Element<'_, Message> {
    let entries = app.agents_settings.entries.iter().map(|e| {
        desktop_task_model::agent_activity::AgentEntry {
            key: &e.key,
            label: &e.label,
            enabled: e.enabled,
        }
    }).collect();
    control::build_control_buttons(&ControlView {
        desktop_locale: app.general_settings.desktop_locale,
        window_size: app.window_size,
        project_path: app.navigation_tabs.project_path.as_deref(),
        task_board_tasks: &app.task_board.task_board_tasks,
        task_board_executor_running: app.task_board.task_board_executor_running,
        task_board_worktree_manual_action_kind: app.task_board.task_board_worktree_manual_action_kind,
        task_board_worktree_manual_confirm_kind: app.task_board.task_board_worktree_manual_confirm_kind,
        task_board_worktree_panel_expanded: app.task_board.task_board_worktree_panel_expanded,
        task_board_worktree_pixel_office: app.task_board.task_board_worktree_pixel_office,
        task_board_worktree_snapshot: app.task_board.task_board_worktree_snapshot.as_ref(),
        task_board_worktree_snapshot_loading: app.task_board.task_board_worktree_snapshot_loading,
        task_board_worktree_action_logs: &app.task_board.task_board_worktree_action_logs,
        agent_entries: entries,
    }).map(Message::TaskBoard)
}

#[cfg(test)]
#[path = "control/integration_tests.rs"]
mod integration_tests;
