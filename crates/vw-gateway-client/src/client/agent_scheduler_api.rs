use vw_api_types::agent_scheduler::{
    AgentSchedulerBulkCommandRequest, AgentSchedulerBulkCommandResponse,
    AgentSchedulerCommandRequest, AgentSchedulerCommandResponse, AgentSchedulerCreateTaskRequest,
    AgentSchedulerCreateTaskResponse, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse, AgentSchedulerOutboxDrainRequest,
    AgentSchedulerOutboxDrainResponse, AgentSchedulerScopeDto, AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskBoardStatusResponse,
    AgentSchedulerTaskBoardWorkflowResponse, AgentSchedulerTaskGraphResponse,
    AgentSchedulerTaskLogsResponse,
    AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerTaskMetadataUpdateResponse,
    AgentSchedulerTeamsResponse, AgentSchedulerUpsertTeamRequest, AgentSchedulerUpsertTeamResponse,
    AgentSchedulerUpsertWorkerRequest, AgentSchedulerUpsertWorkerResponse,
    AgentSchedulerWakeRequest, AgentSchedulerWakeResponse, AgentSchedulerWorkerRunEventRequest,
    AgentSchedulerWorkerRunEventResponse, AgentSchedulerWorkersResponse,
    AgentSchedulerReplaceTaskGraphRequest,
};

use super::GatewayClient;

impl GatewayClient {
    /// 创建或复用 Agent Scheduler 任务。
    pub async fn agent_scheduler_create_task(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        self.post_json(&format!("{}/tasks", agent_scheduler_project_path(scope)), &[], request)
            .await
    }

    /// 发送 Agent Scheduler 桌面任务板命令。
    pub async fn agent_scheduler_command(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerCommandRequest,
    ) -> Result<AgentSchedulerCommandResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_command_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 批量发送 Agent Scheduler 桌面任务板命令。
    pub async fn agent_scheduler_bulk_command(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_bulk_command_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板任务快照。
    pub async fn agent_scheduler_status(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_tasks_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 拉取 Agent Scheduler 项目任务图及实时就绪性快照。
    pub async fn agent_scheduler_task_graph(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskGraphResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_task_graph_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 完整替换 Agent Scheduler 项目任务图。
    pub async fn agent_scheduler_replace_task_graph(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_task_graph_path(scope);
        self.put_json(&path, &[], request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板 workflow 快照。
    pub async fn agent_scheduler_workflow(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_workflow_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 更新 Agent Scheduler 任务板任务元数据投影。
    pub async fn agent_scheduler_update_task_metadata(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
        request: &AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_task_metadata_path(scope, task_id);
        self.patch_json(&path, &[], request).await
    }

    /// 导入旧桌面任务到 Agent Scheduler。
    pub async fn agent_scheduler_import_legacy_tasks(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_legacy_import_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 拉取 Agent Scheduler Worker 快照。
    pub async fn agent_scheduler_workers(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerWorkersResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_workers_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 创建或更新 Agent Scheduler Worker 快照。
    pub async fn agent_scheduler_upsert_worker(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerUpsertWorkerRequest,
    ) -> Result<AgentSchedulerUpsertWorkerResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_workers_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 拉取 Agent Scheduler Team 快照。
    pub async fn agent_scheduler_teams(
        &self,
        scope: &AgentSchedulerScopeDto,
    ) -> Result<AgentSchedulerTeamsResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_teams_path(scope);
        self.get_json(&path, &[]).await
    }

    /// 创建或更新 Agent Scheduler Team 快照。
    pub async fn agent_scheduler_upsert_team(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerUpsertTeamRequest,
    ) -> Result<AgentSchedulerUpsertTeamResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_teams_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 拉取 Agent Scheduler 桌面任务板日志。
    pub async fn agent_scheduler_logs(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
    ) -> Result<AgentSchedulerTaskLogsResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_logs_path(scope, task_id);
        self.get_json(&path, &[]).await
    }

    /// 拉取 Agent Scheduler 桌面任务板产物。
    pub async fn agent_scheduler_artifacts(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_artifacts_path(scope, task_id);
        self.get_json(&path, &[]).await
    }

    /// 按 cursor 拉取 Agent Scheduler 桌面任务板事件。
    pub async fn agent_scheduler_events(
        &self,
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_events_path(scope);
        let query = after_cursor
            .map(|cursor| vec![("after_cursor".to_string(), cursor.to_string())])
            .unwrap_or_default();
        self.get_json(&path, &query).await
    }

    /// 构造 Agent Scheduler 事件 SSE 相对路径，供 EventSource 客户端复用。
    pub fn agent_scheduler_events_stream_path(
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
    ) -> String {
        let mut path = agent_scheduler_events_stream_path(scope);
        if let Some(cursor) = after_cursor {
            path.push_str("?after_cursor=");
            path.push_str(&cursor.to_string());
        }
        path
    }

    /// 唤醒 Agent Scheduler 调度器。
    pub async fn agent_scheduler_wake(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_wake_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 获取 Agent Scheduler outbox 待投递消息。
    pub async fn agent_scheduler_drain_outbox(
        &self,
        scope: &AgentSchedulerScopeDto,
        request: &AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_outbox_drain_path(scope);
        self.post_json(&path, &[], request).await
    }

    /// 上报 Matrix/remote Worker run 生命周期事件。
    pub async fn agent_scheduler_worker_run_event(
        &self,
        scope: &AgentSchedulerScopeDto,
        task_id: &str,
        run_id: &str,
        request: &AgentSchedulerWorkerRunEventRequest,
    ) -> Result<AgentSchedulerWorkerRunEventResponse, String> {
        self.ensure_scheduler_tenant_scope(scope)?;
        let path = agent_scheduler_worker_run_event_path(scope, task_id, run_id);
        self.post_json(&path, &[], request).await
    }

    fn ensure_scheduler_tenant_scope(&self, scope: &AgentSchedulerScopeDto) -> Result<(), String> {
        if scope.project_scope().tenant_scope() != self.endpoint().tenant_scope() {
            return Err("tenant scope does not match gateway endpoint tenant scope".to_string());
        }
        Ok(())
    }
}

fn agent_scheduler_project_path(scope: &AgentSchedulerScopeDto) -> String {
    format!(
        "/v1/agent-scheduler/projects/{}",
        encode_path_segment(scope.project_scope().project_id().as_ref())
    )
}

fn agent_scheduler_command_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/commands", agent_scheduler_project_path(scope))
}

fn agent_scheduler_bulk_command_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/bulk", agent_scheduler_command_path(scope))
}

fn agent_scheduler_tasks_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/tasks", agent_scheduler_project_path(scope))
}

fn agent_scheduler_task_graph_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/task-graph", agent_scheduler_project_path(scope))
}

fn agent_scheduler_workflow_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/workflow", agent_scheduler_project_path(scope))
}

fn agent_scheduler_task_metadata_path(scope: &AgentSchedulerScopeDto, task_id: &str) -> String {
    format!(
        "{}/tasks/{}/metadata",
        agent_scheduler_project_path(scope),
        encode_path_segment(task_id)
    )
}

fn agent_scheduler_legacy_import_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/legacy-import", agent_scheduler_project_path(scope))
}

fn agent_scheduler_workers_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/workers", agent_scheduler_project_path(scope))
}

fn agent_scheduler_teams_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/teams", agent_scheduler_project_path(scope))
}

fn agent_scheduler_logs_path(scope: &AgentSchedulerScopeDto, task_id: &str) -> String {
    format!("{}/tasks/{}/logs", agent_scheduler_project_path(scope), encode_path_segment(task_id))
}

fn agent_scheduler_artifacts_path(scope: &AgentSchedulerScopeDto, task_id: &str) -> String {
    format!(
        "{}/tasks/{}/artifacts",
        agent_scheduler_project_path(scope),
        encode_path_segment(task_id)
    )
}

fn agent_scheduler_events_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/events", agent_scheduler_project_path(scope))
}

fn agent_scheduler_events_stream_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/stream", agent_scheduler_events_path(scope))
}

fn agent_scheduler_wake_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/wake", agent_scheduler_project_path(scope))
}

fn agent_scheduler_outbox_drain_path(scope: &AgentSchedulerScopeDto) -> String {
    format!("{}/outbox/drain", agent_scheduler_project_path(scope))
}

fn agent_scheduler_worker_run_event_path(
    scope: &AgentSchedulerScopeDto,
    task_id: &str,
    run_id: &str,
) -> String {
    format!(
        "{}/tasks/{}/runs/{}/worker-events",
        agent_scheduler_project_path(scope),
        encode_path_segment(task_id),
        encode_path_segment(run_id)
    )
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
