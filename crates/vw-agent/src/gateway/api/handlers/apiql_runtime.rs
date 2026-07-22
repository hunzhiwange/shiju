//! ApiQL 查询运行时。
//!
//! 运行时负责调用 SQL 编译器、复用 AI-DATA 只读执行桥接，并把结果适配为
//! PHP ApiQL 的 `data` + 可选 `page` 响应形状。

use serde_json::{Map, Value, json};
use vw_api_types::apiql::{ApiQlFieldConfig, ApiQlListMode, ApiQlPageDto, ApiQlResponse};
use vw_api_types::data::{AiDataConnectionDto, AiDataCountMode, AiDataQueryResponse};
use vw_api_types::id::TenantScope;

use crate::app::agent::gateway::ApiError;

use super::data_bridge::{ApiQlDataQuery, execute_apiql_data_query};
use super::formatters::{field_format, format_field_value};
use super::params::{ApiQlFieldNameCase, ApiQlReadParams};
use super::schema::{ApiQlEntityRegistry, ApiQlResolvedEntity};
use super::sql::{ApiQlSqlQuery, build_apiql_sql};
use super::transformers::apply_apiql_transformers;
use super::virtuals::{resolve_virtual_field_value, virtual_field};

const PAGE_COMMENT: &str = "分页信息";
const DATA_COMMENT: &str = "数据列表";
const NEXT_CURSOR_COMMENT: &str = "流式分页下一组数据游标值";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApiQlRuntimeResponse {
    pub(crate) response: ApiQlResponse<Value>,
    pub(crate) next_cursor: Option<String>,
}

pub(crate) async fn run_apiql_query(
    tenant: TenantScope,
    connection: AiDataConnectionDto,
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    let mut response = run_apiql_query_base(connection.clone(), registry, entity, params).await?;
    apply_apiql_transformers(
        tenant,
        connection,
        registry,
        entity,
        params,
        &mut response.response.data,
    )
    .await?;
    Ok(response)
}

pub(crate) async fn run_apiql_query_base(
    connection: AiDataConnectionDto,
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    if connection.id != entity.config().connection_id {
        return Err(ApiError::bad_request(format!(
            "ApiQL entity requires connection_id: {}",
            entity.config().connection_id
        )));
    }

    let compiled = build_apiql_sql(registry, entity, params)?;
    let data_response = execute_apiql_data_query(
        connection,
        ApiQlDataQuery {
            sql: compiled.sql.clone(),
            count_sql: compiled.count_sql.clone(),
            params: compiled.params.clone(),
            page: compiled.page,
            limit: compiled.limit,
            count_mode: compiled.count_mode.clone(),
            order_by: compiled.order_by.clone(),
            debug: false,
        },
    )
    .await
    .map_err(ApiError::internal)?;

    adapt_apiql_response(params, entity, &compiled, data_response)
}

fn adapt_apiql_response(
    params: &ApiQlReadParams,
    entity: ApiQlResolvedEntity<'_>,
    compiled: &ApiQlSqlQuery,
    data_response: AiDataQueryResponse,
) -> Result<ApiQlRuntimeResponse, ApiError> {
    if matches!(compiled.count_mode, AiDataCountMode::Only) {
        return Ok(ApiQlRuntimeResponse {
            response: build_response(json!(data_response.page.total_record), None, None, params),
            next_cursor: None,
        });
    }

    let next_cursor = build_next_cursor(&data_response, &compiled.cursor_fields, entity, params);
    let page = if params.expect_page && !params.only && !params.count {
        let without_total = matches!(params.mode, ApiQlListMode::IndexPage);
        Some(ApiQlPageDto {
            per_page: data_response.page.per_page,
            current_page: data_response.page.current_page,
            total_page: (!without_total).then_some(u64::from(data_response.page.total_page)),
            total_record: (!without_total).then_some(data_response.page.total_record),
            total_macro: if without_total { 0 } else { data_response.page.total_record },
            from: data_response.page.from,
            to: (data_response.page.to > 0).then_some(data_response.page.to),
        })
    } else {
        None
    };
    let data = Value::Array(adapt_items(entity, data_response.items, params, compiled)?);

    Ok(ApiQlRuntimeResponse {
        response: build_response(data, page, Some(next_cursor.clone().unwrap_or_default()), params),
        next_cursor,
    })
}

fn build_response(
    data: Value,
    page: Option<ApiQlPageDto>,
    next_cursor: Option<String>,
    params: &ApiQlReadParams,
) -> ApiQlResponse<Value> {
    let has_page = page.is_some();
    let has_next_cursor = next_cursor.is_some();
    ApiQlResponse {
        data,
        page,
        page_comment: (params.development && has_page).then(|| PAGE_COMMENT.to_string()),
        data_comment: params.development.then(|| DATA_COMMENT.to_string()),
        next_cursor_comment: (params.development && has_next_cursor)
            .then(|| NEXT_CURSOR_COMMENT.to_string()),
        next_cursor,
    }
}

fn build_next_cursor(
    response: &AiDataQueryResponse,
    fields: &[String],
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
) -> Option<String> {
    if fields.is_empty() || !response.has_next_page {
        return None;
    }
    let object = response.items.last()?.as_object()?;
    let mut cursor = Map::new();
    for field in fields {
        cursor.insert(
            output_field_name(entity, field, params.field_case),
            object.get(field)?.clone(),
        );
    }
    serde_json::to_string(&Value::Object(cursor)).ok()
}

fn adapt_items(
    entity: ApiQlResolvedEntity<'_>,
    items: Vec<Value>,
    params: &ApiQlReadParams,
    compiled: &ApiQlSqlQuery,
) -> Result<Vec<Value>, ApiError> {
    items.into_iter().map(|item| adapt_item(entity, item, params, compiled)).collect()
}

pub(super) fn adapt_item(
    entity: ApiQlResolvedEntity<'_>,
    item: Value,
    params: &ApiQlReadParams,
    compiled: &ApiQlSqlQuery,
) -> Result<Value, ApiError> {
    let Value::Object(object) = item else {
        return Ok(item);
    };
    let mut adapted = Map::new();
    for (field_name, value) in object {
        let output_name = output_field_name(entity, &field_name, params.field_case);
        let field = regular_field(entity, &field_name);
        adapted.insert(output_name.clone(), value);
        if params.development
            && let Some(field) = field
            && let Some(comment) = field_comment(field)
        {
            adapted.insert(format!("#{output_name}"), Value::String(comment));
        }
        if let Some(field) = field
            && let Some(format_name) = field_format(field)
        {
            let formatted_name = formatted_output_field_name(&output_name, params.field_case);
            let formatted_value =
                format_field_value(entity, field, format_name, adapted.get(&output_name))?;
            adapted.insert(formatted_name.clone(), formatted_value);
            if params.development
                && let Some(comment) =
                    field_comment(field).map(|comment| format!("{comment}「formated」"))
            {
                adapted.insert(format!("#{formatted_name}"), Value::String(comment));
            }
        }
        if let Some(field) = field
            && let Some(enum_map) = enum_map(field)
        {
            let enum_name = enum_output_field_name(&output_name, params.field_case);
            let enum_value = enum_label_for_value(enum_map, adapted.get(&output_name));
            adapted.insert(enum_name.clone(), enum_value.unwrap_or(Value::Null));
            if params.development
                && let Some(comment) =
                    field_comment(field).map(|comment| format!("{comment}「enum」"))
            {
                adapted.insert(format!("#{enum_name}"), Value::String(comment));
            }
        }
    }
    apply_virtual_fields(entity, &mut adapted, params, compiled)?;
    hide_internal_fields(entity, &mut adapted, params, &compiled.hidden_fields);
    Ok(Value::Object(adapted))
}

fn apply_virtual_fields(
    entity: ApiQlResolvedEntity<'_>,
    adapted: &mut Map<String, Value>,
    params: &ApiQlReadParams,
    compiled: &ApiQlSqlQuery,
) -> Result<(), ApiError> {
    for field_name in &compiled.virtual_fields {
        let field = virtual_field(entity, field_name).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL virtual field: {field_name}"))
        })?;
        let output_name = output_field_name_for_config(field, params.field_case);
        let value = resolve_virtual_field_value(entity, field, adapted)?;
        adapted.insert(output_name.clone(), value);
        if params.development
            && let Some(comment) = field_comment(field)
        {
            adapted.insert(format!("#{output_name}"), Value::String(comment));
        }
    }
    Ok(())
}

fn hide_internal_fields(
    entity: ApiQlResolvedEntity<'_>,
    adapted: &mut Map<String, Value>,
    params: &ApiQlReadParams,
    hidden_fields: &[String],
) {
    for field_name in hidden_fields {
        let output_name = output_field_name(entity, field_name, params.field_case);
        adapted.remove(&output_name);
        adapted.remove(&format!("#{output_name}"));
    }
}

fn regular_field<'a>(
    entity: ApiQlResolvedEntity<'a>,
    field_name: &str,
) -> Option<&'a ApiQlFieldConfig> {
    entity.config().fields.iter().find(|field| field.name == field_name)
}

pub(super) fn output_field_name(
    entity: ApiQlResolvedEntity<'_>,
    field_name: &str,
    field_case: ApiQlFieldNameCase,
) -> String {
    regular_field(entity, field_name)
        .map(|field| match field_case {
            ApiQlFieldNameCase::Snake => to_snake_case(&field.name),
            ApiQlFieldNameCase::Camel => to_camel_case(&field.name),
        })
        .unwrap_or_else(|| match field_case {
            ApiQlFieldNameCase::Snake => to_snake_case(field_name),
            ApiQlFieldNameCase::Camel => to_camel_case(field_name),
        })
}

pub(super) fn output_field_name_for_config(
    field: &ApiQlFieldConfig,
    field_case: ApiQlFieldNameCase,
) -> String {
    match field_case {
        ApiQlFieldNameCase::Snake => to_snake_case(&field.name),
        ApiQlFieldNameCase::Camel => to_camel_case(&field.name),
    }
}

fn field_comment(field: &ApiQlFieldConfig) -> Option<String> {
    let label = string_value(field.meta.get("label"))
        .or_else(|| field.label.as_deref().and_then(non_empty_string));
    let comment = string_value(field.meta.get("comment"))
        .or_else(|| field.comment.as_deref().and_then(non_empty_string));
    match (label, comment) {
        (Some(label), Some(comment)) if label != comment => Some(format!("{label} {comment}")),
        (Some(label), _) => Some(label),
        (_, Some(comment)) => Some(comment),
        _ => ["title", "description"].iter().find_map(|key| string_value(field.meta.get(*key))),
    }
}

fn string_value(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).and_then(non_empty_string)
}

fn formatted_output_field_name(field_name: &str, field_case: ApiQlFieldNameCase) -> String {
    match field_case {
        ApiQlFieldNameCase::Snake => format!("{field_name}_formated"),
        ApiQlFieldNameCase::Camel => format!("{field_name}Formated"),
    }
}

fn enum_map(field: &ApiQlFieldConfig) -> Option<&Map<String, Value>> {
    ["enum", "enums", "enum_values"]
        .iter()
        .find_map(|key| field.meta.get(*key).and_then(Value::as_object))
        .or_else(|| {
            ["enum", "enums", "enum_values"]
                .iter()
                .find_map(|key| field.extra.get(*key).and_then(Value::as_object))
        })
}

fn enum_output_field_name(field_name: &str, field_case: ApiQlFieldNameCase) -> String {
    match field_case {
        ApiQlFieldNameCase::Snake => format!("{field_name}_enum"),
        ApiQlFieldNameCase::Camel => format!("{field_name}Enum"),
    }
}

fn enum_label_for_value(enum_map: &Map<String, Value>, value: Option<&Value>) -> Option<Value> {
    let value = value?;
    enum_key_candidates(value).into_iter().find_map(|key| enum_map.get(&key)).map(enum_label_value)
}

pub(super) fn enum_key_candidates(value: &Value) -> Vec<String> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(|value| vec![value.to_string()])
            .or_else(|| number.as_u64().map(|value| vec![value.to_string()]))
            .or_else(|| number.as_f64().map(|value| vec![value.to_string()]))
            .unwrap_or_default(),
        Value::String(text) => vec![text.clone()],
        Value::Bool(value) => {
            let numeric = if *value { "1" } else { "0" };
            vec![numeric.to_string(), value.to_string()]
        }
        Value::Null | Value::Array(_) | Value::Object(_) => Vec::new(),
    }
}

pub(super) fn enum_label_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => object
            .get("label")
            .or_else(|| object.get("name"))
            .or_else(|| object.get("title"))
            .cloned()
            .unwrap_or_else(|| value.clone()),
        _ => value.clone(),
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn to_snake_case(value: &str) -> String {
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

pub(super) fn to_camel_case(value: &str) -> String {
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
