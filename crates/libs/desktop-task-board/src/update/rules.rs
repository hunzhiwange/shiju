//! 规划结果与批量任务元数据的局部转换。
use desktop_task_model::{Task, TaskStatus};
use crate::draft::{set_task_executor_selection, task_acp_agent_label};
use crate::helpers::execution::{truncate_for_ui, TASK_LOG_UI_MAX_DETAIL_CHARS};

pub fn work_plan_from_outcome(
    planning_attempt_id: String,
    outcome: desktop_task_executor::executor::runner::TaskPlanGenerationOutcome,
) -> vw_api_agent_scheduler::WorkPlanPublication {
    vw_api_agent_scheduler::WorkPlanPublication::Generated {
        planning_attempt_id,
        subtasks: outcome.subtasks,
    }
}

pub fn set_bulk_priority(tasks: &mut [Task], priority: u32) {
    for task in tasks {
        let previous_priority = task.priority;
        task.priority = priority;
        task.add_localized_log(
            "批量设置优先级: %s -> %s",
            vec![(previous_priority).to_string().into(), (priority).to_string().into()],
        );
    }
}

pub fn set_bulk_model(tasks: &mut [Task], model: &str) {
    for task in tasks {
        let previous_model = task.model.to_string();
        task.model = model.to_string();
        task.add_localized_log(
            "批量设置模型: %s -> %s",
            vec![(previous_model).to_string().into(), (task.model).to_string().into()],
        );
    }
}

pub fn set_bulk_agent(tasks: &mut [Task], agent: &str, main_agent_key: &str) {
    for task in tasks {
        let previous_agent =
            task.agent.clone().unwrap_or_else(|| main_agent_key.to_string());
        task.agent = Some(agent.to_string());
        task.add_localized_log(
            "批量设置代理: %s -> %s",
            vec![(previous_agent).to_string().into(), (agent).to_string().into()],
        );
    }
}

pub fn set_bulk_executor(tasks: &mut [Task], executor: Option<String>) {
    for task in tasks {
        let previous_executor = task_acp_agent_label(task.acp_agent.as_deref());
        set_task_executor_selection(task, executor.clone());
        task.add_localized_log(
            "批量设置 ACP 智能体: %s -> %s",
            vec![
                (previous_executor).to_string().into(),
                (task_acp_agent_label(task.acp_agent.as_deref())).to_string().into(),
            ],
        );
    }
}

pub fn drop_insert_index(tasks: &[Task], task_id: &str, to_status: desktop_task_model::TaskStatus, insert_index: Option<usize>) -> Option<usize> {
    let mut status_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.status == to_status && t.id != task_id)
        .collect();
    status_tasks.sort_by(|a, b| {
        a.order.cmp(&b.order).then_with(|| a.created_at_ms.cmp(&b.created_at_ms))
    });
    let insert_at = insert_index.unwrap_or(status_tasks.len()).min(status_tasks.len());
    let Some(drag_task) = tasks.iter().find(|t| t.id == task_id) else {
        return None;
    };
    status_tasks.insert(insert_at, drag_task);

    Some(insert_at)
}

pub fn set_bulk_reasoning_effort(all_tasks: &mut [Task], task_ids: &[String], reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>) -> Vec<Task> {
    let mut tasks = Vec::new();
    for task_id in task_ids {
        if let Some(task) = all_tasks.iter_mut().find(|task| &task.id == task_id)
        {
            task.reasoning_effort = reasoning_effort;
            task.add_localized_log("批量设置推理强度", vec![]);
            tasks.push(task.clone());
        }
    }
    tasks
}

pub fn set_bulk_split_mode(all_tasks: &mut [Task], task_ids: &[String], task_split_mode: desktop_task_model::TaskSplitMode) -> Vec<Task> {
    let mut tasks = Vec::new();
    for task_id in task_ids {
        if let Some(task) = all_tasks.iter_mut().find(|task| &task.id == task_id)
        {
            let previous_mode = task.task_split_mode;
            task.task_split_mode = task_split_mode;
            task.add_localized_log(
                "批量设置任务拆分: %s -> %s",
                vec![
                    (previous_mode.label()).to_string().into(),
                    (task_split_mode.label()).to_string().into(),
                ],
            );
            tasks.push(task.clone());
        }
    }
    tasks
}

pub fn completed_task_ids(tasks: &[Task]) -> Vec<String> {
    tasks.iter().filter(|task| !task.deleted && !task.archived && task.status == desktop_task_model::TaskStatus::Completed)
        .map(|task| task.id.clone()).collect()
}
pub fn normalized_bulk_agent(agent: &str, main_agent_key: &str) -> String {
    if agent.trim().is_empty() { main_agent_key.to_string() } else { agent.trim().to_string() }
}

pub fn prepare_manual_merge_queue(tasks: &mut [Task]) -> Vec<String> {
    tasks.sort_by_key(|task| (task.priority, task.order));
    for task in tasks.iter_mut() {
        task.add_localized_log("手动批量合并已入队", vec![]);
    }
    tasks.iter().map(|task| task.id.clone()).collect()
}

pub fn log_worktree_maintenance_failure(tasks: &mut [Task], error: &str) -> bool {
    let mut changed = false;
    for task in tasks {
        if task.status == TaskStatus::Pending
            || task.status == TaskStatus::Planning
            || task.status == TaskStatus::Running
            || task.status == TaskStatus::CodeReview
            || task.status == TaskStatus::PrSubmitted
        {
            task.add_localized_log(
                "后台 worktree 池维护失败: %s",
                vec![
                    (truncate_for_ui(error, TASK_LOG_UI_MAX_DETAIL_CHARS))
                        .to_string()
                        .into(),
                ],
            );
            changed = true;
        }
    }
    changed
}
