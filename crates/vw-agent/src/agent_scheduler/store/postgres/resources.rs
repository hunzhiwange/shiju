use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{HolderId, ResourceId, ResourceLeaseId};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::resources::{
    AcquireResourceLeaseRequest, ResourceKind, ResourceLeaseStatus, StoredResourceLease,
};

const RESOURCE_LEASE_COLUMNS: &str = "id, resource_kind, resource_id, holder_id, status, \
acquired_at_ms, expires_at_ms, released_at_ms, taint_reason, metadata_json";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn acquire_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: AcquireResourceLeaseRequest,
    ) -> Result<Option<StoredResourceLease>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL resource lease acquisition")?;
            transaction
                .query_opt(
                    "SELECT id
                 FROM scheduler_resource_leases
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND resource_kind = $4 AND resource_id = $5 AND status = $6
                 ORDER BY acquired_at_ms DESC, id DESC
                 LIMIT 1
                 FOR UPDATE",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.resource_kind.as_str(),
                        &request.resource_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                    ],
                )
                .context("lock active scoped PostgreSQL resource lease")?;
            transaction
                .execute(
                    "UPDATE scheduler_resource_leases
                 SET status = $1
                 WHERE platform_id = $2 AND company_id = $3 AND project_id = $4
                   AND resource_kind = $5 AND resource_id = $6 AND status = $7
                   AND expires_at_ms IS NOT NULL AND expires_at_ms <= $8",
                    &[
                        &ResourceLeaseStatus::Expired.as_str(),
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.resource_kind.as_str(),
                        &request.resource_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                        &request.acquired_at_ms.0,
                    ],
                )
                .context("expire scoped PostgreSQL resource lease")?;
            let inserted = transaction
                .query_opt(
                    "INSERT INTO scheduler_resource_leases (
                     platform_id, company_id, project_id, id, resource_kind, resource_id,
                     holder_id, status, acquired_at_ms, expires_at_ms, metadata_json
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (platform_id, company_id, project_id, resource_kind, resource_id)
                 WHERE status = 'active'
                 DO NOTHING
                 RETURNING id",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.lease_id.as_str(),
                        &request.resource_kind.as_str(),
                        &request.resource_id.as_str(),
                        &request.holder_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                        &request.acquired_at_ms.0,
                        &request.lease_expires_at_ms.map(|value| value.0),
                        &"{}",
                    ],
                )
                .context("insert scoped PostgreSQL resource lease")?;
            if inserted.is_none() {
                transaction
                    .commit()
                    .context("commit conflicted PostgreSQL resource lease acquisition")?;
                return Ok(None);
            }
            let stored =
                load_resource_lease_with_client(&mut transaction, &scope, &request.lease_id)?
                    .with_context(|| {
                        format!(
                            "PostgreSQL resource lease disappeared after acquisition: {}",
                            request.lease_id.as_str()
                        )
                    })?;
            transaction.commit().context("commit PostgreSQL resource lease acquisition")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn release_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        lease_id: &ResourceLeaseId,
        holder_id: &HolderId,
        released_at_ms: TimestampMs,
    ) -> Result<Option<StoredResourceLease>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL resource lease release")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_resource_leases
                 SET status = $1, released_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND project_id = $5
                   AND id = $6 AND holder_id = $7 AND status = $8",
                    &[
                        &ResourceLeaseStatus::Released.as_str(),
                        &released_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &lease_id.as_str(),
                        &holder_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                    ],
                )
                .context("release scoped PostgreSQL resource lease")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => bail!(
                    "PostgreSQL resource lease release affected {changed} rows: {}",
                    lease_id.as_str()
                ),
            }
            let stored = load_resource_lease_with_client(&mut transaction, &scope, lease_id)?
                .with_context(|| {
                    format!(
                        "PostgreSQL resource lease disappeared after release: {}",
                        lease_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL resource lease release")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn renew_resource_lease(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        lease_id: &ResourceLeaseId,
        holder_id: &HolderId,
        renewed_at_ms: TimestampMs,
        lease_expires_at_ms: TimestampMs,
    ) -> Result<Option<StoredResourceLease>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL resource lease renewal")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_resource_leases
                 SET expires_at_ms = $1
                 WHERE platform_id = $2 AND company_id = $3 AND project_id = $4
                   AND id = $5 AND holder_id = $6 AND status = $7
                   AND (expires_at_ms IS NULL OR expires_at_ms > $8)",
                    &[
                        &lease_expires_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &lease_id.as_str(),
                        &holder_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                        &renewed_at_ms.0,
                    ],
                )
                .context("renew scoped PostgreSQL resource lease")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => bail!(
                    "PostgreSQL resource lease renewal affected {changed} rows: {}",
                    lease_id.as_str()
                ),
            }
            let stored = load_resource_lease_with_client(&mut transaction, &scope, lease_id)?
                .with_context(|| {
                    format!(
                        "PostgreSQL resource lease disappeared after renewal: {}",
                        lease_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL resource lease renewal")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn mark_resource_tainted(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        resource_kind: ResourceKind,
        resource_id: &ResourceId,
        taint_reason: &str,
    ) -> Result<Option<StoredResourceLease>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL resource taint")?;
            let row = transaction
                .query_opt(
                    "UPDATE scheduler_resource_leases
                 SET taint_reason = $1
                 WHERE platform_id = $2 AND company_id = $3 AND project_id = $4
                   AND resource_kind = $5 AND resource_id = $6 AND status = $7
                 RETURNING id",
                    &[
                        &taint_reason,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &resource_kind.as_str(),
                        &resource_id.as_str(),
                        &ResourceLeaseStatus::Active.as_str(),
                    ],
                )
                .context("taint scoped PostgreSQL resource lease")?;
            let Some(row) = row else {
                return Ok(None);
            };
            let lease_id: String = row.try_get(0).context("read tainted PostgreSQL lease id")?;
            let lease_id =
                ResourceLeaseId::new(lease_id).context("parse tainted PostgreSQL lease id")?;
            let stored = load_resource_lease_with_client(&mut transaction, &scope, &lease_id)?
                .with_context(|| {
                    format!(
                        "PostgreSQL resource lease disappeared after taint: {}",
                        lease_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL resource taint")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn list_stale_resource_leases(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<Vec<StoredResourceLease>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {RESOURCE_LEASE_COLUMNS}
                     FROM scheduler_resource_leases
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND status = $4 AND expires_at_ms IS NOT NULL AND expires_at_ms <= $5
                     ORDER BY expires_at_ms, id"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &ResourceLeaseStatus::Active.as_str(),
                        &now_ms.0,
                    ],
                )
                .context("list scoped stale PostgreSQL resource leases")?
                .iter()
                .map(map_resource_lease_row)
                .collect()
        })
    }
}

fn load_resource_lease_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    lease_id: &ResourceLeaseId,
) -> Result<Option<StoredResourceLease>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {RESOURCE_LEASE_COLUMNS} FROM scheduler_resource_leases
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &lease_id.as_str(),
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL resource lease: {}", lease_id.as_str()))?
        .as_ref()
        .map(map_resource_lease_row)
        .transpose()
}

fn map_resource_lease_row(row: &Row) -> Result<StoredResourceLease> {
    let id: String = row.try_get(0).context("read PostgreSQL resource lease id")?;
    let resource_kind: String = row.try_get(1).context("read PostgreSQL resource kind")?;
    let resource_id: String = row.try_get(2).context("read PostgreSQL resource id")?;
    let holder_id: String = row.try_get(3).context("read PostgreSQL resource holder")?;
    let status: String = row.try_get(4).context("read PostgreSQL resource lease status")?;
    let metadata_json: String = row.try_get(9).context("read PostgreSQL resource metadata")?;
    Ok(StoredResourceLease {
        id: ResourceLeaseId::new(id).context("parse PostgreSQL resource lease id")?,
        resource_kind: ResourceKind::parse(&resource_kind)
            .context("parse PostgreSQL resource kind")?,
        resource_id: ResourceId::new(resource_id).context("parse PostgreSQL resource id")?,
        holder_id: HolderId::new(holder_id).context("parse PostgreSQL resource holder")?,
        status: ResourceLeaseStatus::parse(&status)
            .context("parse PostgreSQL resource lease status")?,
        acquired_at_ms: TimestampMs(
            row.try_get(5).context("read PostgreSQL resource acquisition")?,
        ),
        lease_expires_at_ms: row.try_get::<_, Option<i64>>(6)?.map(TimestampMs),
        released_at_ms: row.try_get::<_, Option<i64>>(7)?.map(TimestampMs),
        taint_reason: row.try_get(8).context("read PostgreSQL resource taint")?,
        metadata: serde_json::from_str(&metadata_json)
            .context("parse PostgreSQL resource metadata")?,
    })
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
