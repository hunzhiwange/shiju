use anyhow::Result;
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use super::domain::{AgentWorkerStatus, TaskId, TeamId, WorkerName};
use super::scheduler::worker_selection::{
    WorkerAvailability, WorkerSelectionRequest, select_worker_candidates,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::tasks::StoredAgentTask;
use super::store::workers::{AgentCapabilityRecord, StoredAgentWorker};

#[test]
fn agent_worker_selection_prefers_idle_skill_matched_workers() -> Result<()> {
    let team_id = TeamId::new("team-alpha")?;
    let workers = vec![
        worker("busy-copaw", AgentWorkerStatus::Busy, Some("copaw"), Some(&team_id), &["github"])?,
        worker(
            "idle-openclaw",
            AgentWorkerStatus::Idle,
            Some("openclaw"),
            Some(&team_id),
            &["github"],
        )?,
        worker(
            "stopped-copaw",
            AgentWorkerStatus::Stopped,
            Some("copaw"),
            Some(&team_id),
            &["github"],
        )?,
        worker(
            "idle-copaw",
            AgentWorkerStatus::Running,
            Some("copaw"),
            Some(&team_id),
            &["github"],
        )?,
        worker(
            "unavailable-copaw",
            AgentWorkerStatus::Unavailable,
            Some("copaw"),
            Some(&team_id),
            &["github"],
        )?,
        worker("missing-skill", AgentWorkerStatus::Idle, Some("copaw"), Some(&team_id), &["rust"])?,
    ];
    let active_tasks = vec![
        task("finite-1", "busy-copaw", AgentTaskKind::Finite)?,
        task("infinite-1", "busy-copaw", AgentTaskKind::Infinite)?,
        task("infinite-2", "idle-openclaw", AgentTaskKind::Infinite)?,
    ];

    let candidates = select_worker_candidates(
        &WorkerSelectionRequest {
            required_skills: vec!["github".to_string()],
            team_id: Some(team_id),
            preferred_runtime: Some("copaw".to_string()),
        },
        &workers,
        &active_tasks,
    );

    let candidate_ids =
        candidates.iter().map(|candidate| candidate.worker_id.as_str()).collect::<Vec<_>>();
    assert_eq!(candidate_ids, vec!["idle-copaw", "idle-openclaw", "busy-copaw", "stopped-copaw"]);
    assert_eq!(candidates[0].availability, WorkerAvailability::Idle);
    assert!(candidates[0].can_dispatch_now);
    assert_eq!(candidates[2].availability, WorkerAvailability::Busy);
    assert_eq!(candidates[2].finite_active_tasks, 1);
    assert_eq!(candidates[2].infinite_active_tasks, 1);
    assert_eq!(candidates[3].availability, WorkerAvailability::Stopped);
    assert!(candidates[3].requires_wakeup);

    Ok(())
}

#[test]
fn agent_worker_selection_handles_container_state_and_non_load_bearing_tasks() -> Result<()> {
    let idle = worker("idle", AgentWorkerStatus::Idle, Some("copaw"), None, &["github"])?;
    let mut stopped =
        worker("container-stopped", AgentWorkerStatus::Idle, Some("copaw"), None, &["github"])?;
    stopped.container_state = Some("exited".to_string());
    let mut failed =
        worker("container-failed", AgentWorkerStatus::Idle, Some("copaw"), None, &["github"])?;
    failed.container_state = Some("not_found".to_string());
    let pending = worker("pending", AgentWorkerStatus::Pending, Some("copaw"), None, &["github"])?;

    let mut completed = task("completed", "idle", AgentTaskKind::Finite)?;
    completed.status = AgentTaskStatus::Completed;
    let mut cancelled = task("cancelled", "idle", AgentTaskKind::Infinite)?;
    cancelled.status = AgentTaskStatus::Cancelled;
    let mut archived = task("archived", "idle", AgentTaskKind::Finite)?;
    archived.status = AgentTaskStatus::Archived;
    let team_dag = task("team-dag", "idle", AgentTaskKind::TeamDag)?;
    let mut unassigned = task("unassigned", "idle", AgentTaskKind::Finite)?;
    unassigned.assigned_to = None;

    let candidates = select_worker_candidates(
        &WorkerSelectionRequest {
            required_skills: vec!["  ".to_string(), "github".to_string()],
            team_id: None,
            preferred_runtime: Some(" copaw ".to_string()),
        },
        &[idle.clone(), stopped, failed, pending],
        &[completed, cancelled, archived, team_dag, unassigned],
    );

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].worker_id, idle.id);
    assert_eq!(candidates[0].availability, WorkerAvailability::Idle);
    assert_eq!(candidates[0].finite_active_tasks, 0);
    assert_eq!(candidates[0].infinite_active_tasks, 0);
    assert_eq!(candidates[0].active_tasks, 1);
    assert!(candidates[0].runtime_matched);
    assert_eq!(candidates[1].availability, WorkerAvailability::Stopped);
    assert!(candidates[1].requires_wakeup);

    Ok(())
}

fn worker(
    id: &str,
    status: AgentWorkerStatus,
    runtime: Option<&str>,
    team_id: Option<&TeamId>,
    skills: &[&str],
) -> Result<StoredAgentWorker> {
    Ok(StoredAgentWorker {
        id: WorkerName::new(id)?,
        runtime: runtime.map(str::to_string),
        deployment: Some("local".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status,
        desired_state: Some("Running".to_string()),
        team_id: team_id.cloned(),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some(format!("@{id}:example.test")),
        room_id: Some(format!("!{id}:example.test")),
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_783_000_000_000)),
        last_heartbeat_at: Some("2026-07-02T10:00:00Z".to_string()),
        capabilities: skills.iter().map(|skill| AgentCapabilityRecord::skill(*skill)).collect(),
        updated_at_ms: TimestampMs(1_783_000_000_000),
    })
}

fn task(id: &str, assigned_to: &str, kind: AgentTaskKind) -> Result<StoredAgentTask> {
    Ok(StoredAgentTask {
        id: TaskId::new(id)?,
        title: id.to_string(),
        kind,
        status: AgentTaskStatus::Running,
        spec_uri: format!("file:///{id}/spec.md"),
        meta_uri: format!("file:///{id}/meta.json"),
        assigned_to: Some(WorkerName::new(assigned_to)?),
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: Value::Null,
        created_at_ms: TimestampMs(1_783_000_000_000),
        updated_at_ms: TimestampMs(1_783_000_000_000),
    })
}
