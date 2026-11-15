use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use rusqlite::Connection;
use super::GlobalDatabase;
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::project::store::ProjectStore;
use crate::session::store::SessionStore;

#[test]
fn global_database_configures_connection_and_secure_permissions() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let home = temp_dir.path().join("home");
    let database_path = config_runtime_paths::global_database_path(&home);
    let database_dir = database_path.parent().expect("global database parent");
    let database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(database.clone())?;
    let sessions = SessionStore::new(database.clone())?;
    let scheduler = AgentSchedulerStore::from_database_path(&database_path);
    let connection = database.connect()?;

    assert_eq!(database.path(), database_path);
    assert_eq!(sessions.storage_path(), Some(database.path()));
    assert_eq!(scheduler.database_path(), database.path());
    assert_eq!(
        database.path(),
        config_runtime_paths::home_config_dir(&home).join("database/vibe-window.sqlite3")
    );
    assert!(!legacy_database_path(&home).exists());
    assert_eq!(connection.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))?, 1);
    assert_eq!(
        connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?,
        "wal"
    );
    assert_eq!(connection.query_row("PRAGMA synchronous", [], |row| row.get::<_, i64>(0))?, 1);
    assert_eq!(connection.query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0))?, 5_000);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        assert_eq!(std::fs::metadata(&database_dir)?.permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&database_path)?.permissions().mode() & 0o777, 0o600);
    }

    Ok(())
}

#[test]
fn agent_scheduler_for_home_defers_cutover_until_database_open() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&legacy_path, "deferred")?);
    let target_path = config_runtime_paths::global_database_path(home.path());

    let store = crate::agent_scheduler::store::AgentSchedulerStore::for_home(home.path());

    assert_eq!(store.database_path(), target_path);
    assert!(legacy_path.exists());
    assert!(!target_path.exists());
    Ok(())
}

fn legacy_database_path(home: &Path) -> PathBuf {
    config_runtime_paths::home_config_dir(home).join("agent_scheduler").join("_index.sqlite3")
}

fn create_legacy_database(path: &Path, marker: &str) -> Result<Connection> {
    fs::create_dir_all(path.parent().expect("database parent"))?;
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA wal_autocheckpoint = 0;
         CREATE TABLE migration_fixture (marker TEXT NOT NULL);",
    )?;
    connection.execute("INSERT INTO migration_fixture VALUES (?1)", [marker])?;
    fs::write(path.with_extension("lock"), b"")?;
    Ok(connection)
}
