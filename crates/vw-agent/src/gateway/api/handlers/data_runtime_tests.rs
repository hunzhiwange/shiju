use super::*;
use axum::{
    Json as AxumJson, Router,
    extract::Query,
    routing::{get, post},
};
use serde_json::json;
use std::collections::BTreeMap;
use vw_api_types::data::{
    AiDataConnectionKind, AiDataCountMode, AiDataQueryKind, AiDataSourceMode,
};

fn connection(kind: AiDataConnectionKind) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: "conn-1".to_string(),
        name: "Primary".to_string(),
        kind,
        description: None,
        enabled: true,
        read_only: true,
        base_url: Some("https://api.example.test/root/".to_string()),
        connection_url: Some("postgres://example".to_string()),
        sqlite_path: None,
        default_path: Some("/default".to_string()),
        auth_token: Some("Bearer token".to_string()),
        headers: BTreeMap::from([("X-Test".to_string(), "yes".to_string())]),
        schema_hint: Some("items(id,name)".to_string()),
        updated_at_ms: 100,
        last_used_ms: None,
    }
}

fn sqlite_connection(path: &str) -> AiDataConnectionDto {
    AiDataConnectionDto {
        sqlite_path: Some(path.to_string()),
        base_url: None,
        connection_url: None,
        default_path: None,
        auth_token: None,
        headers: BTreeMap::new(),
        schema_hint: None,
        ..connection(AiDataConnectionKind::Sqlite)
    }
}

fn report() -> AiDataReportDto {
    AiDataReportDto {
        id: "report-1".to_string(),
        name: "Sales".to_string(),
        slug: "sales".to_string(),
        data_source: AiDataSourceMode::Normal,
        default_source_key: Some("main".to_string()),
        report_config: json!({
            "modules": [
                {
                    "type": "table",
                    "source": "main",
                    "template_code": "tpl",
                    "defaultSortFields": [
                        { "sortFieldId": "amount", "sortMethod": "desc" }
                    ],
                    "fields": [
                        { "code": "amount", "valueType": "sys_price" },
                        { "code": "ratio", "valueType": "float" }
                    ]
                },
                { "type": "chart", "show": false },
                { "type": "text", "show": "F" },
                { "type": "summary", "show": true }
            ],
            "alertMenu": { "show": true },
            "mock": {
                "goods_options": [
                    {
                        "company_id": "c1",
                        "goods_id": "g1",
                        "options_id": "1,2",
                        "options_name": "Red / XL"
                    },
                    { "options_id": "9", "options_name": "Fallback" }
                ]
            }
        }),
        sources: vec![AiDataReportSourceDto {
            source_key: "main".to_string(),
            connection_id: "conn-1".to_string(),
            query_kind: AiDataQueryKind::Sql,
            sql: Some("SELECT amount, ratio FROM sales".to_string()),
            count_sql: None,
            cube_query: None,
            http_method: "GET".to_string(),
            http_path: None,
            http_body: None,
            append_pagination: true,
        }],
        updated_at_ms: 200,
    }
}

fn base_resolved() -> ResolvedQuery {
    ResolvedQuery {
        connection: connection(AiDataConnectionKind::Sqlite),
        report_config: None,
        query_kind: AiDataQueryKind::Sql,
        sql: None,
        count_sql: None,
        cube_query: None,
        http_method: "GET".to_string(),
        http_path: None,
        http_body: None,
        params: BTreeMap::new(),
        page: 1,
        limit: 25,
        count_mode: AiDataCountMode::Disabled,
        order_by: None,
        search_fields: None,
        template_fields: None,
        transformer_fields: None,
        template_code: None,
        append_pagination: true,
        debug: false,
    }
}

fn make_sqlite_fixture(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE items (
            id INTEGER PRIMARY KEY,
            name TEXT,
            price REAL,
            active INTEGER,
            blob_value BLOB
        );
        INSERT INTO items (id, name, price, active, blob_value) VALUES
            (1, 'apple', 1.234, 1, x'0A0B'),
            (2, 'banana', 2.0, 0, x'0C'),
            (3, 'pear', 3.5, 1, x'0D');
        "#,
    )
    .unwrap();
}

async fn spawn_data_server() -> String {
    async fn items_get(Query(query): Query<BTreeMap<String, String>>) -> AxumJson<Value> {
        AxumJson(json!({
            "items": [
                { "method": "GET", "q": query.get("q").cloned().unwrap_or_default() }
            ],
            "total": 1
        }))
    }

    async fn items_post(AxumJson(body): AxumJson<Value>) -> AxumJson<Value> {
        AxumJson(json!({
            "data": [
                { "method": "POST", "body": body }
            ],
            "count": 1
        }))
    }

    async fn cube_meta() -> AxumJson<Value> {
        AxumJson(json!({ "cubes": [{ "name": "Orders" }] }))
    }

    async fn cube_load(AxumJson(body): AxumJson<Value>) -> AxumJson<Value> {
        AxumJson(json!({
            "data": [
                { "cube": "Orders", "query": body["query"].clone() }
            ],
            "total": 1
        }))
    }

    async fn text() -> &'static str {
        "plain text"
    }

    async fn fail() -> (axum::http::StatusCode, &'static str) {
        (axum::http::StatusCode::IM_A_TEAPOT, "short failure")
    }

    let app = Router::new()
        .route("/items", get(items_get).post(items_post))
        .route("/cubejs-api/v1/meta", get(cube_meta))
        .route("/cubejs-api/v1/load", post(cube_load))
        .route("/text", get(text))
        .route("/fail", get(fail));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn execute_query_sqlite_applies_templates_count_pagination_and_debug() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("items.db");
    make_sqlite_fixture(&db_path);
    let settings =
        AiDataSettings { default_limit: 2, default_timeout_secs: 5, ..AiDataSettings::default() };
    let connections = vec![sqlite_connection(db_path.to_str().unwrap())];
    let mut params = BTreeMap::new();
    params.insert("active".to_string(), json!(true));

    let response = execute_query(
        &settings,
        &connections,
        &[],
        AiDataQueryRequest {
            connection_id: Some("conn-1".to_string()),
            sql: Some(
                "SELECT id, name, price, blob_value FROM items WHERE active = {{ active }}"
                    .to_string(),
            ),
            params,
            page: Some(1),
            limit: Some(1),
            count: Some(AiDataCountMode::Enabled),
            order_by: Some("id DESC".to_string()),
            search_fields: Some(vec!["id".to_string(), "price".to_string()]),
            template_fields: Some(vec![AiDataTemplateFieldDto {
                code: "price".to_string(),
                value_type: FORMAT_SYS_PRICE,
            }]),
            debug: Some(true),
            ..AiDataQueryRequest::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page.total_record, 2);
    assert_eq!(response.page.total_page, 2);
    assert_eq!(response.page.from, 1);
    assert!(response.has_next_page);
    assert_eq!(response.items, vec![json!({ "id": 3, "price": 3.5 })]);
    assert_eq!(response.debug.as_ref().unwrap()["query_kind"], "sql");
}

#[tokio::test]
async fn sqlite_connection_test_and_catalog_cover_success_and_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("items.db");
    make_sqlite_fixture(&db_path);
    let sqlite = sqlite_connection(db_path.to_str().unwrap());

    let message = test_connection(&sqlite, 1).await.unwrap();
    assert!(message.contains("SQLite OK"));

    let catalog = connection_catalog(&sqlite, 1).await.unwrap();
    assert_eq!(catalog["tables"][0]["name"], "items");
    assert!(sqlite_path(&connection(AiDataConnectionKind::Sqlite)).is_err());
}

#[tokio::test]
async fn execute_query_rejects_invalid_resolution_and_unsupported_backends() {
    let settings = AiDataSettings::default();
    let disabled = AiDataConnectionDto { enabled: false, ..connection(AiDataConnectionKind::Http) };

    let err = execute_query(&settings, &[], &[], AiDataQueryRequest::default()).await.unwrap_err();
    assert!(err.contains("connection_id 或 report_id"));

    let err = execute_query(
        &settings,
        &[disabled],
        &[],
        AiDataQueryRequest { connection_id: Some("conn-1".to_string()), ..Default::default() },
    )
    .await
    .unwrap_err();
    assert!(err.contains("已被禁用"));

    let mysql = connection(AiDataConnectionKind::Mysql);
    assert!(test_connection(&mysql, 1).await.unwrap_err().contains("MySQL"));
    assert!(
        connection_catalog(&connection(AiDataConnectionKind::Postgres), 1)
            .await
            .unwrap_err()
            .contains("PostgreSQL")
    );
}

#[tokio::test]
async fn execute_query_covers_http_and_cube_network_paths() {
    let base_url = spawn_data_server().await;
    let settings =
        AiDataSettings { default_limit: 10, default_timeout_secs: 2, ..AiDataSettings::default() };

    let http = AiDataConnectionDto {
        base_url: Some(base_url.clone()),
        default_path: Some("/items".to_string()),
        ..connection(AiDataConnectionKind::Http)
    };
    let mut params = BTreeMap::new();
    params.insert("q".to_string(), json!("search"));
    let response = execute_query(
        &settings,
        &[http.clone()],
        &[],
        AiDataQueryRequest {
            connection_id: Some("conn-1".to_string()),
            params,
            debug: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(response.items, vec![json!({"method": "GET", "q": "search"})]);
    assert_eq!(response.debug.unwrap()["query_kind"], "http");

    let response = execute_query(
        &settings,
        &[http.clone()],
        &[],
        AiDataQueryRequest {
            connection_id: Some("conn-1".to_string()),
            http_method: Some("POST".to_string()),
            http_path: Some("/items".to_string()),
            http_body: Some(json!({ "tenant": "{{ tenant }}" })),
            params: BTreeMap::from([("tenant".to_string(), json!("t1"))]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(response.items[0]["method"], "POST");
    assert_eq!(response.items[0]["body"]["tenant"], "t1");

    assert!(
        execute_query(
            &settings,
            &[http.clone()],
            &[],
            AiDataQueryRequest {
                connection_id: Some("conn-1".to_string()),
                count: Some(AiDataCountMode::Only),
                ..Default::default()
            },
        )
        .await
        .unwrap_err()
        .contains("HTTP 查询暂不支持")
    );

    let cube = AiDataConnectionDto {
        id: "cube-1".to_string(),
        base_url: Some(base_url),
        default_path: None,
        ..connection(AiDataConnectionKind::Cube)
    };
    assert!(test_connection(&cube, 2).await.unwrap().contains("Cube OK"));
    assert_eq!(connection_catalog(&cube, 2).await.unwrap()["cubes"][0]["name"], "Orders");

    let response = execute_query(
        &settings,
        &[cube],
        &[],
        AiDataQueryRequest {
            connection_id: Some("cube-1".to_string()),
            cube_query: Some(json!({ "measures": ["Orders.count"] })),
            debug: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(response.items[0]["cube"], "Orders");
    assert_eq!(response.debug.unwrap()["query_kind"], "cube");
}

#[tokio::test]
async fn http_helpers_parse_raw_text_and_error_responses() {
    let base_url = spawn_data_server().await;
    let http = AiDataConnectionDto {
        base_url: Some(base_url.clone()),
        ..connection(AiDataConnectionKind::Http)
    };

    let (_, raw) =
        send_json_request(&http, reqwest::Method::GET, Some(&format!("{base_url}/text")), None, 2)
            .await
            .unwrap();
    assert_eq!(raw, json!({ "raw": "plain text" }));

    let err =
        send_json_request(&http, reqwest::Method::GET, Some(&format!("{base_url}/fail")), None, 2)
            .await
            .unwrap_err();
    assert!(err.contains("HTTP 418"));
    assert!(err.contains("short failure"));
    assert!(
        send_json_request(&http, reqwest::Method::GET, None, None, 2)
            .await
            .unwrap_err()
            .contains("请求 URL")
    );
}

#[test]
fn merge_search_condition_and_template_rendering_handle_edge_cases() {
    let mut params = BTreeMap::from([
        ("name".to_string(), json!("O'Reilly")),
        ("nested".to_string(), json!({ "id": 7 })),
        ("tags".to_string(), json!(["a", "b"])),
    ]);
    let merged =
        merge_search_condition(params.clone(), Some(json!(r#"{"name":"pear","extra":3}"#)))
            .unwrap();
    assert_eq!(merged["name"], "pear");
    assert_eq!(merged["extra"], 3);
    assert!(merge_search_condition(BTreeMap::new(), Some(json!([1]))).is_err());
    assert!(merge_search_condition(BTreeMap::new(), Some(json!("[1]"))).is_err());

    assert_eq!(
        render_sql_template("name={{name}} AND id={{nested.id}} AND missing={{none}}", &params)
            .unwrap(),
        "name='O''Reilly' AND id=7 AND missing=NULL"
    );
    assert_eq!(render_string_template("/items/{{nested.id}}/{{tags}}", &params), "/items/7/a,b");
    assert_eq!(render_json_template(&json!("{{ nested.id }}"), &params), json!(7));
    assert_eq!(
        render_json_template(&json!({ "q": "hello {{name}}" }), &params),
        json!({"q": "hello O'Reilly"})
    );

    params.insert("obj".to_string(), json!({"a": "b"}));
    assert_eq!(sql_literal(&params["obj"]), r#"'{\"a\":\"b\"}'"#.replace("\\\"", "\""));
}

#[test]
fn report_config_visibility_defaults_sorting_and_source_context_are_resolved() {
    let report = report();
    let prepared = prepared_report(&report);
    let modules = prepared.report_config["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 2);
    assert!(prepared.report_config["alertMenu"]["menu"].as_array().unwrap().is_empty());
    assert!(prepared.report_config["alertMenu"]["variable"].is_object());
    assert_eq!(
        report_default_order_by(Some(&prepared.report_config)).as_deref(),
        Some("amount DESC")
    );
    assert_eq!(
        report_default_template_code(Some(&prepared.report_config), Some("main")).as_deref(),
        Some("tpl")
    );

    assert_eq!(is_visible_flag(None), true);
    assert_eq!(is_visible_flag(Some(&json!("0"))), false);
    assert_eq!(is_visible_flag(Some(&json!(0))), false);
    assert_eq!(module_is_visible(&json!("not-object")), true);

    let (resolved_report, source) = report_source_context(&[report], Some("sales"), None).unwrap();
    assert_eq!(resolved_report.slug, "sales");
    assert_eq!(source.source_key, "main");
    assert!(report_source_context(&[], Some("missing"), None).is_none());
}

#[test]
fn sql_safety_order_by_pagination_and_endpoints_are_sanitized() {
    ensure_read_only_sql(" SELECT * FROM items").unwrap();
    ensure_read_only_sql("WITH x AS (SELECT 1) SELECT * FROM x").unwrap();
    assert!(ensure_read_only_sql("DELETE FROM items").is_err());
    assert!(ensure_read_only_sql("SELECT * FROM a; DROP TABLE a").is_err());
    assert_eq!(sanitize_order_by(Some("name desc, id")).unwrap().unwrap(), "name DESC, id ASC");
    assert!(sanitize_order_by(Some("name; drop")).is_err());

    assert_eq!(
        build_page(0, 1, 25, 0),
        AiDataPageDto {
            per_page: 25,
            current_page: 1,
            total_page: 0,
            total_record: 0,
            from: 0,
            to: 0,
        }
    );
    assert_eq!(build_page(51, 3, 25, 1).to, 51);

    let conn = connection(AiDataConnectionKind::Http);
    assert_eq!(
        http_endpoint(&conn, Some("/v1/items")).unwrap(),
        "https://api.example.test/root/v1/items"
    );
    assert_eq!(cube_endpoint(&conn, "meta").unwrap(), "https://api.example.test/root/default/meta");
    assert!(http_endpoint(&AiDataConnectionDto { base_url: None, ..conn }, None).is_err());
    assert_eq!(parse_http_method("head").unwrap(), reqwest::Method::HEAD);
    assert!(parse_http_method("put").is_err());
}

#[cfg(feature = "memory-mariadb")]
#[test]
fn mysql_text_protocol_bytes_follow_column_types() {
    use mysql::consts::{ColumnFlags, ColumnType};

    assert_eq!(
        mysql_bytes_to_json(b"42".to_vec(), ColumnType::MYSQL_TYPE_LONG, ColumnFlags::empty()),
        json!(42)
    );
    assert_eq!(
        mysql_bytes_to_json(
            b"42".to_vec(),
            ColumnType::MYSQL_TYPE_LONGLONG,
            ColumnFlags::UNSIGNED_FLAG,
        ),
        json!(42_u64)
    );
    assert_eq!(
        mysql_bytes_to_json(
            b"4.25000000".to_vec(),
            ColumnType::MYSQL_TYPE_NEWDECIMAL,
            ColumnFlags::empty(),
        ),
        json!(4.25)
    );
    assert_eq!(
        mysql_bytes_to_json(
            b"42".to_vec(),
            ColumnType::MYSQL_TYPE_VAR_STRING,
            ColumnFlags::empty()
        ),
        json!("42")
    );
    assert_eq!(
        mysql_bytes_to_json(
            br#"{"ok":true}"#.to_vec(),
            ColumnType::MYSQL_TYPE_JSON,
            ColumnFlags::empty()
        ),
        json!({"ok": true})
    );
    assert_eq!(
        mysql_bytes_to_json(vec![0b0000_0011], ColumnType::MYSQL_TYPE_BIT, ColumnFlags::empty()),
        json!(3_u64)
    );
}

#[test]
fn post_processing_formats_filters_and_transforms_items() {
    let mut resolved = base_resolved();
    resolved.report_config = Some(report().report_config);
    resolved.template_code = Some("tpl".to_string());
    resolved.search_fields = Some(vec![
        "amount".to_string(),
        "ratio".to_string(),
        "options_name".to_string(),
        "options_id".to_string(),
    ]);
    resolved.params.insert("company_id".to_string(), json!("c1"));
    resolved.params.insert("goods_id".to_string(), json!("g1"));
    resolved.transformer_fields = Some(vec![
        AiDataTransformerFieldDto {
            code: "ratio".to_string(),
            transformer_type: "percentage".to_string(),
            transformer_args: BTreeMap::from([("decimals".to_string(), json!(1))]),
        },
        AiDataTransformerFieldDto {
            code: "options_name".to_string(),
            transformer_type: "goods_options_name".to_string(),
            transformer_args: BTreeMap::from([("goods_field".to_string(), json!("goods_id"))]),
        },
    ]);
    let mut items = vec![json!({
        "amount": "12.345",
        "ratio": 0.125,
        "options_id": "[\"1\",\"2\"]",
        "options_name": "",
        "goods_id": "g1",
        "hidden": "remove"
    })];

    post_process_items(&mut items, &resolved).unwrap();

    assert_eq!(
        items,
        vec![json!({
            "amount": 12.35,
            "ratio": "12.5%",
            "options_id": "[\"1\",\"2\"]",
            "options_name": "Red / XL"
        })]
    );

    resolved.transformer_fields = Some(vec![AiDataTransformerFieldDto {
        code: "amount".to_string(),
        transformer_type: "unknown".to_string(),
        transformer_args: BTreeMap::new(),
    }]);
    assert!(post_process_items(&mut items, &resolved).unwrap_err().contains("暂不支持"));
}

#[test]
fn template_sources_value_formats_and_goods_fallbacks_are_supported() {
    let report_config = json!({
        "mockTemplateSource": {
            "templates": [
                {
                    "templateCode": "mock-tpl",
                    "fields": [
                        { "field": "CountValue", "sysFormat": "int" },
                        { "key": "name", "value_type": "string" }
                    ]
                }
            ]
        }
    });
    let formats = resolve_template_formats(Some(&report_config), None, Some("mock-tpl"));
    assert_eq!(formats["count_value"], FORMAT_INT);
    assert_eq!(formats["name"], FORMAT_STRING);
    assert_eq!(format_value_by_type(json!("2.9"), FORMAT_INT), json!(2));
    assert_eq!(format_value_by_type(json!(2), FORMAT_STRING), json!("2"));
    assert_eq!(normalize_field_code("goodsOptions Name"), "goods_options_name");
    assert_eq!(normalize_options_lookup_key(r#"["1","2"]"#), "1,2");
    assert_eq!(value_lookup_string(&json!(["a", 1, true])).unwrap(), "a,1,true");
    assert_eq!(
        format_goods_options_name_value(json!({"color":"red","size":""})),
        json!("color:red / size")
    );
}

#[test]
fn extract_items_total_and_body_merge_cover_response_shapes() {
    assert_eq!(extract_items_and_total(json!([{"id": 1}])).1, Some(1));
    assert_eq!(extract_items_and_total(json!({"data": [{"id": 1}], "total": 9})).1, Some(9));
    assert_eq!(extract_items_and_total(json!({"items": [{"id": 2}], "count": 3})).0[0]["id"], 2);
    assert_eq!(
        extract_items_and_total(json!({"list": [{"id": 3}], "page": {"total_record": 4}})).1,
        Some(4)
    );
    assert_eq!(extract_items_and_total(json!("raw")).0, vec![json!("raw")]);

    let params = BTreeMap::from([("page".to_string(), json!(1))]);
    assert_eq!(
        merge_json_object(Some(json!({"q": "x"})), &params),
        Some(json!({"q": "x", "page": 1}))
    );
    assert_eq!(merge_json_object(Some(json!([1])), &params), Some(json!([1])));
    assert_eq!(merge_json_object(None, &params), Some(json!({"page": 1})));
    assert_eq!(merge_json_object(None, &BTreeMap::new()), None);
    assert_eq!(truncate_text("你好abcdef", 4), "你好ab");
}

#[test]
fn resolve_query_merges_report_defaults_and_clamps_request_boundaries() {
    let settings =
        AiDataSettings { default_limit: 25, default_timeout_secs: 5, ..AiDataSettings::default() };
    let resolved = resolve_query(
        &settings,
        &[connection(AiDataConnectionKind::Sqlite)],
        &[report()],
        AiDataQueryRequest {
            report_id: Some("sales".to_string()),
            params: BTreeMap::from([
                ("shared".to_string(), json!("params")),
                ("stable".to_string(), json!(true)),
            ]),
            search_condition: Some(json!(r#"{"shared":"search","extra":3}"#)),
            page: Some(0),
            limit: Some(20_000),
            count: Some(AiDataCountMode::Only),
            ..AiDataQueryRequest::default()
        },
    )
    .unwrap();

    assert_eq!(resolved.connection.id, "conn-1");
    assert_eq!(resolved.query_kind, AiDataQueryKind::Sql);
    assert_eq!(resolved.sql.as_deref(), Some("SELECT amount, ratio FROM sales"));
    assert_eq!(resolved.page, 1);
    assert_eq!(resolved.limit, 10_000);
    assert_eq!(resolved.count_mode, AiDataCountMode::Only);
    assert_eq!(resolved.order_by.as_deref(), Some("amount DESC"));
    assert_eq!(resolved.template_code.as_deref(), Some("tpl"));
    assert!(resolved.append_pagination);
    assert_eq!(resolved.params["shared"], "search");
    assert_eq!(resolved.params["stable"], true);
    assert_eq!(resolved.params["extra"], 3);
}

#[test]
fn resolve_query_reports_missing_report_source_and_connection_context() {
    let settings = AiDataSettings::default();
    let connections = [connection(AiDataConnectionKind::Sqlite)];

    let error = resolve_query(
        &settings,
        &connections,
        &[],
        AiDataQueryRequest {
            report_id: Some("missing".to_string()),
            ..AiDataQueryRequest::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("报表不存在"));

    let mut no_sources = report();
    no_sources.default_source_key = None;
    no_sources.sources.clear();
    let error = resolve_query(
        &settings,
        &connections,
        &[no_sources],
        AiDataQueryRequest {
            report_id: Some("sales".to_string()),
            ..AiDataQueryRequest::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("未配置可用数据源"));

    let error = resolve_query(
        &settings,
        &connections,
        &[report()],
        AiDataQueryRequest {
            report_id: Some("sales".to_string()),
            source_key: Some("missing".to_string()),
            ..AiDataQueryRequest::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("数据源不存在"));

    let error = resolve_query(
        &settings,
        &[],
        &[report()],
        AiDataQueryRequest {
            report_id: Some("sales".to_string()),
            ..AiDataQueryRequest::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("连接不存在"));
}

#[test]
fn connection_kinds_map_to_their_safe_default_query_kind() {
    assert_eq!(connection_default_query_kind(&AiDataConnectionKind::Sqlite), AiDataQueryKind::Sql);
    assert_eq!(connection_default_query_kind(&AiDataConnectionKind::Mysql), AiDataQueryKind::Sql);
    assert_eq!(
        connection_default_query_kind(&AiDataConnectionKind::Postgres),
        AiDataQueryKind::Sql
    );
    assert_eq!(connection_default_query_kind(&AiDataConnectionKind::Cube), AiDataQueryKind::Cube);
    assert_eq!(connection_default_query_kind(&AiDataConnectionKind::Http), AiDataQueryKind::Http);
}

#[test]
fn report_config_visibility_handles_flag_types_and_non_object_inputs() {
    let scalar = json!(["unchanged"]);
    assert_eq!(prepare_report_config(&scalar), scalar);

    let prepared = prepare_report_config(&json!({
        "modules": [
            { "id": "bool-false", "show": false },
            { "id": "number-zero", "show": 0 },
            { "id": "empty", "show": "  " },
            { "id": "false", "show": " false " },
            { "id": "f", "show": "F" },
            { "id": "zero", "show": "0" },
            { "id": "bool-true", "show": true },
            { "id": "number-one", "show": 1 },
            { "id": "object", "show": {} },
            { "id": "missing" },
            "non-object"
        ],
        "alertMenu": { "show": "1" }
    }));
    let modules = prepared["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 5);
    assert_eq!(modules[0]["id"], "bool-true");
    assert!(prepared["alertMenu"]["menu"].as_array().unwrap().is_empty());
    assert!(prepared["alertMenu"]["variable"].is_object());
    assert!(is_visible_flag(Some(&Value::Null)));
    assert!(is_visible_flag(Some(&json!(0.5))));

    let hidden_alert = prepare_report_config(&json!({ "alertMenu": { "show": false } }));
    assert!(hidden_alert["alertMenu"].get("menu").is_none());
}

#[test]
fn report_defaults_and_source_context_follow_documented_precedence() {
    let config = json!({
        "modules": [
            { "type": "chart", "source": "main", "template_code": "source-chart" },
            { "type": "table", "source": "other", "template_code": "other-table" },
            { "type": "text", "template_code": "fallback" }
        ]
    });
    assert_eq!(
        report_default_template_code(Some(&config), Some("main")).as_deref(),
        Some("source-chart")
    );
    assert_eq!(
        report_default_template_code(Some(&config), Some("missing")).as_deref(),
        Some("other-table")
    );
    assert_eq!(report_default_order_by(Some(&config)), None);
    assert_eq!(report_default_order_by(None), None);

    let mut report = report();
    let mut secondary = report.sources[0].clone();
    secondary.source_key = "secondary".to_string();
    secondary.sql = Some("SELECT 2".to_string());
    report.sources.push(secondary);
    let (_, source) =
        report_source_context(&[report.clone()], Some("report-1"), Some("secondary")).unwrap();
    assert_eq!(source.sql.as_deref(), Some("SELECT 2"));
    assert!(report_source_context(&[report], Some("sales"), Some("missing")).is_none());
}

#[test]
fn template_rendering_preserves_json_types_and_literal_boundaries() {
    let params = BTreeMap::from([
        ("nested".to_string(), json!({ "id": 7 })),
        ("nested.id".to_string(), json!(9)),
        ("bool".to_string(), json!(false)),
        ("null".to_string(), Value::Null),
        ("values".to_string(), json!([1, "two", true])),
    ]);

    assert_eq!(lookup_param(&params, "nested.id"), Some(&json!(9)));
    assert_eq!(lookup_param(&params, "nested.missing"), None);
    assert_eq!(
        render_json_template(
            &json!(["{{ bool }}", "{{ missing }}", "id={{nested.id}}", 4]),
            &params
        ),
        json!([false, null, "id=9", 4])
    );
    assert_eq!(sql_literal(&params["null"]), "NULL");
    assert_eq!(sql_literal(&params["bool"]), "0");
    assert_eq!(sql_literal(&params["values"]), "1, 'two', 1");
    assert_eq!(string_literal(&params["null"]), "");
    assert_eq!(string_literal(&params["values"]), "1,two,true");
    assert_eq!(merge_search_condition(params.clone(), None).unwrap(), params);
    assert!(merge_search_condition(BTreeMap::new(), Some(json!("not-json"))).is_err());
}

#[test]
fn sql_guards_and_page_math_cover_empty_and_forbidden_boundaries() {
    assert_eq!(sanitize_order_by(None).unwrap(), None);
    assert_eq!(sanitize_order_by(Some("  ")).unwrap(), None);
    assert_eq!(
        sanitize_order_by(Some("schema.name ASC, total_desc dEsC")).unwrap().as_deref(),
        Some("schema.name ASC, total_desc DESC")
    );
    assert!(sanitize_order_by(Some("name-with-dash ASC")).is_err());

    for statement in [
        "SELECT 1 INSERT INTO audit VALUES (1)",
        "SELECT 1 UPDATE items SET id = 2",
        "SELECT 1 DELETE FROM items",
        "SELECT 1 DROP TABLE items",
        "SELECT 1 ALTER TABLE items ADD x INT",
        "SELECT 1 TRUNCATE TABLE items",
        "SELECT 1 ATTACH DATABASE 'x' AS x",
        "SELECT 1 DETACH DATABASE x",
        "SELECT 1 REPLACE INTO items VALUES (1)",
        "SELECT 1 PRAGMA schema_version",
        "SELECT 1 CREATE TABLE x(id INT)",
    ] {
        assert!(ensure_read_only_sql(statement).is_err(), "accepted {statement}");
    }

    assert_eq!(
        build_page(5, 2, 2, 2),
        AiDataPageDto {
            per_page: 2,
            current_page: 2,
            total_page: 3,
            total_record: 5,
            from: 3,
            to: 4,
        }
    );
    assert_eq!(build_page(1, 0, 25, 1).from, 1);
}

#[test]
fn sqlite_helpers_cover_row_values_count_shapes_and_empty_results() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("helpers.db");
    make_sqlite_fixture(&db_path);
    let path = db_path.to_str().unwrap();

    let rows = sqlite_query_rows(
        path,
        "SELECT id, name, price, active, blob_value, NULL AS missing FROM items WHERE id = 1",
    )
    .unwrap();
    assert_eq!(
        rows,
        vec![json!({
            "id": 1,
            "name": "apple",
            "price": 1.234,
            "active": 1,
            "blob_value": "0x0a0b",
            "missing": null
        })]
    );
    assert_eq!(sqlite_query_count(path, "SELECT COUNT(*) FROM items").unwrap(), 3);
    assert_eq!(sqlite_query_count(path, "SELECT '7'").unwrap(), 7);
    assert_eq!(sqlite_query_count(path, "SELECT -2").unwrap(), 0);
    assert_eq!(sqlite_query_count(path, "SELECT NULL").unwrap(), 0);
    assert_eq!(sqlite_query_count(path, "SELECT 1 WHERE 0").unwrap(), 0);
    assert!(sqlite_query_count(path, "SELECT 'invalid'").is_err());

    assert_eq!(sqlite_value_ref_to_json(ValueRef::Null), Value::Null);
    assert_eq!(sqlite_value_ref_to_json(ValueRef::Integer(-1)), json!(-1));
    assert_eq!(sqlite_value_ref_to_json(ValueRef::Real(1.5)), json!(1.5));
    assert_eq!(sqlite_value_ref_to_json(ValueRef::Text(b"text")), json!("text"));
    assert_eq!(sqlite_value_ref_to_json(ValueRef::Blob(&[0xff])), json!("0xff"));
}

#[tokio::test]
async fn read_only_sql_wrapper_normalizes_bounds_and_rejects_unsafe_connections() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("wrapper.db");
    make_sqlite_fixture(&db_path);
    let sqlite = sqlite_connection(db_path.to_str().unwrap());
    let response = execute_read_only_sql_query(
        sqlite,
        ReadOnlySqlQuery {
            sql: "SELECT id FROM items ORDER BY id".to_string(),
            count_sql: None,
            params: BTreeMap::new(),
            page: 0,
            limit: 0,
            count_mode: AiDataCountMode::Disabled,
            order_by: None,
            debug: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(response.page.current_page, 1);
    assert_eq!(response.page.per_page, 1);
    assert_eq!(response.items, vec![json!({ "id": 1 })]);
    assert!(response.has_next_page);
    assert!(
        response.debug.unwrap()["rendered_sql"].as_str().unwrap().ends_with("LIMIT 1 OFFSET 0")
    );

    let disabled =
        AiDataConnectionDto { enabled: false, ..connection(AiDataConnectionKind::Sqlite) };
    let error = execute_read_only_sql_query(
        disabled,
        ReadOnlySqlQuery {
            sql: "SELECT 1".to_string(),
            count_sql: None,
            params: BTreeMap::new(),
            page: 1,
            limit: 1,
            count_mode: AiDataCountMode::Disabled,
            order_by: None,
            debug: false,
        },
    )
    .await
    .unwrap_err();
    assert!(error.contains("已被禁用"));

    let error = execute_sql_rows(&connection(AiDataConnectionKind::Http), "SELECT 1".to_string())
        .await
        .unwrap_err();
    assert!(error.contains("不支持 SQL 查询"));
    let error =
        execute_sql_count(&connection(AiDataConnectionKind::Cube), "SELECT COUNT(1)".to_string())
            .await
            .unwrap_err();
    assert!(error.contains("不支持 SQL 计数查询"));
}

#[tokio::test]
async fn sql_runtime_covers_count_only_and_disabled_pagination_modes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("modes.db");
    make_sqlite_fixture(&db_path);

    let mut resolved = base_resolved();
    resolved.connection = sqlite_connection(db_path.to_str().unwrap());
    resolved.sql = Some("SELECT id FROM items".to_string());
    resolved.count_sql = Some("SELECT '3'".to_string());
    resolved.count_mode = AiDataCountMode::Only;
    resolved.limit = 2;
    resolved.debug = true;
    let response = execute_sql_query(&resolved).await.unwrap();
    assert!(response.items.is_empty());
    assert_eq!(response.page.total_record, 3);
    assert_eq!(response.page.total_page, 2);
    assert_eq!(response.page.from, 0);
    assert!(response.has_next_page);
    assert_eq!(response.debug.unwrap()["count_sql"], "SELECT '3'");

    resolved.sql = Some("SELECT id FROM items ORDER BY id DESC LIMIT 2".to_string());
    resolved.count_mode = AiDataCountMode::Disabled;
    resolved.append_pagination = false;
    resolved.limit = 1;
    resolved.order_by = Some("id ASC".to_string());
    let response = execute_sql_query(&resolved).await.unwrap();
    assert_eq!(response.items, vec![json!({ "id": 3 }), json!({ "id": 2 })]);
    assert_eq!(response.page.total_record, 2);
    assert!(response.has_next_page);
    assert_eq!(
        response.debug.unwrap()["rendered_sql"],
        "SELECT id FROM items ORDER BY id DESC LIMIT 2"
    );
}

#[tokio::test]
async fn remote_query_runtimes_reject_invalid_requests_before_network_access() {
    let mut cube = base_resolved();
    cube.connection = connection(AiDataConnectionKind::Cube);
    cube.query_kind = AiDataQueryKind::Cube;
    let error = execute_cube_query(&cube, 1).await.unwrap_err();
    assert!(error.contains("缺少 Cube query"));

    cube.count_mode = AiDataCountMode::Only;
    let error = execute_cube_query(&cube, 1).await.unwrap_err();
    assert!(error.contains("Cube 查询暂不支持 count_only"));

    let mut http = base_resolved();
    http.connection = connection(AiDataConnectionKind::Http);
    http.query_kind = AiDataQueryKind::Http;
    http.http_method = "DELETE".to_string();
    let error = execute_http_query(&http, 1).await.unwrap_err();
    assert!(error.contains("仅支持 GET/POST/HEAD"));
}

#[test]
fn template_format_resolution_prefers_explicit_fields_and_nested_sources() {
    let report_config = json!({
        "templates": {
            "tpl": {
                "fields": [
                    { "code": "TotalPrice", "valueType": "sys_price" },
                    { "field": "Quantity", "sysFormat": 4 },
                    { "key": "ignored", "value_type": "unknown" },
                    "invalid"
                ]
            }
        }
    });
    let formats = resolve_template_formats(Some(&report_config), None, Some("tpl"));
    assert_eq!(formats["total_price"], FORMAT_SYS_PRICE);
    assert_eq!(formats["quantity"], FORMAT_INT);
    assert!(!formats.contains_key("ignored"));

    let explicit =
        [AiDataTemplateFieldDto { code: "ExplicitValue".to_string(), value_type: FORMAT_FLOAT }];
    let formats = resolve_template_formats(Some(&report_config), Some(&explicit), Some("tpl"));
    assert_eq!(formats.len(), 1);
    assert_eq!(formats["explicit_value"], FORMAT_FLOAT);

    assert_eq!(parse_value_format(&json!(1)), Some(FORMAT_SYS_NUMBER));
    assert_eq!(parse_value_format(&json!("sys_order_price")), Some(FORMAT_SYS_ORDER_PRICE));
    assert_eq!(parse_value_format(&json!("float")), Some(FORMAT_FLOAT));
    assert_eq!(parse_value_format(&json!(true)), None);

    let mut items = vec![
        json!({
            "ExplicitValue": "2.5",
            "unformatted": "same"
        }),
        json!("non-object"),
    ];
    apply_template_fields(&mut items, &formats);
    assert_eq!(items[0]["ExplicitValue"], 2.5);
    assert_eq!(items[0]["unformatted"], "same");
    assert_eq!(items[1], "non-object");
}

#[test]
fn transformers_cover_default_percentage_and_specific_goods_option_lookup() {
    let mut percentage_items = vec![
        json!({ "SuccessRate": "0.1255" }),
        json!({ "SuccessRate": "not-a-number" }),
        json!("non-object"),
    ];
    apply_percentage_transformer(&mut percentage_items, "success_rate", &BTreeMap::new());
    assert_eq!(percentage_items[0]["SuccessRate"], "12.55%");
    assert_eq!(percentage_items[1]["SuccessRate"], "not-a-number");

    let report_config = json!({
        "mockGoodsOptions": [
            { "optionsId": "1,2", "optionsName": "Generic" },
            {
                "companyId": "company-1",
                "productId": "goods-1",
                "optionsId": "1,2",
                "optionsName": "Specific"
            }
        ]
    });
    let args = BTreeMap::from([("goodsField".to_string(), json!("ProductCode"))]);
    let params = BTreeMap::from([("companyId".to_string(), json!("company-1"))]);
    let mut items = vec![json!({
        "ProductCode": "goods-1",
        "options_id": "[\"1\", \"2\"]",
        "options_name": ""
    })];
    apply_goods_options_name_transformer(
        &mut items,
        "options_name",
        &args,
        Some(&report_config),
        &params,
    );
    assert_eq!(items[0]["options_name"], "Specific");

    assert_eq!(normalize_options_lookup_key(" 1, , 2 "), "1,2");
    assert_eq!(normalize_options_lookup_key("[]"), "[]");
    assert_eq!(value_lookup_string(&json!({ "id": 1 })), None);
}

#[tokio::test]
async fn http_catalog_and_request_headers_expose_metadata_without_sending_requests() {
    let http = connection(AiDataConnectionKind::Http);
    let catalog = connection_catalog(&http, 1).await.unwrap();
    assert_eq!(catalog["default_path"], "/default");
    assert_eq!(catalog["headers"], json!(["X-Test"]));
    assert!(!serde_json::to_string(&catalog).unwrap().contains("Bearer token"));

    let client = build_http_client(0).unwrap();
    let request =
        apply_request_headers(client.get("http://127.0.0.1/metadata-only"), &http).build().unwrap();
    assert_eq!(request.headers()[reqwest::header::AUTHORIZATION].to_str().unwrap(), "Bearer token");
    assert_eq!(request.headers()["X-Test"].to_str().unwrap(), "yes");

    let without_token = AiDataConnectionDto {
        auth_token: Some("  ".to_string()),
        headers: BTreeMap::new(),
        ..http
    };
    let request = apply_request_headers(
        reqwest::Client::new().get("http://127.0.0.1/metadata-only"),
        &without_token,
    )
    .build()
    .unwrap();
    assert!(request.headers().get(reqwest::header::AUTHORIZATION).is_none());
}

#[test]
fn database_connection_urls_reject_missing_and_blank_values() {
    let missing =
        AiDataConnectionDto { connection_url: None, ..connection(AiDataConnectionKind::Mysql) };
    assert_eq!(
        database_connection_url(&missing, "MySQL").unwrap_err(),
        "MySQL 连接缺少 connection_url"
    );

    let blank = AiDataConnectionDto {
        connection_url: Some("  \n".to_string()),
        ..connection(AiDataConnectionKind::Postgres)
    };
    assert_eq!(
        database_connection_url(&blank, "PostgreSQL").unwrap_err(),
        "PostgreSQL 连接缺少 connection_url"
    );

    let configured = connection(AiDataConnectionKind::Postgres);
    assert_eq!(database_connection_url(&configured, "PostgreSQL").unwrap(), "postgres://example");
}

#[test]
fn goods_option_records_support_nested_maps_and_safe_fallbacks() {
    let transformer_args = BTreeMap::from([(
        "mockData".to_string(),
        json!({
            "companyId": "company-1",
            "goodsId": "goods-1",
            "1, 2": "Red / XL",
            "records": [
                { "optionsId": "3", "optionsName": "Three" }
            ]
        }),
    )]);
    let records = collect_mock_goods_options_records(&transformer_args, None);
    assert_eq!(records.len(), 2);

    let object = json!({
        "companyId": "company-1",
        "goodsId": "goods-1"
    })
    .as_object()
    .unwrap()
    .clone();
    assert_eq!(
        resolve_mock_goods_options_name(
            &object,
            &json!(["1", "2"]),
            &records,
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .as_deref(),
        Some("Red / XL")
    );
    assert_eq!(
        resolve_mock_goods_options_name(
            &object,
            &json!(0),
            &records,
            &BTreeMap::new(),
            &BTreeMap::new(),
        ),
        None
    );

    let mut items = vec![json!({ "options_name": ["Red", "XL"] })];
    apply_goods_options_name_transformer(
        &mut items,
        "options_name",
        &BTreeMap::new(),
        None,
        &BTreeMap::new(),
    );
    assert_eq!(items[0]["options_name"], "Red / XL");
}
