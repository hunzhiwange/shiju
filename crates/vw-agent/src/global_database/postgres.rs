//! PostgreSQL 连接池与模式初始化实现，封装 TLS 和连接生命周期边界。
//!
//! 连接按 URL 与模式复用；模式创建和迁移持有事务级 advisory lock，避免并发初始化竞态。

use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use postgres::{Client, Config, NoTls};
use r2d2::{ManageConnection, Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;
use tokio_postgres_rustls::MakeRustlsConnect;

const DEFAULT_POOL_MAX_SIZE: u32 = 100;
const DEFAULT_POOL_MAX_LIFETIME: Duration = Duration::from_secs(5 * 60);
const POOL_MIN_IDLE: u32 = 0;
const ADVISORY_SCHEMA_LOCK_SQL: &str =
    "SELECT pg_advisory_xact_lock(hashtext(current_database()), hashtext($1))";

fn drop_postgres_connection<M: Send + 'static + ManageConnection>(connection: PooledConnection<M>) {
    if tokio::runtime::Handle::try_current().is_ok() {
        let _ = thread::Builder::new().name("vw-postgres-connection-drop".to_string()).spawn(
            move || {
                drop(connection);
            },
        );
        return;
    }

    drop(connection);
}

type PlainManager = PostgresConnectionManager<NoTls>;
type RustlsManager = PostgresConnectionManager<MakeRustlsConnect>;

static PROVIDERS_BY_CONFIG_PATH: LazyLock<Mutex<HashMap<PathBuf, Weak<PostgresProvider>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// PostgreSQL 传输加密模式。
pub(super) enum PostgresTlsMode {
    Disable,
    Require,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// PostgreSQL 连接池的有效运行参数。
pub(super) struct PostgresPoolSettings {
    /// 池允许的最大连接数。
    pub(super) max_size: u32,
    /// 单个连接的可选最长存活时间。
    pub(super) max_lifetime: Option<Duration>,
    /// 是否记录 SQL 调试信息。
    pub(super) debug_logging: bool,
}

impl Default for PostgresPoolSettings {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_POOL_MAX_SIZE,
            max_lifetime: Some(DEFAULT_POOL_MAX_LIFETIME),
            debug_logging: false,
        }
    }
}

enum PostgresPool {
    Plain(Pool<PlainManager>),
    Rustls(Pool<RustlsManager>),
}

/// 可透明解引用为 PostgreSQL 客户端的池化连接。
pub(crate) enum PostgresConnection {
    Plain(Option<PooledConnection<PlainManager>>),
    Rustls(Option<PooledConnection<RustlsManager>>),
    Closed,
}

impl Deref for PostgresConnection {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Plain(Some(connection)) => connection,
            Self::Rustls(Some(connection)) => connection,
            Self::Closed => {
                unreachable!("PostgresConnection has been moved into a background drop task")
            }
            Self::Plain(None) => unreachable!("PostgresConnection has been dropped"),
            Self::Rustls(None) => unreachable!("PostgresConnection has been dropped"),
        }
    }
}

impl DerefMut for PostgresConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Plain(Some(connection)) => connection,
            Self::Rustls(Some(connection)) => connection,
            Self::Closed => {
                unreachable!("PostgresConnection has been moved into a background drop task")
            }
            Self::Plain(None) => unreachable!("PostgresConnection has been dropped"),
            Self::Rustls(None) => unreachable!("PostgresConnection has been dropped"),
        }
    }
}

impl Drop for PostgresConnection {
    fn drop(&mut self) {
        match self {
            Self::Plain(connection) => {
                if let Some(connection) = connection.take() {
                    drop_postgres_connection(connection);
                }
            }
            Self::Rustls(connection) => {
                if let Some(connection) = connection.take() {
                    drop_postgres_connection(connection);
                }
            }
            Self::Closed => {}
        }
    }
}

/// 绑定连接地址、租户模式和共享连接池的 PostgreSQL 提供者。
pub(super) struct PostgresProvider {
    schema: String,
    pool: PostgresPool,
    settings: PostgresPoolSettings,
}

impl fmt::Debug for PostgresProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresProvider")
            .field("schema", &self.schema)
            .field("max_size", &self.settings.max_size)
            .field("max_lifetime", &self.settings.max_lifetime)
            .field("debug_logging", &self.settings.debug_logging)
            .finish_non_exhaustive()
    }
}

impl PostgresProvider {
    /// 取得或创建匹配连接地址与模式的共享提供者。
    pub(super) fn shared(
        config_path: &Path,
        database_url: &str,
        schema: &str,
        settings: PostgresPoolSettings,
    ) -> Result<Arc<Self>> {
        let cache_key = absolute_config_path(config_path)?;
        let mut providers = PROVIDERS_BY_CONFIG_PATH
            .lock()
            .map_err(|_| anyhow::anyhow!("lock PostgreSQL global database pool registry"))?;
        if let Some(provider) = providers.get(&cache_key).and_then(Weak::upgrade) {
            return Ok(provider);
        }

        let provider = Arc::new(Self::new(database_url, schema, settings)?);
        providers.insert(cache_key, Arc::downgrade(&provider));
        Ok(provider)
    }

    fn new(database_url: &str, schema: &str, settings: PostgresPoolSettings) -> Result<Self> {
        validate_identifier(schema)?;
        if settings.max_size == 0 {
            bail!("PostgreSQL global database pool max_size must be positive");
        }
        if settings.debug_logging {
            bail!("PostgreSQL global database SQL debug logging must remain disabled");
        }
        let (config, tls_mode) = parse_connection_config(database_url)?;
        let pool = match tls_mode {
            PostgresTlsMode::Disable => {
                let manager = PostgresConnectionManager::new(config, NoTls);
                let pool = Pool::builder()
                    .max_size(settings.max_size)
                    .min_idle(Some(POOL_MIN_IDLE))
                    .max_lifetime(settings.max_lifetime)
                    .build(manager)
                    .with_context(|| {
                        format!("create PostgreSQL global database pool for schema {schema}")
                    })?;
                PostgresPool::Plain(pool)
            }
            PostgresTlsMode::Require => {
                let manager = PostgresConnectionManager::new(config, verified_tls_connector());
                let pool = Pool::builder()
                    .max_size(settings.max_size)
                    .min_idle(Some(POOL_MIN_IDLE))
                    .max_lifetime(settings.max_lifetime)
                    .build(manager)
                    .with_context(|| {
                        format!(
                            "create verified-TLS PostgreSQL global database pool for schema {schema}"
                        )
                    })?;
                PostgresPool::Rustls(pool)
            }
        };
        let provider = Self { schema: schema.to_string(), pool, settings };
        provider.create_schema_if_missing()?;
        drop(provider.connect()?);
        Ok(provider)
    }

    fn create_schema_if_missing(&self) -> Result<()> {
        let mut connection = match &self.pool {
            PostgresPool::Plain(pool) => PostgresConnection::Plain(Some(
                pool.get().with_context(|| {
                format!(
                    "checkout PostgreSQL global database connection to create schema {}",
                    self.schema
                )
            })?,
            )),
            PostgresPool::Rustls(pool) => {
                PostgresConnection::Rustls(Some(
                    pool.get().with_context(|| {
                    format!(
                        "checkout verified-TLS PostgreSQL global database connection to create schema {}",
                        self.schema
                    )
                })?,
                ))
            }
        };
        let quoted_schema = quoted_identifier(&self.schema)?;
        let mut transaction = connection.transaction().map_err(|_| {
            anyhow::anyhow!(
                "begin PostgreSQL global database schema creation transaction for {}",
                self.schema
            )
        })?;
        transaction.query_one(ADVISORY_SCHEMA_LOCK_SQL, &[&self.schema]).map_err(|_| {
            anyhow::anyhow!(
                "acquire PostgreSQL global database advisory schema creation lock for {}",
                self.schema
            )
        })?;
        transaction
            .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS {quoted_schema}"))
            .map_err(|_| {
                anyhow::anyhow!("create PostgreSQL global database schema {}", self.schema)
            })?;
        transaction.commit().map_err(|_| {
            anyhow::anyhow!(
                "commit PostgreSQL global database schema creation transaction for {}",
                self.schema
            )
        })?;
        Ok(())
    }

    /// 从连接池获取客户端并设置目标模式搜索路径。
    pub(super) fn connect(&self) -> Result<PostgresConnection> {
        let mut connection = match &self.pool {
            PostgresPool::Plain(pool) => {
                PostgresConnection::Plain(Some(pool.get().with_context(|| {
                    format!(
                        "checkout PostgreSQL global database connection for schema {}",
                        self.schema
                    )
                })?))
            }
            PostgresPool::Rustls(pool) => {
                PostgresConnection::Rustls(Some(pool.get().with_context(|| {
                    format!(
                        "checkout verified-TLS PostgreSQL global database connection for schema {}",
                        self.schema
                    )
                })?))
            }
        };
        configure_search_path(&mut connection, &self.schema)?;
        Ok(connection)
    }

    #[cfg(test)]
    /// 返回连接池的有效参数快照。
    pub(super) fn settings(&self) -> PostgresPoolSettings {
        self.settings
    }

    /// 在持有模式级事务锁时运行表结构初始化。
    pub(super) fn initialize_schema(
        &self,
        initialize: impl FnOnce(&mut postgres::Transaction<'_>) -> Result<()>,
    ) -> Result<()> {
        let mut connection = self.connect()?;
        let mut transaction = connection.transaction().map_err(|_| {
            anyhow::anyhow!(
                "begin PostgreSQL global database schema transaction for {}",
                self.schema
            )
        })?;
        transaction.query_one(ADVISORY_SCHEMA_LOCK_SQL, &[&self.schema]).map_err(|_| {
            anyhow::anyhow!(
                "acquire PostgreSQL global database advisory schema lock for {}",
                self.schema
            )
        })?;
        initialize(&mut transaction)?;
        transaction.commit().map_err(|_| {
            anyhow::anyhow!(
                "commit PostgreSQL global database schema transaction for {}",
                self.schema
            )
        })?;
        Ok(())
    }
}

/// 解析连接地址并选择明文或 TLS 连接模式。
pub(super) fn parse_connection_config(database_url: &str) -> Result<(Config, PostgresTlsMode)> {
    let parsed_url = url::Url::parse(database_url).map_err(|_| {
        anyhow::anyhow!("PostgreSQL global database URL must be a valid URL with explicit sslmode")
    })?;
    if !matches!(parsed_url.scheme(), "postgres" | "postgresql") {
        bail!("PostgreSQL global database URL must use the postgres scheme with explicit sslmode");
    }

    let mut ssl_modes = parsed_url
        .query_pairs()
        .filter_map(|(key, value)| (key == "sslmode").then_some(value.into_owned()));
    let ssl_mode = ssl_modes.next().ok_or_else(|| {
        anyhow::anyhow!(
            "PostgreSQL global database URL requires explicit sslmode=require or sslmode=disable"
        )
    })?;
    if ssl_modes.next().is_some() {
        bail!("PostgreSQL global database URL must define sslmode exactly once");
    }
    let tls_mode = match ssl_mode.as_str() {
        "disable" => PostgresTlsMode::Disable,
        "require" => PostgresTlsMode::Require,
        _ => bail!(
            "PostgreSQL global database URL only accepts explicit sslmode=require or sslmode=disable"
        ),
    };
    let config = Config::from_str(database_url).map_err(|_| {
        anyhow::anyhow!("PostgreSQL global database URL is invalid after sslmode validation")
    })?;
    Ok((config, tls_mode))
}

#[cfg(test)]
/// 建立不设置租户模式的独立连接，仅供模式管理使用。
pub(super) fn connect_unscoped(database_url: &str) -> Result<Client> {
    let (config, tls_mode) = parse_connection_config(database_url)?;
    match tls_mode {
        PostgresTlsMode::Disable => config
            .connect(NoTls)
            .map_err(|_| anyhow::anyhow!("connect local PostgreSQL test database")),
        PostgresTlsMode::Require => config
            .connect(verified_tls_connector())
            .map_err(|_| anyhow::anyhow!("connect verified-TLS local PostgreSQL test database")),
    }
}

/// 校验并引用 PostgreSQL 标识符，防止模式名进入 SQL 注入路径。
pub(super) fn quoted_identifier(identifier: &str) -> Result<String> {
    validate_identifier(identifier)?;
    Ok(format!("\"{identifier}\""))
}

#[cfg(test)]
/// 限制测试模式必须带专用前缀，避免清理生产模式。
pub(super) fn validate_test_schema(schema: &str) -> Result<()> {
    let suffix = schema.strip_prefix("vw_test_").unwrap_or_default();
    if suffix.len() != 32
        || !suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("PostgreSQL test schema must match vw_test_<uuid>");
    }
    Ok(())
}

fn configure_search_path(connection: &mut Client, schema: &str) -> Result<()> {
    let quoted_schema = quoted_identifier(schema)?;
    connection.batch_execute(&format!("SET search_path TO {quoted_schema}")).map_err(|_| {
        anyhow::anyhow!("set PostgreSQL global database search_path to schema {schema}")
    })?;
    let current_schema: Option<String> = connection
        .query_one("SELECT current_schema()", &[])
        .map_err(|_| {
            anyhow::anyhow!("verify PostgreSQL global database search_path for schema {schema}")
        })?
        .get(0);
    if current_schema.as_deref() != Some(schema) {
        bail!("PostgreSQL global database search_path did not resolve schema {schema}");
    }
    Ok(())
}

fn validate_identifier(identifier: &str) -> Result<()> {
    let mut bytes = identifier.bytes();
    let valid = identifier.len() <= 63
        && bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if !valid {
        bail!("PostgreSQL global database schema must be a safe identifier");
    }
    Ok(())
}

fn verified_tls_connector() -> MakeRustlsConnect {
    let roots: rustls::RootCertStore = webpki_roots::TLS_SERVER_ROOTS.iter().cloned().collect();
    let config =
        rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    MakeRustlsConnect::new(config)
}

fn absolute_config_path(config_path: &Path) -> Result<PathBuf> {
    if config_path.is_absolute() {
        return Ok(config_path.to_path_buf());
    }
    Ok(std::env::current_dir()
        .map_err(|_| anyhow::anyhow!("resolve PostgreSQL global database config path"))?
        .join(config_path))
}
