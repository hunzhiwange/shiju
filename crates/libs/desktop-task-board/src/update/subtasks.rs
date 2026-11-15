//! 子任务元数据更新，持久化与日志视图效果由宿主执行。
use desktop_task_model::{Task, SubTask, SubTaskStatus};
use crate::message::TaskBoardMessage;

pub fn update_subtasks(tasks: &mut [Task], message: TaskBoardMessage, now_ms: u64) -> Result<Option<Task>, TaskBoardMessage> {
    match message {
        TaskBoardMessage::AddSubTask { task_id, content } => {
            if content.trim().is_empty() {
                return Ok(None);
            }
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                let subtask = SubTask::new(content);
                task.subtasks.push(subtask);
                task.updated_at_ms = now_ms;
                task_clone = Some(task.clone());
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::RemoveSubTask { task_id, subtask_id } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                task.subtasks.retain(|s| s.id != subtask_id);
                task.updated_at_ms = now_ms;
                task_clone = Some(task.clone());
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::MoveSubTaskUp { task_id, subtask_id } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                let idx = task.subtasks.iter().position(|s| s.id == subtask_id);
                if let Some(i) = idx
                    && i > 0
                {
                    task.subtasks.swap(i, i - 1);
                    task.updated_at_ms = now_ms;
                    task_clone = Some(task.clone());
                }
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::MoveSubTaskDown { task_id, subtask_id } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                let idx = task.subtasks.iter().position(|s| s.id == subtask_id);
                if let Some(i) = idx
                    && i < task.subtasks.len() - 1
                {
                    task.subtasks.swap(i, i + 1);
                    task.updated_at_ms = now_ms;
                    task_clone = Some(task.clone());
                }
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::ToggleSubTaskCompleted { task_id, subtask_id } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id)
                && let Some(subtask) = task.subtasks.iter_mut().find(|s| s.id == subtask_id)
            {
                subtask.completed = !subtask.completed;
                if subtask.completed {
                    subtask.status = SubTaskStatus::Completed;
                } else {
                    subtask.status = SubTaskStatus::Pending;
                    subtask.execution_started_at_ms = None;
                }
                task.updated_at_ms = now_ms;
                task_clone = Some(task.clone());
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::UpdateSubTaskContent { task_id, subtask_id, content } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id)
                && let Some(subtask) = task.subtasks.iter_mut().find(|s| s.id == subtask_id)
            {
                subtask.content = content;
                task.updated_at_ms = now_ms;
                task_clone = Some(task.clone());
            }

            return Ok(task_clone);
        }
        TaskBoardMessage::SubTaskUpdated { task_id, subtasks } => {
            let mut task_clone = None;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                task.subtasks = subtasks;
                task.updated_at_ms = now_ms;
                task_clone = Some(task.clone());
            }

            return Ok(task_clone);
        }
        other => Err(other),
    }
}
