//! 测试 PostgreSQL 运行记录的原子领取、完成与过期查询。

use anyhow::Result;
use vw_api_types::common::TimestampMs;

use super::map_run_row;
use crate::agent_scheduler::domain::AgentRunStatus;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn postgres_run_mapper_preserves_all_run_fields() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let run = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'run-1'::TEXT, 'task-1'::TEXT, 'worker-1'::TEXT, 'running'::TEXT,
                    2::BIGINT, 1000::BIGINT, 2000::BIGINT, 1100::BIGINT, NULL::BIGINT,
                    1500::BIGINT, NULL::TEXT",
            &[],
        )?;
        map_run_row(&row)?
    };

    assert_eq!(run.id.as_str(), "run-1");
    assert_eq!(run.task_id.as_str(), "task-1");
    assert_eq!(run.worker_id.as_ref().map(|worker| worker.as_str()), Some("worker-1"));
    assert_eq!(run.status, AgentRunStatus::Running);
    assert_eq!(run.attempt, 2);
    assert_eq!(run.lease_acquired_at_ms, Some(TimestampMs(1_000)));
    assert_eq!(run.lease_expires_at_ms, Some(TimestampMs(2_000)));
    assert_eq!(run.started_at_ms, Some(TimestampMs(1_100)));
    assert_eq!(run.finished_at_ms, None);
    assert_eq!(run.last_heartbeat_at_ms, Some(TimestampMs(1_500)));
    assert_eq!(run.error_message, None);

    fixture.cleanup()
}

#[test]
fn postgres_run_mapper_preserves_null_worker_and_finished_error_fields() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let run = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'run-2'::TEXT, 'task-2'::TEXT, NULL::TEXT, 'failed'::TEXT,
                    1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, 3000::BIGINT,
                    NULL::BIGINT, 'boom'::TEXT",
            &[],
        )?;
        map_run_row(&row)?
    };

    assert_eq!(run.worker_id, None);
    assert_eq!(run.status, AgentRunStatus::Failed);
    assert_eq!(run.lease_acquired_at_ms, None);
    assert_eq!(run.lease_expires_at_ms, None);
    assert_eq!(run.started_at_ms, None);
    assert_eq!(run.finished_at_ms, Some(TimestampMs(3_000)));
    assert_eq!(run.last_heartbeat_at_ms, None);
    assert_eq!(run.error_message.as_deref(), Some("boom"));

    fixture.cleanup()
}

#[test]
fn postgres_run_mapper_rejects_invalid_ids_status_and_attempt() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let invalid_run = connection.query_one(
            "SELECT ''::TEXT, 'task-1'::TEXT, 'worker-1'::TEXT, 'running'::TEXT,
                    1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT",
            &[],
        )?;
        let error = map_run_row(&invalid_run).expect_err("empty run id should fail");
        assert!(error.to_string().contains("parse PostgreSQL run id"));

        let invalid_task = connection.query_one(
            "SELECT 'run-1'::TEXT, ''::TEXT, 'worker-1'::TEXT, 'running'::TEXT,
                    1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT",
            &[],
        )?;
        let error = map_run_row(&invalid_task).expect_err("empty task id should fail");
        assert!(error.to_string().contains("parse PostgreSQL run task id"));

        let invalid_worker = connection.query_one(
            "SELECT 'run-1'::TEXT, 'task-1'::TEXT, ''::TEXT, 'running'::TEXT,
                    1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT",
            &[],
        )?;
        let error = map_run_row(&invalid_worker).expect_err("empty worker id should fail");
        assert!(error.to_string().contains("parse PostgreSQL run worker"));

        let invalid_status = connection.query_one(
            "SELECT 'run-1'::TEXT, 'task-1'::TEXT, 'worker-1'::TEXT, 'unknown'::TEXT,
                    1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT",
            &[],
        )?;
        let error = map_run_row(&invalid_status).expect_err("unknown status should fail");
        assert!(error.to_string().contains("parse PostgreSQL run status"));

        let negative_attempt = connection.query_one(
            "SELECT 'run-1'::TEXT, 'task-1'::TEXT, 'worker-1'::TEXT, 'running'::TEXT,
                    -1::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT, NULL::BIGINT,
                    NULL::BIGINT, NULL::TEXT",
            &[],
        )?;
        let error = map_run_row(&negative_attempt).expect_err("negative attempt should fail");
        assert!(error.to_string().contains("PostgreSQL run attempt out of range"));
    }
    fixture.cleanup()
}
