use std::time::{SystemTime, UNIX_EPOCH};

use vw_gateway_client::vw_api_types::task::{
    TaskPlanRunMode, TaskPlanRunRequest, TaskPlanRunStatusResponse,
};
use vw_gateway_client::{
    GatewayClient, GatewayStreamAttemptResult, GatewayStreamResume,
};
use vw_shared::session::info::Info as SessionInfo;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_task_plan_run_event,
};
use crate::cli::tui_v2::model::{GoalMode, PromptSubmission, WorkflowMode};

const PLAN_RUN_RESUME_ATTEMPTS: usize = 3;

pub(crate) fn plan_run_request_for_submission(
    submission: &PromptSubmission,
) -> Result<Option<TaskPlanRunRequest>, String> {
    // 工作流或目标任一开启时进入 plan-run（路由细化由后续任务处理）。
    if submission.execution.workflow_mode == WorkflowMode::Disabled
        && submission.execution.goal_mode == GoalMode::Disabled
    {
        return Ok(None);
    }
    if submission.text.trim().is_empty() {
        return Ok(None);
    }
    let mode = if submission.execution.goal_mode == GoalMode::Enabled {
        TaskPlanRunMode::Goal
    } else {
        TaskPlanRunMode::Workflow
    };
    let root = submission
        .root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .ok_or_else(|| "计划运行需要有效的项目根目录".to_string())?;
    Ok(Some(TaskPlanRunRequest {
        query: submission.text.clone(),
        root: root.to_string(),
        model: submission.model.clone(),
        agent: submission.execution.delegate_agent.clone(),
        acp_agent: submission.execution.acp_backend.clone(),
        parent_session_id: submission.session_id.clone(),
        mode,
    }))
}

impl GatewayUiRuntime {
    pub(crate) fn session_is_plan_run_child_blocking(
        &self,
        session_id: &str,
    ) -> Result<bool, String> {
        let directory = self.directory().to_str();
        block_on_gateway(
            self.client()
                .session_get::<SessionInfo>(session_id, directory),
        )
        .map(|info| info.parent_id.is_some() && info.is_plan_run())
    }

    pub(crate) fn stream_task_plan_run_blocking(
        &self,
        request: &TaskPlanRunRequest,
        mut on_event: impl FnMut(UiRuntimeEvent) -> bool,
    ) -> UiRuntimeTerminalEvent {
        let request_id = plan_run_request_id();
        let mut last_event_id = None;
        let mut terminal = None;

        for _ in 0..PLAN_RUN_RESUME_ATTEMPTS {
            let resume = GatewayStreamResume {
                request_id: request_id.clone(),
                last_event_id,
            };
            let mut next_event_id = last_event_id;
            let attempt = block_on_gateway(GatewayClient::stream_task_plan_run_resumable(
                self.endpoint(),
                request,
                &resume,
                |event| {
                    next_event_id = Some(event.id);
                    for runtime_event in adapt_task_plan_run_event(event.event) {
                        if let UiRuntimeEvent::Terminal(runtime_terminal) = &runtime_event {
                            terminal = Some(runtime_terminal.clone());
                        }
                        if !on_event(runtime_event) {
                            return false;
                        }
                    }
                    true
                },
            ));
            last_event_id = next_event_id;
            match attempt {
                Ok(GatewayStreamAttemptResult::Terminated) => {
                    return terminal.unwrap_or_else(|| {
                        UiRuntimeTerminalEvent::Error(
                            "plan-run stream terminated without terminal event".to_string(),
                        )
                    });
                }
                Ok(GatewayStreamAttemptResult::CallerStopped) => {
                    return terminal.unwrap_or(UiRuntimeTerminalEvent::Cancelled {
                        reason: Some("plan-run stream cancelled by TUI".to_string()),
                        usage: None,
                        message_id: None,
                        parent_message_id: None,
                    });
                }
                Ok(GatewayStreamAttemptResult::Incomplete) => {}
                Err(error) => return UiRuntimeTerminalEvent::Error(error),
            }
        }

        UiRuntimeTerminalEvent::Error(
            "plan-run stream ended before a terminal event after resume attempts".to_string(),
        )
    }

    pub(crate) fn task_plan_run_stop_blocking(
        &self,
        task_id: &str,
    ) -> Result<TaskPlanRunStatusResponse, String> {
        block_on_gateway(self.client().task_plan_run_stop(task_id))
    }
}

fn plan_run_request_id() -> String {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("tui-plan-run-{nanos}")
}
