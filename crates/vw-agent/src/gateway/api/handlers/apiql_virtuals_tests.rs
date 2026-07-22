use serde_json::{Map, Value, json};
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};

use super::super::schema::ApiQlEntityRegistry;

fn virtual_field_with_resolver(resolver: serde_json::Value) -> ApiQlFieldConfig {
    let mut field = ApiQlFieldConfig {
        name: "displayName".to_string(),
        column: Some("display_name".to_string()),
        label: Some("Display name".to_string()),
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    };
    field.meta.insert("resolver".to_string(), resolver);
    field
}

fn base_field(name: &str, column: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        searchable: true,
        sortable: true,
        ..ApiQlFieldConfig::default()
    }
}

fn virtual_field(name: &str, column: &str, resolver: Value) -> ApiQlFieldConfig {
    let mut field = ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        label: Some(format!("{name} label")),
        comment: Some(format!("{name} comment")),
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    };
    field.meta.insert("resolver".to_string(), resolver);
    field
}

fn demo_config(virtual_fields: Vec<ApiQlFieldConfig>) -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "demo:item".to_string(),
        connection_id: "demo-main".to_string(),
        table: "demo_items".to_string(),
        fields: vec![
            base_field("id", "id"),
            base_field("firstName", "first_name"),
            base_field("lastName", "last_name"),
            base_field("specTitle", "spec_title"),
        ],
        search_fields: Vec::new(),
        virtual_fields,
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn resolve_demo(config: ApiQlEntityConfig) -> ApiQlEntityRegistry {
    ApiQlEntityRegistry::from_configs(vec![config]).expect("valid ApiQL entity config")
}

#[test]
fn run_virtual_config_executes_inline_rhai_with_field_context() {
    let field = virtual_field_with_resolver(json!({}));
    let config = json!({
        "engine": "rhai",
        "function": "resolve",
        "script": "fn resolve(row, context) { context[\"name\"] }"
    });

    let value = super::run_virtual_config(None, &config, &field, &Map::new())
        .expect("inline rhai resolver should run");

    assert_eq!(value, json!("displayName"));
}

#[test]
fn virtual_field_matches_normalized_names_and_columns() {
    let config = demo_config(vec![
        virtual_field("displayName", "firstName,lastName", json!({})),
        virtual_field("skuTitle", "spec_title", json!({})),
    ]);
    let registry = resolve_demo(config);
    let entity = registry.resolve("demo:item").expect("registered demo entity");

    assert_eq!(
        super::virtual_field(entity, "display.name").expect("dot alias").name,
        "displayName"
    );
    assert_eq!(
        super::virtual_field(entity, "display_name").expect("snake alias").name,
        "displayName"
    );
    assert_eq!(
        super::virtual_field(entity, "displayName").expect("camel alias").name,
        "displayName"
    );
    assert_eq!(
        super::virtual_field(entity, "firstName,lastName").expect("column alias").name,
        "displayName"
    );
    assert_eq!(
        super::virtual_field(entity, "sku_title").expect("snake from camel").name,
        "skuTitle"
    );
    assert!(super::virtual_field(entity, "missing").is_none());
}

#[test]
fn resolve_virtual_field_value_runs_registered_string_resolver() {
    let mut config =
        demo_config(vec![virtual_field("displayName", "firstName,lastName", json!("fullName"))]);
    config.extra.insert(
        "virtuals".to_string(),
        json!({
            "fullName": {
                "engine": "rhai",
                "script": "fn fullName(row, context) { row[\"firstName\"] + \" \" + row[\"lastName\"] + \" :: \" + context[\"comment\"] }"
            }
        }),
    );
    let registry = resolve_demo(config);
    let entity = registry.resolve("demo:item").expect("registered demo entity");
    let field = super::virtual_field(entity, "displayName").expect("virtual field");
    let row = Map::from_iter([
        ("firstName".to_string(), json!("Ada")),
        ("lastName".to_string(), json!("Lovelace")),
    ]);

    let value =
        super::resolve_virtual_field_value(entity, field, &row).expect("registered resolver value");

    assert_eq!(value, json!("Ada Lovelace :: displayName comment"));
}

#[test]
fn resolve_virtual_field_value_runs_object_resolver_reference_and_inline_config() {
    let mut referenced = virtual_field(
        "referencedName",
        "firstName,lastName",
        json!({
            "resolver": "fullName"
        }),
    );
    referenced.comment = Some("referenced comment".to_string());
    let inline = virtual_field(
        "inlineName",
        "firstName,lastName",
        json!({
            "engine": "rhai",
            "function": "resolve",
            "script": "fn resolve(row, context) { context[\"label\"] + \":\" + row[\"firstName\"] }"
        }),
    );
    let mut config = demo_config(vec![referenced, inline]);
    config.extra.insert(
        "virtuals".to_string(),
        json!({
            "fullName": {
                "engine": "rhai",
                "function": "compose",
                "script": "fn compose(row, context) { row[\"firstName\"] + \" \" + row[\"lastName\"] + \" :: \" + context[\"comment\"] }"
            }
        }),
    );
    let registry = resolve_demo(config);
    let entity = registry.resolve("demo:item").expect("registered demo entity");
    let row = Map::from_iter([
        ("firstName".to_string(), json!("Grace")),
        ("lastName".to_string(), json!("Hopper")),
    ]);

    let referenced_value = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "referencedName").expect("referenced field"),
        &row,
    )
    .expect("referenced resolver value");
    let inline_value = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "inlineName").expect("inline field"),
        &row,
    )
    .expect("inline resolver value");

    assert_eq!(referenced_value, json!("Grace Hopper :: referenced comment"));
    assert_eq!(inline_value, json!("inlineName label:Grace"));
}

#[test]
fn resolve_virtual_field_value_prefers_business_virtual_field() {
    let config = ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![base_field("specTitle", "spec_title")],
        virtual_fields: vec![virtual_field("skuTitle", "spec_title", json!(42))],
        ..ApiQlEntityConfig::default()
    };
    let registry = ApiQlEntityRegistry::from_configs(vec![config]).expect("valid product config");
    let entity = registry.resolve("product:product").expect("registered product entity");
    let field = super::virtual_field(entity, "sku_title").expect("business virtual field");
    let row = Map::from_iter([(
        "spec_title".to_string(),
        json!("[\"> Color: Red\", \"> Size: Large\"]"),
    )]);

    let value =
        super::resolve_virtual_field_value(entity, field, &row).expect("business resolver value");

    assert_eq!(value, json!("Red Large"));
}

#[test]
fn resolve_virtual_field_value_reports_missing_unknown_and_invalid_resolvers() {
    let no_resolver = ApiQlFieldConfig {
        name: "missingResolver".to_string(),
        column: Some("firstName".to_string()),
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    };
    let unknown_string = virtual_field("unknownString", "firstName", json!("missing"));
    let unknown_object =
        virtual_field("unknownObject", "firstName", json!({ "resolver": "missing" }));
    let invalid_scalar = virtual_field("invalidScalar", "firstName", json!(false));
    let config = demo_config(vec![no_resolver, unknown_string, unknown_object, invalid_scalar]);
    let registry = resolve_demo(config);
    let entity = registry.resolve("demo:item").expect("registered demo entity");
    let row = Map::new();

    let missing = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "missingResolver").expect("missing resolver field"),
        &row,
    )
    .expect_err("missing resolver should fail");
    assert_eq!(
        missing.to_string(),
        "ApiQL virtual field resolver is not configured: missingResolver"
    );

    let unknown = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "unknownString").expect("unknown resolver field"),
        &row,
    )
    .expect_err("unknown string resolver should fail");
    assert_eq!(unknown.to_string(), "Unknown ApiQL virtual resolver: missing");

    let unknown_object = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "unknownObject").expect("unknown object field"),
        &row,
    )
    .expect_err("unknown object resolver should fail");
    assert_eq!(unknown_object.to_string(), "Unknown ApiQL virtual resolver: missing");

    let invalid = super::resolve_virtual_field_value(
        entity,
        super::virtual_field(entity, "invalidScalar").expect("invalid resolver field"),
        &row,
    )
    .expect_err("invalid scalar resolver should fail");
    assert_eq!(invalid.to_string(), "Invalid ApiQL virtual resolver config");
}

#[test]
fn run_virtual_config_reads_script_files_and_reports_file_errors() {
    let temp = tempfile::tempdir().expect("temp dir");
    let script_path = temp.path().join("resolver.rhai");
    std::fs::write(
        &script_path,
        "fn resolve(row, context) { row[\"firstName\"] + \" from \" + context[\"name\"] }",
    )
    .expect("write resolver script");
    let field = virtual_field_with_resolver(json!({}));
    let row = Map::from_iter([("firstName".to_string(), json!("Ada"))]);

    let file_value = super::run_virtual_config(
        None,
        &json!({ "function": "resolve", "file": script_path }),
        &field,
        &row,
    )
    .expect("file resolver should run");
    assert_eq!(file_value, json!("Ada from displayName"));

    let missing_file = super::run_virtual_config(
        None,
        &json!({ "function": "resolve", "path": temp.path().join("missing.rhai") }),
        &field,
        &row,
    )
    .expect_err("missing file should fail");
    assert!(missing_file.to_string().contains("Failed to read ApiQL rhai virtual resolver file"));
}

#[test]
fn run_virtual_config_reports_shape_function_and_rhai_errors() {
    let field = virtual_field_with_resolver(json!({}));
    let row = Map::new();

    let non_object = super::run_virtual_config(None, &json!("resolve"), &field, &row)
        .expect_err("non-object config should fail");
    assert_eq!(non_object.to_string(), "ApiQL virtual resolver config must be an object");

    let missing_function = super::run_virtual_config(
        None,
        &json!({ "script": "fn resolve(row, context) { 1 }" }),
        &field,
        &row,
    )
    .expect_err("missing function should fail");
    assert_eq!(missing_function.to_string(), "ApiQL rhai virtual resolver requires function");

    let invalid_script = super::run_virtual_config(
        None,
        &json!({ "function": "resolve", "script": "fn resolve(" }),
        &field,
        &row,
    )
    .expect_err("invalid script should fail");
    assert!(invalid_script.to_string().contains("Invalid ApiQL rhai virtual resolver script"));

    let failed_call = super::run_virtual_config(
        None,
        &json!({ "function": "missing", "script": "fn resolve(row, context) { 1 }" }),
        &field,
        &row,
    )
    .expect_err("missing function call should fail");
    assert!(failed_call.to_string().contains("ApiQL rhai virtual resolver failed"));
}

#[test]
fn run_virtual_config_maps_unit_and_prepare_sku_title_results() {
    let field = virtual_field_with_resolver(json!({}));
    let row =
        Map::from_iter([("spec".to_string(), json!(["> Material: Cotton", "> Fit: Relaxed"]))]);

    let unit = super::run_virtual_config(
        None,
        &json!({ "function": "resolve", "script": "fn resolve(row, context) { () }" }),
        &field,
        &row,
    )
    .expect("unit result should map to null");
    assert_eq!(unit, Value::Null);

    let sku_title = super::run_virtual_config(
        None,
        &json!({
            "function": "resolve",
            "script": "fn resolve(row, context) { prepare_sku_title(row[\"spec\"]) }"
        }),
        &field,
        &row,
    )
    .expect("registered helper should run");
    assert_eq!(sku_title, json!("Cotton Relaxed"));
}

#[test]
fn run_virtual_config_rejects_unsupported_or_incomplete_resolvers() {
    let field = virtual_field_with_resolver(json!({}));

    let unsupported = super::run_virtual_config(
        None,
        &json!({ "engine": "javascript", "function": "resolve", "script": "ignored" }),
        &field,
        &Map::new(),
    )
    .expect_err("unsupported engine should fail");
    assert_eq!(unsupported.to_string(), "Unsupported ApiQL virtual resolver engine: javascript");

    let missing_script =
        super::run_virtual_config(None, &json!({ "function": "resolve" }), &field, &Map::new())
            .expect_err("missing script should fail");
    assert_eq!(missing_script.to_string(), "ApiQL rhai virtual resolver requires script or file");
}

#[test]
fn virtual_helpers_normalize_aliases_and_skip_blank_config_strings() {
    assert_eq!(super::to_snake_case("HTTPDisplay Name"), "http_display_name");
    assert_eq!(super::to_snake_case("already__snake"), "already_snake");
    assert_eq!(super::to_snake_case("Field2Value"), "field2_value");
    assert_eq!(super::to_camel_case("display-name value"), "displayNameValue");
    assert_eq!(super::to_camel_case("alreadyCamel"), "alreadyCamel");

    let config = json!({ "function": "  ", "script": " value " });
    let object = config.as_object().expect("object config");
    assert_eq!(super::config_string(object, "function"), None);
    assert_eq!(super::config_string(object, "script"), Some("value"));
}
