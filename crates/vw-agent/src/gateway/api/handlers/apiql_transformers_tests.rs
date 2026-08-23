// 覆盖 ApiQL 关联转换器的解析、子查询构建与结果合并。

use super::*;
use serde_json::json;
use std::collections::BTreeMap;
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};
use vw_api_types::id::TenantScope;

use super::super::connections::ApiQlConnectionKind;

fn field(name: &str, column: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        ..ApiQlFieldConfig::default()
    }
}

fn registry() -> ApiQlEntityRegistry {
    ApiQlEntityRegistry::from_configs(vec![
        ApiQlEntityConfig {
            entity_class: "product:product".to_string(),
            connection_id: "main".to_string(),
            table: "products".to_string(),
            fields: vec![field("productId", "product_id"), field("brandId", "brand_id")],
            ..ApiQlEntityConfig::default()
        },
        ApiQlEntityConfig {
            entity_class: "brand:brand".to_string(),
            connection_id: "main".to_string(),
            table: "brands".to_string(),
            fields: vec![field("id", "id"), field("displayName", "display_name")],
            ..ApiQlEntityConfig::default()
        },
    ])
    .expect("transformer entities should be valid")
}

fn transformer_arg(has_many: bool) -> ApiTransformerArg {
    ApiTransformerArg {
        api: "brand:brand/list-only".to_string(),
        source_key: "product.id".to_string(),
        target_key: "id".to_string(),
        columns: vec!["display_name".to_string()],
        params: Map::new(),
        has_many,
    }
}

fn connection() -> ApiQlConnection {
    ApiQlConnection {
        id: "main".to_string(),
        name: "Main".to_string(),
        kind: ApiQlConnectionKind::Sqlite,
        description: None,
        enabled: true,
        read_only: true,
        connection_url: None,
        sqlite_path: Some(":memory:".to_string()),
        updated_at_ms: 0,
    }
}

#[test]
fn transformer_source_fields_parse_string_configs_and_deduplicate_aliases() {
    let registry = registry();
    let transformer = json!({
        "code": "join-brands",
        "type": "API",
        "args": {
            "brand": {
                "api": "brand:brand/list-only",
                "source_key": " product.id ",
                "target_key": "id",
                "columns": "id, display_name"
            },
            "brand_copy": {
                "api": "brand:brand/only",
                "source_key": "product_id",
                "target_key": "id",
                "columns": ["display_name"]
            }
        }
    })
    .to_string();

    assert_eq!(
        transformer_source_fields(
            registry.resolve("product:product").unwrap(),
            &Value::String(transformer)
        )
        .unwrap(),
        vec!["productId"]
    );
}

#[test]
fn transformer_parsing_rejects_malformed_and_unsupported_configs() {
    let registry = registry();
    let entity = registry.resolve("product:product").unwrap();

    assert!(
        transformer_source_fields(entity, &Value::String("{".to_string()))
            .unwrap_err()
            .to_string()
            .contains("Invalid ApiQL transformer JSON")
    );
    assert_eq!(
        transformer_source_fields(entity, &json!({"code": "x", "type": "rhai"}))
            .unwrap_err()
            .to_string(),
        "Unsupported ApiQL transformer type: rhai"
    );
    assert_eq!(
        transformer_source_fields(
            entity,
            &json!({
                "code": "x",
                "type": "api",
                "args": {"bad": {"api": "brand:brand", "source_key": "missing", "target_key": "id"}}
            })
        )
        .unwrap_err()
        .to_string(),
        "Unknown ApiQL transformer source_key: missing"
    );
    assert_eq!(
        field_list(&json!(["id, display_name", ["extra"]])).unwrap(),
        vec!["id", "display_name", "extra"]
    );
    assert_eq!(
        field_list(&json!(true)).unwrap_err().to_string(),
        "Invalid ApiQL transformer field list"
    );
    assert!(
        parse_api_target(&registry, "brand:brand/unsupported")
            .unwrap_err()
            .to_string()
            .contains("Unsupported ApiQL transformer api mode")
    );
}

#[test]
fn transformer_collects_nonzero_sources_and_merges_flat_and_nested_rows() {
    let registry = registry();
    let source = registry.resolve("product:product").unwrap();
    let target = registry.resolve("brand:brand").unwrap();
    let arg = transformer_arg(false);
    let items = vec![
        json!(null),
        json!({"productId": 0}),
        json!({"productId": "0"}),
        json!({"productId": 7}),
        json!({"productId": "7"}),
    ];

    assert_eq!(
        collect_source_values(source, ApiQlFieldNameCase::Camel, &items, &arg).unwrap(),
        vec![json!(7)]
    );

    let mut params = super::super::params::normalize_apiql_query(
        "fieldCase=camel&transformerKeepColumn=1&transformerDelimiter=__",
        ApiQlListMode::Index,
    )
    .unwrap();
    let rows =
        vec![json!({"id": 7, "displayName": "Acme"}), json!({"id": 8, "displayName": "Other"})];
    let mut object = json!({"productId": "7"}).as_object().unwrap().clone();
    merge_api_rows(&mut object, source, target, &params, "brand", &arg, &rows).unwrap();
    assert_eq!(object["brand__displayName"], "Acme");
    assert_eq!(object["brand__id"], 7);

    params.transformer_keep_column = false;
    let many = transformer_arg(true);
    let mut object = json!({"productId": 7}).as_object().unwrap().clone();
    merge_api_rows(&mut object, source, target, &params, "brands", &many, &rows).unwrap();
    assert_eq!(object["brands"], json!([{"id": 7, "displayName": "Acme"}]));
}

#[tokio::test]
async fn apply_transformers_returns_early_and_rejects_unsupported_runtime_type() {
    let registry = registry();
    let entity = registry.resolve("product:product").unwrap();
    let mut params = super::super::params::normalize_apiql_query("", ApiQlListMode::Index).unwrap();
    let mut object_data = json!({"productId": 7});

    apply_apiql_transformers(
        TenantScope::default(),
        connection(),
        &registry,
        entity,
        &params,
        &mut object_data,
    )
    .await
    .unwrap();
    assert_eq!(object_data, json!({"productId": 7}));

    params.transformer = json!({"code": "noop", "type": "rhai"});
    let mut rows = json!([{"productId": 7}]);
    let error = apply_apiql_transformers(
        TenantScope::default(),
        connection(),
        &registry,
        entity,
        &params,
        &mut rows,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "Unsupported ApiQL transformer type: rhai");
}

#[tokio::test]
async fn api_transformer_skips_query_when_source_values_are_empty() {
    let registry = registry();
    let entity = registry.resolve("product:product").unwrap();
    let params = super::super::params::normalize_apiql_query("", ApiQlListMode::Index).unwrap();
    let transformer = ApiQlTransformer {
        kind: "api".to_string(),
        args: BTreeMap::from([("brand".to_string(), transformer_arg(false))]),
    };
    let mut items = vec![json!(null), json!({"productId": 0}), json!({"other": 7})];

    apply_api_transformer(
        TenantScope::default(),
        connection(),
        &registry,
        entity,
        &params,
        &mut items,
        &transformer,
    )
    .await
    .unwrap();

    assert_eq!(items, vec![json!(null), json!({"productId": 0}), json!({"other": 7})]);
}

#[test]
fn build_child_params_resolves_columns_merges_filters_and_preserves_mode_defaults() {
    let registry = registry();
    let target = registry.resolve("brand:brand").unwrap();
    let parent = super::super::params::normalize_apiql_query(
        "fieldCase=camel&transformerKeepColumn=0&dev=1",
        ApiQlListMode::Index,
    )
    .unwrap();
    let mut arg = transformer_arg(false);
    arg.columns = vec!["display_name".to_string(), "id".to_string(), "displayName".to_string()];
    arg.params = Map::from_iter([
        ("where".to_string(), json!({"active": true})),
        ("column".to_string(), json!(["customColumn"])),
        ("keyColumn".to_string(), json!("id")),
        ("orderBy".to_string(), json!({"displayName": "desc"})),
        ("group_by".to_string(), json!("id")),
        ("customFilter".to_string(), json!("visible")),
        ("limit".to_string(), json!(5)),
        ("only".to_string(), json!(true)),
    ]);

    let child = build_child_params(
        target,
        ApiQlListMode::IndexPage,
        &parent,
        &arg,
        vec![json!(7), json!("8")],
    )
    .unwrap();

    assert_eq!(child.mode, ApiQlListMode::IndexPage);
    assert_eq!(child.page, Some(1));
    assert_eq!(child.limit, Some(10_000));
    assert_eq!(child.column, json!(["customColumn"]));
    assert_eq!(child.key_column, json!("id"));
    assert_eq!(child.order_by, json!({"displayName": "desc"}));
    assert_eq!(child.group_by, json!("id"));
    assert_eq!(child.where_["active"], true);
    assert_eq!(child.where_["customFilter"], "visible");
    assert_eq!(child.where_["id"], json!({"in": [7, "8"]}));
    assert!(!child.transformer_keep_column);
    assert_eq!(child.field_case, ApiQlFieldNameCase::Camel);
    assert!(child.development);
}

#[test]
fn build_child_params_reports_invalid_target_column_and_where_shapes() {
    let registry = registry();
    let target = registry.resolve("brand:brand").unwrap();
    let parent = super::super::params::normalize_apiql_query("", ApiQlListMode::Index).unwrap();
    let mut arg = transformer_arg(false);
    arg.target_key = "missing".to_string();
    assert_eq!(
        build_child_params(target, ApiQlListMode::IndexOnly, &parent, &arg, vec![json!(1)])
            .unwrap_err()
            .to_string(),
        "Unknown ApiQL transformer target_key: missing"
    );

    arg.target_key = "id".to_string();
    arg.columns = vec!["missing_column".to_string()];
    assert_eq!(
        build_child_params(target, ApiQlListMode::IndexOnly, &parent, &arg, vec![json!(1)])
            .unwrap_err()
            .to_string(),
        "Unknown ApiQL transformer column: missing_column"
    );

    arg.columns.clear();
    arg.params = Map::from_iter([("where".to_string(), json!("not-object"))]);
    assert_eq!(
        build_child_params(target, ApiQlListMode::IndexOnly, &parent, &arg, vec![json!(1)])
            .unwrap_err()
            .to_string(),
        "ApiQL transformer where must be an object"
    );

    let mut child = parent.clone();
    child.where_ = Value::Null;
    assert_eq!(
        merge_child_params(&mut child, &Map::from_iter([("free".to_string(), json!(1))]))
            .unwrap_err()
            .to_string(),
        "ApiQL where must be an object"
    );
}

#[test]
fn parse_transformers_accepts_collection_shapes_and_reports_arg_errors() {
    let empty = parse_transformers(&Value::Null).unwrap();
    assert!(empty.is_empty());
    assert!(parse_transformers(&Value::String("  ".to_string())).unwrap().is_empty());
    assert_eq!(
        parse_transformers(&json!([
            {"code": "a", "type": "api"},
            {"code": "b", "type": "API", "args": {}}
        ]))
        .unwrap()
        .len(),
        2
    );
    assert_eq!(parse_transformers(&json!({"code": "direct", "type": "api"})).unwrap().len(), 1);
    assert_eq!(
        parse_transformers(&json!({
            "first": {"code": "a", "type": "api"},
            "second": {"code": "b", "type": "api"}
        }))
        .unwrap()
        .len(),
        2
    );
    assert_eq!(
        parse_transformers(&json!(42)).unwrap_err().to_string(),
        "Invalid ApiQL transformer config"
    );
    assert_eq!(
        parse_transformer(&json!({"type": "api"})).unwrap_err().to_string(),
        "ApiQL transformer missing field: code"
    );
    assert_eq!(
        parse_transformer(&json!("bad")).unwrap_err().to_string(),
        "ApiQL transformer must be an object"
    );
    assert_eq!(
        parse_transformer(&json!({"code": "x"})).unwrap_err().to_string(),
        "ApiQL transformer missing field: type"
    );
    assert_eq!(
        parse_api_arg("brand", &json!("bad")).unwrap_err().to_string(),
        "ApiQL transformer arg must be an object: brand"
    );
    assert_eq!(
        parse_api_arg("brand", &json!({"source_key": "id", "target_key": "id"}))
            .unwrap_err()
            .to_string(),
        "ApiQL transformer missing field: api"
    );
    assert_eq!(
        parse_api_arg("brand", &json!({"api": "brand:brand"})).unwrap_err().to_string(),
        "ApiQL transformer missing field: source_key"
    );
    assert_eq!(
        parse_api_arg("brand", &json!({"api": "brand:brand", "source_key": "id"}))
            .unwrap_err()
            .to_string(),
        "ApiQL transformer missing field: target_key"
    );
}

#[test]
fn parse_api_target_accepts_php_mode_aliases() {
    let registry = registry();
    let cases = [
        ("brand:brand", ApiQlListMode::IndexOnly),
        ("brand:brand/", ApiQlListMode::Index),
        ("brand:brand/list", ApiQlListMode::Index),
        ("brand:brand/page", ApiQlListMode::IndexPage),
        ("brand:brand/count", ApiQlListMode::IndexCount),
    ];

    for (api, expected_mode) in cases {
        let target = parse_api_target(&registry, api).unwrap();
        assert_eq!(target.mode, expected_mode);
        assert_eq!(target.entity.config().entity_class, "brand:brand");
    }
}

#[test]
fn merge_api_rows_handles_missing_values_and_many_flat_outputs() {
    let registry = registry();
    let source = registry.resolve("product:product").unwrap();
    let target = registry.resolve("brand:brand").unwrap();
    let params = super::super::params::normalize_apiql_query(
        "fieldCase=camel&transformerKeepColumn=1&transformerDelimiter=__",
        ApiQlListMode::Index,
    )
    .unwrap();
    let many = transformer_arg(true);
    let rows = vec![
        json!(null),
        json!({"displayName": "missing target"}),
        json!({"id": 8, "displayName": "other"}),
        json!({"id": "7", "displayName": "Acme"}),
        json!({"id": 7.0, "displayName": "Acme duplicate"}),
    ];

    let mut missing_source = json!({"other": 7}).as_object().unwrap().clone();
    merge_api_rows(&mut missing_source, source, target, &params, "brand", &many, &rows).unwrap();
    assert_eq!(missing_source, json!({"other": 7}).as_object().unwrap().clone());

    let mut object = json!({"productId": 7}).as_object().unwrap().clone();
    merge_api_rows(&mut object, source, target, &params, "brand", &many, &rows).unwrap();
    assert_eq!(object["brand__id"], json!(["7", 7.0]));
    assert_eq!(object["brand__displayName"], json!(["Acme", "Acme duplicate"]));

    let mut nested = json!({"productId": 7}).as_object().unwrap().clone();
    let single = transformer_arg(false);
    let nested_params = super::super::params::normalize_apiql_query(
        "fieldCase=camel&transformerKeepColumn=0",
        ApiQlListMode::Index,
    )
    .unwrap();
    merge_api_rows(&mut nested, source, target, &nested_params, "brand", &single, &rows).unwrap();
    assert_eq!(nested["brand"], json!({"id": 7.0, "displayName": "Acme duplicate"}));
}

#[test]
fn key_case_and_loose_value_helpers_cover_fallbacks() {
    let registry = registry();
    let source = registry.resolve("product:product").unwrap();
    let object = json!({
        "primary": 1,
        "product.id": 2,
        "product_id": 3,
        "brandId": 4
    })
    .as_object()
    .unwrap()
    .clone();

    assert_eq!(object_value(&object, "primary", "missing"), Some(&json!(1)));
    assert_eq!(object_value(&object, "missing", "product.id"), Some(&json!(2)));
    assert_eq!(object_value(&object, "missing", "product.id.missing"), None);
    assert_eq!(object_value(&object, "missing", "brand_id"), Some(&json!(4)));
    assert_eq!(source_output_key(source, "product.id", ApiQlFieldNameCase::Camel), "productId");
    assert_eq!(output_field_key(source, "productId", ApiQlFieldNameCase::Snake), "product_id");
    assert_eq!(output_field_key(source, "unknown-key", ApiQlFieldNameCase::Snake), "unknown_key");
    assert_eq!(output_field_key(source, "unknown_key", ApiQlFieldNameCase::Camel), "unknownKey");
    assert_eq!(resolve_entity_field_name(source, "brand_id"), Some("brandId".to_string()));
    assert!(resolve_entity_field(source, "missing").is_none());

    assert!(is_zero_source_value(&Value::Null));
    assert!(is_zero_source_value(&json!(" 0 ")));
    assert!(is_zero_source_value(&json!(0.0)));
    assert!(!is_zero_source_value(&json!(false)));
    assert!(loose_values_equal(&json!("7.0"), &json!(7)));
    assert!(loose_values_equal(&json!({"a": 1}), &json!({"a": 1})));
    assert_eq!(loose_value_key(&json!(true)), "true");
    assert_eq!(loose_value_key(&Value::Null), "");
    assert!(numeric_value(&json!(false)).is_none());
    assert_eq!(to_snake_case("HTTPServer ID"), "http_server_id");
    assert_eq!(to_snake_case("already__snake"), "already_snake");
    assert_eq!(to_camel_case("AlreadyCamel"), "AlreadyCamel");
    assert_eq!(to_camel_case("two words-here"), "twoWordsHere");
}
