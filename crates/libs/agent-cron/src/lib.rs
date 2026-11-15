//! 定时任务领域类型、计划计算与进程内执行快照。

pub mod types;
pub use types::{CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget};
pub mod schedule;
pub use schedule::{next_run_for_schedule, normalize_expression, schedule_cron_expression, validate_schedule};
pub mod active_runs;
pub mod scheduler;
pub mod delivery;
pub mod consolidation;
pub mod execution;

pub mod store;
pub use store::*;
pub use types::normalize_fallbacks;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod store_tests;

pub mod cli;
pub use cli::{CronCommands, handle_command, add_once, add_once_at, pause_job, resume_job};
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
#[path = "tests/schedule.rs"]
mod schedule_acceptance_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "tests/commands.rs"]
mod commands_acceptance_tests;

#[cfg(test)]
#[path = "tests/types.rs"]
mod types_acceptance_tests;

#[cfg(test)]
mod active_runs_tests;

#[cfg(test)]
#[path = "tests/scheduler.rs"]
mod scheduler_acceptance_tests;
