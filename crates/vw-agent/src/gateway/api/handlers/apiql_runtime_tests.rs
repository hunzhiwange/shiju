use std::collections::BTreeMap;

use rusqlite::params;
use serde_json::{Value, json};
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig, ApiQlListMode, ApiQlPageDto};
use vw_api_types::id::TenantScope;

use crate::global_database::GlobalDatabase;
use vw_api_types::data::{AiDataConnectionDto, AiDataConnectionKind, AiDataCountMode};

use super::params::normalize_apiql_query;
use super::runtime;
use super::runtime::{run_apiql_query, run_apiql_query_base};
use super::schema::{ApiQlEntityConfigStore, ApiQlEntityRegistry};
use super::sql::ApiQlSqlQuery;

fn sqlite_connection(path: &str) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: "product-main".to_string(),
        name: "Product SQLite".to_string(),
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
        connection_id: "product-main".to_string(),
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

fn meta_field(name: &str, column: &str, meta_key: &str, meta_value: &str) -> ApiQlFieldConfig {
    let mut meta = BTreeMap::new();
    meta.insert(meta_key.to_string(), json!(meta_value));
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        searchable: false,
        sortable: true,
        virtual_field: false,
        meta,
        ..ApiQlFieldConfig::default()
    }
}

fn status_field() -> ApiQlFieldConfig {
    let mut enum_values = BTreeMap::new();
    enum_values.insert("0".to_string(), json!("正常"));
    enum_values.insert("1".to_string(), json!("停采"));
    enum_values.insert("2".to_string(), json!("淘汰"));
    enum_values.insert("3".to_string(), json!("停售"));
    let mut meta = BTreeMap::new();
    meta.insert("enum".to_string(), json!(enum_values));
    ApiQlFieldConfig {
        name: "status".to_string(),
        column: Some("status".to_string()),
        label: Some("商品状态".to_string()),
        comment: Some("0=正常;1=停采;2=淘汰;3=停售;".to_string()),
        enum_class: Some("ProductStatusEnum".to_string()),
        searchable: false,
        sortable: true,
        virtual_field: false,
        meta,
        ..ApiQlFieldConfig::default()
    }
}

fn decimal_field() -> ApiQlFieldConfig {
    let mut column_struct = BTreeMap::new();
    column_struct.insert("format".to_string(), json!("format_decimal"));
    ApiQlFieldConfig {
        name: "smallSalesPrice".to_string(),
        column: Some("small_sales_price".to_string()),
        label: Some("小单位销售价格".to_string()),
        comment: Some("又名配送价".to_string()),
        searchable: false,
        sortable: true,
        virtual_field: false,
        column_struct,
        ..ApiQlFieldConfig::default()
    }
}

fn image_field() -> ApiQlFieldConfig {
    let mut column_struct = BTreeMap::new();
    column_struct.insert("format".to_string(), json!("format_images"));
    let mut meta = BTreeMap::new();
    meta.insert("attachments_url".to_string(), json!("http://127.0.0.1:9528/attachments"));
    ApiQlFieldConfig {
        name: "images".to_string(),
        column: Some("images".to_string()),
        label: Some("商品主图".to_string()),
        searchable: false,
        sortable: true,
        virtual_field: false,
        column_struct,
        meta,
        ..ApiQlFieldConfig::default()
    }
}

fn labeled_field(name: &str, column: &str, label: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        label: Some(label.to_string()),
        searchable: false,
        sortable: true,
        virtual_field: false,
        ..ApiQlFieldConfig::default()
    }
}

fn camel_product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            field("id", "id", false, true),
            meta_field("spuNo", "spu_no", "comment", "商品编码"),
            meta_field("skuNo", "sku_no", "label", "商品条码"),
            image_field(),
            status_field(),
            decimal_field(),
        ],
        search_fields: Vec::new(),
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn transformer_product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            labeled_field("id", "id", "ID"),
            labeled_field("skuTitle", "sku_title", "规格标题"),
            labeled_field("brandId", "brand_id", "商品品牌"),
            labeled_field("collaboratorId", "collaborator_id", "联营商"),
        ],
        search_fields: Vec::new(),
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn product_brand_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product_brand".to_string(),
        connection_id: "product-main".to_string(),
        table: "product_brand".to_string(),
        fields: vec![labeled_field("id", "id", "ID"), labeled_field("name", "name", "品牌名字")],
        search_fields: Vec::new(),
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn collaborator_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "user:collaborator".to_string(),
        connection_id: "product-main".to_string(),
        table: "collaborator".to_string(),
        fields: vec![
            labeled_field("id", "id", "ID"),
            labeled_field("name", "name", "联营商名称"),
            labeled_field("num", "num", "联营商编号"),
        ],
        search_fields: Vec::new(),
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn virtual_product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            labeled_field("id", "id", "ID"),
            labeled_field("title", "title", "商品标题"),
            labeled_field("specTitle", "spec_title", "规格组标题"),
        ],
        search_fields: Vec::new(),
        virtual_fields: vec![
            ApiQlFieldConfig {
                name: "skuTitle".to_string(),
                column: Some("specTitle".to_string()),
                label: Some("规格标题".to_string()),
                searchable: false,
                sortable: false,
                virtual_field: true,
                ..ApiQlFieldConfig::default()
            },
            ApiQlFieldConfig {
                name: "fullTitle".to_string(),
                column: Some("title,specTitle".to_string()),
                label: Some("商品完整标题".to_string()),
                searchable: false,
                sortable: false,
                virtual_field: true,
                ..ApiQlFieldConfig::default()
            },
        ],
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn rhai_virtual_product_config() -> ApiQlEntityConfig {
    let mut resolver = BTreeMap::new();
    resolver.insert("engine".to_string(), json!("rhai"));
    resolver.insert("function".to_string(), json!("sku_title"));
    resolver.insert(
        "script".to_string(),
        json!(
            r#"
            fn sku_title(row, ctx) {
                if row["spec_title"] == "" {
                    return "";
                }
                prepare_sku_title(row["spec_title"])
            }
        "#
        ),
    );
    let mut meta = BTreeMap::new();
    meta.insert("resolver".to_string(), json!(resolver));
    ApiQlEntityConfig {
        entity_class: "demo:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            labeled_field("id", "id", "ID"),
            labeled_field("specTitle", "spec_title", "规格组标题"),
        ],
        search_fields: Vec::new(),
        virtual_fields: vec![ApiQlFieldConfig {
            name: "skuTitle".to_string(),
            column: Some("specTitle".to_string()),
            label: Some("规格标题".to_string()),
            searchable: false,
            sortable: false,
            virtual_field: true,
            meta,
            ..ApiQlFieldConfig::default()
        }],
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn rhai_product_config(script_path: &str) -> ApiQlEntityConfig {
    let mut config = camel_product_config();
    config.extra.insert(
        "formatters".to_string(),
        json!({
            "format_decimal": {
                "engine": "rhai",
                "file": script_path,
                "function": "format_decimal"
            },
            "format_images": {
                "engine": "rhai",
                "file": script_path,
                "function": "format_images"
            }
        }),
    );
    config
}

fn make_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
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
    .unwrap();
}

fn make_camel_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE product (
            id INTEGER PRIMARY KEY,
            spu_no TEXT NOT NULL,
            sku_no TEXT NOT NULL,
            images TEXT NOT NULL,
            status INTEGER NOT NULL,
            small_sales_price TEXT NOT NULL
        );
        INSERT INTO product (id, spu_no, sku_no, images, status, small_sales_price) VALUES
            (1, 'SPU-1', 'SKU-1', '20250426/22/product/main.png', 0, '4.00000000'),
            (2, 'SPU-2', 'SKU-2', '', 1, '5.12000000');
        "#,
    )
    .unwrap();
}

fn make_transformer_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE product (
            id INTEGER PRIMARY KEY,
            sku_title TEXT NOT NULL,
            brand_id INTEGER NOT NULL,
            collaborator_id INTEGER NOT NULL
        );
        CREATE TABLE product_brand (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        CREATE TABLE collaborator (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            num TEXT NOT NULL
        );
        INSERT INTO product (id, sku_title, brand_id, collaborator_id) VALUES
            (1, '', 10, 20);
        INSERT INTO product_brand (id, name) VALUES
            (10, '喜之郎');
        INSERT INTO collaborator (id, name, num) VALUES
            (20, '成都天地公司', 'cdtdgs');
        "#,
    )
    .unwrap();
}

fn make_virtual_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE product (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            spec_title TEXT NOT NULL
        );
        INSERT INTO product (id, title, spec_title) VALUES
            (1, '测试商品', '{"1":"颜色:红色","2":"尺码:M","3":"罩杯:C"}');
        "#,
    )
    .unwrap();
}

fn empty_compiled_query() -> ApiQlSqlQuery {
    ApiQlSqlQuery {
        sql: String::new(),
        count_sql: None,
        params: BTreeMap::new(),
        page: 1,
        limit: 25,
        count_mode: AiDataCountMode::Disabled,
        order_by: None,
        cursor_fields: Vec::new(),
        virtual_fields: Vec::new(),
        hidden_fields: Vec::new(),
    }
}

#[test]
fn apiql_runtime_adapt_item_preserves_scalars_and_reports_unknown_virtual_field() {
    let registry = ApiQlEntityRegistry::from_configs(vec![virtual_product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params =
        normalize_apiql_query("fieldCase=camel&development=1", ApiQlListMode::Index).unwrap();
    let mut compiled = empty_compiled_query();

    let scalar = runtime::adapt_item(entity, json!("raw"), &params, &compiled)
        .expect("scalar item should pass through");
    assert_eq!(scalar, json!("raw"));

    compiled.virtual_fields = vec!["missingVirtual".to_string()];
    let error = runtime::adapt_item(entity, json!({ "id": 1 }), &params, &compiled)
        .expect_err("unknown virtual field should be rejected");

    assert_eq!(error.to_string(), "Unknown ApiQL virtual field: missingVirtual");
}

#[test]
fn apiql_runtime_helper_edges_keep_php_compat_names_and_enum_values() {
    let registry = ApiQlEntityRegistry::from_configs(vec![virtual_product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let virtual_field = entity
        .config()
        .virtual_fields
        .iter()
        .find(|field| field.name == "fullTitle")
        .expect("virtual field");

    assert_eq!(
        runtime::output_field_name(
            entity,
            "custom-field",
            super::params::ApiQlFieldNameCase::Snake
        ),
        "custom_field"
    );
    assert_eq!(
        runtime::output_field_name(
            entity,
            "custom_field",
            super::params::ApiQlFieldNameCase::Camel
        ),
        "customField"
    );
    assert_eq!(
        runtime::output_field_name_for_config(
            virtual_field,
            super::params::ApiQlFieldNameCase::Camel
        ),
        "fullTitle"
    );
    assert_eq!(runtime::enum_key_candidates(&json!("draft")), vec!["draft"]);
    assert_eq!(runtime::enum_key_candidates(&json!(true)), vec!["1", "true"]);
    assert_eq!(runtime::enum_key_candidates(&json!(false)), vec!["0", "false"]);
    assert!(runtime::enum_key_candidates(&json!([1])).is_empty());
    assert_eq!(runtime::enum_label_value(&json!({ "name": "Named" })), json!("Named"));
    assert_eq!(runtime::enum_label_value(&json!({ "title": "Titled" })), json!("Titled"));
    assert_eq!(runtime::enum_label_value(&json!({ "other": "Raw" })), json!({ "other": "Raw" }));
    assert_eq!(runtime::to_snake_case("sku-code"), "sku_code");
    assert_eq!(runtime::to_snake_case("Version2ID"), "version2_id");
    assert_eq!(runtime::to_snake_case("HTTPServer"), "http_server");
    assert_eq!(runtime::to_camel_case("alreadyCamel"), "alreadyCamel");
    assert_eq!(runtime::to_camel_case("sku-code value"), "skuCodeValue");
}

#[tokio::test]
async fn apiql_runtime_returns_php_page_for_each_list_mode() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime.db");
    make_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");

    let page_params = normalize_apiql_query(
        "column[]=id&column[]=name&page=2&page_size=2&orderBy=id%20asc",
        ApiQlListMode::IndexPage,
    )
    .expect("page params should parse");
    let page_response = run_apiql_query(
        TenantScope::default(),
        connection.clone(),
        &registry,
        registry.resolve("product:product").unwrap(),
        &page_params,
    )
    .await
    .expect("page query should run");

    assert_eq!(page_response.response.data, json!([{ "id": 3, "name": "pear" }]));
    assert_eq!(
        page_response.response.page,
        Some(ApiQlPageDto {
            per_page: 2,
            current_page: 2,
            total_page: None,
            total_record: None,
            total_macro: 0,
            from: 3,
            to: Some(3),
        })
    );

    let only_params =
        normalize_apiql_query("column[]=id&limit=2&orderBy=id%20asc", ApiQlListMode::IndexOnly)
            .expect("only params should parse");
    let only_response = run_apiql_query(
        TenantScope::default(),
        connection.clone(),
        &registry,
        registry.resolve("product:product").unwrap(),
        &only_params,
    )
    .await
    .expect("only query should run");

    assert_eq!(only_response.response.data, json!([{ "id": 1 }, { "id": 2 }]));
    assert_eq!(only_response.response.page, None);

    let count_params =
        normalize_apiql_query("active=1", ApiQlListMode::IndexCount).expect("count params");
    let count_response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &count_params,
    )
    .await
    .expect("count query should run");

    assert_eq!(count_response.response.data, json!(3));
    assert_eq!(count_response.response.page, None);
}

#[tokio::test]
async fn apiql_runtime_builds_next_cursor_from_order_fields() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-cursor.db");
    make_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=id&column[]=created_at&limit=2&orderBy=created_at%20asc,id%20asc",
        ApiQlListMode::Index,
    )
    .expect("cursor params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("cursor query should run");

    let next_cursor = response.next_cursor.expect("next cursor should be present");
    let cursor: Value = serde_json::from_str(&next_cursor).expect("cursor should be JSON");
    assert_eq!(cursor, json!({ "created_at": "2026-01-02T00:00:00Z", "id": 2 }));
}

#[tokio::test]
async fn apiql_runtime_defaults_to_snake_fields_and_dev_comments_from_meta() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-snake-comments.db");
    make_camel_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![camel_product_config()])
        .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=spuNo&column[]=skuNo&column[]=images&column[]=status&column[]=smallSalesPrice&orderBy=spuNo%20asc&limit=1&development=1",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection.clone(),
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "spu_no": "SPU-1",
                "#spu_no": "商品编码",
                "sku_no": "SKU-1",
                "#sku_no": "商品条码",
                "images": "20250426/22/product/main.png",
                "#images": "商品主图",
                "images_formated": "http://127.0.0.1:9528/attachments/20250426/22/product/main.png",
                "#images_formated": "商品主图「formated」",
                "status": 0,
                "#status": "商品状态 0=正常;1=停采;2=淘汰;3=停售;",
                "status_enum": "正常",
                "#status_enum": "商品状态 0=正常;1=停采;2=淘汰;3=停售;「enum」",
                "small_sales_price": "4.00000000",
                "#small_sales_price": "小单位销售价格 又名配送价",
                "small_sales_price_formated": 4.0,
                "#small_sales_price_formated": "小单位销售价格 又名配送价「formated」",
            }
        ])
    );
    assert_eq!(response.response.data_comment.as_deref(), Some("数据列表"));
    assert_eq!(response.response.next_cursor_comment.as_deref(), Some("流式分页下一组数据游标值"));

    let next_cursor = response.response.next_cursor.clone().expect("next cursor field");
    let cursor: Value = serde_json::from_str(&next_cursor).expect("cursor JSON");
    assert_eq!(cursor, json!({ "spu_no": "SPU-1" }));

    let encoded_cursor = urlencoding::encode(&next_cursor);
    let next_params = normalize_apiql_query(
        &format!("column[]=spuNo&orderBy=spuNo%20asc&limit=1&cursor={encoded_cursor}"),
        ApiQlListMode::Index,
    )
    .expect("cursor params should parse");
    let next_response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &next_params,
    )
    .await
    .expect("cursor query should run");

    assert_eq!(next_response.response.data, json!([{ "spu_no": "SPU-2" }]));
}

#[tokio::test]
async fn apiql_runtime_supports_camel_field_response_case() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-camel-case.db");
    make_camel_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![camel_product_config()])
        .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=spuNo&column[]=skuNo&column[]=smallSalesPrice&column[]=status&orderBy=spuNo%20asc&limit=1&fieldCase=camel",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "spuNo": "SPU-1",
                "skuNo": "SKU-1",
                "smallSalesPrice": "4.00000000",
                "smallSalesPriceFormated": 4.0,
                "status": 0,
                "statusEnum": "正常"
            }
        ])
    );
    let next_cursor = response.response.next_cursor.expect("next cursor field");
    let cursor: Value = serde_json::from_str(&next_cursor).expect("cursor JSON");
    assert_eq!(cursor, json!({ "spuNo": "SPU-1" }));
}

#[tokio::test]
async fn apiql_runtime_runs_registered_rhai_formatters() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-rhai-formatters.db");
    let script_path = dir.path().join("product_formatters.rhai");
    std::fs::write(
        &script_path,
        r#"
            fn format_decimal(value, ctx) {
                decimal(value, 8, true)
            }

            fn format_images(value, ctx) {
                image_url(value, ctx)
            }
        "#,
    )
    .unwrap();
    make_camel_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry =
        ApiQlEntityRegistry::from_configs(vec![rhai_product_config(script_path.to_str().unwrap())])
            .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=spuNo&column[]=images&column[]=smallSalesPrice&orderBy=spuNo%20asc&limit=1&development=1",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "spu_no": "SPU-1",
                "#spu_no": "商品编码",
                "images": "20250426/22/product/main.png",
                "#images": "商品主图",
                "images_formated": "http://127.0.0.1:9528/attachments/20250426/22/product/main.png",
                "#images_formated": "商品主图「formated」",
                "small_sales_price": "4.00000000",
                "#small_sales_price": "小单位销售价格 又名配送价",
                "small_sales_price_formated": 4.0,
                "#small_sales_price_formated": "小单位销售价格 又名配送价「formated」"
            }
        ])
    );
}

#[tokio::test]
async fn apiql_rhai_assets_remain_file_backed() {
    let dir = tempfile::tempdir().expect("temporary ApiQL Rhai directory");
    let database =
        GlobalDatabase::new(dir.path().join("database.sqlite3")).expect("global database");
    let tenant = TenantScope::default();
    let store = ApiQlEntityConfigStore::new(database.clone(), tenant.clone())
        .expect("ApiQL entity config store");
    let db_path = dir.path().join("apiql-rhai-file-backed.db");
    let script_path = dir.path().join("formatters.rhai");
    std::fs::write(
        &script_path,
        r#"
            fn format_decimal(value, ctx) { 999.0 }
            fn format_images(value, ctx) { value }
        "#,
    )
    .expect("initial Rhai script");
    let config = rhai_product_config(script_path.to_str().expect("UTF-8 script path"));
    store.save_configs(&[config]).expect("save ApiQL entity config");
    let stored_payload: String = database
        .connect()
        .expect("database connection")
        .query_row(
            "SELECT payload_json FROM gateway_settings \
             WHERE platform_id = ?1 AND company_id = ?2 \
               AND namespace = 'apiql.entities' AND config_key = 'product:product'",
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
            |row| row.get(0),
        )
        .expect("stored ApiQL entity payload");
    assert!(stored_payload.contains(script_path.to_str().expect("UTF-8 script path")));
    assert!(!stored_payload.contains("999.0"));
    std::fs::write(
        &script_path,
        r#"
            fn format_decimal(value, ctx) { decimal(value, 8, true) }
            fn format_images(value, ctx) { image_url(value, ctx) }
        "#,
    )
    .expect("updated Rhai script");
    make_camel_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().expect("UTF-8 database path"));
    let registry = ApiQlEntityRegistry::from_configs(
        store.load_configs().expect("load database entity config"),
    )
    .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=id&column[]=smallSalesPrice&orderBy=id%20asc&limit=1&development=1",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").expect("database entity"),
        &params,
    )
    .await
    .expect("file-backed Rhai formatter should run");

    assert_eq!(response.response.data[0]["small_sales_price_formated"], json!(4.0));
    assert!(script_path.exists());
}

#[tokio::test]
async fn apiql_runtime_applies_api_transformer_flat_fields() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-transformer.db");
    make_transformer_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![
        transformer_product_config(),
        product_brand_config(),
        collaborator_config(),
    ])
    .expect("entity config should be valid");
    let transformer = json!([
        {
            "code": "api",
            "type": "api",
            "args": {
                "product_brand": {
                    "api": "product:product_brand/list-only",
                    "source_key": "brand_id",
                    "target_key": "id",
                    "columns": ["name"],
                    "params": {},
                    "has_many": false
                },
                "collaborator": {
                    "api": "user:collaborator/list-only",
                    "source_key": "collaborator_id",
                    "target_key": "id",
                    "columns": ["name", "num"],
                    "params": {},
                    "has_many": false
                }
            }
        }
    ]);
    let transformer_json = transformer.to_string();
    let encoded_transformer = urlencoding::encode(&transformer_json);
    let params = normalize_apiql_query(
        &format!(
            "column[]=id&column[]=sku_title&orderBy=id%20asc&limit=1&development=1&transformer={encoded_transformer}"
        ),
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "brand_id": 10,
                "#brand_id": "商品品牌",
                "collaborator_id": 20,
                "#collaborator_id": "联营商",
                "id": 1,
                "#id": "ID",
                "sku_title": "",
                "#sku_title": "规格标题",
                "product_brand.id": 10,
                "product_brand.#id": "ID",
                "product_brand.name": "喜之郎",
                "product_brand.#name": "品牌名字",
                "collaborator.id": 20,
                "collaborator.#id": "ID",
                "collaborator.name": "成都天地公司",
                "collaborator.#name": "联营商名称",
                "collaborator.num": "cdtdgs",
                "collaborator.#num": "联营商编号"
            }
        ])
    );
}

#[tokio::test]
async fn apiql_runtime_resolves_virtual_fields_with_business_entity() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-virtuals.db");
    make_virtual_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![virtual_product_config()])
        .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=id&column[]=sku_title&column[]=full_title&orderBy=id%20asc&limit=1&development=1",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("product:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "id": 1,
                "#id": "ID",
                "sku_title": "红色 M C",
                "#sku_title": "规格标题",
                "full_title": "测试商品 红色 M C",
                "#full_title": "商品完整标题"
            }
        ])
    );
}

#[tokio::test]
async fn apiql_runtime_resolves_virtual_fields_with_rhai_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("apiql-runtime-rhai-virtuals.db");
    make_virtual_sqlite_fixture(&db_path);
    let connection = sqlite_connection(db_path.to_str().unwrap());
    let registry = ApiQlEntityRegistry::from_configs(vec![rhai_virtual_product_config()])
        .expect("entity config should be valid");
    let params = normalize_apiql_query(
        "column[]=id&column[]=sku_title&orderBy=id%20asc&limit=1&development=1",
        ApiQlListMode::Index,
    )
    .expect("params should parse");

    let response = run_apiql_query(
        TenantScope::default(),
        connection,
        &registry,
        registry.resolve("demo:product").unwrap(),
        &params,
    )
    .await
    .expect("query should run");

    assert_eq!(
        response.response.data,
        json!([
            {
                "id": 1,
                "#id": "ID",
                "sku_title": "红色 M C",
                "#sku_title": "规格标题"
            }
        ])
    );
}

#[tokio::test]
async fn apiql_runtime_rejects_mismatched_connection_before_query_execution() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let mut connection = sqlite_connection("unused.db");
    connection.id = "wrong-connection".to_string();
    let params = normalize_apiql_query("", ApiQlListMode::Index).expect("params should parse");

    let error = run_apiql_query_base(
        connection,
        &registry,
        registry.resolve("product:product").expect("registered entity"),
        &params,
    )
    .await
    .expect_err("mismatched connection must be rejected");

    assert_eq!(error.to_string(), "ApiQL entity requires connection_id: product-main");
}
