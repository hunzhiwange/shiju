//! ApiQL virtual field resolver execution.

use std::path::{Path, PathBuf};

use rhai::{Dynamic, Engine, ImmutableString, Scope};
use serde_json::{Map, Value, json};
use vw_api_types::apiql::ApiQlFieldConfig;

use crate::app::agent::gateway::ApiError;

use super::entities::{prepare_sku_title, resolve_business_virtual_field};
use super::schema::ApiQlResolvedEntity;

const VIRTUALS_KEY: &str = "virtuals";

pub(crate) fn virtual_field<'a>(
    entity: ApiQlResolvedEntity<'a>,
    field_name: &str,
) -> Option<&'a ApiQlFieldConfig> {
    let normalized = field_name.trim().replace('.', "_");
    let camel = to_camel_case(&normalized);
    entity.config().virtual_fields.iter().find(|field| {
        field.name == field_name
            || field.name == normalized
            || field.name == camel
            || to_snake_case(&field.name) == normalized
            || field.column.as_deref() == Some(field_name)
            || field.column.as_deref() == Some(normalized.as_str())
    })
}

pub(crate) fn resolve_virtual_field_value(
    entity: ApiQlResolvedEntity<'_>,
    field: &ApiQlFieldConfig,
    row: &Map<String, Value>,
) -> Result<Value, ApiError> {
    if let Some(value) = resolve_business_virtual_field(entity, field, row)? {
        return Ok(value);
    }
    let resolver = virtual_resolver(field).ok_or_else(|| {
        ApiError::bad_request(format!(
            "ApiQL virtual field resolver is not configured: {}",
            field.name
        ))
    })?;
    match resolver {
        Value::String(resolver_name) => {
            let registered = registered_virtual(entity, resolver_name).ok_or_else(|| {
                ApiError::bad_request(format!("Unknown ApiQL virtual resolver: {resolver_name}"))
            })?;
            run_virtual_config(Some(resolver_name), registered, field, row)
        }
        Value::Object(config) => {
            if let Some(resolver_name) = config_string(config, "resolver") {
                let registered = registered_virtual(entity, resolver_name).ok_or_else(|| {
                    ApiError::bad_request(format!(
                        "Unknown ApiQL virtual resolver: {resolver_name}"
                    ))
                })?;
                return run_virtual_config(Some(resolver_name), registered, field, row);
            }
            run_virtual_config(None, resolver, field, row)
        }
        _ => Err(ApiError::bad_request("Invalid ApiQL virtual resolver config")),
    }
}

fn virtual_resolver(field: &ApiQlFieldConfig) -> Option<&Value> {
    field
        .meta
        .get("resolver")
        .or_else(|| field.extra.get("resolver"))
        .or_else(|| field.meta.get("virtual_resolver"))
        .or_else(|| field.extra.get("virtual_resolver"))
}

fn registered_virtual<'a>(
    entity: ApiQlResolvedEntity<'a>,
    resolver_name: &str,
) -> Option<&'a Value> {
    entity
        .config()
        .extra
        .get(VIRTUALS_KEY)
        .and_then(Value::as_object)
        .and_then(|virtuals| virtuals.get(resolver_name))
}

fn run_virtual_config(
    fallback_function: Option<&str>,
    config: &Value,
    field: &ApiQlFieldConfig,
    row: &Map<String, Value>,
) -> Result<Value, ApiError> {
    let object = config
        .as_object()
        .ok_or_else(|| ApiError::bad_request("ApiQL virtual resolver config must be an object"))?;
    let engine = config_string(object, "engine").unwrap_or("rhai");
    if engine != "rhai" {
        return Err(ApiError::bad_request(format!(
            "Unsupported ApiQL virtual resolver engine: {engine}"
        )));
    }
    let function = config_string(object, "function")
        .or(fallback_function)
        .ok_or_else(|| ApiError::bad_request("ApiQL rhai virtual resolver requires function"))?;
    let script = resolver_script(object)?;
    run_rhai_virtual(&script, function, field, row)
}

fn resolver_script(config: &Map<String, Value>) -> Result<String, ApiError> {
    if let Some(script) = config_string(config, "script") {
        return Ok(script.to_string());
    }
    let path = config_string(config, "file").or_else(|| config_string(config, "path")).ok_or_else(
        || ApiError::bad_request("ApiQL rhai virtual resolver requires script or file"),
    )?;
    std::fs::read_to_string(resolve_script_path(path)).map_err(|error| {
        ApiError::bad_request(format!("Failed to read ApiQL rhai virtual resolver file: {error}"))
    })
}

fn resolve_script_path(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
}

fn run_rhai_virtual(
    script: &str,
    function: &str,
    field: &ApiQlFieldConfig,
    row: &Map<String, Value>,
) -> Result<Value, ApiError> {
    let mut engine = Engine::new();
    engine.set_max_operations(20_000);
    engine.set_max_call_levels(32);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine.register_fn("prepare_sku_title", rhai_prepare_sku_title);

    let ast = engine.compile(script).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL rhai virtual resolver script: {error}"))
    })?;
    let rhai_row = rhai::serde::to_dynamic(Value::Object(row.clone())).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL virtual resolver row: {error}"))
    })?;
    let rhai_context = rhai::serde::to_dynamic(virtual_context(field)).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL virtual resolver context: {error}"))
    })?;
    let mut scope = Scope::new();
    let result =
        engine.call_fn::<Dynamic>(&mut scope, &ast, function, (rhai_row, rhai_context)).map_err(
            |error| ApiError::bad_request(format!("ApiQL rhai virtual resolver failed: {error}")),
        )?;
    dynamic_to_json(result)
}

fn virtual_context(field: &ApiQlFieldConfig) -> Value {
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
    Value::Object(context)
}

fn dynamic_to_json(value: Dynamic) -> Result<Value, ApiError> {
    if value.is_unit() {
        return Ok(Value::Null);
    }
    rhai::serde::from_dynamic(&value).map_err(|error| {
        ApiError::bad_request(format!("Invalid ApiQL rhai virtual resolver result: {error}"))
    })
}

fn config_string<'a>(config: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    config.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn rhai_prepare_sku_title(value: Dynamic) -> String {
    let spec_title = dynamic_to_plain_string(&value);
    prepare_sku_title(&spec_title).unwrap_or_default()
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

fn to_snake_case(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    for (index, ch) in chars.iter().copied().enumerate() {
        if matches!(ch, '-' | ' ') {
            push_separator(&mut output);
            continue;
        }
        if ch == '_' {
            push_separator(&mut output);
            continue;
        }
        if ch.is_ascii_uppercase() {
            let previous = index.checked_sub(1).and_then(|previous| chars.get(previous)).copied();
            let next = chars.get(index + 1).copied();
            let needs_separator = previous.is_some_and(|previous| {
                previous != '_'
                    && (previous.is_ascii_lowercase()
                        || previous.is_ascii_digit()
                        || next.is_some_and(|next| next.is_ascii_lowercase()))
            });
            if needs_separator {
                push_separator(&mut output);
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

fn to_camel_case(value: &str) -> String {
    if !value.contains('_') && !value.contains('-') && !value.contains(' ') {
        return value.to_string();
    }
    let mut output = String::with_capacity(value.len());
    let mut upper_next = false;
    for ch in value.chars() {
        if matches!(ch, '_' | '-' | ' ') {
            upper_next = !output.is_empty();
            continue;
        }
        if output.is_empty() {
            output.push(ch.to_ascii_lowercase());
        } else if upper_next {
            output.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn push_separator(output: &mut String) {
    if !output.is_empty() && !output.ends_with('_') {
        output.push('_');
    }
}

#[cfg(test)]
#[path = "apiql_virtuals_tests.rs"]
mod apiql_virtuals_tests;
