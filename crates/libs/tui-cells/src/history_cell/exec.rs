//! 由复制版 `exec_cell` 渲染器支持的工具执行历史单元。
//!
//! 本模块只负责把执行模型接入统一的历史单元接口。
//! 命令、输出、活动指示和退出状态的具体布局仍由 `exec_cell` 负责。
//! 完成事件携带输出与耗时；活动事件在结束前保持可刷新状态。
//! Tool execution history cell backed by the copied `exec_cell` renderer.

use std::time::Duration;

use ratatui::text::Line;

use super::HistoryCell;
use crate::exec_cell::{CommandOutput, ExecCell};

#[derive(Debug)]
pub struct ExecHistoryCell {
    cell: ExecCell,
}

impl ExecHistoryCell {
    pub fn from_pending(call_id: String, tool: String, command: String) -> Self {
        Self { cell: ExecCell::pending(call_id, tool, command) }
    }

    pub fn from_result(
        call_id: String,
        tool: String,
        command: String,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    ) -> Self {
        Self {
            cell: ExecCell::completed(
                call_id,
                tool,
                command,
                CommandOutput::new(exit_code, output),
                Duration::from_millis(duration_ms),
            ),
        }
    }

    pub fn exec_cell(&self) -> &ExecCell {
        &self.cell
    }
}

impl HistoryCell for ExecHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.cell.display_lines(width)
    }
}
