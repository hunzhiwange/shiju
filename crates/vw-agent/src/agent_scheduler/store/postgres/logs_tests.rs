//! 测试 PostgreSQL 日志与工作区记录的行映射及运行归属校验。
//! 空运行标识、未知状态和非法任务或工作区标识均不得进入领域对象。

use anyhow::Result;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::ProjectId;

use super::{ensure_run_matches_task, map_log_row, map_workspace_row};
use crate::agent_scheduler::domain::TaskId;
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn postgres_log_mappers_preserve_status_keys_and_workspace_fields() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let (log, workspace) = {
        let mut connection = fixture.database().connect_postgres()?;
        let log_row = connection.query_one(
            "SELECT 'task-1'::TEXT, 'run-1'::TEXT, 3::BIGINT, 1200::BIGINT,
                    'running'::TEXT, 'submitted'::TEXT, 'custom-running'::TEXT,
                    'custom-submitted'::TEXT, 'submitted output'::TEXT",
            &[],
        )?;
        let workspace_row = connection.query_one(
            "SELECT 'task-1'::TEXT, 'run-1'::TEXT, 'slot-a'::TEXT,
                    '/tmp/worktree'::TEXT, 'codex/task-1'::TEXT, 1300::BIGINT",
            &[],
        )?;
        (map_log_row(&log_row)?, map_workspace_row(&workspace_row)?)
    };

    assert_eq!(log.task_id.as_str(), "task-1");
    assert_eq!(log.run_id.as_ref().map(|id| id.as_str()), Some("run-1"));
    assert_eq!(log.seq_no, 3);
    assert_eq!(log.timestamp_ms, TimestampMs(1_200));
    assert_eq!(log.from_status, Some(AgentTaskStatus::Running));
    assert_eq!(log.to_status, Some(AgentTaskStatus::Submitted));
    assert_eq!(log.from_status_key.as_deref(), Some("custom-running"));
    assert_eq!(log.to_status_key.as_deref(), Some("custom-submitted"));
    assert_eq!(log.message, "submitted output");
    assert_eq!(workspace.task_id.as_str(), "task-1");
    assert_eq!(workspace.run_id.as_str(), "run-1");
    assert_eq!(workspace.workspace.slot_id, "slot-a");
    assert_eq!(workspace.workspace.path, "/tmp/worktree");
    assert_eq!(workspace.workspace.branch, "codex/task-1");
    assert_eq!(workspace.recorded_at_ms, TimestampMs(1_300));

    fixture.cleanup()
}

#[test]
fn postgres_log_mapper_preserves_null_run_and_status_keys() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let log = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 'task-1'::TEXT, NULL::TEXT, 0::BIGINT, 99::BIGINT,
                    NULL::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT, 'message'::TEXT",
            &[],
        )?;
        map_log_row(&row)?
    };

    assert_eq!(log.task_id.as_str(), "task-1");
    assert_eq!(log.run_id, None);
    assert_eq!(log.from_status, None);
    assert_eq!(log.to_status, None);
    assert_eq!(log.from_status_key, None);
    assert_eq!(log.to_status_key, None);
    assert_eq!(log.message, "message");

    fixture.cleanup()
}

#[test]
fn postgres_log_mappers_reject_unknown_status_and_empty_run_id() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let unknown_status = connection.query_one(
            "SELECT 'task-1'::TEXT, NULL::TEXT, 0::BIGINT, 1::BIGINT,
                    'unknown'::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT, 'message'::TEXT",
            &[],
        )?;
        let error = map_log_row(&unknown_status).expect_err("unknown log status must fail");
        assert!(error.to_string().contains("parse PostgreSQL task log source status"));

        let empty_run = connection.query_one(
            "SELECT 'task-1'::TEXT, ''::TEXT, 'slot'::TEXT, '/tmp/worktree'::TEXT,
                    'branch'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_workspace_row(&empty_run).expect_err("empty workspace run id must fail");
        assert!(error.to_string().contains("parse PostgreSQL workspace run id"));
    }
    fixture.cleanup()
}

#[test]
fn postgres_log_mappers_reject_invalid_task_and_workspace_ids() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let empty_task_log = connection.query_one(
            "SELECT ''::TEXT, NULL::TEXT, 0::BIGINT, 1::BIGINT,
                    NULL::TEXT, NULL::TEXT, NULL::TEXT, NULL::TEXT, 'message'::TEXT",
            &[],
        )?;
        let error = map_log_row(&empty_task_log).expect_err("empty task id must fail");
        assert!(error.to_string().contains("parse PostgreSQL task log task id"));

        let empty_task_workspace = connection.query_one(
            "SELECT ''::TEXT, 'run-1'::TEXT, 'slot'::TEXT, '/tmp/worktree'::TEXT,
                    'branch'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_workspace_row(&empty_task_workspace)
            .expect_err("empty workspace task id must fail");
        assert!(error.to_string().contains("parse PostgreSQL workspace task id"));
    }
    fixture.cleanup()
}

#[test]
fn ensure_run_matches_task_accepts_absent_run_without_querying() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let scope = AgentSchedulerScope::for_default_tenant(ProjectId::from("logs-project"))?;
        let task_id = TaskId::new("task-1")?;

        ensure_run_matches_task(&mut *connection, &scope, None, &task_id, "task log")?;
    }
    fixture.cleanup()
}
