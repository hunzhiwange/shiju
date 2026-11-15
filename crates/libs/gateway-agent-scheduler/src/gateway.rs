//! 既有调度服务注入契约。

use anyhow::Result;
use async_trait::async_trait;
use gateway_types::ApiError;
use task_scheduler_core::domain::AgentSchedulerContext;
use vw_api_agent_scheduler::{
    AgentSchedulerBulkCommandRequest,
    AgentSchedulerBulkCommandResponse,
    AgentSchedulerCreateTaskRequest,
    AgentSchedulerCreateTaskResponse,
    AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse,
    AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventsResponse,
    AgentSchedulerTaskBoardStatusResponse,
    AgentSchedulerTaskBoardWorkflowResponse,
    AgentSchedulerTaskGraphResponse,
    AgentSchedulerTaskLogsResponse,
    AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerTaskMetadataUpdateResponse,
};
use vw_api_agent_scheduler::{
    AgentSchedulerCommandRequest as AgentSchedulerProjectCommandRequest,
    AgentSchedulerCommandResponse as AgentSchedulerProjectCommandResponse,
    AgentSchedulerWakeRequest as AgentSchedulerProjectWakeRequest,
    AgentSchedulerWakeResponse as AgentSchedulerProjectWakeResponse,
    AgentSchedulerOutboxDrainRequest as AgentSchedulerProjectOutboxDrainRequest,
    AgentSchedulerOutboxDrainResponse as AgentSchedulerProjectOutboxDrainResponse,
};
use crate::types::*;

#[async_trait]
pub trait AgentSchedulerGateway: Send + Sync {
    async fn create_task(
        &self,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError>;

    async fn create_task_scoped(
        &self,
        _context: AgentSchedulerContext,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        self.create_task(request).await
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError>;

    async fn wake(
        &self,
        _request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        Err(ApiError::internal("agent scheduler wake gateway is not configured"))
    }

    async fn recover(
        &self,
        _request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        Err(ApiError::internal("agent scheduler recovery gateway is not configured"))
    }

    async fn list_events(
        &self,
        query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError>;

    async fn drain_outbox(
        &self,
        request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError>;

    async fn command(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectCommandRequest,
    ) -> Result<AgentSchedulerProjectCommandResponse, ApiError> {
        Err(ApiError::internal("agent scheduler command gateway is not configured"))
    }

    async fn bulk_command(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, ApiError> {
        Err(ApiError::internal("agent scheduler bulk command gateway is not configured"))
    }

    async fn task_board_workflow(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board workflow gateway is not configured"))
    }

    async fn task_board_status(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board gateway is not configured"))
    }

    async fn task_graph(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task graph gateway is not configured"))
    }

    async fn replace_task_graph(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task graph gateway is not configured"))
    }

    async fn update_task_metadata(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _task_id: String,
        _request: AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board metadata gateway is not configured"))
    }

    async fn import_legacy_tasks(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, ApiError> {
        Err(ApiError::internal("agent scheduler legacy import gateway is not configured"))
    }

    async fn task_logs(
        &self,
        _project_path: AgentSchedulerContext,
        _task_id: String,
    ) -> Result<AgentSchedulerTaskLogsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler log gateway is not configured"))
    }

    async fn task_artifacts(
        &self,
        _project_path: AgentSchedulerContext,
        _task_id: String,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler artifact gateway is not configured"))
    }

    async fn task_board_events(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _query: AgentSchedulerProjectEventsQuery,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler event gateway is not configured"))
    }

    async fn wake_project(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectWakeRequest,
    ) -> Result<AgentSchedulerProjectWakeResponse, ApiError> {
        Err(ApiError::internal("agent scheduler project wake gateway is not configured"))
    }

    async fn drain_project_outbox(
        &self,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectOutboxDrainRequest,
    ) -> Result<AgentSchedulerProjectOutboxDrainResponse, ApiError> {
        Err(ApiError::internal("agent scheduler project outbox gateway is not configured"))
    }
}
