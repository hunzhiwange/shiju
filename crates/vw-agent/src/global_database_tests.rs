use std::fs;
#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, TransactionBehavior};

use super::config::DatabaseProvider;
use super::{
    GlobalDatabase, absolute_config_path, current_test_global_database_home, move_legacy_database,
    remove_if_exists, run_isolated, run_sync_database,
};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::project::store::ProjectStore;
use crate::session::store::SessionStore;

#[tokio::test]
async fn run_isolated_returns_worker_value_from_named_database_thread() -> Result<()> {
    let (thread_name, has_tokio_runtime) = run_isolated(|| {
        (
            std::thread::current().name().map(str::to_owned),
            tokio::runtime::Handle::try_current().is_ok(),
        )
    })
    .await?;

    assert_eq!(thread_name.as_deref(), Some("vibewindow-database"));
    assert!(!has_tokio_runtime);
    Ok(())
}

#[tokio::test]
async fn run_isolated_reports_worker_termination_before_result() {
    let error = run_isolated::<()>(|| panic!("intentional isolated worker failure"))
        .await
        .expect_err("worker termination must be reported");

    assert!(error.to_string().contains("terminated before returning"));
}

#[tokio::test]
async fn for_home_uses_thread_local_test_home_inside_isolated_worker() -> Result<()> {
    let build_home = option_env!("HOME").context("test build HOME must be available")?;
    let expected =
        vw_config_types::paths::global_database_path(current_test_global_database_home());

    let direct = GlobalDatabase::for_home(Path::new(build_home))?;
    let isolated = run_isolated(move || {
        Ok::<_, anyhow::Error>(
            GlobalDatabase::for_home(Path::new(build_home))?.path().to_path_buf(),
        )
    })
    .await??;

    assert_eq!(direct.path(), expected);
    assert_eq!(isolated, expected);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn run_sync_database_leaves_multithread_tokio_runtime_before_running_operation() -> Result<()>
{
    run_sync_database(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build nested test runtime")?;
        runtime.block_on(async { Ok::<(), anyhow::Error>(()) })
    })?;
    Ok(())
}

#[tokio::test]
async fn run_sync_database_uses_an_isolated_thread_for_current_thread_tokio_runtime() -> Result<()>
{
    let has_tokio_runtime =
        run_sync_database(|| Ok(tokio::runtime::Handle::try_current().is_ok()))?;

    assert!(!has_tokio_runtime);
    Ok(())
}

#[test]
fn global_database_path_helpers_handle_relative_and_missing_paths() -> Result<()> {
    let relative = Path::new("database/config.json");
    assert_eq!(absolute_config_path(relative)?, std::env::current_dir()?.join(relative));

    let temp_dir = tempfile::tempdir()?;
    let missing = temp_dir.path().join("missing.sqlite3");
    remove_if_exists(&missing)?;
    assert!(!missing.exists());

    let error = GlobalDatabase::new(PathBuf::new()).expect_err("empty path must be rejected");
    assert_eq!(error.to_string(), "global database path must name a file");
    Ok(())
}

#[test]
fn global_database_reports_directory_creation_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let blocked_parent = temp_dir.path().join("not-a-directory");
    fs::write(&blocked_parent, b"file blocks directory creation")?;

    let error = GlobalDatabase::new(blocked_parent.join("database.sqlite3"))
        .expect_err("file parent must block database directory creation");

    assert!(format!("{error:#}").contains("create global database directory"));
    Ok(())
}

#[test]
fn sqlite_backend_rejects_postgres_only_accessors_and_pool_comparisons() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database = GlobalDatabase::new(temp_dir.path().join("database.sqlite3"))?;

    assert!(database.initialize_postgres_schema(|_| unreachable!()).is_err());
    assert!(database.postgres_pool_settings().is_err());
    assert!(!database.shares_postgres_pool_with(&database));
    Ok(())
}

#[test]
fn sqlite_connect_reports_pragma_failure_for_invalid_database_file() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let path = temp_dir.path().join("database.sqlite3");
    let database = GlobalDatabase::new(&path)?;
    fs::write(&path, b"not a sqlite database")?;

    let error = database.connect_sqlite().expect_err("invalid SQLite file must fail pragmas");

    assert!(format!("{error:#}").contains("file is not a database"));
    Ok(())
}

#[test]
fn sqlite_schema_initialization_reports_lock_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let path = temp_dir.path().join("database.sqlite3");
    let database = GlobalDatabase::new(&path)?;
    fs::create_dir(path.with_extension("lock"))?;

    let error = database
        .initialize_schema(|_| unreachable!())
        .expect_err("directory at lock path must fail schema initialization");

    assert!(format!("{error:#}").contains("lock SQLite global database schema"));
    Ok(())
}

#[test]
fn sqlite_schema_and_transactions_commit_and_roll_back_expected_changes() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database = GlobalDatabase::new(temp_dir.path().join("database.sqlite3"))?;

    database.initialize_schema(|connection| {
        connection.execute_batch("CREATE TABLE fixture (value TEXT NOT NULL);")?;
        Ok(())
    })?;

    let committed = database.with_transaction(TransactionBehavior::Immediate, |transaction| {
        transaction.execute("INSERT INTO fixture VALUES (?1)", ["committed"])?;
        Ok("ok")
    })?;
    assert_eq!(committed, "ok");

    let error = database
        .with_transaction(TransactionBehavior::Immediate, |transaction| -> Result<()> {
            transaction.execute("INSERT INTO fixture VALUES (?1)", ["rolled-back"])?;
            anyhow::bail!("stop before commit")
        })
        .expect_err("operation errors must leave the transaction uncommitted");
    assert_eq!(error.to_string(), "stop before commit");

    let values = database
        .connect()?
        .prepare("SELECT value FROM fixture ORDER BY value")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    assert_eq!(values, vec!["committed"]);
    Ok(())
}

#[test]
fn move_legacy_database_reports_disappeared_source_without_creating_target() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let source = temp_dir.path().join("legacy.sqlite3");
    let target = temp_dir.path().join("database/vibe-window.sqlite3");

    let error = move_legacy_database(&source, &target).expect_err("missing source must fail");

    assert!(error.to_string().contains("disappeared"));
    assert!(!target.exists());
    Ok(())
}

#[test]
fn move_legacy_database_treats_missing_source_as_done_when_target_exists() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let source = temp_dir.path().join("legacy.sqlite3");
    let target = temp_dir.path().join("database/vibe-window.sqlite3");
    fs::create_dir_all(target.parent().expect("target parent"))?;
    fs::write(&target, b"already migrated")?;

    move_legacy_database(&source, &target)?;

    assert!(!source.with_extension("lock").exists());
    assert_eq!(fs::read(&target)?, b"already migrated");
    Ok(())
}

#[test]
fn remove_if_exists_reports_non_file_removal_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().join("not-a-file");
    fs::create_dir(&directory)?;

    let error = remove_if_exists(&directory).expect_err("directory removal must fail");

    assert!(format!("{error:#}").contains("remove migrated source"));
    Ok(())
}

#[test]
fn global_database_configures_connection_and_secure_permissions() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let home = temp_dir.path().join("home");
    let database_path = vw_config_types::paths::global_database_path(&home);
    let database_dir = database_path.parent().expect("global database parent");
    let database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(database.clone())?;
    let sessions = SessionStore::new(database.clone())?;
    let scheduler = AgentSchedulerStore::from_database_path(&database_path);
    let connection = database.connect()?;

    assert_eq!(database.path(), database_path);
    assert_eq!(sessions.path(), database.path());
    assert_eq!(scheduler.database_path(), database.path());
    assert_eq!(
        database.path(),
        vw_config_types::paths::home_config_dir(&home).join("database/vibe-window.sqlite3")
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
fn global_database_pins_one_backend_without_cross_backend_access_or_hot_switch() -> Result<()> {
    let home = tempfile::tempdir()?;
    let sqlite_path = vw_config_types::paths::global_database_path(home.path());
    let database = GlobalDatabase::for_home_with_environment(home.path(), |_| {
        panic!("default SQLite startup must not read the PostgreSQL URL")
    })?;
    database
        .connect_sqlite()?
        .execute_batch("CREATE TABLE backend_marker (value TEXT NOT NULL);")?;

    assert_eq!(database.provider(), DatabaseProvider::Sqlite);
    assert_eq!(database.sqlite_path()?, sqlite_path);
    assert!(database.connect_postgres().is_err());

    let config_path = vw_config_types::paths::global_database_config_path(home.path());
    fs::write(&config_path, r#"{"provider":"postgres","schema":"changed_after_start"}"#)?;

    assert_eq!(database.provider(), DatabaseProvider::Sqlite);
    assert_eq!(database.clone().provider(), DatabaseProvider::Sqlite);
    assert!(database.connect_sqlite().is_ok());
    assert!(database.connect_postgres().is_err());

    let reopened = GlobalDatabase::for_home_with_environment(home.path(), |_| {
        panic!("process-pinned SQLite backend must not reload PostgreSQL configuration")
    })?;
    assert_eq!(reopened.provider(), DatabaseProvider::Sqlite);
    assert_eq!(reopened.sqlite_path()?, sqlite_path);
    assert!(reopened.connect_postgres().is_err());

    let postgres_home = tempfile::tempdir()?;
    let postgres_config_path =
        vw_config_types::paths::global_database_config_path(postgres_home.path());
    fs::create_dir_all(postgres_config_path.parent().expect("database config parent"))?;
    fs::write(postgres_config_path, r#"{"provider":"postgres","schema":"invalid_tls_test"}"#)?;
    let secret_url = "postgres://secret-user:secret-password@localhost/database?sslmode=prefer";
    let error = GlobalDatabase::for_home_with_environment(postgres_home.path(), |_| {
        Some(secret_url.to_string())
    })
    .expect_err("unsupported TLS policy must fail before any PostgreSQL connection");
    let message = format!("{error:#}");
    assert!(message.contains("sslmode"));
    assert!(!message.contains(secret_url));
    assert!(!message.contains("secret-user"));
    assert!(!message.contains("secret-password"));
    assert!(sqlite_path.exists(), "PostgreSQL selection must not mutate the SQLite database");

    Ok(())
}

#[test]
fn global_database_for_home_moves_previous_shared_database_filename_once() -> Result<()> {
    let home = tempfile::tempdir()?;
    let previous_path = previous_shared_database_path(home.path());
    drop(create_legacy_database(&previous_path, "preserved")?);

    let database = GlobalDatabase::for_home(home.path())?;

    assert_eq!(database.path(), vw_config_types::paths::global_database_path(home.path()));
    assert!(!previous_path.exists());
    assert!(!previous_path.with_extension("sqlite3-wal").exists());
    assert!(!previous_path.with_extension("sqlite3-shm").exists());
    assert!(!previous_path.with_extension("lock").exists());
    assert_eq!(read_marker(database.path())?, "preserved");
    Ok(())
}

#[test]
fn global_database_for_home_moves_legacy_scheduler_database_once() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&legacy_path, "preserved")?);

    let first = GlobalDatabase::for_home(home.path())?;

    let target_path = vw_config_types::paths::global_database_path(home.path());
    assert_eq!(first.path(), target_path);
    assert!(!legacy_path.exists());
    assert!(!legacy_path.with_extension("sqlite3-wal").exists());
    assert!(!legacy_path.with_extension("sqlite3-shm").exists());
    assert!(!legacy_path.with_extension("lock").exists());
    assert_eq!(read_marker(first.path())?, "preserved");

    let second = GlobalDatabase::for_home(home.path())?;
    assert_eq!(second.path(), first.path());
    assert_eq!(read_marker(second.path())?, "preserved");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let database_dir = target_path.parent().expect("global database parent");
        assert_eq!(fs::metadata(database_dir)?.permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(&target_path)?.permissions().mode() & 0o777, 0o600);
    }

    Ok(())
}

#[test]
fn global_database_for_home_rejects_two_legacy_source_paths() -> Result<()> {
    let home = tempfile::tempdir()?;
    let previous_path = previous_shared_database_path(home.path());
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&previous_path, "previous")?);
    drop(create_legacy_database(&legacy_path, "legacy")?);

    let error = GlobalDatabase::for_home(home.path()).expect_err("ambiguous sources must fail");
    let message = error.to_string();

    assert!(message.contains("both source paths"));
    assert_eq!(read_marker(&previous_path)?, "previous");
    assert_eq!(read_marker(&legacy_path)?, "legacy");
    assert!(!vw_config_types::paths::global_database_path(home.path()).exists());
    Ok(())
}

#[test]
fn global_database_for_home_ignores_legacy_workflow_databases() -> Result<()> {
    let home = tempfile::tempdir()?;
    let lifecycle_path = vw_config_types::paths::home_config_dir(home.path())
        .join("task-lifecycle/workflows.sqlite");
    let saved_workflow_path = home.path().join("workflow/workflows.sqlite");
    drop(create_legacy_database(&lifecycle_path, "lifecycle")?);
    drop(create_legacy_database(&saved_workflow_path, "saved-workflow")?);

    let database = GlobalDatabase::for_home(home.path())?;
    let connection = database.connect()?;

    assert_eq!(database.path(), vw_config_types::paths::global_database_path(home.path()));
    assert_eq!(read_marker(&lifecycle_path)?, "lifecycle");
    assert_eq!(read_marker(&saved_workflow_path)?, "saved-workflow");
    assert_eq!(
        connection.query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'migration_fixture'",
            [],
            |row| row.get::<_, i64>(0),
        )?,
        0
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn global_database_for_home_checkpoints_wal_before_move() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    let legacy_connection = create_legacy_database(&legacy_path, "from-wal")?;
    let legacy_wal_path = legacy_path.with_extension("sqlite3-wal");
    assert!(legacy_wal_path.exists(), "fixture must contain a real WAL sidecar");

    let database = GlobalDatabase::for_home(home.path())?;
    drop(legacy_connection);

    assert_eq!(read_marker(database.path())?, "from-wal");
    assert!(!legacy_path.exists());
    assert!(!legacy_wal_path.exists());
    assert!(!legacy_path.with_extension("sqlite3-shm").exists());
    Ok(())
}

#[test]
fn agent_scheduler_for_home_defers_cutover_until_database_open() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&legacy_path, "deferred")?);
    let target_path = vw_config_types::paths::global_database_path(home.path());

    let store = crate::agent_scheduler::store::AgentSchedulerStore::for_home(home.path());

    assert_eq!(store.database_path(), target_path);
    assert!(legacy_path.exists());
    assert!(!target_path.exists());
    Ok(())
}

#[test]
fn global_database_path_cutover_rejects_conflicting_source_and_target() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&legacy_path, "legacy")?);
    let target_path = vw_config_types::paths::global_database_path(home.path());
    drop(create_legacy_database(&target_path, "target")?);

    let error = GlobalDatabase::for_home(home.path()).expect_err("path conflict must fail");

    assert!(error.to_string().contains("both"));
    assert_eq!(read_marker(&legacy_path)?, "legacy");
    assert_eq!(read_marker(&target_path)?, "target");
    Ok(())
}

#[test]
fn move_legacy_database_rejects_existing_target_after_lock_acquire() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let source = temp_dir.path().join("legacy.sqlite3");
    let target = temp_dir.path().join("database/vibe-window.sqlite3");
    drop(create_legacy_database(&source, "legacy")?);
    drop(create_legacy_database(&target, "target")?);

    let error = move_legacy_database(&source, &target).expect_err("target conflict must fail");

    assert!(error.to_string().contains("both source and target exist"));
    assert_eq!(read_marker(&source)?, "legacy");
    assert_eq!(read_marker(&target)?, "target");
    assert!(source.with_extension("lock").exists());
    Ok(())
}

#[test]
fn move_legacy_database_reports_target_directory_creation_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let source = temp_dir.path().join("legacy.sqlite3");
    let blocked_parent = temp_dir.path().join("blocked-parent");
    let target = blocked_parent.join("vibe-window.sqlite3");
    drop(create_legacy_database(&source, "legacy")?);
    fs::write(&blocked_parent, b"file blocks target directory")?;

    let error = move_legacy_database(&source, &target)
        .expect_err("blocked target directory must fail migration");

    assert!(format!("{error:#}").contains("create global database directory"));
    assert_eq!(read_marker(&source)?, "legacy");
    Ok(())
}

#[cfg(unix)]
#[test]
fn move_legacy_database_rejects_active_write_connection() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source = legacy_database_path(home.path());
    let target = vw_config_types::paths::global_database_path(home.path());
    let connection = create_legacy_database(&source, "busy")?;
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         INSERT INTO migration_fixture VALUES ('uncheckpointed');",
    )?;

    let error =
        move_legacy_database(&source, &target).expect_err("active writer must block checkpoint");

    assert!(format!("{error:#}").contains("active write connection"));
    assert_eq!(read_marker(&source)?, "busy");
    Ok(())
}

#[test]
fn drop_postgres_schema_for_test_at_url_rejects_non_test_schema_before_connecting() {
    let error = GlobalDatabase::drop_postgres_schema_for_test_at_url(
        "postgres://postgres:postgres@localhost:1/database?sslmode=disable",
        "vibewindow",
    )
    .expect_err("non-test schema must be rejected before a connection is attempted");

    assert!(error.to_string().contains("vw_test_"));
}

#[test]
fn drop_postgres_schema_for_test_at_url_drops_existing_test_schema_when_configured() -> Result<()> {
    let database_url = match std::env::var("VIBEWINDOW_TEST_POSTGRES_URL") {
        Ok(database_url) => database_url,
        Err(_) => {
            eprintln!(
                "skipping PostgreSQL schema drop coverage: VIBEWINDOW_TEST_POSTGRES_URL is not configured"
            );
            return Ok(());
        }
    };
    let schema = format!("vw_test_{}", uuid::Uuid::new_v4().simple());
    let quoted_schema = super::postgres::quoted_identifier(&schema)?;
    let mut client = ::postgres::Client::connect(&database_url, ::postgres::NoTls)?;
    client.batch_execute(&format!("CREATE SCHEMA {quoted_schema}"))?;

    GlobalDatabase::drop_postgres_schema_for_test_at_url(&database_url, &schema)?;

    let exists: bool =
        client.query_one("SELECT to_regnamespace($1) IS NOT NULL", &[&schema])?.get(0);
    assert!(!exists);
    Ok(())
}

#[test]
fn global_database_path_cutover_preserves_invalid_source() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    fs::create_dir_all(legacy_path.parent().expect("legacy database parent"))?;
    fs::write(&legacy_path, b"not a sqlite database")?;

    let error = GlobalDatabase::for_home(home.path()).expect_err("invalid source must fail");

    assert!(error.to_string().contains("integrity"));
    assert_eq!(fs::read(&legacy_path)?, b"not a sqlite database");
    assert!(!vw_config_types::paths::global_database_path(home.path()).exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn global_database_path_cutover_preserves_locked_source() -> Result<()> {
    let home = tempfile::tempdir()?;
    let legacy_path = legacy_database_path(home.path());
    drop(create_legacy_database(&legacy_path, "locked")?);
    let lock_path = legacy_path.with_extension("lock");
    let lock_file = OpenOptions::new().read(true).write(true).open(&lock_path)?;
    let lock_result = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_eq!(lock_result, 0);

    let error = GlobalDatabase::for_home(home.path()).expect_err("locked source must fail");

    assert!(error.to_string().contains("lock"));
    assert_eq!(read_marker(&legacy_path)?, "locked");
    assert!(!vw_config_types::paths::global_database_path(home.path()).exists());
    Ok(())
}

fn legacy_database_path(home: &Path) -> PathBuf {
    vw_config_types::paths::home_config_dir(home).join("agent_scheduler").join("_index.sqlite3")
}

fn previous_shared_database_path(home: &Path) -> PathBuf {
    vw_config_types::paths::home_config_dir(home).join("database").join("_index.sqlite3")
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

fn read_marker(path: &Path) -> Result<String> {
    Ok(Connection::open(path)?
        .query_row("SELECT marker FROM migration_fixture", [], |row| row.get(0))?)
}
