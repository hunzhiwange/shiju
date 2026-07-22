use super::*;

use rusqlite::{Connection, params};
use vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
    TaskLifecycleWorkflowUpsertBody,
};

use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn task_lifecycle_postgres_provider_is_wired() {
    let _ = postgres::PostgresTaskLifecycleStore::new;
}

#[test]
fn normalize_custom_workflow_trims_label_and_marks_built_in_override() {
    let normalized =
        normalize_custom_workflow(custom_workflow("dev_lifecycle", "  Tenant Override  "))
            .expect("valid built-in override");

    assert_eq!(normalized.label, "Tenant Override");
    assert!(normalized.built_in);
}

#[test]
fn task_lifecycle_store_rejects_read_only_and_invalid_stored_workflows() {
    let read_only = reject_built_in_write("dev_lifecycle")
        .expect_err("built-in workflow writes should be rejected");
    assert!(
        read_only.to_string().contains("built-in lifecycle workflow is read-only: dev_lifecycle")
    );
    reject_built_in_write("custom").expect("custom workflow may be written");

    let mut invalid = custom_workflow("invalid", "Invalid");
    invalid.statuses.clear();
    let error = registry_from_custom_workflows(vec![invalid])
        .expect_err("invalid stored workflow should be rejected");
    assert!(error.to_string().contains(
        "stored task lifecycle workflow invalid: custom lifecycle workflow must contain at least one status: invalid"
    ));
}

#[test]
fn sqlite_helpers_report_malformed_json_and_clamp_negative_timestamps() {
    let conn = Connection::open_in_memory().expect("in-memory database");
    conn.execute_batch(TASK_LIFECYCLE_SCHEMA).expect("task lifecycle schema");
    let tenant = TenantScope::default();
    conn.execute(
        "INSERT INTO workflow_lifecycle_workflows \
         (platform_id, company_id, id, label, workflow_json, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, 'broken', 'Broken', '{', -1, 0)",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
    )
    .expect("malformed workflow row");

    let error = query_custom_workflows(&conn, &tenant)
        .expect_err("malformed workflow JSON should be rejected");
    assert!(error.to_string().contains("task lifecycle json error"));

    let timestamp = conn
        .query_row("SELECT -1", [], |row| read_u64(row, 0))
        .expect("negative timestamp should be readable");
    assert_eq!(timestamp, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_lifecycle_workflows_are_isolated_by_tenant() {
    let home = tempfile::tempdir().expect("temp home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    assert_workflow_lifecycle_workflows_are_isolated_by_tenant(database).await;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database_unit!();
    assert_workflow_lifecycle_workflows_are_isolated_by_tenant(postgres_fixture.database().clone())
        .await;
    postgres_fixture.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_workflow_lifecycle_workflows_are_isolated_by_tenant(database: GlobalDatabase) {
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-a", "company-b").expect("tenant B");
    let empty_tenant = TenantScope::new("platform-b", "company-a").expect("empty tenant");

    create_workflow(
        database.clone(),
        tenant_a.clone(),
        upsert(custom_workflow("shared", "Tenant A")),
    )
    .await
    .expect("create tenant A workflow");
    create_workflow(
        database.clone(),
        tenant_b.clone(),
        upsert(custom_workflow("shared", "Tenant B")),
    )
    .await
    .expect("create tenant B workflow");

    let tenant_a_list =
        list_workflows(database.clone(), tenant_a.clone()).await.expect("list tenant A workflows");
    let tenant_b_list =
        list_workflows(database.clone(), tenant_b.clone()).await.expect("list tenant B workflows");
    assert_eq!(custom_labels(&tenant_a_list), vec![("shared", "Tenant A")]);
    assert_eq!(custom_labels(&tenant_b_list), vec![("shared", "Tenant B")]);
    assert_eq!(
        workflow_by_id(database.clone(), tenant_a.clone(), "shared".into())
            .await
            .expect("load tenant A workflow")
            .label,
        "Tenant A"
    );
    assert_eq!(
        workflow_by_id(database.clone(), tenant_b.clone(), "shared".into())
            .await
            .expect("load tenant B workflow")
            .label,
        "Tenant B"
    );

    update_workflow(
        database.clone(),
        tenant_a.clone(),
        "shared".into(),
        upsert(custom_workflow("shared", "Tenant A Updated")),
    )
    .await
    .expect("update tenant A workflow");
    assert_eq!(
        workflow_by_id(database.clone(), tenant_b.clone(), "shared".into())
            .await
            .expect("tenant B workflow remains")
            .label,
        "Tenant B"
    );

    update_workflow(
        database.clone(),
        tenant_a.clone(),
        "dev_lifecycle".into(),
        upsert(custom_workflow("dev_lifecycle", "Tenant A Built-in Override")),
    )
    .await
    .expect("override tenant A built-in");
    update_workflow(
        database.clone(),
        tenant_b.clone(),
        "dev_lifecycle".into(),
        upsert(custom_workflow("dev_lifecycle", "Tenant B Built-in Override")),
    )
    .await
    .expect("override tenant B built-in");
    let reset = reset_workflow(database.clone(), tenant_a.clone(), "dev_lifecycle".into())
        .await
        .expect("reset tenant A built-in");
    assert_eq!(reset.label, "开发生命周期");
    assert_eq!(
        workflow_by_id(database.clone(), tenant_b.clone(), "dev_lifecycle".into())
            .await
            .expect("tenant B override remains")
            .label,
        "Tenant B Built-in Override"
    );

    let deleted = delete_workflow(database.clone(), tenant_a.clone(), "shared".into())
        .await
        .expect("delete tenant A workflow");
    assert!(deleted.deleted);
    assert!(workflow_by_id(database.clone(), tenant_a, "shared".into()).await.is_err());
    assert_eq!(
        workflow_by_id(database.clone(), tenant_b, "shared".into())
            .await
            .expect("tenant B workflow survives tenant A delete")
            .label,
        "Tenant B"
    );

    let empty_workflows =
        list_workflows(database.clone(), empty_tenant).await.expect("list empty tenant workflows");
    assert!(!empty_workflows.is_empty());
    assert!(empty_workflows.iter().all(|workflow| workflow.built_in));

    let rows: i64 = match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("shared SQLite database connection")
            .query_row(
                "SELECT COUNT(*) FROM workflow_lifecycle_workflows WHERE id = 'shared'",
                [],
                |row| row.get(0),
            )
            .expect("count scoped SQLite workflow rows"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("shared PostgreSQL database connection")
            .query_one("SELECT COUNT(*) FROM workflow_lifecycle_workflows WHERE id = 'shared'", &[])
            .expect("count scoped PostgreSQL workflow rows")
            .get(0),
    };
    assert_eq!(rows, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_store_ignores_legacy_table_and_file_without_mutation() {
    let home = tempfile::tempdir().expect("temp home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let connection = database.connect().expect("shared database connection");
    connection
        .execute_batch(
            "CREATE TABLE task_lifecycle_workflows (
                 id TEXT PRIMARY KEY NOT NULL,
                 label TEXT NOT NULL,
                 workflow_json TEXT NOT NULL,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL
             );
             INSERT INTO task_lifecycle_workflows
                 (id, label, workflow_json, created_at_ms, updated_at_ms)
             VALUES ('legacy-table', 'Legacy Table', '{}', 1, 2);",
        )
        .expect("legacy task lifecycle table");
    drop(connection);
    let legacy_path =
        vw_config_types::paths::home_config_dir(home.path()).join("task-workflows.json");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy file parent"))
        .expect("create legacy file parent");
    let legacy_contents = br#"[{"id":"legacy-file","label":"Legacy File","statuses":[]}]"#;
    std::fs::write(&legacy_path, legacy_contents).expect("write legacy file");

    let workflows = list_workflows(database.clone(), TenantScope::default())
        .await
        .expect("list workflows without legacy import");
    assert!(workflows.iter().all(|workflow| workflow.built_in));
    assert_eq!(std::fs::read(&legacy_path).expect("read unchanged legacy file"), legacy_contents);

    let connection = database.connect().expect("shared database connection");
    let legacy_row: (String, String) = connection
        .query_row(
            "SELECT label, workflow_json FROM task_lifecycle_workflows WHERE id = 'legacy-table'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("legacy table row remains");
    assert_eq!(legacy_row, ("Legacy Table".into(), "{}".into()));
    assert!(column_exists(&connection, "workflow_lifecycle_workflows", "platform_id").unwrap());
    assert!(column_exists(&connection, "workflow_lifecycle_workflows", "company_id").unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_sqlite_store_reports_write_conflicts_and_invalid_requests() {
    let home = tempfile::tempdir().expect("temp home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    let tenant = TenantScope::default();

    create_workflow(
        database.clone(),
        tenant.clone(),
        upsert(custom_workflow("custom_conflict", "Custom Conflict")),
    )
    .await
    .expect("create custom workflow");
    let duplicate = create_workflow(
        database.clone(),
        tenant.clone(),
        upsert(custom_workflow("custom_conflict", "Duplicate")),
    )
    .await
    .expect_err("duplicate custom workflow should be rejected");
    assert!(
        duplicate
            .to_string()
            .contains("custom lifecycle workflow id already exists: custom_conflict")
    );

    let mismatch = update_workflow(
        database.clone(),
        tenant.clone(),
        "custom_conflict".to_string(),
        upsert(custom_workflow("different_id", "Mismatch")),
    )
    .await
    .expect_err("workflow id mismatch should be rejected");
    assert!(mismatch.to_string().contains("task lifecycle workflow id mismatch"));

    let missing = update_workflow(
        database.clone(),
        tenant.clone(),
        "missing_custom".to_string(),
        upsert(custom_workflow("missing_custom", "Missing")),
    )
    .await
    .expect_err("missing custom workflow should be rejected");
    assert!(missing.to_string().contains("task lifecycle workflow not found: missing_custom"));

    let reset_custom = reset_workflow(database, tenant, "custom_conflict".to_string())
        .await
        .expect_err("custom workflow reset should be rejected");
    assert!(reset_custom.to_string().contains(
        "task lifecycle workflow reset is only supported for built-in workflow: custom_conflict"
    ));
}

#[test]
fn sqlite_query_single_workflow_reports_malformed_json_and_error_mappers_are_stable() {
    let conn = Connection::open_in_memory().expect("in-memory database");
    conn.execute_batch(TASK_LIFECYCLE_SCHEMA).expect("task lifecycle schema");
    let tenant = TenantScope::default();
    conn.execute(
        "INSERT INTO workflow_lifecycle_workflows \
         (platform_id, company_id, id, label, workflow_json, created_at_ms, updated_at_ms) \
         VALUES (?1, ?2, 'broken-one', 'Broken One', '{', 1, 2)",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
    )
    .expect("malformed workflow row");

    let error = query_custom_workflow(&conn, &tenant, "broken-one")
        .expect_err("malformed single workflow JSON should be rejected");
    assert!(error.to_string().contains("task lifecycle database error"));
    assert!(
        sql_error(rusqlite::Error::InvalidQuery)
            .to_string()
            .contains("task lifecycle database error")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_postgres_store_ignores_legacy_table_without_mutation() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    crate::global_database::run_sync_database(|| {
        let mut connection = database.connect_postgres().expect("PostgreSQL connection");
        connection
            .batch_execute(
                "CREATE TABLE task_lifecycle_workflows (
                     id TEXT PRIMARY KEY NOT NULL,
                     label TEXT NOT NULL,
                     workflow_json TEXT NOT NULL,
                     created_at_ms BIGINT NOT NULL,
                     updated_at_ms BIGINT NOT NULL
                 );
                 INSERT INTO task_lifecycle_workflows
                     (id, label, workflow_json, created_at_ms, updated_at_ms)
                 VALUES ('legacy-table', 'Legacy Table', '{}', 1, 2);",
            )
            .expect("legacy PostgreSQL task lifecycle table");
        Ok::<(), anyhow::Error>(())
    })
    .expect("prepare PostgreSQL legacy table");

    let workflows = list_workflows(database.clone(), TenantScope::default())
        .await
        .expect("list workflows without legacy import");
    assert!(workflows.iter().all(|workflow| workflow.built_in));

    let legacy_row = crate::global_database::run_sync_database(|| {
        let mut connection = database
            .connect_postgres()
            .expect("PostgreSQL legacy table connection");
        let row = connection
            .query_one(
                "SELECT label, workflow_json FROM task_lifecycle_workflows WHERE id = 'legacy-table'",
                &[],
            )
            .expect("legacy PostgreSQL table row remains");
        Ok::<_, anyhow::Error>((row.get::<_, String>(0), row.get::<_, String>(1)))
    })
    .expect("read PostgreSQL legacy table");
    assert_eq!(legacy_row.0, "Legacy Table");
    assert_eq!(legacy_row.1, "{}");
    crate::global_database::run_sync_database(|| fixture.cleanup())
        .expect("clean PostgreSQL test database");
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_store_rejects_unscoped_new_table_without_mutation() {
    let path = tempfile::tempdir().expect("temp dir").keep().join("lifecycle.sqlite");
    let connection = Connection::open(&path).expect("legacy database");
    connection
        .execute_batch(
            "CREATE TABLE workflow_lifecycle_workflows (
                 id TEXT PRIMARY KEY NOT NULL,
                 label TEXT NOT NULL,
                 workflow_json TEXT NOT NULL,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL
             );
             INSERT INTO workflow_lifecycle_workflows
                 (id, label, workflow_json, created_at_ms, updated_at_ms)
             VALUES ('legacy', 'Preserve Me', '{}', 1, 2);",
        )
        .expect("unscoped workflow lifecycle schema");
    drop(connection);
    let database = GlobalDatabase::new(path).expect("global database");

    let error = list_workflows(database.clone(), TenantScope::default())
        .await
        .expect_err("unscoped schema must be rejected");
    assert!(error.to_string().contains(
        "unsupported unscoped task lifecycle schema: table workflow_lifecycle_workflows must contain platform_id and company_id"
    ));

    let connection = database.connect().expect("unchanged database");
    let label: String = connection
        .query_row(
            "SELECT label FROM workflow_lifecycle_workflows WHERE id = 'legacy'",
            [],
            |row| row.get(0),
        )
        .expect("legacy row remains");
    assert_eq!(label, "Preserve Me");
    assert!(!column_exists(&connection, "workflow_lifecycle_workflows", "platform_id").unwrap());
}

#[test]
fn task_lifecycle_postgres_store_rejects_unscoped_new_table_without_mutation() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    let mut connection = database.connect_postgres().expect("PostgreSQL connection");
    connection
        .batch_execute(
            "CREATE TABLE workflow_lifecycle_workflows (
                 id TEXT PRIMARY KEY NOT NULL,
                 label TEXT NOT NULL,
                 workflow_json TEXT NOT NULL,
                 created_at_ms BIGINT NOT NULL,
                 updated_at_ms BIGINT NOT NULL
             );
             INSERT INTO workflow_lifecycle_workflows
                 (id, label, workflow_json, created_at_ms, updated_at_ms)
             VALUES ('legacy', 'Preserve Me', '{}', 1, 2);",
        )
        .expect("unscoped PostgreSQL workflow lifecycle schema");
    drop(connection);

    let error = list_workflows_blocking(database.clone(), &TenantScope::default())
        .expect_err("unscoped PostgreSQL schema must be rejected");
    assert!(error.to_string().contains(
        "unsupported unscoped task lifecycle schema: table workflow_lifecycle_workflows must contain platform_id and company_id"
    ));

    let row = database
        .connect_postgres()
        .expect("unchanged PostgreSQL database")
        .query_one("SELECT label FROM workflow_lifecycle_workflows WHERE id = 'legacy'", &[])
        .expect("legacy PostgreSQL row remains");
    assert_eq!(row.get::<_, String>(0), "Preserve Me");
    fixture.cleanup().expect("clean PostgreSQL test database");
}

fn upsert(workflow: TaskLifecycleWorkflowDto) -> TaskLifecycleWorkflowUpsertBody {
    TaskLifecycleWorkflowUpsertBody { workflow }
}

fn custom_labels(workflows: &[TaskLifecycleWorkflowDto]) -> Vec<(&str, &str)> {
    workflows
        .iter()
        .filter(|workflow| !workflow.built_in)
        .map(|workflow| (workflow.id.as_str(), workflow.label.as_str()))
        .collect()
}

fn custom_workflow(id: &str, label: &str) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: id.to_string(),
        label: label.to_string(),
        built_in: false,
        statuses: vec![
            status("queued", "Queued", TaskLifecycleStatusRoleDto::Queued, false, Some("running")),
            status(
                "running",
                "Running",
                TaskLifecycleStatusRoleDto::Executing,
                false,
                Some("done"),
            ),
            status("done", "Done", TaskLifecycleStatusRoleDto::Done, true, None),
        ],
    }
}

fn status(
    key: &str,
    label: &str,
    role: TaskLifecycleStatusRoleDto,
    terminal: bool,
    next: Option<&str>,
) -> TaskLifecycleStatusDto {
    TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal,
        auto_execution_candidate: !terminal,
        occupies_execution_slot: role == TaskLifecycleStatusRoleDto::Executing,
        is_scheduling_backlog: role == TaskLifecycleStatusRoleDto::Queued,
        next: next.map(str::to_string),
        transitions: Vec::new(),
    }
}
