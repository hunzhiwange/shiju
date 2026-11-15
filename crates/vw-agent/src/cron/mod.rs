//! 定时任务领域入口，汇集计划表达、持久化、调度器与命令行操作。
//!
//! 命令层只编排领域操作；计划校验、下次运行时间与数据库更新由下层模块负责。

pub mod active_runs;
pub mod consolidation;
pub(crate) mod delivery;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod delivery_tests;

#[cfg(test)]
pub(crate) mod active_runs_tests;
mod schedule;
mod store;
mod types;

pub mod scheduler;

#[allow(unused_imports)]
pub use schedule::{
    next_run_for_schedule, normalize_expression, schedule_cron_expression, validate_schedule,
};
pub(crate) use store::next_run_in_window;
#[allow(unused_imports)]
pub use store::{
    CRON_HISTORY_LIMIT, add_agent_job, add_agent_job_with_window, add_job, add_shell_job,
    add_shell_job_with_window, due_jobs, get_job, list_history, list_jobs, list_runs,
    record_last_run, record_run, remove_job, reschedule_after_run, update_job,
};
pub(crate) use types::normalize_fallbacks;
pub use types::{CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget};

pub use agent_cron::{CronCommands, handle_command, add_once, add_once_at, pause_job, resume_job};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod wecom_acceptance_tests;
