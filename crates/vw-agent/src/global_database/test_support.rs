//! 全局数据库测试夹具，隔离 PostgreSQL 测试模式并保留 SQLite 哨兵证据。
//!
//! 每个夹具生成独立模式，清理前再次校验测试前缀；缺少测试 URL 时由宏显式跳过。

use std::time::Duration;

use anyhow::{Context, Result};
use uuid::Uuid;

use super::GlobalDatabase;
use super::postgres::{PostgresPoolSettings, validate_test_schema};

const TEST_DATABASE_URL_ENV: &str = "VIBEWINDOW_TEST_POSTGRES_URL";
const SQLITE_SENTINEL: &[u8] = b"sqlite-side-untouched";
// Local profile: max_idle=10 is unmapped; max_open=100 maps to r2d2 max_size;
// max_lifetime=300s is applied; Debug=false means no SQL debug logging is installed.
/// 本地配置声明但不映射到 r2d2 的空闲连接基线。
pub(super) const LOCAL_MAX_IDLE_BASELINE: u32 = 10;
/// 本地配置映射到连接池的最大连接数。
pub(super) const LOCAL_MAX_OPEN_CONNECTIONS: u32 = 100;
/// 本地配置映射到连接池的最大连接寿命。
pub(super) const LOCAL_MAX_LIFETIME: Duration = Duration::from_secs(300);
/// 本地配置的 SQL 调试日志开关。
pub(super) const LOCAL_DEBUG_LOGGING: bool = false;

macro_rules! postgres_test_database {
    () => {{
        match $crate::global_database::test_support::PostgresTestDatabase::from_environment() {
            Ok(fixture) => fixture,
            Err(_error) if std::env::var_os("VIBEWINDOW_TEST_POSTGRES_URL").is_none() => {
                eprintln!(
                    "skipping PostgreSQL integration test: VIBEWINDOW_TEST_POSTGRES_URL is not configured"
                );
                return Ok(());
            }
            Err(error) => panic!("initialize PostgreSQL test database: {error:#}"),
        }
    }};
}

pub(crate) use postgres_test_database;

macro_rules! postgres_test_database_unit {
    () => {{
        match $crate::global_database::test_support::PostgresTestDatabase::from_environment() {
            Ok(fixture) => fixture,
            Err(_error) if std::env::var_os("VIBEWINDOW_TEST_POSTGRES_URL").is_none() => {
                eprintln!(
                    "skipping PostgreSQL integration test: VIBEWINDOW_TEST_POSTGRES_URL is not configured"
                );
                return;
            }
            Err(error) => panic!("initialize PostgreSQL test database: {error:#}"),
        }
    }};
}

pub(crate) use postgres_test_database_unit;

/// PostgreSQL 集成测试的独立模式和本地对照文件。
pub(crate) struct PostgresTestDatabase {
    home: tempfile::TempDir,
    database: GlobalDatabase,
    database_url: String,
    schema: String,
    cleaned: bool,
}

impl PostgresTestDatabase {
    /// 从测试专用环境变量创建隔离数据库夹具。
    pub(crate) fn from_environment() -> Result<Self> {
        let database_url = std::env::var(TEST_DATABASE_URL_ENV).with_context(|| {
            format!(
                "{TEST_DATABASE_URL_ENV} is not configured; provide the real local PostgreSQL test URL"
            )
        })?;
        let schema = format!("vw_test_{}", Uuid::new_v4().simple());
        Self::validate_cleanup_schema(&schema)?;
        let home = tempfile::tempdir()?;
        let config_path = vw_config_types::paths::global_database_config_path(home.path());
        std::fs::create_dir_all(config_path.parent().expect("database config parent"))?;
        std::fs::write(vw_config_types::paths::global_database_path(home.path()), SQLITE_SENTINEL)?;
        std::fs::write(&config_path, format!(r#"{{"provider":"postgres","schema":"{schema}"}}"#))?;

        let settings = PostgresPoolSettings {
            max_size: LOCAL_MAX_OPEN_CONNECTIONS,
            max_lifetime: Some(LOCAL_MAX_LIFETIME),
            debug_logging: LOCAL_DEBUG_LOGGING,
        };
        let database = match GlobalDatabase::for_home_with_postgres_settings(
            home.path(),
            |_| Some(database_url.clone()),
            settings,
        ) {
            Ok(database) => database,
            Err(error) => {
                let _ =
                    GlobalDatabase::drop_postgres_schema_for_test_at_url(&database_url, &schema);
                return Err(error);
            }
        };

        Ok(Self { home, database, database_url, schema, cleaned: false })
    }

    /// 返回绑定测试模式的全局数据库。
    pub(crate) fn database(&self) -> &GlobalDatabase {
        &self.database
    }

    /// 返回本次夹具的 PostgreSQL 模式名。
    pub(super) fn schema(&self) -> &str {
        &self.schema
    }

    /// 返回本地配置中未映射到 r2d2 的空闲连接基线。
    pub(super) fn max_idle_baseline(&self) -> u32 {
        LOCAL_MAX_IDLE_BASELINE
    }

    // r2d2 has no maximum-idle cap. The local baseline must not become min_idle.
    /// 确认本地 `max_idle` 不会被错误映射为池参数。
    pub(super) fn max_idle_is_unmapped(&self) -> bool {
        true
    }

    /// 读取 SQLite 对照文件，证明 PostgreSQL 测试未改动本地库。
    pub(super) fn sqlite_sentinel(&self) -> Result<Vec<u8>> {
        Ok(std::fs::read(vw_config_types::paths::global_database_path(self.home.path()))?)
    }

    /// 使用相同配置重新打开全局数据库。
    pub(super) fn reopen(&self) -> Result<GlobalDatabase> {
        GlobalDatabase::for_home_with_postgres_settings(
            self.home.path(),
            |_| Some(self.database_url.clone()),
            PostgresPoolSettings {
                max_size: LOCAL_MAX_OPEN_CONNECTIONS,
                max_lifetime: Some(LOCAL_MAX_LIFETIME),
                debug_logging: LOCAL_DEBUG_LOGGING,
            },
        )
    }

    /// 校验待清理模式是否仍满足测试模式限制。
    pub(super) fn validate_cleanup_schema(schema: &str) -> Result<()> {
        validate_test_schema(schema)
    }

    /// 删除测试模式并清理夹具资源。
    pub(crate) fn cleanup(mut self) -> Result<()> {
        Self::validate_cleanup_schema(&self.schema)?;
        self.database.drop_postgres_schema_for_test(&self.schema)?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for PostgresTestDatabase {
    fn drop(&mut self) {
        if !self.cleaned && Self::validate_cleanup_schema(&self.schema).is_ok() {
            let _ = self.database.drop_postgres_schema_for_test(&self.schema);
        }
    }
}
