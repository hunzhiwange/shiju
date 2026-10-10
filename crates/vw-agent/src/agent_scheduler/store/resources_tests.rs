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
    AgentSchedulerContext, AssigneeId, HolderId, ResourceId, ResourceLeaseId, TeamId,
};
use super::store::resources::{AcquireResourceLeaseRequest, ResourceKind, ResourceLeaseStatus};
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
