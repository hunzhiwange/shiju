use super::data::{
    AiDataAiQueryRequest, AiDataAiQueryResponse, AiDataConnectionCatalogResponse,
    AiDataConnectionDto, AiDataConnectionKind, AiDataConnectionTestResponse,
    AiDataConnectionUpsertBody, AiDataCountMode, AiDataExecutionPlan, AiDataQueryKind,
    AiDataQueryRequest, AiDataQueryResponse, AiDataReportDto, AiDataReportSourceDto,
    AiDataReportUpsertBody, AiDataSettings, AiDataSettingsUpdateBody, AiDataSourceMode,
    AiDataTemplateFieldDto, AiDataTransformerFieldDto,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn parse_query(value: Value) -> AiDataQueryRequest {
    serde_json::from_value(value).expect("query request")
}

#[test]
fn settings_and_connection_defaults_are_applied() {
    let settings: AiDataSettings = serde_json::from_value(json!({})).expect("settings");
    assert_eq!(settings.schema_version, 1);
    assert_eq!(settings.default_limit, 100);
    assert_eq!(settings.default_timeout_secs, 30);

    let connection: AiDataConnectionDto = serde_json::from_value(json!({
        "id": "c1",
        "name": "local",
        "kind": "sqlite"
    }))
    .expect("connection");
    assert_eq!(connection.kind, AiDataConnectionKind::Sqlite);
    assert!(connection.enabled);
    assert!(!connection.read_only);
    assert!(connection.headers.is_empty());
    assert_eq!(connection.updated_at_ms, 0);
    assert!(connection.last_used_ms.is_none());
}

#[test]
fn query_request_accepts_string_encoded_compat_fields() {
    let query: AiDataQueryRequest = serde_json::from_value(json!({
        "count": "count_only",
        "searchFields": " name, email ",
        "searchCondition": "{\"name\":\"Ada\"}"
    }))
    .expect("query");

    assert_eq!(query.count, Some(AiDataCountMode::Only));
    assert_eq!(query.search_fields, Some(vec!["name".into(), "email".into()]));
    assert_eq!(query.search_condition, Some(json!({"name": "Ada"})));
}

#[test]
fn report_source_defaults_http_fields() {
    let source: AiDataReportSourceDto = serde_json::from_value(json!({
        "source_key": "main",
        "connection_id": "c1",
        "query_kind": "http"
    }))
    .expect("source");

    assert_eq!(source.http_method, "GET");
    assert!(source.append_pagination);
    assert!(source.cube_query.is_none());
    assert!(source.http_body.is_none());
}

#[test]
fn query_request_normalizes_mixed_search_field_arrays() {
    let query = parse_query(json!({
        "search_fields": [" name ", "", 7, true, null, {"field": "email"}]
    }));

    assert_eq!(
        query.search_fields,
        Some(vec![
            "name".into(),
            "7".into(),
            "true".into(),
            "null".into(),
            "{\"field\":\"email\"}".into(),
        ])
    );
}

#[test]
fn query_request_accepts_json_encoded_search_field_array() {
    let query = parse_query(json!({
        "searchFields": "[\"name\",\"email\"]"
    }));

    assert_eq!(query.search_fields, Some(vec!["name".into(), "email".into()]));
}

#[test]
fn query_request_treats_empty_compatibility_fields_as_absent() {
    let empty_strings = parse_query(json!({
        "searchFields": "  ",
        "templateFields": "\n",
        "transformerFields": "\t",
        "searchCondition": "\t",
    }));
    let nulls = parse_query(json!({
        "searchFields": null,
        "templateFields": null,
        "transformerFields": null,
        "searchCondition": null,
        "count": null
    }));

    assert_eq!(empty_strings.search_fields, None);
    assert_eq!(empty_strings.template_fields, None);
    assert_eq!(empty_strings.transformer_fields, None);
    assert_eq!(empty_strings.search_condition, None);
    assert_eq!(nulls.search_fields, None);
    assert_eq!(nulls.template_fields, None);
    assert_eq!(nulls.transformer_fields, None);
    assert_eq!(nulls.search_condition, None);
    assert_eq!(nulls.count, None);
}

#[test]
fn query_request_accepts_template_fields_as_array_and_json_string() {
    let from_array = parse_query(json!({
        "template_fields": [{"code": "price", "valueType": 2}]
    }));
    let from_string = parse_query(json!({
        "templateFields": "[{\"code\":\"created_at\",\"valueType\":3}]"
    }));

    assert_eq!(
        from_array.template_fields,
        Some(vec![AiDataTemplateFieldDto { code: "price".into(), value_type: 2 }])
    );
    assert_eq!(
        from_string.template_fields,
        Some(vec![AiDataTemplateFieldDto { code: "created_at".into(), value_type: 3 }])
    );
}

#[test]
fn query_request_accepts_transformer_fields_as_array_and_json_string() {
    let from_array = parse_query(json!({
        "transformer_fields": [{
            "code": "amount",
            "transformerType": "currency",
            "transformerArgs": {"currency": "CNY"}
        }]
    }));
    let from_string = parse_query(json!({
        "transformerFields": "[{\"code\":\"name\",\"transformerType\":\"uppercase\"}]"
    }));

    assert_eq!(
        from_array.transformer_fields,
        Some(vec![AiDataTransformerFieldDto {
            code: "amount".into(),
            transformer_type: "currency".into(),
            transformer_args: BTreeMap::from([("currency".into(), json!("CNY"))]),
        }])
    );
    assert_eq!(
        from_string.transformer_fields,
        Some(vec![AiDataTransformerFieldDto {
            code: "name".into(),
            transformer_type: "uppercase".into(),
            transformer_args: BTreeMap::new(),
        }])
    );
}

#[test]
fn query_request_accepts_numeric_and_text_count_modes() {
    let cases = [
        (json!(0), AiDataCountMode::Disabled),
        (json!(1), AiDataCountMode::Enabled),
        (json!(2), AiDataCountMode::Only),
        (json!("0"), AiDataCountMode::Disabled),
        (json!("1"), AiDataCountMode::Enabled),
        (json!("2"), AiDataCountMode::Only),
        (json!(" DISABLED "), AiDataCountMode::Disabled),
        (json!("Enabled"), AiDataCountMode::Enabled),
        (json!("ONLY"), AiDataCountMode::Only),
    ];

    for (value, expected) in cases {
        let query = parse_query(json!({"count": value}));
        assert_eq!(query.count, Some(expected));
    }
}

#[test]
fn query_request_rejects_invalid_search_fields_payloads() {
    for value in [json!(42), json!("[\"valid\","), json!("[1]")] {
        let error = serde_json::from_value::<AiDataQueryRequest>(json!({
            "searchFields": value
        }));
        assert!(error.is_err());
    }
}

#[test]
fn query_request_rejects_invalid_structured_compatibility_fields() {
    let cases = [
        json!({"templateFields": true}),
        json!({"templateFields": "not json"}),
        json!({"templateFields": [{"code": "missing-value-type"}]}),
        json!({"transformerFields": 1}),
        json!({"transformerFields": "not json"}),
        json!({"transformerFields": [{"code": "missing-type"}]}),
    ];

    for value in cases {
        assert!(serde_json::from_value::<AiDataQueryRequest>(value).is_err());
    }
}

#[test]
fn query_request_rejects_invalid_count_modes() {
    for value in [json!(3), json!(1.5), json!("sometimes"), json!([])] {
        let error = serde_json::from_value::<AiDataQueryRequest>(json!({"count": value}))
            .expect_err("invalid count mode must be rejected");
        assert!(error.to_string().contains("invalid count mode"));
    }
}

#[test]
fn query_request_preserves_non_json_search_condition_strings() {
    let query = parse_query(json!({"searchCondition": " status = active "}));

    assert_eq!(query.search_condition, Some(Value::String(" status = active ".into())));
}

#[test]
fn query_request_accepts_direct_json_search_conditions() {
    let object = parse_query(json!({"search_condition": {"active": true}}));
    let array = parse_query(json!({"searchCondition": ["admin", "owner"]}));

    assert_eq!(object.search_condition, Some(json!({"active": true})));
    assert_eq!(array.search_condition, Some(json!(["admin", "owner"])));
}

#[test]
fn query_request_aliases_serialize_to_canonical_field_names() {
    let query = parse_query(json!({
        "report": "report-1",
        "source": "main",
        "pageSize": 25,
        "orderBy": "created_at desc",
        "templateCode": "sales"
    }));

    assert_eq!(
        serde_json::to_value(query).expect("serialize query request"),
        json!({
            "report_id": "report-1",
            "source_key": "main",
            "params": {},
            "limit": 25,
            "order_by": "created_at desc",
            "template_code": "sales"
        })
    );
}

#[test]
fn enum_variants_use_snake_case_protocol_values() {
    assert_eq!(
        serde_json::to_value([
            AiDataConnectionKind::Sqlite,
            AiDataConnectionKind::Mysql,
            AiDataConnectionKind::Postgres,
            AiDataConnectionKind::Cube,
            AiDataConnectionKind::Http,
        ])
        .expect("connection kinds"),
        json!(["sqlite", "mysql", "postgres", "cube", "http"])
    );
    assert_eq!(
        serde_json::to_value([AiDataSourceMode::Normal, AiDataSourceMode::Ai])
            .expect("source modes"),
        json!(["normal", "ai"])
    );
    assert_eq!(
        serde_json::to_value([AiDataQueryKind::Sql, AiDataQueryKind::Cube, AiDataQueryKind::Http,])
            .expect("query kinds"),
        json!(["sql", "cube", "http"])
    );
    assert_eq!(
        serde_json::to_value([
            AiDataCountMode::Disabled,
            AiDataCountMode::Enabled,
            AiDataCountMode::Only,
        ])
        .expect("count modes"),
        json!(["disabled", "enabled", "only"])
    );
}

#[test]
fn connection_upsert_and_test_response_apply_defaults() {
    let connection: AiDataConnectionUpsertBody = serde_json::from_value(json!({
        "name": "warehouse",
        "kind": "postgres"
    }))
    .expect("connection upsert");
    let response: AiDataConnectionTestResponse =
        serde_json::from_value(json!({"message": "not attempted"}))
            .expect("connection test response");

    assert!(connection.enabled);
    assert!(!connection.read_only);
    assert!(connection.headers.is_empty());
    assert!(connection.auth_token.is_none());
    assert!(!response.ok);
    assert_eq!(response.latency_ms, 0);
}

#[test]
fn report_and_query_response_apply_collection_defaults() {
    let report: AiDataReportDto = serde_json::from_value(json!({
        "id": "report-1",
        "name": "Sales",
        "slug": "sales",
        "data_source": "normal"
    }))
    .expect("report");
    let response: AiDataQueryResponse = serde_json::from_value(json!({
        "page": {
            "per_page": 20,
            "current_page": 1,
            "total_page": 0,
            "total_record": 0,
            "from": 0,
            "to": 0
        }
    }))
    .expect("query response");

    assert_eq!(report.report_config, Value::Null);
    assert!(report.sources.is_empty());
    assert_eq!(report.updated_at_ms, 0);
    assert!(response.items.is_empty());
    assert!(!response.has_next_page);
    assert!(response.next_cursor.is_none());
}

#[test]
fn ai_query_and_execution_plan_apply_defaults() {
    let request: AiDataAiQueryRequest = serde_json::from_value(json!({
        "prompt": "show sales",
        "report": "report-1",
        "source": "main"
    }))
    .expect("AI query request");
    let plan: AiDataExecutionPlan = serde_json::from_value(json!({
        "connection_id": "connection-1",
        "query_kind": "http"
    }))
    .expect("execution plan");

    assert_eq!(request.report_id.as_deref(), Some("report-1"));
    assert_eq!(request.source_key.as_deref(), Some("main"));
    assert!(request.params.is_empty());
    assert_eq!(plan.http_method, "GET");
    assert!(plan.http_path.is_none());
    assert!(plan.explanation.is_none());
}

#[test]
fn settings_update_serializes_required_limits() {
    let update = AiDataSettingsUpdateBody { default_limit: 250, default_timeout_secs: 45 };

    assert_eq!(
        serde_json::to_value(update).expect("settings update"),
        json!({"default_limit": 250, "default_timeout_secs": 45})
    );
}

#[test]
fn connection_catalog_preserves_nested_catalog_payload() {
    let catalog: AiDataConnectionCatalogResponse = serde_json::from_value(json!({
        "connection_id": "connection-1",
        "kind": "postgres",
        "catalog": {
            "schemas": [{"name": "public", "tables": ["orders", "customers"]}]
        }
    }))
    .expect("connection catalog");

    assert_eq!(catalog.connection_id, "connection-1");
    assert_eq!(catalog.kind, AiDataConnectionKind::Postgres);
    assert_eq!(
        catalog.catalog,
        json!({"schemas": [{"name": "public", "tables": ["orders", "customers"]}]})
    );
}

#[test]
fn report_upsert_applies_empty_configuration_defaults() {
    let report: AiDataReportUpsertBody = serde_json::from_value(json!({
        "name": "Inventory",
        "slug": "inventory",
        "data_source": "ai"
    }))
    .expect("report upsert");

    assert_eq!(report.data_source, AiDataSourceMode::Ai);
    assert_eq!(report.report_config, Value::Null);
    assert!(report.sources.is_empty());
    assert!(report.default_source_key.is_none());
}

#[test]
fn ai_query_response_omits_absent_optional_fields() {
    let response: AiDataAiQueryResponse = serde_json::from_value(json!({
        "plan": {
            "connection_id": "connection-1",
            "query_kind": "sql"
        },
        "result": {
            "page": {
                "per_page": 20,
                "current_page": 1,
                "total_page": 1,
                "total_record": 1,
                "from": 1,
                "to": 1
            },
            "items": [{"id": 1}]
        }
    }))
    .expect("AI query response");

    let serialized = serde_json::to_value(response).expect("serialize AI query response");
    assert!(serialized.get("answer").is_none());
    assert!(serialized.get("raw_model_response").is_none());
    assert_eq!(serialized["result"]["items"], json!([{"id": 1}]));
}
