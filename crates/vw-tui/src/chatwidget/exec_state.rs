//! Grouped command/tool bookkeeping retained by the copied exec-cell renderer.
//! 本模块负责分组命令和工具调用的执行单元展示状态。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use ratatui::text::Line;

use crate::exec_cell::{
    CommandOutput, ExecCall, ExecCell, ExecCommandSource, OutputLinesParams, output_lines,
};
use crate::history_cell::HistoryCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 描述 `CallMutation` 在当前界面流程中传递的数据。
pub(super) enum CallMutation {
    Applied,
    MissingStart,
    Duplicate,
    AlreadyTerminal,
}

#[derive(Clone, Debug)]
/// 描述 `LifecycleExecCell` 在本模块状态转换中传递的数据。
pub(super) struct LifecycleExecCell {
    cell: ExecCell,
    live_output: HashMap<String, String>,
}

impl HistoryCell for LifecycleExecCell {
    // 计算 `display_lines` 所需的界面结果。
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for call in &self.cell.calls {
            let single = ExecCell {
                calls: vec![call.clone()],
                animations_enabled: self.cell.animations_enabled,
            };
            let mut call_lines = single.display_lines(width);
            if call.duration.is_none()
                && let Some(output) = self.live_output.get(&call.call_id).filter(|s| !s.is_empty())
            {
                let indicator = call_lines.pop();
                call_lines.extend(output_lines(
                    output,
                    OutputLinesParams {
                        max_lines: crate::exec_cell::TOOL_CALL_MAX_LINES,
                        width: usize::from(width).saturating_sub(2).max(1),
                    },
                ));
                call_lines.extend(indicator);
            }
            lines.extend(call_lines);
        }
        lines
    }
}

#[derive(Debug, Default)]
pub(super) struct ExecState {
    cell: Option<ExecCell>,
    call_indexes: HashMap<String, usize>,
    live_output: HashMap<String, String>,
    terminal_call_ids: HashSet<String>,
}

impl ExecState {
    // 判断 `is_active` 对应的界面条件是否成立。
    pub(super) fn is_active(&self) -> bool {
        self.cell.is_some()
    }

    pub(super) fn has_running_calls(&self) -> bool {
        self.cell.as_ref().is_some_and(|cell| cell.calls.iter().any(|call| call.duration.is_none()))
    }

    pub(super) fn start(&mut self, call_id: String, tool: String, command: String) -> CallMutation {
        if self.terminal_call_ids.contains(&call_id) {
            return CallMutation::AlreadyTerminal;
        }
        if self.call_indexes.contains_key(&call_id) {
            return CallMutation::Duplicate;
        }
        let cell = self
            .cell
            .get_or_insert_with(|| ExecCell { calls: Vec::new(), animations_enabled: false });
        let index = cell.calls.len();
        cell.calls.push(ExecCall {
            call_id: call_id.clone(),
            tool,
            command,
            output: None,
            source: ExecCommandSource::Tool,
            start_time: Some(Instant::now()),
            duration: None,
        });
        self.call_indexes.insert(call_id, index);
        CallMutation::Applied
    }

    // 计算 `append_output` 所需的界面结果。
    pub(super) fn append_output(&mut self, call_id: &str, delta: &str) -> CallMutation {
        if self.terminal_call_ids.contains(call_id) {
            return CallMutation::AlreadyTerminal;
        }
        if !self.call_indexes.contains_key(call_id) {
            return CallMutation::MissingStart;
        }
        self.live_output.entry(call_id.to_string()).or_default().push_str(delta);
        CallMutation::Applied
    }

    pub(super) fn finish(
        &mut self,
        call_id: &str,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    ) -> CallMutation {
        if self.terminal_call_ids.contains(call_id) {
            return CallMutation::AlreadyTerminal;
        }
        let Some(index) = self.call_indexes.get(call_id).copied() else {
            return CallMutation::MissingStart;
        };
        let final_output = if output.is_empty() {
            self.live_output.remove(call_id).unwrap_or_default()
        } else {
            self.live_output.remove(call_id);
            output
        };
        let call = &mut self.cell.as_mut().expect("indexed call has an exec cell").calls[index];
        call.output = Some(CommandOutput::new(exit_code, final_output));
        call.duration = Some(Duration::from_millis(duration_ms));
        call.start_time = None;
        self.terminal_call_ids.insert(call_id.to_string());
        CallMutation::Applied
    }

    // 处理 `finish_running` 对应的界面事件并同步相关状态。
    pub(super) fn finish_running(&mut self, exit_code: i32, message: &str) -> Vec<String> {
        let Some(cell) = self.cell.as_mut() else {
            return Vec::new();
        };
        let mut call_ids = Vec::new();
        for call in &mut cell.calls {
            if call.duration.is_some() {
                continue;
            }
            let mut output = self.live_output.remove(&call.call_id).unwrap_or_default();
            if !message.is_empty() {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(message);
            }
            call.output = Some(CommandOutput::new(exit_code, output));
            call.duration = Some(Duration::ZERO);
            call.start_time = None;
            self.terminal_call_ids.insert(call.call_id.clone());
            call_ids.push(call.call_id.clone());
        }
        call_ids
    }

    pub(super) fn snapshot(&self) -> Option<LifecycleExecCell> {
        self.cell.as_ref().map(|cell| LifecycleExecCell {
            cell: cell.clone(),
            live_output: self.live_output.clone(),
        })
    }

    pub(super) fn clear_group(&mut self) {
        self.cell = None;
        self.call_indexes.clear();
        self.live_output.clear();
    }
}
