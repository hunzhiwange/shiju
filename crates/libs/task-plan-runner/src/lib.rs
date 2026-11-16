//! 任务计划文档与批次调度。

mod model;
mod schedule;

pub use model::{PlanTaskEntry, PlanTasksDocument, load_tasks_json, parse_tasks_json};
pub use schedule::build_batch_stages;

#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod schedule_tests;

pub mod control;
pub mod goal;
pub mod prompts;
pub mod runner;
mod host;
pub mod session_bind;
mod store_port;
pub use host::{PlanRunHost, install_host};
pub use control::{PlanRunControlRegistry, PlanRunStopOutcome, global_plan_run_control_registry};
pub use goal::{GoalTaskFilesState, build_goal_continuation_prompt, goal_turn_session_id, inspect_goal_task_files};
pub use prompts::*;
pub use runner::{SessionRunRequest, SessionRunner, run_task_plan, run_task_plan_with_cancellation};

#[cfg(test)]
mod host_split_5_tests;

#[cfg(test)]
mod test_host;
#[cfg(test)]
mod host_split_6_tests;
