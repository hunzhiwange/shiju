//! 实现 SQLite 任务产物的记录与查询，并校验产物关联的运行确属目标任务。

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{RunId, TaskId};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const ARTIFACT_COLUMNS: &str = "id, task_id, run_id, kind, name, path, payload_json, created_at_ms";

#[derive(Debug, Clone, PartialEq)]
/// 记录任务产物所需的任务归属、可选运行归属及载荷。
pub struct RecordTaskArtifactRequest {
    pub task_id: TaskId,
    pub run_id: Option<RunId>,
    pub kind: String,
    pub name: String,
    pub path: Option<String>,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
/// 已持久化并分配数据库序号的任务产物。
pub struct StoredTaskArtifact {
    pub id: i64,
    pub task_id: TaskId,
    pub run_id: Option<RunId>,
    pub kind: String,
    pub name: String,
    pub path: Option<String>,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

impl AgentSchedulerStore {
    /// 校验请求和运行归属后记录产物，返回完整持久化记录。
    pub fn record_task_artifact(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: RecordTaskArtifactRequest,
    ) -> Result<StoredTaskArtifact> {
        validate_artifact_request(&request)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.record_task_artifact(context, request);
        }
        let payload_json =
            serde_json::to_string(&request.payload).context("serialize task artifact payload")?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_run_matches_task(&tx, context, request.run_id.as_ref(), &request.task_id)?;
        tx.execute(
            "INSERT INTO scheduler_task_artifacts (
                platform_id, company_id, project_id, task_id, run_id, kind, name, path,
                payload_json, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                platform_id,
                company_id,
                project_id,
                request.task_id.as_str(),
                request.run_id.as_ref().map(RunId::as_str),
                request.kind,
                request.name,
                request.path,
                payload_json,
                request.created_at_ms.0,
            ],
        )?;
        let id = tx.last_insert_rowid();
        let stored = load_artifact_with_conn(&tx, context, id)?
            .with_context(|| format!("task artifact disappeared after record: {id}"))?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn list_task_artifacts(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Vec<StoredTaskArtifact>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_artifacts(context, task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {ARTIFACT_COLUMNS}
             FROM scheduler_task_artifacts
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4
             ORDER BY id"
        ))?;
        let rows = stmt.query_map(
            params![platform_id, company_id, project_id, task_id.as_str()],
            map_artifact_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list agent scheduler task artifacts")
    }
}

fn validate_artifact_request(request: &RecordTaskArtifactRequest) -> Result<()> {
    if request.kind.trim().is_empty() {
        bail!("task artifact kind must not be empty");
    }
    if request.name.trim().is_empty() {
        bail!("task artifact name must not be empty");
    }
    Ok(())
}

fn ensure_run_matches_task(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    run_id: Option<&RunId>,
    task_id: &TaskId,
) -> Result<()> {
    let Some(run_id) = run_id else {
        return Ok(());
    };
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let run_task_id = conn
        .query_row(
            "SELECT task_id FROM scheduler_runs
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4",
            params![platform_id, company_id, project_id, run_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .with_context(|| format!("task artifact run not found: {}", run_id.as_str()))?;
    if run_task_id != task_id.as_str() {
        bail!("task artifact run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
    }
    Ok(())
}

fn load_artifact_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    id: i64,
) -> Result<Option<StoredTaskArtifact>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {ARTIFACT_COLUMNS} FROM scheduler_task_artifacts
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, id], map_artifact_row)
        .optional()
        .with_context(|| format!("load agent scheduler task artifact: {id}"))
}

fn map_artifact_row(row: &Row<'_>) -> rusqlite::Result<StoredTaskArtifact> {
    let task_id: String = row.get(1)?;
    let run_id: Option<String> = row.get(2)?;
    let payload_json: String = row.get(6)?;

    Ok(StoredTaskArtifact {
        id: row.get(0)?,
        task_id: TaskId::new(task_id).map_err(|error| conversion_error(1, error))?,
        run_id: run_id.map(RunId::new).transpose().map_err(|error| conversion_error(2, error))?,
        kind: row.get(3)?,
        name: row.get(4)?,
        path: row.get(5)?,
        payload: serde_json::from_str(&payload_json)
            .map_err(|error| conversion_error(6, anyhow::Error::new(error)))?,
        created_at_ms: TimestampMs(row.get(7)?),
    })
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}
