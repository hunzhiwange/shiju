// 覆盖配置存储后端选择、租户读写及兼容行为。

use anyhow::Result;
use vw_api_types::id::TenantScope;

use super::SystemConfigStore;
use crate::app::agent::security::gateway_skey_store::{GatewaySkeyReplacement, GatewaySkeyStore};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;

// 正常值、缺省值与非法输入成组验证，用于固定配置兼容边界和失败语义。
#[test]
fn system_config_store_isolates_platform_and_company_rows() -> Result<()> {
    for_each_system_config_store_provider(|store, _| system_config_store_isolation(store))?;
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
    for_each_system_config_store_provider(|store, _| project_config_store_isolation(store))?;
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

#[test]
fn system_config_store_conditional_write_preserves_winner_and_rolls_back_skeys() -> Result<()> {
    for_each_system_config_store_provider(|store, database| {
        let tenant = TenantScope::new("platform", "company")?;
        let other = TenantScope::new("platform", "other")?;
        let skey_store = GatewaySkeyStore::new(database.clone())?;
        let skey = serde_json::from_value::<vw_config_types::gateway::GatewaySkey>(
            serde_json::json!({"skey_hash": "a".repeat(64)}),
        )?;
        let initial = GatewaySkeyReplacement::new(std::slice::from_ref(&skey))?;
        let original = r#"{"version":1}"#;
        assert!(store.compare_and_swap(&tenant, None, original, Some(&initial))?);
        let original_skeys = skey_store.load(&tenant)?;
        assert_eq!(original_skeys.len(), 1);
        assert!(!store.compare_and_swap(&tenant, None, "{}", None)?);

        let barrier = std::sync::Barrier::new(2);
        let editor = r#"{"version":2}"#;
        let terminal = r#"{"version":3}"#;
        let (editor_won, terminal_won) = std::thread::scope(|scope| {
            let editor_write = scope.spawn(|| {
                barrier.wait();
                store.compare_and_swap(&tenant, Some(original), editor, None)
            });
            let terminal_write = scope.spawn(|| {
                barrier.wait();
                store.compare_and_swap(&tenant, Some(original), terminal, None)
            });
            Ok::<_, anyhow::Error>((editor_write.join().unwrap()?, terminal_write.join().unwrap()?))
        })?;
        assert_ne!(editor_won, terminal_won);
        let winner = if editor_won { editor } else { terminal };
        assert_eq!(store.get(&tenant)?.as_deref(), Some(winner));

        let duplicate = GatewaySkeyReplacement::new(&[skey.clone(), skey])?;
        assert!(store.compare_and_swap(&tenant, Some(winner), "{}", Some(&duplicate)).is_err());
        assert_eq!(store.get(&tenant)?.as_deref(), Some(winner));
        assert_eq!(skey_store.load(&tenant)?, original_skeys);
        assert_eq!(store.get(&other)?, None);
        assert!(skey_store.load(&other)?.is_empty());
        Ok(())
    })
}

fn for_each_system_config_store_provider(
    mut check: impl FnMut(&SystemConfigStore, &GlobalDatabase) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_db = GlobalDatabase::for_home(sqlite_home.path())?;
    let sqlite_store = SystemConfigStore::new(sqlite_db.clone())?;
    check(&sqlite_store, &sqlite_db)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_store = SystemConfigStore::new(postgres_fixture.database().clone())?;
    check(&postgres_store, postgres_fixture.database())?;
    postgres_fixture.cleanup()?;
    Ok(())
}
