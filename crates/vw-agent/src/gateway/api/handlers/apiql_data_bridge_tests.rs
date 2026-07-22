use std::collections::BTreeMap;

use serde_json::json;
use vw_api_types::data::{AiDataConnectionDto, AiDataConnectionKind, AiDataCountMode};

use super::data_bridge::{ApiQlDataQuery, execute_apiql_data_query};

fn sqlite_connection(path: &str) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: "apiql-sqlite".to_string(),
        name: "ApiQL SQLite".to_string(),
        kind: AiDataConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        base_url: None,
        connection_url: None,
        sqlite_path: Some(path.to_string()),
        default_path: None,
        auth_token: None,
        headers: BTreeMap::new(),
        schema_hint: None,
        updated_at_ms: 0,
        last_used_ms: None,
    }
}

fn database_connection(
    kind: AiDataConnectionKind,
    connection_url: Option<&str>,
) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: "apiql-database".to_string(),
        name: "ApiQL Database".to_string(),
        kind,
        description: None,
        enabled: true,
        read_only: true,
        base_url: None,
        connection_url: connection_url.map(ToOwned::to_owned),
        sqlite_path: None,
        default_path: None,
        auth_token: None,
        headers: BTreeMap::new(),
        schema_hint: None,
        updated_at_ms: 0,
        last_used_ms: None,
    }
}

fn make_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE items (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            active INTEGER NOT NULL
        );
        INSERT INTO items (id, name, active) VALUES
            (1, 'apple', 1),
            (2, 'banana', 0),
            (3, 'pear', 1);
        "#,
    )
    .unwrap();
}

#[tokio::test]
async fn apiql_data_bridge_executes_read_only_sqlite_query() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql.db");
    make_sqlite_fixture(&db_path);

    let response = execute_apiql_data_query(
        sqlite_connection(db_path.to_str().unwrap()),
        ApiQlDataQuery {
            sql: "SELECT id, name FROM items WHERE active = {{ active }}".to_string(),
            params: BTreeMap::from([("active".to_string(), json!(true))]),
            page: 1,
            limit: 1,
            count_mode: AiDataCountMode::Enabled,
            order_by: Some("id DESC".to_string()),
            ..ApiQlDataQuery::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page.total_record, 2);
    assert_eq!(response.page.total_page, 2);
    assert_eq!(response.items, vec![json!({ "id": 3, "name": "pear" })]);
    assert!(response.has_next_page);
}

#[tokio::test]
async fn apiql_data_bridge_rejects_write_sql() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql.db");
    make_sqlite_fixture(&db_path);

    let err = execute_apiql_data_query(
        sqlite_connection(db_path.to_str().unwrap()),
        ApiQlDataQuery { sql: "DELETE FROM items".to_string(), ..ApiQlDataQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("只读 SELECT/CTE"));
}

#[tokio::test]
async fn apiql_data_bridge_mysql_requires_connection_url() {
    let err = execute_apiql_data_query(
        database_connection(AiDataConnectionKind::Mysql, None),
        ApiQlDataQuery { sql: "SELECT 1".to_string(), ..ApiQlDataQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("connection_url"));
}

#[tokio::test]
async fn apiql_data_bridge_postgres_requires_connection_url() {
    let err = execute_apiql_data_query(
        database_connection(AiDataConnectionKind::Postgres, None),
        ApiQlDataQuery { sql: "SELECT 1".to_string(), ..ApiQlDataQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("connection_url"));
}

#[cfg(not(feature = "memory-mariadb"))]
#[tokio::test]
async fn apiql_data_bridge_mysql_reports_disabled_executor() {
    let err = execute_apiql_data_query(
        database_connection(AiDataConnectionKind::Mysql, Some("mysql://localhost/apiql")),
        ApiQlDataQuery { sql: "SELECT 1".to_string(), ..ApiQlDataQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("memory-mariadb"));
}

#[cfg(not(feature = "memory-postgres"))]
#[tokio::test]
async fn apiql_data_bridge_postgres_reports_disabled_executor() {
    let err = execute_apiql_data_query(
        database_connection(AiDataConnectionKind::Postgres, Some("postgres://localhost/apiql")),
        ApiQlDataQuery { sql: "SELECT 1".to_string(), ..ApiQlDataQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("memory-postgres"));
}
