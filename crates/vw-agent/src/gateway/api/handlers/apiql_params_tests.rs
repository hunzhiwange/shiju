// 覆盖 ApiQL 查询参数的别名、默认值与非法形状处理。

use serde_json::json;
use vw_api_types::apiql::ApiQlListMode;

use super::params::{ApiQlFieldNameCase, normalize_apiql_query};

// 字段别名、数据库值与分页参数由同一组夹具固定，以同时约束兼容转换和查询语义。
#[test]
fn apiql_params_merges_free_query_into_where() {
    let params = normalize_apiql_query(
        "name=free&status=enabled&where[name]=explicit&page=2&page_size=25&size=99",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    assert_eq!(params.page, Some(2));
    assert_eq!(params.page_size, Some(25));
    assert_eq!(
        params.where_,
        json!({
            "name": "explicit",
            "status": "enabled",
            "size": "99",
        })
    );
}

#[test]
fn apiql_params_merges_free_symbol_operator_query_into_where() {
    let params = normalize_apiql_query("id[<=]=10&where[active][!=]=0", ApiQlListMode::Index)
        .expect("query params should parse");

    assert_eq!(
        params.where_,
        json!({
            "id": {
                "<=": "10",
            },
            "active": {
                "!=": "0",
            },
        })
    );
}

#[test]
fn apiql_params_parses_nested_relation_config() {
    let params = normalize_apiql_query(
        "relation[role][orderBy]=id%20desc&relation[role][where][enabled]=1&orderBy[]=id%20desc&orderBy[]=name%20asc",
        ApiQlListMode::Index,
    )
    .expect("nested relation params should parse");

    assert_eq!(
        params.relation,
        json!({
            "role": {
                "orderBy": "id desc",
                "where": {
                    "enabled": "1",
                },
            },
        }),
    );
    assert_eq!(params.order_by, json!(["id desc", "name asc"]));
}

#[test]
fn apiql_params_sets_list_mode_flags() {
    let index_only = normalize_apiql_query("", ApiQlListMode::IndexOnly).expect("index-only mode");
    let index_count =
        normalize_apiql_query("expectPage=1", ApiQlListMode::IndexCount).expect("index-count mode");
    let index_page = normalize_apiql_query("", ApiQlListMode::IndexPage).expect("index-page mode");

    assert!(index_only.only);
    assert!(!index_only.count);
    assert!(!index_only.expect_page);

    assert!(!index_count.only);
    assert!(index_count.count);
    assert!(index_count.expect_page);

    assert!(!index_page.only);
    assert!(!index_page.count);
    assert!(index_page.expect_page);
}

#[test]
fn apiql_params_parses_response_shape_switches() {
    let defaults = normalize_apiql_query("", ApiQlListMode::Index).expect("default params");
    assert_eq!(defaults.field_case, ApiQlFieldNameCase::Snake);
    assert!(!defaults.development);
    assert!(defaults.transformer_keep_column);
    assert_eq!(defaults.transformer_delimiter, ".");

    let params = normalize_apiql_query(
        "fieldCase=camel&development=1&transformerKeepColumn=0&transformerDelimiter=__&name=apple",
        ApiQlListMode::Index,
    )
    .expect("response shape params should parse");

    assert_eq!(params.field_case, ApiQlFieldNameCase::Camel);
    assert!(params.development);
    assert!(!params.transformer_keep_column);
    assert_eq!(params.transformer_delimiter, "__");
    assert_eq!(params.where_, json!({ "name": "apple" }));
}

#[test]
fn apiql_params_ignores_php_signature_metadata() {
    let params = normalize_apiql_query(
        "column=id&format=json&app_key=key&app_secret=secret&timestamp=123&signature_method=hmac_sha256&token=jwt&token_type=bearer&version=v1&signature=sig&name=apple",
        ApiQlListMode::Index,
    )
    .expect("query params should parse");

    assert_eq!(params.column, json!("id"));
    assert_eq!(params.where_, json!({ "name": "apple" }));
}

// 非法字段、运算符和解析器配置必须在生成查询或执行脚本前被拒绝。
#[test]
fn apiql_params_rejects_invalid_encoding_and_bracket_shapes() {
    let encoding = normalize_apiql_query("name=%E0%A4%A", ApiQlListMode::Index)
        .expect_err("invalid percent encoding should fail");
    let key = normalize_apiql_query("[name]=apple", ApiQlListMode::Index)
        .expect_err("key cannot start with a bracket");
    let shape = normalize_apiql_query("where=active&where[id]=1", ApiQlListMode::Index)
        .expect_err("scalar and object shapes must not conflict");

    assert!(encoding.to_string().contains("Invalid ApiQL query encoding"));
    assert!(key.to_string().contains("Invalid ApiQL query key"));
    assert!(shape.to_string().contains("Conflicting ApiQL query parameter shape"));
}

#[test]
fn apiql_params_collects_repeated_free_fields_into_arrays() {
    let params = normalize_apiql_query("tag=red&tag=blue&tag=green", ApiQlListMode::Index)
        .expect("repeated fields should parse");

    assert_eq!(params.where_, json!({ "tag": ["red", "blue", "green"] }));
}

#[test]
fn apiql_params_parses_missing_equals_and_push_array_shapes() {
    let params = normalize_apiql_query(
        "flag&where[tags][]=red&where[tags][]=blue&where[items][][id]=1&where[items][][id]=2",
        ApiQlListMode::Index,
    )
    .expect("push array query params should parse");

    assert_eq!(
        params.where_,
        json!({
            "flag": "",
            "tags": ["red", "blue"],
            "items": [
                { "id": "1" },
                { "id": "2" },
            ],
        })
    );
}

#[test]
fn apiql_params_accepts_boolean_words_and_field_case_aliases() {
    let params = normalize_apiql_query(
        "expectPage=yes&only=off&count=on&field_case=camel-case&dev=no",
        ApiQlListMode::Index,
    )
    .expect("boolean aliases should parse");

    assert!(params.expect_page);
    assert!(!params.only);
    assert!(params.count);
    assert_eq!(params.field_case, ApiQlFieldNameCase::Camel);
    assert!(!params.development);
}

#[test]
fn apiql_params_allows_scalar_where_only_without_free_fields() {
    let scalar = normalize_apiql_query("where=active", ApiQlListMode::Index)
        .expect("standalone scalar where is preserved");
    let mixed = normalize_apiql_query("where=active&name=apple", ApiQlListMode::Index)
        .expect_err("scalar where cannot merge with free fields");

    assert_eq!(scalar.where_, json!("active"));
    assert!(mixed.to_string().contains("ApiQL where must be an object"));
}

#[test]
fn apiql_params_rejects_empty_keys_malformed_brackets_and_array_conflicts() {
    let empty_key = normalize_apiql_query("=value", ApiQlListMode::Index)
        .expect_err("empty query key should fail");
    let trailing = normalize_apiql_query("where[id]bad=1", ApiQlListMode::Index)
        .expect_err("trailing text after bracket should fail");
    let unclosed = normalize_apiql_query("where[id=1", ApiQlListMode::Index)
        .expect_err("unclosed bracket should fail");
    let array_conflict =
        normalize_apiql_query("where[tags]=red&where[tags][]=blue", ApiQlListMode::Index)
            .expect_err("scalar and array shapes should conflict");

    assert!(empty_key.to_string().contains("Invalid ApiQL query key"));
    assert!(trailing.to_string().contains("Invalid ApiQL query key"));
    assert!(unclosed.to_string().contains("Invalid ApiQL query key"));
    assert!(array_conflict.to_string().contains("Conflicting ApiQL query parameter shape"));
}
