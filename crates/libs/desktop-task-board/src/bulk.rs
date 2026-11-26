use desktop_task_model::{Task, TaskStatus, normalize_task_model_input};
use std::collections::HashSet;
pub fn visible_task_ids_for_status(tasks: &[Task], status: TaskStatus) -> Vec<String> {
    tasks
        .iter()
        .filter(|task| task.status == status && !task.deleted && !task.archived)
        .map(|task| task.id.clone())
        .collect()
}
pub fn selected_task_ids_for_status(
    tasks: &[Task],
    selected: &HashSet<String>,
    status: TaskStatus,
) -> Vec<String> {
    visible_task_ids_for_status(tasks, status)
        .into_iter()
        .filter(|task_id| selected.contains(task_id))
        .collect()
}
pub fn prune_bulk_selection(tasks: &[Task], selected: &mut HashSet<String>) {
    let valid_ids = tasks
        .iter()
        .filter(|task| !task.deleted && !task.archived)
        .map(|task| task.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    selected.retain(|task_id| valid_ids.contains(task_id.as_str()));
}
pub fn clear_selected_task_ids(selected: &mut HashSet<String>, task_ids: &[String]) {
    for task_id in task_ids {
        selected.remove(task_id);
    }
}
pub fn clear_selection_for_status(
    tasks: &[Task],
    selected: &mut HashSet<String>,
    status: TaskStatus,
) {
    let task_ids = visible_task_ids_for_status(tasks, status);
    clear_selected_task_ids(selected, &task_ids);
}
pub fn selected_tasks_for_status(
    tasks: &[Task],
    selected: &HashSet<String>,
    status: TaskStatus,
) -> Vec<Task> {
    tasks
        .iter()
        .filter(|task| {
            task.status == status && !task.deleted && !task.archived && selected.contains(&task.id)
        })
        .cloned()
        .collect()
}
pub fn agent_scheduler_command_request(
    task_id: &str,
    command: vw_api_agent_scheduler::AgentSchedulerCommandKindDto,
    reason: impl Into<Option<String>>,
    payload: vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto,
) -> vw_api_agent_scheduler::AgentSchedulerCommandRequest {
    vw_api_agent_scheduler::AgentSchedulerCommandRequest {
        task_id: task_id.to_string().into(),
        command,
        max_concurrent: None,
        reason: reason.into(),
        payload,
    }
}
pub fn normalized_bulk_model_input(value: &str) -> String {
    normalize_task_model_input(value)
}

pub fn manual_merge_running(tasks: &[Task], running: &[String]) -> bool {
    running.iter().any(|task_id| {
        tasks
            .iter()
            .find(|task| task.id == *task_id)
            .is_some_and(|task| task.status == TaskStatus::PrSubmitted)
    })
}

pub fn pop_next_manual_merge_task(
    tasks: &[Task],
    running: &[String],
    queue: &mut Vec<String>,
) -> Option<Task> {
    while !queue.is_empty() {
        let task_id = queue.remove(0);
        if let Some(task) = tasks
            .iter()
            .find(|task| {
                task.id == task_id
                    && !task.deleted
                    && !task.archived
                    && task.status == TaskStatus::PrSubmitted
                    && !running.contains(&task.id)
            })
            .cloned()
        {
            return Some(task);
        }
    }
    None
}

#[cfg(test)]
#[path = "bulk_tests.rs"]
mod bulk_tests;

pub fn task_metadata_patch_for_gateway(task: &Task) -> vw_api_agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
    vw_api_agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
        title: Some(crate::board::task_title(task).chars().take(120).collect()),
        status_key: None,
        status: None,
        priority: Some(task.priority),
        order_no: Some(i64::from(task.order)),
        model: Some(task.model.clone()),
        agent: None,
        acp_agent: None,
        reasoning_effort: task.reasoning_effort,
        prompt: Some(task.prompt.clone()),
        last_error: task.last_error.clone(),
        pause_reason: task.pause_reason.clone(),
        merge_source_branch: task.merge_source_branch.clone(),
        merge_target_branch: task.merge_target_branch.clone(),
        selected_worktree_path: task.selected_worktree_path.clone(),
        selected_worktree_slot_id: task.selected_worktree_slot_id.clone(),
        selected_worktree_branch: task.selected_worktree_branch.clone(),
        workflow_id: None,
        status_category: None,
        metadata_merge: Some(serde_json::json!({
            "description": &task.description,
            "auto_promote_delay_ms": task.auto_promote_delay_ms,
        })),
        metadata_remove_keys: Vec::new(),
        label_ids: None,
        property_changes: Default::default(),
    }
}

pub enum ManualMergeEffect {
    None,
    MissingProject,
    Dispatch(Task),
}
pub fn prepare_manual_merge(
    tasks: &mut [Task],
    running: &[String],
    queue: &mut Vec<String>,
    has_project: bool,
) -> ManualMergeEffect {
    if manual_merge_running(tasks, running) {
        return ManualMergeEffect::None;
    }
    if !has_project {
        queue.clear();
        return ManualMergeEffect::MissingProject;
    }
    let Some(task) = pop_next_manual_merge_task(tasks, running, queue) else {
        return ManualMergeEffect::None;
    };
    if let Some(existing) = tasks.iter_mut().find(|t| t.id == task.id) {
        existing.add_localized_log("手动批量合并已发送 Agent Scheduler 合并命令", vec![]);
    }
    ManualMergeEffect::Dispatch(task)
}

pub fn persist_updated_tasks_commands(
    tasks: &[Task],
    action_label: &str,
    _workflow_id: &str,
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest> {
    tasks
        .iter()
        .map(|task| {
            agent_scheduler_command_request(
                &task.id,
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::UpdateMetadata,
                Some(action_label.to_string()),
                vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
                    metadata_update: Some(task_metadata_patch_for_gateway(task)),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>()
}

pub fn delete_tasks_commands(
    task_ids: &[String],
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest> {
    task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::SoftDelete,
                Some("删除任务".to_string()),
                Default::default(),
            )
        })
        .collect::<Vec<_>>()
}

pub fn archive_tasks_commands(
    task_ids: &[String],
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest> {
    task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Archive,
                Some("归档任务".to_string()),
                Default::default(),
            )
        })
        .collect::<Vec<_>>()
}

pub fn move_task_status_commands(
    task_id: &str,
    target_status: TaskStatus,
    workflow_id: &str,
    order_no: Option<i64>,
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest> {
    vec![move_task_status_command(task_id, target_status, workflow_id, order_no)]
}

pub fn move_task_status_command(
    task_id: &str,
    target_status: TaskStatus,
    workflow_id: &str,
    order_no: Option<i64>,
) -> vw_api_agent_scheduler::AgentSchedulerCommandRequest {
    let status_key = crate::helpers::execution::agent_scheduler_status_key_for_task_status(
        target_status,
        workflow_id,
    );
    agent_scheduler_command_request(
        task_id,
        vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Move,
        Some("手动移动任务".to_string()),
        vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
            move_task: Some(vw_api_agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
                from_status_key: None,
                to_status_key: status_key,
                to_status: None,
                order_no,
            }),
            ..Default::default()
        },
    )
}

pub fn move_tasks_to_status_commands(
    task_ids: &[String],
    target_status: TaskStatus,
    workflow_id: &str,
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest> {
    let status_key = crate::helpers::execution::agent_scheduler_status_key_for_task_status(
        target_status,
        workflow_id,
    );
    task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Move,
                Some("批量移动任务".to_string()),
                vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
                    move_task: Some(vw_api_agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
                        from_status_key: None,
                        to_status_key: status_key.clone(),
                        to_status: None,
                        order_no: None,
                    }),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>()
}
