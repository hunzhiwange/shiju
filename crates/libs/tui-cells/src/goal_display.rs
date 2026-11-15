//! 为复制来的历史单元格式化目标及步骤文本。
//! Goal/step text formatting for the copied history cells.

use tui_contracts::event::V3StepStatus;

pub const GOAL_USAGE: &str = "Usage: /goal [<objective>|clear|edit|pause|resume]";

pub fn format_goal_elapsed_seconds(seconds: i64) -> String {
    // 负值按零处理，并只保留适合紧凑状态行的最大两到三个单位。
    let seconds = seconds.max(0) as u64;
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;
    if hours >= 24 {
        return format!("{}d {}h {}m", hours / 24, hours % 24, remaining_minutes);
    }
    if remaining_minutes == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {remaining_minutes}m")
    }
}

pub fn step_status_label(status: V3StepStatus) -> &'static str {
    match status {
        V3StepStatus::Pending => "pending",
        V3StepStatus::InProgress => "in progress",
        V3StepStatus::Completed => "completed",
        V3StepStatus::Failed => "failed",
    }
}

#[allow(unused_imports)]
pub use crate::step::{format_step, step_status_marker};

#[cfg(test)]
#[path = "goal_display_tests.rs"]
mod tests;
