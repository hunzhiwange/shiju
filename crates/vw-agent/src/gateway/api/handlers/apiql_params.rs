//! ApiQL PHP 兼容查询参数归一化。
//!
//! PHP 入口会先把非保留 query 字段归入 `where`，再合并显式 `where`。
//! 这里保留同样的数据边界，只负责 URL query 到内部参数结构的转换。

use serde_json::{Map, Value};
use vw_api_types::apiql::ApiQlListMode;

use crate::app::agent::gateway::ApiError;

pub(crate) const READ_PARAMS_REMAINED_FIELD: &[&str] = &[
    "page",
    "page_size",
    "pageSize",
    "expect_page",
    "expectPage",
    "limit",
    "column",
    "key_column",
    "keyColumn",
    "order_by",
    "orderBy",
    "group_by",
    "groupBy",
    "where",
    "relation",
    "join",
    "cursor",
    "transformer",
    "transformer_keep_column",
    "transformerKeepColumn",
    "transformer_delimiter",
    "transformerDelimiter",
    "field_case",
    "fieldCase",
    "development",
    "dev",
    "only",
    "count",
];

pub(crate) const PARAMS_DTO_SPECIAL_FIELD: &[&str] = &[
    "format",
    "app_key",
    "app_secret",
    "timestamp",
    "signature_method",
    "token",
    "token_type",
    "version",
    "signature",
];

pub(crate) const PARAMS_DTO_REMAINED_FIELD: &[&str] = &[
    "page",
    "page_size",
    "pageSize",
    "expect_page",
    "expectPage",
    "limit",
    "column",
    "key_column",
    "keyColumn",
    "order_by",
    "orderBy",
    "group_by",
    "groupBy",
    "cursor",
    "transformer",
    "transformer_keep_column",
    "transformerKeepColumn",
    "transformer_delimiter",
    "transformerDelimiter",
    "field_case",
    "fieldCase",
    "development",
    "dev",
    "only",
    "count",
    "entity",
    "entity_data",
    "entityData",
    "entity_class",
    "entityClass",
    "entity_method",
    "entityMethod",
    "validator_scene",
    "validatorScene",
    "validator_class",
    "validatorClass",
];

const PAGE_KEYS: &[&str] = &["page"];
const PAGE_SIZE_KEYS: &[&str] = &["page_size", "pageSize"];
const EXPECT_PAGE_KEYS: &[&str] = &["expect_page", "expectPage"];
const LIMIT_KEYS: &[&str] = &["limit"];
const COLUMN_KEYS: &[&str] = &["column"];
const KEY_COLUMN_KEYS: &[&str] = &["key_column", "keyColumn"];
const ORDER_BY_KEYS: &[&str] = &["order_by", "orderBy"];
const GROUP_BY_KEYS: &[&str] = &["group_by", "groupBy"];
const WHERE_KEYS: &[&str] = &["where"];
const RELATION_KEYS: &[&str] = &["relation"];
const JOIN_KEYS: &[&str] = &["join"];
const CURSOR_KEYS: &[&str] = &["cursor"];
const TRANSFORMER_KEYS: &[&str] = &["transformer"];
const TRANSFORMER_KEEP_COLUMN_KEYS: &[&str] = &["transformer_keep_column", "transformerKeepColumn"];
const TRANSFORMER_DELIMITER_KEYS: &[&str] = &["transformer_delimiter", "transformerDelimiter"];
const FIELD_CASE_KEYS: &[&str] = &["field_case", "fieldCase"];
const DEVELOPMENT_KEYS: &[&str] = &["development", "dev"];
const ONLY_KEYS: &[&str] = &["only"];
const COUNT_KEYS: &[&str] = &["count"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiQlFieldNameCase {
    Snake,
    Camel,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApiQlReadParams {
    pub(crate) mode: ApiQlListMode,
    pub(crate) page: Option<u32>,
    pub(crate) page_size: Option<u32>,
    pub(crate) expect_page: bool,
    pub(crate) limit: Option<u32>,
    pub(crate) column: Value,
    pub(crate) key_column: Value,
    pub(crate) order_by: Value,
    pub(crate) group_by: Value,
    pub(crate) where_: Value,
    pub(crate) relation: Value,
    pub(crate) join: Value,
    pub(crate) cursor: Value,
    pub(crate) transformer: Value,
    pub(crate) transformer_keep_column: bool,
    pub(crate) transformer_delimiter: String,
    pub(crate) field_case: ApiQlFieldNameCase,
    pub(crate) development: bool,
    pub(crate) only: bool,
    pub(crate) count: bool,
}

pub(crate) fn normalize_apiql_query(
    raw_query: &str,
    mode: ApiQlListMode,
) -> Result<ApiQlReadParams, ApiError> {
    let mut fields = parse_bracket_query(raw_query)?;

    let page = take_optional_u32(&mut fields, PAGE_KEYS)?;
    let page_size = take_optional_u32(&mut fields, PAGE_SIZE_KEYS)?;
    let expect_page = take_optional_bool(&mut fields, EXPECT_PAGE_KEYS)?.unwrap_or(false)
        || matches!(mode, ApiQlListMode::IndexPage);
    let limit = take_optional_u32(&mut fields, LIMIT_KEYS)?;
    let column = take_value(&mut fields, COLUMN_KEYS);
    let key_column = take_value(&mut fields, KEY_COLUMN_KEYS);
    let order_by = take_value(&mut fields, ORDER_BY_KEYS);
    let group_by = take_value(&mut fields, GROUP_BY_KEYS);
    let explicit_where = take_value(&mut fields, WHERE_KEYS);
    let relation = take_value(&mut fields, RELATION_KEYS);
    let join = take_value(&mut fields, JOIN_KEYS);
    let cursor = take_value(&mut fields, CURSOR_KEYS);
    let transformer = take_value(&mut fields, TRANSFORMER_KEYS);
    let transformer_keep_column =
        take_optional_bool(&mut fields, TRANSFORMER_KEEP_COLUMN_KEYS)?.unwrap_or(true);
    let transformer_delimiter = take_optional_string(&mut fields, TRANSFORMER_DELIMITER_KEYS)?
        .unwrap_or_else(|| ".".to_string());
    let field_case = take_optional_field_name_case(&mut fields, FIELD_CASE_KEYS)?
        .unwrap_or(ApiQlFieldNameCase::Snake);
    let development = take_optional_bool(&mut fields, DEVELOPMENT_KEYS)?.unwrap_or(false);
    let only = take_optional_bool(&mut fields, ONLY_KEYS)?.unwrap_or(false)
        || matches!(mode, ApiQlListMode::IndexOnly);
    let count = take_optional_bool(&mut fields, COUNT_KEYS)?.unwrap_or(false)
        || matches!(mode, ApiQlListMode::IndexCount);
    discard_aliases(&mut fields, PARAMS_DTO_SPECIAL_FIELD);
    discard_aliases(&mut fields, PARAMS_DTO_REMAINED_FIELD);

    let where_ = merge_free_query_into_where(fields, explicit_where)?;

    Ok(ApiQlReadParams {
        mode,
        page,
        page_size,
        expect_page,
        limit,
        column,
        key_column,
        order_by,
        group_by,
        where_,
        relation,
        join,
        cursor,
        transformer,
        transformer_keep_column,
        transformer_delimiter,
        field_case,
        development,
        only,
        count,
    })
}

fn parse_bracket_query(raw_query: &str) -> Result<Map<String, Value>, ApiError> {
    let mut root = Value::Object(Map::new());
    let query = raw_query.strip_prefix('?').unwrap_or(raw_query);
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (raw_key, raw_value) = split_query_pair(pair);
        let key = decode_query_component(raw_key)?;
        if key.is_empty() {
            return Err(ApiError::bad_request("Invalid ApiQL query key"));
        }
        let value = Value::String(decode_query_component(raw_value)?);
        let path = parse_query_key(&key)?;
        insert_query_value(&mut root, &path, value)?;
    }

    match root {
        Value::Object(fields) => Ok(fields),
        _ => Err(ApiError::bad_request("Invalid ApiQL query")),
    }
}

fn split_query_pair(pair: &str) -> (&str, &str) {
    let mut bracket_depth = 0usize;
    for (index, ch) in pair.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '=' if bracket_depth == 0 => return (&pair[..index], &pair[index + 1..]),
            _ => {}
        }
    }
    (pair, "")
}

fn decode_query_component(raw: &str) -> Result<String, ApiError> {
    let bytes = raw.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
        {
            return Err(ApiError::bad_request("Invalid ApiQL query encoding"));
        }
    }
    let form_value = raw.replace('+', " ");
    urlencoding::decode(&form_value)
        .map(|value| value.into_owned())
        .map_err(|error| ApiError::bad_request(format!("Invalid ApiQL query encoding: {error}")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueryPathSegment {
    Key(String),
    Push,
}

fn parse_query_key(key: &str) -> Result<Vec<QueryPathSegment>, ApiError> {
    let Some(first_bracket) = key.find('[') else {
        return Ok(vec![QueryPathSegment::Key(key.to_string())]);
    };
    if first_bracket == 0 {
        return Err(ApiError::bad_request(format!("Invalid ApiQL query key: {key}")));
    }

    let mut path = vec![QueryPathSegment::Key(key[..first_bracket].to_string())];
    let mut rest = &key[first_bracket..];
    while !rest.is_empty() {
        if !rest.starts_with('[') {
            return Err(ApiError::bad_request(format!("Invalid ApiQL query key: {key}")));
        }
        let Some(close) = rest.find(']') else {
            return Err(ApiError::bad_request(format!("Invalid ApiQL query key: {key}")));
        };
        let segment = &rest[1..close];
        if segment.is_empty() {
            path.push(QueryPathSegment::Push);
        } else {
            path.push(QueryPathSegment::Key(segment.to_string()));
        }
        rest = &rest[close + 1..];
    }
    Ok(path)
}

fn insert_query_value(
    target: &mut Value,
    path: &[QueryPathSegment],
    value: Value,
) -> Result<(), ApiError> {
    let Some((segment, rest)) = path.split_first() else {
        return Err(ApiError::bad_request("Invalid ApiQL query key"));
    };

    match segment {
        QueryPathSegment::Key(key) => insert_key_value(target, key, rest, value),
        QueryPathSegment::Push => insert_array_value(target, rest, value),
    }
}

fn insert_key_value(
    target: &mut Value,
    key: &str,
    rest: &[QueryPathSegment],
    value: Value,
) -> Result<(), ApiError> {
    ensure_object(target)?;
    let object = target
        .as_object_mut()
        .ok_or_else(|| ApiError::bad_request("Invalid ApiQL query object"))?;
    if rest.is_empty() {
        merge_leaf_value(object.entry(key.to_string()).or_insert(Value::Null), value);
        return Ok(());
    }

    let child = object.entry(key.to_string()).or_insert_with(|| empty_container_for(rest.first()));
    insert_query_value(child, rest, value)
}

fn insert_array_value(
    target: &mut Value,
    rest: &[QueryPathSegment],
    value: Value,
) -> Result<(), ApiError> {
    ensure_array(target)?;
    let array =
        target.as_array_mut().ok_or_else(|| ApiError::bad_request("Invalid ApiQL query array"))?;
    if rest.is_empty() {
        array.push(value);
        return Ok(());
    }

    let mut child = empty_container_for(rest.first());
    insert_query_value(&mut child, rest, value)?;
    array.push(child);
    Ok(())
}

fn ensure_object(value: &mut Value) -> Result<(), ApiError> {
    match value {
        Value::Null => {
            *value = Value::Object(Map::new());
            Ok(())
        }
        Value::Object(_) => Ok(()),
        _ => Err(ApiError::bad_request("Conflicting ApiQL query parameter shape")),
    }
}

fn ensure_array(value: &mut Value) -> Result<(), ApiError> {
    match value {
        Value::Null => {
            *value = Value::Array(Vec::new());
            Ok(())
        }
        Value::Array(_) => Ok(()),
        _ => Err(ApiError::bad_request("Conflicting ApiQL query parameter shape")),
    }
}

fn empty_container_for(segment: Option<&QueryPathSegment>) -> Value {
    match segment {
        Some(QueryPathSegment::Push) => Value::Array(Vec::new()),
        _ => Value::Object(Map::new()),
    }
}

fn merge_leaf_value(slot: &mut Value, value: Value) {
    match slot {
        Value::Null => *slot = value,
        Value::Array(values) => values.push(value),
        _ => {
            let old = std::mem::replace(slot, Value::Null);
            *slot = Value::Array(vec![old, value]);
        }
    }
}

fn take_value(fields: &mut Map<String, Value>, aliases: &[&str]) -> Value {
    take_alias(fields, aliases).unwrap_or(Value::Null)
}

fn take_optional_u32(
    fields: &mut Map<String, Value>,
    aliases: &[&str],
) -> Result<Option<u32>, ApiError> {
    take_alias(fields, aliases).map(|value| parse_u32_value(&value)).transpose()
}

fn take_optional_bool(
    fields: &mut Map<String, Value>,
    aliases: &[&str],
) -> Result<Option<bool>, ApiError> {
    take_alias(fields, aliases).map(|value| parse_bool_value(&value)).transpose()
}

fn take_optional_string(
    fields: &mut Map<String, Value>,
    aliases: &[&str],
) -> Result<Option<String>, ApiError> {
    take_alias(fields, aliases).map(|value| parse_string_value(&value)).transpose()
}

fn take_optional_field_name_case(
    fields: &mut Map<String, Value>,
    aliases: &[&str],
) -> Result<Option<ApiQlFieldNameCase>, ApiError> {
    take_alias(fields, aliases).map(|value| parse_field_name_case(&value)).transpose()
}

fn take_alias(fields: &mut Map<String, Value>, aliases: &[&str]) -> Option<Value> {
    aliases.iter().find_map(|alias| fields.remove(*alias))
}

fn discard_aliases(fields: &mut Map<String, Value>, aliases: &[&str]) {
    for alias in aliases {
        fields.remove(*alias);
    }
}

fn parse_u32_value(value: &Value) -> Result<u32, ApiError> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| ApiError::bad_request("Invalid ApiQL unsigned integer parameter")),
        Value::String(text) => text
            .parse::<u32>()
            .map_err(|_| ApiError::bad_request("Invalid ApiQL unsigned integer parameter")),
        _ => Err(ApiError::bad_request("Invalid ApiQL unsigned integer parameter")),
    }
}

fn parse_bool_value(value: &Value) -> Result<bool, ApiError> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::Number(number) => match number.as_u64() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(ApiError::bad_request("Invalid ApiQL boolean parameter")),
        },
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "" | "0" | "false" | "off" | "no" => Ok(false),
            "1" | "true" | "on" | "yes" => Ok(true),
            _ => Err(ApiError::bad_request("Invalid ApiQL boolean parameter")),
        },
        _ => Err(ApiError::bad_request("Invalid ApiQL boolean parameter")),
    }
}

fn parse_string_value(value: &Value) -> Result<String, ApiError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        _ => Err(ApiError::bad_request("Invalid ApiQL string parameter")),
    }
}

fn parse_field_name_case(value: &Value) -> Result<ApiQlFieldNameCase, ApiError> {
    let Value::String(text) = value else {
        return Err(ApiError::bad_request("Invalid ApiQL field_case parameter"));
    };
    match text.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "" | "0" | "snake" | "snake_case" => Ok(ApiQlFieldNameCase::Snake),
        "1" | "camel" | "camelcase" | "camel_case" => Ok(ApiQlFieldNameCase::Camel),
        _ => Err(ApiError::bad_request("Invalid ApiQL field_case parameter")),
    }
}

fn merge_free_query_into_where(
    free_fields: Map<String, Value>,
    explicit_where: Value,
) -> Result<Value, ApiError> {
    let mut where_fields = Map::new();
    for (key, value) in free_fields {
        if is_reserved_field(&key) {
            return Err(ApiError::bad_request(format!(
                "Unsupported ApiQL reserved parameter: {key}"
            )));
        }
        where_fields.insert(key, value);
    }

    match explicit_where {
        Value::Null => Ok(Value::Object(where_fields)),
        Value::Object(explicit_fields) => {
            where_fields.extend(explicit_fields);
            Ok(Value::Object(where_fields))
        }
        other if where_fields.is_empty() => Ok(other),
        _ => Err(ApiError::bad_request("ApiQL where must be an object")),
    }
}

fn is_reserved_field(field: &str) -> bool {
    READ_PARAMS_REMAINED_FIELD.contains(&field)
        || PARAMS_DTO_SPECIAL_FIELD.contains(&field)
        || PARAMS_DTO_REMAINED_FIELD.contains(&field)
}
