//! 将宿主存储作用域与生命周期动作绑定到核心策略。
use anyhow::Result;
use serde_json::Value;
use vw_api_transport::common::TimestampMs;
use vw_api_task::TaskPoolWorkflowSnapshotDto;
use super::domain::{RunId, TaskId};
use super::state_machine::AgentTaskStatus;
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use super::store::tasks::{AgentTaskFilter, StoredAgentTask};
use task_scheduler_core::policy_port::*;
pub use task_scheduler_core::policy::{AgentSchedulerPolicyConfig, AgentSchedulerPolicyOutcome};

pub struct AgentSchedulerPolicyService<'a> {
    store: &'a AgentSchedulerStore,
}

impl<'a> AgentSchedulerPolicyService<'a> {
    pub fn new(store: &'a AgentSchedulerStore) -> Self { Self { store } }

    pub fn apply_project_policy(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        config: &AgentSchedulerPolicyConfig,
    ) -> Result<AgentSchedulerPolicyOutcome> {
        let host = ScopedPolicyHost { store: self.store, context: project_path };
        task_scheduler_core::policy::AgentSchedulerPolicyService::new(&host)
            .apply_project_policy(now_ms, config)
    }
}

struct ScopedPolicyHost<'a, C: AgentSchedulerStorageContext + ?Sized> {
    store: &'a AgentSchedulerStore,
    context: &'a C,
}

impl<C: AgentSchedulerStorageContext + ?Sized> PolicyHost for ScopedPolicyHost<'_, C> {
    fn load_task_board_workflow(&self) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
        self.store.load_task_board_workflow(self.context)
    }
    fn load_dispatch_workflow(&self) -> Result<TaskPoolWorkflowSnapshotDto> {
        super::engine::dispatch::load_dispatch_workflow(self.store, self.context)
    }
    fn list_active_tasks(&self) -> Result<Vec<StoredAgentTask>> {
        self.store.list_active_tasks(self.context, AgentTaskFilter::default())
    }
    fn task_board_status_key(&self, task: &StoredAgentTask) -> String {
        super::task_board::task_board_status_key(task)
    }
    fn resolve_outcome(&self, workflow: &TaskPoolWorkflowSnapshotDto, from: &str, outcome: &str) -> Result<PolicyStatusProjection> {
        let target = super::lifecycle_bridge::LifecycleBridge::new(workflow).resolve_outcome(from, outcome)?;
        Ok(PolicyStatusProjection { status_key: target.status_key, status_role: target.status_role, runtime_status: target.runtime_status })
    }
    fn transition_task_status_with_metadata(&self, task_id: &TaskId, status: AgentTaskStatus, metadata: Value, now: TimestampMs) -> Result<Option<StoredAgentTask>> {
        self.store.transition_task_status_with_metadata(self.context, task_id, status, metadata, now)
    }
    fn load_latest_run_for_task(&self, task_id: &TaskId) -> Result<Option<PolicyRunSnapshot>> {
        Ok(self.store.load_latest_run_for_task(self.context, task_id)?.map(|run| PolicyRunSnapshot {
            id: run.id, attempt: run.attempt, lease_acquired_at_ms: run.lease_acquired_at_ms,
            started_at_ms: run.started_at_ms, finished_at_ms: run.finished_at_ms,
            last_heartbeat_at_ms: run.last_heartbeat_at_ms, error_message: run.error_message,
        }))
    }
    fn list_task_logs(&self, task_id: &TaskId) -> Result<Vec<PolicyLogSnapshot>> {
        Ok(self.store.list_task_logs(self.context, task_id)?.into_iter().map(|log| PolicyLogSnapshot {
            timestamp_ms: log.timestamp_ms, message: log.message,
        }).collect())
    }
    fn finish_timed_out_run(&self, run_id: &RunId, now: TimestampMs, error: Option<String>) -> Result<()> {
        self.store.finish_run(self.context, run_id, super::store::runs::AgentRunFinishStatus::TimedOut, now, error)?;
        Ok(())
    }
    fn append_task_log(&self, request: PolicyLogRequest) -> Result<()> {
        self.store.append_task_log(self.context, super::store::logs::AppendAgentTaskLogRequest {
            task_id: request.task_id, run_id: request.run_id, timestamp_ms: request.timestamp_ms,
            from_status: request.from_status, to_status: request.to_status,
            from_status_key: request.from_status_key, to_status_key: request.to_status_key, message: request.message,
        })?;
        Ok(())
    }
    fn append_event(&self, request: PolicyEventRequest) -> Result<()> {
        self.store.append_event(self.context, super::store::events::AppendAgentEventRequest {
            task_id: request.task_id, run_id: request.run_id, worker_id: request.worker_id, team_id: request.team_id,
            kind: request.kind, from_status: request.from_status, to_status: request.to_status,
            from_status_key: request.from_status_key, to_status_key: request.to_status_key,
            message: request.message, payload: request.payload, created_at_ms: request.created_at_ms,
        })?;
        Ok(())
    }
}
