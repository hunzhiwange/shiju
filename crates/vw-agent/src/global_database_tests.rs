use std::path::{Path, PathBuf};
use anyhow::Result;
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


fn legacy_database_path(home: &Path) -> PathBuf {
    config_runtime_paths::home_config_dir(home).join("agent_scheduler").join("_index.sqlite3")
}
