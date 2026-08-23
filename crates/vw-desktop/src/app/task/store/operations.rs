//! 提供任务的查询、创建、更新与删除操作，并隔离平台持久化差异。

use std::collections::HashMap;
use std::io;

#[cfg(target_arch = "wasm32")]
use time::OffsetDateTime;

#[cfg(target_arch = "wasm32")]
use crate::app::task::models::{SubTask, SubTaskStatus};
use crate::app::task::models::{Task, TaskStatus};

#[cfg(target_arch = "wasm32")]
use super::paths::get_task_log_dir;
#[cfg(not(target_arch = "wasm32"))]
use super::paths::native_cutover_error;
#[cfg(target_arch = "wasm32")]
use super::persistence::{
    delete_task_file, load_index_unlocked, load_task, max_sequence_for_date, save_index_unlocked,
};
use super::persistence::{load_all_tasks, save_task};

/// 按状态分组任务，并以创建时间、优先级、顺序和 ID 稳定排序。
pub fn load_tasks_by_status(project_path: &str) -> HashMap<TaskStatus, Vec<Task>> {
    let mut result =
        TaskStatus::all().into_iter().map(|status| (status, Vec::new())).collect::<HashMap<_, _>>();
    for task in load_all_tasks(project_path) {
        result.entry(task.status).or_default().push(task);
    }
    for tasks in result.values_mut() {
        tasks.sort_by(|left, right| {
            right
                .created_at_ms
                .cmp(&left.created_at_ms)
                .then_with(|| right.priority.cmp(&left.priority))
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    result
}

/// 创建任务；原生桌面已迁移到全局调度器，因此仅旧版平台路径可写。
pub fn create_task(project_path: &str, task: Task) -> io::Result<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut task = task;
        let mut index = load_index_unlocked(project_path);
        let now_ms = crate::app::time::now_ms();
        let seconds = i64::try_from(now_ms / 1_000).unwrap_or_default();
        let date_time =
            OffsetDateTime::from_unix_timestamp(seconds).unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let month: u8 = date_time.month().into();
        let date = format!("{:04}{:02}{:02}", date_time.year(), month, date_time.day());
        let last_sequence = if index.last_task_date.as_deref() == Some(date.as_str()) {
            index.last_task_seq
        } else {
            0
        }
        .max(max_sequence_for_date(&index, &date));
        let sequence = last_sequence.saturating_add(1);
        task.id = format!("T{date}.{sequence:04}");
        let status_key = task.status.to_string_key().to_string();
        task.order = index.order_by_status.get(&status_key).map_or(0, |tasks| tasks.len() as u32);
        index.last_task_date = Some(date);
        index.last_task_seq = sequence;
        index.tasks.insert(task.id.clone(), status_key.clone());
        index.order_by_status.entry(status_key).or_default().push(task.id.clone());
        save_task(project_path, &task)?;
        save_index_unlocked(project_path, &index)?;
        Ok(task)
    }
}

#[cfg(target_arch = "wasm32")]
fn reset_subtask_for_rebuild(mut subtask: SubTask) -> SubTask {
    subtask.completed = false;
    subtask.status = SubTaskStatus::Pending;
    subtask.execution_started_at_ms = None;
    subtask.last_execution_duration_ms = None;
    subtask
}

/// 从当前分支重建失败任务，并重置子任务运行状态与结果。
pub fn rebuild_failed_task_from_current_branch(
    project_path: &str,
    old_task: Task,
) -> io::Result<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, old_task);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let old_task_id = old_task.id.clone();
        let mut task = Task::new(old_task.priority);
        task.assignee = old_task.assignee;
        task.model = old_task.model;
        task.agent = old_task.agent;
        task.acp_agent = old_task.acp_agent;
        task.reasoning_effort = old_task.reasoning_effort;
        task.description = old_task.description;
        task.prompt = old_task.prompt;
        task.status = TaskStatus::Pool;
        task.subtasks = old_task.subtasks.into_iter().map(reset_subtask_for_rebuild).collect();
        task.task_split_mode = old_task.task_split_mode;
        task.auto_promote_delay_ms = old_task.auto_promote_delay_ms;
        delete_task_file(project_path, &old_task_id)?;
        let log_directory = get_task_log_dir(project_path);
        for file_name in [format!("[{old_task_id}].log"), format!("[{old_task_id}].review.log")] {
            match std::fs::remove_file(log_directory.join(file_name)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        create_task(project_path, task)
    }
}

/// 持久化现有任务的最新状态。
pub fn update_task(project_path: &str, task: &Task) -> io::Result<()> {
    save_task(project_path, task)
}

/// 在同一状态内重排任务，并保存连续的顺序值。
pub fn reorder_tasks_in_status(
    project_path: &str,
    status: TaskStatus,
    task_ids: Vec<String>,
) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, status, task_ids);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut index = load_index_unlocked(project_path);
        let status_key = status.to_string_key().to_string();
        index.order_by_status.insert(status_key, task_ids.clone());
        for (order, task_id) in task_ids.iter().enumerate() {
            if let Some(mut task) = load_task(project_path, task_id) {
                task.order = order as u32;
                save_task(project_path, &task)?;
            }
        }
        save_index_unlocked(project_path, &index)
    }
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod operations_tests;
