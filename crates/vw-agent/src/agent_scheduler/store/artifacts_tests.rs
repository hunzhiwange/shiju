//! 测试调度产物存储的写入、列表、去重与完整作用域隔离。

use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{AgentSchedulerContext, RunId, TaskId, WorkerName};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::artifacts::RecordTaskArtifactRequest;
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::NewAgentTask;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_scheduler_artifact_store_lists_ordered_scope_local_artifacts() -> Result<()> {
    for_each_artifact_store_provider(assert_lists_ordered_scope_local_artifacts)
}

#[test]
fn agent_scheduler_artifact_store_rejects_blank_kind_and_name_before_storage() -> Result<()> {
    let store = AgentSchedulerStore::new();
    let project_path = std::path::Path::new("unused-artifact-validation-project");

    let blank_kind = store
        .record_task_artifact(
            project_path,
            artifact_request("  ", "result.json", "artifact-validation-kind")?,
        )
        .expect_err("blank artifact kind must be rejected");
    assert_eq!(blank_kind.to_string(), "task artifact kind must not be empty");

    let blank_name = store
        .record_task_artifact(
            project_path,
            artifact_request("result", "\t", "artifact-validation-name")?,
        )
        .expect_err("blank artifact name must be rejected");
    assert_eq!(blank_name.to_string(), "task artifact name must not be empty");

    Ok(())
}

#[test]
fn agent_scheduler_artifact_store_rejects_missing_run_before_recording() -> Result<()> {
    let store = AgentSchedulerStore::new();
    let project_home = tempfile::tempdir()?;
    let project_path = project_home.path();
    let task_id = TaskId::new("artifact-missing-run-task")?;

    store.initialize_project(project_path)?;
    store.create_task(project_path, finite_task(&task_id)?)?;

    let error = store
        .record_task_artifact(
            project_path,
            RecordTaskArtifactRequest {
                task_id,
                run_id: Some(RunId::new("artifact-missing-run")?),
                kind: "execution_result".to_string(),
                name: "missing-run.json".to_string(),
                path: None,
                payload: json!({}),
                created_at_ms: TimestampMs(1_010),
            },
        )
        .expect_err("artifact must reject an unknown run id");

    assert_eq!(error.to_string(), "task artifact run not found: artifact-missing-run");
    Ok(())
}

#[test]
fn agent_scheduler_artifact_store_rejects_run_for_different_task() -> Result<()> {
    let store = AgentSchedulerStore::new();
    let project_home = tempfile::tempdir()?;
    let project_path = project_home.path();
    let owning_task_id = TaskId::new("artifact-owning-task")?;
    let other_task_id = TaskId::new("artifact-other-task")?;
    let run_id = RunId::new("artifact-owned-run")?;

    store.initialize_project(project_path)?;
    store.create_task(project_path, finite_task(&owning_task_id)?)?;
    store.create_task(project_path, finite_task(&other_task_id)?)?;
    store.claim_task_run(
        project_path,
        ClaimTaskRunRequest {
            run_id: run_id.clone(),
            task_id: owning_task_id,
            worker_id: WorkerName::new("worker-artifact-mismatch")?,
            lease_acquired_at_ms: TimestampMs(1_000),
            lease_expires_at_ms: Some(TimestampMs(5_000)),
        },
    )?;

    let error = store
        .record_task_artifact(
            project_path,
            RecordTaskArtifactRequest {
                task_id: other_task_id,
                run_id: Some(run_id),
                kind: "execution_result".to_string(),
                name: "mismatched-run.json".to_string(),
                path: None,
                payload: json!({}),
                created_at_ms: TimestampMs(1_020),
            },
        )
        .expect_err("artifact must reject a run owned by another task");

    assert_eq!(
        error.to_string(),
        "task artifact run artifact-owned-run does not belong to task artifact-other-task"
    );
    Ok(())
}

fn artifact_request(kind: &str, name: &str, task_id: &str) -> Result<RecordTaskArtifactRequest> {
    Ok(RecordTaskArtifactRequest {
        task_id: TaskId::new(task_id)?,
        run_id: None,
        kind: kind.to_string(),
        name: name.to_string(),
        path: None,
        payload: json!({}),
        created_at_ms: TimestampMs(1_000),
    })
}

fn assert_lists_ordered_scope_local_artifacts(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    let task_id = TaskId::new("task-artifact-store-1")?;
    let run_id = RunId::new("run-artifact-store-1")?;

    for scope in [context, isolated_context] {
        store.initialize_project(scope)?;
        store.create_task(scope, finite_task(&task_id)?)?;
        store.claim_task_run(
            scope,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new("worker-artifact")?,
                lease_acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(5_000)),
            },
        )?;
    }

    let first = store.record_task_artifact(
        context,
        RecordTaskArtifactRequest {
            task_id: task_id.clone(),
            run_id: Some(run_id),
            kind: "execution_result".to_string(),
            name: "first.json".to_string(),
            path: Some("shared/tasks/task-artifact-store-1/artifacts/first.json".to_string()),
            payload: json!({"result": "first"}),
            created_at_ms: TimestampMs(1_100),
        },
    )?;
    let second = store.record_task_artifact(
        context,
        RecordTaskArtifactRequest {
            task_id: task_id.clone(),
            run_id: None,
            kind: "note".to_string(),
            name: "second.json".to_string(),
            path: None,
            payload: json!({"result": "second"}),
            created_at_ms: TimestampMs(1_200),
        },
    )?;
    store.record_task_artifact(
        isolated_context,
        RecordTaskArtifactRequest {
            task_id: task_id.clone(),
            run_id: None,
            kind: "note".to_string(),
            name: "isolated.json".to_string(),
            path: None,
            payload: json!({"result": "isolated"}),
            created_at_ms: TimestampMs(1_150),
        },
    )?;

    assert!(first.id > 0);
    assert!(second.id > first.id);
    let artifacts = store.list_task_artifacts(context, &task_id)?;
    assert_eq!(
        artifacts.iter().map(|artifact| artifact.name.as_str()).collect::<Vec<_>>(),
        vec!["first.json", "second.json"]
    );
    assert_eq!(artifacts[0].payload["result"], "first");
    assert_eq!(artifacts[1].payload["result"], "second");
    let isolated_artifacts = store.list_task_artifacts(isolated_context, &task_id)?;
    assert_eq!(isolated_artifacts.len(), 1);
    assert_eq!(isolated_artifacts[0].name, "isolated.json");

    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Record scheduler artifact".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Dispatched,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: Some(WorkerName::new("worker-artifact")?),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "artifact-store-test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn for_each_artifact_store_provider(
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
    let primary_tenant = TenantScope::new("artifacts-platform", "artifacts-company")?;
    let isolated_tenant =
        TenantScope::new("artifacts-platform-isolated", "artifacts-company-isolated")?;
    let project_id = ProjectId::from("artifacts-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Artifacts store project".to_owned()),
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
