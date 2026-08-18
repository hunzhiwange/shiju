use std::collections::BTreeMap;

use super::connections::{ApiQlConnection, ApiQlConnectionKind};
use super::executor::{ApiQlCountMode, ApiQlDatabaseQuery, execute_read_only_query};
use serde_json::json;

fn sqlite_connection(path: &str) -> ApiQlConnection {
    ApiQlConnection {
        id: "apiql-sqlite".to_string(),
        name: "ApiQL SQLite".to_string(),
        kind: ApiQlConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        connection_url: None,
        sqlite_path: Some(path.to_string()),
        updated_at_ms: 0,
    }
}

fn database_connection(kind: ApiQlConnectionKind, connection_url: Option<&str>) -> ApiQlConnection {
    ApiQlConnection {
        id: "apiql-database".to_string(),
        name: "ApiQL Database".to_string(),
        kind,
        description: None,
        enabled: true,
        read_only: true,
        connection_url: connection_url.map(ToOwned::to_owned),
        sqlite_path: None,
        updated_at_ms: 0,
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
async fn apiql_executor_executes_read_only_sqlite_query() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql.db");
    make_sqlite_fixture(&db_path);

    let response = execute_read_only_query(
        sqlite_connection(db_path.to_str().unwrap()),
        ApiQlDatabaseQuery {
            sql: "SELECT id, name FROM items WHERE active = {{ active }}".to_string(),
            params: BTreeMap::from([("active".to_string(), json!(true))]),
            page: 1,
            limit: 1,
            count_mode: ApiQlCountMode::Enabled,
            order_by: Some("id DESC".to_string()),
            ..ApiQlDatabaseQuery::default()
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
async fn apiql_executor_rejects_write_sql() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql.db");
    make_sqlite_fixture(&db_path);

    let err = execute_read_only_query(
        sqlite_connection(db_path.to_str().unwrap()),
        ApiQlDatabaseQuery {
            sql: "DELETE FROM items".to_string(),
            ..ApiQlDatabaseQuery::default()
        },
    )
    .await
    .unwrap_err();

    assert!(err.contains("read-only SELECT/CTE"));
}

#[tokio::test]
async fn apiql_executor_mysql_requires_connection_url() {
    let err = execute_read_only_query(
        database_connection(ApiQlConnectionKind::Mysql, None),
        ApiQlDatabaseQuery { sql: "SELECT 1".to_string(), ..ApiQlDatabaseQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("connection_url"));
}

#[tokio::test]
async fn apiql_executor_postgres_requires_connection_url() {
    let err = execute_read_only_query(
        database_connection(ApiQlConnectionKind::Postgres, None),
        ApiQlDatabaseQuery { sql: "SELECT 1".to_string(), ..ApiQlDatabaseQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("connection_url"));
}

#[cfg(not(feature = "memory-mariadb"))]
#[tokio::test]
async fn apiql_executor_mysql_reports_disabled_executor() {
    let err = execute_read_only_query(
        database_connection(ApiQlConnectionKind::Mysql, Some("mysql://localhost/apiql")),
        ApiQlDatabaseQuery { sql: "SELECT 1".to_string(), ..ApiQlDatabaseQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("memory-mariadb"));
}

#[cfg(not(feature = "memory-postgres"))]
#[tokio::test]
async fn apiql_executor_postgres_reports_disabled_executor() {
    let err = execute_read_only_query(
        database_connection(ApiQlConnectionKind::Postgres, Some("postgres://localhost/apiql")),
        ApiQlDatabaseQuery { sql: "SELECT 1".to_string(), ..ApiQlDatabaseQuery::default() },
    )
    .await
    .unwrap_err();

    assert!(err.contains("memory-postgres"));
}
