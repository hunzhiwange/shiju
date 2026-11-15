//! 历史展示、转录、执行单元、动态指示及只读状态卡。

#[allow(dead_code)]
pub mod exec_cell;
#[allow(dead_code)]
pub mod motion;
#[allow(dead_code)]
pub mod status;

pub use status::{StatusHistoryCell, V3SessionDisplayData, V3TokenUsage};

#[allow(dead_code)]
mod diff_render;
pub mod history_cell;
pub mod step;
pub mod thread_transcript;

pub use history_cell::{ExecHistoryCell, HistoryCell, history_cell_for, render_display_event};

pub mod goal_display;
