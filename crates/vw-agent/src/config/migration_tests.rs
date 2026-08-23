use anyhow::Result;
use vw_api_types::id::TenantScope;
use vw_config_types::config::Config;

use super::SystemConfigStore;
use super::migration::{MigrationOutcome, migrate_user_config, migrate_user_config_with_database};
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;

fn write_source(home: &std::path::Path, config: &Config) -> Result<std::path::PathBuf> {
    let config_dir = vw_config_types::paths::home_config_dir(home);
    std::fs::create_dir_all(&config_dir)?;
    let source = config_dir.join("vibewindow.json");
    std::fs::write(&source, serde_json::to_vec_pretty(config)?)?;
    Ok(source)
}

fn backup_path(home: &std::path::Path) -> std::path::PathBuf {
    vw_config_types::paths::home_config_dir(home)
        .join("database/migration-backups/system-config/vibewindow.json")
}

#[test]
fn system_config_migration_reports_no_source_without_creating_storage() -> Result<()> {
    let temp_home = tempfile::tempdir()?;

    assert_eq!(migrate_user_config(temp_home.path())?, MigrationOutcome::NoSource);
    assert!(!vw_config_types::paths::global_database_path(temp_home.path()).exists());

    let database = GlobalDatabase::for_home(temp_home.path())?;
    assert_eq!(
        migrate_user_config_with_database(temp_home.path(), database)?,
        MigrationOutcome::NoSource
    );
    assert!(!backup_path(temp_home.path()).exists());
    Ok(())
}

#[test]
fn system_config_migration_refuses_to_overwrite_existing_backup() -> Result<()> {
    let temp_home = tempfile::tempdir()?;
    let mut source_config = Config::default();
    source_config.default_model = Some("backup-conflict-model".to_string());
    let source = write_source(temp_home.path(), &source_config)?;
    let backup = backup_path(temp_home.path());
    std::fs::create_dir_all(backup.parent().expect("backup parent"))?;
    std::fs::write(&backup, b"existing backup")?;

    let error = migrate_user_config(temp_home.path()).expect_err("backup conflict must fail");

    assert!(error.to_string().contains("backup conflict"));
    assert!(source.is_file());
    assert_eq!(std::fs::read(&backup)?, b"existing backup");
    Ok(())
}

#[test]
fn system_config_migration_imports_local_config_only_after_verification() -> Result<()> {
    for_each_migration_provider(system_config_migration_round_trip)?;
    Ok(())
}

fn system_config_migration_round_trip(
    home: &std::path::Path,
    database: GlobalDatabase,
) -> Result<()> {
    let mut source_config = Config::default();
    source_config.default_model = Some("migration-model".to_string());
    source_config.api_key = Some("migration-secret".to_string());
    let source = write_source(home, &source_config)?;

    assert_eq!(
        migrate_user_config_with_database(home, database.clone())?,
        MigrationOutcome::Migrated
    );

    assert!(!source.exists());
    assert!(backup_path(home).is_file());
    let store = SystemConfigStore::new(database)?;
    let stored = store.get(&TenantScope::default())?.expect("local config row");
    let stored_json: serde_json::Value = serde_json::from_str(&stored)?;
    assert_eq!(stored_json["default_model"], "migration-model");
    assert_ne!(stored_json["api_key"], "migration-secret");
    assert_eq!(store.get(&TenantScope::new("remote", "company")?)?, None);
    Ok(())
}

fn for_each_migration_provider(
    mut check: impl FnMut(&std::path::Path, GlobalDatabase) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    check(sqlite_home.path(), GlobalDatabase::for_home(sqlite_home.path())?)?;

    let postgres_home = tempfile::tempdir()?;
    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    check(postgres_home.path(), postgres_fixture.database().clone())?;
    postgres_fixture.cleanup()?;
    Ok(())
}

#[test]
fn system_config_migration_finishes_when_database_already_has_same_config() -> Result<()> {
    for_each_migration_provider(system_config_migration_already_imported)?;
    Ok(())
}

fn system_config_migration_already_imported(
    home: &std::path::Path,
    database: GlobalDatabase,
) -> Result<()> {
    let mut source_config = Config::default();
    source_config.default_model = Some("same-model".to_string());
    let source = write_source(home, &source_config)?;
    let store = SystemConfigStore::new(database.clone())?;
    store.upsert(&TenantScope::default(), &serde_json::to_string(&source_config)?)?;

    assert_eq!(
        migrate_user_config_with_database(home, database)?,
        MigrationOutcome::AlreadyImported
    );

    assert!(!source.exists());
    assert!(backup_path(home).is_file());
    Ok(())
}

#[test]
fn system_config_migration_preserves_conflicting_source() -> Result<()> {
    for_each_migration_provider(system_config_migration_conflict)?;
    Ok(())
}

fn system_config_migration_conflict(
    home: &std::path::Path,
    database: GlobalDatabase,
) -> Result<()> {
    let mut source_config = Config::default();
    source_config.default_model = Some("source-model".to_string());
    let source = write_source(home, &source_config)?;
    let store = SystemConfigStore::new(database.clone())?;
    let mut target_config = Config::default();
    target_config.default_model = Some("target-model".to_string());
    let target_payload = serde_json::to_string(&target_config)?;
    store.upsert(&TenantScope::default(), &target_payload)?;

    let error = migrate_user_config_with_database(home, database).expect_err("conflict must fail");

    assert!(error.to_string().contains("conflict"));
    assert!(source.is_file());
    assert!(!backup_path(home).exists());
    assert_eq!(store.get(&TenantScope::default())?.as_deref(), Some(target_payload.as_str()));
    Ok(())
}

#[test]
fn system_config_migration_preserves_invalid_source() -> Result<()> {
    let temp_home = tempfile::tempdir()?;
    let config_dir = vw_config_types::paths::home_config_dir(temp_home.path());
    std::fs::create_dir_all(&config_dir)?;
    let source = config_dir.join("vibewindow.json");
    let mut invalid_config = Config::default();
    invalid_config.gateway.host.clear();
    std::fs::write(&source, serde_json::to_vec_pretty(&invalid_config)?)?;

    let error = migrate_user_config(temp_home.path()).expect_err("invalid config must fail");

    assert!(format!("{error:#}").contains("gateway.host"));
    assert!(source.is_file());
    assert!(!backup_path(temp_home.path()).exists());
    let store = SystemConfigStore::new(GlobalDatabase::for_home(temp_home.path())?)?;
    assert_eq!(store.get(&TenantScope::default())?, None);
    Ok(())
}

#[test]
fn system_config_migration_accepts_legacy_toml_source() -> Result<()> {
    let temp_home = tempfile::tempdir()?;
    let config_dir = vw_config_types::paths::home_config_dir(temp_home.path());
    std::fs::create_dir_all(&config_dir)?;
    let source = config_dir.join("vibewindow.json");
    let mut source_config = Config::default();
    source_config.default_model = Some("legacy-toml-model".to_string());
    std::fs::write(&source, toml::to_string_pretty(&source_config)?)?;

    assert_eq!(migrate_user_config(temp_home.path())?, MigrationOutcome::Migrated);

    let store = SystemConfigStore::new(GlobalDatabase::for_home(temp_home.path())?)?;
    let stored = store.get(&TenantScope::default())?.expect("local config row");
    let stored_json: serde_json::Value = serde_json::from_str(&stored)?;
    assert_eq!(stored_json["default_model"], "legacy-toml-model");
    assert!(!source.exists());
    assert!(backup_path(temp_home.path()).is_file());
    Ok(())
}
// 本文件验证系统配置迁移的备份、校验、冲突保护与幂等性。
