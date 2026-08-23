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

    pub fn from_result(call_id: String, output: String, exit_code: i32, duration_ms: u64) -> Self {
        Self {
            cell: ExecCell::completed(
                call_id,
                "tool".to_string(),
                "completed command".to_string(),
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
