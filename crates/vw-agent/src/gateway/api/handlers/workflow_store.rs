use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::workflow::{
    WorkflowRecord, WorkflowRecordDeleteResponse, WorkflowRecordSummary, WorkflowRecordUpsertBody,
};

use crate::app::agent::gateway::ApiError;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

mod postgres;

use postgres::PostgresWorkflowStore;

const WORKFLOW_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_workflows (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    workflow_yaml TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (platform_id, company_id, uuid)
);
"#;

pub(super) async fn list_records(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Vec<WorkflowRecordSummary>, ApiError> {
    spawn_store_task(database, move |database| list_records_blocking(database, &tenant)).await
}

pub(super) async fn get_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
) -> Result<WorkflowRecord, ApiError> {
    spawn_store_task(database, move |database| get_record_blocking(database, &tenant, &uuid)).await
}

pub(super) async fn create_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, ApiError> {
    spawn_store_task(database, move |database| {
        let uuid = body.uuid.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        upsert_record_blocking(database, &tenant, &uuid, &body)
    })
    .await
}

pub(super) async fn update_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
    body: WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, ApiError> {
    spawn_store_task(database, move |database| {
        upsert_record_blocking(database, &tenant, &uuid, &body)
    })
    .await
}

pub(super) async fn delete_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
) -> Result<WorkflowRecordDeleteResponse, ApiError> {
    spawn_store_task(database, move |database| delete_record_blocking(database, &tenant, &uuid))
        .await
}

async fn spawn_store_task<T: Send + 'static>(
    database: GlobalDatabase,
    task: impl FnOnce(GlobalDatabase) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || task(database))
        .await
        .map_err(|error| ApiError::internal(format!("workflow store task failed: {error}")))?
}

fn list_records_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
) -> Result<Vec<WorkflowRecordSummary>, ApiError> {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            let mut stmt = conn
                .prepare(
                    "SELECT uuid, name, description, created_at_ms, updated_at_ms \
                     FROM workflow_workflows \
                     WHERE platform_id = ?1 AND company_id = ?2 \
                     ORDER BY updated_at_ms DESC, name ASC",
                )
                .map_err(sql_error)?;

            let rows = stmt
                .query_map(
                    params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                    |row| {
                        Ok(WorkflowRecordSummary {
                            uuid: row.get(0)?,
                            name: row.get(1)?,
                            description: row.get(2)?,
                            created_at_ms: read_u64(row, 3)?,
                            updated_at_ms: read_u64(row, 4)?,
                        })
                    },
                )
                .map_err(sql_error)?;

            rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
        }
        DatabaseProvider::Postgres => PostgresWorkflowStore::new(database)
            .and_then(|store| store.list_records(tenant))
            .map_err(database_error),
    }
}

fn get_record_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
) -> Result<WorkflowRecord, ApiError> {
    validate_uuid(uuid)?;
    let record = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            query_record(&conn, tenant, uuid)?
        }
        DatabaseProvider::Postgres => PostgresWorkflowStore::new(database)
            .and_then(|store| store.query_record(tenant, uuid))
            .map_err(database_error)?,
    };
    record.ok_or_else(|| ApiError::not_found(format!("workflow record not found: {uuid}")))
}

fn upsert_record_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
    body: &WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, ApiError> {
    validate_uuid(uuid)?;
    validate_body(body)?;
    let now = now_ms();
    let record = match database.provider() {
        DatabaseProvider::Sqlite => {
            let conn = open_db(&database)?;
            let created_at_ms = query_record(&conn, tenant, uuid)?
                .map(|record| record.created_at_ms)
                .unwrap_or(now);

            conn.execute(
                "INSERT INTO workflow_workflows \
                 (platform_id, company_id, uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(platform_id, company_id, uuid) DO UPDATE SET \
                 name = excluded.name, \
                 description = excluded.description, \
                 workflow_yaml = excluded.workflow_yaml, \
                 updated_at_ms = excluded.updated_at_ms",
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    uuid,
                    body.name.trim(),
                    body.description.trim(),
                    body.workflow_yaml.as_str(),
                    created_at_ms,
                    now,
                ],
            )
            .map_err(sql_error)?;

            query_record(&conn, tenant, uuid)?
        }
        DatabaseProvider::Postgres => PostgresWorkflowStore::new(database)
            .and_then(|store| store.upsert_record(tenant, uuid, body, now))
            .map_err(database_error)?,
    };
    record.ok_or_else(|| ApiError::internal("workflow record was not saved"))
}

fn delete_record_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
) -> Result<WorkflowRecordDeleteResponse, ApiError> {
    validate_uuid(uuid)?;
    let changed = match database.provider() {
        DatabaseProvider::Sqlite => {
            open_db(&database)?
                .execute(
                    "DELETE FROM workflow_workflows \
                 WHERE platform_id = ?1 AND company_id = ?2 AND uuid = ?3",
                    params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), uuid],
                )
                .map_err(sql_error)?
                > 0
        }
        DatabaseProvider::Postgres => PostgresWorkflowStore::new(database)
            .and_then(|store| store.delete_record(tenant, uuid))
            .map_err(database_error)?,
    };

    Ok(WorkflowRecordDeleteResponse { uuid: uuid.to_string(), deleted: changed })
}

fn query_record(
    conn: &Connection,
    tenant: &TenantScope,
    uuid: &str,
) -> Result<Option<WorkflowRecord>, ApiError> {
    conn.query_row(
        "SELECT uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms \
         FROM workflow_workflows \
         WHERE platform_id = ?1 AND company_id = ?2 AND uuid = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), uuid],
        |row| {
            Ok(WorkflowRecord {
                uuid: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                workflow_yaml: row.get(3)?,
                created_at_ms: read_u64(row, 4)?,
                updated_at_ms: read_u64(row, 5)?,
            })
        },
    )
    .optional()
    .map_err(sql_error)
}

fn open_db(database: &GlobalDatabase) -> Result<Connection, ApiError> {
    database
        .initialize_schema(|connection| {
            reject_unscoped_workflow_schema(connection)?;
            connection.execute_batch(WORKFLOW_SCHEMA)?;
            Ok(())
        })
        .map_err(database_error)?;
    database.connect().map_err(database_error)
}

fn reject_unscoped_workflow_schema(conn: &Connection) -> anyhow::Result<()> {
    let table = "workflow_workflows";
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
            "unsupported unscoped workflow schema: table {table} must contain platform_id and company_id"
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

fn validate_uuid(uuid: &str) -> Result<(), ApiError> {
    Uuid::parse_str(uuid).map(|_| ()).map_err(|_| ApiError::bad_request("workflow uuid is invalid"))
}

fn validate_body(body: &WorkflowRecordUpsertBody) -> Result<(), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("workflow name is required"));
    }
    if body.workflow_yaml.trim().is_empty() {
        return Err(ApiError::bad_request("workflow_yaml is required"));
    }
    Ok(())
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
    ApiError::internal(format!("workflow database error: {error}"))
}

fn database_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(format!("workflow database error: {error:#}"))
}

#[cfg(test)]
#[path = "workflow_store_tests.rs"]
mod workflow_store_tests;
