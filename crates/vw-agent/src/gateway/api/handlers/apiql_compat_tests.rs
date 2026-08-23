// 覆盖 ApiQL 对 PHP 风格输入和输出的兼容转换。

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig, ApiQlListMode, ApiQlPageDto};
use vw_api_types::id::TenantScope;

use super::params::normalize_apiql_query;
use super::runtime::run_apiql_query;
use super::schema::ApiQlEntityRegistry;

// Compatibility notes for unsupported PHP dynamics:
// - Entity metadata must be registered explicitly; PHP reflection is not available.
// - PHP api transformers are supported through explicitly registered ApiQL entities.
// - Relations are never silently dropped; unregistered relation names return a clear error.
// - Non-SQLite product databases require the corresponding cargo feature and connection config.

fn sqlite_connection(path: &str) -> ApiQlConnection {
    ApiQlConnection {
        id: "apiql-compat-main".to_string(),
        name: "ApiQL compat SQLite".to_string(),
        kind: ApiQlConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        connection_url: None,
        sqlite_path: Some(path.to_string()),
        updated_at_ms: 0,
    }
}

fn field(name: &str, column: &str, searchable: bool, sortable: bool) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        searchable,
        sortable,
        virtual_field: false,
        ..ApiQlFieldConfig::default()
    }
}

fn virtual_field(name: &str, dependencies: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(dependencies.to_string()),
        searchable: false,
        sortable: false,
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    }
}

fn product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "apiql-compat-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            field("id", "id", false, true),
            field("name", "name", true, true),
            field("sku", "sku", true, true),
            field("active", "active", false, false),
            field("created_at", "created_at", false, true),
        ],
        search_fields: vec!["name".to_string(), "sku".to_string()],
        virtual_fields: vec![virtual_field("search_key", "name,sku")],
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn make_sqlite_fixture(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("open SQLite fixture");
    conn.execute_batch(
        r#"
        CREATE TABLE product (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            sku TEXT NOT NULL,
            active INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        INSERT INTO product (id, name, sku, active, created_at) VALUES
            (1, 'apple', 'A-1', 1, '2026-01-01T00:00:00Z'),
            (2, 'banana', 'B-2', 1, '2026-01-02T00:00:00Z'),
            (3, 'pear', 'P-3', 1, '2026-01-03T00:00:00Z'),
            (4, 'orange', 'O-4', 0, '2026-01-04T00:00:00Z');
        "#,
    )
    .expect("seed SQLite fixture");
}

async fn run_fixture_query(
    connection: ApiQlConnection,
    registry: &ApiQlEntityRegistry,
    raw_query: &str,
    mode: ApiQlListMode,
) -> super::runtime::ApiQlRuntimeResponse {
    let params = normalize_apiql_query(raw_query, mode).expect("ApiQL query should parse");
    run_apiql_query(
        TenantScope::default(),
        connection,
        registry,
        registry.resolve("product:product").expect("registered product entity"),
        &params,
    )
    .await
    .expect("ApiQL query should run")
}

// 字段别名、数据库值与分页参数由同一组夹具固定，以同时约束兼容转换和查询语义。
#[tokio::test]
async fn apiql_compat_matches_php_index_modes_on_fixture_data() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("apiql-compat.db");
    make_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().expect("fixture path should be UTF-8"));
    let registry =
        ApiQlEntityRegistry::from_configs(vec![product_config()]).expect("valid entity config");

    let index = run_fixture_query(
        connection.clone(),
        &registry,
        "column[]=id&column[]=name&active=0&where[active]=1&where[search_key]=app%7Cban%20-pear&orderBy=id%20asc&limit=5&expectPage=1",
        ApiQlListMode::Index,
    )
    .await;
    assert_eq!(
        index.response.data,
        json!([{ "id": 1, "name": "apple" }, { "id": 2, "name": "banana" }])
    );
    assert_eq!(
        index.response.page,
        Some(ApiQlPageDto {
            per_page: 5,
            current_page: 1,
            total_page: Some(1),
            total_record: Some(2),
            total_macro: 2,
            from: 1,
            to: Some(2),
        })
    );

    let index_only = run_fixture_query(
        connection.clone(),
        &registry,
        "column[]=id&where[active]=1&orderBy=id%20desc&limit=2",
        ApiQlListMode::IndexOnly,
    )
    .await;
    assert_eq!(index_only.response.data, json!([{ "id": 3 }, { "id": 2 }]));
    assert_eq!(index_only.response.page, None);

    let index_count =
        run_fixture_query(connection.clone(), &registry, "active=1", ApiQlListMode::IndexCount)
            .await;
    assert_eq!(index_count.response.data, json!(3));
    assert_eq!(index_count.response.page, None);

    let index_page = run_fixture_query(
        connection,
        &registry,
        "column[]=id&page=2&page_size=2&orderBy=id%20asc",
        ApiQlListMode::IndexPage,
    )
    .await;
    assert_eq!(index_page.response.data, json!([{ "id": 3 }, { "id": 4 }]));
    assert_eq!(
        index_page.response.page,
        Some(ApiQlPageDto {
            per_page: 2,
            current_page: 2,
            total_page: None,
            total_record: None,
            total_macro: 0,
            from: 3,
            to: Some(4),
        })
    );

    let top_level = serde_json::to_value(index_page.response).expect("serialize response");
    assert!(top_level.get("data").is_some());
    assert!(top_level.get("items").is_none());
}

#[tokio::test]
async fn apiql_compat_reports_unsupported_relation_without_silent_drop() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("apiql-compat-relation.db");
    make_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().expect("fixture path should be UTF-8"));
    let registry =
        ApiQlEntityRegistry::from_configs(vec![product_config()]).expect("valid entity config");
    let params =
        normalize_apiql_query("column[]=id&relation[category][column][]=id", ApiQlListMode::Index)
            .expect("relation query should parse");

    let error = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").expect("registered product entity"),
        &params,
    )
    .await
    .expect_err("unknown relation should fail");

    assert_eq!(error.to_string(), "Unknown ApiQL relation: category");
}
use super::connections::{ApiQlConnection, ApiQlConnectionKind};
