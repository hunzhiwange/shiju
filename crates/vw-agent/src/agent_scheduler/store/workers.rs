use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{AgentCapabilityKind, AgentWorkerStatus, TeamId, WorkerName};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const WORKER_COLUMNS: &str = "id, runtime, deployment, model, model_provider, status, \
desired_state, team_id, worker_role, matrix_user_id, room_id, container_state, idle_since_ms, \
last_seen_at_ms, last_heartbeat_at, capabilities_json, updated_at_ms";

#[derive(Debug, Clone, PartialEq)]
pub struct NewAgentWorkerSnapshot {
    pub id: WorkerName,
    pub runtime: Option<String>,
    pub deployment: Option<String>,
    pub model: String,
    pub model_provider: Option<String>,
    pub status: AgentWorkerStatus,
    pub desired_state: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
    pub matrix_user_id: Option<String>,
    pub room_id: Option<String>,
    pub container_state: Option<String>,
    pub idle_since_ms: Option<TimestampMs>,
    pub last_seen_at_ms: Option<TimestampMs>,
    pub last_heartbeat_at: Option<String>,
    pub capabilities: Vec<AgentCapabilityRecord>,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredAgentWorker {
    pub id: WorkerName,
    pub runtime: Option<String>,
    pub deployment: Option<String>,
    pub model: String,
    pub model_provider: Option<String>,
    pub status: AgentWorkerStatus,
    pub desired_state: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
    pub matrix_user_id: Option<String>,
    pub room_id: Option<String>,
    pub container_state: Option<String>,
    pub idle_since_ms: Option<TimestampMs>,
    pub last_seen_at_ms: Option<TimestampMs>,
    pub last_heartbeat_at: Option<String>,
    pub capabilities: Vec<AgentCapabilityRecord>,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCapabilityRecord {
    pub kind: AgentCapabilityKind,
    pub name: String,
    pub metadata: Value,
}

impl AgentCapabilityRecord {
    pub fn skill(name: impl Into<String>) -> Self {
        Self { kind: AgentCapabilityKind::Skill, name: name.into(), metadata: Value::Null }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentWorkerFilter {
    pub runtime: Option<String>,
    pub required_skills: Vec<String>,
    pub team_id: Option<TeamId>,
    pub container_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAgentWorkerLifecycleRequest {
    pub worker_id: WorkerName,
    pub status: AgentWorkerStatus,
    pub container_state: String,
    pub idle_since_ms: Option<TimestampMs>,
    pub last_seen_at_ms: TimestampMs,
    pub last_heartbeat_at: Option<String>,
}

impl AgentSchedulerStore {
    pub fn upsert_worker_snapshot(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        worker: NewAgentWorkerSnapshot,
    ) -> Result<StoredAgentWorker> {
        validate_worker_snapshot(&worker)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.upsert_worker_snapshot(context, worker);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let capabilities_json = serialize_capabilities(&worker.capabilities)?;
        tx.execute(
            "INSERT INTO scheduler_workers (
                 platform_id, company_id, project_id, id, runtime, deployment, model,
                 model_provider, status, desired_state, team_id, worker_role, matrix_user_id,
                 room_id, container_state, idle_since_ms, last_seen_at_ms, last_heartbeat_at,
                 capabilities_json, updated_at_ms
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                 ?17, ?18, ?19, ?20
             )
             ON CONFLICT(platform_id, company_id, project_id, id) DO UPDATE SET
                 runtime = excluded.runtime,
                 deployment = excluded.deployment,
                 model = excluded.model,
                 model_provider = excluded.model_provider,
                 status = excluded.status,
                 desired_state = excluded.desired_state,
                 team_id = excluded.team_id,
                 worker_role = excluded.worker_role,
                 matrix_user_id = excluded.matrix_user_id,
                 room_id = excluded.room_id,
                 container_state = excluded.container_state,
                 idle_since_ms = excluded.idle_since_ms,
                 last_seen_at_ms = excluded.last_seen_at_ms,
                 last_heartbeat_at = excluded.last_heartbeat_at,
                 capabilities_json = excluded.capabilities_json,
                 updated_at_ms = excluded.updated_at_ms",
            params![
                platform_id,
                company_id,
                project_id,
                worker.id.as_str(),
                worker.runtime,
                worker.deployment,
                worker.model,
                worker.model_provider,
                worker.status.as_str(),
                worker.desired_state,
                worker.team_id.as_ref().map(TeamId::as_str),
                worker.worker_role,
                worker.matrix_user_id,
                worker.room_id,
                worker.container_state,
                worker.idle_since_ms.map(|value| value.0),
                worker.last_seen_at_ms.map(|value| value.0),
                worker.last_heartbeat_at,
                capabilities_json,
                worker.updated_at_ms.0,
            ],
        )?;

        let stored = load_worker_with_conn(&tx, context, &worker.id)?
            .with_context(|| format!("worker disappeared after upsert: {}", worker.id.as_str()))?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn list_workers(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentWorkerFilter,
    ) -> Result<Vec<StoredAgentWorker>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_workers(context, filter);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut query = format!(
            "SELECT {WORKER_COLUMNS} FROM scheduler_workers
             WHERE platform_id = ? AND company_id = ? AND project_id = ?"
        );
        let mut bind_values =
            vec![platform_id.to_string(), company_id.to_string(), project_id.to_string()];

        if let Some(runtime) = &filter.runtime {
            query.push_str(" AND runtime = ?");
            bind_values.push(runtime.to_string());
        }
        if let Some(team_id) = &filter.team_id {
            query.push_str(" AND team_id = ?");
            bind_values.push(team_id.as_str().to_string());
        }
        if let Some(container_state) = &filter.container_state {
            query.push_str(" AND container_state = ?");
            bind_values.push(container_state.to_string());
        }
        query.push_str(" ORDER BY id");

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params_from_iter(bind_values.iter()), map_worker_row)?;
        let mut workers = rows.collect::<rusqlite::Result<Vec<_>>>().context("list workers")?;
        filter_workers_by_required_skills(&mut workers, &filter.required_skills);
        Ok(workers)
    }

    pub fn load_worker(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        worker_id: &WorkerName,
    ) -> Result<Option<StoredAgentWorker>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_worker(context, worker_id);
        }
        let conn = open_initialized_connection(self, context)?;
        load_worker_with_conn(&conn, context, worker_id)
    }

    pub fn update_worker_lifecycle(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: UpdateAgentWorkerLifecycleRequest,
    ) -> Result<StoredAgentWorker> {
        ensure_non_empty(&request.container_state, "worker container state")?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.update_worker_lifecycle(context, request);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_workers
             SET status = ?1,
                 container_state = ?2,
                 idle_since_ms = ?3,
                 last_seen_at_ms = ?4,
                 last_heartbeat_at = COALESCE(?5, last_heartbeat_at),
                 updated_at_ms = ?4
             WHERE platform_id = ?6 AND company_id = ?7 AND project_id = ?8 AND id = ?9",
            params![
                request.status.as_str(),
                request.container_state,
                request.idle_since_ms.map(|value| value.0),
                request.last_seen_at_ms.0,
                request.last_heartbeat_at,
                platform_id,
                company_id,
                project_id,
                request.worker_id.as_str(),
            ],
        )?;
        if updated == 0 {
            bail!("worker not found: {}", request.worker_id.as_str());
        }

        let stored =
            load_worker_with_conn(&tx, context, &request.worker_id)?.with_context(|| {
                format!("worker disappeared after lifecycle update: {}", request.worker_id.as_str())
            })?;
        tx.commit()?;
        Ok(stored)
    }
}

fn validate_worker_snapshot(worker: &NewAgentWorkerSnapshot) -> Result<()> {
    ensure_non_empty(&worker.model, "worker model")?;
    for capability in &worker.capabilities {
        ensure_non_empty(&capability.name, "worker capability name")?;
    }
    Ok(())
}

fn load_worker_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    worker_id: &WorkerName,
) -> Result<Option<StoredAgentWorker>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {WORKER_COLUMNS} FROM scheduler_workers
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, worker_id.as_str()], map_worker_row)
        .optional()
        .with_context(|| format!("load worker: {}", worker_id.as_str()))
}

fn map_worker_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentWorker> {
    let status: String = row.get(5)?;
    let team_id: Option<String> = row.get(7)?;
    let capabilities_json: String = row.get(15)?;

    Ok(StoredAgentWorker {
        id: parse_worker_name(row.get(0)?, 0)?,
        runtime: row.get(1)?,
        deployment: row.get(2)?,
        model: row.get(3)?,
        model_provider: row.get(4)?,
        status: AgentWorkerStatus::parse(&status).map_err(|error| conversion_error(5, error))?,
        desired_state: row.get(6)?,
        team_id: parse_optional_team_id(team_id, 7)?,
        worker_role: row.get(8)?,
        matrix_user_id: row.get(9)?,
        room_id: row.get(10)?,
        container_state: row.get(11)?,
        idle_since_ms: row.get::<_, Option<i64>>(12)?.map(TimestampMs),
        last_seen_at_ms: row.get::<_, Option<i64>>(13)?.map(TimestampMs),
        last_heartbeat_at: row.get(14)?,
        capabilities: parse_capabilities(&capabilities_json, 15)?,
        updated_at_ms: TimestampMs(row.get(16)?),
    })
}

pub(super) fn filter_workers_by_required_skills(
    workers: &mut Vec<StoredAgentWorker>,
    required_skills: &[String],
) {
    let required_skills = required_skills
        .iter()
        .map(|skill| skill.trim())
        .filter(|skill| !skill.is_empty())
        .collect::<Vec<_>>();
    if required_skills.is_empty() {
        return;
    }

    workers.retain(|worker| {
        required_skills.iter().all(|required_skill| {
            worker.capabilities.iter().any(|capability| {
                capability.kind == AgentCapabilityKind::Skill && capability.name == *required_skill
            })
        })
    });
}

pub(super) fn serialize_capabilities(capabilities: &[AgentCapabilityRecord]) -> Result<String> {
    let json = capabilities.iter().map(CapabilityJson::from).collect::<Vec<_>>();
    serde_json::to_string(&json).context("serialize worker capabilities")
}

fn parse_capabilities(value: &str, column: usize) -> rusqlite::Result<Vec<AgentCapabilityRecord>> {
    parse_capabilities_value(value).map_err(|error| conversion_error(column, error))
}

pub(super) fn parse_capabilities_value(value: &str) -> Result<Vec<AgentCapabilityRecord>> {
    let json = serde_json::from_str::<Vec<CapabilityJson>>(value).map_err(anyhow::Error::new)?;
    json.into_iter().map(AgentCapabilityRecord::try_from).collect::<Result<Vec<_>>>()
}

fn parse_worker_name(value: String, column: usize) -> rusqlite::Result<WorkerName> {
    WorkerName::new(value).map_err(|error| conversion_error(column, error))
}

fn parse_optional_team_id(
    value: Option<String>,
    column: usize,
) -> rusqlite::Result<Option<TeamId>> {
    value.map(TeamId::new).transpose().map_err(|error| conversion_error(column, error))
}

fn ensure_non_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field_name} must not be empty");
    }
    Ok(())
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

#[derive(Debug, Serialize, Deserialize)]
struct CapabilityJson {
    kind: String,
    name: String,
    #[serde(default)]
    metadata: Value,
}

impl From<&AgentCapabilityRecord> for CapabilityJson {
    fn from(value: &AgentCapabilityRecord) -> Self {
        Self {
            kind: value.kind.as_str().to_string(),
            name: value.name.clone(),
            metadata: value.metadata.clone(),
        }
    }
}

impl TryFrom<CapabilityJson> for AgentCapabilityRecord {
    type Error = anyhow::Error;

    fn try_from(value: CapabilityJson) -> Result<Self> {
        Ok(Self {
            kind: AgentCapabilityKind::parse(&value.kind)?,
            name: value.name,
            metadata: value.metadata,
        })
    }
}
