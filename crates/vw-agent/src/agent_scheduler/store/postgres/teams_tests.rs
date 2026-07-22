use anyhow::Result;
use vw_api_types::common::TimestampMs;

use super::map_team_row;
use crate::agent_scheduler::domain::AgentTeamStatus;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn map_team_row_preserves_rooms_readiness_counts_members_and_timestamp() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let team = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            r#"SELECT 'team-1'::TEXT, 'Alpha Team'::TEXT, 'active'::TEXT, 'leader-1'::TEXT,
                      'room-team'::TEXT, 'room-dm'::TEXT, TRUE::BOOL, 2::BIGINT, 3::BIGINT,
                      '[{"worker_id":"leader-1","runtime_name":"runtime-a","role":"lead","room_id":"room-a","matrix_user_id":"@a:example","ready":true},
                        {"worker_id":"worker-2","runtime_name":null,"role":"reviewer","room_id":null,"matrix_user_id":null,"ready":false}]'::TEXT,
                      1234::BIGINT"#,
            &[],
        )?;
        map_team_row(&row)?
    };

    assert_eq!(team.id.as_str(), "team-1");
    assert_eq!(team.name, "Alpha Team");
    assert_eq!(team.status, AgentTeamStatus::Active);
    assert_eq!(team.leader_id.as_str(), "leader-1");
    assert_eq!(team.team_room_id.as_deref(), Some("room-team"));
    assert_eq!(team.leader_dm_room_id.as_deref(), Some("room-dm"));
    assert!(team.leader_ready);
    assert_eq!(team.ready_workers, 2);
    assert_eq!(team.total_workers, 3);
    assert_eq!(team.members.len(), 2);
    assert_eq!(team.members[0].worker_id.as_str(), "leader-1");
    assert_eq!(team.members[0].runtime_name.as_ref().map(|id| id.as_str()), Some("runtime-a"));
    assert_eq!(team.members[0].role, "lead");
    assert_eq!(team.members[0].room_id.as_deref(), Some("room-a"));
    assert_eq!(team.members[0].matrix_user_id.as_deref(), Some("@a:example"));
    assert!(team.members[0].ready);
    assert_eq!(team.members[1].worker_id.as_str(), "worker-2");
    assert_eq!(team.members[1].runtime_name, None);
    assert!(!team.members[1].ready);
    assert_eq!(team.updated_at_ms, TimestampMs(1_234));

    fixture.cleanup()
}

#[test]
fn map_team_row_preserves_null_rooms_and_empty_members() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let team = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'team-2'::TEXT, 'Beta'::TEXT, 'pending'::TEXT, 'leader-2'::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, 0::BIGINT, 1::BIGINT,
                    '[]'::TEXT, 5678::BIGINT",
            &[],
        )?;
        map_team_row(&row)?
    };

    assert_eq!(team.id.as_str(), "team-2");
    assert_eq!(team.status, AgentTeamStatus::Pending);
    assert_eq!(team.team_room_id, None);
    assert_eq!(team.leader_dm_room_id, None);
    assert!(!team.leader_ready);
    assert!(team.members.is_empty());
    assert_eq!(team.updated_at_ms, TimestampMs(5_678));

    fixture.cleanup()
}

#[test]
fn map_team_row_rejects_invalid_identifiers_status_counts_and_members() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let invalid_team_id = connection.query_one(
            "SELECT ''::TEXT, 'Team'::TEXT, 'active'::TEXT, 'leader'::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, 0::BIGINT, 1::BIGINT,
                    '[]'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_team_row(&invalid_team_id).expect_err("empty team id must fail");
        assert!(error.to_string().contains("parse PostgreSQL team id"));

        let invalid_status = connection.query_one(
            "SELECT 'team'::TEXT, 'Team'::TEXT, 'archived'::TEXT, 'leader'::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, 0::BIGINT, 1::BIGINT,
                    '[]'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_team_row(&invalid_status).expect_err("unknown status must fail");
        assert!(error.to_string().contains("parse PostgreSQL team status"));

        let invalid_leader = connection.query_one(
            "SELECT 'team'::TEXT, 'Team'::TEXT, 'active'::TEXT, ''::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, 0::BIGINT, 1::BIGINT,
                    '[]'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_team_row(&invalid_leader).expect_err("empty leader must fail");
        assert!(error.to_string().contains("parse PostgreSQL team leader"));

        let negative_ready = connection.query_one(
            "SELECT 'team'::TEXT, 'Team'::TEXT, 'active'::TEXT, 'leader'::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, -1::BIGINT, 1::BIGINT,
                    '[]'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_team_row(&negative_ready).expect_err("negative ready count must fail");
        assert!(error.to_string().contains("PostgreSQL team ready workers out of range"));

        let malformed_members = connection.query_one(
            "SELECT 'team'::TEXT, 'Team'::TEXT, 'active'::TEXT, 'leader'::TEXT,
                    NULL::TEXT, NULL::TEXT, FALSE::BOOL, 0::BIGINT, 1::BIGINT,
                    '{'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_team_row(&malformed_members).expect_err("bad members JSON must fail");
        assert!(error.to_string().contains("parse PostgreSQL team members"));
    }
    fixture.cleanup()
}
