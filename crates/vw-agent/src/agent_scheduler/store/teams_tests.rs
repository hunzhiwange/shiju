//! 测试团队存储在多后端下的快照校验、加载与列表行为。
//! 同一快照契约在不同后端复用，非法就绪计数必须在写入前后都不可见。

use anyhow::Result;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::domain::{AgentSchedulerContext, AgentTeamStatus, TeamId, WorkerName};
use super::store::AgentSchedulerStore;
use super::store::teams::{AgentTeamMemberRecord, NewAgentTeamSnapshot, StoredAgentTeam};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn agent_team_store_lists_and_loads_team_snapshots() -> Result<()> {
    for_each_team_store_provider(assert_team_store_lists_and_loads_team_snapshots)
}

#[test]
fn agent_team_store_rejects_ready_workers_above_total() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let (store, context, _) = team_store_fixture(database, &home.path().join("project"))?;
    let mut team = team_snapshot("team-invalid", "Invalid Team", "leader-invalid", 2)?;
    team.ready_workers = 3;

    let error = store
        .upsert_team_snapshot(&context, team)
        .expect_err("ready workers above total must fail");

    assert!(error.to_string().contains("team ready workers must not exceed total workers"));
    Ok(())
}


// 团队快照契约在所有可用后端执行，确保校验与列表顺序一致。
fn for_each_team_store_provider(
    mut check: impl FnMut(
        &AgentSchedulerStore,
        &AgentSchedulerContext,
        &AgentSchedulerContext,
    ) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_project = sqlite_home.path().join("project");
    let sqlite_database = GlobalDatabase::for_home(sqlite_home.path())?;
    let (sqlite_store, sqlite_context, sqlite_isolated_context) =
        team_store_fixture(sqlite_database, &sqlite_project)?;
    check(&sqlite_store, &sqlite_context, &sqlite_isolated_context)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_project = tempfile::tempdir()?;
    let (postgres_store, postgres_context, postgres_isolated_context) =
        team_store_fixture(postgres_fixture.database().clone(), postgres_project.path())?;
    check(&postgres_store, &postgres_context, &postgres_isolated_context)?;
    postgres_fixture.cleanup()
}

fn team_store_fixture(
    database: GlobalDatabase,
    project_path: &std::path::Path,
) -> Result<(AgentSchedulerStore, AgentSchedulerContext, AgentSchedulerContext)> {
    let primary_tenant = TenantScope::new("teams-platform", "teams-company")?;
    let isolated_tenant = TenantScope::new("teams-platform-isolated", "teams-company-isolated")?;
    let project_id = ProjectId::from("teams-store-project");
    let project_store = ProjectStore::new(database.clone())?;
    for tenant in [&primary_tenant, &isolated_tenant] {
        project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.as_ref().to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: Some("Teams store project".to_owned()),
                icon: None,
                commands: None,
                time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
    }
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(primary_tenant, project_id.clone())?),
        project_path,
    )?;
    let isolated_context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(isolated_tenant, project_id)?),
        project_path,
    )?;
    Ok((AgentSchedulerStore::from_database(database), context, isolated_context))
}


fn team_ids(teams: &[StoredAgentTeam]) -> Vec<&str> {
    teams.iter().map(|team| team.id.as_str()).collect()
}

fn assert_team_store_lists_and_loads_team_snapshots(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    isolated_context: &AgentSchedulerContext,
) -> Result<()> {
    store.initialize_project(context)?;
    let team_zulu = team_snapshot("team-zulu", "Zulu Team", "leader-zulu", 1)?;
    let team_alpha = team_snapshot("team-alpha", "Alpha Team", "leader-alpha", 2)?;
    store.upsert_team_snapshot(context, team_zulu)?;
    let stored_alpha = store.upsert_team_snapshot(context, team_alpha)?;
    store.upsert_team_snapshot(
        isolated_context,
        team_snapshot("team-alpha", "Isolated Team", "leader-isolated", 1)?,
    )?;

    assert_eq!(stored_alpha.id.as_str(), "team-alpha");
    assert_eq!(stored_alpha.members.len(), 2);

    let teams = store.list_teams(context)?;
    assert_eq!(team_ids(&teams), vec!["team-alpha", "team-zulu"]);

    let loaded = store.load_team(context, &TeamId::new("team-alpha")?)?.expect("team stored");
    assert_eq!(loaded.name, "Alpha Team");
    assert_eq!(loaded.status, AgentTeamStatus::Active);
    assert_eq!(loaded.leader_id.as_str(), "leader-alpha");
    assert_eq!(loaded.team_room_id.as_deref(), Some("!team-alpha:example.test"));
    assert_eq!(loaded.leader_dm_room_id.as_deref(), Some("!leader-alpha:example.test"));
    assert!(loaded.leader_ready);
    assert_eq!(loaded.ready_workers, 2);
    assert_eq!(loaded.total_workers, 2);
    assert_eq!(loaded.updated_at_ms, TimestampMs(1_783_000_020_000));
    assert_eq!(loaded.members.len(), 2);
    assert_eq!(loaded.members[0].worker_id.as_str(), "leader-alpha");
    assert_eq!(
        loaded.members[0].runtime_name.as_ref().map(WorkerName::as_str),
        Some("leader-alpha-runtime")
    );
    assert_eq!(loaded.members[0].role, "team_leader");
    assert_eq!(loaded.members[0].room_id.as_deref(), Some("!leader-room:example.test"));
    assert_eq!(loaded.members[0].matrix_user_id.as_deref(), Some("@leader-alpha:example.test"));
    assert!(loaded.members[0].ready);
    assert_eq!(loaded.members[1].worker_id.as_str(), "worker-alpha");
    assert_eq!(loaded.members[1].runtime_name, None);
    assert_eq!(loaded.members[1].role, "worker");
    assert_eq!(loaded.members[1].room_id, None);
    assert_eq!(loaded.members[1].matrix_user_id, None);
    assert!(loaded.members[1].ready);
    assert_eq!(
        store
            .load_team(isolated_context, &TeamId::new("team-alpha")?)?
            .expect("isolated team stored")
            .name,
        "Isolated Team"
    );

    Ok(())
}

fn team_snapshot(
    id: &str,
    name: &str,
    leader_id: &str,
    ready_workers: u32,
) -> Result<NewAgentTeamSnapshot> {
    Ok(NewAgentTeamSnapshot {
        id: TeamId::new(id)?,
        name: name.to_string(),
        status: AgentTeamStatus::Active,
        leader_id: WorkerName::new(leader_id)?,
        team_room_id: Some(format!("!{id}:example.test")),
        leader_dm_room_id: Some(format!("!{leader_id}:example.test")),
        leader_ready: true,
        ready_workers,
        total_workers: 2,
        members: vec![
            AgentTeamMemberRecord {
                worker_id: WorkerName::new(leader_id)?,
                runtime_name: Some(WorkerName::new(format!("{leader_id}-runtime"))?),
                role: "team_leader".to_string(),
                room_id: Some("!leader-room:example.test".to_string()),
                matrix_user_id: Some(format!("@{leader_id}:example.test")),
                ready: true,
            },
            AgentTeamMemberRecord {
                worker_id: WorkerName::new(id.replace("team", "worker"))?,
                runtime_name: None,
                role: "worker".to_string(),
                room_id: None,
                matrix_user_id: None,
                ready: ready_workers == 2,
            },
        ],
        updated_at_ms: TimestampMs(1_783_000_020_000),
    })
}
