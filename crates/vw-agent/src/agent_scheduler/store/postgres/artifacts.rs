//! 实现 PostgreSQL 任务产物存取，并在事务内验证运行与任务的归属关系。
//! 写入先锁定关联运行，防止并发收尾期间把产物挂到错误任务。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::artifacts::{RecordTaskArtifactRequest, StoredTaskArtifact};

const ARTIFACT_COLUMNS: &str = "id, task_id, run_id, kind, name, path, payload_json, created_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn record_task_artifact(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: RecordTaskArtifactRequest,
    ) -> Result<StoredTaskArtifact> {
        crate::global_database::run_sync_database(|| {
            let payload_json = serde_json::to_string(&request.payload)
                .context("serialize task artifact payload")?;
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task artifact record")?;
            ensure_run_matches_task(
                &mut transaction,
                &scope,
                request.run_id.as_ref(),
                &request.task_id,
            )?;
            let id: i64 = transaction
                .query_one(
                    "INSERT INTO scheduler_task_artifacts (
                     platform_id, company_id, project_id, task_id, run_id, kind, name, path,
                     payload_json, created_at_ms
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 RETURNING id",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                        &request.run_id.as_ref().map(RunId::as_str),
                        &request.kind,
                        &request.name,
                        &request.path,
                        &payload_json,
                        &request.created_at_ms.0,
                    ],
                )
                .context("insert scoped PostgreSQL task artifact")?
                .try_get(0)
                .context("read inserted PostgreSQL task artifact id")?;
            let stored =
                load_artifact_with_client(&mut transaction, &scope, id)?.with_context(|| {
                    format!("PostgreSQL task artifact disappeared after record: {id}")
                })?;
            transaction.commit().context("commit PostgreSQL task artifact record")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn list_task_artifacts(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Vec<StoredTaskArtifact>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {ARTIFACT_COLUMNS} FROM scheduler_task_artifacts
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND task_id = $4
                     ORDER BY id"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                    ],
                )
                .context("list scoped PostgreSQL task artifacts")?
                .iter()
                .map(map_artifact_row)
                .collect()
        })
    }
}

fn ensure_run_matches_task(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    run_id: Option<&RunId>,
    task_id: &TaskId,
) -> Result<()> {
    let Some(run_id) = run_id else {
        return Ok(());
    };
    let project = scope.project_scope();
    let Some(row) = client
        .query_opt(
            "SELECT task_id FROM scheduler_runs
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &run_id.as_str(),
            ],
        )
        .context("load scoped PostgreSQL task artifact run")?
    else {
        bail!("task artifact run not found: {}", run_id.as_str());
    };
    let run_task_id: String =
        row.try_get(0).context("read PostgreSQL task artifact run task id")?;
    if run_task_id != task_id.as_str() {
        bail!("task artifact run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
    }
    Ok(())
}

fn load_artifact_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    id: i64,
) -> Result<Option<StoredTaskArtifact>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {ARTIFACT_COLUMNS} FROM scheduler_task_artifacts
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &id,
            ],
        )
        .context("load scoped PostgreSQL task artifact")?
        .as_ref()
        .map(map_artifact_row)
        .transpose()
}

fn map_artifact_row(row: &Row) -> Result<StoredTaskArtifact> {
    let task_id: String = row.try_get(1).context("read PostgreSQL artifact task id")?;
    let run_id: Option<String> = row.try_get(2).context("read PostgreSQL artifact run id")?;
    let payload_json: String = row.try_get(6).context("read PostgreSQL artifact payload")?;
    Ok(StoredTaskArtifact {
        id: row.try_get(0).context("read PostgreSQL artifact id")?,
        task_id: TaskId::new(task_id).context("parse PostgreSQL artifact task id")?,
        run_id: run_id.map(RunId::new).transpose().context("parse PostgreSQL artifact run id")?,
        kind: row.try_get(3).context("read PostgreSQL artifact kind")?,
        name: row.try_get(4).context("read PostgreSQL artifact name")?,
        path: row.try_get(5).context("read PostgreSQL artifact path")?,
        payload: serde_json::from_str(&payload_json)
            .context("parse PostgreSQL task artifact payload")?,
        created_at_ms: TimestampMs(row.try_get(7).context("read PostgreSQL artifact timestamp")?),
    })
}

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod artifacts_tests;
