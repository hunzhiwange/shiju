use anyhow::{Context, Result, bail};
use vw_api_types::TenantScope;
use vw_api_types::task_lifecycle::TaskLifecycleWorkflowDto;

use super::StoredWorkflow;
use crate::global_database::GlobalDatabase;

const TASK_LIFECYCLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_lifecycle_workflows (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    label TEXT NOT NULL,
    workflow_json TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, id)
);
"#;

#[derive(Debug, Clone)]
pub(super) struct PostgresTaskLifecycleStore {
    database: GlobalDatabase,
}

impl PostgresTaskLifecycleStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            reject_unscoped_lifecycle_schema(transaction)?;
            transaction
                .batch_execute(TASK_LIFECYCLE_SCHEMA)
                .context("initialize PostgreSQL task lifecycle schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn query_custom_workflows(
        &self,
        tenant: &TenantScope,
    ) -> Result<Vec<TaskLifecycleWorkflowDto>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT workflow_json FROM workflow_lifecycle_workflows
                 WHERE platform_id = $1 AND company_id = $2
                 ORDER BY updated_at_ms DESC, label ASC",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("list tenant PostgreSQL task lifecycle workflows")?
            .into_iter()
            .map(|row| {
                let workflow_json: String =
                    row.try_get(0).context("read PostgreSQL task lifecycle workflow JSON")?;
                serde_json::from_str(&workflow_json)
                    .context("decode PostgreSQL task lifecycle workflow JSON")
            })
            .collect()
    }

    pub(super) fn query_custom_workflow(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<StoredWorkflow>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query_opt(
                "SELECT workflow_json, created_at_ms FROM workflow_lifecycle_workflows
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &id],
            )
            .context("load tenant PostgreSQL task lifecycle workflow")?
            .map(|row| {
                let workflow_json: String =
                    row.try_get(0).context("read PostgreSQL task lifecycle workflow JSON")?;
                let workflow = serde_json::from_str(&workflow_json)
                    .context("decode PostgreSQL task lifecycle workflow JSON")?;
                let created_at_ms: i64 =
                    row.try_get(1).context("read PostgreSQL task lifecycle creation timestamp")?;
                Ok(StoredWorkflow {
                    workflow,
                    created_at_ms: u64::try_from(created_at_ms).unwrap_or_default(),
                })
            })
            .transpose()
    }

    pub(super) fn save_custom_workflow(
        &self,
        tenant: &TenantScope,
        workflow: &TaskLifecycleWorkflowDto,
        created_at_ms: u64,
        updated_at_ms: u64,
    ) -> Result<()> {
        let workflow_json = serde_json::to_string(workflow)
            .context("encode PostgreSQL task lifecycle workflow JSON")?;
        let created_at_ms = i64::try_from(created_at_ms).unwrap_or(i64::MAX);
        let updated_at_ms = i64::try_from(updated_at_ms).unwrap_or(i64::MAX);
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "INSERT INTO workflow_lifecycle_workflows
                 (platform_id, company_id, id, label, workflow_json, created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (platform_id, company_id, id) DO UPDATE SET
                 label = excluded.label,
                 workflow_json = excluded.workflow_json,
                 updated_at_ms = excluded.updated_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &workflow.id.as_str(),
                    &workflow.label.as_str(),
                    &workflow_json,
                    &created_at_ms,
                    &updated_at_ms,
                ],
            )
            .context("upsert tenant PostgreSQL task lifecycle workflow")?;
        if changed != 1 {
            bail!("PostgreSQL task lifecycle workflow upsert affected an unexpected row count");
        }
        Ok(())
    }

    pub(super) fn delete_custom_workflow(&self, tenant: &TenantScope, id: &str) -> Result<usize> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "DELETE FROM workflow_lifecycle_workflows
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &id],
            )
            .context("delete tenant PostgreSQL task lifecycle workflow")?;
        usize::try_from(changed).context("convert PostgreSQL task lifecycle delete row count")
    }
}

fn reject_unscoped_lifecycle_schema(connection: &mut impl ::postgres::GenericClient) -> Result<()> {
    let table = "workflow_lifecycle_workflows";
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
        .context("inspect PostgreSQL task lifecycle table")?
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
            .context("inspect PostgreSQL task lifecycle scope columns")?
            .get(0);
        if !column_exists {
            bail!(
                "unsupported unscoped task lifecycle schema: table {table} must contain platform_id and company_id"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;
