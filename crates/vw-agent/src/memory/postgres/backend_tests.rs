// 验证 PostgreSQL 记忆后端的标识符安全、连接超时与建表语句。

// 模式 SQL 还需保证依赖对象先于索引创建，并可重复初始化。
use super::super::{quote_identifier, validate_identifier};
use super::*;

#[test]
fn validates_schema_and_table_identifiers_before_connecting() {
    assert!(validate_identifier("agent_memory", "schema").is_ok());
    assert!(validate_identifier("1bad", "schema").is_err());
    assert!(validate_identifier("bad-name", "table").is_err());
    assert_eq!(quote_identifier("memories"), "\"memories\"");
}

#[test]
fn connect_timeout_is_bounded_to_postgres_cap() {
    assert_eq!(PostgresMemory::bounded_connect_timeout(0), std::time::Duration::from_secs(0));
    assert_eq!(PostgresMemory::bounded_connect_timeout(42), std::time::Duration::from_secs(42));
    assert_eq!(
        PostgresMemory::bounded_connect_timeout(POSTGRES_CONNECT_TIMEOUT_CAP_SECS + 1),
        std::time::Duration::from_secs(POSTGRES_CONNECT_TIMEOUT_CAP_SECS)
    );
}

#[test]
fn connect_timeout_accepts_exact_cap_and_clamps_largest_value() {
    assert_eq!(
        PostgresMemory::bounded_connect_timeout(POSTGRES_CONNECT_TIMEOUT_CAP_SECS),
        std::time::Duration::from_secs(POSTGRES_CONNECT_TIMEOUT_CAP_SECS)
    );
    assert_eq!(
        PostgresMemory::bounded_connect_timeout(u64::MAX),
        std::time::Duration::from_secs(POSTGRES_CONNECT_TIMEOUT_CAP_SECS)
    );
}

#[test]
fn schema_sql_uses_quoted_schema_and_table_everywhere() {
    let sql = PostgresMemory::schema_sql("\"agent\"", "\"agent\".\"memories\"");

    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS \"agent\""));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"agent\".\"memories\""));
    assert!(sql.contains("id TEXT PRIMARY KEY"));
    assert!(sql.contains("key TEXT UNIQUE NOT NULL"));
    assert!(sql.contains("session_id TEXT"));
    assert!(sql.contains("idx_memories_category ON \"agent\".\"memories\"(category)"));
    assert!(sql.contains("idx_memories_session_id ON \"agent\".\"memories\"(session_id)"));
    assert!(sql.contains("idx_memories_updated_at ON \"agent\".\"memories\"(updated_at DESC)"));
}

#[test]
fn schema_sql_defines_complete_idempotent_schema() {
    let sql = PostgresMemory::schema_sql("\"archive\"", "\"archive\".\"entries\"");

    assert_eq!(sql.matches("CREATE SCHEMA IF NOT EXISTS").count(), 1);
    assert_eq!(sql.matches("CREATE TABLE IF NOT EXISTS").count(), 1);
    assert_eq!(sql.matches("CREATE INDEX IF NOT EXISTS").count(), 3);
    assert_eq!(sql.matches("\"archive\".\"entries\"").count(), 4);
    assert!(sql.contains("content TEXT NOT NULL"));
    assert!(sql.contains("category TEXT NOT NULL"));
    assert!(sql.contains("created_at TIMESTAMPTZ NOT NULL"));
    assert!(sql.contains("updated_at TIMESTAMPTZ NOT NULL"));
}

#[test]
fn schema_sql_orders_dependencies_before_indexes() {
    let sql = PostgresMemory::schema_sql("\"agent\"", "\"agent\".\"memories\"");
    let schema = sql.find("CREATE SCHEMA").expect("schema statement");
    let table = sql.find("CREATE TABLE").expect("table statement");
    let category_index = sql.find("idx_memories_category").expect("category index statement");
    let session_index = sql.find("idx_memories_session_id").expect("session index statement");
    let updated_index = sql.find("idx_memories_updated_at").expect("updated index statement");

    assert!(schema < table);
    assert!(table < category_index);
    assert!(category_index < session_index);
    assert!(session_index < updated_index);
}

#[test]
fn initialize_client_reports_invalid_connection_url_without_connecting() {
    let error = match PostgresMemory::initialize_client(
        "postgres://[invalid-host/db".to_string(),
        Some(999),
        false,
        "\"agent\"".to_string(),
        "\"agent\".\"memories\"".to_string(),
    ) {
        Ok(_) => panic!("invalid connection URL should fail"),
        Err(error) => error,
    };

    assert!(format!("{error:#}").contains("invalid PostgreSQL connection URL"));
}

#[test]
fn initialize_client_validates_url_before_tls_setup() {
    let error = match PostgresMemory::initialize_client(
        "postgres://[invalid-host/db".to_string(),
        None,
        true,
        "\"agent\"".to_string(),
        "\"agent\".\"memories\"".to_string(),
    ) {
        Ok(_) => panic!("invalid connection URL should fail before TLS setup"),
        Err(error) => error,
    };

    assert!(format!("{error:#}").contains("invalid PostgreSQL connection URL"));
}
