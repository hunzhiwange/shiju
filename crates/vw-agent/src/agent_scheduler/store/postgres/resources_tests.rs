//! 测试 PostgreSQL 资源租约行映射保留生命周期、污染状态和元数据。
//! 未知种类、状态、非法标识与畸形元数据必须显式拒绝。

use anyhow::Result;
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::map_resource_lease_row;
use crate::agent_scheduler::store::resources::{ResourceKind, ResourceLeaseStatus};
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn map_resource_lease_row_preserves_lifecycle_taint_and_metadata() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let lease = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'worktree'::TEXT, 'worktree-a'::TEXT,
                    'run-1'::TEXT, 'released'::TEXT, 1000::BIGINT, 2000::BIGINT,
                    1500::BIGINT, 'dirty worktree'::TEXT, '{\"path\":\"/tmp/a\"}'::TEXT",
            &[],
        )?;
        map_resource_lease_row(&row)?
    };

    assert_eq!(lease.id.as_str(), "lease-1");
    assert_eq!(lease.resource_kind, ResourceKind::Worktree);
    assert_eq!(lease.resource_id.as_str(), "worktree-a");
    assert_eq!(lease.holder_id.as_str(), "run-1");
    assert_eq!(lease.status, ResourceLeaseStatus::Released);
    assert_eq!(lease.acquired_at_ms, TimestampMs(1_000));
    assert_eq!(lease.lease_expires_at_ms, Some(TimestampMs(2_000)));
    assert_eq!(lease.released_at_ms, Some(TimestampMs(1_500)));
    assert_eq!(lease.taint_reason.as_deref(), Some("dirty worktree"));
    assert_eq!(lease.metadata, json!({"path": "/tmp/a"}));

    fixture.cleanup()
}

#[test]
fn map_resource_lease_row_preserves_active_null_lifecycle_fields() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let lease = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'lease-2'::TEXT, 'team_capacity'::TEXT, 'team-a'::TEXT,
                    'scheduler-1'::TEXT, 'active'::TEXT, 3000::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{\"slots\":2}'::TEXT",
            &[],
        )?;
        map_resource_lease_row(&row)?
    };

    assert_eq!(lease.id.as_str(), "lease-2");
    assert_eq!(lease.resource_kind, ResourceKind::TeamCapacity);
    assert_eq!(lease.resource_id.as_str(), "team-a");
    assert_eq!(lease.holder_id.as_str(), "scheduler-1");
    assert_eq!(lease.status, ResourceLeaseStatus::Active);
    assert_eq!(lease.acquired_at_ms, TimestampMs(3_000));
    assert_eq!(lease.lease_expires_at_ms, None);
    assert_eq!(lease.released_at_ms, None);
    assert_eq!(lease.taint_reason, None);
    assert_eq!(lease.metadata, json!({"slots": 2}));

    fixture.cleanup()
}

#[test]
fn map_resource_lease_row_rejects_unknown_kind_and_malformed_metadata() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let unknown_kind = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'unknown'::TEXT, 'resource-1'::TEXT,
                    'holder-1'::TEXT, 'active'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{}'::TEXT",
            &[],
        )?;
        let error =
            map_resource_lease_row(&unknown_kind).expect_err("unknown resource kind must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource kind"));

        let malformed_metadata = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'worker_slot'::TEXT, 'resource-1'::TEXT,
                    'holder-1'::TEXT, 'active'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{'::TEXT",
            &[],
        )?;
        let error = map_resource_lease_row(&malformed_metadata)
            .expect_err("malformed resource metadata must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource metadata"));
    }
    fixture.cleanup()
}

#[test]
fn map_resource_lease_row_rejects_invalid_ids_and_unknown_status() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let empty_lease_id = connection.query_one(
            "SELECT ''::TEXT, 'worktree'::TEXT, 'resource-1'::TEXT,
                    'holder-1'::TEXT, 'active'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{}'::TEXT",
            &[],
        )?;
        let error =
            map_resource_lease_row(&empty_lease_id).expect_err("empty resource lease id must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource lease id"));

        let empty_resource_id = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'worktree'::TEXT, ''::TEXT,
                    'holder-1'::TEXT, 'active'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{}'::TEXT",
            &[],
        )?;
        let error =
            map_resource_lease_row(&empty_resource_id).expect_err("empty resource id must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource id"));

        let empty_holder_id = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'worktree'::TEXT, 'resource-1'::TEXT,
                    ''::TEXT, 'active'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{}'::TEXT",
            &[],
        )?;
        let error =
            map_resource_lease_row(&empty_holder_id).expect_err("empty holder id must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource holder"));

        let unknown_status = connection.query_one(
            "SELECT 'lease-1'::TEXT, 'worktree'::TEXT, 'resource-1'::TEXT,
                    'holder-1'::TEXT, 'stuck'::TEXT, 1::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT, '{}'::TEXT",
            &[],
        )?;
        let error = map_resource_lease_row(&unknown_status).expect_err("unknown status must fail");
        assert!(error.to_string().contains("parse PostgreSQL resource lease status"));
    }
    fixture.cleanup()
}
