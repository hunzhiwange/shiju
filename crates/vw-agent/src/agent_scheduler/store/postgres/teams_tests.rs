//! 测试 PostgreSQL 团队行映射保留房间、就绪计数、成员与时间戳。
//! 空可选房间合法，非法标识、状态、计数和成员数据必须拒绝。

use anyhow::Result;
use vw_api_types::common::TimestampMs;

use super::map_team_row;
use crate::agent_scheduler::domain::AgentTeamStatus;
use crate::global_database::test_support::PostgresTestDatabase;


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
