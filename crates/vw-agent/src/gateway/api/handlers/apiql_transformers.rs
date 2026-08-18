//! ApiQL PHP-compatible transformers.
//!
//! The first supported transformer is `type = "api"`. It batches source IDs
//! from the current rows, queries another registered ApiQL entity in list-only
//! style, then merges the related fields back into the original row.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::future::try_join_all;
use serde_json::{Map, Value};
use vw_api_types::apiql::{ApiQlFieldConfig, ApiQlListMode};
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;

use super::connections::ApiQlConnection;
use super::handlers::load_connection;
use super::params::{ApiQlFieldNameCase, ApiQlReadParams};
use super::runtime::run_apiql_query_base;
use super::schema::{ApiQlEntityRegistry, ApiQlResolvedEntity};

#[derive(Debug, Clone)]
struct ApiQlTransformer {
    kind: String,
    args: BTreeMap<String, ApiTransformerArg>,
}

#[derive(Debug, Clone)]
struct ApiTransformerArg {
    api: String,
    source_key: String,
    target_key: String,
    columns: Vec<String>,
    params: Map<String, Value>,
    has_many: bool,
}

#[derive(Debug, Clone, Copy)]
struct TransformerApiTarget<'a> {
    entity: ApiQlResolvedEntity<'a>,
    mode: ApiQlListMode,
}

pub(crate) fn transformer_source_fields(
    entity: ApiQlResolvedEntity<'_>,
    transformer: &Value,
) -> Result<Vec<String>, ApiError> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for transformer in parse_transformers(transformer)? {
        if transformer.kind != "api" {
            return Err(ApiError::bad_request(format!(
                "Unsupported ApiQL transformer type: {}",
                transformer.kind
            )));
        }
        for arg in transformer.args.values() {
            let Some(field_name) =
                resolve_entity_field_name(entity, &normalize_source_key(&arg.source_key))
            else {
                return Err(ApiError::bad_request(format!(
                    "Unknown ApiQL transformer source_key: {}",
                    arg.source_key
                )));
            };
            if seen.insert(field_name.clone()) {
                fields.push(field_name);
            }
        }
    }
    Ok(fields)
}

pub(crate) async fn apply_apiql_transformers(
    tenant: TenantScope,
    connection: ApiQlConnection,
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
    data: &mut Value,
) -> Result<(), ApiError> {
    let transformers = parse_transformers(&params.transformer)?;
    if transformers.is_empty() {
        return Ok(());
    }
    let Value::Array(items) = data else {
        return Ok(());
    };
    for transformer in transformers {
        match transformer.kind.as_str() {
            "api" => {
                apply_api_transformer(
                    tenant.clone(),
                    connection.clone(),
                    registry,
                    entity,
                    params,
                    items,
                    &transformer,
                )
                .await?;
            }
            _ => {
                return Err(ApiError::bad_request(format!(
                    "Unsupported ApiQL transformer type: {}",
                    transformer.kind
                )));
            }
        }
    }
    Ok(())
}

async fn apply_api_transformer(
    tenant: TenantScope,
    connection: ApiQlConnection,
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
    items: &mut [Value],
    transformer: &ApiQlTransformer,
) -> Result<(), ApiError> {
    let mut jobs = Vec::new();
    for (alias, arg) in &transformer.args {
        let source_values = collect_source_values(entity, params.field_case, items, arg)?;
        if source_values.is_empty() {
            continue;
        }
        let target = parse_api_target(registry, &arg.api)?;
        let child_params =
            build_child_params(target.entity, target.mode, params, arg, source_values)?;
        let alias = alias.clone();
        let arg = arg.clone();
        let parent_connection = connection.clone();
        let tenant = tenant.clone();
        jobs.push(async move {
            let child_connection = if target.entity.config().connection_id == parent_connection.id {
                parent_connection
            } else {
                load_connection(tenant, &target.entity.config().connection_id).await?
            };
            let child_response =
                run_apiql_query_base(child_connection, registry, target.entity, &child_params)
                    .await?;
            let rows = match child_response.response.data {
                Value::Array(rows) => rows,
                _ => Vec::new(),
            };
            Ok::<_, ApiError>((alias, target, arg, rows))
        });
    }
    let batches = try_join_all(jobs)
        .await?
        .into_iter()
        .filter(|(_, _, _, rows)| !rows.is_empty())
        .map(|(alias, target, arg, rows)| (alias, (target, arg, rows)))
        .collect::<BTreeMap<_, _>>();

    for item in items {
        let Value::Object(object) = item else {
            continue;
        };
        for (alias, (target, arg, rows)) in &batches {
            merge_api_rows(object, entity, target.entity, params, alias, arg, rows)?;
        }
    }
    Ok(())
}

fn collect_source_values(
    entity: ApiQlResolvedEntity<'_>,
    field_case: ApiQlFieldNameCase,
    items: &[Value],
    arg: &ApiTransformerArg,
) -> Result<Vec<Value>, ApiError> {
    let source_key = source_output_key(entity, &arg.source_key, field_case);
    let mut values = Vec::new();
    let mut seen = BTreeSet::new();
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        let Some(value) = object_value(object, &source_key, &arg.source_key) else {
            continue;
        };
        if is_zero_source_value(value) {
            continue;
        }
        let key = loose_value_key(value);
        if seen.insert(key) {
            values.push(value.clone());
        }
    }
    Ok(values)
}

fn build_child_params(
    target_entity: ApiQlResolvedEntity<'_>,
    mode: ApiQlListMode,
    parent_params: &ApiQlReadParams,
    arg: &ApiTransformerArg,
    source_values: Vec<Value>,
) -> Result<ApiQlReadParams, ApiError> {
    let target_field =
        resolve_entity_field_name(target_entity, &arg.target_key).ok_or_else(|| {
            ApiError::bad_request(format!(
                "Unknown ApiQL transformer target_key: {}",
                arg.target_key
            ))
        })?;
    let mut columns = Vec::new();
    let mut seen = BTreeSet::new();
    for column in arg.columns.iter().chain(std::iter::once(&arg.target_key)) {
        let field_name = resolve_entity_field_name(target_entity, column).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL transformer column: {column}"))
        })?;
        if seen.insert(field_name.clone()) {
            columns.push(Value::String(field_name));
        }
    }

    let mut child = default_child_params(parent_params, mode);
    child.column = Value::Array(columns);
    merge_child_params(&mut child, &arg.params)?;
    let where_object = child
        .where_
        .as_object_mut()
        .ok_or_else(|| ApiError::bad_request("ApiQL where must be an object"))?;
    where_object.insert(
        target_field,
        Value::Object(Map::from_iter([("in".to_string(), Value::Array(source_values))])),
    );
    Ok(child)
}

fn default_child_params(parent: &ApiQlReadParams, mode: ApiQlListMode) -> ApiQlReadParams {
    ApiQlReadParams {
        mode,
        page: Some(1),
        page_size: None,
        expect_page: false,
        limit: Some(10_000),
        column: Value::Null,
        key_column: Value::Null,
        order_by: Value::Null,
        group_by: Value::Null,
        where_: Value::Object(Map::new()),
        relation: Value::Null,
        join: Value::Null,
        cursor: Value::Null,
        transformer: Value::Null,
        transformer_keep_column: parent.transformer_keep_column,
        transformer_delimiter: parent.transformer_delimiter.clone(),
        field_case: parent.field_case,
        development: parent.development,
        only: matches!(mode, ApiQlListMode::IndexOnly),
        count: matches!(mode, ApiQlListMode::IndexCount),
    }
}

fn merge_child_params(
    child: &mut ApiQlReadParams,
    params: &Map<String, Value>,
) -> Result<(), ApiError> {
    for (key, value) in params {
        match key.as_str() {
            "column" => child.column = value.clone(),
            "key_column" | "keyColumn" => child.key_column = value.clone(),
            "order_by" | "orderBy" => child.order_by = value.clone(),
            "group_by" | "groupBy" => child.group_by = value.clone(),
            "where" => {
                let extra = value.as_object().ok_or_else(|| {
                    ApiError::bad_request("ApiQL transformer where must be an object")
                })?;
                let where_object = child
                    .where_
                    .as_object_mut()
                    .ok_or_else(|| ApiError::bad_request("ApiQL where must be an object"))?;
                where_object.extend(extra.clone());
            }
            "page"
            | "page_size"
            | "pageSize"
            | "limit"
            | "expect_page"
            | "expectPage"
            | "transformer"
            | "transformer_keep_column"
            | "transformerKeepColumn"
            | "transformer_delimiter"
            | "transformerDelimiter"
            | "field_case"
            | "fieldCase"
            | "development"
            | "dev"
            | "only"
            | "count"
            | "relation"
            | "join"
            | "cursor" => {}
            _ => {
                let where_object = child
                    .where_
                    .as_object_mut()
                    .ok_or_else(|| ApiError::bad_request("ApiQL where must be an object"))?;
                where_object.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(())
}

fn merge_api_rows(
    object: &mut Map<String, Value>,
    source_entity: ApiQlResolvedEntity<'_>,
    target_entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
    alias: &str,
    arg: &ApiTransformerArg,
    rows: &[Value],
) -> Result<(), ApiError> {
    let source_key = source_output_key(source_entity, &arg.source_key, params.field_case);
    let Some(source_value) = object_value(object, &source_key, &arg.source_key).cloned() else {
        return Ok(());
    };
    let target_key = output_field_key(target_entity, &arg.target_key, params.field_case);
    for row in rows {
        let Some(row_object) = row.as_object() else {
            continue;
        };
        let Some(target_value) = object_value(row_object, &target_key, &arg.target_key) else {
            continue;
        };
        if !loose_values_equal(&source_value, target_value) {
            continue;
        }
        if params.transformer_keep_column {
            merge_flat_row(object, alias, &params.transformer_delimiter, row_object, arg.has_many);
        } else {
            merge_nested_row(object, alias, row_object, arg.has_many);
        }
    }
    Ok(())
}

fn merge_flat_row(
    object: &mut Map<String, Value>,
    alias: &str,
    delimiter: &str,
    row: &Map<String, Value>,
    has_many: bool,
) {
    for (key, value) in row {
        let output_key = format!("{alias}{delimiter}{key}");
        if has_many {
            object.entry(output_key.clone()).or_insert_with(|| Value::Array(Vec::new()));
            if let Some(values) = object.get_mut(&output_key).and_then(Value::as_array_mut) {
                values.push(value.clone());
            }
        } else {
            object.insert(output_key, value.clone());
        }
    }
}

fn merge_nested_row(
    object: &mut Map<String, Value>,
    alias: &str,
    row: &Map<String, Value>,
    has_many: bool,
) {
    if has_many {
        object.entry(alias.to_string()).or_insert_with(|| Value::Array(Vec::new()));
        if let Some(values) = object.get_mut(alias).and_then(Value::as_array_mut) {
            values.push(Value::Object(row.clone()));
        }
    } else {
        object.insert(alias.to_string(), Value::Object(row.clone()));
    }
}

fn parse_transformers(value: &Value) -> Result<Vec<ApiQlTransformer>, ApiError> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(text) if text.trim().is_empty() => Ok(Vec::new()),
        Value::String(text) => {
            let decoded: Value = serde_json::from_str(text).map_err(|error| {
                ApiError::bad_request(format!("Invalid ApiQL transformer JSON: {error}"))
            })?;
            parse_transformers(&decoded)
        }
        Value::Array(items) => items.iter().map(parse_transformer).collect(),
        Value::Object(object) if object.contains_key("code") || object.contains_key("type") => {
            Ok(vec![parse_transformer(value)?])
        }
        Value::Object(object) => object.values().map(parse_transformer).collect(),
        _ => Err(ApiError::bad_request("Invalid ApiQL transformer config")),
    }
}

fn parse_transformer(value: &Value) -> Result<ApiQlTransformer, ApiError> {
    let object = value
        .as_object()
        .ok_or_else(|| ApiError::bad_request("ApiQL transformer must be an object"))?;
    required_string(object, "code")?;
    let kind = required_string(object, "type")?.to_ascii_lowercase();
    let args = object
        .get("args")
        .and_then(Value::as_object)
        .map(parse_api_args)
        .transpose()?
        .unwrap_or_default();
    Ok(ApiQlTransformer { kind, args })
}

fn parse_api_args(
    object: &Map<String, Value>,
) -> Result<BTreeMap<String, ApiTransformerArg>, ApiError> {
    object
        .iter()
        .map(|(alias, value)| parse_api_arg(alias, value).map(|arg| (alias.clone(), arg)))
        .collect()
}

fn parse_api_arg(alias: &str, value: &Value) -> Result<ApiTransformerArg, ApiError> {
    let object = value.as_object().ok_or_else(|| {
        ApiError::bad_request(format!("ApiQL transformer arg must be an object: {alias}"))
    })?;
    Ok(ApiTransformerArg {
        api: required_string(object, "api")?,
        source_key: required_string(object, "source_key")?,
        target_key: required_string(object, "target_key")?,
        columns: field_list(object.get("columns").unwrap_or(&Value::Null))?,
        params: object.get("params").and_then(Value::as_object).cloned().unwrap_or_default(),
        has_many: object.get("has_many").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn parse_api_target<'a>(
    registry: &'a ApiQlEntityRegistry,
    api: &str,
) -> Result<TransformerApiTarget<'a>, ApiError> {
    let (entity_key, mode) = api
        .split_once('/')
        .map_or((api, "list-only"), |(entity, mode)| (entity, mode.trim_matches('/')));
    let mode = match mode {
        "" | "index" | "list" => ApiQlListMode::Index,
        "list-only" | "only" => ApiQlListMode::IndexOnly,
        "list-page" | "page" => ApiQlListMode::IndexPage,
        "list-count" | "count" => ApiQlListMode::IndexCount,
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unsupported ApiQL transformer api mode: {mode}"
            )));
        }
    };
    Ok(TransformerApiTarget { entity: registry.resolve(entity_key)?, mode })
}

fn required_string(object: &Map<String, Value>, key: &str) -> Result<String, ApiError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::bad_request(format!("ApiQL transformer missing field: {key}")))
}

fn field_list(value: &Value) -> Result<Vec<String>, ApiError> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(text
            .split(',')
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(ToOwned::to_owned)
            .collect()),
        Value::Array(items) => items.iter().try_fold(Vec::new(), |mut fields, item| {
            fields.extend(field_list(item)?);
            Ok(fields)
        }),
        _ => Err(ApiError::bad_request("Invalid ApiQL transformer field list")),
    }
}

fn object_value<'a>(
    object: &'a Map<String, Value>,
    primary_key: &str,
    raw_key: &str,
) -> Option<&'a Value> {
    object
        .get(primary_key)
        .or_else(|| object.get(raw_key))
        .or_else(|| object.get(&normalize_source_key(raw_key)))
        .or_else(|| object.get(&to_snake_case(raw_key)))
        .or_else(|| object.get(&to_camel_case(raw_key)))
}

fn source_output_key(
    entity: ApiQlResolvedEntity<'_>,
    source_key: &str,
    field_case: ApiQlFieldNameCase,
) -> String {
    output_field_key(entity, &normalize_source_key(source_key), field_case)
}

fn output_field_key(
    entity: ApiQlResolvedEntity<'_>,
    field_key: &str,
    field_case: ApiQlFieldNameCase,
) -> String {
    resolve_entity_field(entity, field_key)
        .map(|field| match field_case {
            ApiQlFieldNameCase::Snake => to_snake_case(&field.name),
            ApiQlFieldNameCase::Camel => to_camel_case(&field.name),
        })
        .unwrap_or_else(|| match field_case {
            ApiQlFieldNameCase::Snake => to_snake_case(field_key),
            ApiQlFieldNameCase::Camel => to_camel_case(field_key),
        })
}

fn resolve_entity_field_name(entity: ApiQlResolvedEntity<'_>, field_key: &str) -> Option<String> {
    resolve_entity_field(entity, field_key).map(|field| field.name.clone())
}

fn resolve_entity_field<'a>(
    entity: ApiQlResolvedEntity<'a>,
    field_key: &str,
) -> Option<&'a ApiQlFieldConfig> {
    let normalized = normalize_source_key(field_key);
    let camel = to_camel_case(&normalized);
    entity.config().fields.iter().find(|field| {
        field.name == field_key
            || field.name == normalized
            || field.name == camel
            || to_snake_case(&field.name) == normalized
            || field.column.as_deref() == Some(field_key)
            || field.column.as_deref() == Some(normalized.as_str())
    })
}

fn normalize_source_key(value: &str) -> String {
    value.trim().replace('.', "_")
}

fn is_zero_source_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => matches!(text.trim(), "" | "0"),
        Value::Number(number) => {
            number.as_i64() == Some(0) || number.as_u64() == Some(0) || number.as_f64() == Some(0.0)
        }
        _ => false,
    }
}

fn loose_values_equal(left: &Value, right: &Value) -> bool {
    if let (Some(left), Some(right)) = (numeric_value(left), numeric_value(right)) {
        return (left - right).abs() < f64::EPSILON;
    }
    loose_value_key(left) == loose_value_key(right)
}

fn loose_value_key(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
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
#[path = "apiql_transformers_tests.rs"]
mod apiql_transformers_tests;
