use anyhow::Result;
use vw_api_types::id::TenantScope;

use super::SystemConfigStore;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;

#[test]
fn system_config_store_isolates_platform_and_company_rows() -> Result<()> {
    for_each_system_config_store_provider(system_config_store_isolation)?;
    Ok(())
}

fn system_config_store_isolation(store: &SystemConfigStore) -> Result<()> {
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let same_platform = TenantScope::new("platform-a", "company-b")?;
    let same_company = TenantScope::new("platform-b", "company-a")?;

    assert_eq!(store.get(&tenant_a)?, None);
    assert_eq!(store.get(&same_platform)?, None);
    assert_eq!(store.get(&same_company)?, None);

    store.upsert(&tenant_a, r#"{"shared_key":"tenant-a"}"#)?;
    store.upsert(&same_platform, r#"{"shared_key":"company-b"}"#)?;
    store.upsert(&same_company, r#"{"shared_key":"platform-b"}"#)?;

    assert_eq!(store.get(&tenant_a)?.as_deref(), Some(r#"{"shared_key":"tenant-a"}"#));
    assert_eq!(store.get(&same_platform)?.as_deref(), Some(r#"{"shared_key":"company-b"}"#));
    assert_eq!(store.get(&same_company)?.as_deref(), Some(r#"{"shared_key":"platform-b"}"#));

    store.upsert(&tenant_a, r#"{"shared_key":"tenant-a-updated"}"#)?;

    assert_eq!(store.get(&tenant_a)?.as_deref(), Some(r#"{"shared_key":"tenant-a-updated"}"#));
    assert_eq!(store.get(&same_platform)?.as_deref(), Some(r#"{"shared_key":"company-b"}"#));
    assert_eq!(store.get(&same_company)?.as_deref(), Some(r#"{"shared_key":"platform-b"}"#));
    Ok(())
}

#[test]
fn project_config_store_isolates_tenants_and_project_paths() -> Result<()> {
    for_each_system_config_store_provider(project_config_store_isolation)?;
    Ok(())
}

fn project_config_store_isolation(store: &SystemConfigStore) -> Result<()> {
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-a", "company-b")?;
    let project_a = "/workspace/project-a";
    let project_b = "/workspace/project-b";

    assert_eq!(store.get_project(&tenant_a, project_a)?, None);
    assert_eq!(store.get_project(&tenant_a, project_b)?, None);
    assert_eq!(store.get_project(&tenant_b, project_a)?, None);

    store.upsert_project(&tenant_a, project_a, r#"{"model":"tenant-a-project-a"}"#)?;
    store.upsert_project(&tenant_a, project_b, r#"{"model":"tenant-a-project-b"}"#)?;
    store.upsert_project(&tenant_b, project_a, r#"{"model":"tenant-b-project-a"}"#)?;

    assert_eq!(
        store.get_project(&tenant_a, project_a)?.as_deref(),
        Some(r#"{"model":"tenant-a-project-a"}"#)
    );
    assert_eq!(
        store.get_project(&tenant_a, project_b)?.as_deref(),
        Some(r#"{"model":"tenant-a-project-b"}"#)
    );
    assert_eq!(
        store.get_project(&tenant_b, project_a)?.as_deref(),
        Some(r#"{"model":"tenant-b-project-a"}"#)
    );
    Ok(())
}

#[test]
fn system_config_store_keeps_sqlite_path_contract() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(temp_dir.path())?;
    let expected_path = vw_config_types::paths::global_database_path(temp_dir.path());
    assert_eq!(database.path(), expected_path);
    assert_eq!(
        database.path().file_name().and_then(|name| name.to_str()),
        Some("vibe-window.sqlite3")
    );
    assert_eq!(
        database.path().parent().and_then(|path| path.file_name()).and_then(|name| name.to_str()),
        Some("database")
    );

    Ok(())
}

#[test]
fn system_config_store_reports_sqlite_schema_initialization_failure() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("invalid.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    std::fs::write(&database_path, b"not a SQLite database")?;

    let _ = SystemConfigStore::new(database).expect_err("invalid database must be rejected");
    assert!(database_path.is_file());
    Ok(())
}

fn for_each_system_config_store_provider(
    mut check: impl FnMut(&SystemConfigStore) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_store = SystemConfigStore::new(GlobalDatabase::for_home(sqlite_home.path())?)?;
    check(&sqlite_store)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_store = SystemConfigStore::new(postgres_fixture.database().clone())?;
    check(&postgres_store)?;
    postgres_fixture.cleanup()?;
    Ok(())
}
