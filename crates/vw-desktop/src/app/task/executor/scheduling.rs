//! 任务执行器的 scheduling.rs 子模块。
//!
//! 该模块聚焦任务运行过程中的一个局部职责，供执行器入口组合调用。注释说明边界、错误传播和平台差异，避免调用方需要阅读完整执行链才能理解行为。

use super::*;
use crate::app::task::models;

/// 公开的 ExecutorEvent 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    TaskStarted { task_id: String },
    TaskProgress { task_id: String, message: String },
    TaskCompleted { task_id: String },
    TaskFailed { task_id: String, error: String },
    StatusChanged { task_id: String, from: TaskStatus, to: TaskStatus },
}

/// 公开的 get_next_tasks_for_execution 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn get_next_tasks_for_execution(
    project_path: &str,
    max_count: u32,
    exclude_ids: &[String],
) -> Vec<(String, u32, u32)> {
    let tasks_by_status = store::load_tasks_by_status(project_path);
    select_next_tasks(&tasks_by_status, max_count, exclude_ids)
}

fn select_next_tasks(
    tasks_by_status: &HashMap<TaskStatus, Vec<models::Task>>,
    max_count: u32,
    exclude_ids: &[String],
) -> Vec<(String, u32, u32)> {
    let mut candidates: Vec<(String, u32, u32)> = Vec::new();

    if let Some(pending_tasks) = tasks_by_status.get(&TaskStatus::Pending) {
        for task in pending_tasks {
            if !exclude_ids.contains(&task.id) {
                candidates.push((task.id.clone(), task.priority, task.order));
            }
        }
    }

    candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2)));
    candidates.truncate(max_count as usize);
    candidates
}

/// 公开的 simulate_task_execution_step 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn simulate_task_execution_step(
    _project_path: &str,
    _task_id: &str,
    current_status: TaskStatus,
) -> Option<TaskStatus> {
    crate::app::task::TaskStateMachine::default_lifecycle()
        .next_for_intent(current_status, crate::app::task::TaskTransitionIntent::Simulation)
}

/// 公开的 count_running_tasks 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn count_running_tasks(tasks_by_status: &HashMap<TaskStatus, Vec<models::Task>>) -> usize {
    let workflow = crate::app::task::default_task_workflow();
    tasks_by_status
        .iter()
        .filter(|(status, _)| workflow.role_occupies_execution_slot(**status))
        .map(|(_, tasks)| tasks.len())
        .sum()
}

/// 公开的 get_pool_and_pending_count 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn get_pool_and_pending_count(
    tasks_by_status: &HashMap<TaskStatus, Vec<models::Task>>,
) -> usize {
    let workflow = crate::app::task::default_task_workflow();
    tasks_by_status
        .iter()
        .filter(|(status, _)| workflow.role_is_scheduling_backlog(**status))
        .map(|(_, tasks)| tasks.len())
        .sum()
}

/// 公开的 get_total_task_count 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn get_total_task_count(tasks_by_status: &HashMap<TaskStatus, Vec<models::Task>>) -> usize {
    tasks_by_status.values().map(std::vec::Vec::len).sum()
}

#[cfg(test)]
#[path = "scheduling_tests.rs"]
mod scheduling_tests;
