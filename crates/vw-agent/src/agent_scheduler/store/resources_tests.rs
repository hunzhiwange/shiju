//! 测试资源存储在多后端下的租约排他性、过期与作用域隔离。
//! 公共断言在 SQLite 与可用 PostgreSQL 后端复用，另用并发屏障验证单一持有者。

use std::sync::{Arc, Barrier};

use anyhow::Result;
use rusqlite::{Connection, params};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{
    AgentSchedulerContext, AgentTeamStatus, AgentWorkerStatus, HolderId, ResourceId,
    ResourceLeaseId, TeamId, WorkerName,
};
use super::scheduler::worker_selection::{WorkerSelectionRequest, select_worker_candidates};
use super::store::resources::{AcquireResourceLeaseRequest, ResourceKind, ResourceLeaseStatus};
use super::store::teams::NewAgentTeamSnapshot;
use super::store::workers::{
    AgentWorkerFilter, NewAgentWorkerSnapshot, UpdateAgentWorkerLifecycleRequest,
};
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;


#[test]
fn agent_resource_store_rejects_invalid_lease_times_and_empty_taint_reason() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let lease_id = ResourceLeaseId::new("invalid-time-lease")?;
    let holder_id = HolderId::new("invalid-time-holder")?;

    let acquire_error = store
        .acquire_resource_lease(
            project_path,
            AcquireResourceLeaseRequest {
                lease_id: lease_id.clone(),
                resource_kind: ResourceKind::Worktree,
                resource_id: ResourceId::new("invalid-time-resource")?,
                holder_id: holder_id.clone(),
                acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(999)),
            },
        )
        .expect_err("lease expiry before acquisition must be rejected");
    assert!(
        acquire_error
            .to_string()
            .contains("resource lease expiry must not be before acquisition time")
    );

    let renew_error = store
        .renew_resource_lease(
            project_path,
            &lease_id,
            &holder_id,
            TimestampMs(1_000),
            TimestampMs(999),
        )
        .expect_err("lease expiry before renewal must be rejected");
    assert!(
        renew_error.to_string().contains("resource lease expiry must not be before renewal time")
    );

    let taint_error = store
        .mark_resource_tainted(
            project_path,
            ResourceKind::Worktree,
            &ResourceId::new("invalid-time-resource")?,
            " \t".to_string(),
        )
        .expect_err("empty taint reason must be rejected");
    assert!(taint_error.to_string().contains("resource taint reason must not be empty"));

    Ok(())
}

#[test]
fn agent_resource_store_enforces_exclusive_leases_and_expiry() -> Result<()> {
    for_each_resource_store_provider(assert_resource_store_enforces_exclusive_leases_and_expiry)
}


#[test]
fn agent_resource_store_reports_corrupt_lease_rows() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    insert_raw_resource_lease(
        &store,
        project_path,
        "bad-kind",
        "unsupported_kind",
        "active",
        "{}",
    )?;

    let kind_error = store
        .list_stale_resource_leases(project_path, TimestampMs(2_000))
        .expect_err("unsupported resource kind should fail row conversion");
    let kind_error_chain = format!("{kind_error:#}");
    assert!(kind_error_chain.contains("unsupported resource kind"), "{kind_error_chain}");

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    insert_raw_resource_lease(&store, project_path, "bad-json", "worker_slot", "active", "{")?;

    let metadata_error = store
        .list_stale_resource_leases(project_path, TimestampMs(2_000))
        .expect_err("malformed metadata should fail row conversion");
    let metadata_error_chain = format!("{metadata_error:#}");
    assert!(metadata_error_chain.contains("list stale resource leases"), "{metadata_error_chain}");

    Ok(())
}

fn assert_resource_store_enforces_exclusive_leases_and_expiry(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    _isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    let resource_id = ResourceId::new("worker-a/slot-0")?;

    store.initialize_project(context)?;

    let first = store
        .acquire_resource_lease(
            context,
            AcquireResourceLeaseRequest {
                lease_id: ResourceLeaseId::new("lease-1")?,
                resource_kind: ResourceKind::WorkerSlot,
                resource_id: resource_id.clone(),
                holder_id: HolderId::new("run-1")?,
                acquired_at_ms: TimestampMs(1_000),
                lease_expires_at_ms: Some(TimestampMs(1_500)),
            },
        )?
        .expect("first worker slot lease succeeds");
    assert_eq!(first.resource_kind, ResourceKind::WorkerSlot);
    assert_eq!(first.resource_id, resource_id);
    assert_eq!(first.holder_id.as_str(), "run-1");
    assert_eq!(first.status, ResourceLeaseStatus::Active);
    assert_eq!(first.lease_expires_at_ms, Some(TimestampMs(1_500)));
    assert!(
        store
            .renew_resource_lease(
                context,
                &ResourceLeaseId::new("lease-1")?,
                &HolderId::new("run-2")?,
                TimestampMs(1_400),
                TimestampMs(1_600),
            )?
            .is_none()
    );

    let renewed = store
        .renew_resource_lease(
            context,
            &ResourceLeaseId::new("lease-1")?,
            &HolderId::new("run-1")?,
            TimestampMs(1_400),
            TimestampMs(1_600),
        )?
        .expect("active worker slot lease renews");
    assert_eq!(renewed.lease_expires_at_ms, Some(TimestampMs(1_600)));

    let conflict = store.acquire_resource_lease(
        context,
        AcquireResourceLeaseRequest {
            lease_id: ResourceLeaseId::new("lease-2")?,
            resource_kind: ResourceKind::WorkerSlot,
            resource_id: ResourceId::new("worker-a/slot-0")?,
            holder_id: HolderId::new("run-2")?,
            acquired_at_ms: TimestampMs(1_100),
            lease_expires_at_ms: Some(TimestampMs(2_000)),
        },
    )?;
    assert!(conflict.is_none(), "same worker slot cannot be held by two runs");

    let tainted = store
        .mark_resource_tainted(
            context,
            ResourceKind::WorkerSlot,
            &ResourceId::new("worker-a/slot-0")?,
            "worker slot failed health check".to_string(),
        )?
        .expect("active resource can be marked tainted");
    assert_eq!(tainted.taint_reason.as_deref(), Some("worker slot failed health check"));

    let takeover = store
        .acquire_resource_lease(
            context,
            AcquireResourceLeaseRequest {
                lease_id: ResourceLeaseId::new("lease-3")?,
                resource_kind: ResourceKind::WorkerSlot,
                resource_id: ResourceId::new("worker-a/slot-0")?,
                holder_id: HolderId::new("run-3")?,
                acquired_at_ms: TimestampMs(1_600),
                lease_expires_at_ms: Some(TimestampMs(2_500)),
            },
        )?
        .expect("expired worker slot lease can be reclaimed");
    assert_eq!(takeover.holder_id.as_str(), "run-3");
    assert_eq!(takeover.status, ResourceLeaseStatus::Active);
    assert!(
        store
            .renew_resource_lease(
                context,
                &ResourceLeaseId::new("lease-1")?,
                &HolderId::new("run-1")?,
                TimestampMs(1_600),
                TimestampMs(2_000),
            )?
            .is_none()
    );

    let released = store
        .release_resource_lease(
            context,
            &ResourceLeaseId::new("lease-3")?,
            &HolderId::new("run-3")?,
            TimestampMs(1_700),
        )?
        .expect("active resource lease releases");
    assert_eq!(released.status, ResourceLeaseStatus::Released);
    assert_eq!(released.released_at_ms, Some(TimestampMs(1_700)));

    let after_release = store
        .acquire_resource_lease(
            context,
            AcquireResourceLeaseRequest {
                lease_id: ResourceLeaseId::new("lease-4")?,
                resource_kind: ResourceKind::WorkerSlot,
                resource_id: ResourceId::new("worker-a/slot-0")?,
                holder_id: HolderId::new("run-4")?,
                acquired_at_ms: TimestampMs(1_800),
                lease_expires_at_ms: None,
            },
        )?
        .expect("released worker slot can be leased again");
    assert_eq!(after_release.holder_id.as_str(), "run-4");

    Ok(())
}

#[test]
fn agent_scheduler_resources_are_isolated_by_full_scope() -> Result<()> {
    for_each_resource_store_provider(assert_scheduler_resources_are_isolated_by_full_scope)
}

fn assert_scheduler_resources_are_isolated_by_full_scope(
    store: &AgentSchedulerStore,
    scope_a: &AgentSchedulerContext,
    scope_b: &AgentSchedulerContext,
) -> Result<()> {
    store.upsert_worker_snapshot(scope_a, worker_snapshot("shared-worker", "model-a")?)?;
    store.upsert_worker_snapshot(scope_b, worker_snapshot("shared-worker", "model-b")?)?;
    let workers_a = store.list_workers(scope_a, AgentWorkerFilter::default())?;
    let workers_b = store.list_workers(scope_b, AgentWorkerFilter::default())?;
    assert_eq!(workers_a[0].model, "model-a");
    assert_eq!(workers_b[0].model, "model-b");
    let selection = WorkerSelectionRequest {
        preferred_runtime: Some("model-a".to_string()),
        ..WorkerSelectionRequest::default()
    };
    assert!(select_worker_candidates(&selection, &workers_a, &[])[0].runtime_matched);
    assert!(!select_worker_candidates(&selection, &workers_b, &[])[0].runtime_matched);

    store.update_worker_lifecycle(
        scope_a,
        UpdateAgentWorkerLifecycleRequest {
            worker_id: WorkerName::new("shared-worker")?,
            status: AgentWorkerStatus::Running,
            container_state: "running".to_string(),
            idle_since_ms: None,
            last_seen_at_ms: TimestampMs(1_100),
            last_heartbeat_at: None,
        },
    )?;
    assert_eq!(
        store.load_worker(scope_b, &WorkerName::new("shared-worker")?)?.unwrap().status,
        AgentWorkerStatus::Idle
    );

    store.upsert_team_snapshot(scope_a, team_snapshot("shared-team", "Team A")?)?;
    store.upsert_team_snapshot(scope_b, team_snapshot("shared-team", "Team B")?)?;
    assert_eq!(store.list_teams(scope_a)?[0].name, "Team A");
    assert_eq!(store.load_team(scope_b, &TeamId::new("shared-team")?)?.unwrap().name, "Team B");

    let lease_a = acquire_request("shared-lease", "shared-resource", "holder-a")?;
    let lease_b = acquire_request("shared-lease", "shared-resource", "holder-b")?;
    assert!(store.acquire_resource_lease(scope_a, lease_a)?.is_some());
    assert!(store.acquire_resource_lease(scope_b, lease_b)?.is_some());
    assert!(
        store
            .release_resource_lease(
                scope_b,
                &ResourceLeaseId::new("shared-lease")?,
                &HolderId::new("holder-a")?,
                TimestampMs(1_200),
            )?
            .is_none()
    );
    assert_eq!(store.list_stale_resource_leases(scope_a, TimestampMs(2_000))?.len(), 1);
    assert_eq!(store.list_stale_resource_leases(scope_b, TimestampMs(2_000))?.len(), 1);

    store.release_resource_lease(
        scope_a,
        &ResourceLeaseId::new("shared-lease")?,
        &HolderId::new("holder-a")?,
        TimestampMs(2_100),
    )?;
    assert!(store.list_stale_resource_leases(scope_a, TimestampMs(2_200))?.is_empty());
    assert_eq!(store.list_stale_resource_leases(scope_b, TimestampMs(2_200))?.len(), 1);

    Ok(())
}

#[test]
fn agent_resource_store_postgres_acquires_one_lease_under_concurrency() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let project_path = tempfile::tempdir()?;
    let database = fixture.database().clone();
    let (store, context, _) = resource_store_fixture(database.clone(), project_path.path())?;
    let competing_store = AgentSchedulerStore::from_database(database);
    store.initialize_project(&context)?;

    let barrier = Arc::new(Barrier::new(3));
    let leases = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_store = &store;
        let first_context = &context;
        let first = scope.spawn(move || {
            first_barrier.wait();
            first_store.acquire_resource_lease(
                first_context,
                concurrent_acquire_request("concurrent-lease-1", "holder-a"),
            )
        });
        let second_barrier = Arc::clone(&barrier);
        let second_store = &competing_store;
        let second_context = &context;
        let second = scope.spawn(move || {
            second_barrier.wait();
            second_store.acquire_resource_lease(
                second_context,
                concurrent_acquire_request("concurrent-lease-2", "holder-b"),
            )
        });
        barrier.wait();
        Ok::<_, anyhow::Error>([
            first.join().expect("first resource lease thread")?,
            second.join().expect("second resource lease thread")?,
        ])
    })?;

    assert_eq!(leases.iter().filter(|lease| lease.is_some()).count(), 1);
    fixture.cleanup()
}

fn concurrent_acquire_request(lease_id: &str, holder_id: &str) -> AcquireResourceLeaseRequest {
    AcquireResourceLeaseRequest {
        lease_id: ResourceLeaseId::new(lease_id).expect("valid lease id"),
        resource_kind: ResourceKind::WorkerSlot,
        resource_id: ResourceId::new("shared-concurrent-resource").expect("valid resource id"),
        holder_id: HolderId::new(holder_id).expect("valid holder id"),
        acquired_at_ms: TimestampMs(1_000),
        lease_expires_at_ms: Some(TimestampMs(2_000)),
    }
}

// 跨后端执行租约契约；并发专用场景另行使用独立连接验证。
fn for_each_resource_store_provider(
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
        resource_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        resource_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn resource_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("resources-platform", "resources-company")?;
    let isolated_tenant =
        TenantScope::new("resources-platform-isolated", "resources-company-isolated")?;
    let project_id = ProjectId::from("resources-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Resources store project".to_owned()),
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


fn team_snapshot(id: &str, name: &str) -> Result<NewAgentTeamSnapshot> {
    Ok(NewAgentTeamSnapshot {
        id: TeamId::new(id)?,
        name: name.to_string(),
        status: AgentTeamStatus::Active,
        leader_id: WorkerName::new("shared-worker")?,
        team_room_id: None,
        leader_dm_room_id: None,
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: Vec::new(),
        updated_at_ms: TimestampMs(1_000),
    })
}

fn acquire_request(
    lease_id: &str,
    resource_id: &str,
    holder_id: &str,
) -> Result<AcquireResourceLeaseRequest> {
    Ok(AcquireResourceLeaseRequest {
        lease_id: ResourceLeaseId::new(lease_id)?,
        resource_kind: ResourceKind::WorkerSlot,
        resource_id: ResourceId::new(resource_id)?,
        holder_id: HolderId::new(holder_id)?,
        acquired_at_ms: TimestampMs(1_000),
        lease_expires_at_ms: Some(TimestampMs(1_500)),
    })
}

fn insert_raw_resource_lease(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    lease_id: &str,
    resource_kind: &str,
    status: &str,
    metadata_json: &str,
) -> Result<()> {
    let scope = project_path.storage_scope()?;
    let project = scope.project_scope();
    let conn = Connection::open(store.database_path())?;
    conn.execute(
        "INSERT INTO scheduler_resource_leases (
             platform_id, company_id, project_id, id, resource_kind, resource_id, holder_id,
             status, acquired_at_ms, expires_at_ms, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            project.platform_id().as_ref(),
            project.company_id().as_ref(),
            project.project_id().as_ref(),
            lease_id,
            resource_kind,
            format!("{lease_id}-resource"),
            format!("{lease_id}-holder"),
            status,
            1_000_i64,
            1_500_i64,
            metadata_json,
        ],
    )?;
    Ok(())
}

fn worker_snapshot(id: &str, model: &str) -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new(id)?,
        runtime: Some(model.to_string()),
        deployment: Some("local".to_string()),
        model: model.to_string(),
        model_provider: Some("test".to_string()),
        status: AgentWorkerStatus::Idle,
        desired_state: Some("Running".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: None,
        room_id: None,
        container_state: Some("ready".to_string()),
        idle_since_ms: Some(TimestampMs(1_000)),
        last_seen_at_ms: Some(TimestampMs(1_000)),
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: TimestampMs(1_000),
    })
}
