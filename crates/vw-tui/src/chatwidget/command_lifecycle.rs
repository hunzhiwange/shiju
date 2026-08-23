//! Command grouping and copied exec-cell lifecycle integration.
//! 本模块负责命令分组与执行单元生命周期。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::*;
use crate::chatwidget::exec_state::CallMutation;

impl ChatWidget {
    // 处理 `on_command_started` 对应的界面事件并同步相关状态。
    pub(super) fn on_command_started(
        &mut self,
        turn_id: &str,
        call_id: String,
        tool: String,
        command: String,
    ) {
        if !self.accept_turn_event(turn_id, "tool start") {
            return;
        }
        self.commit_stream_segment();
        match self.exec_state.start(call_id.clone(), tool, command) {
            CallMutation::Applied => self.sync_exec_group(),
            CallMutation::Duplicate => {
                self.push_diagnostic("duplicate tool start", format!("call_id={call_id}"));
            }
            CallMutation::AlreadyTerminal => {
                self.push_diagnostic("tool start after terminal", format!("call_id={call_id}"));
            }
            CallMutation::MissingStart => unreachable!("start cannot be missing"),
        }
    }

    // 接收 `on_command_output_delta` 对应事件，并只更新本地展示状态。
    pub(super) fn on_command_output_delta(
        &mut self,
        turn_id: &str,
        call_id: String,
        delta: String,
    ) {
        if !self.accept_turn_event(turn_id, "tool output delta") {
            return;
        }
        match self.exec_state.append_output(&call_id, &delta) {
            CallMutation::Applied => {}
            CallMutation::MissingStart => {
                self.push_diagnostic("tool output without start", format!("call_id={call_id}"));
                let _ = self.exec_state.start(
                    call_id.clone(),
                    "unknown".to_string(),
                    format!("[missing tool start: {call_id}]"),
                );
                let _ = self.exec_state.append_output(&call_id, &delta);
            }
            CallMutation::AlreadyTerminal => {
                self.push_diagnostic("tool output after terminal", format!("call_id={call_id}"));
                return;
            }
            CallMutation::Duplicate => unreachable!("output mutation cannot be duplicate"),
        }
        self.sync_exec_group();
    }

    pub(super) fn on_command_finished(
        &mut self,
        turn_id: &str,
        call_id: String,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    ) {
        if !self.accept_turn_event(turn_id, "tool terminal") {
            return;
        }
        let mut result = self.exec_state.finish(
            &call_id,
            output.clone(),
            exit_code,
            duration_ms,
        );
        if result == CallMutation::MissingStart {
            self.push_diagnostic("tool terminal without start", format!("call_id={call_id}"));
            let _ = self.exec_state.start(
                call_id.clone(),
                "unknown".to_string(),
                format!("[missing tool start: {call_id}]"),
            );
            result = self.exec_state.finish(
                &call_id,
                output,
                exit_code,
                duration_ms,
            );
        }
        match result {
            CallMutation::Applied => self.sync_exec_group(),
            CallMutation::AlreadyTerminal => {
                self.push_diagnostic("duplicate tool terminal", format!("call_id={call_id}"));
            }
            CallMutation::Duplicate | CallMutation::MissingStart => unreachable!(),
        }
    }

    pub(super) fn sync_exec_group(&mut self) {
        if let Some(cell) = self.exec_state.snapshot() {
            self.transcript.active_cell = Some(Box::new(cell));
            self.transcript.bump_active_cell_revision();
            self.transcript.set_live_tail_event(None);
        }
    }

    pub(super) fn commit_exec_group(&mut self) {
        if self.exec_state.is_active() {
            self.sync_exec_group();
            self.transcript.commit_active_cell();
            self.exec_state.clear_group();
        }
    }
}
