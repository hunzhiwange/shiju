//! 全局数据库门面，管理后端复用、SQLite 文件安全与旧库迁移。
//!
//! 同一配置路径复用后端实例；SQLite 创建时收紧文件权限，PostgreSQL 操作保持显式提供者边界。

#[cfg(test)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io;
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub mod config;
mod postgres;

use config::{DatabaseConfig, DatabaseProvider};
use postgres::{PostgresConnection, PostgresPoolSettings, PostgresProvider};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const LEGACY_DATABASE_DIRECTORY: &str = "agent_scheduler";
const PREVIOUS_DATABASE_FILE: &str = "_index.sqlite3";

static BACKENDS_BY_CONFIG_PATH: LazyLock<Mutex<HashMap<PathBuf, Arc<GlobalDatabaseBackend>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 在线程隔离环境执行同步操作，并将任务崩溃转换为错误。
pub(crate) async fn run_isolated<T>(operation: impl FnOnce() -> T + Send + 'static) -> Result<T>
where
    T: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    #[cfg(test)]
    let test_global_database_home = current_test_global_database_home();
    std::thread::Builder::new()
        .name("vibewindow-database".to_string())
        .spawn(move || {
            #[cfg(test)]
            TEST_GLOBAL_DATABASE_HOME_OVERRIDE.with(|slot| {
                *slot.borrow_mut() = Some(test_global_database_home);
            });
            let _ = sender.send(operation());
        })
        .context("spawn isolated global database thread")?;
    receiver.await.context("isolated global database thread terminated before returning")
}

/// 在当前线程或临时工作线程执行同步数据库操作，避免阻塞异步运行时工作线程。
pub(crate) fn run_sync_database<T>(operation: impl FnOnce() -> Result<T> + Send) -> Result<T>
where
    T: Send,
{
    let inside_tokio = tokio::runtime::Handle::try_current().is_ok();
    if !inside_tokio {
        return operation();
    }

    std::thread::scope(|scope| {
        scope
            .spawn(operation)
            .join()
            .map_err(|_| anyhow::anyhow!("isolated synchronous database thread panicked"))?
    })
}

#[derive(Debug, Clone)]
/// 全局 SQLite 或 PostgreSQL 数据库的共享访问门面。
pub struct GlobalDatabase {
    backend: Arc<GlobalDatabaseBackend>,
}

#[derive(Debug)]
enum GlobalDatabaseBackend {
    Sqlite { path: PathBuf },
    Postgres(Arc<PostgresProvider>),
}

#[cfg(test)]
static TEST_GLOBAL_DATABASE_HOMES: LazyLock<Mutex<Vec<tempfile::TempDir>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
std::thread_local! {
    static TEST_GLOBAL_DATABASE_HOME: tempfile::TempDir =
        tempfile::tempdir().expect("create isolated global database test home");
    static TEST_GLOBAL_DATABASE_HOME_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn current_test_global_database_home() -> PathBuf {
    if let Some(path) = TEST_GLOBAL_DATABASE_HOME_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return path;
    }
    TEST_GLOBAL_DATABASE_HOME.with(|test_home| test_home.path().to_path_buf())
}

/// 让阻塞边界继承测试调用者的数据库，退出或 panic 时恢复线程原有隔离环境。
#[cfg(test)]
pub(crate) fn inherit_test_database_context<T>(
    operation: impl FnOnce() -> T,
) -> impl FnOnce() -> T {
    let home = current_test_global_database_home();
    move || {
        struct Restore(Option<PathBuf>);
        impl Drop for Restore {
            fn drop(&mut self) {
                TEST_GLOBAL_DATABASE_HOME_OVERRIDE.with(|slot| *slot.borrow_mut() = self.0.take());
            }
        }
        let _restore =
            Restore(TEST_GLOBAL_DATABASE_HOME_OVERRIDE.with(|slot| slot.replace(Some(home))));
        operation()
    }
}

impl GlobalDatabase {
    /// 从用户主目录加载后端配置并取得可复用数据库实例。
    pub fn for_home(home: impl AsRef<Path>) -> Result<Self> {
        // 专用窗口集成测试进程显式指定临时库；不改变 HOME，也不读取真实后端配置。
        #[cfg(test)]
        if let Some(root) = std::env::var_os("VW_MODEL_RECOVERY_TEST_ROOT") {
            let root = PathBuf::from(root);
            anyhow::ensure!(
                root.is_absolute() && root.is_dir(),
                "window test root must exist and be absolute"
            );
            return Self::new(vw_config_types::paths::global_database_path(root));
        }
        let home = home.as_ref();
        #[cfg(test)]
        if option_env!("HOME").is_some_and(|build_home| home == Path::new(build_home)) {
            return Self::new(vw_config_types::paths::global_database_path(
                current_test_global_database_home(),
            ));
        }

        run_sync_database(|| Self::for_home_with_environment(home, |key| std::env::var(key).ok()))
    }

    /// 使用显式环境读取器加载数据库，供测试隔离环境变量。
    pub(crate) fn for_home_with_environment(
        home: impl AsRef<Path>,
        read_environment: impl FnOnce(&str) -> Option<String>,
    ) -> Result<Self> {
        Self::for_home_with_postgres_settings(
            home,
            read_environment,
            PostgresPoolSettings::default(),
        )
    }

    fn for_sqlite_home(home: &Path) -> Result<Self> {
        let target_path = vw_config_types::paths::global_database_path(home);
        let config_directory = vw_config_types::paths::home_config_dir(home);
        let previous_path = config_directory.join("database").join(PREVIOUS_DATABASE_FILE);
        let legacy_path =
            config_directory.join(LEGACY_DATABASE_DIRECTORY).join(PREVIOUS_DATABASE_FILE);
        let source_path = match (previous_path.exists(), legacy_path.exists()) {
            (false, false) => return Self::new(target_path),
            (true, false) => previous_path,
            (false, true) => legacy_path,
            (true, true) => {
                bail!(
                    "global database path cutover refused because both source paths exist: {} and {}",
                    previous_path.display(),
                    legacy_path.display()
                );
            }
        };
        if target_path.exists() {
            bail!(
                "global database path cutover refused because both source and target exist: {} and {}",
                source_path.display(),
                target_path.display()
            );
        }

        move_legacy_database(&source_path, &target_path)?;
        Self::new(target_path)
    }

    fn for_home_with_postgres_settings(
        home: impl AsRef<Path>,
        read_environment: impl FnOnce(&str) -> Option<String>,
        postgres_settings: PostgresPoolSettings,
    ) -> Result<Self> {
        let home = home.as_ref();
        let config_path =
            absolute_config_path(&vw_config_types::paths::global_database_config_path(home))?;
        let mut backends = BACKENDS_BY_CONFIG_PATH
            .lock()
            .map_err(|_| anyhow::anyhow!("lock global database backend registry"))?;
        if let Some(backend) = backends.get(&config_path) {
            return Ok(Self { backend: backend.clone() });
        }

        let config = DatabaseConfig::load_for_home(home, read_environment)?;
        let database = match config.provider() {
            DatabaseProvider::Sqlite => Self::for_sqlite_home(home),
            DatabaseProvider::Postgres => {
                let schema = config
                    .postgres_schema()
                    .context("PostgreSQL global database config has no schema")?;
                let database_url = config
                    .postgres_url()
                    .context("PostgreSQL global database config has no URL")?;
                let provider = PostgresProvider::shared(
                    &config_path,
                    database_url,
                    schema,
                    postgres_settings,
                )?;
                Ok(Self { backend: Arc::new(GlobalDatabaseBackend::Postgres(provider)) })
            }
        }?;
        backends.insert(config_path, database.backend.clone());
        Ok(database)
    }

    /// 打开显式 SQLite 路径并执行旧库迁移与权限检查。
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if path.as_os_str().is_empty() || path.file_name().is_none() {
            bail!("global database path must name a file");
        }

        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("create global database directory: {}", parent.display())
            })?;
            set_directory_permissions(parent)?;
        }

        create_database_file(&path)?;
        Ok(Self { backend: Arc::new(GlobalDatabaseBackend::Sqlite { path }) })
    }

    /// 返回当前数据库提供者。
    pub fn provider(&self) -> DatabaseProvider {
        match self.backend.as_ref() {
            GlobalDatabaseBackend::Sqlite { .. } => DatabaseProvider::Sqlite,
            GlobalDatabaseBackend::Postgres(_) => DatabaseProvider::Postgres,
        }
    }

    /// 返回 SQLite 文件路径；PostgreSQL 后端调用会失败。
    pub fn sqlite_path(&self) -> Result<&Path> {
        match self.backend.as_ref() {
            GlobalDatabaseBackend::Sqlite { path } => Ok(path),
            GlobalDatabaseBackend::Postgres(_) => {
                bail!("PostgreSQL global database does not expose a SQLite path")
            }
        }
    }

    /// 返回兼容旧调用者的数据库路径；PostgreSQL 后端得到配置路径。
    pub fn path(&self) -> &Path {
        self.sqlite_path()
            .expect("path() is SQLite-only; select sqlite_path() when the provider is known")
    }

    /// 创建初始化过的 SQLite 连接；非 SQLite 后端会失败。
    pub fn connect_sqlite(&self) -> Result<Connection> {
        let path = self.sqlite_path()?;
        let connection = Connection::open(path)
            .with_context(|| format!("open SQLite global database: {}", path.display()))?;
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(connection)
    }

    /// `connect_sqlite` 的兼容别名。
    pub fn connect(&self) -> Result<Connection> {
        self.connect_sqlite()
    }

    /// 从共享池取得 PostgreSQL 连接；非 PostgreSQL 后端会失败。
    pub(crate) fn connect_postgres(&self) -> Result<PostgresConnection> {
        crate::global_database::run_sync_database(|| match self.backend.as_ref() {
            GlobalDatabaseBackend::Sqlite { .. } => {
                bail!("SQLite global database does not expose a PostgreSQL connection")
            }
            GlobalDatabaseBackend::Postgres(provider) => provider.connect(),
        })
    }

    /// 在 SQLite 连接上串行执行一次表结构初始化。
    pub(crate) fn initialize_sqlite_schema(
        &self,
        initialize: impl FnOnce(&Connection) -> Result<()>,
    ) -> Result<()> {
        let path = self.sqlite_path()?;
        let lock_path = path.with_extension("lock");
        let _guard = SchemaLockGuard::acquire(&lock_path).with_context(|| {
            format!("lock SQLite global database schema: {}", lock_path.display())
        })?;
        let connection = self.connect_sqlite()?;
        initialize(&connection)
    }

    /// 按当前提供者执行表结构初始化闭包。
    pub(crate) fn initialize_schema(
        &self,
        initialize: impl FnOnce(&Connection) -> Result<()>,
    ) -> Result<()> {
        self.initialize_sqlite_schema(initialize)
    }

    /// 在带模式锁的 PostgreSQL 事务中初始化表结构。
    pub(crate) fn initialize_postgres_schema(
        &self,
        initialize: impl FnOnce(&mut ::postgres::Transaction<'_>) -> Result<()> + Send,
    ) -> Result<()> {
        run_sync_database(|| match self.backend.as_ref() {
            GlobalDatabaseBackend::Sqlite { .. } => {
                bail!("SQLite global database has no PostgreSQL schema transaction")
            }
            GlobalDatabaseBackend::Postgres(provider) => provider.initialize_schema(initialize),
        })
    }

    /// 在立即事务中执行 SQLite 写操作并提交。
    pub(crate) fn with_sqlite_transaction<T>(
        &self,
        behavior: TransactionBehavior,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut connection = self.connect_sqlite()?;
        let transaction = connection.transaction_with_behavior(behavior)?;
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    /// 按当前提供者执行事务闭包。
    pub(crate) fn with_transaction<T>(
        &self,
        behavior: TransactionBehavior,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        self.with_sqlite_transaction(behavior, operation)
    }

    #[cfg(test)]
    fn shares_postgres_pool_with(&self, other: &Self) -> bool {
        match (self.backend.as_ref(), other.backend.as_ref()) {
            (GlobalDatabaseBackend::Postgres(left), GlobalDatabaseBackend::Postgres(right)) => {
                Arc::ptr_eq(left, right)
            }
            _ => false,
        }
    }

    #[cfg(test)]
    fn postgres_pool_settings(&self) -> Result<PostgresPoolSettings> {
        match self.backend.as_ref() {
            GlobalDatabaseBackend::Sqlite { .. } => {
                bail!("SQLite global database has no PostgreSQL pool settings")
            }
            GlobalDatabaseBackend::Postgres(provider) => Ok(provider.settings()),
        }
    }

    #[cfg(test)]
    fn drop_postgres_schema_for_test(&self, schema: &str) -> Result<()> {
        postgres::validate_test_schema(schema)?;
        let mut connection = self.connect_postgres()?;
        connection
            .batch_execute("SET search_path TO pg_catalog")
            .map_err(|_| anyhow::anyhow!("leave isolated PostgreSQL test schema"))?;
        let schema = postgres::quoted_identifier(schema)?;
        connection
            .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
            .map_err(|_| anyhow::anyhow!("drop isolated PostgreSQL test schema"))?;
        Ok(())
    }

    #[cfg(test)]
    fn drop_postgres_schema_for_test_at_url(database_url: &str, schema: &str) -> Result<()> {
        postgres::validate_test_schema(schema)?;
        let mut connection = postgres::connect_unscoped(database_url)?;
        let schema = postgres::quoted_identifier(schema)?;
        connection
            .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
            .map_err(|_| anyhow::anyhow!("drop isolated PostgreSQL test schema"))?;
        Ok(())
    }
}

fn absolute_config_path(config_path: &Path) -> Result<PathBuf> {
    if config_path.is_absolute() {
        return Ok(config_path.to_path_buf());
    }
    Ok(std::env::current_dir().context("resolve global database config path")?.join(config_path))
}

fn move_legacy_database(source_path: &Path, target_path: &Path) -> Result<()> {
    let lock_path = source_path.with_extension("lock");
    let source_guard = MigrationLockGuard::try_acquire(&lock_path).with_context(|| {
        format!("lock legacy global database for path cutover: {}", source_path.display())
    })?;

    if !source_path.exists() {
        if target_path.exists() {
            drop(source_guard);
            remove_if_exists(&lock_path)?;
            return Ok(());
        }
        bail!("legacy global database disappeared during path cutover: {}", source_path.display());
    }
    if target_path.exists() {
        bail!(
            "global database path cutover refused because both source and target exist: {} and {}",
            source_path.display(),
            target_path.display()
        );
    }

    checkpoint_legacy_database(source_path)?;
    set_file_permissions(source_path)?;
    let target_directory = target_path.parent().context("global database target has no parent")?;
    std::fs::create_dir_all(target_directory).with_context(|| {
        format!("create global database directory: {}", target_directory.display())
    })?;
    set_directory_permissions(target_directory)?;
    std::fs::rename(source_path, target_path).with_context(|| {
        format!(
            "move legacy global database into shared directory: {} -> {}",
            source_path.display(),
            target_path.display()
        )
    })?;
    remove_if_exists(&source_path.with_extension("sqlite3-wal"))?;
    remove_if_exists(&source_path.with_extension("sqlite3-shm"))?;
    drop(source_guard);
    remove_if_exists(&lock_path)?;
    Ok(())
}

fn checkpoint_legacy_database(source_path: &Path) -> Result<()> {
    let connection = Connection::open(source_path)
        .with_context(|| format!("open legacy global database: {}", source_path.display()))?;
    connection.busy_timeout(Duration::ZERO)?;
    let integrity: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .context("verify legacy global database integrity")?;
    if integrity != "ok" {
        bail!("legacy global database integrity verification failed: {integrity}");
    }
    let (busy, _log_frames, _checkpointed_frames): (i64, i64, i64) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if busy != 0 {
        bail!("legacy global database has an active write connection");
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("remove migrated source: {}", path.display()))
        }
    }
}

fn create_database_file(path: &Path) -> Result<()> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(path)
        .with_context(|| format!("create global database file: {}", path.display()))?;
    set_file_permissions(path)
}

#[cfg(unix)]
fn set_directory_permissions(path: &Path) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .with_context(|| format!("secure global database directory: {}", path.display()))
}

#[cfg(not(unix))]
fn set_directory_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_file_permissions(path: &Path) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("secure global database file: {}", path.display()))
}

#[cfg(not(unix))]
fn set_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

struct SchemaLockGuard {
    _file: File,
}

struct MigrationLockGuard {
    _file: File,
}

impl MigrationLockGuard {
    fn try_acquire(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(path)?;
        #[cfg(unix)]
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        try_acquire_exclusive_file_lock(&file)?;
        Ok(Self { _file: file })
    }
}

impl SchemaLockGuard {
    fn acquire(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(path)?;
        #[cfg(unix)]
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        acquire_exclusive_file_lock(&file)?;
        Ok(Self { _file: file })
    }
}

#[cfg(unix)]
fn acquire_exclusive_file_lock(file: &File) -> io::Result<()> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

#[cfg(not(unix))]
fn acquire_exclusive_file_lock(_file: &File) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn try_acquire_exclusive_file_lock(file: &File) -> io::Result<()> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

#[cfg(not(unix))]
fn try_acquire_exclusive_file_lock(_file: &File) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "global_database/config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "global_database/postgres_tests.rs"]
mod postgres_tests;

#[cfg(test)]
#[path = "global_database/test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "global_database_tests.rs"]
mod global_database_tests;
