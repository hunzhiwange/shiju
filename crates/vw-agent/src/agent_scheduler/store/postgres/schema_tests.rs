//! 测试 PostgreSQL 调度表结构、约束和迁移幂等性。

use anyhow::Result;

use super::{initialize_schema, reject_unsupported_schema};
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn reject_unsupported_schema_allows_empty_scheduler_schema() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        reject_unsupported_schema(&mut *connection)?;
    }
    fixture.cleanup()
}

#[test]
fn reject_unsupported_schema_rejects_legacy_unprefixed_tables() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        connection.batch_execute("CREATE TABLE tasks (id TEXT PRIMARY KEY);")?;

        let error = reject_unsupported_schema(&mut *connection)
            .expect_err("legacy scheduler table must be rejected");
        assert_eq!(error.to_string(), "unsupported unprefixed agent scheduler table: tasks");
    }
    fixture.cleanup()
}

#[test]
fn reject_unsupported_schema_rejects_current_tables_without_required_scope_columns() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    {
        let mut connection = fixture.database().connect_postgres()?;
        connection.batch_execute("CREATE TABLE scheduler_tasks (id TEXT PRIMARY KEY);")?;

        let error = reject_unsupported_schema(&mut *connection)
            .expect_err("unscoped current scheduler table must be rejected");
        assert_eq!(
            error.to_string(),
            "unsupported unscoped agent scheduler table: scheduler_tasks missing required column platform_id"
        );
    }
    fixture.cleanup()
}

#[test]
fn initialize_schema_creates_scoped_tables_indexes_and_is_idempotent() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database().clone();
    let _projects = ProjectStore::new(database.clone())?;
    {
        let mut connection = database.connect_postgres()?;
        initialize_schema(&mut *connection)?;
        initialize_schema(&mut *connection)?;

        for table in [
            "scheduler_tasks",
            "scheduler_runs",
            "scheduler_task_artifacts",
            "scheduler_teams",
            "scheduler_resource_leases",
        ] {
            let exists: bool = connection
                .query_one(
                    "SELECT EXISTS (
                         SELECT 1 FROM information_schema.tables
                         WHERE table_schema = current_schema()
                           AND table_name = $1
                           AND table_type = 'BASE TABLE'
                     )",
                    &[&table],
                )?
                .get(0);
            assert!(exists, "{table} should exist after schema initialization");

            for column in ["platform_id", "company_id", "project_id"] {
                let scoped_column: bool = connection
                    .query_one(
                        "SELECT EXISTS (
                             SELECT 1 FROM information_schema.columns
                             WHERE table_schema = current_schema()
                               AND table_name = $1
                               AND column_name = $2
                               AND is_nullable = 'NO'
                         )",
                        &[&table, &column],
                    )?
                    .get(0);
                assert!(scoped_column, "{table}.{column} should be a required scope column");
            }
        }

        for index in [
            "idx_scheduler_tasks_status",
            "idx_scheduler_task_artifacts_task",
            "idx_scheduler_resource_leases_active_resource",
        ] {
            let exists: bool = connection
                .query_one(
                    "SELECT EXISTS (
                         SELECT 1 FROM pg_indexes
                         WHERE schemaname = current_schema()
                           AND indexname = $1
                     )",
                    &[&index],
                )?
                .get(0);
            assert!(exists, "{index} should exist after schema initialization");
        }
    }
    fixture.cleanup()
}
