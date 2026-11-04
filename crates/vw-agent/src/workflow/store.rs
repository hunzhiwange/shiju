//! 工作流记录存储门面，按租户路由 SQLite 或 PostgreSQL 持久化操作。
//!
//! 异步入口把阻塞式数据库工作移入隔离线程，并在写入前拒绝未带租户列的旧表结构。

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::workflow::{
    WorkflowRecord, WorkflowRecordDeleteResponse, WorkflowRecordSummary, WorkflowRecordUpsertBody,
};

mod error;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
pub use error::WorkflowStoreError;

mod postgres;
mod template_records;
pub(super) use template_records::write_template_record;

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

/// 按更新时间倒序列出租户工作流摘要。
pub(crate) async fn list_records(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Vec<WorkflowRecordSummary>, WorkflowStoreError> {
    spawn_store_task(database, move |database| list_records_blocking(database, &tenant)).await
}

/// 读取租户内指定 UUID 的完整工作流。
pub(crate) async fn get_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    spawn_store_task(database, move |database| get_record_blocking(database, &tenant, &uuid)).await
}

/// 创建工作流；请求未提供 UUID 时生成新标识。
pub(crate) async fn create_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    spawn_store_task(database, move |database| {
        let uuid = body.uuid.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        upsert_record_blocking(database, &tenant, &uuid, &body)
    })
    .await
}

/// 更新租户内指定 UUID 的工作流。
pub(crate) async fn update_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
    body: WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    spawn_store_task(database, move |database| {
        upsert_record_blocking(database, &tenant, &uuid, &body)
    })
    .await
}

/// 删除租户内指定 UUID 的工作流并报告是否命中记录。
pub(crate) async fn delete_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    uuid: String,
) -> Result<WorkflowRecordDeleteResponse, WorkflowStoreError> {
    spawn_store_task(database, move |database| delete_record_blocking(database, &tenant, &uuid))
        .await
}

async fn spawn_store_task<T: Send + 'static>(
    database: GlobalDatabase,
    task: impl FnOnce(GlobalDatabase) -> Result<T, WorkflowStoreError> + Send + 'static,
) -> Result<T, WorkflowStoreError> {
    tokio::task::spawn_blocking(move || task(database)).await.map_err(|error| {
        WorkflowStoreError::internal(format!("workflow store task failed: {error}"))
    })?
}

fn list_records_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
) -> Result<Vec<WorkflowRecordSummary>, WorkflowStoreError> {
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
) -> Result<WorkflowRecord, WorkflowStoreError> {
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
    record
        .ok_or_else(|| WorkflowStoreError::not_found(format!("workflow record not found: {uuid}")))
}

fn upsert_record_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
    body: &WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    validate_uuid(uuid)?;
    validate_body(body)?;
    reject_system_template_write(uuid)?;
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
    record.ok_or_else(|| WorkflowStoreError::internal("workflow record was not saved"))
}

fn delete_record_blocking(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
) -> Result<WorkflowRecordDeleteResponse, WorkflowStoreError> {
    validate_uuid(uuid)?;
    if uuid == super::templates::VIDEO_EDIT_TEMPLATE_UUID {
        return Err(WorkflowStoreError::bad_request(
            "系统入口模板不可删除；请编辑或明确确认后恢复默认",
        ));
    }
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
) -> Result<Option<WorkflowRecord>, WorkflowStoreError> {
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

fn open_db(database: &GlobalDatabase) -> Result<Connection, WorkflowStoreError> {
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

fn validate_uuid(uuid: &str) -> Result<(), WorkflowStoreError> {
    Uuid::parse_str(uuid)
        .map(|_| ())
        .map_err(|_| WorkflowStoreError::bad_request("workflow uuid is invalid"))
}

fn validate_body(body: &WorkflowRecordUpsertBody) -> Result<(), WorkflowStoreError> {
    if body.name.trim().is_empty() {
        return Err(WorkflowStoreError::bad_request("workflow name is required"));
    }
    if body.workflow_yaml.trim().is_empty() {
        return Err(WorkflowStoreError::bad_request("workflow_yaml is required"));
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

fn sql_error(error: rusqlite::Error) -> WorkflowStoreError {
    WorkflowStoreError::internal(format!("workflow database error: {error}"))
}

fn database_error(error: anyhow::Error) -> WorkflowStoreError {
    WorkflowStoreError::internal(format!("workflow database error: {error:#}"))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;

fn reject_system_template_write(uuid: &str) -> Result<(), WorkflowStoreError> {
    if uuid == super::templates::VIDEO_EDIT_TEMPLATE_UUID {
        return Err(WorkflowStoreError::bad_request("系统入口模板必须使用带版本校验的更新操作"));
    }
    Ok(())
}
