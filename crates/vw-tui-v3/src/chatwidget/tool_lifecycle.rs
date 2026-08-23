//! Product-neutral tool event adapters for the copied command lifecycle.

use super::*;

impl ChatWidget {
    pub(super) fn on_tool_started(
        &mut self,
        turn_id: String,
        call_id: String,
        tool: String,
        command: String,
    ) {
        self.on_command_started(&turn_id, call_id, tool, command);
    }

    pub(super) fn on_tool_output_delta(
        &mut self,
        turn_id: String,
        call_id: String,
        delta: String,
    ) {
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
        self.on_command_finished(
            &turn_id,
            call_id,
            output,
            exit_code,
            duration_ms,
        );
    }

    pub(super) fn on_tool_failed(
        &mut self,
        turn_id: String,
        call_id: String,
        message: String,
        duration_ms: u64,
    ) {
        self.on_command_finished(&turn_id, call_id, message, 1, duration_ms);
    }

    pub(super) fn on_tool_cancelled(
        &mut self,
        turn_id: String,
        call_id: String,
        reason: String,
        duration_ms: u64,
    ) {
        self.on_command_finished(&turn_id, call_id, reason, 130, duration_ms);
    }
}
