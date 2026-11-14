//! 历史步骤的紧凑状态标记。

use tui_contracts::event::V3StepStatus;

pub fn format_step(title: &str, status: V3StepStatus) -> String {
    format!("{} {}", step_status_marker(status), title)
}

pub fn step_status_marker(status: V3StepStatus) -> char {
    // 标记必须在无颜色终端中也能仅靠字形区分状态。
    match status {
        V3StepStatus::Pending => '○',
        V3StepStatus::InProgress => '◐',
        V3StepStatus::Completed => '●',
        V3StepStatus::Failed => '×',
    }
}
