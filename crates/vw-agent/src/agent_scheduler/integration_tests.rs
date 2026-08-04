use std::fs;
use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::task::TaskPoolWorkflowSnapshotDto;
use vw_api_types::task_lifecycle::TaskLifecycleWorkflowUpsertBody;
use vw_shared::project::{Info, TimeInfo};
use vw_shared::session::info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionStep, SessionTodoItem, TokenUsage,
};

use super::adapters::controller::{
    FakeControllerAdapter, WorkerControllerSnapshot, WorkerLifecycleState,
};
use super::adapters::matrix::FakeMatrixDispatchAdapter;
use super::artifacts::TaskArtifactWriter;
use super::domain::{
    AgentRunStatus, AgentSchedulerContext, AgentTeamStatus, AgentWorkerStatus, HolderId,
    ResourceId, ResourceLeaseId, RunId, TaskId, TeamId, WorkerName,
};
use super::engine::dispatch::DispatchEngine;
use super::events::AgentEventKind;
use super::outbox_dispatcher::{OutboxDispatcher, OutboxDrainSummary};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::events::AppendAgentEventRequest;
use super::store::logs::{AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest};
use super::store::outbox::EnqueueOutboxMessageRequest;
use super::store::resources::{AcquireResourceLeaseRequest, ResourceKind};
use super::store::runs::AgentRunFinishStatus;
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::NewAgentTask;
use super::store::teams::NewAgentTeamSnapshot;
use super::store::workers::{
    AgentCapabilityRecord, AgentWorkerFilter, NewAgentWorkerSnapshot,
    UpdateAgentWorkerLifecycleRequest,
};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;
use crate::session::store::SessionStore;

const GLOBAL_SCOPED_TABLES: [&str; 18] = [
    "projects",
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_events",
    "scheduler_outbox_messages",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "session_index",
    "session_messages",
    "session_calls",
    "session_steps",
    "session_todos",
    "session_agent_sessions",
];
const GLOBAL_APPLICATION_TABLES: [&str; 22] = [
    "platform",
    "company",
    "projects",
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_events",
    "scheduler_outbox_messages",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "session_index",
    "session_messages",
    "session_calls",
    "session_steps",
    "session_todos",
    "session_agent_sessions",
    "workflow_lifecycle_workflows",
    "workflow_workflows",
];

#[test]
fn agent_scheduler_requires_same_scope_parent_project() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    assert_scheduler_requires_same_scope_parent_project(sqlite_database)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    assert_scheduler_requires_same_scope_parent_project(postgres_fixture.database().clone())?;
    postgres_fixture.cleanup()?;
    Ok(())
}

fn assert_scheduler_requires_same_scope_parent_project(database: GlobalDatabase) -> Result<()> {
    let store = AgentSchedulerStore::from_database(database.clone());
    let project_path = Path::new("scheduler-parent-project-test");
    let project_scope = ProjectScope::new(
        TenantScope::new("platform-a", "company-a")?,
        ProjectId::from("shared-project"),
    )?;
    let context =
        AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope.clone()), project_path)?;

    let missing_error = store
        .initialize_project(&context)
        .expect_err("scheduler schema must require an initialized projects table");
    assert!(missing_error.to_string().contains("projects table"));

    let projects = ProjectStore::new(database)?;
    projects.create(
        &TenantScope::new("platform-a", "company-b")?,
        &isolation_project_info("shared-project", project_path, "wrong-tenant"),
    )?;
    let wrong_scope_error = store
        .initialize_project(&context)
        .expect_err("same project id in another tenant must not satisfy the parent check");
    assert!(wrong_scope_error.to_string().contains("parent project does not exist"));

    projects.create(
        project_scope.tenant_scope(),
        &isolation_project_info("shared-project", project_path, "matching-parent"),
    )?;
    store.initialize_project(&context)
}

#[tokio::test]
async fn global_database_enforces_project_and_scheduler_scope_isolation() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let home = temp_dir.path().join("home");
    let database_path = vw_config_types::paths::global_database_path(&home);
    let database = GlobalDatabase::new(&database_path)?;
    let projects = ProjectStore::new(database.clone())?;
    let scheduler = AgentSchedulerStore::from_database_path(&database_path);
    let sessions = SessionStore::new(database.clone())?;
    let fixture_definitions = [
        ("platform-a", "company-a", "shared-project", "tenant-a"),
        ("platform-a", "company-b", "shared-project", "tenant-b"),
        ("platform-b", "company-a", "shared-project", "tenant-c"),
        ("platform-a", "company-a", "second-project", "project-d"),
    ];
    let mut fixtures = Vec::new();

    for (platform_id, company_id, project_id, label) in fixture_definitions {
        let tenant = TenantScope::new(platform_id, company_id)?;
        let project_scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?;
        let project_path = temp_dir.path().join(label);
        fs::create_dir_all(&project_path)?;
        projects.create(&tenant, &isolation_project_info(project_id, &project_path, label))?;
        let context =
            AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope), project_path)?;
        scheduler.initialize_project(&context)?;
        fixtures.push((context, label));
    }

    let lifecycle = crate::task_lifecycle::store::create_workflow(
        database.clone(),
        fixtures[0].0.scope().project_scope().tenant_scope().clone(),
        serde_json::from_value::<TaskLifecycleWorkflowUpsertBody>(json!({
            "workflow": {
                "id": "unified_lifecycle",
                "label": "Unified Lifecycle",
                "statuses": [
                    {
                        "key": "queued",
                        "label": "Queued",
                        "role": "queued",
                        "next": "done"
                    },
                    {
                        "key": "done",
                        "label": "Done",
                        "role": "done",
                        "terminal": true
                    }
                ]
            }
        }))?,
    )
    .await
    .map_err(|error| anyhow::anyhow!(error.message))?;
    let loaded_lifecycle = crate::task_lifecycle::store::workflow_by_id(
        database.clone(),
        fixtures[0].0.scope().project_scope().tenant_scope().clone(),
        lifecycle.id.clone(),
    )
    .await
    .map_err(|error| anyhow::anyhow!(error.message))?;
    let workflow_snapshot = serde_json::from_value::<TaskPoolWorkflowSnapshotDto>(
        serde_json::to_value(&loaded_lifecycle)?,
    )?;
    scheduler.save_task_board_workflow(&fixtures[0].0, &workflow_snapshot)?;
    assert_eq!(scheduler.load_task_board_workflow(&fixtures[0].0)?, Some(workflow_snapshot));

    database.initialize_schema(|connection| {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS workflow_workflows (
                 platform_id TEXT NOT NULL,
                 company_id TEXT NOT NULL,
                 uuid TEXT NOT NULL,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL,
                 workflow_yaml TEXT NOT NULL,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL,
                 PRIMARY KEY (platform_id, company_id, uuid)
             );",
        )?;
        Ok(())
    })?;
    let workflow_uuid = "00000000-0000-4000-8000-000000000301";
    database.connect()?.execute(
        "INSERT INTO workflow_workflows
         (platform_id, company_id, uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
         VALUES ('platform-a', 'company-a', ?1, 'Unified saved workflow', '', 'shared: true', 1, 1)",
        [workflow_uuid],
    )?;
    let hydrated_yaml: String = database.connect()?.query_row(
        "SELECT workflow_yaml FROM workflow_workflows
         WHERE platform_id = 'platform-a' AND company_id = 'company-a' AND uuid = ?1",
        [workflow_uuid],
        |row| row.get(0),
    )?;
    assert_eq!(hydrated_yaml, "shared: true");

    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    assert_eq!(projects.list(&tenant_a)?.len(), 2);
    let mut tenant_b_project =
        projects.get(fixtures[1].0.scope().project_scope())?.expect("tenant B project exists");
    tenant_b_project.name = Some("tenant-b-updated".to_string());
    assert!(projects.update(fixtures[1].0.scope().project_scope(), &tenant_b_project)?);
    assert_eq!(
        projects.get(fixtures[0].0.scope().project_scope())?.and_then(|project| project.name),
        Some("tenant-a".to_string())
    );

    let task_id = TaskId::new("shared-task")?;
    let run_id = RunId::new("shared-run")?;
    let worker_id = WorkerName::new("shared-worker")?;
    let team_id = TeamId::new("shared-team")?;
    let lease_id = ResourceLeaseId::new("shared-lease")?;

    for (context, label) in &fixtures {
        let scope = context.scope().project_scope();
        database.connect()?.execute(
            "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
             VALUES (?1, ?2, ?3, 'isolation.fixture', ?4)",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                label,
            ],
        )?;
        scheduler.create_task(context, isolation_task(label)?)?;
        scheduler.claim_task_run(
            context,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: worker_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(1_500)),
            },
        )?;
        scheduler.append_task_log(
            context,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: Some(run_id.clone()),
                timestamp_ms: TimestampMs(1_010),
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: format!("{label}-log"),
            },
        )?;
        scheduler.record_run_workspace_context(
            context,
            &task_id,
            &run_id,
            AgentSchedulerWorkspaceContext {
                slot_id: format!("{label}-slot"),
                path: context.project_path().to_string_lossy().into_owned(),
                branch: format!("codex/{label}"),
            },
            TimestampMs(1_020),
        )?;
        scheduler.record_task_artifact(
            context,
            super::store::artifacts::RecordTaskArtifactRequest {
                task_id: task_id.clone(),
                run_id: Some(run_id.clone()),
                kind: "report".to_string(),
                name: format!("{label}-artifact"),
                path: None,
                payload: json!({"scope": label}),
                created_at_ms: TimestampMs(1_030),
            },
        )?;
        scheduler.append_event(
            context,
            AppendAgentEventRequest {
                task_id: Some(task_id.clone()),
                run_id: Some(run_id.clone()),
                worker_id: Some(worker_id.clone()),
                team_id: Some(team_id.clone()),
                kind: AgentEventKind::HumanIntervened,
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: format!("{label}-event"),
                payload: json!({"scope": label}),
                created_at_ms: TimestampMs(1_040),
            },
        )?;
        scheduler.enqueue_outbox_message(
            context,
            EnqueueOutboxMessageRequest {
                message_id: "shared-message".to_string(),
                channel: "matrix".to_string(),
                target: "room:shared".to_string(),
                recipient_id: None,
                body: format!("{label}-outbox"),
                idempotency_key: "shared-idempotency".to_string(),
                payload: json!({"scope": label}),
                created_at_ms: TimestampMs(1_050),
                next_attempt_at_ms: None,
            },
        )?;
        scheduler.upsert_worker_snapshot(context, isolation_worker(label)?)?;
        scheduler.upsert_team_snapshot(context, isolation_team(label)?)?;
        scheduler.acquire_resource_lease(
            context,
            AcquireResourceLeaseRequest {
                lease_id: lease_id.clone(),
                resource_kind: ResourceKind::WorkerSlot,
                resource_id: ResourceId::new("shared-resource")?,
                holder_id: HolderId::new("shared-holder")?,
                acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(1_500)),
            },
        )?;
        let project_scope = context.scope().project_scope();
        sessions.save_session(project_scope, &isolation_session(label))?;
        sessions.save_session_todos(
            project_scope,
            "shared-session",
            &[SessionTodoItem {
                id: "shared-todo".to_string(),
                content: format!("{label}-todo"),
                status: "pending".to_string(),
                priority: "medium".to_string(),
            }],
        )?;
        sessions.save_agent_session(
            project_scope,
            &isolation_agent_session(project_scope.project_id().as_ref(), label),
        )?;
    }

    let scope_a = &fixtures[0].0;
    let scope_b = &fixtures[1].0;
    scheduler.transition_task_status(
        scope_a,
        &task_id,
        AgentTaskStatus::Dispatched,
        TimestampMs(1_100),
    )?;
    assert_eq!(
        scheduler.load_task(scope_b, &task_id)?.expect("tenant B task").status,
        AgentTaskStatus::Assigned
    );
    scheduler.heartbeat_run(scope_a, &run_id, TimestampMs(1_200), TimestampMs(5_000))?;
    assert!(scheduler.list_stale_runs(scope_a, TimestampMs(2_000))?.is_empty());
    assert_eq!(scheduler.list_stale_runs(scope_b, TimestampMs(2_000))?.len(), 1);
    scheduler.update_worker_lifecycle(
        scope_a,
        UpdateAgentWorkerLifecycleRequest {
            worker_id: worker_id.clone(),
            status: AgentWorkerStatus::Running,
            container_state: "running".to_string(),
            idle_since_ms: None,
            last_seen_at_ms: TimestampMs(1_200),
            last_heartbeat_at: None,
        },
    )?;
    assert_eq!(
        scheduler.load_worker(scope_b, &worker_id)?.expect("tenant B worker").status,
        AgentWorkerStatus::Idle
    );
    scheduler.release_resource_lease(
        scope_a,
        &lease_id,
        &HolderId::new("shared-holder")?,
        TimestampMs(2_000),
    )?;
    assert!(scheduler.list_stale_resource_leases(scope_a, TimestampMs(2_100))?.is_empty());
    assert_eq!(scheduler.list_stale_resource_leases(scope_b, TimestampMs(2_100))?.len(), 1);

    for (context, label) in &fixtures {
        assert_eq!(
            scheduler.load_task(context, &task_id)?.expect("scoped task").title,
            format!("{label}-task")
        );
        assert_eq!(scheduler.list_task_logs(context, &task_id)?[0].message, format!("{label}-log"));
        assert_eq!(
            scheduler.list_task_artifacts(context, &task_id)?[0].name,
            format!("{label}-artifact")
        );
        assert_eq!(
            scheduler.list_events_after(context, None, 10)?[0].message,
            format!("{label}-event")
        );
        assert_eq!(scheduler.list_workers(context, AgentWorkerFilter::default())?.len(), 1);
        assert_eq!(scheduler.list_teams(context)?[0].name, format!("{label}-team"));
        let claimed = scheduler.claim_outbox_batch(context, TimestampMs(2_200), 10)?;
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].body, format!("{label}-outbox"));
        assert_project_local_databases_absent(context.project_path());
    }

    let connection = database.connect()?;
    assert_eq!(connection.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))?, 1);
    assert_eq!(
        connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })?,
        0
    );
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_schema
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
         ORDER BY name",
    )?;
    let application_tables = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut expected_tables = GLOBAL_APPLICATION_TABLES;
    expected_tables.sort_unstable();
    assert_eq!(application_tables.iter().map(String::as_str).collect::<Vec<_>>(), expected_tables);
    drop(statement);
    for (context, _) in &fixtures {
        for table in GLOBAL_SCOPED_TABLES {
            let scope = context.scope().project_scope();
            let sql = if table == "scheduler_meta" {
                format!(
                    "SELECT COUNT(*) FROM {table}
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND key = 'isolation.fixture'"
                )
            } else {
                format!(
                    "SELECT COUNT(*) FROM {table}
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3"
                )
            };
            let count: i64 = connection.query_row(
                &sql,
                params![
                    scope.platform_id().as_ref(),
                    scope.company_id().as_ref(),
                    scope.project_id().as_ref(),
                ],
                |row| row.get(0),
            )?;
            assert_eq!(count, 1, "unexpected scoped row count for {table}");
        }
        assert!(
            projects.delete(context.scope().project_scope()).is_err(),
            "scheduler rows must restrict project deletion"
        );
    }
    let first_scope = fixtures[0].0.scope().project_scope();
    let second_scope = fixtures[1].0.scope().project_scope();
    assert!(sessions.delete_session(first_scope, "shared-session")?);
    assert!(sessions.load_session(first_scope, "shared-session")?.is_none());
    assert_eq!(
        sessions
            .load_session(second_scope, "shared-session")?
            .expect("second project session remains")
            .title,
        "tenant-b"
    );
    drop(connection);

    assert_eq!(scheduler.database_path(), database_path);
    assert_eq!(
        database_path,
        vw_config_types::paths::home_config_dir(&home).join("database/vibe-window.sqlite3")
    );
    assert!(
        !vw_config_types::paths::home_config_dir(&home)
            .join("agent_scheduler/_index.sqlite3")
            .exists()
    );
    assert!(
        !vw_config_types::paths::home_config_dir(&home)
            .join("task-lifecycle/workflows.sqlite")
            .exists()
    );
    assert!(!home.join("workflow/workflows.sqlite").exists());
    assert!(!vw_config_types::paths::home_config_dir(&home).join("storage/project").exists());
    assert_global_database_permissions(&database_path)?;
    Ok(())
}

fn isolation_session(label: &str) -> ChatSession {
    ChatSession {
        id: "shared-session".to_string(),
        title: label.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: format!("{label}-message"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("shared-message".to_string())],
        calls: vec![json!({"scope": label})],
        steps: vec![ChatSessionStep {
            index: 0,
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
                cached_tokens: 0,
                reasoning_tokens: 0,
            },
            cost_usd: None,
            finish_reason: Some("stop".to_string()),
            model: Some("test-model".to_string()),
        }],
        created_ms: 1,
        updated_ms: 2,
    }
}

fn isolation_agent_session(project_id: &str, label: &str) -> AgentSessionInfo {
    AgentSessionInfo {
        id: "shared-agent-session".to_string(),
        slug: format!("slug-{label}"),
        project_id: project_id.to_string(),
        directory: format!("/work/{label}"),
        parent_id: None,
        summary: None,
        share: None,
        title: format!("{label}-agent"),
        version: "test".to_string(),
        time: AgentSessionTimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

#[tokio::test]
async fn agent_scheduler_e2e_dispatches_and_completes_finite_task() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-e2e");
    let artifact_root = temp_dir.path().join("artifacts");
    fs::create_dir_all(&project_path)?;

    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let matrix = FakeMatrixDispatchAdapter::new();
    let mut clock = TestClock::at(1_783_001_000_000);
    let task_id = TaskId::new("task-e2e-finite")?;
    let worker_id = WorkerName::new("worker-e2e")?;

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;
    store.upsert_worker_snapshot(&project_path, idle_worker(&worker_id)?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: worker_id.clone(),
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-e2e")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-e2e-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller);
    let dispatch = engine
        .dispatch_once(&project_path, clock.advance_ms(1_000))
        .await?
        .expect("queued finite task dispatches");
    assert_eq!(dispatch.task_id, task_id);
    assert_eq!(dispatch.worker_id, worker_id);
    assert_eq!(controller.recorded_lifecycle_requests().len(), 1);

    let dispatched =
        store.load_task(&project_path, &task_id)?.expect("dispatched task remains persisted");
    assert_eq!(dispatched.status, AgentTaskStatus::Dispatched);

    let acknowledged_at_ms = clock.advance_ms(1_000);
    store
        .transition_task_status(
            &project_path,
            &task_id,
            AgentTaskStatus::Acknowledged,
            acknowledged_at_ms,
        )?
        .expect("worker acknowledgment is accepted");
    append_lifecycle_event(
        &store,
        &project_path,
        &task_id,
        &dispatch.run_id,
        &worker_id,
        AgentEventKind::WorkerAcknowledged,
        AgentTaskStatus::Dispatched,
        AgentTaskStatus::Acknowledged,
        "worker acknowledged task dispatch",
        json!({"ack": "received"}),
        acknowledged_at_ms,
    )?;

    let running_at_ms = clock.advance_ms(1_000);
    store
        .transition_task_status(&project_path, &task_id, AgentTaskStatus::Running, running_at_ms)?
        .expect("acknowledged task starts running");
    store
        .heartbeat_run(
            &project_path,
            &dispatch.run_id,
            running_at_ms,
            TimestampMs(running_at_ms.0 + 30 * 60 * 1_000),
        )?
        .expect("leased run accepts worker heartbeat");
    append_lifecycle_event(
        &store,
        &project_path,
        &task_id,
        &dispatch.run_id,
        &worker_id,
        AgentEventKind::RunStarted,
        AgentTaskStatus::Acknowledged,
        AgentTaskStatus::Running,
        "worker started the run",
        json!({"stage": "running"}),
        running_at_ms,
    )?;

    let submitted_at_ms = clock.advance_ms(1_000);
    store
        .transition_task_status(
            &project_path,
            &task_id,
            AgentTaskStatus::Submitted,
            submitted_at_ms,
        )?
        .expect("running task accepts worker submission");
    append_lifecycle_event(
        &store,
        &project_path,
        &task_id,
        &dispatch.run_id,
        &worker_id,
        AgentEventKind::RunSubmitted,
        AgentTaskStatus::Running,
        AgentTaskStatus::Submitted,
        "worker submitted result",
        json!({"result": "E2E lifecycle completed."}),
        submitted_at_ms,
    )?;

    let completed_at_ms = clock.advance_ms(1_000);
    let finished_run = store
        .finish_run(
            &project_path,
            &dispatch.run_id,
            AgentRunFinishStatus::Succeeded,
            completed_at_ms,
            None,
        )?
        .expect("worker run finishes successfully");
    let completed_task = store
        .transition_task_status(
            &project_path,
            &task_id,
            AgentTaskStatus::Completed,
            completed_at_ms,
        )?
        .expect("submitted task completes");
    append_lifecycle_event(
        &store,
        &project_path,
        &task_id,
        &dispatch.run_id,
        &worker_id,
        AgentEventKind::RunCompleted,
        AgentTaskStatus::Submitted,
        AgentTaskStatus::Completed,
        "worker completed finite task",
        json!({"summary": "E2E lifecycle completed."}),
        completed_at_ms,
    )?;

    let dispatcher = OutboxDispatcher::new(&store, &matrix);
    let delivered_at_ms = clock.advance_ms(1_000);
    let drain = dispatcher.drain_once(&project_path, delivered_at_ms, 10).await?;
    assert_eq!(drain, OutboxDrainSummary { claimed: 1, delivered: 1, failed: 0 });
    let matrix_requests = matrix.recorded_requests();
    assert_eq!(matrix_requests.len(), 1);
    assert_eq!(matrix_requests[0].target, "room:!worker-e2e-room:example.test");
    assert_eq!(matrix_requests[0].recipient_id.as_deref(), Some("@worker-e2e:example.test"));
    assert!(matrix_requests[0].body.contains("[task-e2e-finite]"));
    assert_eq!(matrix_requests[0].payload_json["event_version"], 1);

    assert_eq!(completed_task.status, AgentTaskStatus::Completed);
    assert_eq!(finished_run.status, AgentRunStatus::Succeeded);
    assert_eq!(finished_run.finished_at_ms, Some(completed_at_ms));
    assert_run_persisted(&store, &dispatch.run_id, "succeeded", completed_at_ms)?;
    let outbox_message_id =
        dispatch.outbox_message_id.as_deref().expect("Matrix dispatch writes outbox");
    assert_outbox_delivered(&store, outbox_message_id, delivered_at_ms)?;

    let events = store.list_events_after(&project_path, None, 20)?;
    let event_kinds = events.iter().map(|event| event.kind).collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        vec![
            AgentEventKind::WorkerSelected,
            AgentEventKind::MatrixDispatched,
            AgentEventKind::WorkerAcknowledged,
            AgentEventKind::RunStarted,
            AgentEventKind::RunSubmitted,
            AgentEventKind::RunCompleted,
        ]
    );

    assert_eq!(completed_task.id, task_id);
    assert_eq!(completed_task.status, AgentTaskStatus::Completed);
    assert_eq!(finished_run.id, dispatch.run_id);
    assert_eq!(events.last().expect("completion event").kind, AgentEventKind::RunCompleted);

    let artifacts =
        TaskArtifactWriter::new().write_task_artifacts(&artifact_root, &completed_task)?;
    let meta: Value = serde_json::from_str(&fs::read_to_string(&artifacts.meta_json)?)?;
    assert_eq!(meta["type"], "finite");
    assert_eq!(meta["status"], "completed");
    assert_eq!(meta["task_id"], "task-e2e-finite");
    assert!(meta["completed_at"].as_str().is_some());
    assert!(
        fs::read_to_string(&artifacts.spec_md)?.contains("Complete the scheduler E2E lifecycle.")
    );
    assert!(fs::read_to_string(&artifacts.result_md)?.contains("E2E lifecycle completed."));

    let imported = TaskArtifactWriter::new()
        .import_legacy_task(&artifact_root.join("shared/tasks/task-e2e-finite"))?;
    assert_eq!(imported.task.status, AgentTaskStatus::Completed);
    assert_eq!(imported.spec.acceptance_criteria.len(), 3);

    Ok(())
}

fn append_lifecycle_event(
    store: &AgentSchedulerStore,
    project_path: &Path,
    task_id: &TaskId,
    run_id: &RunId,
    worker_id: &WorkerName,
    kind: AgentEventKind,
    from_status: AgentTaskStatus,
    to_status: AgentTaskStatus,
    message: &str,
    payload: Value,
    created_at_ms: TimestampMs,
) -> Result<()> {
    store.append_event(
        project_path,
        AppendAgentEventRequest {
            task_id: Some(task_id.clone()),
            run_id: Some(run_id.clone()),
            worker_id: Some(worker_id.clone()),
            team_id: Some(TeamId::new("team-e2e")?),
            kind,
            from_status: Some(from_status),
            to_status: Some(to_status),
            from_status_key: None,
            to_status_key: None,
            message: message.to_string(),
            payload,
            created_at_ms,
        },
    )?;
    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Agent scheduler E2E finite task".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: None,
        team_id: Some(TeamId::new("team-e2e")?),
        room_id: None,
        project_room_id: Some("!project-e2e-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "required_skills": ["scheduler-e2e"],
            "preferred_runtime": "copaw",
            "requirements": "Complete the scheduler E2E lifecycle.",
            "acceptance_criteria": [
                "dispatch selects the idle worker",
                "worker acknowledgment and completion are persisted",
                "outbox delivery and compatibility export remain readable"
            ],
            "context": "Covers create task -> dispatch -> ack -> complete -> outbox delivered.",
            "prompt": "Complete the scheduler E2E lifecycle.",
            "result": "E2E lifecycle completed."
        }),
        created_at_ms: TimestampMs(1_783_001_000_000),
    })
}

fn idle_worker(worker_id: &WorkerName) -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: worker_id.clone(),
        runtime: Some("copaw".to_string()),
        deployment: Some("remote".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Idle,
        desired_state: Some("Running".to_string()),
        team_id: Some(TeamId::new("team-e2e")?),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-e2e:example.test".to_string()),
        room_id: Some("!worker-e2e-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_000_900_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_000_950_000)),
        last_heartbeat_at: Some("2026-07-02T10:15:50Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill("scheduler-e2e")],
        updated_at_ms: TimestampMs(1_783_000_950_000),
    })
}

fn assert_run_persisted(
    store: &AgentSchedulerStore,
    run_id: &RunId,
    expected_status: &str,
    finished_at_ms: TimestampMs,
) -> Result<()> {
    let conn = Connection::open(store.database_path())?;
    let (status, finished_at): (String, Option<i64>) = conn.query_row(
        "SELECT status, finished_at_ms FROM scheduler_runs WHERE id = ?1",
        [run_id.as_str()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(status, expected_status);
    assert_eq!(finished_at, Some(finished_at_ms.0));
    Ok(())
}

fn assert_outbox_delivered(
    store: &AgentSchedulerStore,
    message_id: &str,
    delivered_at_ms: TimestampMs,
) -> Result<()> {
    let conn = Connection::open(store.database_path())?;
    let (status, attempts, delivered_at): (String, i64, Option<i64>) = conn.query_row(
        "SELECT status, attempts, delivered_at_ms FROM scheduler_outbox_messages WHERE id = ?1",
        [message_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 1);
    assert_eq!(delivered_at, Some(delivered_at_ms.0));
    Ok(())
}

fn isolation_project_info(id: &str, project_path: &Path, label: &str) -> Info {
    Info {
        id: id.to_string(),
        worktree: project_path.to_string_lossy().into_owned(),
        vcs: None,
        name: Some(label.to_string()),
        icon: None,
        commands: None,
        time: TimeInfo { created: 1, updated: 1, initialized: None },
        sandboxes: Vec::new(),
    }
}

fn isolation_task(label: &str) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("shared-task")?,
        title: format!("{label}-task"),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Assigned,
        spec_uri: "shared/tasks/shared-task/spec.md".to_string(),
        meta_uri: "shared/tasks/shared-task/meta.json".to_string(),
        assigned_to: Some(WorkerName::new("shared-worker")?),
        team_id: Some(TeamId::new("shared-team")?),
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"scope": label}),
        created_at_ms: TimestampMs(900),
    })
}

fn isolation_worker(label: &str) -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("shared-worker")?,
        runtime: Some("local".to_string()),
        deployment: Some("local".to_string()),
        model: format!("{label}-model"),
        model_provider: Some("test".to_string()),
        status: AgentWorkerStatus::Idle,
        desired_state: Some("Running".to_string()),
        team_id: Some(TeamId::new("shared-team")?),
        worker_role: Some("worker".to_string()),
        matrix_user_id: None,
        room_id: None,
        container_state: Some("ready".to_string()),
        idle_since_ms: Some(TimestampMs(900)),
        last_seen_at_ms: Some(TimestampMs(900)),
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: TimestampMs(900),
    })
}

fn isolation_team(label: &str) -> Result<NewAgentTeamSnapshot> {
    Ok(NewAgentTeamSnapshot {
        id: TeamId::new("shared-team")?,
        name: format!("{label}-team"),
        status: AgentTeamStatus::Active,
        leader_id: WorkerName::new("shared-worker")?,
        team_room_id: None,
        leader_dm_room_id: None,
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: Vec::new(),
        updated_at_ms: TimestampMs(900),
    })
}

fn assert_project_local_databases_absent(project_path: &Path) {
    let project_config = project_path.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME);
    for domain in ["agent_scheduler", "tasks"] {
        let database = project_config.join(domain).join("_index.sqlite3");
        assert!(!database.exists(), "project-local database was recreated: {}", database.display());
        assert!(!Path::new(&format!("{}-wal", database.display())).exists());
        assert!(!Path::new(&format!("{}-shm", database.display())).exists());
        assert!(!database.with_file_name("_index.lock").exists());
    }
}

#[cfg(unix)]
fn assert_global_database_permissions(database_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let directory = database_path.parent().expect("global database parent");
    assert_eq!(fs::metadata(directory)?.permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(database_path)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}

#[cfg(not(unix))]
fn assert_global_database_permissions(_database_path: &Path) -> Result<()> {
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct TestClock {
    now_ms: TimestampMs,
}

impl TestClock {
    fn at(now_ms: i64) -> Self {
        Self { now_ms: TimestampMs(now_ms) }
    }

    fn advance_ms(&mut self, delta_ms: i64) -> TimestampMs {
        self.now_ms = TimestampMs(self.now_ms.0 + delta_ms);
        self.now_ms
    }
}
