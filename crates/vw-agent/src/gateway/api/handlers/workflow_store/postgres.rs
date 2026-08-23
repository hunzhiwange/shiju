//! PostgreSQL 工作流记录存储，强制租户作用域并维护时间戳。

use anyhow::{Context, Result, bail};
use vw_api_types::TenantScope;
use vw_api_types::workflow::{WorkflowRecord, WorkflowRecordSummary, WorkflowRecordUpsertBody};

use crate::global_database::GlobalDatabase;

const WORKFLOW_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_workflows (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    workflow_yaml TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, uuid)
);
"#;

#[derive(Debug, Clone)]
pub(super) struct PostgresWorkflowStore {
    database: GlobalDatabase,
}

impl PostgresWorkflowStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            reject_unscoped_workflow_schema(transaction)?;
            transaction
                .batch_execute(WORKFLOW_SCHEMA)
                .context("initialize PostgreSQL workflow schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn list_records(&self, tenant: &TenantScope) -> Result<Vec<WorkflowRecordSummary>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT uuid, name, description, created_at_ms, updated_at_ms
                 FROM workflow_workflows
                 WHERE platform_id = $1 AND company_id = $2
                 ORDER BY updated_at_ms DESC, name ASC",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("list tenant PostgreSQL workflow records")?
            .into_iter()
            .map(|row| {
                let created_at_ms: i64 =
                    row.try_get(3).context("read PostgreSQL workflow creation timestamp")?;
                let updated_at_ms: i64 =
                    row.try_get(4).context("read PostgreSQL workflow update timestamp")?;
                Ok(WorkflowRecordSummary {
                    uuid: row.try_get(0).context("read PostgreSQL workflow UUID")?,
                    name: row.try_get(1).context("read PostgreSQL workflow name")?,
                    description: row.try_get(2).context("read PostgreSQL workflow description")?,
                    created_at_ms: u64::try_from(created_at_ms).unwrap_or_default(),
                    updated_at_ms: u64::try_from(updated_at_ms).unwrap_or_default(),
                })
            })
            .collect()
    }

    pub(super) fn query_record(
        &self,
        tenant: &TenantScope,
        uuid: &str,
    ) -> Result<Option<WorkflowRecord>> {
        let mut connection = self.database.connect_postgres()?;
        query_record(&mut *connection, tenant, uuid)
    }

    pub(super) fn upsert_record(
        &self,
        tenant: &TenantScope,
        uuid: &str,
        body: &WorkflowRecordUpsertBody,
        now_ms: u64,
    ) -> Result<Option<WorkflowRecord>> {
        let mut connection = self.database.connect_postgres()?;
        let created_at_ms = query_record(&mut *connection, tenant, uuid)?
            .map(|record| record.created_at_ms)
            .unwrap_or(now_ms);
        let created_at_ms = i64::try_from(created_at_ms).unwrap_or(i64::MAX);
        let updated_at_ms = i64::try_from(now_ms).unwrap_or(i64::MAX);
        let changed = connection
            .execute(
                "INSERT INTO workflow_workflows
                 (platform_id, company_id, uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (platform_id, company_id, uuid) DO UPDATE SET
                 name = excluded.name,
                 description = excluded.description,
                 workflow_yaml = excluded.workflow_yaml,
                 updated_at_ms = excluded.updated_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &uuid,
                    &body.name.trim(),
                    &body.description.trim(),
                    &body.workflow_yaml.as_str(),
                    &created_at_ms,
                    &updated_at_ms,
                ],
            )
            .context("upsert tenant PostgreSQL workflow record")?;
        if changed != 1 {
            bail!("PostgreSQL workflow upsert affected an unexpected row count");
        }
        query_record(&mut *connection, tenant, uuid)
    }

    pub(super) fn delete_record(&self, tenant: &TenantScope, uuid: &str) -> Result<bool> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "DELETE FROM workflow_workflows
                 WHERE platform_id = $1 AND company_id = $2 AND uuid = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &uuid],
            )
            .context("delete tenant PostgreSQL workflow record")?;
        Ok(changed > 0)
    }
}

fn query_record(
    connection: &mut impl ::postgres::GenericClient,
    tenant: &TenantScope,
    uuid: &str,
) -> Result<Option<WorkflowRecord>> {
    connection
        .query_opt(
            "SELECT uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms
             FROM workflow_workflows
             WHERE platform_id = $1 AND company_id = $2 AND uuid = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &uuid],
        )
        .context("load tenant PostgreSQL workflow record")?
        .map(|row| {
            let created_at_ms: i64 =
                row.try_get(4).context("read PostgreSQL workflow creation timestamp")?;
            let updated_at_ms: i64 =
                row.try_get(5).context("read PostgreSQL workflow update timestamp")?;
            Ok(WorkflowRecord {
                uuid: row.try_get(0).context("read PostgreSQL workflow UUID")?,
                name: row.try_get(1).context("read PostgreSQL workflow name")?,
                description: row.try_get(2).context("read PostgreSQL workflow description")?,
                workflow_yaml: row.try_get(3).context("read PostgreSQL workflow YAML")?,
                created_at_ms: u64::try_from(created_at_ms).unwrap_or_default(),
                updated_at_ms: u64::try_from(updated_at_ms).unwrap_or_default(),
            })
        })
        .transpose()
}

fn reject_unscoped_workflow_schema(connection: &mut impl ::postgres::GenericClient) -> Result<()> {
    let table = "workflow_workflows";
    let exists: bool = connection
        .query_one(
            "SELECT EXISTS(
                 SELECT 1 FROM information_schema.tables
                 WHERE table_schema = current_schema()
                   AND table_name = $1
                   AND table_type = 'BASE TABLE'
             )",
            &[&table],
        )
        .context("inspect PostgreSQL workflow table")?
        .get(0);
    if !exists {
        return Ok(());
    }
    for column in ["platform_id", "company_id"] {
        let column_exists: bool = connection
            .query_one(
                "SELECT EXISTS(
                     SELECT 1 FROM information_schema.columns
                     WHERE table_schema = current_schema()
                       AND table_name = $1
                       AND column_name = $2
                 )",
                &[&table, &column],
            )
            .context("inspect PostgreSQL workflow scope columns")?
            .get(0);
        if !column_exists {
            bail!(
                "unsupported unscoped workflow schema: table {table} must contain platform_id and company_id"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;
