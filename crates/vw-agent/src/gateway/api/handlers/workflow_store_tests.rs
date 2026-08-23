// 验证工作流存储的后端装配、作用域约束与数据规范化行为。
use super::*;
use axum::http::StatusCode;
use tempfile::TempDir;

use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn workflow_postgres_provider_is_wired() {
    let _ = postgres::PostgresWorkflowStore::new;
}

fn body(name: &str, workflow_yaml: &str) -> WorkflowRecordUpsertBody {
    WorkflowRecordUpsertBody {
        uuid: None,
        name: name.to_string(),
        description: "  description  ".to_string(),
        workflow_yaml: workflow_yaml.to_string(),
    }
}

fn database() -> (TempDir, GlobalDatabase) {
    let home = tempfile::tempdir().expect("temp home");
    let expected_path = vw_config_types::paths::global_database_path(home.path());
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    assert_eq!(database.path(), expected_path);
    (home, database)
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_workflows_are_isolated_by_tenant() {
    let (_home, database) = database();
    assert_workflow_workflows_are_isolated_by_tenant(database).await;

    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    assert_workflow_workflows_are_isolated_by_tenant(fixture.database().clone()).await;
    fixture.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_workflow_workflows_are_isolated_by_tenant(database: GlobalDatabase) {
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-a", "company-b").expect("tenant B");
    let shared_uuid = Uuid::new_v4().to_string();
    let second_uuid = Uuid::new_v4().to_string();
    create_legacy_workflow_table(&database, &shared_uuid);

    assert!(
        list_records(database.clone(), tenant_a.clone())
            .await
            .expect("tenant A empty list")
            .is_empty()
    );
    assert!(
        list_records(database.clone(), tenant_b.clone())
            .await
            .expect("tenant B empty list")
            .is_empty()
    );

    assert_current_workflow_table(&database);

    let mut tenant_a_body = body("  Tenant A  ", "tenant: a");
    tenant_a_body.uuid = Some(shared_uuid.clone());
    let tenant_a_record = create_record(database.clone(), tenant_a.clone(), tenant_a_body)
        .await
        .expect("tenant A create");
    assert_eq!(tenant_a_record.uuid, shared_uuid);
    assert_eq!(tenant_a_record.name, "Tenant A");
    assert_eq!(tenant_a_record.description, "description");

    let mut tenant_b_body = body("Tenant B", "tenant: b");
    tenant_b_body.uuid = Some(shared_uuid.clone());
    create_record(database.clone(), tenant_b.clone(), tenant_b_body)
        .await
        .expect("tenant B create with shared uuid");

    let mut second_body = body("Second A", "tenant: second-a");
    second_body.uuid = Some(second_uuid.clone());
    let second = create_record(database.clone(), tenant_a.clone(), second_body)
        .await
        .expect("second tenant A create");
    assert_eq!(second.uuid, second_uuid);

    let fetched_a = get_record(database.clone(), tenant_a.clone(), shared_uuid.clone())
        .await
        .expect("get tenant A");
    let fetched_b = get_record(database.clone(), tenant_b.clone(), shared_uuid.clone())
        .await
        .expect("get tenant B");
    assert_eq!(fetched_a.workflow_yaml, "tenant: a");
    assert_eq!(fetched_b.workflow_yaml, "tenant: b");

    let updated = update_record(
        database.clone(),
        tenant_a.clone(),
        shared_uuid.clone(),
        body("Tenant A Updated", "tenant: a-updated"),
    )
    .await
    .expect("update tenant A");
    assert_eq!(updated.created_at_ms, tenant_a_record.created_at_ms);
    assert!(updated.updated_at_ms >= tenant_a_record.updated_at_ms);
    assert_eq!(updated.workflow_yaml, "tenant: a-updated");
    assert_eq!(
        get_record(database.clone(), tenant_b.clone(), shared_uuid.clone())
            .await
            .expect("tenant B remains")
            .workflow_yaml,
        "tenant: b"
    );

    let tenant_a_summaries =
        list_records(database.clone(), tenant_a.clone()).await.expect("list tenant A records");
    let tenant_b_summaries =
        list_records(database.clone(), tenant_b.clone()).await.expect("list tenant B records");
    assert_eq!(tenant_a_summaries.len(), 2);
    assert_eq!(tenant_b_summaries.len(), 1);
    assert_eq!(tenant_b_summaries[0].uuid, shared_uuid);

    let deleted = delete_record(database.clone(), tenant_a.clone(), shared_uuid.clone())
        .await
        .expect("delete tenant A");
    assert_eq!(deleted.uuid, shared_uuid);
    assert!(deleted.deleted);
    assert_eq!(
        get_record(database.clone(), tenant_b, shared_uuid.clone())
            .await
            .expect("tenant B survives tenant A delete")
            .workflow_yaml,
        "tenant: b"
    );

    let missing_delete = delete_record(database.clone(), tenant_a.clone(), shared_uuid.clone())
        .await
        .expect("delete missing is ok");
    assert!(!missing_delete.deleted);

    let missing = get_record(database.clone(), tenant_a, shared_uuid.clone())
        .await
        .expect_err("tenant A record is missing");
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    assert_legacy_workflow_row(&database, &shared_uuid);
}

#[test]
fn workflow_store_rejects_unscoped_new_table_without_mutation() {
    let (_home, database) = database();
    let connection = database.connect().expect("legacy database");
    connection
        .execute_batch(
            "CREATE TABLE workflow_workflows (
                 uuid TEXT PRIMARY KEY NOT NULL,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL,
                 workflow_yaml TEXT NOT NULL,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL
             );
             INSERT INTO workflow_workflows
                 (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
             VALUES ('legacy', 'Legacy', 'preserve me', 'legacy: true', 1, 1);",
        )
        .expect("legacy schema");
    drop(connection);

    let error = list_records_blocking(database.clone(), &TenantScope::default())
        .expect_err("unscoped workflow table must be rejected");
    assert_eq!(
        error.to_string(),
        "workflow database error: unsupported unscoped workflow schema: table workflow_workflows must contain platform_id and company_id"
    );

    let connection = database.connect().expect("legacy connection");
    let row: (String, String) = connection
        .query_row(
            "SELECT name, description FROM workflow_workflows WHERE uuid = 'legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("legacy row remains");
    assert_eq!(row, ("Legacy".to_string(), "preserve me".to_string()));
    assert!(!column_exists(&connection, "workflow_workflows", "platform_id").unwrap());
    assert!(!column_exists(&connection, "workflow_workflows", "company_id").unwrap());
}

#[test]
fn workflow_store_rejects_partially_scoped_table_missing_company_id() {
    let (_home, database) = database();
    let connection = database.connect().expect("partial database");
    connection
        .execute_batch(
            "CREATE TABLE workflow_workflows (
                 platform_id TEXT NOT NULL,
                 uuid TEXT PRIMARY KEY NOT NULL
             );",
        )
        .expect("partially scoped schema");
    drop(connection);

    let error = list_records_blocking(database.clone(), &TenantScope::default())
        .expect_err("workflow table missing company_id must be rejected");

    assert_eq!(
        error.to_string(),
        "workflow database error: unsupported unscoped workflow schema: table workflow_workflows must contain platform_id and company_id"
    );
    let connection = database.connect().expect("partial database remains readable");
    assert!(column_exists(&connection, "workflow_workflows", "platform_id").unwrap());
    assert!(!column_exists(&connection, "workflow_workflows", "company_id").unwrap());
}

#[test]
fn workflow_postgres_store_rejects_unscoped_new_table_without_mutation() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    let database = fixture.database().clone();
    let mut connection = database.connect_postgres().expect("PostgreSQL connection");
    connection
        .batch_execute(
            "CREATE TABLE workflow_workflows (
                 uuid TEXT PRIMARY KEY NOT NULL,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL,
                 workflow_yaml TEXT NOT NULL,
                 created_at_ms BIGINT NOT NULL,
                 updated_at_ms BIGINT NOT NULL
             );
             INSERT INTO workflow_workflows
                 (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
             VALUES ('legacy', 'Legacy', 'preserve me', 'legacy: true', 1, 1);",
        )
        .expect("legacy PostgreSQL schema");
    drop(connection);

    let error = list_records_blocking(database.clone(), &TenantScope::default())
        .expect_err("unscoped PostgreSQL workflow table must be rejected");
    assert_eq!(
        error.to_string(),
        "workflow database error: unsupported unscoped workflow schema: table workflow_workflows must contain platform_id and company_id"
    );

    let row = database
        .connect_postgres()
        .expect("PostgreSQL legacy connection")
        .query_one("SELECT name, description FROM workflow_workflows WHERE uuid = 'legacy'", &[])
        .expect("legacy PostgreSQL row remains");
    assert_eq!(row.get::<_, String>(0), "Legacy");
    assert_eq!(row.get::<_, String>(1), "preserve me");
    fixture.cleanup().expect("clean PostgreSQL test database");
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_store_create_generates_uuid_and_validates_inputs() {
    let (_home, database) = database();
    let tenant = TenantScope::default();

    let generated =
        create_record(database.clone(), tenant.clone(), body("Generated", "workflow: true"))
            .await
            .expect("generated create");
    Uuid::parse_str(&generated.uuid).expect("generated uuid should be valid");

    let invalid_uuid = get_record(database.clone(), tenant.clone(), "not-a-uuid".to_string())
        .await
        .expect_err("invalid uuid");
    assert_eq!(invalid_uuid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid_uuid.to_string(), "workflow uuid is invalid");

    let blank_name = create_record(database.clone(), tenant.clone(), body("   ", "workflow"))
        .await
        .expect_err("blank name");
    assert_eq!(blank_name.status, StatusCode::BAD_REQUEST);
    assert_eq!(blank_name.to_string(), "workflow name is required");

    let blank_yaml =
        create_record(database, tenant, body("Name", "  ")).await.expect_err("blank yaml");
    assert_eq!(blank_yaml.status, StatusCode::BAD_REQUEST);
    assert_eq!(blank_yaml.to_string(), "workflow_yaml is required");
}

#[test]
fn workflow_store_blocking_helpers_cover_sql_and_integer_edges() {
    let (_home, database) = database();
    let tenant = TenantScope::default();
    let uuid = Uuid::new_v4().to_string();

    let record =
        upsert_record_blocking(database.clone(), &tenant, &uuid, &body("Negative", "yaml"))
            .expect("upsert should create");
    assert_eq!(record.created_at_ms, record.updated_at_ms);

    let conn = open_db(&database).expect("db should open");
    conn.execute(
        "UPDATE workflow_workflows SET created_at_ms = -1, updated_at_ms = -5 \
         WHERE platform_id = ?1 AND company_id = ?2 AND uuid = ?3",
        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), uuid],
    )
    .expect("manual update should succeed");
    drop(conn);

    let record = get_record_blocking(database.clone(), &tenant, &uuid).expect("record should load");
    assert_eq!(record.created_at_ms, 0);
    assert_eq!(record.updated_at_ms, 0);

    let missing_uuid = Uuid::new_v4().to_string();
    let missing = get_record_blocking(database.clone(), &tenant, &missing_uuid)
        .expect_err("missing valid uuid should fail");
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    let invalid_update =
        update_record_blocking_for_test(database, &tenant, "bad", body("Name", "yaml"));
    assert_eq!(invalid_update.expect_err("invalid uuid").status, StatusCode::BAD_REQUEST);

    let invalid_home = tempfile::tempdir().expect("invalid temp home");
    let invalid_path = invalid_home.path().join("database").join("_index.sqlite3");
    let invalid_database = GlobalDatabase::new(invalid_path.clone()).expect("initial database");
    std::fs::remove_file(&invalid_path).expect("remove database file");
    std::fs::create_dir(&invalid_path).expect("replace database with directory");
    let sql = list_records_blocking(invalid_database, &tenant)
        .expect_err("opening directory as database should fail");
    assert_eq!(sql.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(sql.to_string().contains("workflow database error"));
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_store_maps_blocking_task_panics_to_internal_error() {
    let (_home, database) = database();
    let error = spawn_store_task(database, |_database| -> Result<(), ApiError> {
        panic!("store task panic");
    })
    .await
    .expect_err("panic should become internal error");

    assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.to_string().contains("workflow store task failed"));
}

fn update_record_blocking_for_test(
    database: GlobalDatabase,
    tenant: &TenantScope,
    uuid: &str,
    body: WorkflowRecordUpsertBody,
) -> Result<WorkflowRecord, ApiError> {
    upsert_record_blocking(database, tenant, uuid, &body)
}

fn create_legacy_workflow_table(database: &GlobalDatabase, uuid: &str) {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let connection = database.connect_sqlite().expect("legacy SQLite database");
            connection
                .execute_batch(
                    "CREATE TABLE dify_workflows (
                         uuid TEXT PRIMARY KEY NOT NULL,
                         name TEXT NOT NULL,
                         description TEXT NOT NULL,
                         workflow_yaml TEXT NOT NULL,
                         created_at_ms INTEGER NOT NULL,
                         updated_at_ms INTEGER NOT NULL
                     );",
                )
                .expect("legacy SQLite schema");
            connection
                .execute(
                    "INSERT INTO dify_workflows \
                     (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms) \
                     VALUES (?1, 'Legacy', '', 'legacy: true', 1, 1)",
                    params![uuid],
                )
                .expect("legacy SQLite workflow");
        }
        DatabaseProvider::Postgres => {
            let mut connection = database.connect_postgres().expect("legacy PostgreSQL database");
            connection
                .batch_execute(
                    "CREATE TABLE dify_workflows (
                         uuid TEXT PRIMARY KEY NOT NULL,
                         name TEXT NOT NULL,
                         description TEXT NOT NULL,
                         workflow_yaml TEXT NOT NULL,
                         created_at_ms BIGINT NOT NULL,
                         updated_at_ms BIGINT NOT NULL
                     );",
                )
                .expect("legacy PostgreSQL schema");
            connection
                .execute(
                    "INSERT INTO dify_workflows
                     (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
                     VALUES ($1, 'Legacy', '', 'legacy: true', 1, 1)",
                    &[&uuid],
                )
                .expect("legacy PostgreSQL workflow");
        }
    }
}

fn assert_current_workflow_table(database: &GlobalDatabase) {
    let table_name: String = match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("shared SQLite connection")
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'workflow_workflows'",
                [],
                |row| row.get(0),
            )
            .expect("SQLite workflow_workflows table"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("shared PostgreSQL connection")
            .query_one(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = current_schema() AND table_name = 'workflow_workflows'",
                &[],
            )
            .expect("PostgreSQL workflow_workflows table")
            .get(0),
    };
    assert_eq!(table_name, "workflow_workflows");
}

fn assert_legacy_workflow_row(database: &GlobalDatabase, uuid: &str) {
    let legacy: (String, String) = match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("legacy SQLite connection")
            .query_row(
                "SELECT name, workflow_yaml FROM dify_workflows WHERE uuid = ?1",
                params![uuid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("legacy SQLite row remains"),
        DatabaseProvider::Postgres => {
            let row = database
                .connect_postgres()
                .expect("legacy PostgreSQL connection")
                .query_one(
                    "SELECT name, workflow_yaml FROM dify_workflows WHERE uuid = $1",
                    &[&uuid],
                )
                .expect("legacy PostgreSQL row remains");
            (row.get(0), row.get(1))
        }
    };
    assert_eq!(legacy, ("Legacy".to_string(), "legacy: true".to_string()));
}
