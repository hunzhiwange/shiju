//! Working-directory slash commands for the current local session.
//! 本模块负责当前本地会话的工作目录命令。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;
use crate::history_cell::McpInventoryLoadingCell;

impl ChatWidget {
    // 判断 `can_change_working_directory` 对应的界面条件是否成立。
    pub(crate) fn can_change_working_directory(&self, thread_id: ThreadId) -> bool {
        let active = &self.transcript.active_cell;
        self.thread_id == Some(thread_id)
            && !self.active_side_conversation
            && !self.blocks_direct_input
            && !self.config.ephemeral
            && self.unified_exec_processes.is_empty()
            && !self.is_user_turn_pending_or_running()
            && !self.input_queue.has_queued_follow_up_messages()
            && self.input_queue.pending_steers.is_empty()
            && active
                .as_ref()
                .is_none_or(|cell| !cell.as_any().is::<McpInventoryLoadingCell>())
    }

    // 计算 `request_working_directory_change` 在当前界面流程中需要的局部结果。
    pub(super) fn request_working_directory_change(&mut self, path: &str) {
        if !self.is_session_configured() {
            self.add_error_message(
                "The session must start before you can change its working directory.".to_string(),
            );
            return;
        }

        let Some(thread_id) = self.thread_id else {
            return;
        };
        if !self.can_change_working_directory(thread_id) {
            let message =
                "Changing directories requires an idle primary session without queued input.";
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                history_cell::new_error_event(message.to_string()),
            )));
            return;
        }

        self.app_event_tx.send(AppEvent::ChangeWorkingDirectory {
            thread_id,
            requested_cwd: PathBuf::from(path),
        });
    }
}
