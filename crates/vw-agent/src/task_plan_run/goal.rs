use std::path::Path;
use super::PlanTaskEntry;
pub use task_plan_runner::goal::GoalTaskFilesState;
pub fn inspect_goal_task_files(
    plan_abs: &Path,
    tasks: &[PlanTaskEntry],
    restrict_files: bool,
) -> Result<GoalTaskFilesState, String>  { super::runner::ensure_host(); task_plan_runner::goal::inspect_goal_task_files(plan_abs, tasks, restrict_files) }
#[cfg(test)]
pub fn inspect_goal_task_file(
    plan_abs: &Path,
    task: &PlanTaskEntry,
    restrict_files: bool,
) -> Result<GoalTaskFilesState, String>  { super::runner::ensure_host(); task_plan_runner::goal::inspect_goal_task_file(plan_abs, task, restrict_files) }
pub fn build_goal_continuation_prompt(
    original_goal: &str,
    plan_abs: &Path,
    tasks: &[PlanTaskEntry],
    plan_markdown: &str,
    reason: &str,
    task_files_state: &GoalTaskFilesState,
    execution_evidence: &str,
) -> String  { super::runner::ensure_host(); task_plan_runner::goal::build_goal_continuation_prompt(original_goal, plan_abs, tasks, plan_markdown, reason, task_files_state, execution_evidence) }
pub fn goal_turn_session_id(task_id: &str, turn: u32) -> String  { super::runner::ensure_host(); task_plan_runner::goal::goal_turn_session_id(task_id, turn) }
