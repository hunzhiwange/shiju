//! ApiQL 字段格式化器执行层，支持内置名称和受限的 Rhai 自定义规则。
//!
//! ApiQL field formatter execution.
//!
//! Formatters are intentionally small and explicit. Legacy string names keep
//! working, while custom formatters can be registered in an entity `formatters`
//! object or declared inline on a field with `engine = "rhai"`.

use std::path::{Path, PathBuf};

use rhai::{Dynamic, Engine, FLOAT, INT, ImmutableString, Map as RhaiMap, Scope};
use serde_json::{Map, Value, json};
use vw_api_types::apiql::ApiQlFieldConfig;

use crate::app::agent::gateway::ApiError;

use super::schema::ApiQlResolvedEntity;

const FORMATTERS_KEY: &str = "formatters";

pub(crate) fn field_format(field: &ApiQlFieldConfig) -> Option<&Value> {
    field
        .column_struct
        .get("format")
        .or_else(|| field.meta.get("format"))
        .or_else(|| field.extra.get("format"))
}

pub(crate) fn format_field_value(
    entity: ApiQlResolvedEntity<'_>,
    field: &ApiQlFieldConfig,
    format_spec: &Value,
    value: Option<&Value>,
) -> Result<Value, ApiError> {
    match format_spec {
        Value::String(format_name) => {
            if let Some(registered) = registered_formatter(entity, format_name) {
                run_formatter_config(Some(format_name), registered, field, value)
            } else {
                format_legacy_value(format_name, value, field)
            }
        }
        Value::Object(config) => {
            if let Some(formatter_name) = config_string(config, "formatter") {
                let registered = registered_formatter(entity, formatter_name).ok_or_else(|| {
                    ApiError::bad_request(format!(
                        "Unknown ApiQL field formatter: {formatter_name}"
                    ))
                })?;
                return run_formatter_config(Some(formatter_name), registered, field, value);
            }
            run_formatter_config(None, format_spec, field, value)
        }
        _ => Err(ApiError::bad_request("Invalid ApiQL field format config")),
    }
}

fn registered_formatter<'a>(
    entity: ApiQlResolvedEntity<'a>,
    format_name: &str,
) -> Option<&'a Value> {
    entity
        .config()
        .extra
        .get(FORMATTERS_KEY)
        .and_then(Value::as_object)
        .and_then(|formatters| formatters.get(format_name))
}

fn run_formatter_config(
    fallback_function: Option<&str>,
    config: &Value,
    field: &ApiQlFieldConfig,
    value: Option<&Value>,
) -> Result<Value, ApiError> {
    let object = config
        .as_object()
        .ok_or_else(|| ApiError::bad_request("ApiQL formatter config must be an object"))?;
    let engine = config_string(object, "engine").unwrap_or("rhai");
    if engine != "rhai" {
        return Err(ApiError::bad_request(format!("Unsupported ApiQL formatter engine: {engine}")));
    }
    let function = config_string(object, "function")
        .or(fallback_function)
        .ok_or_else(|| ApiError::bad_request("ApiQL rhai formatter requires function"))?;
    let script = formatter_script(object)?;
    run_rhai_formatter(&script, function, field, value)
}

fn formatter_script(config: &Map<String, Value>) -> Result<String, ApiError> {
    if let Some(script) = config_string(config, "script") {
        return Ok(script.to_string());
    }
    let path = config_string(config, "file")
        .or_else(|| config_string(config, "path"))
        .ok_or_else(|| ApiError::bad_request("ApiQL rhai formatter requires script or file"))?;
    std::fs::read_to_string(resolve_formatter_path(path)).map_err(|error| {
        ApiError::bad_request(format!("Failed to read ApiQL rhai formatter file: {error}"))
    })
}

fn resolve_formatter_path(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
}

fn run_rhai_formatter(
    script: &str,
    function: &str,
    field: &ApiQlFieldConfig,
    value: Option<&Value>,
) -> Result<Value, ApiError> {
    let mut engine = Engine::new();
    engine.set_max_operations(20_000);
    engine.set_max_call_levels(32);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine.register_fn("decimal", rhai_decimal);
    engine.register_fn("image_url", rhai_image_url);

    let ast = engine.compile(script).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL rhai formatter script: {error}"))
    })?;
    let rhai_value =
        rhai::serde::to_dynamic(value.cloned().unwrap_or(Value::Null)).map_err(|error| {
            ApiError::bad_request(format!("Invalid ApiQL formatter value: {error}"))
        })?;
    let context = formatter_context(field);
    let rhai_context = rhai::serde::to_dynamic(context).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL formatter context: {error}"))
    })?;
    let mut scope = Scope::new();
    let result = engine
        .call_fn::<Dynamic>(&mut scope, &ast, function, (rhai_value, rhai_context))
        .map_err(|error| ApiError::bad_request(format!("ApiQL rhai formatter failed: {error}")))?;
    dynamic_to_json(result)
}

fn formatter_context(field: &ApiQlFieldConfig) -> Value {
    let mut context = Map::new();
    context.insert("name".to_string(), json!(field.name));
    if let Some(column) = &field.column {
        context.insert("column".to_string(), json!(column));
    }
    if let Some(label) = &field.label {
        context.insert("label".to_string(), json!(label));
    }
    if let Some(comment) = &field.comment {
        context.insert("comment".to_string(), json!(comment));
    }
    context.insert("meta".to_string(), json!(field.meta));
    context.insert("column_struct".to_string(), json!(field.column_struct));
    context.insert("extra".to_string(), json!(field.extra));
    if let Some(base_url) = format_base_url(field) {
        context.insert("attachments_url".to_string(), json!(base_url));
        context.insert("format_base_url".to_string(), json!(base_url));
        context.insert("base_url".to_string(), json!(base_url));
    }
    Value::Object(context)
}

fn dynamic_to_json(value: Dynamic) -> Result<Value, ApiError> {
    if value.is_unit() {
        return Ok(Value::Null);
    }
    rhai::serde::from_dynamic(&value).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL rhai formatter result: {error}"))
    })
}

fn config_string<'a>(config: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    config.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn format_legacy_value(
    format_name: &str,
    value: Option<&Value>,
    field: &ApiQlFieldConfig,
) -> Result<Value, ApiError> {
    match format_name {
        "format_decimal" => Ok(format_decimal_value(value)),
        "format_images" => Ok(format_images_value(value, field)),
        _ => Err(ApiError::bad_request(format!("Unsupported ApiQL field format: {format_name}"))),
    }
}

fn format_decimal_value(value: Option<&Value>) -> Value {
    let Some(number) = value.and_then(numeric_value) else {
        return Value::Null;
    };
    let rounded = format!("{number:.8}").parse::<f64>().unwrap_or(number);
    serde_json::Number::from_f64(rounded).map(Value::Number).unwrap_or(Value::Null)
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn format_images_value(value: Option<&Value>, field: &ApiQlFieldConfig) -> Value {
    let Some(path) = value.and_then(Value::as_str).map(str::trim) else {
        return Value::String(String::new());
    };
    if path.is_empty() {
        return Value::String(String::new());
    }
    let Some(base_url) = format_base_url(field) else {
        return Value::String(path.to_string());
    };
    Value::String(join_url(base_url, path))
}

fn format_base_url(field: &ApiQlFieldConfig) -> Option<&str> {
    ["attachments_url", "format_base_url", "base_url"]
        .iter()
        .find_map(|key| string_ref(field.meta.get(*key)))
        .or_else(|| {
            ["attachments_url", "format_base_url", "base_url"]
                .iter()
                .find_map(|key| string_ref(field.column_struct.get(*key)))
        })
        .or_else(|| {
            ["attachments_url", "format_base_url", "base_url"]
                .iter()
                .find_map(|key| string_ref(field.extra.get(*key)))
        })
}

fn string_ref(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn rhai_decimal(value: Dynamic, scale: INT, trim_zero: bool) -> Dynamic {
    let Some(number) = dynamic_to_f64(&value) else {
        return ().into();
    };
    let scale = scale.clamp(0, 16) as usize;
    let formatted = format!("{number:.scale$}");
    if !trim_zero {
        return formatted.into();
    }
    formatted.parse::<FLOAT>().unwrap_or(number as FLOAT).into()
}

fn rhai_image_url(value: Dynamic, context: Dynamic) -> String {
    let path = dynamic_to_plain_string(&value).trim().to_string();
    if path.is_empty() {
        return String::new();
    }
    let Some(base_url) = dynamic_context_string(&context, "attachments_url")
        .or_else(|| dynamic_context_string(&context, "format_base_url"))
        .or_else(|| dynamic_context_string(&context, "base_url"))
    else {
        return path;
    };
    join_url(&base_url, &path)
}

fn dynamic_context_string(context: &Dynamic, key: &str) -> Option<String> {
    let map = context.clone().try_cast::<RhaiMap>()?;
    map.get(key).map(dynamic_to_plain_string).and_then(|value| non_empty_string(&value))
}

fn dynamic_to_f64(value: &Dynamic) -> Option<f64> {
    if let Some(value) = value.clone().try_cast::<FLOAT>() {
        return Some(value as f64);
    }
    if let Some(value) = value.clone().try_cast::<INT>() {
        return Some(value as f64);
    }
    dynamic_to_plain_string(value).trim().parse::<f64>().ok()
}

fn dynamic_to_plain_string(value: &Dynamic) -> String {
    if value.is_unit() {
        return String::new();
    }
    if let Some(value) = value.clone().try_cast::<ImmutableString>() {
        return value.to_string();
    }
    if let Some(value) = value.clone().try_cast::<String>() {
        return value;
    }
    value.to_string()
}

fn join_url(base_url: &str, path: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'))
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "apiql_formatters_tests.rs"]
mod apiql_formatters_tests;
