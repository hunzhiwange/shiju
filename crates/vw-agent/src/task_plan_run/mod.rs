//! 聊天工作流执行（task-plan-run）领域模块。
//!
//! 提供 `tasks.json` 解析、批次调度，以及规划/子会话执行器。
//! Gateway SSE 路由不在本模块内（见 gateway handlers）。

mod control;
mod goal;
mod model;
mod prompts;
mod runner;
mod schedule;
mod session_bind;

pub use model::{PlanTaskEntry, PlanTasksDocument, load_tasks_json, parse_tasks_json};
pub(crate) use control::global_plan_run_control_registry;
#[cfg(test)]
pub(crate) use control::PlanRunControlRegistry;
pub use prompts::{
    CREATE_TASK_PLANS_SKILL_SOURCE, DO_TASK_PLANS_SKILL_SOURCE, build_execution_prompt,
    build_goal_planning_prompt, build_planning_prompt, plan_dir_abs, plan_dir_name, plan_dir_rel,
    sanitize_task_plan_id_part,
};
pub use runner::{
    GatewaySessionRunner, SessionRunRequest, SessionRunner, run_task_plan,
    run_task_plan_with_cancellation,
};
#[cfg(test)]
pub(crate) use runner::gateway_session_options;
pub use schedule::build_batch_stages;

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod schedule_tests;

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod prompts_tests;

#[cfg(test)]
#[path = "session_bind_tests.rs"]
mod session_bind_tests;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

#[cfg(test)]
#[path = "goal_tests.rs"]
mod goal_tests;

pub use goal::{
    GoalTaskFilesState, build_goal_continuation_prompt, goal_turn_session_id,
    inspect_goal_task_files,
};

#[cfg(test)]
#[path = "control_tests.rs"]
mod control_tests;
