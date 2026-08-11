use anyhow::Result;
use serde_json::{Value, json};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

use super::domain::{AgentRunStatus, TaskId, WorkerName};
use super::events::AgentEventKind;
use super::policy::{
    AgentSchedulerPolicyConfig, AgentSchedulerPolicyOutcome, AgentSchedulerPolicyService,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::logs::AgentSchedulerWorkspaceContext;
use super::store::runs::{AgentRunFinishStatus, ClaimTaskRunRequest};
use super::store::tasks::NewAgentTask;
use crate::task_lifecycle::DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID;

#[test]
fn agent_scheduler_policy_promotes_retries_times_out_and_plans_merge() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    store.save_task_board_workflow(
        &project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "dev_lifecycle".to_string(),
            label: "Development Lifecycle".to_string(),
            statuses: vec![
                workflow_status("queued", TaskPoolWorkflowStatusRoleDto::Queued),
                workflow_status_with_transitions(
                    "running",
                    TaskPoolWorkflowStatusRoleDto::Executing,
                    vec![workflow_transition("failure", "failed")],
                ),
                workflow_status("code_complete", TaskPoolWorkflowStatusRoleDto::ReviewReady),
                workflow_status_with_transitions(
                    "failed",
                    TaskPoolWorkflowStatusRoleDto::Failed,
                    vec![workflow_transition("retry", "queued")],
                ),
            ],
        },
    )?;

    let pool_id = TaskId::new("task-policy-pool")?;
    let retry_id = TaskId::new("task-policy-retry")?;
    let running_id = TaskId::new("task-policy-timeout")?;
    let merge_id = TaskId::new("task-policy-skip-review")?;

    store.create_task(
        &project_path,
        finite_task(
            &pool_id,
            AgentTaskStatus::Draft,
            json!({
                "task_board.status_key": "pool",
                "task_board.status_role": "backlog",
                "auto_promote_delay_ms": 0
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store.create_task(
        &project_path,
        finite_task(
            &retry_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed",
                "task_board.status_role": "failed",
                "last_error": "model response timed out",
                "last_active_at_ms": 1_000,
                "retry_count": 1
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store.create_task(
        &project_path,
        finite_task(
            &running_id,
            AgentTaskStatus::Running,
            json!({
                "task_board.status_key": "running",
                "task_board.status_role": "executing"
            }),
            TimestampMs(1_000),
        )?,
    )?;
    let run_id = super::domain::RunId::new("run-policy-timeout")?;
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: running_id.clone(),
                worker_id: WorkerName::new("policy-worker")?,
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(10_000)),
            },
        )?
        .expect("running task run claimed");
    store.create_task(
        &project_path,
        finite_task(
            &merge_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "code_complete",
                "task_board.status_role": "review_ready",
                "merge_source_branch": "feature/policy",
                "merge_target_branch": "main",
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/policy"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let service = AgentSchedulerPolicyService::new(&store);
    let config = AgentSchedulerPolicyConfig {
        failed_retry_minutes: 1,
        running_timeout_ms: 1,
        ..AgentSchedulerPolicyConfig::default()
    };
    let first = service.apply_project_policy(&project_path, TimestampMs(121_000), &config)?;
    assert!(first.promoted_task_ids.is_empty());
    assert_eq!(first.retry_task_ids, vec![retry_id.clone()]);
    assert_eq!(first.timeout_grace_task_ids, vec![running_id.clone()]);
    assert!(first.skipped_review_task_ids.is_empty());
    assert!(first.merge_task_ids.is_empty());

    assert_eq!(
        store.load_task(&project_path, &pool_id)?.expect("pool task").status,
        AgentTaskStatus::Draft
    );
    assert_eq!(
        store.load_task(&project_path, &retry_id)?.expect("retry task").status,
        AgentTaskStatus::Queued
    );
    let merge_task = store.load_task(&project_path, &merge_id)?.expect("merge task");
    assert_eq!(merge_task.status, AgentTaskStatus::Submitted);
    assert_eq!(merge_task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(merge_task.metadata["task_board.status_role"], "review_ready");

    let second = service.apply_project_policy(&project_path, TimestampMs(122_000), &config)?;
    assert_eq!(second.timed_out_task_ids, vec![running_id.clone()]);
    let running_task = store.load_task(&project_path, &running_id)?.expect("running task");
    assert_eq!(running_task.status, AgentTaskStatus::Failed);
    let run = store.load_run(&project_path, &run_id)?.expect("timeout run");
    assert_eq!(run.status, AgentRunStatus::TimedOut);

    let timeout_logs = store.list_task_logs(&project_path, &running_id)?;
    assert!(timeout_logs.iter().any(|log| log.message.contains("超时保活")));
    assert!(timeout_logs.iter().any(|log| {
        log.to_status == Some(AgentTaskStatus::Failed) && log.message.contains("two policy windows")
    }));
    let events = store.list_events_after(&project_path, None, 100)?;
    assert!(events.iter().any(|event| event.kind == AgentEventKind::TaskResumed));
    assert!(events.iter().any(|event| event.kind == AgentEventKind::RunFailed));

    Ok(())
}

#[test]
fn agent_scheduler_policy_leaves_pool_task_draft_without_legacy_auto_promote() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-pool-disabled");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;

    let pool_id = TaskId::new("task-policy-pool-disabled")?;
    store.create_task(
        &project_path,
        finite_task(
            &pool_id,
            AgentTaskStatus::Draft,
            json!({
                "task_board.status_key": "pool",
                "task_board.status_role": "backlog",
                "auto_promote_delay_ms": 0
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(121_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.promoted_task_ids.is_empty());
    assert_eq!(
        store.load_task(&project_path, &pool_id)?.expect("pool task").status,
        AgentTaskStatus::Draft
    );
    Ok(())
}

#[test]
fn agent_scheduler_policy_promotes_due_pool_task_when_explicitly_enabled() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-pool-enabled");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;

    let pool_id = TaskId::new("task-policy-pool-enabled")?;
    store.create_task(
        &project_path,
        finite_task(
            &pool_id,
            AgentTaskStatus::Draft,
            json!({
                "task_board.status_key": "pool",
                "task_board.status_role": "backlog",
                "auto_promote_delay_ms": 0
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(1_001),
        &AgentSchedulerPolicyConfig {
            auto_promote_pool_tasks: true,
            ..AgentSchedulerPolicyConfig::default()
        },
    )?;

    assert_eq!(outcome.promoted_task_ids, vec![pool_id.clone()]);
    let task = store.load_task(&project_path, &pool_id)?.expect("pool task");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(task.metadata["task_board.status_role"], "queued");
    Ok(())
}

#[test]
fn agent_scheduler_policy_leaves_review_ready_task_for_stage_action() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-workspace");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-policy-workspace-merge")?;
    let run_id = super::domain::RunId::new("run-policy-workspace-merge")?;
    let source_branch = "feature/policy-workspace";
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Running,
            json!({
                "task_board.status_key": "running",
                "task_board.status_role": "executing",
                "merge_target_branch": "main"
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new("policy-worker")?,
                lease_acquired_at_ms: TimestampMs(1_100),
                lease_expires_at_ms: None,
            },
        )?
        .expect("workspace task run claimed");
    store
        .record_run_workspace_context(
            &project_path,
            &task_id,
            &run_id,
            AgentSchedulerWorkspaceContext {
                slot_id: "slot-policy-workspace".to_string(),
                path: project_path.to_string_lossy().to_string(),
                branch: source_branch.to_string(),
            },
            TimestampMs(1_200),
        )?
        .expect("workspace recorded");
    store.finish_run(
        &project_path,
        &run_id,
        AgentRunFinishStatus::Succeeded,
        TimestampMs(1_250),
        None,
    )?;
    store
        .transition_task_status(
            &project_path,
            &task_id,
            AgentTaskStatus::Submitted,
            TimestampMs(1_300),
        )?
        .expect("task submitted");

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.skipped_review_task_ids.is_empty());
    assert!(outcome.merge_task_ids.is_empty());
    assert_eq!(outcome.blocked_task_ids, Vec::<TaskId>::new());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");

    Ok(())
}

#[test]
fn agent_scheduler_policy_no_longer_reads_legacy_review_switch() -> Result<()> {
    let source = include_str!("policy.rs");
    assert!(!source.contains("code_review_enabled"));

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-review-cutover");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-policy-review-cutover")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "code_complete",
                "task_board.status_role": "review_ready",
                "merge_source_branch": "feature/review-cutover",
                "merge_target_branch": "main",
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/review-cutover"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.review_task_ids.is_empty());
    assert!(outcome.skipped_review_task_ids.is_empty());
    assert!(outcome.merge_task_ids.is_empty());
    assert!(outcome.blocked_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");

    Ok(())
}

#[test]
fn agent_scheduler_policy_leaves_review_ready_without_merge_metadata_for_stage_action() -> Result<()>
{
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-block");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-policy-missing-merge")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "code_complete",
                "task_board.status_role": "review_ready"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let service = AgentSchedulerPolicyService::new(&store);
    let outcome = service.apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.blocked_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.is_empty());
    let events = store.list_events_after(&project_path, None, 100)?;
    assert!(!events.iter().any(|event| event.kind == AgentEventKind::TaskBlocked));

    Ok(())
}

#[test]
fn agent_scheduler_policy_completes_marketing_submitted_task_without_git_merge() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-marketing");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    store.save_task_board_workflow(&project_path, &marketing_workflow())?;
    let task_id = TaskId::new("task-policy-marketing")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.workflow_id": "marketing_campaign",
                "task_board.status_key": "campaign_review_ready",
                "task_board.status_role": "review_ready"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.merge_task_ids.is_empty());
    assert!(outcome.blocked_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    assert_eq!(task.metadata["task_board.status_key"], "campaign_done");
    assert_eq!(task.metadata["task_board.status_role"], "done");
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.message.contains("without git merge")
            && log.to_status == Some(AgentTaskStatus::Completed)
    }));

    Ok(())
}

#[test]
fn agent_scheduler_policy_retries_failed_task_by_lifecycle_transition() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-retry-outcome");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    store.save_task_board_workflow(
        &project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "policy_retry_flow".to_string(),
            label: "Policy Retry Flow".to_string(),
            statuses: vec![
                workflow_status("initial_queue", TaskPoolWorkflowStatusRoleDto::Queued),
                workflow_status_with_transitions(
                    "custom_failed",
                    TaskPoolWorkflowStatusRoleDto::Failed,
                    vec![workflow_transition("retry", "retry_queue")],
                ),
                workflow_status("retry_queue", TaskPoolWorkflowStatusRoleDto::Queued),
            ],
        },
    )?;
    let task_id = TaskId::new("task-policy-retry-outcome")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.workflow_id": "policy_retry_flow",
                "task_board.status_key": "custom_failed",
                "task_board.status_role": "failed",
                "last_error": "transient failure",
                "retry_count": 1
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert_eq!(outcome.retry_task_ids, vec![task_id.clone()]);
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(task.metadata["task_board.status_key"], "retry_queue");
    assert_eq!(task.metadata["task_board.status_role"], "queued");
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.from_status_key.as_deref() == Some("custom_failed")
            && log.to_status_key.as_deref() == Some("retry_queue")
    }));
    Ok(())
}

#[test]
fn agent_scheduler_policy_times_out_running_task_by_lifecycle_transition() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-timeout-outcome");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    store.save_task_board_workflow(
        &project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "policy_timeout_flow".to_string(),
            label: "Policy Timeout Flow".to_string(),
            statuses: vec![
                workflow_status("generic_failed", TaskPoolWorkflowStatusRoleDto::Failed),
                workflow_status_with_transitions(
                    "custom_running",
                    TaskPoolWorkflowStatusRoleDto::Executing,
                    vec![workflow_transition("failure", "timed_out_failed")],
                ),
                workflow_status("timed_out_failed", TaskPoolWorkflowStatusRoleDto::Failed),
            ],
        },
    )?;
    let task_id = TaskId::new("task-policy-timeout-outcome")?;
    let run_id = super::domain::RunId::new("run-policy-timeout-outcome")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Running,
            json!({
                "task_board.workflow_id": "policy_timeout_flow",
                "task_board.status_key": "custom_running",
                "task_board.status_role": "executing"
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new("policy-worker")?,
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(10_000)),
            },
        )?
        .expect("running task run claimed");
    let service = AgentSchedulerPolicyService::new(&store);
    let config = AgentSchedulerPolicyConfig {
        running_timeout_ms: 1,
        ..AgentSchedulerPolicyConfig::default()
    };

    let first = service.apply_project_policy(&project_path, TimestampMs(2_000), &config)?;
    assert_eq!(first.timeout_grace_task_ids, vec![task_id.clone()]);
    let second = service.apply_project_policy(&project_path, TimestampMs(3_000), &config)?;

    assert_eq!(second.timed_out_task_ids, vec![task_id.clone()]);
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Failed);
    assert_eq!(task.metadata["task_board.status_key"], "timed_out_failed");
    assert_eq!(task.metadata["task_board.status_role"], "failed");
    let run = store.load_run(&project_path, &run_id)?.expect("timeout run");
    assert_eq!(run.status, AgentRunStatus::TimedOut);
    Ok(())
}

#[test]
fn agent_scheduler_policy_disables_failed_task_retry_when_max_attempts_is_zero() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-retry-disabled");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-policy-retry-disabled")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed",
                "task_board.status_role": "failed",
                "last_error": "transient failure"
            }),
            TimestampMs(1_000),
        )?,
    )?;
    let config = AgentSchedulerPolicyConfig {
        failed_retry_max_attempts: 0,
        ..AgentSchedulerPolicyConfig::default()
    };

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(100_000),
        &config,
    )?;

    assert!(outcome.retry_task_ids.is_empty());
    assert_eq!(
        store.load_task(&project_path, &task_id)?.expect("failed task remains").status,
        AgentTaskStatus::Failed
    );
    assert!(store.list_task_logs(&project_path, &task_id)?.is_empty());
    Ok(())
}

#[test]
fn agent_scheduler_policy_skips_deleted_and_archived_tasks() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-skips-hidden")?;
    let deleted_id = TaskId::new("task-policy-deleted")?;
    let archived_id = TaskId::new("task-policy-archived")?;
    store.create_task(
        &project_path,
        finite_task(
            &deleted_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed",
                "task_board.status_role": "failed",
                "task_board.deleted": true,
                "last_error": "transient failure"
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store.create_task(
        &project_path,
        finite_task(
            &archived_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "code_complete",
                "task_board.status_role": "review_ready",
                "task_board.archived": true
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(100_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert_eq!(outcome, AgentSchedulerPolicyOutcome::default());
    assert!(store.list_task_logs(&project_path, &deleted_id)?.is_empty());
    assert!(store.list_task_logs(&project_path, &archived_id)?.is_empty());
    Ok(())
}

#[test]
fn agent_scheduler_policy_keeps_failed_tasks_when_retry_limit_or_backoff_blocks() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-retry-boundaries")?;
    let exhausted_id = TaskId::new("task-policy-retry-exhausted")?;
    let waiting_id = TaskId::new("task-policy-retry-waiting")?;
    store.create_task(
        &project_path,
        finite_task(
            &exhausted_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed",
                "task_board.status_role": "failed",
                "last_error": "transient failure",
                "retry_count": 1
            }),
            TimestampMs(1_000),
        )?,
    )?;
    store.create_task(
        &project_path,
        finite_task(
            &waiting_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed",
                "task_board.status_role": "failed",
                "last_error": "gateway stream idle timeout",
                "last_active_at_ms": 90_000,
                "retry_count": 1
            }),
            TimestampMs(1_000),
        )?,
    )?;
    let config = AgentSchedulerPolicyConfig {
        failed_retry_minutes: 20,
        failed_retry_max_attempts: 1,
        ..AgentSchedulerPolicyConfig::default()
    };

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(100_000),
        &config,
    )?;

    assert!(outcome.retry_task_ids.is_empty());
    assert_eq!(
        store.load_task(&project_path, &exhausted_id)?.expect("exhausted task").status,
        AgentTaskStatus::Failed
    );
    assert_eq!(
        store.load_task(&project_path, &waiting_id)?.expect("waiting task").status,
        AgentTaskStatus::Failed
    );
    Ok(())
}

#[test]
fn agent_scheduler_policy_retries_failed_task_without_error_message() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-retry-without-error")?;
    store.save_task_board_workflow(
        &project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "policy_retry_without_error".to_string(),
            label: "Policy Retry Without Error".to_string(),
            statuses: vec![
                workflow_status_with_transitions(
                    "failed_without_error",
                    TaskPoolWorkflowStatusRoleDto::Failed,
                    vec![workflow_transition("retry", "queued_after_errorless_retry")],
                ),
                workflow_status(
                    "queued_after_errorless_retry",
                    TaskPoolWorkflowStatusRoleDto::Queued,
                ),
            ],
        },
    )?;
    let task_id = TaskId::new("task-policy-retry-without-error")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.workflow_id": "policy_retry_without_error",
                "task_board.status_key": "failed_without_error",
                "task_board.status_role": "failed"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert_eq!(outcome.retry_task_ids, vec![task_id.clone()]);
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.message == "agent scheduler policy queued failed task for retry after attempt 1"
    }));
    Ok(())
}

#[test]
fn agent_scheduler_policy_logs_skip_when_lifecycle_workflow_is_missing() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-missing-workflow")?;
    let task_id = TaskId::new("task-policy-missing-workflow")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Failed,
            json!({
                "task_board.status_key": "failed_without_workflow",
                "task_board.status_role": "failed",
                "last_error": "transient failure"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.retry_task_ids.is_empty());
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.from_status_key.as_deref() == Some("failed_without_workflow")
            && log.message.contains("task board workflow is required")
    }));
    Ok(())
}

#[test]
fn agent_scheduler_policy_ignores_running_task_before_timeout_window() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-timeout-window")?;
    for (suffix, status) in [
        ("assigned", AgentTaskStatus::Assigned),
        ("dispatched", AgentTaskStatus::Dispatched),
        ("acknowledged", AgentTaskStatus::Acknowledged),
        ("running", AgentTaskStatus::Running),
    ] {
        let task_id = TaskId::new(format!("task-policy-timeout-{suffix}"))?;
        store.create_task(
            &project_path,
            finite_task(
                &task_id,
                status,
                json!({
                    "task_board.status_key": status.as_str(),
                    "task_board.status_role": "executing"
                }),
                TimestampMs(99_500),
            )?,
        )?;
    }
    let config = AgentSchedulerPolicyConfig {
        running_timeout_ms: 1_000,
        ..AgentSchedulerPolicyConfig::default()
    };

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(100_000),
        &config,
    )?;

    assert!(outcome.timeout_grace_task_ids.is_empty());
    assert!(outcome.timed_out_task_ids.is_empty());
    Ok(())
}

#[test]
fn agent_scheduler_policy_ignores_submitted_task_outside_review_or_merge_roles() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-submitted-irrelevant")?;
    let task_id = TaskId::new("task-policy-submitted-irrelevant")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.status_key": "manual_hold",
                "task_board.status_role": "paused"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.merge_task_ids.is_empty());
    assert!(outcome.blocked_task_ids.is_empty());
    assert!(store.list_task_logs(&project_path, &task_id)?.is_empty());
    Ok(())
}

#[test]
fn agent_scheduler_policy_completes_non_development_merge_stage_without_git_merge() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-marketing-merge")?;
    store.save_task_board_workflow(
        &project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "marketing_campaign".to_string(),
            label: "Marketing Campaign".to_string(),
            statuses: vec![
                workflow_status_with_transitions(
                    "campaign_closing",
                    TaskPoolWorkflowStatusRoleDto::Merging,
                    vec![workflow_transition("success", "campaign_done")],
                ),
                workflow_status("campaign_done", TaskPoolWorkflowStatusRoleDto::Done),
            ],
        },
    )?;
    let task_id = TaskId::new("task-policy-marketing-merge")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.workflow_id": "marketing_campaign",
                "task_board.status_key": "campaign_closing",
                "task_board.status_role": "merging"
            }),
            TimestampMs(1_000),
        )?,
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.merge_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    assert_eq!(task.metadata["task_board.status_key"], "campaign_done");
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.message.contains("completed non-development merge stage without git merge")
    }));
    Ok(())
}

#[test]
fn agent_scheduler_policy_requests_merge_before_stall_timeout() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-merge-request")?;
    store.save_task_board_workflow(&project_path, &merge_policy_workflow("merge_blocked"))?;
    let task_id = TaskId::new("task-policy-merge-request")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.workflow_id": DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID,
                "task_board.status_key": "merging",
                "task_board.status_role": "merging",
                "last_active_at_ms": 1_900,
                "merge_target_branch": "main",
                "selected_worktree_path": project_path.to_string_lossy()
            }),
            TimestampMs(1_000),
        )?,
    )?;
    let config = AgentSchedulerPolicyConfig {
        merge_stall_timeout_ms: 1_000,
        ..AgentSchedulerPolicyConfig::default()
    };

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &config,
    )?;

    assert_eq!(outcome.merge_task_ids, vec![task_id.clone()]);
    assert!(outcome.blocked_task_ids.is_empty());
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(
        logs.iter().any(|log| {
            log.message == "agent scheduler policy requested merge for submitted task"
        })
    );
    Ok(())
}

#[test]
fn agent_scheduler_policy_blocks_stalled_merge_with_unknown_merge_metadata() -> Result<()> {
    let (_temp_dir, project_path, store) =
        initialized_policy_store("project-agent-scheduler-policy-merge-stall")?;
    store.save_task_board_workflow(&project_path, &merge_policy_workflow("merge_failed"))?;
    let task_id = TaskId::new("task-policy-merge-stall")?;
    store.create_task(
        &project_path,
        finite_task(
            &task_id,
            AgentTaskStatus::Submitted,
            json!({
                "task_board.workflow_id": DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID,
                "task_board.status_key": "merging",
                "task_board.status_role": "merging",
                "last_active_at_ms": 1_000
            }),
            TimestampMs(1_000),
        )?,
    )?;
    let config = AgentSchedulerPolicyConfig {
        merge_stall_timeout_ms: 1,
        ..AgentSchedulerPolicyConfig::default()
    };

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &config,
    )?;

    assert_eq!(outcome.blocked_task_ids, vec![task_id.clone()]);
    assert!(outcome.merge_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Failed);
    assert_eq!(task.metadata["task_board.status_key"], "merge_failed");
    assert_eq!(task.metadata["last_active_at_ms"], 2_000);
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(logs.iter().any(|log| {
        log.message.contains("merge stall detected")
            && log.message.contains("worktree=unknown")
            && log.message.contains("target=unknown")
    }));
    Ok(())
}

fn initialized_policy_store(
    name: &str,
) -> Result<(tempfile::TempDir, std::path::PathBuf, AgentSchedulerStore)> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join(name);
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    Ok((temp_dir, project_path, store))
}

fn finite_task(
    task_id: &TaskId,
    status: AgentTaskStatus,
    metadata: Value,
    created_at_ms: TimestampMs,
) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: format!("Policy task {}", task_id.as_str()),
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata,
        created_at_ms,
    })
}

fn marketing_workflow() -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: "marketing_campaign".to_string(),
        label: "营销活动执行".to_string(),
        statuses: vec![
            workflow_status_with_transitions(
                "campaign_review_ready",
                TaskPoolWorkflowStatusRoleDto::ReviewReady,
                vec![workflow_transition("success", "campaign_done")],
            ),
            workflow_status("campaign_closing", TaskPoolWorkflowStatusRoleDto::Merging),
            workflow_status("campaign_done", TaskPoolWorkflowStatusRoleDto::Done),
        ],
    }
}

fn merge_policy_workflow(blocked_target: &str) -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID.to_string(),
        label: "Development Merge Policy".to_string(),
        statuses: vec![
            workflow_status_with_transitions(
                "merging",
                TaskPoolWorkflowStatusRoleDto::Merging,
                vec![workflow_transition("blocked", blocked_target)],
            ),
            workflow_status("merge_blocked", TaskPoolWorkflowStatusRoleDto::Merging),
            workflow_status("merge_failed", TaskPoolWorkflowStatusRoleDto::Failed),
        ],
    }
}

fn workflow_status(key: &str, role: TaskPoolWorkflowStatusRoleDto) -> TaskPoolWorkflowStatusDto {
    workflow_status_with_transitions(key, role, Vec::new())
}

fn workflow_status_with_transitions(
    key: &str,
    role: TaskPoolWorkflowStatusRoleDto,
    transitions: Vec<TaskPoolWorkflowTransitionDto>,
) -> TaskPoolWorkflowStatusDto {
    TaskPoolWorkflowStatusDto {
        key: key.to_string(),
        label: key.to_string(),
        role,
        enabled: true,
        terminal: false,
        next: None,
        transitions,
    }
}

fn workflow_transition(outcome: &str, target: &str) -> TaskPoolWorkflowTransitionDto {
    TaskPoolWorkflowTransitionDto {
        outcome: outcome.to_string(),
        label: None,
        target: target.to_string(),
    }
}
