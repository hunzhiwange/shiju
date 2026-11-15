//! 日志查看器滚动消息更新。
use crate::message::TaskBoardMessage;
use crate::helpers::logs::{TaskLogsState, close_logs_context_menu, apply_task_logs_scroll_lines, task_logs_max_scroll_top_line};

pub fn update_logs(mut state: TaskLogsState<'_>, message: TaskBoardMessage) -> Result<(), TaskBoardMessage> {
    match message {
        TaskBoardMessage::LogsViewerEditorAction(action) => {
            close_logs_context_menu(&mut state);
            if let iced::widget::text_editor::Action::Scroll { lines } = &action {
                apply_task_logs_scroll_lines(&mut state, *lines);
            }
            (*state.task_board_logs_editor).perform(action);
        }
        TaskBoardMessage::LogsViewerEditorWheelScrolled { delta, viewport_height } => {
            close_logs_context_menu(&mut state);
            (*state.task_board_logs_viewport_height) = viewport_height.max(0.0);

            let line_height = (*state.current_line_height).max(1.0);
            let delta_lines = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => -y * 1.25,
                iced::mouse::ScrollDelta::Pixels { y, .. } => -y / line_height,
            };

            (*state.task_board_logs_scroll_remainder) += delta_lines;

            let whole_lines = if (*state.task_board_logs_scroll_remainder) >= 0.0 {
                (*state.task_board_logs_scroll_remainder).floor() as i32
            } else {
                (*state.task_board_logs_scroll_remainder).ceil() as i32
            };

            if whole_lines != 0 {
                (*state.task_board_logs_scroll_remainder) -= whole_lines as f32;
                apply_task_logs_scroll_lines(&mut state, whole_lines);
                (*state.task_board_logs_editor)
                    .perform(iced::widget::text_editor::Action::Scroll { lines: whole_lines });
            }

        }
        TaskBoardMessage::LogsViewerScrollbarChanged { top_line, viewport_height } => {
            close_logs_context_menu(&mut state);
            (*state.task_board_logs_viewport_height) = viewport_height.max(0.0);

            let max_scroll = task_logs_max_scroll_top_line(&state);
            let target_top_line = top_line.round().clamp(0.0, max_scroll);
            let current_top_line = (*state.task_board_logs_scroll_top_line).round();
            let delta = (target_top_line - current_top_line) as i32;

            if delta != 0 {
                apply_task_logs_scroll_lines(&mut state, delta);
                (*state.task_board_logs_editor)
                    .perform(iced::widget::text_editor::Action::Scroll { lines: delta });
            }

        }
        other => return Err(other),
    }
    Ok(())
}
