//! Product-neutral tool event adapters for the copied command lifecycle.
//! 本模块负责工具事件到命令生命周期的适配。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;

impl ChatWidget {
    // 处理 `on_tool_started` 对应的界面事件并同步相关状态。
    pub(super) fn on_tool_started(
        &mut self,
        turn_id: String,
        call_id: String,
        tool: String,
        command: String,
    ) {
        self.on_command_started(&turn_id, call_id, tool, command);
    }

    // 接收 `on_tool_output_delta` 对应事件，并只更新本地展示状态。
    pub(super) fn on_tool_output_delta(&mut self, turn_id: String, call_id: String, delta: String) {
        self.on_command_output_delta(&turn_id, call_id, delta);
    }

    pub(super) fn on_tool_completed(
        &mut self,
        turn_id: String,
        call_id: String,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    ) {
        self.on_command_finished(&turn_id, call_id, output, exit_code, duration_ms);
        self.flush_file_diffs_if_tool_group_finished();
    }

    pub(super) fn on_tool_failed(
        &mut self,
        turn_id: String,
        call_id: String,
        message: String,
        duration_ms: u64,
    ) {
        self.on_command_finished(&turn_id, call_id, message, 1, duration_ms);
        self.flush_file_diffs_if_tool_group_finished();
    }

    pub(super) fn on_tool_cancelled(
        &mut self,
        turn_id: String,
        call_id: String,
        reason: String,
        duration_ms: u64,
    ) {
        self.on_command_finished(&turn_id, call_id, reason, 130, duration_ms);
        self.flush_file_diffs_if_tool_group_finished();
    }

    pub(super) fn on_file_diff(&mut self, turn_id: String, diff: V3FileDiff) {
        if !self.accept_turn_event(&turn_id, "file diff") || diff.files.is_empty() {
            return;
        }
        self.pending_file_diffs.push(diff);
        self.flush_file_diffs_if_tool_group_finished();
    }

    fn flush_file_diffs_if_tool_group_finished(&mut self) {
        if self.pending_file_diffs.is_empty() || self.exec_state.has_running_calls() {
            return;
        }

        self.commit_exec_group();
        for diff in std::mem::take(&mut self.pending_file_diffs) {
            self.transcript.push_history(V3DisplayEvent::FileDiff { diff });
        }
    }
}
