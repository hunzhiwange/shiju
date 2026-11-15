//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::sync::mpsc::Sender;
use std::time::Duration;

use super::models::Task;

mod backend_output;
mod command_exec;
mod gateway_output;
mod git;
mod process_utils;
mod programs;
mod runner;
mod scheduling;
mod worktree_admin;


/// 对外暴露当前模块需要复用的能力。
pub use command_exec::{execute_task_command, execute_task_command_with_streaming};
/// 对外暴露当前模块需要复用的能力。
pub use git::build_review_diff_context;
/// 对外暴露当前模块需要复用的能力。
pub use programs::{ExecutorCommand, build_executor_command};
/// 对外暴露当前模块需要复用的能力。
pub use runner::{
    TaskPlanGenerationOutcome, TaskPlanSubTask, execute_gateway_prompt_with_streaming,
    execute_task_async, execute_task_merge_async, execute_task_plan_async,
    execute_task_retry_async, execute_task_review_async, write_task_plan_files,
};
/// 对外暴露当前模块需要复用的能力。
pub use scheduling::{
    ExecutorEvent, count_running_tasks, get_next_tasks_for_execution, get_pool_and_pending_count,
    get_total_task_count, simulate_task_execution_step,
};
/// 对外暴露当前模块需要复用的能力。
pub use desktop_task_executor::executor::state::{
    TaskExecutorState, TaskLogStream, WorktreePoolSnapshot, WorktreeSlotSnapshot, WorktreeState,
};
/// 对外暴露当前模块需要复用的能力。
pub use worktree_admin::{
    assign_task_execution_worktree, commit_merge_all_worktrees, commit_merge_all_worktrees_async,
    commit_merge_all_worktrees_async_with_logs, commit_merge_all_worktrees_with_logs,
    delete_all_managed_worktrees, delete_all_managed_worktrees_async,
    delete_all_managed_worktrees_async_with_logs, delete_all_managed_worktrees_with_logs,
    reset_all_managed_worktrees, reset_all_managed_worktrees_async,
    reset_all_managed_worktrees_async_with_logs, reset_all_managed_worktrees_with_logs,
};
