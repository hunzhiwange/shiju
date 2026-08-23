//! 实现 PostgreSQL 团队快照的写入与读取，并校验成员和计数字段的领域类型。
//! 快照覆盖在单事务内完成，成员 JSON 与就绪计数不会以不同版本暴露。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{AgentTeamStatus, TeamId, WorkerName};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::teams::{
    NewAgentTeamSnapshot, StoredAgentTeam, parse_team_members_value, serialize_team_members,
};

const TEAM_COLUMNS: &str = "id, name, status, leader_id, team_room_id, leader_dm_room_id, \
leader_ready, ready_workers, total_workers, members_json, updated_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn upsert_team_snapshot(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        team: NewAgentTeamSnapshot,
    ) -> Result<StoredAgentTeam> {
        crate::global_database::run_sync_database(|| {
            let members_json = serialize_team_members(&team.members)?;
            let ready_workers = i64::from(team.ready_workers);
            let total_workers = i64::from(team.total_workers);
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL team upsert")?;
            transaction
                .execute(
                    "INSERT INTO scheduler_teams (
                     platform_id, company_id, project_id, id, name, status, leader_id,
                     team_room_id, leader_dm_room_id, leader_ready, ready_workers, total_workers,
                     members_json, updated_at_ms
                 ) VALUES (
                     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                 )
                 ON CONFLICT (platform_id, company_id, project_id, id) DO UPDATE SET
                     name = EXCLUDED.name,
                     status = EXCLUDED.status,
                     leader_id = EXCLUDED.leader_id,
                     team_room_id = EXCLUDED.team_room_id,
                     leader_dm_room_id = EXCLUDED.leader_dm_room_id,
                     leader_ready = EXCLUDED.leader_ready,
                     ready_workers = EXCLUDED.ready_workers,
                     total_workers = EXCLUDED.total_workers,
                     members_json = EXCLUDED.members_json,
                     updated_at_ms = EXCLUDED.updated_at_ms",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &team.id.as_str(),
                        &team.name,
                        &team.status.as_str(),
                        &team.leader_id.as_str(),
                        &team.team_room_id,
                        &team.leader_dm_room_id,
                        &team.leader_ready,
                        &ready_workers,
                        &total_workers,
                        &members_json,
                        &team.updated_at_ms.0,
                    ],
                )
                .context("upsert scoped PostgreSQL team")?;
            let stored =
                load_team_with_client(&mut transaction, &scope, &team.id)?.with_context(|| {
                    format!("PostgreSQL team disappeared after upsert: {}", team.id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL team upsert")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn list_teams(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<StoredAgentTeam>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {TEAM_COLUMNS} FROM scheduler_teams
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                     ORDER BY id"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                    ],
                )
                .context("list scoped PostgreSQL teams")?
                .iter()
                .map(map_team_row)
                .collect()
        })
    }

    pub(in crate::agent_scheduler::store) fn load_team(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        team_id: &TeamId,
    ) -> Result<Option<StoredAgentTeam>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            load_team_with_client(&mut *self.database.connect_postgres()?, &scope, team_id)
        })
    }
}

fn load_team_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    team_id: &TeamId,
) -> Result<Option<StoredAgentTeam>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {TEAM_COLUMNS} FROM scheduler_teams
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &team_id.as_str(),
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL team: {}", team_id.as_str()))?
        .as_ref()
        .map(map_team_row)
        .transpose()
}

fn map_team_row(row: &Row) -> Result<StoredAgentTeam> {
    let id: String = row.try_get(0).context("read PostgreSQL team id")?;
    let status: String = row.try_get(2).context("read PostgreSQL team status")?;
    let leader_id: String = row.try_get(3).context("read PostgreSQL team leader")?;
    let ready_workers: i64 = row.try_get(7).context("read PostgreSQL team ready workers")?;
    let total_workers: i64 = row.try_get(8).context("read PostgreSQL team total workers")?;
    let members_json: String = row.try_get(9).context("read PostgreSQL team members")?;
    Ok(StoredAgentTeam {
        id: TeamId::new(id).context("parse PostgreSQL team id")?,
        name: row.try_get(1).context("read PostgreSQL team name")?,
        status: AgentTeamStatus::parse(&status).context("parse PostgreSQL team status")?,
        leader_id: WorkerName::new(leader_id).context("parse PostgreSQL team leader")?,
        team_room_id: row.try_get(4).context("read PostgreSQL team room")?,
        leader_dm_room_id: row.try_get(5).context("read PostgreSQL team leader room")?,
        leader_ready: row.try_get(6).context("read PostgreSQL team leader readiness")?,
        ready_workers: u32::try_from(ready_workers).with_context(|| {
            format!("PostgreSQL team ready workers out of range: {ready_workers}")
        })?,
        total_workers: u32::try_from(total_workers).with_context(|| {
            format!("PostgreSQL team total workers out of range: {total_workers}")
        })?,
        members: parse_team_members_value(&members_json)
            .context("parse PostgreSQL team members")?,
        updated_at_ms: TimestampMs(row.try_get(10).context("read PostgreSQL team update")?),
    })
}

#[cfg(test)]
#[path = "teams_tests.rs"]
mod teams_tests;
