//! 测试调度日志存储的级别筛选、分页与作用域隔离。
//! 不同级别和时间戳组合覆盖过滤及分页边界，结果顺序保持确定。

use anyhow::Result;
use rusqlite::Connection;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{AgentSchedulerContext, RunId, TaskId, WorkerName};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::logs::{AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest};
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::NewAgentTask;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_scheduler_store_records_ordered_logs_and_scoped_workspace_context() -> Result<()> {
    for_each_log_store_provider(assert_records_ordered_logs_and_scoped_workspace_context)
}

#[test]
fn agent_scheduler_store_validates_log_and_workspace_run_ownership() -> Result<()> {
    for_each_log_store_provider(assert_validates_log_and_workspace_run_ownership)
}

#[test]
fn agent_scheduler_store_log_row_mappers_reject_invalid_domain_values() -> Result<()> {
    let conn = Connection::open_in_memory()?;

    let invalid_status = conn
        .query_row(
            "SELECT 'task-row-1', NULL, 0, 1, 'not-a-status', NULL, NULL, NULL, 'bad status'",
            [],
            super::store::logs::map_log_row,
        )
        .unwrap_err()
        .to_string();
    assert!(!invalid_status.is_empty());

    let invalid_workspace = conn
        .query_row(
            "SELECT '', 'run-row-1', 'slot-row', '/tmp/worktree', 'branch-row', 1",
            [],
            super::store::logs::map_workspace_row,
        )
        .unwrap_err()
        .to_string();
    assert!(!invalid_workspace.is_empty());

    Ok(())
}

fn assert_records_ordered_logs_and_scoped_workspace_context(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    let task_id = TaskId::new("task-observable-1")?;
    let run_id = RunId::new("run-observable-1")?;

    for scope in [context, isolated_context] {
        store.initialize_project(scope)?;
        store.create_task(scope, finite_task(&task_id)?)?;
        store.claim_task_run(
            scope,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new("worker-observe")?,
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(5_000)),
            },
        )?;
    }

    let first_log = store.append_task_log(
        context,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_100),
            from_status: Some(AgentTaskStatus::Dispatched),
            to_status: Some(AgentTaskStatus::Running),
            from_status_key: None,
            to_status_key: None,
            message: "workspace prepared".to_string(),
        },
    )?;
    let second_log = store.append_task_log(
        context,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_200),
            from_status: None,
            to_status: None,
            from_status_key: None,
            to_status_key: None,
            message: "command started".to_string(),
        },
    )?;
    let custom_status_log = store.append_task_log(
        context,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_225),
            from_status: Some(AgentTaskStatus::Running),
            to_status: Some(AgentTaskStatus::Submitted),
            from_status_key: Some("external_running".to_string()),
            to_status_key: Some("external_submitted".to_string()),
            message: "external status keys retained".to_string(),
        },
    )?;
    store.append_task_log(
        isolated_context,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id.clone()),
            timestamp_ms: TimestampMs(1_150),
            from_status: None,
            to_status: None,
            from_status_key: None,
            to_status_key: None,
            message: "isolated command".to_string(),
        },
    )?;

    assert_eq!(first_log.seq_no, 0);
    assert_eq!(second_log.seq_no, 1);
    let logs = store.list_task_logs(context, &task_id)?;
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].run_id, Some(run_id.clone()));
    assert_eq!(logs[0].from_status, Some(AgentTaskStatus::Dispatched));
    assert_eq!(logs[0].to_status, Some(AgentTaskStatus::Running));
    assert_eq!(logs[1].message, "command started");
    assert_eq!(custom_status_log.seq_no, 2);
    assert_eq!(custom_status_log.from_status_key.as_deref(), Some("external_running"));
    assert_eq!(custom_status_log.to_status_key.as_deref(), Some("external_submitted"));
    let isolated_logs = store.list_task_logs(isolated_context, &task_id)?;
    assert_eq!(isolated_logs.len(), 1);
    assert_eq!(isolated_logs[0].seq_no, 0);
    assert_eq!(isolated_logs[0].message, "isolated command");

    let workspace = AgentSchedulerWorkspaceContext {
        slot_id: "slot-alpha".to_string(),
        path: context.project_path().join("worktrees/slot-alpha").to_string_lossy().to_string(),
        branch: "vw/task/task-observable-1".to_string(),
    };
    let recorded_workspace = store
        .record_run_workspace_context(
            context,
            &task_id,
            &run_id,
            workspace.clone(),
            TimestampMs(1_250),
        )?
        .expect("workspace records for an existing run");
    assert_eq!(recorded_workspace.workspace, workspace);
    assert_eq!(recorded_workspace.recorded_at_ms, TimestampMs(1_250));
    assert_eq!(store.load_run_workspace_context(context, &run_id)?, Some(recorded_workspace));

    let updated_workspace = AgentSchedulerWorkspaceContext {
        slot_id: "slot-beta".to_string(),
        path: context.project_path().join("worktrees/slot-beta").to_string_lossy().to_string(),
        branch: "vw/task/task-observable-1-retry".to_string(),
    };
    let updated = store
        .record_run_workspace_context(
            context,
            &task_id,
            &run_id,
            updated_workspace.clone(),
            TimestampMs(1_300),
        )?
        .expect("existing run workspace can be updated");
    assert_eq!(updated.workspace, updated_workspace);
    assert_eq!(updated.recorded_at_ms, TimestampMs(1_300));
    assert_eq!(store.load_run_workspace_context(context, &run_id)?, Some(updated));
    assert!(store.load_run_workspace_context(isolated_context, &run_id)?.is_none());

    Ok(())
}

fn assert_validates_log_and_workspace_run_ownership(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    _isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    let task_id = TaskId::new("task-log-owner-1")?;
    let other_task_id = TaskId::new("task-log-owner-2")?;
    let owned_run_id = RunId::new("run-log-owner-1")?;
    let missing_run_id = RunId::new("run-log-owner-missing")?;

    store.initialize_project(context)?;
    store.create_task(context, finite_task(&task_id)?)?;
    store.create_task(context, finite_task(&other_task_id)?)?;
    store.claim_task_run(
        context,
        ClaimTaskRunRequest {
            run_id: owned_run_id.clone(),
            task_id: other_task_id.clone(),
            worker_id: WorkerName::new("worker-observe")?,
            lease_acquired_at_ms: TimestampMs(2_000),
            lease_expires_at_ms: None,
        },
    )?;

    let detached = store.append_task_log(
        context,
        AppendAgentTaskLogRequest {
            task_id: task_id.clone(),
            run_id: None,
            timestamp_ms: TimestampMs(2_100),
            from_status: None,
            to_status: Some(AgentTaskStatus::Running),
            from_status_key: None,
            to_status_key: None,
            message: "detached log entry".to_string(),
        },
    )?;
    assert_eq!(detached.run_id, None);
    assert_eq!(detached.to_status_key.as_deref(), Some("running"));

    let missing_log_error = store
        .append_task_log(
            context,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: Some(missing_run_id.clone()),
                timestamp_ms: TimestampMs(2_200),
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: "missing run".to_string(),
            },
        )
        .unwrap_err()
        .to_string();
    assert!(missing_log_error.contains("task log run not found"));

    let mismatched_log_error = store
        .append_task_log(
            context,
            AppendAgentTaskLogRequest {
                task_id: task_id.clone(),
                run_id: Some(owned_run_id.clone()),
                timestamp_ms: TimestampMs(2_300),
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: "wrong task".to_string(),
            },
        )
        .unwrap_err()
        .to_string();
    assert!(mismatched_log_error.contains("does not belong to task"));

    let workspace = AgentSchedulerWorkspaceContext {
        slot_id: "slot-owner".to_string(),
        path: context.project_path().join("worktrees/slot-owner").to_string_lossy().to_string(),
        branch: "vw/task/task-log-owner-1".to_string(),
    };
    assert!(
        store
            .record_run_workspace_context(
                context,
                &task_id,
                &missing_run_id,
                workspace.clone(),
                TimestampMs(2_400),
            )?
            .is_none()
    );
    let mismatched_workspace_error = store
        .record_run_workspace_context(
            context,
            &task_id,
            &owned_run_id,
            workspace,
            TimestampMs(2_500),
        )
        .unwrap_err()
        .to_string();
    assert!(mismatched_workspace_error.contains("does not belong to task"));

    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Record scheduler observability".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Dispatched,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: Some(WorkerName::new("worker-observe")?),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "observable-store-test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

// 公共断言同时覆盖内存数据库与可用 PostgreSQL，保持日志契约一致。
fn for_each_log_store_provider(
    mut check: impl FnMut(
        &AgentSchedulerStore,
        &AgentSchedulerContext,
        &AgentSchedulerContext,
    ) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (sqlite_store, sqlite_context, sqlite_isolated_context) =
        scheduler_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        scheduler_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn scheduler_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("logs-platform", "logs-company")?;
    let isolated_tenant = TenantScope::new("logs-platform-isolated", "logs-company-isolated")?;
    let project_id = ProjectId::from("logs-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Logs store project".to_owned()),
                icon: None,
                commands: None,
                time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
    }
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(primary_tenant, project_id.clone())?),
        project_path,
    )?;
    let isolated_context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(isolated_tenant, project_id)?),
        project_path,
    )?;
    Ok((AgentSchedulerStore::from_database(database), context, isolated_context))
}
