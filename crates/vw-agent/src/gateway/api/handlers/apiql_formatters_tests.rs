// 覆盖 ApiQL 字段格式化器的解析、执行与错误处理。

use serde_json::{Value, json};
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};

use super::{field_format, format_field_value};
use crate::gateway::api::handlers::apiql::schema::ApiQlEntityRegistry;

fn registry(extra: serde_json::Map<String, Value>) -> ApiQlEntityRegistry {
    ApiQlEntityRegistry::from_configs(vec![ApiQlEntityConfig {
        entity_class: "demo:item".to_string(),
        connection_id: "main".to_string(),
        table: "items".to_string(),
        extra: extra.into_iter().collect(),
        ..ApiQlEntityConfig::default()
    }])
    .expect("valid entity config")
}

fn field() -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: "price".to_string(),
        column: Some("price_cents".to_string()),
        label: Some("Price".to_string()),
        comment: Some("Displayed price".to_string()),
        ..Default::default()
    }
}

// 字段别名、数据库值与分页参数由同一组夹具固定，以同时约束兼容转换和查询语义。
#[test]
fn field_format_prefers_column_struct_then_meta_then_extra() {
    let mut field = field();
    field.extra.insert("format".to_string(), json!("extra"));
    field.meta.insert("format".to_string(), json!("meta"));
    field.column_struct.insert("format".to_string(), json!("column"));
    assert_eq!(field_format(&field), Some(&json!("column")));

    field.column_struct.remove("format");
    assert_eq!(field_format(&field), Some(&json!("meta")));
    field.meta.remove("format");
    assert_eq!(field_format(&field), Some(&json!("extra")));
}

#[test]
fn inline_rhai_formatter_uses_registered_decimal_helper() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");
    let config = json!({
        "engine": "rhai",
        "function": "render",
        "script": "fn render(value, context) { decimal(value, 2, false) }"
    });

    let formatted = format_field_value(entity, &field(), &config, Some(&json!("1.2")))
        .expect("inline formatter should run");

    assert_eq!(formatted, json!("1.20"));
}

#[test]
fn registered_formatter_uses_its_name_as_fallback_function() {
    let registry = registry(
        json!({
            "formatters": {
                "double": {
                    "script": "fn double(value, context) { value * 2 }"
                }
            }
        })
        .as_object()
        .expect("object")
        .clone(),
    );
    let entity = registry.resolve("demo:item").expect("entity");

    let formatted = format_field_value(entity, &field(), &json!("double"), Some(&json!(4)))
        .expect("registered formatter should run");

    assert_eq!(formatted, json!(8));
}

#[test]
fn legacy_formatters_cover_decimal_images_and_unsupported_names() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");
    let mut image_field = field();
    image_field.meta.insert("attachments_url".to_string(), json!("https://cdn.example.test/root/"));

    assert_eq!(
        format_field_value(entity, &field(), &json!("format_decimal"), Some(&json!("1.234567891")))
            .expect("decimal string should format"),
        json!(1.23456789)
    );
    assert_eq!(
        format_field_value(entity, &field(), &json!("format_decimal"), Some(&json!("not-number")))
            .expect("invalid decimal should become null"),
        Value::Null
    );
    assert_eq!(
        format_field_value(
            entity,
            &image_field,
            &json!("format_images"),
            Some(&json!("/images/item.png")),
        )
        .expect("image path should join base url"),
        json!("https://cdn.example.test/root/images/item.png")
    );
    assert_eq!(
        format_field_value(entity, &field(), &json!("format_images"), Some(&json!("plain.png")))
            .expect("image path without base url should be preserved"),
        json!("plain.png")
    );

    let unsupported = format_field_value(entity, &field(), &json!("format_unknown"), None)
        .expect_err("unknown legacy formatter should fail");
    assert!(unsupported.to_string().contains("Unsupported ApiQL field format"));
}

#[test]
fn file_backed_rhai_formatter_uses_image_helper_and_context_base_url() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");
    let temp = tempfile::tempdir().expect("temp dir");
    let script_path = temp.path().join("formatter.rhai");
    std::fs::write(
        &script_path,
        r#"
            fn render(value, context) {
                image_url(value, context)
            }
        "#,
    )
    .expect("write formatter");

    let mut image_field = field();
    image_field.extra.insert("base_url".to_string(), json!("https://assets.example.test/api"));
    let formatted = format_field_value(
        entity,
        &image_field,
        &json!({ "engine": "rhai", "function": "render", "path": script_path }),
        Some(&json!("files/photo.jpg")),
    )
    .expect("file-backed formatter should run");

    assert_eq!(formatted, json!("https://assets.example.test/api/files/photo.jpg"));
}

#[test]
fn rhai_formatter_reports_compile_runtime_and_unit_result_edges() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");

    let compile_error = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render", "script": "fn render(value, context) {" }),
        None,
    )
    .expect_err("invalid script should fail");
    let runtime_error = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render", "script": "fn render(value, context) { missing(value) }" }),
        Some(&json!(1)),
    )
    .expect_err("runtime failure should be reported");
    let unit_result = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render", "script": "fn render(value, context) { print(\"quiet\"); debug(\"quiet\"); }" }),
        Some(&json!(1)),
    )
    .expect("unit return should become null");

    assert!(compile_error.to_string().contains("Invalid ApiQL rhai formatter script"));
    assert!(runtime_error.to_string().contains("ApiQL rhai formatter failed"));
    assert_eq!(unit_result, Value::Null);
}

#[test]
fn rhai_decimal_helper_trims_zeroes_and_accepts_numeric_values() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");
    let config = json!({
        "engine": "rhai",
        "function": "render",
        "script": "fn render(value, context) { decimal(value, 4, true) }"
    });

    assert_eq!(
        format_field_value(entity, &field(), &config, Some(&json!(4))).expect("int should format"),
        json!(4.0)
    );
    assert_eq!(
        format_field_value(entity, &field(), &config, Some(&json!(1.25)))
            .expect("float should format"),
        json!(1.25)
    );
    assert_eq!(
        format_field_value(entity, &field(), &config, Some(&json!("bad")))
            .expect("bad numeric input should return null"),
        Value::Null
    );
}

#[test]
fn formatter_edges_cover_registered_object_lookup_and_empty_image_paths() {
    let registry = registry(
        json!({
            "formatters": {
                "double": {
                    "script": "fn double(value, context) { value * 2 }"
                },
                "bad": true
            }
        })
        .as_object()
        .expect("object")
        .clone(),
    );
    let entity = registry.resolve("demo:item").expect("entity");

    assert_eq!(
        format_field_value(entity, &field(), &json!({ "formatter": "double" }), Some(&json!(5)))
            .expect("object formatter should use registered config"),
        json!(10)
    );
    assert_eq!(
        format_field_value(entity, &field(), &json!("format_decimal"), Some(&json!(2.5)))
            .expect("numeric decimal should format"),
        json!(2.5)
    );
    assert_eq!(
        format_field_value(entity, &field(), &json!("format_images"), Some(&json!("   ")))
            .expect("blank image path should return empty string"),
        json!("")
    );
    assert_eq!(
        format_field_value(entity, &field(), &json!("format_images"), Some(&json!(false)))
            .expect("non-string image path should return empty string"),
        json!("")
    );

    let bad_registered = format_field_value(entity, &field(), &json!("bad"), None)
        .expect_err("registered formatter config must be an object");
    let missing_script = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render" }),
        None,
    )
    .expect_err("formatter with function still needs script or file");
    let empty_image = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render", "script": "fn render(value, context) { image_url(\"\", context) }" }),
        Some(&json!("ignored")),
    )
    .expect("empty helper path should return empty string");
    let no_base_image = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "function": "render", "script": "fn render(value, context) { image_url(value, context) }" }),
        Some(&json!("plain.jpg")),
    )
    .expect("missing base url should preserve path");

    assert!(bad_registered.to_string().contains("formatter config must be an object"));
    assert!(missing_script.to_string().contains("requires script or file"));
    assert_eq!(empty_image, json!(""));
    assert_eq!(no_base_image, json!("plain.jpg"));
}

// 非法字段、运算符和解析器配置必须在生成查询或执行脚本前被拒绝。
#[test]
fn formatter_rejects_invalid_specs_engines_and_unknown_names() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");

    let invalid = format_field_value(entity, &field(), &json!(true), None)
        .expect_err("non-string and non-object config should fail");
    let engine = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "javascript", "function": "render", "script": "" }),
        None,
    )
    .expect_err("unsupported engine should fail");
    let unknown = format_field_value(entity, &field(), &json!({ "formatter": "missing" }), None)
        .expect_err("unknown registered formatter should fail");

    assert!(invalid.to_string().contains("Invalid ApiQL field format config"));
    assert!(engine.to_string().contains("Unsupported ApiQL formatter engine"));
    assert!(unknown.to_string().contains("Unknown ApiQL field formatter"));
}

#[test]
fn rhai_formatter_reports_missing_function_and_script_file() {
    let registry = registry(serde_json::Map::new());
    let entity = registry.resolve("demo:item").expect("entity");
    let temp = tempfile::tempdir().expect("temp dir");
    let missing_script = temp.path().join("missing.rhai");

    let missing_function = format_field_value(
        entity,
        &field(),
        &json!({ "engine": "rhai", "script": "fn render(value, context) { value }" }),
        None,
    )
    .expect_err("inline formatter requires a function");
    let missing_file = format_field_value(
        entity,
        &field(),
        &json!({
            "engine": "rhai",
            "function": "render",
            "file": missing_script.to_string_lossy()
        }),
        None,
    )
    .expect_err("missing script file should fail");

    assert!(missing_function.to_string().contains("requires function"));
    assert!(missing_file.to_string().contains("Failed to read ApiQL rhai formatter file"));
}
