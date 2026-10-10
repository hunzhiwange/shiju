//! 测试运行存储在多后端下的原子领取、租约与终止状态。
//! 公共断言覆盖竞争领取、依赖阻塞、陈旧租约和各类终止结果。

use std::sync::{Arc, Barrier};

use anyhow::Result;
use rusqlite::params;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{AgentRunStatus, AgentSchedulerContext, AssigneeId, RunId, TaskId};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::runs::{AgentRunFinishStatus, ClaimReadyTaskRunRequest, ClaimTaskRunRequest};
use super::store::tasks::NewAgentTask;
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use super::task_graph::AgentTaskDependency;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_run_store_rejects_invalid_lease_times_and_returns_none_for_missing_runs() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let missing_run_id = RunId::new("missing-run")?;
    let missing_task_id = TaskId::new("missing-task")?;

    let claim_error = store
        .claim_task_run(
            project_path,
            ClaimTaskRunRequest {
                run_id: RunId::new("invalid-lease-run")?,
                task_id: missing_task_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(999)),
            },
        )
        .expect_err("lease expiry before acquisition must be rejected");
    assert!(
        claim_error
            .to_string()
            .contains("run lease expiry must not be before lease acquisition time")
    );

    let heartbeat_error = store
        .heartbeat_run(project_path, &missing_run_id, TimestampMs(1_000), TimestampMs(999))
        .expect_err("lease expiry before heartbeat must be rejected");
    assert!(
        heartbeat_error.to_string().contains("run lease expiry must not be before heartbeat time")
    );

    assert!(store.load_run(project_path, &missing_run_id)?.is_none());
    assert!(store.load_latest_run_for_task(project_path, &missing_task_id)?.is_none());
    assert!(
        store
            .claim_task_run(
                project_path,
                claim_request("missing-task-run", &missing_task_id, "worker-a"),
            )?
            .is_none()
    );
    assert!(
        store
            .heartbeat_run(project_path, &missing_run_id, TimestampMs(1_000), TimestampMs(1_001),)?
            .is_none()
    );
    assert!(
        store
            .finish_run(
                project_path,
                &missing_run_id,
                AgentRunFinishStatus::Failed,
                TimestampMs(1_002),
                Some("missing run".to_string()),
            )?
            .is_none()
    );

    Ok(())
}

#[test]
fn agent_run_store_claims_task_once_and_lists_stale_leases() -> Result<()> {
    for_each_run_store_provider(assert_run_store_claims_task_once_and_lists_stale_leases)
}

#[test]
fn claim_ready_task_run_assigns_and_inserts_atomically() -> Result<()> {
    for_each_run_store_provider(assert_claim_ready_task_run_assigns_and_inserts_atomically)
}

fn assert_claim_ready_task_run_assigns_and_inserts_atomically(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    let predecessor_id = TaskId::new("ready-claim-predecessor")?;
    let task_id = TaskId::new("ready-claim-successor")?;
    store.initialize_project(context)?;
    store.create_task(
        context,
        finite_task_with_status(&predecessor_id, AgentTaskStatus::Completed)?,
    )?;
    store.create_task(context, finite_task_with_status(&task_id, AgentTaskStatus::Queued)?)?;
    store.replace_task_dependencies(
        context,
        vec![AgentTaskDependency::new(predecessor_id, task_id.clone())],
    )?;

    let claimed = store
        .claim_ready_task_run(
            context,
            ready_claim_request("ready-claim-run", &task_id, "worker-a"),
        )?
        .expect("completed dependencies allow the queued task to be claimed");
    assert_eq!(claimed.id.as_str(), "ready-claim-run");
    assert_eq!(claimed.status, AgentRunStatus::Leased);
    assert_eq!(
        store.load_task(context, &task_id)?.expect("claimed task exists").status,
        AgentTaskStatus::Assigned
    );

    assert!(
        store
            .claim_ready_task_run(
                context,
                ready_claim_request("ready-claim-run-duplicate", &task_id, "worker-b"),
            )?
            .is_none()
    );
    assert_eq!(
        store.load_latest_run_for_task(context, &task_id)?.map(|run| run.id),
        Some(RunId::new("ready-claim-run")?)
    );
    Ok(())
}

#[test]
fn claim_ready_task_run_rejects_unsatisfied_dependencies_without_state_change() -> Result<()> {
    for_each_run_store_provider(assert_claim_ready_task_run_rejects_unsatisfied_dependencies)
}

fn assert_claim_ready_task_run_rejects_unsatisfied_dependencies(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;
    for (suffix, predecessor_status) in [
        ("waiting", AgentTaskStatus::Queued),
        ("submitted", AgentTaskStatus::Submitted),
        ("failed", AgentTaskStatus::Failed),
        ("cancelled", AgentTaskStatus::Cancelled),
        ("archived", AgentTaskStatus::Archived),
    ] {
        let predecessor_id = TaskId::new(format!("unsatisfied-predecessor-{suffix}"))?;
        let task_id = TaskId::new(format!("unsatisfied-successor-{suffix}"))?;
        store
            .create_task(context, finite_task_with_status(&predecessor_id, predecessor_status)?)?;
        store.create_task(context, finite_task_with_status(&task_id, AgentTaskStatus::Queued)?)?;
        store.replace_task_dependencies(
            context,
            vec![AgentTaskDependency::new(predecessor_id, task_id.clone())],
        )?;

        assert!(
            store
                .claim_ready_task_run(
                    context,
                    ready_claim_request(&format!("unsatisfied-run-{suffix}"), &task_id, "worker-a"),
                )?
                .is_none(),
            "{suffix} predecessor must prevent claiming"
        );
        assert_eq!(
            store.load_task(context, &task_id)?.expect("queued task exists").status,
            AgentTaskStatus::Queued
        );
        assert!(store.load_latest_run_for_task(context, &task_id)?.is_none());
        store.replace_task_dependencies(context, Vec::new())?;
    }
    Ok(())
}

#[test]
fn claim_ready_task_run_allows_one_competing_claimer() -> Result<()> {
    for_each_run_store_provider(assert_claim_ready_task_run_allows_one_competing_claimer)
}

fn assert_claim_ready_task_run_allows_one_competing_claimer(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    let task_id = TaskId::new("ready-claim-concurrent-task")?;
    store.initialize_project(context)?;
    store.create_task(context, finite_task_with_status(&task_id, AgentTaskStatus::Queued)?)?;

    let barrier = Arc::new(Barrier::new(3));
    let claims = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_task_id = task_id.clone();
        let first = scope.spawn(move || {
            first_barrier.wait();
            store.claim_ready_task_run(
                context,
                ready_claim_request("ready-claim-concurrent-1", &first_task_id, "worker-a"),
            )
        });
        let second_barrier = Arc::clone(&barrier);
        let second_task_id = task_id.clone();
        let second = scope.spawn(move || {
            second_barrier.wait();
            store.claim_ready_task_run(
                context,
                ready_claim_request("ready-claim-concurrent-2", &second_task_id, "worker-b"),
            )
        });
        barrier.wait();
        Ok::<_, anyhow::Error>([
            first.join().expect("first ready claim thread")?,
            second.join().expect("second ready claim thread")?,
        ])
    })?;

    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    assert_eq!(
        store.load_task(context, &task_id)?.expect("concurrent task exists").status,
        AgentTaskStatus::Assigned
    );
    Ok(())
}

fn assert_run_store_claims_task_once_and_lists_stale_leases(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    let task_id = TaskId::new("task-run-1")?;

    store.initialize_project(context)?;
    store.create_task(context, finite_task(&task_id)?)?;

    let first_claim = store
        .claim_task_run(
            context,
            ClaimTaskRunRequest {
                run_id: RunId::new("run-1")?,
                task_id: task_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(1_500)),
            },
        )?
        .expect("first claim succeeds");
    assert_eq!(first_claim.id.as_str(), "run-1");
    assert_eq!(first_claim.task_id, task_id);
    assert_eq!(first_claim.status, AgentRunStatus::Leased);
    assert_eq!(first_claim.attempt, 1);
    assert_eq!(first_claim.lease_acquired_at_ms, Some(TimestampMs(1_000)));
    assert_eq!(first_claim.lease_expires_at_ms, Some(TimestampMs(1_500)));

    let duplicate_claim = store.claim_task_run(
        context,
        ClaimTaskRunRequest {
            run_id: RunId::new("run-2")?,
            task_id: task_id.clone(),
            lease_acquired_at_ms: TimestampMs(1_100),
            lease_expires_at_ms: Some(TimestampMs(1_600)),
        },
    )?;
    assert!(duplicate_claim.is_none());

    assert!(store.list_stale_runs(context, TimestampMs(1_499))?.is_empty());
    let stale = store.list_stale_runs(context, TimestampMs(1_500))?;
    assert_eq!(run_ids(&stale), vec!["run-1"]);

    let heartbeat = store
        .heartbeat_run(context, &RunId::new("run-1")?, TimestampMs(1_600), TimestampMs(2_400))?
        .expect("heartbeat refreshes active run");
    assert_eq!(heartbeat.status, AgentRunStatus::Running);
    assert_eq!(heartbeat.lease_expires_at_ms, Some(TimestampMs(2_400)));
    assert_eq!(heartbeat.last_heartbeat_at_ms, Some(TimestampMs(1_600)));
    assert!(store.list_stale_runs(context, TimestampMs(2_399))?.is_empty());
    assert_eq!(run_ids(&store.list_stale_runs(context, TimestampMs(2_400))?), vec!["run-1"]);

    let finished = store
        .finish_run(
            context,
            &RunId::new("run-1")?,
            AgentRunFinishStatus::Succeeded,
            TimestampMs(2_500),
            None,
        )?
        .expect("active run finishes");
    assert_eq!(finished.status, AgentRunStatus::Succeeded);
    assert_eq!(finished.finished_at_ms, Some(TimestampMs(2_500)));
    assert!(store.list_stale_runs(context, TimestampMs(3_000))?.is_empty());

    Ok(())
}

#[test]
fn run_lease_has_no_worker_identity() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-run-without-worker")?;
    store.initialize_project(temp_dir.path())?;
    store.create_task(temp_dir.path(), finite_task(&task_id)?)?;

    let run = store
        .claim_task_run(
            temp_dir.path(),
            ClaimTaskRunRequest {
                run_id: RunId::new("run-without-worker")?,
                task_id,
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(1_500)),
            },
        )?
        .expect("run claimed");

    assert_eq!(run.task_id.as_str(), "task-run-without-worker");
    Ok(())
}

#[test]
fn agent_run_store_postgres_claims_task_once_under_concurrency() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let project_path = tempfile::tempdir()?;
    let database = fixture.database().clone();
    let (store, context) = scheduler_store_fixture(database.clone(), project_path.path())?;
    let competing_store = AgentSchedulerStore::from_database(database);
    let task_id = TaskId::new("task-concurrent-claim")?;
    store.initialize_project(&context)?;
    store.create_task(&context, finite_task(&task_id)?)?;

    let barrier = Arc::new(Barrier::new(3));
    let claims = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_store = &store;
        let first_context = &context;
        let first_task_id = task_id.clone();
        let first = scope.spawn(move || {
            first_barrier.wait();
            first_store.claim_task_run(
                first_context,
                claim_request("concurrent-run-1", &first_task_id, "worker-a"),
            )
        });
        let second_barrier = Arc::clone(&barrier);
        let second_store = &competing_store;
        let second_context = &context;
        let second_task_id = task_id.clone();
        let second = scope.spawn(move || {
            second_barrier.wait();
            second_store.claim_task_run(
                second_context,
                claim_request("concurrent-run-2", &second_task_id, "worker-b"),
            )
        });
        barrier.wait();
        Ok::<_, anyhow::Error>([
            first.join().expect("first claim thread")?,
            second.join().expect("second claim thread")?,
        ])
    })?;

    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    fixture.cleanup()
}

#[test]
fn agent_run_store_claims_each_dispatchable_task_status() -> Result<()> {
    for_each_run_store_provider(assert_run_store_claims_each_dispatchable_task_status)
}

fn assert_run_store_claims_each_dispatchable_task_status(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;

    let cases = [
        ("assigned", AgentTaskStatus::Assigned),
        ("dispatched", AgentTaskStatus::Dispatched),
        ("acknowledged", AgentTaskStatus::Acknowledged),
        ("active", AgentTaskStatus::Active),
        ("running", AgentTaskStatus::Running),
    ];
    for (suffix, task_status) in cases {
        let task_id = TaskId::new(format!("task-claim-{suffix}"))?;
        let run_id = RunId::new(format!("run-claim-{suffix}"))?;
        store.create_task(context, finite_task_with_status(&task_id, task_status)?)?;

        let claimed = store
            .claim_task_run(
                context,
                ClaimTaskRunRequest {
                    run_id: run_id.clone(),
                    task_id: task_id.clone(),
                    lease_acquired_at_ms: TimestampMs(1_000),
                    lease_expires_at_ms: None,
                },
            )?
            .expect("dispatchable task status claims a run");

        assert_eq!(claimed.id, run_id);
        assert_eq!(claimed.task_id, task_id);
        assert_eq!(claimed.status, AgentRunStatus::Leased);
        assert_eq!(claimed.lease_expires_at_ms, None);
    }

    Ok(())
}

#[test]
fn agent_run_store_finishes_runs_as_timed_out_and_cancelled() -> Result<()> {
    for_each_run_store_provider(assert_run_store_finishes_runs_as_timed_out_and_cancelled)
}

fn assert_run_store_finishes_runs_as_timed_out_and_cancelled(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;

    let cases = [
        ("timed-out", AgentRunFinishStatus::TimedOut, AgentRunStatus::TimedOut),
        ("cancelled", AgentRunFinishStatus::Cancelled, AgentRunStatus::Cancelled),
    ];
    for (suffix, finish_status, expected_status) in cases {
        let task_id = TaskId::new(format!("task-finish-{suffix}"))?;
        let run_id = RunId::new(format!("run-finish-{suffix}"))?;
        store.create_task(context, finite_task(&task_id)?)?;
        store
            .claim_task_run(context, claim_request(run_id.as_str(), &task_id, "worker-finish"))?
            .expect("run claims before finish");

        let finished = store
            .finish_run(
                context,
                &run_id,
                finish_status,
                TimestampMs(3_000),
                Some(format!("{suffix} by policy")),
            )?
            .expect("active run finishes");

        assert_eq!(finished.status, expected_status);
        assert_eq!(finished.error_message.as_deref(), Some(format!("{suffix} by policy").as_str()));
        assert!(
            store
                .heartbeat_run(context, &run_id, TimestampMs(3_100), TimestampMs(3_200))?
                .is_none()
        );
        assert_eq!(
            store.load_latest_run_for_task(context, &task_id)?.map(|run| run.status),
            Some(expected_status)
        );
    }

    Ok(())
}

#[test]
fn agent_run_store_loads_pending_and_stale_rows_from_sqlite() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (store, context) = scheduler_store_fixture(sqlite_database, &sqlite_project)?;
    store.initialize_project(&context)?;

    let cases = [("pending", AgentRunStatus::Pending), ("stale", AgentRunStatus::Stale)];
    let scope = context.storage_scope()?;
    let project = scope.project_scope();
    let platform_id = project.platform_id().as_ref();
    let company_id = project.company_id().as_ref();
    let project_id = project.project_id().as_ref();
    let conn = rusqlite::Connection::open(store.database_path())?;

    for (status_key, expected_status) in cases {
        let task_id = TaskId::new(format!("task-{status_key}"))?;
        let run_id = RunId::new(format!("run-{status_key}"))?;
        store.create_task(&context, finite_task(&task_id)?)?;
        conn.execute(
            "INSERT INTO scheduler_runs (
                 platform_id, company_id, project_id, id, task_id, worker_id, status, attempt
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
            params![
                platform_id,
                company_id,
                project_id,
                run_id.as_str(),
                task_id.as_str(),
                status_key,
                1_i64,
            ],
        )?;

        let loaded = store.load_run(&context, &run_id)?.expect("run row loads");
        assert_eq!(loaded.status, expected_status);
    }

    let invalid_task_id = TaskId::new("task-invalid-run-status")?;
    let invalid_run_id = RunId::new("run-invalid-status")?;
    store.create_task(&context, finite_task(&invalid_task_id)?)?;
    conn.execute(
        "INSERT INTO scheduler_runs (
             platform_id, company_id, project_id, id, task_id, worker_id, status, attempt
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
        params![
            platform_id,
            company_id,
            project_id,
            invalid_run_id.as_str(),
            invalid_task_id.as_str(),
            "teleported",
            1_i64,
        ],
    )?;
    let error =
        store.load_run(&context, &invalid_run_id).expect_err("invalid run status row is rejected");
    let error_chain = format!("{error:?}");
    assert!(error_chain.contains("unsupported agent run status: teleported"), "{error_chain}");

    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    finite_task_with_status(task_id, AgentTaskStatus::Dispatched)
}

fn finite_task_with_status(task_id: &TaskId, status: AgentTaskStatus) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Run lease storage".to_string(),
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: Some(AssigneeId::new("worker-a")?),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "run-store-test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn run_ids(runs: &[super::store::runs::StoredAgentRun]) -> Vec<&str> {
    runs.iter().map(|run| run.id.as_str()).collect()
}

fn claim_request(run_id: &str, task_id: &TaskId, _worker_id: &str) -> ClaimTaskRunRequest {
    ClaimTaskRunRequest {
        run_id: RunId::new(run_id).expect("valid run id"),
        task_id: task_id.clone(),
        lease_acquired_at_ms: TimestampMs(1_000),
        lease_expires_at_ms: Some(TimestampMs(2_000)),
    }
}

fn ready_claim_request(
    run_id: &str,
    task_id: &TaskId,
    _worker_id: &str,
) -> ClaimReadyTaskRunRequest {
    ClaimReadyTaskRunRequest {
        run_id: RunId::new(run_id).expect("valid run id"),
        task_id: task_id.clone(),
        lease_acquired_at_ms: TimestampMs(1_000),
        lease_expires_at_ms: Some(TimestampMs(2_000)),
    }
}

// 公共运行记录断言在各后端复用，原子竞争场景保持后端专用实现。
fn for_each_run_store_provider(
    mut check: impl FnMut(&AgentSchedulerStore, &AgentSchedulerContext) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (sqlite_store, sqlite_context) = scheduler_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context) =
        scheduler_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context)?;
    postgres_fixture.cleanup()
}

fn scheduler_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext)> {
    let tenant = TenantScope::default();
    ProjectStore::new(database.clone())?.create(
        &tenant,
        &ProjectInfo {
            id: "run-store-project".to_owned(),
            worktree: project_path.to_string_lossy().into_owned(),
            vcs: None,
            name: Some("Run store project".to_owned()),
            icon: None,
            commands: None,
            time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::for_default_tenant(ProjectId::from("run-store-project"))?,
        project_path,
    )?;
    Ok((AgentSchedulerStore::from_database(database), context))
}
