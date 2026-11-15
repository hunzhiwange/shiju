use std::path::{Path, PathBuf};
pub use task_plan_runner::prompts::{CREATE_TASK_PLANS_SKILL_SOURCE, DO_TASK_PLANS_SKILL_SOURCE};
pub fn sanitize_task_plan_id_part(value: &str) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::sanitize_task_plan_id_part(value) }
pub fn plan_dir_name(task_id: &str) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::plan_dir_name(task_id) }
pub fn plan_dir_abs(home: impl AsRef<Path>, task_id: &str) -> PathBuf  { super::runner::ensure_host(); task_plan_runner::prompts::plan_dir_abs(home, task_id) }
pub fn build_planning_prompt(task_id: &str, query: &str) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::build_planning_prompt(task_id, query) }
#[cfg(test)]
pub fn build_planning_prompt_for_dir(task_id: &str, query: &str, plan_dir: &Path) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::build_planning_prompt_for_dir(task_id, query, plan_dir) }
pub fn build_goal_planning_prompt(task_id: &str, query: &str) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::build_goal_planning_prompt(task_id, query) }
#[cfg(test)]
pub fn build_goal_planning_prompt_for_dir(
    task_id: &str,
    query: &str,
    plan_dir: &Path,
) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::build_goal_planning_prompt_for_dir(task_id, query, plan_dir) }
pub fn build_execution_prompt(
    query: &str,
    batch: u32,
    depends_on_batches: &[u32],
    plan_abs: &Path,
    task_file_abs: &Path,
    plan_markdown: &str,
    subtask_markdown: &str,
) -> String  { super::runner::ensure_host(); task_plan_runner::prompts::build_execution_prompt(query, batch, depends_on_batches, plan_abs, task_file_abs, plan_markdown, subtask_markdown) }
