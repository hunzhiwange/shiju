//! 测试调度引擎对过期租约和中断运行的恢复策略。
//! 过期与仍有效租约并列验证，恢复流程只接管真正失联的运行。

use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::recovery::{RecoveryEngine, RecoveryPolicy, StaleRunResolution};
use crate::agent_scheduler::adapters::controller::{
    FakeControllerAdapter, WorkerControllerSnapshot, WorkerLifecycleState,
};
use crate::agent_scheduler::domain::{
    AgentWorkerStatus, HolderId, ResourceId, ResourceLeaseId, RunId, TaskId, TeamId, WorkerName,
};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::resources::{
    AcquireResourceLeaseRequest, ResourceKind, ResourceLeaseStatus,
};
use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;
use crate::agent_scheduler::store::tasks::NewAgentTask;
use crate::agent_scheduler::store::workers::{AgentCapabilityRecord, NewAgentWorkerSnapshot};

#[tokio::test]
async fn agent_recovery_marks_stale_runs_and_releases_resources_after_grace() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let task_id = TaskId::new("task-recovery-1")?;
    let run_id = RunId::new("run-recovery-1")?;
    let resource_id = ResourceId::new("worktree/task-recovery-1")?;

    store.initialize_project(project_path)?;
    store.create_task(project_path, dispatched_task(&task_id)?)?;
    store.upsert_worker_snapshot(project_path, running_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-recovery")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-recovery:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });
    store.claim_task_run(
        project_path,
        ClaimTaskRunRequest {
            run_id: run_id.clone(),
            task_id: task_id.clone(),
            lease_acquired_at_ms: TimestampMs(1_000),
            lease_expires_at_ms: Some(TimestampMs(1_500)),
        },
    )?;
    store.acquire_resource_lease(
        project_path,
        AcquireResourceLeaseRequest {
            lease_id: ResourceLeaseId::new("lease-recovery-1")?,
            resource_kind: ResourceKind::Worktree,
            resource_id: resource_id.clone(),
            holder_id: HolderId::new(run_id.as_str())?,
            acquired_at_ms: TimestampMs(1_000),
            lease_expires_at_ms: Some(TimestampMs(1_500)),
        },
    )?;

    let policy = RecoveryPolicy {
        stale_run_grace_ms: 60_000,
        stale_run_resolution: StaleRunResolution::FailTask,
        admin_notification_target: Some("room:!admin:example.test".to_string()),
        ..RecoveryPolicy::default()
    };
    let engine = RecoveryEngine::with_policy(&store, &controller, policy);

    let first = engine.reconcile_once(project_path, TimestampMs(1_500)).await?;
    assert_eq!(first.graced_run_ids, vec![run_id.clone()]);
    assert!(first.recovered_run_ids.is_empty());
    assert!(first.released_resource_lease_ids.is_empty());
    assert_eq!(
        store.list_stale_resource_leases(project_path, TimestampMs(1_500))?[0].status,
        ResourceLeaseStatus::Active
    );

    let second = engine.reconcile_once(project_path, TimestampMs(61_500)).await?;
    assert_eq!(second.recovered_run_ids, vec![run_id.clone()]);
    assert_eq!(second.released_resource_lease_ids, vec![ResourceLeaseId::new("lease-recovery-1")?]);
    assert!(store.list_stale_runs(project_path, TimestampMs(600_000))?.is_empty());
    assert!(store.list_stale_resource_leases(project_path, TimestampMs(600_000))?.is_empty());

    let task = store.load_task(project_path, &task_id)?.expect("task still exists");
    assert_eq!(task.status, AgentTaskStatus::Failed);

    let notifications = store.claim_outbox_batch(project_path, TimestampMs(61_501), 10)?;
    assert!(notifications.iter().any(|message| {
        message.target == "room:!admin:example.test"
            && message.body.contains("run-recovery-1")
            && message.payload["kind"] == "recovery"
    }));

    Ok(())
}


fn dispatched_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Recover stale agent run".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Dispatched,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: Some(WorkerName::new("worker-recovery")?),
        team_id: Some(TeamId::new("team-alpha")?),
        room_id: Some("!worker-recovery:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "recovery-test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn running_worker() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("worker-recovery")?,
        runtime: Some("copaw".to_string()),
        deployment: Some("local".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Running,
        desired_state: Some("Running".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-recovery:example.test".to_string()),
        room_id: Some("!worker-recovery:example.test".to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_300)),
        last_heartbeat_at: Some("2026-07-02T10:01:40Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill("recovery")],
        updated_at_ms: TimestampMs(1_300),
    })
}
