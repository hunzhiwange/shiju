use vw_shared::todo::Todo;

use super::super::model::UiTodoOverlay;
use super::super::model::background_task::{
    BackgroundTask, BackgroundTaskPanel, BackgroundTaskStatus,
};
use super::gateway::{GatewayUiRuntime, block_on_gateway};
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto,
};
use vw_gateway_client::vw_api_types::id::TaskId;

pub(crate) trait TasksRuntime {
    fn list_background_tasks(&mut self) -> Result<Vec<BackgroundTask>, String>;
    fn save_todos(&mut self, session_id: &str, todos: Vec<Todo>) -> Result<(), String>;
    fn stop_task(&mut self, task_id: &str) -> Result<(), String>;
}

impl TasksRuntime for GatewayUiRuntime {
    fn list_background_tasks(&mut self) -> Result<Vec<BackgroundTask>, String> {
        let scope = self.agent_scheduler_scope_blocking()?;
        let current_session = self.session_id().unwrap_or_default().to_string();
        block_on_gateway(self.client().agent_scheduler_status(&scope)).map(|response| {
            response
                .tasks
                .into_iter()
                .map(|task| {
                    let status = scheduler_status(task.status_key.as_str());
                    BackgroundTask {
                        id: task.task_id.as_ref().to_string(),
                        session_id: current_session.clone(),
                        kind: super::super::model::background_task::BackgroundTaskKind::Agent,
                        title: task.read_model.title.unwrap_or_else(|| {
                            task.prompt.lines().next().unwrap_or("Agent task").to_string()
                        }),
                        status,
                        detail: format!(
                            "status={} model={} run={}",
                            task.status_key,
                            task.model,
                            task.run_id.as_deref().unwrap_or("-")
                        ),
                        log: Vec::new(),
                        supports_stop: false,
                    }
                })
                .collect()
        })
    }

    fn save_todos(&mut self, session_id: &str, todos: Vec<Todo>) -> Result<(), String> {
        self.session_todo_update_blocking(Some(session_id), &todos)
    }

    fn stop_task(&mut self, task_id: &str) -> Result<(), String> {
        let scope = self.agent_scheduler_scope_blocking()?;
        let request = AgentSchedulerCommandRequest {
            task_id: TaskId::from(task_id),
            command: AgentSchedulerCommandKindDto::Cancel,
            dispatch_mode: AgentSchedulerDispatchModeDto::Local,
            max_concurrent: None,
            reason: Some("cancelled from TUI".to_string()),
            payload: AgentSchedulerCommandPayloadDto::default(),
        };
        block_on_gateway(self.client().agent_scheduler_command(&scope, &request)).and_then(
            |response| {
                response
                    .accepted
                    .then_some(())
                    .ok_or_else(|| "scheduler rejected task cancellation".to_string())
            },
        )
    }
}

fn scheduler_status(status: &str) -> BackgroundTaskStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "queued" | "pending" | "draft" => BackgroundTaskStatus::Queued,
        "running" | "assigned" | "dispatched" => BackgroundTaskStatus::Running,
        "completed" | "done" | "merged" => BackgroundTaskStatus::Completed,
        "cancelled" | "canceled" | "archived" => BackgroundTaskStatus::Cancelled,
        _ => BackgroundTaskStatus::Failed,
    }
}

pub(crate) fn refresh_background_tasks(
    panel: &mut BackgroundTaskPanel,
    runtime: &mut dyn TasksRuntime,
) {
    panel.loading = true;
    match runtime.list_background_tasks() {
        Ok(tasks) => panel.replace(tasks),
        Err(error) => {
            panel.loading = false;
            panel.error = Some(error);
        }
    }
}

pub(crate) fn save_todos(
    overlay: &mut UiTodoOverlay,
    runtime: &mut dyn TasksRuntime,
) -> Result<(), String> {
    let session_id =
        overlay.session_id.as_deref().ok_or_else(|| "todo session id is required".to_string())?;
    let todos = overlay
        .items
        .iter()
        .map(|item| Todo {
            id: item.id.clone(),
            content: item.content.clone(),
            status: item.status.clone(),
            priority: item.priority.clone(),
        })
        .collect();
    runtime.save_todos(session_id, todos)?;
    overlay.dirty = false;
    Ok(())
}

pub(crate) fn stop_background_task(
    task: &mut BackgroundTask,
    current_session_id: &str,
    confirmed: bool,
    runtime: &mut dyn TasksRuntime,
) -> Result<(), String> {
    if task.session_id != current_session_id {
        return Err("background task is outside the current session".to_string());
    }
    if !task.supports_stop {
        return Err("gateway does not expose a stop operation for this task".to_string());
    }
    if !confirmed {
        return Err("stopping a background task requires confirmation".to_string());
    }
    runtime.stop_task(&task.id)?;
    task.status = BackgroundTaskStatus::Cancelled;
    Ok(())
}
