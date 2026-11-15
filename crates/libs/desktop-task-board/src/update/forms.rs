//! 创建、复制及编辑任务的表单转换。
use desktop_task_model::{Task, TaskDraft, TaskStatus, SubTask};
use crate::draft::{normalize_task_model, set_task_executor_selection, set_draft_executor_selection};
use desktop_task_model::{TaskImportRecord, normalize_task_acp_agent_input};

pub fn duplicate_task(source_task: Task, initial_status: TaskStatus) -> Task {
    let mut copied_task = Task::new(source_task.priority);
    copied_task.assignee = source_task.assignee;
    copied_task.model = source_task.model;
    copied_task.acp_agent = source_task.acp_agent;
    copied_task.reasoning_effort = source_task.reasoning_effort;
    copied_task.description = source_task.description;
    copied_task.prompt = source_task.prompt;
    copied_task.status = initial_status;
    copied_task.auto_promote_delay_ms = source_task.auto_promote_delay_ms;
    copied_task.task_split_mode = source_task.task_split_mode;
    copied_task.subtasks = source_task
        .subtasks
        .iter()
        .map(|subtask| SubTask::new(subtask.content.clone()))
        .collect();
    copied_task
}

pub fn task_from_draft(draft: &TaskDraft, initial_status: TaskStatus, main_agent_key: &str) -> Option<Task> {
    if draft.prompt.trim().is_empty() { return None; }
    let priority = draft.priority.parse().unwrap_or(999);
    let auto_promote_delay_seconds: u64 =
        draft.auto_promote_delay_seconds.parse().unwrap_or(0);
    let auto_promote_delay_ms = auto_promote_delay_seconds.saturating_mul(1000);
    let model = normalize_task_model(&draft.model);

    let mut task = Task::new(priority);
    task.status = initial_status;
    task.model = model;
    task.agent =
        draft.agent.clone().or_else(|| Some(main_agent_key.to_string()));
    task.acp_agent = draft.acp_agent.clone();
    task.reasoning_effort = draft.reasoning_effort;
    task.task_split_mode = draft.task_split_mode;
    task.prompt = draft.prompt.clone();
    task.subtasks = draft
        .subtasks
        .iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .map(SubTask::new)
        .collect::<Vec<_>>();
    task.auto_promote_delay_ms =
        if auto_promote_delay_ms > 0 { Some(auto_promote_delay_ms) } else { None };
    Some(task)
}

pub fn apply_task_draft(task: &mut Task, draft: &TaskDraft, main_agent_key: &str, now_ms: u64) {
    task.priority = draft.priority.parse().unwrap_or(task.priority);
    task.model = normalize_task_model(&draft.model);
    task.agent =
        draft.agent.clone().or_else(|| Some(main_agent_key.to_string()));
    task.acp_agent = draft.acp_agent.clone();
    task.reasoning_effort = draft.reasoning_effort;
    task.task_split_mode = draft.task_split_mode;
    task.prompt = draft.prompt.clone();
    task.updated_at_ms = now_ms;
}

fn tasks_from_import(imported: Vec<TaskImportRecord>, default_priority: u32, draft: &TaskDraft, initial_status: TaskStatus, main_agent_key: &str) -> Vec<Task> {
    let mut tasks_to_create = Vec::new();
            for record in imported {
                let mut task = Task::new(record.priority.unwrap_or(default_priority));
                task.status = initial_status;
                task.prompt = record.prompt;
                task.model = record.model.unwrap_or_else(|| draft.model.clone());
                task.agent = record
                    .agent
                    .or_else(|| draft.agent.clone())
                    .or_else(|| Some(main_agent_key.to_string()));
                set_task_executor_selection(
                    &mut task,
                    record
                        .acp_agent
                        .as_deref()
                        .and_then(normalize_task_acp_agent_input)
                        .or_else(|| draft.acp_agent.clone()),
                );
                task.reasoning_effort =
                    record.reasoning_effort.or(draft.reasoning_effort);
                task.task_split_mode =
                    record.task_split_mode.unwrap_or(draft.task_split_mode);
                tasks_to_create.push(task);
            }

    tasks_to_create
}

pub fn draft_from_task(draft: &mut TaskDraft, task: &Task, main_agent_key: &str) {
    draft.priority =
        task.priority.to_string();
    draft.model =
        task.model.clone();
    draft.reasoning_effort =
        task.reasoning_effort;
    draft.agent = task
        .agent
        .clone()
        .or_else(|| Some(main_agent_key.to_string()));
    set_draft_executor_selection(
        draft,
        task.acp_agent.clone(),
    );
    draft.task_split_mode =
        task.task_split_mode;
    draft.prompt =
        task.prompt.clone();
    draft.subtasks = task
        .subtasks
        .iter()
        .map(|subtask| subtask.content.clone())
        .collect();
}

/// 从当前会话只读投影输入任务实际需要的模型选择。
pub struct InputModelSelection<'a> {
    pub auto_model: bool,
    pub model: &'a str,
    pub task_mode_enabled: bool,
    pub executor: Option<String>,
}
pub struct InputTaskOptions {
    pub priority: String,
    pub model: String,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub task_split_mode: desktop_task_model::TaskSplitMode,
    pub subtasks: Vec<String>,
}
pub fn task_from_input(
    content: &str, default_priority: u32, initial_status: TaskStatus,
    selection: InputModelSelection<'_>, options: Option<InputTaskOptions>,
) -> Option<Task> {
    let raw = content.trim();
    if raw.is_empty() { return None; }
    let priority = options.as_ref()
        .and_then(|options| options.priority.trim().parse::<u32>().ok().filter(|p| *p > 0))
        .unwrap_or(default_priority);
    let mut task = Task::new(priority);
    task.status = initial_status;
    if let Some(options) = options {
        let subtasks = options.subtasks.into_iter()
            .map(|line| line.trim().to_string()).filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        task.prompt = crate::helpers::review::build_task_prompt_from_input(raw, Some(priority), &subtasks);
        task.subtasks = subtasks.into_iter().map(SubTask::new).collect();
        task.reasoning_effort = options.reasoning_effort;
        task.task_split_mode = options.task_split_mode;
        if selection.task_mode_enabled {
            task.model = normalize_task_model(&options.model);
        } else if !selection.auto_model && !selection.model.trim().is_empty() {
            task.model = selection.model.trim().to_string();
        }
        set_task_executor_selection(&mut task, selection.executor);
    } else {
        task.prompt = crate::helpers::review::build_task_prompt_from_input(raw, Some(priority), &[]);
        if !selection.auto_model && !selection.model.trim().is_empty() {
            task.model = selection.model.trim().to_string();
        }
        set_task_executor_selection(&mut task, None);
    }
    Some(task)
}

pub fn tasks_from_import_content(content: &str, default_priority: u32, draft: &TaskDraft, initial_status: TaskStatus, main_agent_key: &str) -> Result<Vec<Task>, String> {
    if content.trim().is_empty() { return Ok(Vec::new()); }
    let imported = desktop_task_model::parse_task_import_content(content)?;
    Ok(tasks_from_import(imported, default_priority, draft, initial_status, main_agent_key))
}
