use anyhow::Result;
use rusqlite::Connection;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::id::{
    DEFAULT_COMPANY_ID, DEFAULT_PLATFORM_ID, ProjectId, ProjectScope, TenantScope,
};
use vw_shared::project::{Commands, Icon, Info, TimeInfo, Vcs};

use super::ProjectStore;
use crate::agent_scheduler::domain::AgentSchedulerContext;
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;

#[derive(Debug, PartialEq, Eq)]
struct SeedCatalog {
    platforms: Vec<(String, String)>,
    companies: Vec<(String, String, String)>,
}

#[test]
fn project_store_initializes_default_platform_and_company() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("global").join("_index.sqlite3");
    let sqlite_database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(sqlite_database.clone())?;

    let connection = Connection::open(&database_path)?;
    assert_sqlite_object_exists(&connection, "table", "platform")?;
    assert_sqlite_object_exists(&connection, "table", "company")?;
    assert_sqlite_object_exists(&connection, "table", "projects")?;
    assert_sqlite_object_exists(&connection, "index", "idx_projects_tenant_worktree")?;
    assert_eq!(
        project_columns(&connection)?,
        vec![
            "platform_id",
            "company_id",
            "project_id",
            "worktree",
            "vcs",
            "name",
            "icon_json",
            "commands_json",
            "created_at_ms",
            "updated_at_ms",
            "initialized_at_ms",
            "sandboxes_json",
        ]
    );

    let expected = expected_seed_catalog();
    let sqlite_seed_catalog = load_seed_catalog(&sqlite_database)?;
    assert_eq!(sqlite_seed_catalog, expected);
    rename_default_seed_entries(&sqlite_database)?;
    ProjectStore::new(sqlite_database.clone())?;
    assert_default_seed_names_are_preserved(&sqlite_database)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_database = postgres_fixture.database().clone();
    ProjectStore::new(postgres_database.clone())?;
    let postgres_seed_catalog = load_seed_catalog(&postgres_database)?;
    assert_eq!(postgres_seed_catalog, expected);
    assert_eq!(postgres_seed_catalog, sqlite_seed_catalog);
    rename_default_seed_entries(&postgres_database)?;
    ProjectStore::new(postgres_database.clone())?;
    assert_default_seed_names_are_preserved(&postgres_database)?;
    postgres_fixture.cleanup()?;

    Ok(())
}

#[test]
fn project_store_round_trips_info_with_full_tenant_isolation() -> Result<()> {
    for_each_project_store_provider(project_store_round_trip)?;
    Ok(())
}

fn project_store_round_trip(store: &ProjectStore) -> Result<()> {
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-b", "company-b")?;
    let scope_a = project_scope(&tenant_a, "same-project")?;
    let scope_b = project_scope(&tenant_b, "same-project")?;
    let project_a = full_info("same-project", "/workspace/a", "Tenant A");
    let mut project_b = full_info("same-project", "/workspace/b", "Tenant B");

    store.create(&tenant_a, &project_a)?;
    store.create(&tenant_b, &project_b)?;

    assert_eq!(store.get(&scope_a)?, Some(project_a.clone()));
    assert_eq!(store.get(&scope_b)?, Some(project_b.clone()));
    assert_eq!(store.list(&tenant_a)?, vec![project_a.clone()]);
    assert_eq!(store.list(&tenant_b)?, vec![project_b.clone()]);

    project_b.name = Some("Tenant B updated".to_owned());
    project_b.time.updated = 30;
    assert!(store.update(&scope_b, &project_b)?);
    assert_eq!(store.get(&scope_a)?, Some(project_a.clone()));
    assert_eq!(store.get(&scope_b)?, Some(project_b.clone()));

    assert!(store.add_sandbox(&scope_b, "/workspace/b-sandbox-2")?);
    assert!(store.remove_sandbox(&scope_b, "/workspace/b-sandbox")?);
    assert_eq!(
        store.get(&scope_b)?.expect("tenant B project should exist").sandboxes,
        vec!["/workspace/b-sandbox-2".to_owned()]
    );
    assert_eq!(
        store.get(&scope_a)?.expect("tenant A project should exist").sandboxes,
        vec!["/workspace/a-sandbox".to_owned()]
    );

    assert!(store.delete(&scope_b)?);
    assert_eq!(store.get(&scope_b)?, None);
    assert_eq!(store.get(&scope_a)?, Some(project_a));

    Ok(())
}

#[test]
fn project_store_import_and_mutate_roll_back_on_conflict() -> Result<()> {
    for_each_project_store_provider(project_store_transaction_rollbacks)?;
    Ok(())
}

fn project_store_transaction_rollbacks(store: &ProjectStore) -> Result<()> {
    let tenant = TenantScope::new("rollback-platform", "rollback-company")?;
    let existing = full_info("existing", "/workspace/existing", "Existing");
    store.create(&tenant, &existing)?;

    let inserted_before_conflict = full_info("inserted", "/workspace/inserted", "Inserted");
    let conflicting = full_info("existing", "/workspace/conflict", "Conflict");
    let error = store
        .import_exact_registry(&tenant, &[inserted_before_conflict.clone(), conflicting])
        .expect_err("registry conflict must roll back the transaction");
    assert!(error.to_string().contains("project registry conflict"));
    assert_eq!(store.get(&project_scope(&tenant, "inserted")?)?, None);
    assert_eq!(store.get(&project_scope(&tenant, "existing")?)?, Some(existing.clone()));

    let existing_scope = project_scope(&tenant, "existing")?;
    let error = store
        .mutate(&existing_scope, |project| project.worktree = " ".to_owned())
        .expect_err("invalid mutation must roll back the transaction");
    assert!(error.to_string().contains("project worktree must not be empty"));
    assert_eq!(store.get(&existing_scope)?, Some(existing));
    Ok(())
}

#[test]
fn project_store_rejects_blank_scope_and_preserves_same_id_across_tenants() -> Result<()> {
    assert!(TenantScope::new(" ", "company").is_err());
    assert!(TenantScope::new("platform", "\t").is_err());
    assert!(ProjectScope::new(TenantScope::default(), ProjectId::from(" ")).is_err());

    let temp_dir = tempfile::tempdir()?;
    let store = ProjectStore::new(GlobalDatabase::new(temp_dir.path().join("db.sqlite3"))?)?;
    let tenant_a = TenantScope::new("platform", "company-a")?;
    let tenant_b = TenantScope::new("platform", "company-b")?;

    let mut blank_id = full_info(" ", "/workspace/a", "Blank ID");
    assert!(store.create(&tenant_a, &blank_id).is_err());
    blank_id.id = "same-project".to_owned();
    blank_id.worktree = " \t".to_owned();
    assert!(store.create(&tenant_a, &blank_id).is_err());

    let project_a = full_info("same-project", "/workspace/a", "Tenant A");
    let project_b = full_info("same-project", "/workspace/b", "Tenant B");
    store.create(&tenant_a, &project_a)?;
    store.create(&tenant_b, &project_b)?;

    let missing_scope = project_scope(&tenant_a, "missing")?;
    assert_eq!(store.get(&missing_scope)?, None);
    assert!(!store.update(&missing_scope, &full_info("missing", "/missing", "Missing"))?);
    assert!(!store.delete(&missing_scope)?);
    assert!(!store.add_sandbox(&missing_scope, "/missing-sandbox")?);
    assert!(!store.remove_sandbox(&missing_scope, "/missing-sandbox")?);

    let tenant_a_scope = project_scope(&tenant_a, "same-project")?;
    let mismatched_info = full_info("other-project", "/workspace/other", "Other");
    assert!(store.update(&tenant_a_scope, &mismatched_info).is_err());
    assert_eq!(store.get(&tenant_a_scope)?, Some(project_a));
    assert_eq!(store.get(&project_scope(&tenant_b, "same-project")?)?, Some(project_b));

    Ok(())
}

#[test]
fn project_store_restricts_delete_while_scheduler_rows_exist() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("global").join("_index.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    let store = ProjectStore::new(database.clone())?;
    let tenant = TenantScope::default();
    let scope = project_scope(&tenant, "scheduled-project")?;
    let worktree = temp_dir.path().join("scheduled-project");
    std::fs::create_dir_all(&worktree)?;
    store.create(
        &tenant,
        &full_info("scheduled-project", &worktree.to_string_lossy(), "Scheduled"),
    )?;
    let scheduler = AgentSchedulerStore::from_database_path(&database_path);
    let context = AgentSchedulerContext::new(AgentSchedulerScope::new(scope.clone()), worktree)?;
    scheduler.initialize_project(&context)?;
    database.connect()?.execute(
        "INSERT INTO scheduler_tasks (
             platform_id, company_id, project_id, id, kind, status, title,
             created_at_ms, updated_at_ms
         ) VALUES (
             ?1, ?2, 'scheduled-project', 'fixture', 'finite', 'draft',
             'kept', 1, 1
         )",
        rusqlite::params![DEFAULT_PLATFORM_ID, DEFAULT_COMPANY_ID],
    )?;

    let error = store.delete(&scope).expect_err("scheduler foreign key must restrict delete");
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
    assert!(store.get(&scope)?.is_some());
    Ok(())
}

#[test]
fn project_store_rejects_duplicate_registry_ids_before_writing() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = ProjectStore::new(GlobalDatabase::new(temp_dir.path().join("db.sqlite3"))?)?;
    let tenant = TenantScope::new("duplicate-platform", "duplicate-company")?;
    let first = full_info("duplicate", "/workspace/first", "First");
    let second = full_info("duplicate", "/workspace/second", "Second");

    let error = store
        .import_exact_registry(&tenant, &[first, second])
        .expect_err("duplicate source ids must be rejected before import");

    assert!(error.to_string().contains("duplicate project id in registry import"));
    assert!(store.list(&tenant)?.is_empty());
    Ok(())
}

#[test]
fn project_store_rejects_inexact_empty_registry_import() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = ProjectStore::new(GlobalDatabase::new(temp_dir.path().join("db.sqlite3"))?)?;
    let tenant = TenantScope::new("exact-platform", "exact-company")?;
    let existing = full_info("existing", "/workspace/existing", "Existing");
    store.create(&tenant, &existing)?;

    let error = store
        .import_exact_registry(&tenant, &[])
        .expect_err("an exact empty import must reject an existing target row");

    assert!(error.to_string().contains("count verification failed"));
    assert_eq!(store.list(&tenant)?, vec![existing]);
    Ok(())
}

#[test]
fn project_store_conversion_helpers_cover_invalid_boundaries() {
    assert_eq!(super::serialize_vcs(None), None);
    assert_eq!(super::serialize_vcs(Some(&Vcs::Git)), Some("git"));
    assert_eq!(super::parse_vcs("git").expect("git should be supported"), Vcs::Git);
    assert!(
        super::parse_vcs("svn").expect_err("unsupported vcs must fail").to_string().contains("svn")
    );

    assert_eq!(super::to_sql_timestamp(i64::MAX as u64).expect("i64 max should fit"), i64::MAX);
    assert!(super::to_sql_timestamp(i64::MAX as u64 + 1).is_err());
    assert_eq!(super::from_sql_timestamp(0).expect("zero should convert"), 0);
    assert!(super::from_sql_timestamp(-1).is_err());
}

fn project_scope(tenant: &TenantScope, project_id: &str) -> Result<ProjectScope> {
    Ok(ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?)
}

fn full_info(id: &str, worktree: &str, name: &str) -> Info {
    Info {
        id: id.to_owned(),
        worktree: worktree.to_owned(),
        vcs: Some(Vcs::Git),
        name: Some(name.to_owned()),
        icon: Some(Icon {
            url: Some(format!("https://example.test/{id}.png")),
            override_icon: Some("rocket".to_owned()),
            color: Some("#123456".to_owned()),
        }),
        commands: Some(Commands { start: Some("cargo run".to_owned()) }),
        time: TimeInfo { created: 10, updated: 20, initialized: Some(15) },
        sandboxes: vec![format!("{worktree}-sandbox")],
    }
}

fn assert_sqlite_object_exists(
    connection: &Connection,
    object_type: &str,
    name: &str,
) -> Result<()> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        (object_type, name),
        |row| row.get(0),
    )?;
    assert_eq!(count, 1, "missing sqlite {object_type}: {name}");
    Ok(())
}

fn project_columns(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection.prepare("PRAGMA table_info(projects)")?;
    let columns =
        statement.query_map([], |row| row.get(1))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(columns)
}

fn expected_seed_catalog() -> SeedCatalog {
    SeedCatalog {
        platforms: vec![
            ("10000".to_owned(), "默认平台".to_owned()),
            ("10001".to_owned(), "测试平台1".to_owned()),
            ("10002".to_owned(), "测试平台2".to_owned()),
        ],
        companies: vec![
            ("10000".to_owned(), "100000000".to_owned(), "默认公司".to_owned()),
            ("10000".to_owned(), "100000001".to_owned(), "测试公司1".to_owned()),
            ("10000".to_owned(), "100000002".to_owned(), "测试公司2".to_owned()),
            ("10000".to_owned(), "100000003".to_owned(), "测试公司3".to_owned()),
            ("10000".to_owned(), "100000004".to_owned(), "测试公司4".to_owned()),
            ("10000".to_owned(), "100000005".to_owned(), "测试公司5".to_owned()),
            ("10001".to_owned(), "100010000".to_owned(), "默认公司".to_owned()),
            ("10001".to_owned(), "100010001".to_owned(), "测试公司1".to_owned()),
            ("10001".to_owned(), "100010002".to_owned(), "测试公司2".to_owned()),
            ("10001".to_owned(), "100010003".to_owned(), "测试公司3".to_owned()),
            ("10001".to_owned(), "100010004".to_owned(), "测试公司4".to_owned()),
            ("10001".to_owned(), "100010005".to_owned(), "测试公司5".to_owned()),
            ("10002".to_owned(), "100020000".to_owned(), "默认公司".to_owned()),
            ("10002".to_owned(), "100020001".to_owned(), "测试公司1".to_owned()),
            ("10002".to_owned(), "100020002".to_owned(), "测试公司2".to_owned()),
            ("10002".to_owned(), "100020003".to_owned(), "测试公司3".to_owned()),
            ("10002".to_owned(), "100020004".to_owned(), "测试公司4".to_owned()),
            ("10002".to_owned(), "100020005".to_owned(), "测试公司5".to_owned()),
        ],
    }
}

fn load_seed_catalog(database: &GlobalDatabase) -> Result<SeedCatalog> {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let connection = database.connect_sqlite()?;
            let platforms = connection
                .prepare("SELECT platform_id, name FROM platform ORDER BY platform_id")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let companies = connection
                .prepare(
                    "SELECT platform_id, company_id, name
                     FROM company
                     ORDER BY platform_id, company_id",
                )?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(SeedCatalog { platforms, companies })
        }
        DatabaseProvider::Postgres => {
            let mut connection = database.connect_postgres()?;
            let platforms = connection
                .query("SELECT platform_id, name FROM platform ORDER BY platform_id", &[])?
                .into_iter()
                .map(|row| (row.get(0), row.get(1)))
                .collect();
            let companies = connection
                .query(
                    "SELECT platform_id, company_id, name
                     FROM company
                     ORDER BY platform_id, company_id",
                    &[],
                )?
                .into_iter()
                .map(|row| (row.get(0), row.get(1), row.get(2)))
                .collect();
            Ok(SeedCatalog { platforms, companies })
        }
    }
}

fn rename_default_seed_entries(database: &GlobalDatabase) -> Result<()> {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let connection = database.connect_sqlite()?;
            connection.execute(
                "UPDATE platform SET name = '自定义平台' WHERE platform_id = ?1",
                [DEFAULT_PLATFORM_ID],
            )?;
            connection.execute(
                "UPDATE company SET name = '自定义公司'
                 WHERE platform_id = ?1 AND company_id = ?2",
                [DEFAULT_PLATFORM_ID, DEFAULT_COMPANY_ID],
            )?;
        }
        DatabaseProvider::Postgres => {
            let mut connection = database.connect_postgres()?;
            connection.execute(
                "UPDATE platform SET name = '自定义平台' WHERE platform_id = $1",
                &[&DEFAULT_PLATFORM_ID],
            )?;
            connection.execute(
                "UPDATE company SET name = '自定义公司'
                 WHERE platform_id = $1 AND company_id = $2",
                &[&DEFAULT_PLATFORM_ID, &DEFAULT_COMPANY_ID],
            )?;
        }
    }
    Ok(())
}

fn assert_default_seed_names_are_preserved(database: &GlobalDatabase) -> Result<()> {
    let catalog = load_seed_catalog(database)?;
    assert_eq!(catalog.platforms.len(), 3);
    assert_eq!(catalog.companies.len(), 18);
    assert_eq!(catalog.platforms[0], ("10000".to_owned(), "自定义平台".to_owned()));
    assert_eq!(
        catalog.companies[0],
        ("10000".to_owned(), "100000000".to_owned(), "自定义公司".to_owned())
    );
    Ok(())
}

fn for_each_project_store_provider(
    mut check: impl FnMut(&ProjectStore) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database_path = sqlite_home.path().join("global").join("_index.sqlite3");
    let sqlite_store = ProjectStore::new(GlobalDatabase::new(&sqlite_database_path)?)?;
    check(&sqlite_store)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_store = ProjectStore::new(postgres_fixture.database().clone())?;
    check(&postgres_store)?;
    postgres_fixture.cleanup()?;
    Ok(())
}
