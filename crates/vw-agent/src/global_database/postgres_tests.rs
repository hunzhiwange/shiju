use anyhow::Result;
use std::time::Duration;

use super::config::DatabaseProvider;
use super::postgres::{self, PostgresPoolSettings, PostgresTlsMode};
use super::test_support::{
    LOCAL_DEBUG_LOGGING, LOCAL_MAX_IDLE_BASELINE, LOCAL_MAX_LIFETIME, LOCAL_MAX_OPEN_CONNECTIONS,
    PostgresTestDatabase,
};

const SECRET_URL: &str = "postgres://secret-user:secret-password@localhost/database?sslmode=prefer";

#[test]
fn postgres_connection_requires_explicit_verified_tls_without_secret_disclosure() {
    for url in [
        "postgres://localhost/database",
        SECRET_URL,
        "postgres://localhost/database?sslmode=allow",
        "postgres://localhost/database?sslmode=verify-ca",
    ] {
        let error = postgres::parse_connection_config(url)
            .expect_err("implicit or unsupported TLS policy must fail");
        let message = format!("{error:#}");
        assert!(message.contains("sslmode"));
        assert!(!message.contains(SECRET_URL));
        assert!(!message.contains("secret-user"));
        assert!(!message.contains("secret-password"));
    }

    assert_eq!(
        postgres::parse_connection_config("postgres://localhost/database?sslmode=disable")
            .expect("explicit plaintext mode")
            .1,
        PostgresTlsMode::Disable
    );
    assert_eq!(
        postgres::parse_connection_config("postgres://localhost/database?sslmode=require")
            .expect("explicit verified TLS mode")
            .1,
        PostgresTlsMode::Require
    );
}

#[test]
fn postgres_fixture_uses_local_profile_and_auto_creates_isolated_schema_without_debug_logging()
-> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database();
    let schema = fixture.schema();

    assert_eq!(database.provider(), DatabaseProvider::Postgres);
    assert!(database.connect_sqlite().is_err());
    assert!(schema.starts_with("vw_test_"));
    assert_eq!(fixture.max_idle_baseline(), LOCAL_MAX_IDLE_BASELINE);
    let settings = database.postgres_pool_settings()?;
    assert_eq!(settings.max_size, LOCAL_MAX_OPEN_CONNECTIONS);
    assert_eq!(settings.max_lifetime, Some(LOCAL_MAX_LIFETIME));
    assert_eq!(settings.debug_logging, LOCAL_DEBUG_LOGGING);
    assert_eq!(LOCAL_MAX_IDLE_BASELINE, 10);
    assert_eq!(LOCAL_MAX_OPEN_CONNECTIONS, 100);
    assert_eq!(LOCAL_MAX_LIFETIME.as_secs(), 300);
    assert!(!LOCAL_DEBUG_LOGGING);
    assert!(fixture.max_idle_is_unmapped());
    assert_eq!(fixture.sqlite_sentinel()?, b"sqlite-side-untouched");

    let mut connection = database.connect_postgres()?;
    let current_schema: String = connection.query_one("SELECT current_schema()", &[])?.get(0);
    assert_eq!(current_schema, schema);
    drop(connection);

    database.initialize_postgres_schema(|transaction| {
        transaction
            .batch_execute("CREATE TABLE backend_boundary_fixture (value BIGINT NOT NULL)")?;
        Ok(())
    })?;
    let mut connection = database.connect_postgres()?;
    let table_exists: bool = connection
        .query_one("SELECT to_regclass('backend_boundary_fixture') IS NOT NULL", &[])?
        .get(0);
    assert!(table_exists);
    drop(connection);

    let reopened = fixture.reopen()?;
    assert!(database.shares_postgres_pool_with(&reopened));

    fixture.cleanup()?;
    Ok(())
}

#[test]
fn postgres_fixture_cleanup_refuses_non_test_schema() {
    let error = PostgresTestDatabase::validate_cleanup_schema("vibewindow")
        .expect_err("cleanup must reject production-looking schemas");
    assert!(error.to_string().contains("vw_test_"));
}

#[test]
fn postgres_pool_settings_default_to_bounded_non_debug_profile() {
    let settings = PostgresPoolSettings::default();

    assert_eq!(settings.max_size, 10);
    assert_eq!(settings.max_lifetime, Some(Duration::from_secs(30 * 60)));
    assert!(!settings.debug_logging);
}

#[test]
fn postgres_connection_rejects_non_postgres_and_duplicate_sslmode_urls() {
    for url in [
        "https://localhost/database?sslmode=require",
        "postgres://localhost/database?sslmode=require&sslmode=disable",
        "not a url",
    ] {
        let error = postgres::parse_connection_config(url)
            .expect_err("unsafe connection URL must be rejected");
        assert!(error.to_string().contains("PostgreSQL global database URL"));
    }
}

#[test]
fn postgres_identifier_quoting_accepts_boundary_safe_names() {
    let longest = format!("a{}", "9".repeat(62));

    assert_eq!(postgres::quoted_identifier("_schema9").unwrap(), "\"_schema9\"");
    assert_eq!(postgres::quoted_identifier(&longest).unwrap(), format!("\"{longest}\""));
}

#[test]
fn postgres_identifier_quoting_rejects_unsafe_names() {
    let too_long = format!("a{}", "9".repeat(63));
    for identifier in ["", "9schema", "schema-name", "schema name", "schema\"name", &too_long] {
        let error = postgres::quoted_identifier(identifier)
            .expect_err("unsafe schema identifier must be rejected");
        assert!(error.to_string().contains("safe identifier"));
    }
}

#[test]
fn postgres_test_schema_validation_requires_exact_lowercase_uuid_shape() {
    assert!(postgres::validate_test_schema("vw_test_0123456789abcdef0123456789abcdef").is_ok());

    for schema in [
        "vw_test_0123456789abcdef",
        "vw_test_0123456789ABCDEF0123456789ABCDEF",
        "test_0123456789abcdef0123456789abcdef",
        "vw_test_0123456789abcdef0123456789abcdeg",
    ] {
        assert!(postgres::validate_test_schema(schema).is_err());
    }
}
