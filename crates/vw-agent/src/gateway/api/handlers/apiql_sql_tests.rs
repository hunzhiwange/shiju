// 覆盖 ApiQL SQL 构建的筛选、排序、分页与注入防护。

use serde_json::json;
use vw_api_types::apiql::{
    ApiQlEntityConfig, ApiQlFieldConfig, ApiQlListMode, ApiQlRelationConfig,
};

use super::executor::ApiQlCountMode;
use super::params::normalize_apiql_query;
use super::schema::ApiQlEntityRegistry;
use super::sql::{ApiQlSqlQuery, build_apiql_sql};

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
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            field("id", "id", false, true),
            field("name", "name", true, true),
            field("sku", "sku", true, true),
            field("specTitle", "spec_title", false, true),
            field("active", "active", false, false),
            field("created_at", "created_at", false, true),
        ],
        search_fields: vec!["name".to_string(), "sku".to_string()],
        virtual_fields: vec![
            virtual_field("search_key", "name,sku"),
            virtual_field("skuTitle", "specTitle"),
        ],
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn vendor_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "vendor:vendor".to_string(),
        connection_id: "product-main".to_string(),
        table: "catalog.vendor".to_string(),
        fields: vec![field("id", "id", false, true), field("title", "title", true, true)],
        ..ApiQlEntityConfig::default()
    }
}

fn related_product_config() -> ApiQlEntityConfig {
    let mut config = product_config();
    config.fields.push(field("vendorId", "vendor_id", false, true));
    config.relations.push(ApiQlRelationConfig {
        name: "vendor".to_string(),
        entity_class: "vendor:vendor".to_string(),
        local_field: Some("vendorId".to_string()),
        foreign_field: Some("id".to_string()),
        ..ApiQlRelationConfig::default()
    });
    config
}

fn build_product_query(raw_query: &str, mode: ApiQlListMode) -> Result<ApiQlSqlQuery, String> {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query(raw_query, mode).expect("query params should parse");
    build_apiql_sql(&registry, entity, &params).map_err(|error| error.to_string())
}

fn build_related_product_query(
    raw_query: &str,
    config: ApiQlEntityConfig,
) -> Result<ApiQlSqlQuery, String> {
    let registry = ApiQlEntityRegistry::from_configs(vec![config, vendor_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params =
        normalize_apiql_query(raw_query, ApiQlListMode::Index).expect("query params should parse");
    build_apiql_sql(&registry, entity, &params).map_err(|error| error.to_string())
}

#[test]
fn apiql_sql_accepts_snake_virtual_column_alias() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query("column=sku_title", ApiQlListMode::Index)
        .expect("query params should parse");

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert_eq!(query.sql, "SELECT apiql_base.spec_title AS specTitle FROM product AS apiql_base");
    assert_eq!(query.virtual_fields, vec!["skuTitle"]);
    assert_eq!(query.hidden_fields, vec!["specTitle"]);
}

#[test]
fn apiql_sql_builds_default_select_object_fields_and_relation_join() {
    let query = build_related_product_query(
        "column[id]=1&column[name]=yes&column[sku]=0&column[active]=false&keyColumn=vendor_id&join=vendor&relation[vendor]=1&expectPage=1&pageSize=0",
        related_product_config(),
    )
    .expect("SQL should build");

    assert_eq!(
        query.sql,
        "SELECT apiql_base.id AS id, apiql_base.name AS name, apiql_base.vendor_id AS vendorId FROM product AS apiql_base LEFT JOIN catalog.vendor AS vendor ON apiql_base.vendor_id = vendor.id"
    );
    assert_eq!(query.limit, 1);
    assert_eq!(query.count_mode, ApiQlCountMode::Enabled);
}

#[test]
fn apiql_sql_selects_all_regular_fields_when_column_is_empty() {
    let query = build_product_query("", ApiQlListMode::Index).expect("SQL should build");

    assert_eq!(
        query.sql,
        "SELECT apiql_base.id AS id, apiql_base.name AS name, apiql_base.sku AS sku, apiql_base.spec_title AS specTitle, apiql_base.active AS active, apiql_base.created_at AS created_at FROM product AS apiql_base"
    );
    assert_eq!(query.count_sql, None);
    assert!(query.params.is_empty());
    assert_eq!(query.order_by, None);
}

#[test]
fn apiql_sql_builds_where_in_and_search_key_conditions() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query(
        "column[]=id&column[]=name&active=1&where[id][in][]=1&where[id][in][]=3&where[search_key]=apple%7Cpear%20-archived&orderBy=id%20desc",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert_eq!(
        query.sql,
        "SELECT apiql_base.id AS id, apiql_base.name AS name FROM product AS apiql_base WHERE apiql_base.active = {{ where_active }} AND apiql_base.id IN ({{ where_id_0 }}, {{ where_id_1 }}) AND ((apiql_base.name LIKE {{ search_0 }} OR apiql_base.sku LIKE {{ search_0 }}) OR (apiql_base.name LIKE {{ search_1 }} OR apiql_base.sku LIKE {{ search_1 }})) AND NOT (apiql_base.name LIKE {{ search_2 }} OR apiql_base.sku LIKE {{ search_2 }})"
    );
    assert_eq!(query.params.get("where_active"), Some(&json!("1")));
    assert_eq!(query.params.get("where_id_0"), Some(&json!("1")));
    assert_eq!(query.params.get("where_id_1"), Some(&json!("3")));
    assert_eq!(query.params.get("search_0"), Some(&json!("%apple%")));
    assert_eq!(query.params.get("search_1"), Some(&json!("%pear%")));
    assert_eq!(query.params.get("search_2"), Some(&json!("%archived%")));
    assert_eq!(query.page, 1);
    assert_eq!(query.limit, 25);
    assert_eq!(query.count_mode, ApiQlCountMode::Disabled);
    assert_eq!(query.order_by.as_deref(), Some("id DESC"));
}

#[test]
fn apiql_sql_builds_symbol_operator_conditions() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query(
        "column[]=id&id[=]=1&id[!=]=2&id[>]=3&id[>=]=4&id[<]=5&id[<=]=6",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert!(query.sql.contains("apiql_base.id = {{ where_id_eq }}"));
    assert!(query.sql.contains("apiql_base.id != {{ where_id_ne }}"));
    assert!(query.sql.contains("apiql_base.id > {{ where_id_gt }}"));
    assert!(query.sql.contains("apiql_base.id >= {{ where_id_gte }}"));
    assert!(query.sql.contains("apiql_base.id < {{ where_id_lt }}"));
    assert!(query.sql.contains("apiql_base.id <= {{ where_id_lte }}"));
    assert_eq!(query.params.get("where_id_eq"), Some(&json!("1")));
    assert_eq!(query.params.get("where_id_ne"), Some(&json!("2")));
    assert_eq!(query.params.get("where_id_gt"), Some(&json!("3")));
    assert_eq!(query.params.get("where_id_gte"), Some(&json!("4")));
    assert_eq!(query.params.get("where_id_lt"), Some(&json!("5")));
    assert_eq!(query.params.get("where_id_lte"), Some(&json!("6")));
}

#[test]
fn apiql_sql_builds_named_operator_conditions() {
    let query = build_product_query(
        "column=id&name[like]=app%25&sku[not_like]=old&id[not_in]=1,2&id[between][]=3&id[between][]=7&active[raw]=IS%20NOT%20NULL",
        ApiQlListMode::Index,
    )
    .expect("SQL should build");

    assert!(query.sql.contains("apiql_base.name LIKE {{ where_name_like }}"));
    assert!(query.sql.contains("apiql_base.sku NOT LIKE {{ where_sku_not like }}"));
    assert!(query.sql.contains("apiql_base.id NOT IN ({{ where_id_0 }}, {{ where_id_1 }})"));
    assert!(query.sql.contains(
        "apiql_base.id BETWEEN {{ where_id_between_start }} AND {{ where_id_between_end }}"
    ));
    assert!(query.sql.contains("apiql_base.active IS NOT NULL"));
    assert_eq!(query.params.get("where_name_like"), Some(&json!("app%")));
    assert_eq!(query.params.get("where_sku_not like"), Some(&json!("%old%")));
    assert_eq!(query.params.get("where_id_0"), Some(&json!("1")));
    assert_eq!(query.params.get("where_id_1"), Some(&json!("2")));
    assert_eq!(query.params.get("where_id_between_start"), Some(&json!("3")));
    assert_eq!(query.params.get("where_id_between_end"), Some(&json!("7")));
}

#[test]
fn apiql_sql_builds_null_array_and_raw_scalar_conditions() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let mut params = normalize_apiql_query("column=id", ApiQlListMode::Index)
        .expect("query params should parse");
    params.where_ = json!({
        "active": null,
        "id": [1, 2],
        "name": "raw:LOWER(name)"
    });

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert!(query.sql.contains("apiql_base.active IS NULL"));
    assert!(query.sql.contains("apiql_base.id IN ({{ where_id_0 }}, {{ where_id_1 }})"));
    assert!(query.sql.contains("apiql_base.name = LOWER(name)"));
}

#[test]
fn apiql_sql_builds_search_key_edge_cases() {
    let empty = build_product_query("column=id&where[search_key]=%20-%20", ApiQlListMode::Index)
        .expect("empty search terms should be ignored");
    assert_eq!(empty.sql, "SELECT apiql_base.id AS id FROM product AS apiql_base");

    let scoped = build_product_query(
        "column=id&where[search_key]=name%3Aapple%20!sku%3Aold",
        ApiQlListMode::Index,
    )
    .expect("field-scoped search should build");
    assert!(scoped.sql.contains("apiql_base.name LIKE {{ search_0 }}"));
    assert!(scoped.sql.contains("NOT (apiql_base.sku LIKE {{ search_1 }})"));

    assert_eq!(
        build_product_query("column=id&where[search_key]=active%3A1", ApiQlListMode::Index)
            .expect_err("unknown search field should fail"),
        "Unknown ApiQL search field: active"
    );
    assert_eq!(
        build_product_query("column=id&where[search_key]=name%3A", ApiQlListMode::Index)
            .expect_err("empty scoped search should fail"),
        "ApiQL search term cannot be empty"
    );

    let mut no_search_config = product_config();
    no_search_config.search_fields.clear();
    for field in &mut no_search_config.fields {
        field.searchable = false;
    }
    let registry = ApiQlEntityRegistry::from_configs(vec![no_search_config])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query("column=id&where[search_key]=apple", ApiQlListMode::Index)
        .expect("query params should parse");
    let error = build_apiql_sql(&registry, entity, &params)
        .expect_err("search without registered fields should fail");
    assert_eq!(error.to_string(), "ApiQL search_key has no registered search fields");
}

#[test]
fn apiql_sql_builds_cursor_grouping_and_clamps_pagination() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query(
        "column=id&groupBy=id&orderBy=created_at%20asc%2Cid%20desc&cursor=%7B%22created_at%22%3A%222024-01-01%22%2C%22id%22%3A9%7D&page=0&limit=20000&count=1",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert!(query.sql.contains("GROUP BY apiql_base.id"));
    assert!(query.sql.contains("apiql_base.created_at > {{ cursor_created_at }}"));
    assert!(query.sql.contains(
        "(apiql_base.created_at = {{ cursor_created_at }} AND apiql_base.id < {{ cursor_id }})"
    ));
    assert_eq!(query.params.get("cursor_created_at"), Some(&json!("2024-01-01")));
    assert_eq!(query.params.get("cursor_id"), Some(&json!(9)));
    assert_eq!(query.page, 1);
    assert_eq!(query.limit, 10_000);
    assert_eq!(query.count_mode, ApiQlCountMode::Only);
    assert_eq!(query.cursor_fields, vec!["created_at", "id"]);
}

#[test]
fn apiql_sql_accepts_camel_cursor_output_field_names() {
    let query = build_product_query(
        "column=id&orderBy=created_at%20asc&fieldCase=camel&cursor=%7B%22createdAt%22%3A%222024-01-01%22%7D",
        ApiQlListMode::Index,
    )
    .expect("SQL should build");

    assert!(query.sql.contains("apiql_base.created_at > {{ cursor_created_at }}"));
    assert_eq!(query.params.get("cursor_created_at"), Some(&json!("2024-01-01")));
}

#[test]
fn apiql_sql_parses_array_and_object_order_by() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let mut params = normalize_apiql_query("column=id", ApiQlListMode::Index)
        .expect("query params should parse");
    params.order_by = json!(["name desc", {"id": "asc"}]);

    let query = build_apiql_sql(&registry, entity, &params).expect("SQL should build");

    assert_eq!(query.order_by.as_deref(), Some("name DESC, id ASC"));
    assert_eq!(query.cursor_fields, vec!["name", "id"]);
}

#[test]
fn apiql_sql_rejects_unsafe_raw_where_expression() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query(
        "column=id&where[raw%3Aid%20%3D%201%3B%20DROP%20TABLE%20product]=1",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    let error =
        build_apiql_sql(&registry, entity, &params).expect_err("unsafe raw SQL should be rejected");

    assert_eq!(error.to_string(), "Unsafe ApiQL raw SQL expression");
}

#[test]
fn apiql_sql_rejects_invalid_field_lists_and_selectors() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let mut params =
        normalize_apiql_query("", ApiQlListMode::Index).expect("query params should parse");
    params.column = json!(123);
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("numeric column should fail")
            .to_string(),
        "Invalid ApiQL field list"
    );

    params.column = json!({"id": []});
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("array selector should fail")
            .to_string(),
        "Invalid ApiQL field selector"
    );

    let mut empty_config = product_config();
    empty_config.fields.clear();
    empty_config.search_fields.clear();
    empty_config.virtual_fields.clear();
    let registry = ApiQlEntityRegistry::from_configs(vec![empty_config])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    params.column = json!(null);
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("empty select should fail")
            .to_string(),
        "ApiQL select column list is empty"
    );

    params.column = json!("missing");
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("unknown column should fail")
            .to_string(),
        "Unknown ApiQL column field: missing"
    );
}

#[test]
fn apiql_sql_rejects_where_operator_and_search_errors() {
    assert_eq!(
        build_product_query("column=id&where=bad", ApiQlListMode::Index)
            .expect_err("scalar where should fail"),
        "ApiQL where must be an object"
    );
    assert_eq!(
        build_product_query("column=id&where[missing]=1", ApiQlListMode::Index)
            .expect_err("unknown where field should fail"),
        "Unknown ApiQL where field: missing"
    );
    assert_eq!(
        build_product_query("column=id&name[like][]=a", ApiQlListMode::Index)
            .expect_err("non-string like should fail"),
        "ApiQL LIKE operator requires a string"
    );
    assert_eq!(
        build_product_query("column=id&id[between][]=1", ApiQlListMode::Index)
            .expect_err("short between should fail"),
        "ApiQL BETWEEN operator requires two values"
    );
    assert_eq!(
        build_product_query("column=id&id[unknown]=1", ApiQlListMode::Index)
            .expect_err("unsupported operator should fail"),
        "Unsupported ApiQL where operator: unknown"
    );
    assert_eq!(
        build_product_query("column=id&active[raw][]=1", ApiQlListMode::Index)
            .expect_err("non-string raw operator should fail"),
        "ApiQL raw operator requires a string"
    );
    assert_eq!(
        build_product_query("column=id&where[search_key][]=apple", ApiQlListMode::Index)
            .expect_err("non-string search should fail"),
        "ApiQL search_key must be a string"
    );
}

#[test]
fn apiql_sql_rejects_cursor_order_and_group_errors() {
    assert_eq!(
        build_product_query("column=id&cursor=%7B%22id%22%3A1%7D", ApiQlListMode::Index)
            .expect_err("cursor without order should fail"),
        "ApiQL cursor requires order_by"
    );
    assert_eq!(
        build_product_query(
            "column=id&orderBy=id%20asc&cursor=%7B%22name%22%3A%22a%22%7D",
            ApiQlListMode::Index
        )
        .expect_err("missing cursor field should fail"),
        "ApiQL cursor is missing order field: id"
    );
    assert_eq!(
        build_product_query("column=id&orderBy=active%20asc", ApiQlListMode::Index)
            .expect_err("unsortable order should fail"),
        "ApiQL field is not sortable: active"
    );
    assert_eq!(
        build_product_query("column=id&orderBy=id%20sideways", ApiQlListMode::Index)
            .expect_err("bad order direction should fail"),
        "Invalid ApiQL default order direction"
    );
    assert_eq!(
        build_product_query("column=id&orderBy=1", ApiQlListMode::Index)
            .expect_err("invalid order field should fail"),
        "Unknown ApiQL order field: 1"
    );
    assert_eq!(
        build_product_query("column=id&groupBy=missing", ApiQlListMode::Index)
            .expect_err("unknown group field should fail"),
        "Unknown ApiQL group field: missing"
    );
}

#[test]
fn apiql_sql_rejects_cursor_json_shapes() {
    for (raw_query, expected) in [
        ("column=id&orderBy=id%20asc&cursor=not-json", "ApiQL cursor must be a JSON object"),
        ("column=id&orderBy=id%20asc&cursor=%5B1%5D", "ApiQL cursor must be a JSON object"),
        ("column=id&orderBy=id%20asc&cursor[]=1", "ApiQL cursor must be an object"),
    ] {
        assert_eq!(
            build_product_query(raw_query, ApiQlListMode::Index)
                .expect_err("invalid cursor should fail"),
            expected
        );
    }
}

#[test]
fn apiql_sql_rejects_relation_and_identifier_errors() {
    assert_eq!(
        build_related_product_query("column=id&join=missing", related_product_config())
            .expect_err("unknown relation should fail"),
        "Unknown ApiQL relation: missing"
    );

    let mut missing_related = related_product_config();
    missing_related.relations[0].entity_class = "vendor:missing".to_string();
    assert_eq!(
        build_related_product_query("column=id&join=vendor", missing_related)
            .expect_err("unknown related entity should fail"),
        "ApiQL entity not registered: vendor:missing"
    );

    let mut missing_local = related_product_config();
    missing_local.relations[0].local_field = Some("missing".to_string());
    assert_eq!(
        build_related_product_query("column=id&join=vendor", missing_local)
            .expect_err("unknown local relation field should fail"),
        "Unknown ApiQL relation local field: missing"
    );

    let mut missing_foreign = related_product_config();
    missing_foreign.relations[0].foreign_field = Some("missing".to_string());
    assert_eq!(
        build_related_product_query("column=id&join=vendor", missing_foreign)
            .expect_err("unknown foreign relation field should fail"),
        "Unknown ApiQL relation foreign field: missing"
    );

    let mut invalid_alias = related_product_config();
    invalid_alias.relations[0].name = "bad-name".to_string();
    assert_eq!(
        build_related_product_query("column=id&join=bad-name", invalid_alias)
            .expect_err("invalid relation alias should fail"),
        "Invalid ApiQL SQL identifier: bad-name"
    );
}

#[test]
fn apiql_sql_rejects_invalid_sql_identifiers_and_columns() {
    let mut invalid_table = product_config();
    invalid_table.table = "bad-table".to_string();
    let registry = ApiQlEntityRegistry::from_configs(vec![invalid_table])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query("column=id", ApiQlListMode::Index)
        .expect("query params should parse");
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("invalid table should fail")
            .to_string(),
        "Invalid ApiQL SQL identifier: bad-table"
    );

    let mut missing_column = product_config();
    missing_column.fields[0].column = None;
    let registry = ApiQlEntityRegistry::from_configs(vec![missing_column])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("missing field column should fail")
            .to_string(),
        "ApiQL field has no column: id"
    );

    let mut invalid_column = product_config();
    invalid_column.fields[0].column = Some("bad-column".to_string());
    let registry = ApiQlEntityRegistry::from_configs(vec![invalid_column])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");
    let params = normalize_apiql_query("column=id", ApiQlListMode::Index)
        .expect("query params should parse");
    assert_eq!(
        build_apiql_sql(&registry, entity, &params)
            .expect_err("invalid field column should fail")
            .to_string(),
        "Invalid ApiQL SQL identifier: bad-column"
    );
}
