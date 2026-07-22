//! ApiQL 只读查询 SQL 编译。
//!
//! 本模块只把已归一化参数和登记实体元数据编译为只读 SQL 模板，
//! 实际 SQL 安全检查、分页追加和数据库执行继续复用 AI-DATA runtime。

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use vw_api_types::apiql::{ApiQlFieldConfig, ApiQlListMode};
use vw_api_types::data::AiDataCountMode;

use crate::app::agent::gateway::ApiError;

use super::params::{ApiQlFieldNameCase, ApiQlReadParams};
use super::schema::{ApiQlEntityRegistry, ApiQlOrderBy, ApiQlResolvedEntity, ApiQlSortDirection};
use super::transformers::transformer_source_fields;

const BASE_ALIAS: &str = "apiql_base";
const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApiQlSqlQuery {
    pub(crate) sql: String,
    pub(crate) count_sql: Option<String>,
    pub(crate) params: BTreeMap<String, Value>,
    pub(crate) page: u32,
    pub(crate) limit: u32,
    pub(crate) count_mode: AiDataCountMode,
    pub(crate) order_by: Option<String>,
    pub(crate) cursor_fields: Vec<String>,
    pub(crate) virtual_fields: Vec<String>,
    pub(crate) hidden_fields: Vec<String>,
}

pub(crate) fn build_apiql_sql(
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
) -> Result<ApiQlSqlQuery, ApiError> {
    let order = parse_order_by(entity, &params.order_by)?;
    let select = resolve_select_fields(entity, params, &order)?;
    let joins = build_join_sql(registry, entity, &params.join, &params.relation)?;
    let mut builder = SqlBuildState::default();
    let mut conditions = build_where_conditions(entity, &params.where_, &mut builder)?;
    if let Some(cursor_condition) =
        build_cursor_condition(entity, &params.cursor, &order, params.field_case, &mut builder)?
    {
        conditions.push(cursor_condition);
    }

    let mut sql = format!(
        "SELECT {} FROM {} AS {BASE_ALIAS}",
        select_clause(entity, &select.fields)?,
        table_name(&entity.config().table)?
    );
    for join in joins {
        sql.push(' ');
        sql.push_str(&join);
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    if let Some(group_by) = group_by_clause(entity, &params.group_by)? {
        sql.push_str(" GROUP BY ");
        sql.push_str(&group_by);
    }

    Ok(ApiQlSqlQuery {
        sql,
        count_sql: None,
        params: builder.params,
        page: params.page.unwrap_or(1).max(1),
        limit: params.limit.or(params.page_size).unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        count_mode: match (params.count, params.expect_page, params.mode) {
            (true, _, _) => AiDataCountMode::Only,
            (false, true, ApiQlListMode::IndexPage) | (false, false, _) => {
                AiDataCountMode::Disabled
            }
            (false, true, _) => AiDataCountMode::Enabled,
        },
        order_by: order_by_clause(&order),
        cursor_fields: order.into_iter().map(|item| item.field).collect(),
        virtual_fields: select.virtual_fields,
        hidden_fields: select.hidden_fields,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiQlSelectFields {
    fields: Vec<String>,
    virtual_fields: Vec<String>,
    hidden_fields: Vec<String>,
}

#[derive(Default)]
struct SqlBuildState {
    params: BTreeMap<String, Value>,
    search_index: usize,
}

fn resolve_select_fields(
    entity: ApiQlResolvedEntity<'_>,
    params: &ApiQlReadParams,
    order: &[ApiQlOrderBy],
) -> Result<ApiQlSelectFields, ApiError> {
    let requested = field_list(&params.column)?;
    let mut fields = if requested.is_empty() {
        entity.config().fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>()
    } else {
        requested
    };
    fields.extend(field_list(&params.key_column)?);
    fields.extend(transformer_source_fields(entity, &params.transformer)?);

    let mut seen = BTreeSet::new();
    let mut resolved = Vec::new();
    let mut direct_regular = BTreeSet::new();
    let mut virtual_dependencies = BTreeSet::new();
    let mut virtual_seen = BTreeSet::new();
    let mut virtual_fields = Vec::new();
    for field in fields {
        if let Some(field_config) = regular_field(entity, &field) {
            direct_regular.insert(field_config.name.clone());
            if seen.insert(field_config.name.clone()) {
                resolved.push(field_config.name.clone());
            }
            continue;
        }
        if let Some(virtual_field) = virtual_field(entity, &field) {
            if virtual_seen.insert(virtual_field.name.clone()) {
                virtual_fields.push(virtual_field.name.clone());
            }
            for dependency in entity.virtual_field_dependencies(&virtual_field.name)? {
                virtual_dependencies.insert(dependency.to_string());
                if seen.insert(dependency.to_string()) {
                    resolved.push(dependency.to_string());
                }
            }
            continue;
        }
        return Err(ApiError::bad_request(format!("Unknown ApiQL column field: {field}")));
    }

    for order_item in order {
        if seen.insert(order_item.field.clone()) {
            resolved.push(order_item.field.clone());
            virtual_dependencies.insert(order_item.field.clone());
        }
    }

    let hidden_fields = virtual_dependencies
        .into_iter()
        .filter(|field| !direct_regular.contains(field))
        .collect();

    Ok(ApiQlSelectFields { fields: resolved, virtual_fields, hidden_fields })
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
        Value::Object(object) => object.iter().try_fold(Vec::new(), |mut fields, (key, value)| {
            if truthy_selector(value)? {
                fields.push(key.clone());
            }
            Ok(fields)
        }),
        _ => Err(ApiError::bad_request("Invalid ApiQL field list")),
    }
}

fn truthy_selector(value: &Value) -> Result<bool, ApiError> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::Number(number) => Ok(number.as_i64().unwrap_or_default() != 0),
        Value::String(text) => Ok(!matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "off" | "no"
        )),
        Value::Null => Ok(false),
        _ => Err(ApiError::bad_request("Invalid ApiQL field selector")),
    }
}

fn select_clause(entity: ApiQlResolvedEntity<'_>, fields: &[String]) -> Result<String, ApiError> {
    if fields.is_empty() {
        return Err(ApiError::bad_request("ApiQL select column list is empty"));
    }

    fields
        .iter()
        .map(|field_name| {
            let field = regular_field(entity, field_name).ok_or_else(|| {
                ApiError::bad_request(format!("Unknown ApiQL column field: {field_name}"))
            })?;
            Ok(format!(
                "{} AS {}",
                qualified_base_column(field_column(field)?)?,
                identifier(&field.name)?
            ))
        })
        .collect::<Result<Vec<_>, ApiError>>()
        .map(|parts| parts.join(", "))
}

fn build_where_conditions(
    entity: ApiQlResolvedEntity<'_>,
    where_: &Value,
    builder: &mut SqlBuildState,
) -> Result<Vec<String>, ApiError> {
    let object = match where_ {
        Value::Null => return Ok(Vec::new()),
        Value::Object(object) => object,
        _ => return Err(ApiError::bad_request("ApiQL where must be an object")),
    };

    let mut conditions = Vec::new();
    for (field_name, value) in object {
        if field_name == "search_key" {
            if let Some(condition) = build_search_key_condition(entity, value, builder)? {
                conditions.push(condition);
            }
            continue;
        }
        if let Some(raw) = field_name.strip_prefix("raw:") {
            conditions.push(raw_condition(raw)?);
            continue;
        }
        let field = regular_field(entity, field_name).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL where field: {field_name}"))
        })?;
        let column = qualified_base_column(field_column(field)?)?;
        conditions.extend(field_conditions(field_name, &column, value, builder)?);
    }
    Ok(conditions)
}

fn field_conditions(
    field_name: &str,
    column: &str,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<Vec<String>, ApiError> {
    match value {
        Value::Null => Ok(vec![format!("{column} IS NULL")]),
        Value::Array(_) => Ok(vec![in_condition(field_name, column, value, false, builder)?]),
        Value::Object(object) => object
            .iter()
            .map(|(operator, value)| {
                operator_condition(field_name, column, operator, value, builder)
            })
            .collect(),
        _ => {
            if let Some(raw) = value.as_str().and_then(|text| text.strip_prefix("raw:")) {
                return Ok(vec![format!("{column} = {}", raw_value(raw)?)]);
            }
            let param = format!("where_{}", param_segment(field_name)?);
            builder.params.insert(param.clone(), value.clone());
            Ok(vec![format!("{column} = {{{{ {param} }}}}")])
        }
    }
}

fn operator_condition(
    field_name: &str,
    column: &str,
    operator: &str,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let normalized = operator.replace('_', "").to_ascii_lowercase();
    match normalized.as_str() {
        "=" | "==" | "eq" => scalar_condition(field_name, column, "=", value, builder),
        "!=" | "<>" | "ne" | "neq" => scalar_condition(field_name, column, "!=", value, builder),
        ">" | "gt" => scalar_condition(field_name, column, ">", value, builder),
        ">=" | "gte" => scalar_condition(field_name, column, ">=", value, builder),
        "<" | "lt" => scalar_condition(field_name, column, "<", value, builder),
        "<=" | "lte" => scalar_condition(field_name, column, "<=", value, builder),
        "like" => like_condition(field_name, column, "LIKE", value, builder),
        "notlike" => like_condition(field_name, column, "NOT LIKE", value, builder),
        "in" => in_condition(field_name, column, value, false, builder),
        "notin" | "nin" => in_condition(field_name, column, value, true, builder),
        "between" => between_condition(field_name, column, value, builder),
        "isnull" | "null" => Ok(format!("{column} IS NULL")),
        "notnull" | "isnotnull" => Ok(format!("{column} IS NOT NULL")),
        "raw" => value
            .as_str()
            .ok_or_else(|| ApiError::bad_request("ApiQL raw operator requires a string"))
            .and_then(raw_value)
            .map(|raw| format!("{column} {raw}")),
        _ => Err(ApiError::bad_request(format!("Unsupported ApiQL where operator: {operator}"))),
    }
}

fn scalar_condition(
    field_name: &str,
    column: &str,
    operator: &str,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let param = format!("where_{}_{}", param_segment(field_name)?, operator_param_name(operator));
    builder.params.insert(param.clone(), value.clone());
    Ok(format!("{column} {operator} {{{{ {param} }}}}"))
}

fn like_condition(
    field_name: &str,
    column: &str,
    operator: &str,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let param = format!("where_{}_{}", param_segment(field_name)?, operator.to_ascii_lowercase());
    let text = value
        .as_str()
        .ok_or_else(|| ApiError::bad_request("ApiQL LIKE operator requires a string"))?;
    builder.params.insert(param.clone(), Value::String(like_pattern(text)));
    Ok(format!("{column} {operator} {{{{ {param} }}}}"))
}

fn in_condition(
    field_name: &str,
    column: &str,
    value: &Value,
    negated: bool,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let values = in_values(value)?;
    if values.is_empty() {
        return Ok(if negated { "1 = 1".to_string() } else { "1 = 0".to_string() });
    }
    let field_segment = param_segment(field_name)?;
    let mut placeholders = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        let param = format!("where_{field_segment}_{index}");
        builder.params.insert(param.clone(), value);
        placeholders.push(format!("{{{{ {param} }}}}"));
    }
    let operator = if negated { "NOT IN" } else { "IN" };
    Ok(format!("{column} {operator} ({})", placeholders.join(", ")))
}

fn in_values(value: &Value) -> Result<Vec<Value>, ApiError> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::String(text) => Ok(text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| Value::String(item.to_string()))
            .collect()),
        Value::Null => Ok(Vec::new()),
        _ => Ok(vec![value.clone()]),
    }
}

fn between_condition(
    field_name: &str,
    column: &str,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let values = in_values(value)?;
    if values.len() != 2 {
        return Err(ApiError::bad_request("ApiQL BETWEEN operator requires two values"));
    }
    let field_segment = param_segment(field_name)?;
    let start = format!("where_{field_segment}_between_start");
    let end = format!("where_{field_segment}_between_end");
    builder.params.insert(start.clone(), values[0].clone());
    builder.params.insert(end.clone(), values[1].clone());
    Ok(format!("{column} BETWEEN {{{{ {start} }}}} AND {{{{ {end} }}}}"))
}

fn build_search_key_condition(
    entity: ApiQlResolvedEntity<'_>,
    value: &Value,
    builder: &mut SqlBuildState,
) -> Result<Option<String>, ApiError> {
    let text = match value {
        Value::Null => return Ok(None),
        Value::String(text) => text.trim(),
        _ => return Err(ApiError::bad_request("ApiQL search_key must be a string")),
    };
    if text.is_empty() {
        return Ok(None);
    }

    let search_fields = entity.search_field_names();
    if search_fields.is_empty() {
        return Err(ApiError::bad_request("ApiQL search_key has no registered search fields"));
    }

    let mut groups = Vec::new();
    for raw_token in text.split_whitespace() {
        let mut token = raw_token.trim();
        if token.is_empty() {
            continue;
        }
        let negated = token.starts_with('-') || token.starts_with('!');
        if negated {
            token = &token[1..];
        }
        if token.is_empty() {
            continue;
        }

        let mut alternatives = Vec::new();
        for raw_alternative in token.split('|') {
            let alternative = raw_alternative.trim();
            if alternative.is_empty() {
                continue;
            }
            alternatives.push(search_term_condition(entity, &search_fields, alternative, builder)?);
        }
        if alternatives.is_empty() {
            continue;
        }
        let expression = if alternatives.len() == 1 {
            alternatives.remove(0)
        } else {
            format!("({})", alternatives.join(" OR "))
        };
        if negated {
            groups.push(format!("NOT {expression}"));
        } else {
            groups.push(expression);
        }
    }

    Ok((!groups.is_empty()).then(|| groups.join(" AND ")))
}

fn search_term_condition(
    entity: ApiQlResolvedEntity<'_>,
    search_fields: &[&str],
    term: &str,
    builder: &mut SqlBuildState,
) -> Result<String, ApiError> {
    let (fields, needle) = if let Some((field, text)) = term.split_once(':') {
        let field = field.trim();
        if !search_fields.contains(&field) {
            return Err(ApiError::bad_request(format!("Unknown ApiQL search field: {field}")));
        }
        (vec![field], text.trim())
    } else {
        (search_fields.to_vec(), term)
    };
    if needle.is_empty() {
        return Err(ApiError::bad_request("ApiQL search term cannot be empty"));
    }

    let param = format!("search_{}", builder.search_index);
    builder.search_index += 1;
    builder.params.insert(param.clone(), Value::String(like_pattern(needle)));
    let mut parts = Vec::new();
    for field_name in fields {
        let field = regular_field(entity, field_name).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL search field: {field_name}"))
        })?;
        parts.push(format!(
            "{} LIKE {{{{ {param} }}}}",
            qualified_base_column(field_column(field)?)?
        ));
    }
    Ok(format!("({})", parts.join(" OR ")))
}

fn build_cursor_condition(
    entity: ApiQlResolvedEntity<'_>,
    cursor: &Value,
    order: &[ApiQlOrderBy],
    field_case: ApiQlFieldNameCase,
    builder: &mut SqlBuildState,
) -> Result<Option<String>, ApiError> {
    let cursor = cursor_object(cursor)?;
    if cursor.is_empty() {
        return Ok(None);
    }
    if order.is_empty() {
        return Err(ApiError::bad_request("ApiQL cursor requires order_by"));
    }

    let mut branches = Vec::new();
    for (index, item) in order.iter().enumerate() {
        let value =
            cursor_field_value(entity, &cursor, &item.field, field_case).ok_or_else(|| {
                ApiError::bad_request(format!(
                    "ApiQL cursor is missing order field: {}",
                    item.field
                ))
            })?;
        let param = format!("cursor_{}", param_segment(&item.field)?);
        builder.params.entry(param.clone()).or_insert_with(|| value.clone());
        let mut parts = Vec::new();
        for previous in &order[..index] {
            let previous_param = format!("cursor_{}", param_segment(&previous.field)?);
            let previous_field = regular_field(entity, &previous.field).ok_or_else(|| {
                ApiError::bad_request(format!("Unknown ApiQL cursor field: {}", previous.field))
            })?;
            parts.push(format!(
                "{} = {{{{ {previous_param} }}}}",
                qualified_base_column(field_column(previous_field)?)?
            ));
        }
        let field = regular_field(entity, &item.field).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL cursor field: {}", item.field))
        })?;
        let operator = match item.direction {
            ApiQlSortDirection::Asc => ">",
            ApiQlSortDirection::Desc => "<",
        };
        parts.push(format!(
            "{} {operator} {{{{ {param} }}}}",
            qualified_base_column(field_column(field)?)?
        ));
        branches.push(if parts.len() == 1 {
            parts.remove(0)
        } else {
            format!("({})", parts.join(" AND "))
        });
    }

    Ok(Some(format!("({})", branches.join(" OR "))))
}

fn cursor_field_value<'a>(
    entity: ApiQlResolvedEntity<'_>,
    cursor: &'a Map<String, Value>,
    field_name: &str,
    field_case: ApiQlFieldNameCase,
) -> Option<&'a Value> {
    cursor.get(field_name).or_else(|| {
        let output_name = output_field_name(entity, field_name, field_case);
        cursor.get(&output_name)
    })
}

fn output_field_name(
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

fn cursor_object(cursor: &Value) -> Result<Map<String, Value>, ApiError> {
    match cursor {
        Value::Null => Ok(Map::new()),
        Value::Object(object) => Ok(object.clone()),
        Value::String(text) if text.trim().is_empty() => Ok(Map::new()),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .map_err(|_| ApiError::bad_request("ApiQL cursor must be a JSON object"))?
            .as_object()
            .cloned()
            .ok_or_else(|| ApiError::bad_request("ApiQL cursor must be a JSON object")),
        _ => Err(ApiError::bad_request("ApiQL cursor must be an object")),
    }
}

fn parse_order_by(
    entity: ApiQlResolvedEntity<'_>,
    value: &Value,
) -> Result<Vec<ApiQlOrderBy>, ApiError> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => entity.parse_default_order(text),
        Value::Array(items) => {
            let mut parsed = Vec::new();
            for item in items {
                parsed.extend(parse_order_by(entity, item)?);
            }
            Ok(parsed)
        }
        Value::Object(object) => object
            .iter()
            .map(|(field, direction)| {
                let direction = direction.as_str().ok_or_else(|| {
                    ApiError::bad_request("ApiQL order direction must be a string")
                })?;
                entity.parse_default_order(&format!("{field} {direction}"))
            })
            .collect::<Result<Vec<_>, ApiError>>()
            .map(|items| items.into_iter().flatten().collect()),
        _ => Err(ApiError::bad_request("Invalid ApiQL order_by")),
    }
}

fn order_by_clause(order: &[ApiQlOrderBy]) -> Option<String> {
    if order.is_empty() {
        return None;
    }
    Some(
        order
            .iter()
            .map(|item| {
                let direction = match item.direction {
                    ApiQlSortDirection::Asc => "ASC",
                    ApiQlSortDirection::Desc => "DESC",
                };
                format!("{} {direction}", item.field)
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn group_by_clause(
    entity: ApiQlResolvedEntity<'_>,
    value: &Value,
) -> Result<Option<String>, ApiError> {
    let fields = field_list(value)?;
    if fields.is_empty() {
        return Ok(None);
    }
    fields
        .iter()
        .map(|field_name| {
            let field = regular_field(entity, field_name).ok_or_else(|| {
                ApiError::bad_request(format!("Unknown ApiQL group field: {field_name}"))
            })?;
            qualified_base_column(field_column(field)?)
        })
        .collect::<Result<Vec<_>, ApiError>>()
        .map(|parts| Some(parts.join(", ")))
}

fn build_join_sql(
    registry: &ApiQlEntityRegistry,
    entity: ApiQlResolvedEntity<'_>,
    join: &Value,
    relation: &Value,
) -> Result<Vec<String>, ApiError> {
    let mut names = requested_relation_names(join)?;
    names.extend(requested_relation_names(relation)?);
    let mut seen = BTreeSet::new();
    let mut joins = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }
        let relation = entity
            .config()
            .relations
            .iter()
            .find(|relation| relation.name == name)
            .ok_or_else(|| ApiError::bad_request(format!("Unknown ApiQL relation: {name}")))?;
        let related = registry.resolve(&relation.entity_class)?;
        let local_field = relation.local_field.as_deref().unwrap_or("id");
        let foreign_field = relation.foreign_field.as_deref().unwrap_or("id");
        let local = regular_field(entity, local_field).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL relation local field: {local_field}"))
        })?;
        let foreign = regular_field(related, foreign_field).ok_or_else(|| {
            ApiError::bad_request(format!("Unknown ApiQL relation foreign field: {foreign_field}"))
        })?;
        let alias = identifier(&relation.name)?;
        joins.push(format!(
            "LEFT JOIN {} AS {alias} ON {} = {}",
            table_name(&related.config().table)?,
            qualified_base_column(field_column(local)?)?,
            qualified_column(&alias, field_column(foreign)?)?,
        ));
    }
    Ok(joins)
}

fn requested_relation_names(value: &Value) -> Result<Vec<String>, ApiError> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(text) => Ok(text
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .collect()),
        Value::Array(items) => items.iter().try_fold(Vec::new(), |mut names, item| {
            names.extend(requested_relation_names(item)?);
            Ok(names)
        }),
        Value::Object(object) => Ok(object.keys().cloned().collect()),
        _ => Err(ApiError::bad_request("Invalid ApiQL relation config")),
    }
}

fn regular_field<'a>(
    entity: ApiQlResolvedEntity<'a>,
    field_name: &str,
) -> Option<&'a ApiQlFieldConfig> {
    let normalized = field_name.trim().replace('.', "_");
    let camel = to_camel_case(&normalized);
    entity.config().fields.iter().find(|field| {
        field.name == field_name
            || field.name == normalized
            || field.name == camel
            || to_snake_case(&field.name) == normalized
            || field.column.as_deref() == Some(field_name)
            || field.column.as_deref() == Some(normalized.as_str())
    })
}

fn virtual_field<'a>(
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

fn field_column(field: &ApiQlFieldConfig) -> Result<&str, ApiError> {
    field
        .column
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("ApiQL field has no column: {}", field.name)))
}

fn qualified_base_column(column: &str) -> Result<String, ApiError> {
    qualified_column(BASE_ALIAS, column)
}

fn qualified_column(alias: &str, column: &str) -> Result<String, ApiError> {
    Ok(format!("{}.{}", identifier(alias)?, identifier(column)?))
}

fn table_name(value: &str) -> Result<String, ApiError> {
    let parts =
        value.split('.').map(str::trim).map(identifier).collect::<Result<Vec<_>, ApiError>>()?;
    if parts.is_empty() {
        return Err(ApiError::bad_request("Invalid ApiQL table name"));
    }
    Ok(parts.join("."))
}

fn identifier(value: &str) -> Result<String, ApiError> {
    if value.is_empty()
        || !value.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || value.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return Err(ApiError::bad_request(format!("Invalid ApiQL SQL identifier: {value}")));
    }
    Ok(value.to_string())
}

fn param_segment(value: &str) -> Result<String, ApiError> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(ApiError::bad_request(format!("Invalid ApiQL parameter name: {value}")));
    }
    Ok(value.to_string())
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

fn operator_param_name(operator: &str) -> &'static str {
    match operator {
        "=" => "eq",
        "!=" => "ne",
        ">" => "gt",
        ">=" => "gte",
        "<" => "lt",
        "<=" => "lte",
        _ => "op",
    }
}

fn like_pattern(value: &str) -> String {
    if value.contains('%') { value.to_string() } else { format!("%{value}%") }
}

fn raw_condition(fragment: &str) -> Result<String, ApiError> {
    safe_raw_sql(fragment)?;
    Ok(format!("({fragment})"))
}

fn raw_value(fragment: &str) -> Result<String, ApiError> {
    safe_raw_sql(fragment)?;
    Ok(fragment.to_string())
}

fn safe_raw_sql(fragment: &str) -> Result<(), ApiError> {
    let trimmed = fragment.trim();
    if trimmed.is_empty()
        || trimmed.contains(';')
        || trimmed.contains("--")
        || trimmed.contains("/*")
        || trimmed.contains("*/")
    {
        return Err(ApiError::bad_request("Unsafe ApiQL raw SQL expression"));
    }
    let lower = format!(" {} ", trimmed.to_ascii_lowercase());
    let forbidden = [
        " insert ",
        " update ",
        " delete ",
        " drop ",
        " alter ",
        " truncate ",
        " attach ",
        " detach ",
        " replace ",
        " pragma ",
        " create ",
    ];
    if forbidden.iter().any(|token| lower.contains(token)) {
        return Err(ApiError::bad_request("Unsafe ApiQL raw SQL expression"));
    }
    if !trimmed.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || ch.is_ascii_whitespace()
            || matches!(
                ch,
                '_' | '.'
                    | '\''
                    | '"'
                    | '%'
                    | '?'
                    | ':'
                    | '='
                    | '<'
                    | '>'
                    | '!'
                    | '('
                    | ')'
                    | ','
                    | '+'
                    | '-'
                    | '*'
                    | '/'
            )
    }) {
        return Err(ApiError::bad_request("Unsafe ApiQL raw SQL expression"));
    }
    Ok(())
}
