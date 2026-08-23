//! 测试调度模式在 SQLite 与 PostgreSQL 上的命名、约束和幂等初始化。

use anyhow::Result;
use rusqlite::{Connection, params};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info, TimeInfo};

use super::domain::AgentSchedulerContext;
use super::store::AgentSchedulerStore;
use super::store::schema::initialize_schema;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

const SQLITE_SCHEDULER_TABLES: [&str; 12] = [
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_task_dependencies",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "scheduler_events",
    "scheduler_outbox_messages",
];

const POSTGRES_SCHEDULER_TABLES: [&str; 12] = [
    "scheduler_meta",
    "scheduler_tasks",
    "scheduler_task_dependencies",
    "scheduler_runs",
    "scheduler_task_logs",
    "scheduler_task_artifacts",
    "scheduler_run_workspaces",
    "scheduler_workers",
    "scheduler_teams",
    "scheduler_resource_leases",
    "scheduler_events",
    "scheduler_outbox_messages",
];

const LEGACY_SCHEDULER_TABLES: [&str; 10] = [
    "tasks",
    "runs",
    "task_logs",
    "task_artifacts",
    "run_workspaces",
    "events",
    "outbox_messages",
    "workers",
    "teams",
    "resource_leases",
];

const SQLITE_SCHEDULER_PRIMARY_KEYS: [(&str, &[&str]); 12] = [
    ("scheduler_meta", &["platform_id", "company_id", "project_id", "key"]),
    ("scheduler_tasks", &["platform_id", "company_id", "project_id", "id"]),
    (
        "scheduler_task_dependencies",
        &["platform_id", "company_id", "project_id", "predecessor_task_id", "successor_task_id"],
    ),
    ("scheduler_runs", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_task_logs", &["platform_id", "company_id", "project_id", "task_id", "seq_no"]),
    ("scheduler_task_artifacts", &["id"]),
    ("scheduler_run_workspaces", &["platform_id", "company_id", "project_id", "run_id"]),
    ("scheduler_workers", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_teams", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_resource_leases", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_events", &["id"]),
    ("scheduler_outbox_messages", &["platform_id", "company_id", "project_id", "id"]),
];

const POSTGRES_SCHEDULER_PRIMARY_KEYS: [(&str, &[&str]); 12] = [
    ("scheduler_meta", &["platform_id", "company_id", "project_id", "key"]),
    ("scheduler_tasks", &["platform_id", "company_id", "project_id", "id"]),
    (
        "scheduler_task_dependencies",
        &["platform_id", "company_id", "project_id", "predecessor_task_id", "successor_task_id"],
    ),
    ("scheduler_runs", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_task_logs", &["platform_id", "company_id", "project_id", "task_id", "seq_no"]),
    ("scheduler_task_artifacts", &["id"]),
    ("scheduler_run_workspaces", &["platform_id", "company_id", "project_id", "run_id"]),
    ("scheduler_workers", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_teams", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_resource_leases", &["platform_id", "company_id", "project_id", "id"]),
    ("scheduler_events", &["id"]),
    ("scheduler_outbox_messages", &["platform_id", "company_id", "project_id", "id"]),
];

#[test]
fn agent_scheduler_initialize_schema_is_idempotent_for_in_memory_database() -> Result<()> {
    let conn = Connection::open_in_memory()?;

    initialize_schema(&conn)?;
    initialize_schema(&conn)?;

    let scheduler_table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name LIKE 'scheduler_%'",
        [],
        |row| row.get(0),
    )?;
    let foreign_keys_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    assert_eq!(scheduler_table_count, SQLITE_SCHEDULER_TABLES.len() as i64);
    assert_eq!(foreign_keys_enabled, 1);

    Ok(())
}

#[test]
fn agent_scheduler_schema_uses_only_prefixed_scheduler_tables() -> Result<()> {
    assert_sqlite_scheduler_schema()?;
    assert_postgres_scheduler_schema()?;
    Ok(())
}

fn assert_sqlite_scheduler_schema() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let home = temp_dir.path().join("home");
    let project_path = temp_dir.path().join("project");
    std::fs::create_dir_all(&project_path)?;
    let db_path = vw_config_types::paths::global_database_path(&home);
    let project_store = ProjectStore::new(GlobalDatabase::new(&db_path)?)?;
    project_store.create(
        &TenantScope::default(),
        &Info {
            id: "test-project".to_owned(),
            worktree: project_path.to_string_lossy().into_owned(),
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let store = AgentSchedulerStore::for_home(&home);
    let context = AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(
            TenantScope::default(),
            ProjectId::from("test-project"),
        )?),
        &project_path,
    )?;

    store.initialize_project(&context)?;

    let conn = Connection::open(&db_path)?;

    assert_sqlite_object_exists(&conn, "table", "projects")?;
    for table in SQLITE_SCHEDULER_TABLES {
        assert_sqlite_object_exists(&conn, "table", table)?;
    }

    for index in [
        "idx_scheduler_tasks_status",
        "idx_scheduler_tasks_assignee",
        "idx_scheduler_task_dependencies_predecessor",
        "idx_scheduler_task_dependencies_successor",
        "idx_scheduler_runs_lease_expiry",
        "idx_scheduler_task_logs_task_seq",
        "idx_scheduler_task_artifacts_task",
        "idx_scheduler_run_workspaces_task",
        "idx_scheduler_workers_runtime_status",
        "idx_scheduler_workers_container_state",
        "idx_scheduler_resource_leases_active_resource",
        "idx_scheduler_outbox_delivery_status",
    ] {
        assert_sqlite_object_exists(&conn, "index", index)?;
    }

    for table in SQLITE_SCHEDULER_TABLES {
        assert_scope_columns(&conn, table)?;
        assert_project_restrict_foreign_key(&conn, table)?;
    }
    for (table, expected_columns) in SQLITE_SCHEDULER_PRIMARY_KEYS {
        assert_sqlite_primary_key(&conn, table, expected_columns)?;
    }

    for table in LEGACY_SCHEDULER_TABLES {
        assert_sqlite_object_missing(&conn, "table", table)?;
    }

    conn.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["local", "local", "test-project", "schema_test_marker", "kept"],
    )?;
    drop(conn);

    store.initialize_project(&context)?;

    let conn = Connection::open(db_path)?;
    let marker: String = conn.query_row(
        "SELECT value FROM scheduler_meta
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND key = ?4",
        params!["local", "local", "test-project", "schema_test_marker"],
        |row| row.get(0),
    )?;
    assert_eq!(marker, "kept");

    Ok(())
}

fn assert_postgres_scheduler_schema() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database().clone();
    let project_path = "postgres-scheduler-schema-project".to_owned();
    let project_store = ProjectStore::new(database.clone())?;
    project_store.create(
        &TenantScope::default(),
        &Info {
            id: "test-project".to_owned(),
            worktree: project_path.clone(),
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let store = AgentSchedulerStore::from_database(database.clone());
    let context = scheduler_context(&project_path)?;

    store.initialize_project(&context)?;

    let mut connection = database.connect_postgres()?;
    let table_names = connection
        .query(
            "SELECT table_name
             FROM information_schema.tables
             WHERE table_schema = current_schema()
               AND table_type = 'BASE TABLE'
               AND table_name LIKE 'scheduler_%'
             ORDER BY table_name",
            &[],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    let mut expected_tables = POSTGRES_SCHEDULER_TABLES.map(str::to_owned).to_vec();
    expected_tables.sort();
    assert_eq!(table_names, expected_tables);

    for table in POSTGRES_SCHEDULER_TABLES {
        assert_postgres_scope_columns(&mut *connection, table)?;
        assert_postgres_project_restrict_foreign_key(&mut *connection, table)?;
    }
    for (table, expected_columns) in POSTGRES_SCHEDULER_PRIMARY_KEYS {
        assert_postgres_primary_key(&mut *connection, table, expected_columns)?;
    }
    assert_postgres_task_dependency_foreign_keys(&mut *connection)?;
    for index in
        ["idx_scheduler_task_dependencies_predecessor", "idx_scheduler_task_dependencies_successor"]
    {
        assert_postgres_object_exists(&mut *connection, "index", index)?;
    }
    for table in LEGACY_SCHEDULER_TABLES {
        assert_postgres_object_missing(&mut *connection, table)?;
    }

    connection.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES ($1, $2, $3, $4, $5)",
        &[&"local", &"local", &"test-project", &"schema_test_marker", &"kept"],
    )?;
    drop(connection);

    store.initialize_project(&context)?;

    let mut connection = database.connect_postgres()?;
    let marker: String = connection
        .query_one(
            "SELECT value FROM scheduler_meta
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND key = $4",
            &[&"local", &"local", &"test-project", &"schema_test_marker"],
        )?
        .get(0);
    assert_eq!(marker, "kept");
    drop(connection);
    fixture.cleanup()
}

#[test]
fn agent_scheduler_schema_rejects_unprefixed_scheduler_tables() -> Result<()> {
    assert_sqlite_rejects_unprefixed_scheduler_tables()?;
    assert_postgres_rejects_unprefixed_scheduler_tables()?;
    Ok(())
}

#[test]
fn agent_scheduler_postgres_schema_rejects_unscoped_scheduler_table() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database().clone();
    ProjectStore::new(database.clone())?;
    let mut connection = database.connect_postgres()?;
    connection.batch_execute(
        "CREATE TABLE scheduler_tasks (marker TEXT NOT NULL);
         INSERT INTO scheduler_tasks (marker) VALUES ('kept');",
    )?;
    drop(connection);

    let store = AgentSchedulerStore::from_database(database.clone());
    let context = scheduler_context("postgres-unscoped-scheduler-project")?;
    let error = store
        .initialize_project(&context)
        .expect_err("unscoped PostgreSQL scheduler table must be rejected");
    assert!(error.to_string().contains("scheduler_tasks"));
    assert!(error.to_string().contains("platform_id"));

    let mut connection = database.connect_postgres()?;
    let marker: String = connection.query_one("SELECT marker FROM scheduler_tasks", &[])?.get(0);
    assert_eq!(marker, "kept");
    drop(connection);
    fixture.cleanup()
}

fn assert_sqlite_rejects_unprefixed_scheduler_tables() -> Result<()> {
    for legacy_table in LEGACY_SCHEDULER_TABLES {
        let temp_dir = tempfile::tempdir()?;
        let home = temp_dir.path().join("home");
        let project_path = temp_dir.path().join("project");
        std::fs::create_dir_all(&project_path)?;
        let db_path = vw_config_types::paths::global_database_path(&home);
        let project_store = ProjectStore::new(GlobalDatabase::new(&db_path)?)?;
        project_store.create(
            &TenantScope::default(),
            &Info {
                id: "test-project".to_owned(),
                worktree: project_path.to_string_lossy().into_owned(),
                vcs: None,
                name: None,
                icon: None,
                commands: None,
                time: TimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(&format!(
            "CREATE TABLE {legacy_table} (marker TEXT NOT NULL);\
             INSERT INTO {legacy_table} (marker) VALUES ('kept');"
        ))?;
        drop(conn);

        let store = AgentSchedulerStore::for_home(&home);
        let context = AgentSchedulerContext::new(
            AgentSchedulerScope::new(ProjectScope::new(
                TenantScope::default(),
                ProjectId::from("test-project"),
            )?),
            &project_path,
        )?;

        let error = store
            .initialize_project(&context)
            .expect_err("unprefixed scheduler table must be rejected");
        assert!(
            error.to_string().contains(legacy_table),
            "error must identify rejected table {legacy_table}: {error:#}"
        );

        let conn = Connection::open(&db_path)?;
        let marker: String =
            conn.query_row(&format!("SELECT marker FROM {legacy_table}"), [], |row| row.get(0))?;
        assert_eq!(marker, "kept", "legacy table {legacy_table} was modified");
        for current_table in SQLITE_SCHEDULER_TABLES {
            assert_sqlite_object_missing(&conn, "table", current_table)?;
        }
    }

    Ok(())
}

fn assert_postgres_rejects_unprefixed_scheduler_tables() -> Result<()> {
    for legacy_table in LEGACY_SCHEDULER_TABLES {
        let fixture = crate::global_database::test_support::postgres_test_database!();
        let database = fixture.database().clone();
        ProjectStore::new(database.clone())?;
        let mut connection = database.connect_postgres()?;
        connection.batch_execute(&format!(
            "CREATE TABLE {legacy_table} (marker TEXT NOT NULL);\
             INSERT INTO {legacy_table} (marker) VALUES ('kept');"
        ))?;
        drop(connection);

        let store = AgentSchedulerStore::from_database(database.clone());
        let context = scheduler_context("postgres-legacy-scheduler-project")?;
        let error = store
            .initialize_project(&context)
            .expect_err("unprefixed PostgreSQL scheduler table must be rejected");
        assert!(
            error.to_string().contains(legacy_table),
            "error must identify rejected PostgreSQL table {legacy_table}: {error:#}"
        );

        let mut connection = database.connect_postgres()?;
        let marker: String =
            connection.query_one(&format!("SELECT marker FROM {legacy_table}"), &[])?.get(0);
        assert_eq!(marker, "kept", "legacy PostgreSQL table {legacy_table} was modified");
        for current_table in POSTGRES_SCHEDULER_TABLES {
            assert_postgres_object_missing(&mut *connection, current_table)?;
        }
        drop(connection);
        fixture.cleanup()?;
    }
    Ok(())
}

fn scheduler_context(project_path: impl AsRef<std::path::Path>) -> Result<AgentSchedulerContext> {
    AgentSchedulerContext::new(
        AgentSchedulerScope::new(ProjectScope::new(
            TenantScope::default(),
            ProjectId::from("test-project"),
        )?),
        project_path.as_ref().to_path_buf(),
    )
}

fn assert_scope_columns(conn: &Connection, table: &str) -> Result<()> {
    for column in ["platform_id", "company_id", "project_id"] {
        let found: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('{table}')
                 WHERE name = ?1 AND \"notnull\" = 1"
            ),
            [column],
            |row| row.get(0),
        )?;
        assert_eq!(found, 1, "missing required scope column {table}.{column}");
    }
    Ok(())
}

fn assert_sqlite_object_exists(conn: &Connection, object_type: &str, name: &str) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        params![object_type, name],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1, "missing sqlite {object_type}: {name}");
    Ok(())
}

fn assert_sqlite_object_missing(conn: &Connection, object_type: &str, name: &str) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        params![object_type, name],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0, "unexpected sqlite {object_type}: {name}");
    Ok(())
}

fn assert_project_restrict_foreign_key(conn: &Connection, table: &str) -> Result<()> {
    let mut statement = conn.prepare(&format!("PRAGMA foreign_key_list('{table}')"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let project_columns = rows
        .iter()
        .filter(|(parent, _, _, action)| parent == "projects" && action == "RESTRICT")
        .map(|(_, child, parent, _)| (child.as_str(), parent.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        project_columns,
        vec![
            ("platform_id", "platform_id"),
            ("company_id", "company_id"),
            ("project_id", "project_id"),
        ],
        "missing project RESTRICT foreign key on {table}"
    );
    Ok(())
}

fn assert_sqlite_primary_key(
    connection: &Connection,
    table: &str,
    expected_columns: &[&str],
) -> Result<()> {
    let mut statement = connection.prepare(&format!(
        "SELECT name FROM pragma_table_info('{table}') WHERE pk > 0 ORDER BY pk"
    ))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assert_eq!(columns, expected_columns, "unexpected SQLite primary key on {table}");
    Ok(())
}

fn assert_postgres_scope_columns(
    connection: &mut impl postgres::GenericClient,
    table: &str,
) -> Result<()> {
    for column in ["platform_id", "company_id", "project_id"] {
        let found: i64 = connection
            .query_one(
                "SELECT COUNT(*)
                 FROM information_schema.columns
                 WHERE table_schema = current_schema()
                   AND table_name = $1 AND column_name = $2 AND is_nullable = 'NO'",
                &[&table, &column],
            )?
            .get(0);
        assert_eq!(found, 1, "missing required PostgreSQL scope column {table}.{column}");
    }
    Ok(())
}

fn assert_postgres_object_missing(
    connection: &mut impl postgres::GenericClient,
    table: &str,
) -> Result<()> {
    let found: bool = connection
        .query_one(
            "SELECT EXISTS(
                 SELECT 1 FROM pg_catalog.pg_class AS class
                 JOIN pg_catalog.pg_namespace AS namespace
                   ON namespace.oid = class.relnamespace
                 WHERE namespace.nspname = current_schema()
                   AND class.relname = $1 AND class.relkind IN ('r', 'p')
             )",
            &[&table],
        )?
        .get(0);
    assert!(!found, "unexpected PostgreSQL scheduler table: {table}");
    Ok(())
}

fn assert_postgres_object_exists(
    connection: &mut impl postgres::GenericClient,
    object_type: &str,
    name: &str,
) -> Result<()> {
    let relation_kind = match object_type {
        "index" => "i",
        other => anyhow::bail!("unsupported PostgreSQL object type: {other}"),
    };
    let found: bool = connection
        .query_one(
            "SELECT EXISTS(
                 SELECT 1
                 FROM pg_catalog.pg_class AS class
                 JOIN pg_catalog.pg_namespace AS namespace
                   ON namespace.oid = class.relnamespace
                 WHERE namespace.nspname = current_schema()
                   AND class.relname = $1
                   AND class.relkind = $2
             )",
            &[&name, &relation_kind],
        )?
        .get(0);
    assert!(found, "missing PostgreSQL {object_type}: {name}");
    Ok(())
}

fn assert_postgres_task_dependency_foreign_keys(
    connection: &mut impl postgres::GenericClient,
) -> Result<()> {
    let definitions = connection
        .query(
            "SELECT pg_catalog.pg_get_constraintdef(constraint.oid)
             FROM pg_catalog.pg_constraint AS constraint
             WHERE constraint.conrelid = 'scheduler_task_dependencies'::regclass
               AND constraint.contype = 'f'",
            &[],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    for column in ["predecessor_task_id", "successor_task_id"] {
        assert!(
            definitions.iter().any(|definition| {
                definition.contains(&format!(
                    "FOREIGN KEY (platform_id, company_id, project_id, {column})"
                )) && definition
                    .contains("REFERENCES scheduler_tasks(platform_id, company_id, project_id, id)")
                    && definition.contains("ON DELETE CASCADE")
            }),
            "missing PostgreSQL scoped {column} foreign key: {definitions:?}"
        );
    }
    Ok(())
}

fn assert_postgres_project_restrict_foreign_key(
    connection: &mut impl postgres::GenericClient,
    table: &str,
) -> Result<()> {
    let definitions = connection
        .query(
            "SELECT pg_catalog.pg_get_constraintdef(constraint.oid)
             FROM pg_catalog.pg_constraint AS constraint
             WHERE constraint.conrelid = to_regclass($1)
               AND constraint.contype = 'f'
               AND constraint.confrelid = to_regclass('projects')",
            &[&table],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    assert!(
        definitions.iter().any(|definition| {
            definition.contains("FOREIGN KEY (platform_id, company_id, project_id)")
                && definition.contains("REFERENCES projects(platform_id, company_id, project_id)")
                && definition.contains("ON DELETE RESTRICT")
        }),
        "missing PostgreSQL project RESTRICT foreign key on {table}: {definitions:?}"
    );
    Ok(())
}

fn assert_postgres_primary_key(
    connection: &mut impl postgres::GenericClient,
    table: &str,
    expected_columns: &[&str],
) -> Result<()> {
    let columns = connection
        .query(
            "SELECT attribute.attname
             FROM pg_catalog.pg_constraint AS constraint
             CROSS JOIN LATERAL unnest(constraint.conkey)
                 WITH ORDINALITY AS key(column_number, position)
             JOIN pg_catalog.pg_attribute AS attribute
               ON attribute.attrelid = constraint.conrelid
              AND attribute.attnum = key.column_number
             WHERE constraint.conrelid = to_regclass($1)
               AND constraint.contype = 'p'
             ORDER BY key.position",
            &[&table],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    assert_eq!(columns, expected_columns, "unexpected PostgreSQL primary key on {table}");
    Ok(())
}
