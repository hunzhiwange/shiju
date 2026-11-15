//! Deterministic `V3Event` reduction into copied ChatWidget presentation state.
//! 本模块负责`V3Event` 到展示状态的确定性归约。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
//! 归约过程不执行 I/O，也不拥有 Gateway、会话、审批或命令执行权限。

use super::*;
use crate::vibe::event::V3Event;

impl ChatWidget {
    /// Applies one product-neutral UI lifecycle event.
    ///
    /// The reducer performs no I/O and has no Gateway, session, approval, or
    /// command-execution authority.
    /// 应用一个产品无关的 UI 生命周期事件。归约器不执行 I/O，也不拥有 Gateway、会话、审批或命令执行权限。
    pub fn handle_v3_event(&mut self, event: V3Event) {
        match event {
            V3Event::TurnStarted { turn_id } => self.on_turn_started(turn_id),
            V3Event::ThinkingDelta { turn_id, delta } => {
                self.on_thinking_delta(&turn_id, delta);
            }
            V3Event::TextDelta { turn_id, delta } => self.on_text_delta(&turn_id, delta),
            V3Event::ToolStarted { turn_id, call_id, tool, command } => {
                self.on_tool_started(turn_id, call_id, tool, command)
            }
            V3Event::ToolOutputDelta { turn_id, call_id, delta } => {
                self.on_tool_output_delta(turn_id, call_id, delta)
            }
            V3Event::ToolCompleted { turn_id, call_id, output, exit_code, duration_ms } => {
                self.on_tool_completed(turn_id, call_id, output, exit_code, duration_ms)
            }
            V3Event::FileDiff { turn_id, diff } => self.on_file_diff(turn_id, diff),
            V3Event::ToolFailed { turn_id, call_id, message, duration_ms } => {
                self.on_tool_failed(turn_id, call_id, message, duration_ms)
            }
            V3Event::ToolCancelled { turn_id, call_id, reason, duration_ms } => {
                self.on_tool_cancelled(turn_id, call_id, reason, duration_ms)
            }
            V3Event::TurnCompleted { turn_id } => self.on_turn_completed(turn_id),
            V3Event::TurnFailed { turn_id, message } => self.on_turn_failed(turn_id, message),
            V3Event::TurnCancelled { turn_id, reason } => {
                self.on_turn_cancelled(turn_id, reason);
            }
            V3Event::Unknown { type_name, detail } => self.push_diagnostic(type_name, detail),
        }
    }
}
