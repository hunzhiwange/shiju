// 覆盖 ApiQL HTTP 处理器的请求校验、租户作用域与响应映射。

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tower::ServiceExt;
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};

use crate::app::agent::gateway::router::build_router;
use crate::gateway::api::handlers::apiql::connections::{
    APIQL_CONNECTION_STORAGE_TEST_LOCK as APIQL_HANDLER_STORAGE_LOCK, ApiQlConnection,
    ApiQlConnectionKind, ApiQlConnectionStore, local_connection_store,
};
use crate::gateway::api::handlers::apiql::schema::local_entity_config_store;
use crate::global_database::GlobalDatabase;
use crate::security::SecretStore;
use vw_api_types::id::TenantScope;

fn apiql_tenant() -> TenantScope {
    TenantScope::new("platform-test", "company-test").expect("ApiQL test tenant")
}

struct ApiQlStorageSnapshot {
    product_entity: Option<ApiQlEntityConfig>,
    connections: Option<Vec<ApiQlConnection>>,
}

impl ApiQlStorageSnapshot {
    async fn capture() -> Self {
        let config_store = local_connection_store(apiql_tenant()).expect("ApiQL config store");
        let entity_store =
            local_entity_config_store(apiql_tenant()).expect("ApiQL entity config store");
        Self {
            product_entity: entity_store
                .load_configs()
                .expect("capture ApiQL entity configs")
                .into_iter()
                .find(|config| config.entity_class == "product:product"),
            connections: Some(config_store.load_connections().expect("capture ApiQL connections")),
        }
    }

    async fn restore(self) {
        let entity_store =
            local_entity_config_store(apiql_tenant()).expect("ApiQL entity config store");
        match self.product_entity {
            Some(config) => {
                entity_store.save_configs(&[config]).expect("restore ApiQL entity config")
            }
            None => {
                entity_store.delete_config("product:product").expect("remove ApiQL entity config")
            }
        }
        let config_store = local_connection_store(apiql_tenant()).expect("ApiQL config store");
        config_store
            .save_connections(self.connections.as_deref().unwrap_or_default())
            .expect("restore ApiQL connections");
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

fn product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "apiql-route-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            field("id", "id", false, true),
            field("name", "name", true, true),
            field("sku", "sku", true, true),
            field("active", "active", false, false),
            field("created_at", "created_at", false, true),
        ],
        search_fields: vec!["name".to_string(), "sku".to_string()],
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn sqlite_connection(path: &str) -> ApiQlConnection {
    ApiQlConnection {
        id: "apiql-route-main".to_string(),
        name: "ApiQL route SQLite".to_string(),
        kind: ApiQlConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        connection_url: None,
        sqlite_path: Some(path.to_string()),
        updated_at_ms: 0,
    }
}

fn make_sqlite_fixture(path: &std::path::Path) {
    let conn = Connection::open(path).expect("open sqlite fixture");
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
            (3, 'pear', 'P-3', 1, '2026-01-03T00:00:00Z');
        "#,
    )
    .expect("seed sqlite fixture");
}

// 字段别名、数据库值与分页参数由同一组夹具固定，以同时约束兼容转换和查询语义。
#[tokio::test]
async fn apiql_loads_connections_from_gateway_settings() {
    let home = tempfile::tempdir().expect("temp home");
    let database =
        GlobalDatabase::new(home.path().join("database.sqlite3")).expect("global database");
    let store = ApiQlConnectionStore::new(
        database,
        SecretStore::new(&home.path().join("secrets"), true),
        TenantScope::default(),
    )
    .expect("ApiQL config store");
    let database_connection = sqlite_connection("/tmp/database.sqlite");
    store
        .save_connections(std::slice::from_ref(&database_connection))
        .expect("database connection should save");
    let legacy_dir =
        vw_config_types::paths::home_config_dir(home.path()).join("storage").join("apiql");
    std::fs::create_dir_all(&legacy_dir).expect("legacy ApiQL dir");
    std::fs::write(
        legacy_dir.join("connections.json"),
        serde_json::to_vec(&vec![sqlite_connection("/tmp/legacy.sqlite")])
            .expect("legacy connection JSON"),
    )
    .expect("legacy connection input");

    let loaded = super::handlers::load_connections_from_store(&store)
        .expect("ApiQL should load the ApiQL store");

    assert_eq!(loaded, vec![database_connection]);
    assert!(legacy_dir.join("connections.json").exists());
}

#[tokio::test]
async fn apiql_load_connection_reports_unknown_connection_id() {
    let tenant = TenantScope::new("apiql-missing-platform", "apiql-missing-company")
        .expect("missing connection tenant");

    let error = super::handlers::load_connection(tenant, "definitely-missing-connection")
        .await
        .expect_err("unknown connection should fail");

    assert_eq!(error.to_string(), "ApiQL connection not found: definitely-missing-connection");
}

async fn install_apiql_fixture(db_path: &std::path::Path) {
    local_entity_config_store(apiql_tenant())
        .expect("ApiQL entity config store")
        .save_configs(&[product_config()])
        .expect("write ApiQL entity config");
    local_connection_store(apiql_tenant())
        .expect("ApiQL config store")
        .save_connections(&[sqlite_connection(
            db_path.to_str().expect("fixture path should be UTF-8"),
        )])
        .expect("write ApiQL connection");
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let response = build_router(Vec::new())
        .oneshot(
            Request::get(uri)
                .header(vw_api_types::id::PLATFORM_ID_HEADER, "platform-test")
                .header(vw_api_types::id::COMPANY_ID_HEADER, "company-test")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("route request");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    if body.is_empty() {
        return (status, Value::Null);
    }
    let json = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, json)
}

#[tokio::test]
async fn apiql_routes_dispatch_four_php_compatible_list_modes() {
    let _guard = APIQL_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = ApiQlStorageSnapshot::capture().await;
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("apiql-routes.db");
    make_sqlite_fixture(&db_path);
    install_apiql_fixture(&db_path).await;

    let index = get_json(
        "/apiQL/product:product/?column%5B%5D=id&column%5B%5D=name&orderBy=id%20asc&limit=2",
    )
    .await;
    let list_only =
        get_json("/apiQL/product:product/list-only/?column%5B%5D=id&orderBy=id%20asc&limit=1")
            .await;
    let list_count = get_json("/apiQL/product:product/list-count/?active=1").await;
    let list_page = get_json(
        "/apiQL/product:product/list-page/?column%5B%5D=id&page=2&page_size=2&orderBy=id%20asc",
    )
    .await;
    let bad_column = get_json("/apiQL/product:product/?column%5B%5D=missing").await;
    let direct_index = super::functions::api_ql(
        apiql_tenant(),
        "product:product",
        "column%5B%5D=id&orderBy=id%20asc&limit=1",
    )
    .await
    .expect("direct api_ql should run");
    let direct_list_count =
        super::functions::api_ql_list_count(apiql_tenant(), "product:product", "active=1")
            .await
            .expect("direct api_ql_list_count should run");
    let direct_list_only = super::functions::api_ql_list_only(
        apiql_tenant(),
        "product:product",
        "column%5B%5D=id&orderBy=id%20asc&limit=1",
    )
    .await
    .expect("direct api_ql_list_only should run");
    let direct_list_page = super::functions::api_ql_list_page(
        apiql_tenant(),
        "product:product",
        "column%5B%5D=id&page=2&page_size=2&orderBy=id%20asc",
    )
    .await
    .expect("direct api_ql_list_page should run");

    snapshot.restore().await;

    assert_eq!(index.0, StatusCode::OK, "index route");
    assert_eq!(
        index.1["data"],
        json!([{ "id": 1, "name": "apple" }, { "id": 2, "name": "banana" }])
    );
    assert_eq!(index.1.get("page"), None);
    let index_next_cursor: Value =
        serde_json::from_str(index.1["next_cursor"].as_str().expect("next cursor string"))
            .expect("next cursor should be JSON");
    assert_eq!(index_next_cursor, json!({ "id": 2 }));

    assert_eq!(list_only.0, StatusCode::OK, "list-only route");
    assert_eq!(list_only.1["data"], json!([{ "id": 1 }]));
    assert_eq!(list_only.1.get("page"), None);

    assert_eq!(list_count.0, StatusCode::OK, "list-count route");
    assert_eq!(list_count.1["data"], json!(3));
    assert_eq!(list_count.1.get("page"), None);

    assert_eq!(list_page.0, StatusCode::OK, "list-page route");
    assert_eq!(list_page.1["data"], json!([{ "id": 3 }]));
    assert_eq!(
        list_page.1["page"],
        json!({
            "per_page": 2,
            "current_page": 2,
            "total_page": null,
            "total_record": null,
            "total_macro": 0,
            "from": 3,
            "to": 3
        })
    );

    assert_eq!(bad_column.0, StatusCode::BAD_REQUEST, "bad column route");
    assert!(!bad_column.1.to_string().contains("super-secret-token"));
    assert_eq!(direct_index.response.data, json!([{ "id": 1 }]));
    assert_eq!(direct_index.response.page, None);
    assert_eq!(direct_list_count.response.data, json!(3));
    assert_eq!(direct_list_only.response.data, json!([{ "id": 1 }]));
    assert_eq!(direct_list_only.response.page, None);
    assert_eq!(direct_list_page.response.data, json!([{ "id": 3 }]));
    assert_eq!(direct_list_page.response.page.unwrap().total_record, None);
}

// 非法字段、运算符和解析器配置必须在生成查询或执行脚本前被拒绝。
#[tokio::test]
async fn apiql_routes_accept_missing_trailing_slash() {
    let _guard = APIQL_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = ApiQlStorageSnapshot::capture().await;
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("apiql-routes-no-slash.db");
    make_sqlite_fixture(&db_path);
    install_apiql_fixture(&db_path).await;

    let index = get_json("/apiQL/product:product?column%5B%5D=id&orderBy=id%20asc&limit=1").await;
    let list_only =
        get_json("/apiQL/product:product/list-only?column%5B%5D=id&orderBy=id%20asc&limit=1").await;
    let list_count = get_json("/apiQL/product:product/list-count?active=1").await;
    let list_page = get_json(
        "/apiQL/product:product/list-page?column%5B%5D=id&page=1&page_size=1&orderBy=id%20asc",
    )
    .await;

    snapshot.restore().await;

    assert_eq!(index.0, StatusCode::OK, "index route without trailing slash");
    assert_eq!(index.1["data"], json!([{ "id": 1 }]));
    assert_eq!(list_only.0, StatusCode::OK, "list-only route without trailing slash");
    assert_eq!(list_count.0, StatusCode::OK, "list-count route without trailing slash");
    assert_eq!(list_page.0, StatusCode::OK, "list-page route without trailing slash");
}

#[tokio::test]
async fn apiql_runtime_uses_the_requested_tenant_for_same_entity_and_connection_ids() {
    let _guard = APIQL_HANDLER_STORAGE_LOCK.lock().await;
    let snapshot = ApiQlStorageSnapshot::capture().await;
    let second_tenant =
        TenantScope::new("apiql-platform-b", "apiql-company-b").expect("second tenant");
    let dir = tempfile::tempdir().expect("create temp dir");
    let first_path = dir.path().join("apiql-tenant-a.db");
    let second_path = dir.path().join("apiql-tenant-b.db");
    make_sqlite_fixture(&first_path);
    make_sqlite_fixture(&second_path);
    Connection::open(&second_path)
        .expect("open second fixture")
        .execute("UPDATE product SET name = 'tenant-b' WHERE id = 1", [])
        .expect("customize second fixture");
    install_apiql_fixture(&first_path).await;
    let second_entity_store =
        local_entity_config_store(second_tenant.clone()).expect("second entity store");
    second_entity_store.save_configs(&[product_config()]).expect("save second entity");
    let second_connection_store =
        local_connection_store(second_tenant.clone()).expect("second connection store");
    second_connection_store
        .save_connections(&[sqlite_connection(second_path.to_str().expect("second fixture UTF-8"))])
        .expect("save second connection");

    let first = super::functions::api_ql(
        apiql_tenant(),
        "product:product",
        "column%5B%5D=id&column%5B%5D=name&orderBy=id%20asc&limit=1",
    )
    .await
    .expect("first tenant query");
    let second = super::functions::api_ql(
        second_tenant,
        "product:product",
        "column%5B%5D=id&column%5B%5D=name&orderBy=id%20asc&limit=1",
    )
    .await
    .expect("second tenant query");

    assert_eq!(first.response.data, json!([{ "id": 1, "name": "apple" }]));
    assert_eq!(second.response.data, json!([{ "id": 1, "name": "tenant-b" }]));
    second_entity_store.delete_config("product:product").expect("clear second entity");
    second_connection_store.save_connections(&[]).expect("clear second connection");
    snapshot.restore().await;
}
