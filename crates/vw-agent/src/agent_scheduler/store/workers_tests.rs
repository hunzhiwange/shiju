//! 测试工作代理存储在多后端下的能力、团队归属与生命周期。
//! 独立成员角色和能力组合覆盖列表顺序、空白校验与缺失记录更新。

use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{
    AgentCapabilityKind, AgentSchedulerContext, AgentWorkerStatus, TeamId, WorkerName,
};
use super::store::AgentSchedulerStore;
use super::store::workers::{
    AgentCapabilityRecord, AgentWorkerFilter, NewAgentWorkerSnapshot,
    UpdateAgentWorkerLifecycleRequest,
};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_worker_store_tracks_capabilities_and_lifecycle_state() -> Result<()> {
    for_each_worker_store_provider(assert_worker_store_tracks_capabilities_and_lifecycle_state)
}

#[test]
fn agent_worker_store_rejects_blank_model_and_capability_name() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let (store, context, _) = worker_store_fixture(database, &home.path().join("project"))?;

    let mut blank_model = standalone_worker()?;
    blank_model.model = " ".to_string();
    let error =
        store.upsert_worker_snapshot(&context, blank_model).expect_err("blank model must fail");
    assert!(error.to_string().contains("worker model must not be empty"));

    let mut blank_capability = standalone_worker()?;
    blank_capability.capabilities[0].name = " ".to_string();
    let error = store
        .upsert_worker_snapshot(&context, blank_capability)
        .expect_err("blank capability name must fail");
    assert!(error.to_string().contains("worker capability name must not be empty"));

    Ok(())
}

#[test]
fn agent_worker_lifecycle_rejects_blank_state_and_missing_worker() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let (store, context, _) = worker_store_fixture(database, &home.path().join("project"))?;

    let blank_state = store
        .update_worker_lifecycle(
            &context,
            UpdateAgentWorkerLifecycleRequest {
                worker_id: WorkerName::new("worker-missing")?,
                status: AgentWorkerStatus::Running,
                container_state: " ".to_string(),
                idle_since_ms: None,
                last_seen_at_ms: TimestampMs(1_783_000_030_000),
                last_heartbeat_at: None,
            },
        )
        .expect_err("blank container state must fail");
    assert!(blank_state.to_string().contains("worker container state must not be empty"));

    store.initialize_project(&context)?;
    let missing_worker = store
        .update_worker_lifecycle(
            &context,
            UpdateAgentWorkerLifecycleRequest {
                worker_id: WorkerName::new("worker-missing")?,
                status: AgentWorkerStatus::Running,
                container_state: "running".to_string(),
                idle_since_ms: None,
                last_seen_at_ms: TimestampMs(1_783_000_030_000),
                last_heartbeat_at: None,
            },
        )
        .expect_err("missing worker must fail");
    assert!(missing_worker.to_string().contains("worker not found: worker-missing"));

    Ok(())
}

fn assert_worker_store_tracks_capabilities_and_lifecycle_state(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;
    store.upsert_worker_snapshot(context, standalone_worker()?)?;
    store.upsert_worker_snapshot(context, team_leader()?)?;
    store.upsert_worker_snapshot(context, team_worker()?)?;
    let mut isolated = standalone_worker()?;
    isolated.model = "isolated-model".to_string();
    store.upsert_worker_snapshot(isolated_context, isolated)?;

    let copaw_workers = store.list_workers(
        context,
        AgentWorkerFilter { runtime: Some("copaw".to_string()), ..AgentWorkerFilter::default() },
    )?;
    assert_eq!(worker_ids(&copaw_workers), vec!["leader-a", "worker-api"]);

    let github_workers = store.list_workers(
        context,
        AgentWorkerFilter {
            required_skills: vec!["github-operations".to_string(), "git-delegation".to_string()],
            ..AgentWorkerFilter::default()
        },
    )?;
    assert_eq!(worker_ids(&github_workers), vec!["worker-standalone"]);
    assert_eq!(github_workers[0].deployment.as_deref(), Some("local"));
    assert_eq!(github_workers[0].room_id.as_deref(), Some("!standalone-room:example.test"));
    assert_eq!(github_workers[0].desired_state.as_deref(), Some("Running"));
    assert_eq!(github_workers[0].container_state.as_deref(), Some("running"));
    assert_eq!(github_workers[0].idle_since_ms, Some(TimestampMs(1_783_000_000_000)));
    assert_eq!(github_workers[0].last_seen_at_ms, Some(TimestampMs(1_783_000_010_000)));

    let team_workers = store.list_workers(
        context,
        AgentWorkerFilter {
            team_id: Some(TeamId::new("team-alpha")?),
            container_state: Some("ready".to_string()),
            ..AgentWorkerFilter::default()
        },
    )?;
    assert_eq!(worker_ids(&team_workers), vec!["leader-a", "worker-api"]);
    assert_eq!(team_workers[0].worker_role.as_deref(), Some("team_leader"));
    assert_eq!(team_workers[1].worker_role.as_deref(), Some("worker"));
    assert_eq!(
        store
            .load_worker(isolated_context, &WorkerName::new("worker-standalone")?)?
            .expect("isolated worker stored")
            .model,
        "isolated-model"
    );

    Ok(())
}

// 工作代理生命周期断言跨后端复用，隔离具体 SQL 表达差异。
fn for_each_worker_store_provider(
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
        worker_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        worker_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn worker_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("workers-platform", "workers-company")?;
    let isolated_tenant =
        TenantScope::new("workers-platform-isolated", "workers-company-isolated")?;
    let project_id = ProjectId::from("workers-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Workers store project".to_owned()),
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




fn worker_ids(workers: &[super::store::workers::StoredAgentWorker]) -> Vec<&str> {
    workers.iter().map(|worker| worker.id.as_str()).collect()
}

fn standalone_worker() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("worker-standalone")?,
        runtime: Some("openclaw".to_string()),
        deployment: Some("local".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Idle,
        desired_state: Some("Running".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-standalone:example.test".to_string()),
        room_id: Some("!standalone-room:example.test".to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_000_000_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_000_010_000)),
        last_heartbeat_at: Some("2026-07-02T10:01:40Z".to_string()),
        capabilities: vec![
            AgentCapabilityRecord::skill("github-operations"),
            AgentCapabilityRecord::skill("git-delegation"),
            AgentCapabilityRecord {
                kind: AgentCapabilityKind::Runtime,
                name: "openclaw".to_string(),
                metadata: json!({"source": "registry"}),
            },
        ],
        updated_at_ms: TimestampMs(1_783_000_010_000),
    })
}

fn team_leader() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("leader-a")?,
        runtime: Some("copaw".to_string()),
        deployment: Some("remote".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Running,
        desired_state: Some("Running".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("team_leader".to_string()),
        matrix_user_id: Some("@leader-a:example.test".to_string()),
        room_id: Some("!leader-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_783_000_020_000)),
        last_heartbeat_at: Some("2026-07-02T10:03:20Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill("project-management")],
        updated_at_ms: TimestampMs(1_783_000_020_000),
    })
}

fn team_worker() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("worker-api")?,
        runtime: Some("copaw".to_string()),
        deployment: Some("remote".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Running,
        desired_state: Some("Running".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-api:example.test".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_783_000_020_000)),
        last_heartbeat_at: Some("2026-07-02T10:03:20Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill("api-implementation")],
        updated_at_ms: TimestampMs(1_783_000_020_000),
    })
}
