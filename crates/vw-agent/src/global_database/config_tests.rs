// 验证全局数据库配置的默认值、连接参数与提供方解析。
use std::collections::HashMap;

use anyhow::Result;

use super::config::{DatabaseConfig, DatabaseProvider};

const DATABASE_URL_ENV: &str = "VIBEWINDOW_DATABASE_URL";
const SECRET_DATABASE_URL: &str = "postgres://secret-user:secret-password@localhost/database";

#[test]
fn database_config_defaults_to_sqlite_and_requires_safe_postgres_input() -> Result<()> {
    let home = tempfile::tempdir()?;
    let missing = DatabaseConfig::load_for_home(home.path(), |_| {
        panic!("missing configuration must not read the PostgreSQL URL")
    })?;
    assert!(matches!(missing.provider(), DatabaseProvider::Sqlite));

    let config_path = vw_config_types::paths::global_database_config_path(home.path());
    std::fs::create_dir_all(config_path.parent().expect("database config parent"))?;
    std::fs::write(&config_path, r#"{"provider":"sqlite"}"#)?;
    let sqlite = DatabaseConfig::load_for_home(home.path(), |_| {
        panic!("SQLite configuration must not read the PostgreSQL URL")
    })?;
    assert_eq!(sqlite.provider(), DatabaseProvider::Sqlite);
    let sqlite_clone = sqlite.clone();
    assert_eq!(sqlite_clone.provider(), DatabaseProvider::Sqlite);
    assert_eq!(sqlite_clone.postgres_schema(), None);
    assert_eq!(sqlite_clone.postgres_url(), None);
    assert_eq!(
        format!("{sqlite_clone:?}"),
        "DatabaseConfig { provider: Sqlite, postgres_schema: None, postgres_url: None }"
    );

    let mut environment = HashMap::new();
    environment.insert(DATABASE_URL_ENV.to_string(), SECRET_DATABASE_URL.to_string());
    std::fs::write(&config_path, r#"{"provider":"postgres"}"#)?;
    let postgres = DatabaseConfig::load_for_home(home.path(), |key| environment.get(key).cloned())?;
    let postgres = postgres.clone();
    assert_eq!(postgres.provider(), DatabaseProvider::Postgres);
    assert_eq!(postgres.postgres_schema(), Some("vibewindow"));
    assert_eq!(postgres.postgres_url(), Some(SECRET_DATABASE_URL));
    let debug = format!("{postgres:?}");
    assert!(!debug.contains(SECRET_DATABASE_URL));
    assert!(!debug.contains("secret-user"));
    assert!(!debug.contains("secret-password"));

    std::fs::write(&config_path, r#"{"provider":"postgres","schema":"tenant_42"}"#)?;
    let custom = DatabaseConfig::load_for_home(home.path(), |key| environment.get(key).cloned())?;
    assert_eq!(custom.postgres_schema(), Some("tenant_42"));

    let rejected = [
        (r#"{}"#, None, "provider"),
        (r#"{"provider":"SQLite"}"#, None, "provider"),
        (r#"{"provider":"unknown"}"#, None, "provider"),
        (r#"{"provider":"sqlite","schema":"unused"}"#, None, "schema"),
        (r#"{"provider":"postgres","schema":"bad-name"}"#, Some(SECRET_DATABASE_URL), "schema"),
        (r#"{"provider":"postgres","schema":"9tenant"}"#, Some(SECRET_DATABASE_URL), "schema"),
        (r#"{"provider":"postgres","schema":""}"#, Some(SECRET_DATABASE_URL), "schema"),
        (r#"{"provider":"postgres","migration":true}"#, Some(SECRET_DATABASE_URL), "migration"),
        (r#"{"provider":"postgres","dual_write":true}"#, Some(SECRET_DATABASE_URL), "dual_write"),
        (r#"{"provider":"postgres","hot_reload":true}"#, Some(SECRET_DATABASE_URL), "hot_reload"),
        (r#"{"provider":"postgres"}"#, None, DATABASE_URL_ENV),
        (r#"{"provider":"postgres"}"#, Some("   "), DATABASE_URL_ENV),
    ];
    for (contents, database_url, expected) in rejected {
        std::fs::write(&config_path, contents)?;
        let error = DatabaseConfig::load_for_home(home.path(), |key| {
            assert_eq!(key, DATABASE_URL_ENV);
            database_url.map(str::to_owned)
        })
        .expect_err("unsafe database configuration must fail");
        let message = format!("{error:#}");
        assert!(message.contains(expected), "unexpected error: {message}");
        assert!(!message.contains(SECRET_DATABASE_URL));
        assert!(!message.contains("secret-user"));
        assert!(!message.contains("secret-password"));
    }

    Ok(())
}

#[test]
fn database_config_reports_read_and_parse_context_without_reading_environment() -> Result<()> {
    let unreadable_home = tempfile::tempdir()?;
    let unreadable_path =
        vw_config_types::paths::global_database_config_path(unreadable_home.path());
    std::fs::create_dir_all(&unreadable_path)?;

    let read_error = DatabaseConfig::load_for_home(unreadable_home.path(), |_| {
        panic!("unreadable configuration must fail before reading the environment")
    })
    .expect_err("a directory at the config path must not be readable as configuration");
    assert!(format!("{read_error:#}").contains("read global database config"));

    let malformed_home = tempfile::tempdir()?;
    let malformed_path = vw_config_types::paths::global_database_config_path(malformed_home.path());
    std::fs::create_dir_all(malformed_path.parent().expect("database config parent"))?;
    std::fs::write(&malformed_path, r#"{"provider":"postgres""#)?;

    let parse_error = DatabaseConfig::load_for_home(malformed_home.path(), |_| {
        panic!("malformed configuration must fail before reading the environment")
    })
    .expect_err("malformed JSON must be rejected");
    assert!(format!("{parse_error:#}").contains("parse global database config"));

    Ok(())
}

#[test]
fn postgres_schema_enforces_identifier_byte_limit_and_trims_url() -> Result<()> {
    let home = tempfile::tempdir()?;
    let config_path = vw_config_types::paths::global_database_config_path(home.path());
    std::fs::create_dir_all(config_path.parent().expect("database config parent"))?;

    let boundary_schema = format!("_{}", "a".repeat(62));
    std::fs::write(
        &config_path,
        serde_json::json!({"provider": "postgres", "schema": boundary_schema}).to_string(),
    )?;
    let config = DatabaseConfig::load_for_home(home.path(), |key| {
        assert_eq!(key, DATABASE_URL_ENV);
        Some(format!("  {SECRET_DATABASE_URL}  "))
    })?;
    assert_eq!(config.postgres_schema(), Some(boundary_schema.as_str()));
    assert_eq!(config.postgres_url(), Some(SECRET_DATABASE_URL));

    let overlong_schema = format!("_{}", "a".repeat(63));
    std::fs::write(
        &config_path,
        serde_json::json!({"provider": "postgres", "schema": overlong_schema}).to_string(),
    )?;
    let error = DatabaseConfig::load_for_home(home.path(), |_| Some(SECRET_DATABASE_URL.into()))
        .expect_err("a 64-byte PostgreSQL identifier must be rejected");
    assert!(format!("{error:#}").contains("at most 63 bytes"));

    Ok(())
}
