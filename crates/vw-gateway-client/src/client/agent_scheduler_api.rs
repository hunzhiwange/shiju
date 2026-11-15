use vw_api_agent_scheduler::{
    AgentSchedulerBulkCommandRequest, AgentSchedulerBulkCommandResponse,
    AgentSchedulerCommandRequest, AgentSchedulerCommandResponse, AgentSchedulerCreateTaskRequest,
    AgentSchedulerCreateTaskResponse, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse, AgentSchedulerOutboxDrainRequest,
    AgentSchedulerOutboxDrainResponse, AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerScopeDto, AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskBoardStatusResponse,
    AgentSchedulerTaskBoardWorkflowResponse, AgentSchedulerTaskGraphResponse,
    AgentSchedulerTaskLogsResponse, AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerTaskMetadataUpdateResponse, AgentSchedulerWakeRequest,
    AgentSchedulerWakeResponse,
};

use super::GatewayClient;

impl GatewayClient {
    /// 创建或复用 Agent Scheduler 任务。
    pub async fn agent_scheduler_create_task(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_create_task(scope, request).await
    }

    /// 发送 Agent Scheduler 桌面任务板命令。
    pub async fn agent_scheduler_command(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerCommandRequest,
    ) -> Result<AgentSchedulerCommandResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_command(scope, request).await
    }

    /// 批量发送 Agent Scheduler 桌面任务板命令。
    pub async fn agent_scheduler_bulk_command(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_bulk_command(scope, request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板任务快照。
    pub async fn agent_scheduler_status(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_status(scope).await
    }

    /// 拉取 Agent Scheduler 项目任务图及实时就绪性快照。
    pub async fn agent_scheduler_task_graph(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskGraphResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_task_graph(scope).await
    }

    /// 完整替换 Agent Scheduler 项目任务图。
    pub async fn agent_scheduler_replace_task_graph(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_replace_task_graph(scope, request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板 workflow 快照。
    pub async fn agent_scheduler_workflow(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_workflow(scope).await
    }

    /// 更新 Agent Scheduler 任务板任务元数据投影。
    pub async fn agent_scheduler_update_task_metadata(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
        request: &AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_update_task_metadata(scope, task_id, request).await
    }

    /// 导入旧桌面任务到 Agent Scheduler。
    pub async fn agent_scheduler_import_legacy_tasks(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_import_legacy_tasks(scope, request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板日志。
    pub async fn agent_scheduler_logs(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
    ) -> Result<AgentSchedulerTaskLogsResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_logs(scope, task_id).await
    }

    /// 拉取 Agent Scheduler 桌面任务板产物。
    pub async fn agent_scheduler_artifacts(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_artifacts(scope, task_id).await
    }

    /// 按 cursor 拉取 Agent Scheduler 桌面任务板事件。
    pub async fn agent_scheduler_events(
        &self,
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_events(scope, after_cursor).await
    }

    /// 构造 Agent Scheduler 事件 SSE 相对路径，供 EventSource 客户端复用。
    pub fn agent_scheduler_events_stream_path(
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
    ) -> String {
        gateway_scheduler::GatewaySchedulerClient::agent_scheduler_events_stream_path(scope, after_cursor)
    }

    /// 唤醒 Agent Scheduler 调度器。
    pub async fn agent_scheduler_wake(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_wake(scope, request).await
    }

    /// 获取 Agent Scheduler outbox 待投递消息。
    pub async fn agent_scheduler_drain_outbox(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, String> {
        gateway_scheduler::GatewaySchedulerClient::new(self.transport().clone()).agent_scheduler_drain_outbox(scope, request).await
    }
}
