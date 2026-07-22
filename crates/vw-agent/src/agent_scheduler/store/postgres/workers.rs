use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{AgentWorkerStatus, TeamId, WorkerName};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::workers::{
    AgentWorkerFilter, NewAgentWorkerSnapshot, StoredAgentWorker,
    UpdateAgentWorkerLifecycleRequest, filter_workers_by_required_skills, parse_capabilities_value,
    serialize_capabilities,
};

const WORKER_COLUMNS: &str = "id, runtime, deployment, model, model_provider, status, \
desired_state, team_id, worker_role, matrix_user_id, room_id, container_state, idle_since_ms, \
last_seen_at_ms, last_heartbeat_at, capabilities_json, updated_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn upsert_worker_snapshot(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        worker: NewAgentWorkerSnapshot,
    ) -> Result<StoredAgentWorker> {
        crate::global_database::run_sync_database(|| {
            let capabilities_json = serialize_capabilities(&worker.capabilities)?;
            let team_id = worker.team_id.as_ref().map(TeamId::as_str);
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL worker upsert")?;
            transaction
                .execute(
                    "INSERT INTO scheduler_workers (
                     platform_id, company_id, project_id, id, runtime, deployment, model,
                     model_provider, status, desired_state, team_id, worker_role, matrix_user_id,
                     room_id, container_state, idle_since_ms, last_seen_at_ms, last_heartbeat_at,
                     capabilities_json, updated_at_ms
                 ) VALUES (
                     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                     $16, $17, $18, $19, $20
                 )
                 ON CONFLICT (platform_id, company_id, project_id, id) DO UPDATE SET
                     runtime = EXCLUDED.runtime,
                     deployment = EXCLUDED.deployment,
                     model = EXCLUDED.model,
                     model_provider = EXCLUDED.model_provider,
                     status = EXCLUDED.status,
                     desired_state = EXCLUDED.desired_state,
                     team_id = EXCLUDED.team_id,
                     worker_role = EXCLUDED.worker_role,
                     matrix_user_id = EXCLUDED.matrix_user_id,
                     room_id = EXCLUDED.room_id,
                     container_state = EXCLUDED.container_state,
                     idle_since_ms = EXCLUDED.idle_since_ms,
                     last_seen_at_ms = EXCLUDED.last_seen_at_ms,
                     last_heartbeat_at = EXCLUDED.last_heartbeat_at,
                     capabilities_json = EXCLUDED.capabilities_json,
                     updated_at_ms = EXCLUDED.updated_at_ms",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &worker.id.as_str(),
                        &worker.runtime,
                        &worker.deployment,
                        &worker.model,
                        &worker.model_provider,
                        &worker.status.as_str(),
                        &worker.desired_state,
                        &team_id,
                        &worker.worker_role,
                        &worker.matrix_user_id,
                        &worker.room_id,
                        &worker.container_state,
                        &worker.idle_since_ms.map(|value| value.0),
                        &worker.last_seen_at_ms.map(|value| value.0),
                        &worker.last_heartbeat_at,
                        &capabilities_json,
                        &worker.updated_at_ms.0,
                    ],
                )
                .context("upsert scoped PostgreSQL worker")?;
            let stored = load_worker_with_client(&mut transaction, &scope, &worker.id)?
                .with_context(|| {
                    format!("PostgreSQL worker disappeared after upsert: {}", worker.id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL worker upsert")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn list_workers(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentWorkerFilter,
    ) -> Result<Vec<StoredAgentWorker>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let runtime = filter.runtime.as_deref();
            let team_id = filter.team_id.as_ref().map(TeamId::as_str);
            let container_state = filter.container_state.as_deref();
            let mut workers = self
                .database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {WORKER_COLUMNS} FROM scheduler_workers
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND ($4::TEXT IS NULL OR runtime = $4)
                       AND ($5::TEXT IS NULL OR team_id = $5)
                       AND ($6::TEXT IS NULL OR container_state = $6)
                     ORDER BY id"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &runtime,
                        &team_id,
                        &container_state,
                    ],
                )
                .context("list scoped PostgreSQL workers")?
                .iter()
                .map(map_worker_row)
                .collect::<Result<Vec<_>>>()?;
            filter_workers_by_required_skills(&mut workers, &filter.required_skills);
            Ok(workers)
        })
    }

    pub(in crate::agent_scheduler::store) fn load_worker(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        worker_id: &WorkerName,
    ) -> Result<Option<StoredAgentWorker>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            load_worker_with_client(&mut *self.database.connect_postgres()?, &scope, worker_id)
        })
    }

    pub(in crate::agent_scheduler::store) fn update_worker_lifecycle(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: UpdateAgentWorkerLifecycleRequest,
    ) -> Result<StoredAgentWorker> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL worker lifecycle update")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_workers
                 SET status = $1,
                     container_state = $2,
                     idle_since_ms = $3,
                     last_seen_at_ms = $4,
                     last_heartbeat_at = COALESCE($5, last_heartbeat_at),
                     updated_at_ms = $4
                 WHERE platform_id = $6 AND company_id = $7 AND project_id = $8 AND id = $9",
                    &[
                        &request.status.as_str(),
                        &request.container_state,
                        &request.idle_since_ms.map(|value| value.0),
                        &request.last_seen_at_ms.0,
                        &request.last_heartbeat_at,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.worker_id.as_str(),
                    ],
                )
                .context("update scoped PostgreSQL worker lifecycle")?;
            match changed {
                0 => bail!("worker not found: {}", request.worker_id.as_str()),
                1 => {}
                _ => bail!(
                    "PostgreSQL worker lifecycle update affected {changed} rows: {}",
                    request.worker_id.as_str()
                ),
            }
            let stored = load_worker_with_client(&mut transaction, &scope, &request.worker_id)?
                .with_context(|| {
                    format!(
                        "PostgreSQL worker disappeared after lifecycle update: {}",
                        request.worker_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL worker lifecycle update")?;
            Ok(stored)
        })
    }
}

fn load_worker_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    worker_id: &WorkerName,
) -> Result<Option<StoredAgentWorker>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {WORKER_COLUMNS} FROM scheduler_workers
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &worker_id.as_str(),
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL worker: {}", worker_id.as_str()))?
        .as_ref()
        .map(map_worker_row)
        .transpose()
}

fn map_worker_row(row: &Row) -> Result<StoredAgentWorker> {
    let id: String = row.try_get(0).context("read PostgreSQL worker id")?;
    let status: String = row.try_get(5).context("read PostgreSQL worker status")?;
    let team_id: Option<String> = row.try_get(7).context("read PostgreSQL worker team")?;
    let capabilities_json: String =
        row.try_get(15).context("read PostgreSQL worker capabilities")?;
    Ok(StoredAgentWorker {
        id: WorkerName::new(id).context("parse PostgreSQL worker id")?,
        runtime: row.try_get(1).context("read PostgreSQL worker runtime")?,
        deployment: row.try_get(2).context("read PostgreSQL worker deployment")?,
        model: row.try_get(3).context("read PostgreSQL worker model")?,
        model_provider: row.try_get(4).context("read PostgreSQL worker model provider")?,
        status: AgentWorkerStatus::parse(&status).context("parse PostgreSQL worker status")?,
        desired_state: row.try_get(6).context("read PostgreSQL worker desired state")?,
        team_id: team_id.map(TeamId::new).transpose().context("parse PostgreSQL worker team")?,
        worker_role: row.try_get(8).context("read PostgreSQL worker role")?,
        matrix_user_id: row.try_get(9).context("read PostgreSQL worker Matrix user")?,
        room_id: row.try_get(10).context("read PostgreSQL worker room")?,
        container_state: row.try_get(11).context("read PostgreSQL worker container state")?,
        idle_since_ms: row.try_get::<_, Option<i64>>(12)?.map(TimestampMs),
        last_seen_at_ms: row.try_get::<_, Option<i64>>(13)?.map(TimestampMs),
        last_heartbeat_at: row.try_get(14).context("read PostgreSQL worker heartbeat")?,
        capabilities: parse_capabilities_value(&capabilities_json)
            .context("parse PostgreSQL worker capabilities")?,
        updated_at_ms: TimestampMs(row.try_get(16).context("read PostgreSQL worker update")?),
    })
}

#[cfg(test)]
#[path = "workers_tests.rs"]
mod workers_tests;
