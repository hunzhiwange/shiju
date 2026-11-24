//! 测试调度器跨存储、策略与运行器组件的集成流程。
//! 使用真实存储门面串联任务、工作代理和运行记录，覆盖组件边界上的状态一致性。

use std::path::Path;

use anyhow::Result;
use vw_api_agent_scheduler::AgentSchedulerScope;
use vw_api_transport::common::TimestampMs;
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
use project_types::{Info, TimeInfo};

use crate::domain::{AgentSchedulerContext, HolderId, ResourceId, ResourceLeaseId};
use crate::store::AgentSchedulerStore;
use crate::store::resources::{AcquireResourceLeaseRequest, ResourceKind};
use agent_database::global_database::GlobalDatabase;
use agent_project::store::ProjectStore;

#[test]
fn agent_scheduler_requires_same_scope_parent_project() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    assert_scheduler_requires_same_scope_parent_project(sqlite_database)?;

    let postgres_fixture = agent_database::global_database::test_support::postgres_test_database!();
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

#[tokio::test]
async fn task_pool_recovery_resources_wait_for_live_directory_owner() -> Result<()> {
    use crate::engine::recovery::RecoveryEngine;
    use crate::runner::workspace_claim::{WorkspaceClaim, lock_root};
    let project = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(project.path())?;
    let lease = store
        .acquire_resource_lease(
            project.path(),
            AcquireResourceLeaseRequest {
                lease_id: ResourceLeaseId::new("old-lease")?,
                resource_kind: ResourceKind::WorkerSlot,
                resource_id: ResourceId::new("slot")?,
                holder_id: HolderId::new("live-run")?,
                acquired_at_ms: TimestampMs(1),
                lease_expires_at_ms: Some(TimestampMs(2)),
            },
        )?
        .unwrap();
    let mut claim =
        WorkspaceClaim::acquire(&lock_root()?, project.path(), "live-run".into())?.unwrap();
    let engine = RecoveryEngine::new(&store);
    assert!(
        engine
            .reconcile_once(project.path(), TimestampMs(3))
            .await?
            .released_resource_lease_ids
            .is_empty()
    );
    claim.release("live-run")?;
    let result = engine.reconcile_once(project.path(), TimestampMs(4)).await?;
    assert_eq!(result.released_resource_lease_ids, vec![lease.id]);
    assert!(
        engine
            .reconcile_once(project.path(), TimestampMs(5))
            .await?
            .released_resource_lease_ids
            .is_empty()
    );
    Ok(())
}

#[test]
fn task_pool_recovery_old_lease_cannot_taint_new_owner() -> Result<()> {
    let project = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(project.path())?;
    let request = |id: &str, owner: &str, at| -> Result<AcquireResourceLeaseRequest> {
        Ok(AcquireResourceLeaseRequest {
            lease_id: ResourceLeaseId::new(id)?,
            resource_kind: ResourceKind::WorkerSlot,
            resource_id: ResourceId::new("slot")?,
            holder_id: HolderId::new(owner)?,
            acquired_at_ms: TimestampMs(at),
            lease_expires_at_ms: Some(TimestampMs(at + 1)),
        })
    };
    let old = store.acquire_resource_lease(project.path(), request("old", "old-run", 1)?)?.unwrap();
    let new = store.acquire_resource_lease(project.path(), request("new", "new-run", 3)?)?.unwrap();
    assert!(
        store.mark_resource_lease_tainted(project.path(), &old, "old failure".into())?.is_none()
    );
    assert!(
        store
            .release_resource_lease(project.path(), &old.id, &old.holder_id, TimestampMs(4))?
            .is_none()
    );
    let current = store
        .release_resource_lease(project.path(), &new.id, &new.holder_id, TimestampMs(4))?
        .unwrap();
    assert!(current.taint_reason.is_none());
    Ok(())
}
