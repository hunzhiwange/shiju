//! 测试 PostgreSQL 产物行映射及运行记录与任务归属校验。
//! 空可选字段、非法标识和畸形载荷分别覆盖宽松读取与拒绝边界。

use anyhow::Result;
use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::ProjectId;

use super::{ensure_run_matches_task, map_artifact_row};
use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn map_artifact_row_preserves_optional_run_path_payload_and_timestamp() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let artifact = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 7::BIGINT, 'task-1'::TEXT, 'run-1'::TEXT, 'log'::TEXT,
                    'output.txt'::TEXT, '/tmp/output.txt'::TEXT,
                    '{\"lines\":[\"a\",\"b\"]}'::TEXT, 1234::BIGINT",
            &[],
        )?;
        map_artifact_row(&row)?
    };

    assert_eq!(artifact.id, 7);
    assert_eq!(artifact.task_id.as_str(), "task-1");
    assert_eq!(artifact.run_id.as_ref().map(|id| id.as_str()), Some("run-1"));
    assert_eq!(artifact.kind, "log");
    assert_eq!(artifact.name, "output.txt");
    assert_eq!(artifact.path.as_deref(), Some("/tmp/output.txt"));
    assert_eq!(artifact.payload, json!({"lines": ["a", "b"]}));
    assert_eq!(artifact.created_at_ms, TimestampMs(1_234));

    fixture.cleanup()
}

#[test]
fn map_artifact_row_preserves_null_run_and_path() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let artifact = {
        let mut connection = fixture.database().connect_postgres()?;
        let row = connection.query_one(
            "SELECT 8::BIGINT, 'task-2'::TEXT, NULL::TEXT, 'json'::TEXT,
                    'payload'::TEXT, NULL::TEXT, '{}'::TEXT, 5678::BIGINT",
            &[],
        )?;
        map_artifact_row(&row)?
    };

    assert_eq!(artifact.task_id.as_str(), "task-2");
    assert_eq!(artifact.run_id, None);
    assert_eq!(artifact.path, None);
    assert_eq!(artifact.payload, json!({}));
    assert_eq!(artifact.created_at_ms, TimestampMs(5_678));

    fixture.cleanup()
}

#[test]
fn map_artifact_row_rejects_invalid_ids_and_payload() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let empty_task = connection.query_one(
            "SELECT 1::BIGINT, ''::TEXT, NULL::TEXT, 'kind'::TEXT,
                    'name'::TEXT, NULL::TEXT, '{}'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_artifact_row(&empty_task).expect_err("empty task id must fail");
        assert!(error.to_string().contains("parse PostgreSQL artifact task id"));

        let empty_run = connection.query_one(
            "SELECT 1::BIGINT, 'task-1'::TEXT, ''::TEXT, 'kind'::TEXT,
                    'name'::TEXT, NULL::TEXT, '{}'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_artifact_row(&empty_run).expect_err("empty run id must fail");
        assert!(error.to_string().contains("parse PostgreSQL artifact run id"));

        let malformed_payload = connection.query_one(
            "SELECT 1::BIGINT, 'task-1'::TEXT, NULL::TEXT, 'kind'::TEXT,
                    'name'::TEXT, NULL::TEXT, '{'::TEXT, 1::BIGINT",
            &[],
        )?;
        let error = map_artifact_row(&malformed_payload).expect_err("bad payload must fail");
        assert!(error.to_string().contains("parse PostgreSQL task artifact payload"));
    }
    fixture.cleanup()
}

#[test]
fn ensure_run_matches_task_accepts_absent_run_without_table_access() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        let scope = AgentSchedulerScope::for_default_tenant(ProjectId::from("artifacts-project"))?;
        let task_id = TaskId::new("task-1")?;

        ensure_run_matches_task(&mut *connection, &scope, None, &task_id)?;
    }
    fixture.cleanup()
}

#[test]
fn ensure_run_matches_task_validates_run_scope_and_task_binding() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        connection.batch_execute(
            "CREATE TABLE scheduler_runs (
                platform_id TEXT NOT NULL,
                company_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                id TEXT NOT NULL,
                task_id TEXT NOT NULL
            );",
        )?;
        let scope = AgentSchedulerScope::for_default_tenant(ProjectId::from("artifacts-project"))?;
        let project = scope.project_scope();
        connection.execute(
            "INSERT INTO scheduler_runs (platform_id, company_id, project_id, id, task_id)
             VALUES ($1, $2, $3, 'run-1', 'task-1')",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
            ],
        )?;

        let task_id = TaskId::new("task-1")?;
        let run_id = RunId::new("run-1")?;
        ensure_run_matches_task(&mut *connection, &scope, Some(&run_id), &task_id)?;

        let missing = RunId::new("missing")?;
        let error = ensure_run_matches_task(&mut *connection, &scope, Some(&missing), &task_id)
            .expect_err("missing run must fail");
        assert_eq!(error.to_string(), "task artifact run not found: missing");

        let other_task = TaskId::new("other-task")?;
        let error = ensure_run_matches_task(&mut *connection, &scope, Some(&run_id), &other_task)
            .expect_err("mismatched task must fail");
        assert_eq!(error.to_string(), "task artifact run run-1 does not belong to task other-task");
    }
    fixture.cleanup()
}
