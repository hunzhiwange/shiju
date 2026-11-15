//! 规划结果与批量任务元数据的局部转换。
use desktop_task_model::{Task, SubTask, TaskStatus};
use crate::draft::{set_task_executor_selection, task_acp_agent_label};
use crate::helpers::execution::{truncate_for_ui, TASK_LOG_UI_MAX_ENTRY_CHARS, TASK_LOG_UI_MAX_DETAIL_CHARS};
use crate::helpers::execution::{is_model_response_timeout_error, record_task_worktree_recycle_after_failure};
use super::projection::summarize_subtask_batches;

fn apply_plan_outcome(task: &mut Task, outcome: desktop_task_executor::executor::runner::TaskPlanGenerationOutcome) {
    task.subtasks.clear();
    for (offset, plan_subtask) in outcome.subtasks.into_iter().enumerate() {
        let mut subtask = SubTask::new(plan_subtask.title);
        subtask.order = offset as u32;
        subtask.boundary = plan_subtask.boundary;
        subtask.acceptance_criteria = plan_subtask.acceptance_criteria;
        subtask.target_files = plan_subtask.target_files;
        subtask.batch = plan_subtask.batch;
        subtask.depends_on_batches = plan_subtask.depends_on_batches;
        task.subtasks.push(subtask);
    }
    task.add_localized_log(
        "任务拆分完成: %s 个子任务",
        vec![(task.subtasks.len()).to_string().into()],
    );
    task.add_localized_log(
        "create-task-plans 计划目录: %s",
        vec![
            (truncate_for_ui(
                &outcome.plan_dir,
                TASK_LOG_UI_MAX_ENTRY_CHARS,
            ))
            .to_string()
            .into(),
        ],
    );
    task.add_localized_log(
        "计划入口文件路径: %s",
        vec![
            (truncate_for_ui(
                &outcome.prompt_md_path,
                TASK_LOG_UI_MAX_ENTRY_CHARS,
            ))
            .to_string()
            .into(),
        ],
    );
    task.add_localized_log(
        "批次摘要: %s",
        vec![
            (truncate_for_ui(
                &summarize_subtask_batches(&task.subtasks),
                TASK_LOG_UI_MAX_DETAIL_CHARS,
            ))
            .to_string()
            .into(),
        ],
    );
    if !outcome.raw_output.trim().is_empty() {
        task.add_localized_log(
            "拆分结果: %s",
            vec![
                (truncate_for_ui(
                    &outcome.raw_output,
                    TASK_LOG_UI_MAX_DETAIL_CHARS,
                ))
                .to_string()
                .into(),
            ],
        );
    }
}

pub fn start_planned_task(task: &mut Task, assigned_worktree_path: Option<String>) {
    let _ = task.start_execution("开始执行任务".to_string());
    task.selected_worktree_path = assigned_worktree_path;
    if let Some(selected_path) = &task.selected_worktree_path {
        task.add_localized_log(
            "[WORKTREE] 执行前已分配工作区: %s",
            vec![(selected_path).to_string().into()],
        );
    }
    task.add_localized_log("子任务将按顺序串行执行", vec![]);
}

fn mark_plan_failed(task: &mut Task, error: &str) {
    if is_model_response_timeout_error(&error) {
        task.mark_execution_failed(format!(
            "任务拆分模型响应超时: {}",
            error
        ));
        record_task_worktree_recycle_after_failure(
            task,
            "任务拆分模型响应超时",
        );
    } else {
        task.mark_execution_failed(format!(
            "任务拆分失败（create-task-plans 计划拆分产物缺失或无效，如 prompt.md 缺失、解析失败或批次校验失败）: {}",
            error
        ));
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

pub enum PlanningEffect { Ignored, AllocateWorktree, PersistFailure }
pub fn apply_planning_result(
    task: &mut Task,
    final_logs: Vec<desktop_task_executor::TaskLogStream>,
    result: Result<desktop_task_executor::executor::runner::TaskPlanGenerationOutcome, String>,
) -> PlanningEffect {
    for log in final_logs {
        let _ = crate::helpers::execution::append_task_log_stream(task, &log);
    }
    if task.status != TaskStatus::Planning {
        task.add_localized_log("任务状态已变化，忽略本次拆分结果", vec![]);
        return PlanningEffect::Ignored;
    }
    match result {
        Ok(outcome) => {
            apply_plan_outcome(task, outcome);
            PlanningEffect::AllocateWorktree
        }
        Err(error) => {
            mark_plan_failed(task, &error);
            PlanningEffect::PersistFailure
        }
    }
}
