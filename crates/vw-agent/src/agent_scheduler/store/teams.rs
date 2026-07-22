use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{AgentTeamStatus, TeamId, WorkerName};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const TEAM_COLUMNS: &str = "id, name, status, leader_id, team_room_id, leader_dm_room_id, \
leader_ready, ready_workers, total_workers, members_json, updated_at_ms";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAgentTeamSnapshot {
    pub id: TeamId,
    pub name: String,
    pub status: AgentTeamStatus,
    pub leader_id: WorkerName,
    pub team_room_id: Option<String>,
    pub leader_dm_room_id: Option<String>,
    pub leader_ready: bool,
    pub ready_workers: u32,
    pub total_workers: u32,
    pub members: Vec<AgentTeamMemberRecord>,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAgentTeam {
    pub id: TeamId,
    pub name: String,
    pub status: AgentTeamStatus,
    pub leader_id: WorkerName,
    pub team_room_id: Option<String>,
    pub leader_dm_room_id: Option<String>,
    pub leader_ready: bool,
    pub ready_workers: u32,
    pub total_workers: u32,
    pub members: Vec<AgentTeamMemberRecord>,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTeamMemberRecord {
    pub worker_id: WorkerName,
    pub runtime_name: Option<WorkerName>,
    pub role: String,
    pub room_id: Option<String>,
    pub matrix_user_id: Option<String>,
    pub ready: bool,
}

impl AgentSchedulerStore {
    pub fn upsert_team_snapshot(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        team: NewAgentTeamSnapshot,
    ) -> Result<StoredAgentTeam> {
        validate_team_snapshot(&team)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.upsert_team_snapshot(context, team);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let members_json = serialize_team_members(&team.members)?;
        tx.execute(
            "INSERT INTO scheduler_teams (
                 platform_id, company_id, project_id, id, name, status, leader_id, team_room_id,
                 leader_dm_room_id, leader_ready, ready_workers, total_workers, members_json,
                 updated_at_ms
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
             )
             ON CONFLICT(platform_id, company_id, project_id, id) DO UPDATE SET
                 name = excluded.name,
                 status = excluded.status,
                 leader_id = excluded.leader_id,
                 team_room_id = excluded.team_room_id,
                 leader_dm_room_id = excluded.leader_dm_room_id,
                 leader_ready = excluded.leader_ready,
                 ready_workers = excluded.ready_workers,
                 total_workers = excluded.total_workers,
                 members_json = excluded.members_json,
                 updated_at_ms = excluded.updated_at_ms",
            params![
                platform_id,
                company_id,
                project_id,
                team.id.as_str(),
                team.name,
                team.status.as_str(),
                team.leader_id.as_str(),
                team.team_room_id,
                team.leader_dm_room_id,
                bool_to_i64(team.leader_ready),
                i64::from(team.ready_workers),
                i64::from(team.total_workers),
                members_json,
                team.updated_at_ms.0,
            ],
        )?;

        let stored = load_team_with_conn(&tx, context, &team.id)?
            .with_context(|| format!("team disappeared after upsert: {}", team.id.as_str()))?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn list_teams(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<StoredAgentTeam>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_teams(context);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {TEAM_COLUMNS} FROM scheduler_teams
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
             ORDER BY id"
        ))?;
        let rows = stmt.query_map(params![platform_id, company_id, project_id], map_team_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list teams")
    }

    pub fn load_team(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        team_id: &TeamId,
    ) -> Result<Option<StoredAgentTeam>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_team(context, team_id);
        }
        let conn = open_initialized_connection(self, context)?;
        load_team_with_conn(&conn, context, team_id)
    }
}

fn validate_team_snapshot(team: &NewAgentTeamSnapshot) -> Result<()> {
    ensure_non_empty(&team.name, "team name")?;
    if team.ready_workers > team.total_workers {
        bail!("team ready workers must not exceed total workers");
    }
    for member in &team.members {
        ensure_non_empty(&member.role, "team member role")?;
    }
    Ok(())
}

fn load_team_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    team_id: &TeamId,
) -> Result<Option<StoredAgentTeam>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {TEAM_COLUMNS} FROM scheduler_teams
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, team_id.as_str()], map_team_row)
        .optional()
        .with_context(|| format!("load team: {}", team_id.as_str()))
}

fn map_team_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentTeam> {
    let status: String = row.get(2)?;
    let members_json: String = row.get(9)?;

    Ok(StoredAgentTeam {
        id: parse_team_id(row.get(0)?, 0)?,
        name: row.get(1)?,
        status: AgentTeamStatus::parse(&status).map_err(|error| conversion_error(2, error))?,
        leader_id: parse_worker_name(row.get(3)?, 3)?,
        team_room_id: row.get(4)?,
        leader_dm_room_id: row.get(5)?,
        leader_ready: row.get::<_, i64>(6)? != 0,
        ready_workers: parse_u32(row.get(7)?, 7)?,
        total_workers: parse_u32(row.get(8)?, 8)?,
        members: parse_team_members(&members_json, 9)?,
        updated_at_ms: TimestampMs(row.get(10)?),
    })
}

pub(super) fn serialize_team_members(members: &[AgentTeamMemberRecord]) -> Result<String> {
    let json = members.iter().map(TeamMemberJson::from).collect::<Vec<_>>();
    serde_json::to_string(&json).context("serialize team members")
}

fn parse_team_members(value: &str, column: usize) -> rusqlite::Result<Vec<AgentTeamMemberRecord>> {
    parse_team_members_value(value).map_err(|error| conversion_error(column, error))
}

pub(super) fn parse_team_members_value(value: &str) -> Result<Vec<AgentTeamMemberRecord>> {
    let json = serde_json::from_str::<Vec<TeamMemberJson>>(value).map_err(anyhow::Error::new)?;
    json.into_iter().map(AgentTeamMemberRecord::try_from).collect::<Result<Vec<_>>>()
}

fn parse_worker_name(value: String, column: usize) -> rusqlite::Result<WorkerName> {
    WorkerName::new(value).map_err(|error| conversion_error(column, error))
}

fn parse_team_id(value: String, column: usize) -> rusqlite::Result<TeamId> {
    TeamId::new(value).map_err(|error| conversion_error(column, error))
}

fn parse_u32(value: i64, column: usize) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|error| conversion_error(column, anyhow::Error::new(error)))
}

fn ensure_non_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field_name} must not be empty");
    }
    Ok(())
}

fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamMemberJson {
    worker_id: String,
    runtime_name: Option<String>,
    role: String,
    room_id: Option<String>,
    matrix_user_id: Option<String>,
    ready: bool,
}

impl From<&AgentTeamMemberRecord> for TeamMemberJson {
    fn from(value: &AgentTeamMemberRecord) -> Self {
        Self {
            worker_id: value.worker_id.as_str().to_string(),
            runtime_name: value.runtime_name.as_ref().map(WorkerName::as_str).map(str::to_string),
            role: value.role.clone(),
            room_id: value.room_id.clone(),
            matrix_user_id: value.matrix_user_id.clone(),
            ready: value.ready,
        }
    }
}

impl TryFrom<TeamMemberJson> for AgentTeamMemberRecord {
    type Error = anyhow::Error;

    fn try_from(value: TeamMemberJson) -> Result<Self> {
        Ok(Self {
            worker_id: WorkerName::new(value.worker_id)?,
            runtime_name: value.runtime_name.map(WorkerName::new).transpose()?,
            role: value.role,
            room_id: value.room_id,
            matrix_user_id: value.matrix_user_id,
            ready: value.ready,
        })
    }
}
