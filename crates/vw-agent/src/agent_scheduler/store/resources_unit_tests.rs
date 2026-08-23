//! 测试资源租约状态解析和底层过期更新辅助逻辑。

use anyhow::Result;
use rusqlite::{Connection, params};
use vw_api_types::common::TimestampMs;

use super::{
    ResourceKind, ResourceLeaseStatus, active_resource_lease_exists, expire_resource_leases,
};
use crate::agent_scheduler::domain::ResourceId;

#[test]
fn resource_lease_status_storage_values_parse_all_terminal_states() {
    assert_eq!(ResourceLeaseStatus::Revoked.as_str(), "revoked");
    assert_eq!(ResourceLeaseStatus::parse("expired").unwrap(), ResourceLeaseStatus::Expired);
    assert_eq!(ResourceLeaseStatus::parse("revoked").unwrap(), ResourceLeaseStatus::Revoked);

    let error = ResourceLeaseStatus::parse("paused").unwrap_err();
    assert!(error.to_string().contains("unsupported resource lease status: paused"));
}

#[test]
fn resource_lease_helpers_expire_only_matching_active_rows() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE TABLE scheduler_resource_leases (
            platform_id TEXT NOT NULL,
            company_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            id TEXT NOT NULL,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            holder_id TEXT NOT NULL,
            status TEXT NOT NULL,
            acquired_at_ms INTEGER NOT NULL,
            expires_at_ms INTEGER,
            released_at_ms INTEGER,
            taint_reason TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );",
    )?;
    conn.execute(
        "INSERT INTO scheduler_resource_leases (
            platform_id, company_id, project_id, id, resource_kind, resource_id, holder_id,
            status, acquired_at_ms, expires_at_ms, metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            "platform",
            "company",
            "project",
            "lease-active",
            ResourceKind::WorkerSlot.as_str(),
            "worker/slot-1",
            "holder",
            ResourceLeaseStatus::Active.as_str(),
            1_000_i64,
            1_500_i64,
            "{}",
        ],
    )?;
    conn.execute(
        "INSERT INTO scheduler_resource_leases (
            platform_id, company_id, project_id, id, resource_kind, resource_id, holder_id,
            status, acquired_at_ms, expires_at_ms, metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            "platform",
            "company",
            "project",
            "lease-other",
            ResourceKind::Worktree.as_str(),
            "worktree-1",
            "holder",
            ResourceLeaseStatus::Active.as_str(),
            1_000_i64,
            1_500_i64,
            "{}",
        ],
    )?;

    let resource_id = ResourceId::new("worker/slot-1")?;
    assert!(active_resource_lease_exists(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::WorkerSlot,
        &resource_id,
    )?);

    expire_resource_leases(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::WorkerSlot,
        &resource_id,
        TimestampMs(1_500),
    )?;

    assert!(!active_resource_lease_exists(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::WorkerSlot,
        &resource_id,
    )?);
    assert!(active_resource_lease_exists(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::Worktree,
        &ResourceId::new("worktree-1")?,
    )?);

    Ok(())
}

#[test]
fn resource_lease_helpers_propagate_sql_errors() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    let resource_id = ResourceId::new("missing-table-resource")?;

    let expire_error = expire_resource_leases(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::WorkerSlot,
        &resource_id,
        TimestampMs(1_500),
    )
    .expect_err("missing table should fail expiry update");
    assert!(expire_error.to_string().contains("scheduler_resource_leases"));

    let exists_error = active_resource_lease_exists(
        &conn,
        "platform",
        "company",
        "project",
        ResourceKind::WorkerSlot,
        &resource_id,
    )
    .expect_err("missing table should fail active lookup");
    assert!(exists_error.to_string().contains("scheduler_resource_leases"));

    Ok(())
}
