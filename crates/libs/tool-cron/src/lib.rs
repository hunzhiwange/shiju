//! 定时任务工具；存储由 agent-cron 持有，执行由宿主显式注入。

pub mod cron_add;
pub mod cron_list;
mod cron_preview;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
mod delivery;
mod options;
mod schedule;

pub use cron_add::CronAddTool;
pub use cron_list::CronListTool;
pub use cron_preview::CronPreviewTool;
pub use cron_remove::CronRemoveTool;
pub use cron_run::CronRunTool;
pub use cron_runs::CronRunsTool;
pub use cron_update::CronUpdateTool;

#[cfg(test)]
mod host_migration_33_tests;

#[cfg(any(test, feature = "test-support"))]
pub mod history_test_support;

fn failure(error: impl Into<String>) -> tool_types::ToolResult {
    tool_types::ToolResult { success: false, output: String::new(), error: Some(error.into()) }
}
