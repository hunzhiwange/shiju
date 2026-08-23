//! 实现 SQLite 资源租约的获取、查询、续期和释放，并维护互斥与过期语义。

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{HolderId, ResourceId, ResourceLeaseId};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const RESOURCE_LEASE_COLUMNS: &str = "id, resource_kind, resource_id, holder_id, status, \
acquired_at_ms, expires_at_ms, released_at_ms, taint_reason, metadata_json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    WorkerSlot,
    Worktree,
    MatrixDispatch,
    TeamCapacity,
    MergeTarget,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkerSlot => "worker_slot",
            Self::Worktree => "worktree",
            Self::MatrixDispatch => "matrix_dispatch",
            Self::TeamCapacity => "team_capacity",
            Self::MergeTarget => "merge_target",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "worker_slot" => Ok(Self::WorkerSlot),
            "worktree" => Ok(Self::Worktree),
            "matrix_dispatch" => Ok(Self::MatrixDispatch),
            "team_capacity" => Ok(Self::TeamCapacity),
            "merge_target" => Ok(Self::MergeTarget),
            other => bail!("unsupported resource kind: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLeaseStatus {
    Active,
    Released,
    Expired,
    Revoked,
}

impl ResourceLeaseStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Released => "released",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "released" => Ok(Self::Released),
            "expired" => Ok(Self::Expired),
            "revoked" => Ok(Self::Revoked),
            other => bail!("unsupported resource lease status: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquireResourceLeaseRequest {
    pub lease_id: ResourceLeaseId,
    pub resource_kind: ResourceKind,
    pub resource_id: ResourceId,
    pub holder_id: HolderId,
    pub acquired_at_ms: TimestampMs,
    pub lease_expires_at_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredResourceLease {
    pub id: ResourceLeaseId,
    pub resource_kind: ResourceKind,
    pub resource_id: ResourceId,
    pub holder_id: HolderId,
    pub status: ResourceLeaseStatus,
    pub acquired_at_ms: TimestampMs,
    pub lease_expires_at_ms: Option<TimestampMs>,
    pub released_at_ms: Option<TimestampMs>,
    pub taint_reason: Option<String>,
    pub metadata: Value,
}

impl AgentSchedulerStore {
    pub fn acquire_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: AcquireResourceLeaseRequest,
    ) -> Result<Option<StoredResourceLease>> {
        validate_acquire_request(&request)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.acquire_resource_lease(context, request);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        expire_resource_leases(
            &tx,
            platform_id,
            company_id,
            project_id,
            request.resource_kind,
            &request.resource_id,
            request.acquired_at_ms,
        )?;
        if active_resource_lease_exists(
            &tx,
            platform_id,
            company_id,
            project_id,
            request.resource_kind,
            &request.resource_id,
        )? {
            tx.commit()?;
            return Ok(None);
        }

        tx.execute(
            "INSERT INTO scheduler_resource_leases (
                 platform_id, company_id, project_id, id, resource_kind, resource_id, holder_id,
                 status, acquired_at_ms, expires_at_ms, metadata_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                platform_id,
                company_id,
                project_id,
                request.lease_id.as_str(),
                request.resource_kind.as_str(),
                request.resource_id.as_str(),
                request.holder_id.as_str(),
                ResourceLeaseStatus::Active.as_str(),
                request.acquired_at_ms.0,
                request.lease_expires_at_ms.map(|value| value.0),
                "{}",
            ],
        )?;

        let stored =
            load_resource_lease_with_conn(&tx, context, &request.lease_id)?.with_context(|| {
                format!("resource lease disappeared after acquire: {}", request.lease_id.as_str())
            })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn release_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        lease_id: &ResourceLeaseId,
        holder_id: &HolderId,
        released_at_ms: TimestampMs,
    ) -> Result<Option<StoredResourceLease>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.release_resource_lease(context, lease_id, holder_id, released_at_ms);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_resource_leases
             SET status = ?1, released_at_ms = ?2
             WHERE platform_id = ?3 AND company_id = ?4 AND project_id = ?5
               AND id = ?6 AND holder_id = ?7 AND status = ?8",
            params![
                ResourceLeaseStatus::Released.as_str(),
                released_at_ms.0,
                platform_id,
                company_id,
                project_id,
                lease_id.as_str(),
                holder_id.as_str(),
                ResourceLeaseStatus::Active.as_str(),
            ],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }

        let stored = load_resource_lease_with_conn(&tx, context, lease_id)?.with_context(|| {
            format!("resource lease disappeared after release: {}", lease_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn renew_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        lease_id: &ResourceLeaseId,
        holder_id: &HolderId,
        renewed_at_ms: TimestampMs,
        lease_expires_at_ms: TimestampMs,
    ) -> Result<Option<StoredResourceLease>> {
        if lease_expires_at_ms < renewed_at_ms {
            bail!("resource lease expiry must not be before renewal time");
        }
        if let Some(store) = postgres_provider(self, context)? {
            return store.renew_resource_lease(
                context,
                lease_id,
                holder_id,
                renewed_at_ms,
                lease_expires_at_ms,
            );
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_resource_leases
             SET expires_at_ms = ?1
             WHERE platform_id = ?2 AND company_id = ?3 AND project_id = ?4
               AND id = ?5 AND holder_id = ?6 AND status = ?7
               AND (expires_at_ms IS NULL OR expires_at_ms > ?8)",
            params![
                lease_expires_at_ms.0,
                platform_id,
                company_id,
                project_id,
                lease_id.as_str(),
                holder_id.as_str(),
                ResourceLeaseStatus::Active.as_str(),
                renewed_at_ms.0,
            ],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let stored = load_resource_lease_with_conn(&tx, context, lease_id)?.with_context(|| {
            format!("resource lease disappeared after renewal: {}", lease_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn mark_resource_tainted(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        resource_kind: ResourceKind,
        resource_id: &ResourceId,
        taint_reason: String,
    ) -> Result<Option<StoredResourceLease>> {
        let taint_reason = non_empty(taint_reason, "resource taint reason")?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.mark_resource_tainted(context, resource_kind, resource_id, &taint_reason);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(lease_id) = active_resource_lease_id(
            &tx,
            platform_id,
            company_id,
            project_id,
            resource_kind,
            resource_id,
        )?
        else {
            tx.commit()?;
            return Ok(None);
        };

        tx.execute(
            "UPDATE scheduler_resource_leases
             SET taint_reason = ?1
             WHERE platform_id = ?2 AND company_id = ?3 AND project_id = ?4
               AND id = ?5 AND status = ?6",
            params![
                taint_reason,
                platform_id,
                company_id,
                project_id,
                lease_id.as_str(),
                ResourceLeaseStatus::Active.as_str()
            ],
        )?;

        let stored =
            load_resource_lease_with_conn(&tx, context, &lease_id)?.with_context(|| {
                format!("resource lease disappeared after taint: {}", lease_id.as_str())
            })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn list_stale_resource_leases(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<Vec<StoredResourceLease>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_stale_resource_leases(context, now_ms);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {RESOURCE_LEASE_COLUMNS}
             FROM scheduler_resource_leases
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND status = ?4
               AND expires_at_ms IS NOT NULL
               AND expires_at_ms <= ?5
             ORDER BY expires_at_ms, id"
        ))?;
        let rows = stmt.query_map(
            params![
                platform_id,
                company_id,
                project_id,
                ResourceLeaseStatus::Active.as_str(),
                now_ms.0
            ],
            map_resource_lease_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list stale resource leases")
    }
}

fn validate_acquire_request(request: &AcquireResourceLeaseRequest) -> Result<()> {
    if let Some(lease_expires_at_ms) = request.lease_expires_at_ms
        && lease_expires_at_ms < request.acquired_at_ms
    {
        bail!("resource lease expiry must not be before acquisition time");
    }
    Ok(())
}

fn expire_resource_leases(
    conn: &Connection,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
    resource_kind: ResourceKind,
    resource_id: &ResourceId,
    now_ms: TimestampMs,
) -> Result<()> {
    conn.execute(
        "UPDATE scheduler_resource_leases
         SET status = ?1
         WHERE platform_id = ?2 AND company_id = ?3 AND project_id = ?4
           AND resource_kind = ?5
           AND resource_id = ?6
           AND status = ?7
           AND expires_at_ms IS NOT NULL
           AND expires_at_ms <= ?8",
        params![
            ResourceLeaseStatus::Expired.as_str(),
            platform_id,
            company_id,
            project_id,
            resource_kind.as_str(),
            resource_id.as_str(),
            ResourceLeaseStatus::Active.as_str(),
            now_ms.0,
        ],
    )?;
    Ok(())
}

fn active_resource_lease_exists(
    conn: &Connection,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
    resource_kind: ResourceKind,
    resource_id: &ResourceId,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM scheduler_resource_leases
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
           AND resource_kind = ?4
           AND resource_id = ?5
           AND status = ?6",
        params![
            platform_id,
            company_id,
            project_id,
            resource_kind.as_str(),
            resource_id.as_str(),
            ResourceLeaseStatus::Active.as_str()
        ],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn active_resource_lease_id(
    conn: &Connection,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
    resource_kind: ResourceKind,
    resource_id: &ResourceId,
) -> Result<Option<ResourceLeaseId>> {
    let lease_id = conn
        .query_row(
            "SELECT id
             FROM scheduler_resource_leases
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND resource_kind = ?4
               AND resource_id = ?5
               AND status = ?6
             ORDER BY acquired_at_ms DESC, id DESC
             LIMIT 1",
            params![
                platform_id,
                company_id,
                project_id,
                resource_kind.as_str(),
                resource_id.as_str(),
                ResourceLeaseStatus::Active.as_str(),
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    lease_id
        .map(ResourceLeaseId::new)
        .transpose()
        .with_context(|| format!("parse active resource lease id for {}", resource_id.as_str()))
}

fn load_resource_lease_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    lease_id: &ResourceLeaseId,
) -> Result<Option<StoredResourceLease>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {RESOURCE_LEASE_COLUMNS} FROM scheduler_resource_leases
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(
        params![platform_id, company_id, project_id, lease_id.as_str()],
        map_resource_lease_row,
    )
    .optional()
    .with_context(|| format!("load resource lease: {}", lease_id.as_str()))
}

fn map_resource_lease_row(row: &Row<'_>) -> rusqlite::Result<StoredResourceLease> {
    let resource_kind: String = row.get(1)?;
    let status: String = row.get(4)?;
    let metadata_json: String = row.get(9)?;

    Ok(StoredResourceLease {
        id: parse_resource_lease_id(row.get(0)?)?,
        resource_kind: ResourceKind::parse(&resource_kind)
            .map_err(|error| conversion_error(1, error))?,
        resource_id: parse_resource_id(row.get(2)?)?,
        holder_id: parse_holder_id(row.get(3)?)?,
        status: ResourceLeaseStatus::parse(&status).map_err(|error| conversion_error(4, error))?,
        acquired_at_ms: TimestampMs(row.get(5)?),
        lease_expires_at_ms: row.get::<_, Option<i64>>(6)?.map(TimestampMs),
        released_at_ms: row.get::<_, Option<i64>>(7)?.map(TimestampMs),
        taint_reason: row.get(8)?,
        metadata: serde_json::from_str(&metadata_json)
            .map_err(|error| conversion_error(9, anyhow::Error::new(error)))?,
    })
}

fn parse_resource_lease_id(value: String) -> rusqlite::Result<ResourceLeaseId> {
    ResourceLeaseId::new(value).map_err(|error| conversion_error(0, error))
}

fn parse_resource_id(value: String) -> rusqlite::Result<ResourceId> {
    ResourceId::new(value).map_err(|error| conversion_error(2, error))
}

fn parse_holder_id(value: String) -> rusqlite::Result<HolderId> {
    HolderId::new(value).map_err(|error| conversion_error(3, error))
}

fn non_empty(value: String, label: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(value)
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

#[cfg(test)]
#[path = "resources_unit_tests.rs"]
mod unit_tests;
