//! 全局数据库后端配置，解析本地配置并校验 PostgreSQL 连接与模式名。
//!
//! 未找到配置文件时安全回退 SQLite；选择 PostgreSQL 时必须同时获得非空连接地址。

use std::fmt;
use std::io;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const DATABASE_URL_ENV: &str = "VIBEWINDOW_DATABASE_URL";
const DEFAULT_POSTGRES_SCHEMA: &str = "vibewindow";
const MAX_POSTGRES_IDENTIFIER_BYTES: usize = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 全局数据库支持的后端类型。
pub enum DatabaseProvider {
    Sqlite,
    Postgres,
}

#[derive(Clone)]
/// 已校验的全局数据库连接配置。
pub struct DatabaseConfig {
    provider: DatabaseProvider,
    postgres_schema: Option<String>,
    postgres_url: Option<String>,
}

impl DatabaseConfig {
    /// 从用户主目录加载配置，并通过回调读取连接地址环境变量。
    pub fn load_for_home(
        home: impl AsRef<Path>,
        read_environment: impl FnOnce(&str) -> Option<String>,
    ) -> Result<Self> {
        let path = vw_config_types::paths::global_database_config_path(home);
        let contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::sqlite()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("read global database config: {}", path.display()));
            }
        };
        let raw: RawDatabaseConfig = serde_json::from_str(&contents)
            .with_context(|| format!("parse global database config: {}", path.display()))?;

        match raw.provider.as_str() {
            "sqlite" => {
                if raw.schema.is_some() {
                    bail!("SQLite global database config must not define schema");
                }
                Ok(Self::sqlite())
            }
            "postgres" => {
                let schema = raw.schema.unwrap_or_else(|| DEFAULT_POSTGRES_SCHEMA.to_string());
                if !is_safe_postgres_identifier(&schema) {
                    bail!(
                        "PostgreSQL global database schema must be a safe identifier of at most {MAX_POSTGRES_IDENTIFIER_BYTES} bytes"
                    );
                }
                let url = read_environment(DATABASE_URL_ENV)
                    .filter(|url| !url.trim().is_empty())
                    .ok_or_else(|| {
                    anyhow::anyhow!(
                        "PostgreSQL global database requires non-empty {DATABASE_URL_ENV}"
                    )
                })?;
                Ok(Self {
                    provider: DatabaseProvider::Postgres,
                    postgres_schema: Some(schema),
                    postgres_url: Some(url.trim().to_string()),
                })
            }
            _ => bail!("global database provider must be exactly sqlite or postgres"),
        }
    }

    /// 返回选定数据库提供者。
    pub fn provider(&self) -> DatabaseProvider {
        self.provider
    }

    /// 返回 PostgreSQL 模式名；SQLite 配置返回 `None`。
    pub fn postgres_schema(&self) -> Option<&str> {
        self.postgres_schema.as_deref()
    }

    /// 返回 PostgreSQL 连接地址；只在 crate 内暴露以避免意外记录敏感值。
    pub(crate) fn postgres_url(&self) -> Option<&str> {
        self.postgres_url.as_deref()
    }

    fn sqlite() -> Self {
        Self { provider: DatabaseProvider::Sqlite, postgres_schema: None, postgres_url: None }
    }
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DatabaseConfig")
            .field("provider", &self.provider)
            .field("postgres_schema", &self.postgres_schema)
            .field("postgres_url", &self.postgres_url.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDatabaseConfig {
    provider: String,
    schema: Option<String>,
}

fn is_safe_postgres_identifier(identifier: &str) -> bool {
    if identifier.is_empty() || identifier.len() > MAX_POSTGRES_IDENTIFIER_BYTES {
        return false;
    }
    let bytes = identifier.as_bytes();
    let first = bytes[0];
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes[1..].iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}
