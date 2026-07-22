use rusqlite::{Connection, OptionalExtension, params};
use vw_api_types::TenantScope;
use vw_api_types::task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

use crate::app::agent::gateway::ApiError;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

use super::{TaskLifecycleRegistry, built_in_workflow_by_id, is_built_in_workflow_id};

mod postgres;

use postgres::PostgresTaskLifecycleStore;

const TASK_LIFECYCLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_lifecycle_workflows (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    label TEXT NOT NULL,
    workflow_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, id)
);
"#;

pub(crate) async fn list_workflows(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Vec<TaskLifecycleWorkflowDto>, ApiError> {
    spawn_store_task(database, move |database| list_workflows_blocking(database, &tenant)).await
}

pub(crate) async fn workflow_by_id(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    spawn_store_task(database, move |database| workflow_by_id_blocking(database, &tenant, &id))
        .await
}

pub(crate) async fn create_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    spawn_store_task(database, move |database| create_workflow_blocking(database, &tenant, body))
        .await
}

pub(crate) async fn update_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    spawn_store_task(database, move |database| {
        update_workflow_blocking(database, &tenant, &id, body)
    })
    .await
}

pub(crate) async fn delete_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDeleteResponse, ApiError> {
    spawn_store_task(database, move |database| delete_workflow_blocking(database, &tenant, &id))
        .await
}

pub(crate) async fn reset_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    spawn_store_task(database, move |database| reset_workflow_blocking(database, &tenant, &id))
        .await
}

async fn spawn_store_task<T: Send + 'static>(
    database: GlobalDatabase,
    task: impl FnOnce(GlobalDatabase) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || task(database))
        .await
        .map_err(|error| ApiError::internal(format!("task lifecycle store task failed: {error}")))?
}

fn list_workflows_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
) -> Result<Vec<TaskLifecycleWorkflowDto>, ApiError> {
    let workflows = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            query_custom_workflows(&conn, tenant)?
        }
        DatabaseProvider::Postgres => PostgresTaskLifecycleStore::new(database)
            .and_then(|store| store.query_custom_workflows(tenant))
            .map_err(database_error)?,
    };
    let registry = registry_from_custom_workflows(workflows)?;
    Ok(registry.all_workflows())
}

fn workflow_by_id_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    id: &str,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    let workflows = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            query_custom_workflows(&conn, tenant)?
        }
        DatabaseProvider::Postgres => PostgresTaskLifecycleStore::new(database)
            .and_then(|store| store.query_custom_workflows(tenant))
            .map_err(database_error)?,
    };
    let registry = registry_from_custom_workflows(workflows)?;
    registry
        .workflow_by_id(id)
        .ok_or_else(|| ApiError::not_found(format!("task lifecycle workflow not found: {id}")))
}

fn create_workflow_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    let workflow = normalize_custom_workflow(body.workflow)?;
    let existing = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            query_custom_workflow(&conn, tenant, workflow.id.as_str())?
        }
        DatabaseProvider::Postgres => PostgresTaskLifecycleStore::new(database.clone())
            .and_then(|store| store.query_custom_workflow(tenant, workflow.id.as_str()))
            .map_err(database_error)?,
    };
    if existing.is_some() {
        return Err(ApiError::bad_request(format!(
            "custom lifecycle workflow id already exists: {}",
            workflow.id
        )));
    }
    let stored = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            save_custom_workflow(&conn, tenant, &workflow, now_ms(), now_ms())?;
            query_custom_workflow(&conn, tenant, workflow.id.as_str())?
        }
        DatabaseProvider::Postgres => {
            let store = PostgresTaskLifecycleStore::new(database).map_err(database_error)?;
            store
                .save_custom_workflow(tenant, &workflow, now_ms(), now_ms())
                .and_then(|()| store.query_custom_workflow(tenant, workflow.id.as_str()))
                .map_err(database_error)?
        }
    };
    stored
        .map(|row| row.workflow)
        .ok_or_else(|| ApiError::internal("task lifecycle workflow was not saved"))
}

fn update_workflow_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    id: &str,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    if body.workflow.id != id {
        return Err(ApiError::bad_request(format!("task lifecycle workflow id mismatch: {id}")));
    }
    let workflow = normalize_custom_workflow(body.workflow)?;
    let existing = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            query_custom_workflow(&conn, tenant, id)?
        }
        DatabaseProvider::Postgres => PostgresTaskLifecycleStore::new(database.clone())
            .and_then(|store| store.query_custom_workflow(tenant, id))
            .map_err(database_error)?,
    };
    let created_at_ms = match existing {
        Some(existing) => existing.created_at_ms,
        None if is_built_in_workflow_id(id) => now_ms(),
        None => {
            return Err(ApiError::not_found(format!("task lifecycle workflow not found: {id}")));
        }
    };
    let stored = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            save_custom_workflow(&conn, tenant, &workflow, created_at_ms, now_ms())?;
            query_custom_workflow(&conn, tenant, id)?
        }
        DatabaseProvider::Postgres => {
            let store = PostgresTaskLifecycleStore::new(database).map_err(database_error)?;
            store
                .save_custom_workflow(tenant, &workflow, created_at_ms, now_ms())
                .and_then(|()| store.query_custom_workflow(tenant, id))
                .map_err(database_error)?
        }
    };
    stored
        .map(|row| row.workflow)
        .ok_or_else(|| ApiError::internal("task lifecycle workflow was not saved"))
}

fn delete_workflow_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    id: &str,
) -> Result<TaskLifecycleWorkflowDeleteResponse, ApiError> {
    reject_built_in_write(id)?;
    let changed = match database.provider() {
        DatabaseProvider::Sqlite => open_db(&database)?
            .execute(
                "DELETE FROM workflow_lifecycle_workflows \
                 WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), id],
            )
            .map_err(sql_error)?,
        DatabaseProvider::Postgres => PostgresTaskLifecycleStore::new(database)
            .and_then(|store| store.delete_custom_workflow(tenant, id))
            .map_err(database_error)?,
    };
    Ok(TaskLifecycleWorkflowDeleteResponse { id: id.to_string(), deleted: changed > 0 })
}

fn reset_workflow_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    id: &str,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    if !is_built_in_workflow_id(id) {
        return Err(ApiError::bad_request(format!(
            "task lifecycle workflow reset is only supported for built-in workflow: {id}"
        )));
    }
    match database.provider() {
        DatabaseProvider::Sqlite => {
            open_db(&database)?
                .execute(
                    "DELETE FROM workflow_lifecycle_workflows \
                     WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
                    params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), id],
                )
                .map_err(sql_error)?;
        }
        DatabaseProvider::Postgres => {
            PostgresTaskLifecycleStore::new(database)
                .and_then(|store| store.delete_custom_workflow(tenant, id))
                .map_err(database_error)?;
        }
    }
    built_in_workflow_by_id(id)
        .ok_or_else(|| ApiError::internal(format!("built-in lifecycle workflow missing: {id}")))
}

fn registry_from_custom_workflows(
    workflows: Vec<TaskLifecycleWorkflowDto>,
) -> Result<TaskLifecycleRegistry, ApiError> {
    TaskLifecycleRegistry::from_custom_workflows(workflows).map_err(|error| {
        ApiError::internal(format!("stored task lifecycle workflow invalid: {error}"))
    })
}

fn normalize_custom_workflow(
    mut workflow: TaskLifecycleWorkflowDto,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    workflow.label = workflow.label.trim().to_string();
    let mut registry = TaskLifecycleRegistry::new();
    registry.register(workflow).map_err(|error| ApiError::bad_request(error.to_string()))?;
    registry.custom_workflows().first().cloned().ok_or_else(|| {
        ApiError::internal("task lifecycle workflow validation produced no workflow")
    })
}

fn reject_built_in_write(id: &str) -> Result<(), ApiError> {
    if is_built_in_workflow_id(id) {
        return Err(ApiError::bad_request(format!(
            "built-in lifecycle workflow is read-only: {id}"
        )));
    }
    Ok(())
}

fn query_custom_workflows(
    conn: &Connection,
    tenant: &TenantScope,
) -> Result<Vec<TaskLifecycleWorkflowDto>, ApiError> {
    let mut stmt = conn
        .prepare(
            "SELECT workflow_json FROM workflow_lifecycle_workflows \
             WHERE platform_id = ?1 AND company_id = ?2 \
             ORDER BY updated_at_ms DESC, label ASC",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map(params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(sql_error)?;
    rows.map(|row| {
        let workflow_json = row.map_err(sql_error)?;
        serde_json::from_str(&workflow_json).map_err(json_error)
    })
    .collect()
}

fn query_custom_workflow(
    conn: &Connection,
    tenant: &TenantScope,
    id: &str,
) -> Result<Option<StoredWorkflow>, ApiError> {
    conn.query_row(
        "SELECT workflow_json, created_at_ms FROM workflow_lifecycle_workflows \
         WHERE platform_id = ?1 AND company_id = ?2 AND id = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), id],
        |row| {
            let workflow_json: String = row.get(0)?;
            let workflow = serde_json::from_str(&workflow_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(StoredWorkflow { workflow, created_at_ms: read_u64(row, 1)? })
        },
    )
    .optional()
    .map_err(sql_error)
}

fn save_custom_workflow(
    conn: &Connection,
    tenant: &TenantScope,
    workflow: &TaskLifecycleWorkflowDto,
    created_at_ms: u64,
    updated_at_ms: u64,
) -> Result<(), ApiError> {
    let workflow_json = serde_json::to_string(workflow).map_err(json_error)?;
    conn.execute(
        "INSERT INTO workflow_lifecycle_workflows \
         (platform_id, company_id, id, label, workflow_json, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT(platform_id, company_id, id) DO UPDATE SET \
         label = excluded.label, \
         workflow_json = excluded.workflow_json, \
         updated_at_ms = excluded.updated_at_ms",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            workflow.id.as_str(),
            workflow.label.as_str(),
            workflow_json,
            created_at_ms,
            updated_at_ms,
        ],
    )
    .map_err(sql_error)?;
    Ok(())
}

fn open_db(database: &GlobalDatabase) -> Result<Connection, ApiError> {
    database
        .initialize_schema(|connection| {
            reject_unscoped_lifecycle_schema(connection)?;
            connection.execute_batch(TASK_LIFECYCLE_SCHEMA)?;
            Ok(())
        })
        .map_err(database_error)?;
    database.connect().map_err(database_error)
}

fn reject_unscoped_lifecycle_schema(conn: &Connection) -> anyhow::Result<()> {
    let table = "workflow_lifecycle_workflows";
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )?;
    if exists
        && (!column_exists(conn, table, "platform_id")?
            || !column_exists(conn, table, "company_id")?)
    {
        anyhow::bail!(
            "unsupported unscoped task lifecycle schema: table {table} must contain platform_id and company_id"
        );
    }
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    Ok(u64::try_from(value).unwrap_or_default())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn sql_error(error: rusqlite::Error) -> ApiError {
    ApiError::internal(format!("task lifecycle database error: {error}"))
}

fn database_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(format!("task lifecycle database error: {error:#}"))
}

fn json_error(error: serde_json::Error) -> ApiError {
    ApiError::internal(format!("task lifecycle json error: {error}"))
}

#[derive(Debug)]
struct StoredWorkflow {
    workflow: TaskLifecycleWorkflowDto,
    created_at_ms: u64,
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
