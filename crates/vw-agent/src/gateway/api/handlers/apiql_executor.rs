//! ApiQL 只读数据库执行器，负责查询参数绑定、分页与后端结果归一化。
//!
//! Read-only database executor used by ApiQL.

use std::collections::BTreeMap;
#[cfg(feature = "memory-mariadb")]
use std::collections::HashMap;
#[cfg(feature = "memory-mariadb")]
use std::sync::Arc;

use once_cell::sync::Lazy;
#[cfg(feature = "memory-mariadb")]
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};

use super::connections::{ApiQlConnection, ApiQlConnectionKind};

static PLACEHOLDER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}").expect("placeholder regex"));
static ORDER_BY_SEGMENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^([A-Za-z0-9_.]+)(\s+(ASC|DESC))?$").expect("order by regex"));
#[cfg(feature = "memory-mariadb")]
static MYSQL_POOLS: Lazy<Mutex<HashMap<String, Arc<mysql::Pool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ApiQlCountMode {
    Disabled,
    Enabled,
    Only,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiQlQueryPage {
    pub(crate) per_page: u32,
    pub(crate) current_page: u32,
    pub(crate) total_page: u32,
    pub(crate) total_record: u64,
    pub(crate) from: u64,
    pub(crate) to: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApiQlQueryResult {
    pub(crate) page: ApiQlQueryPage,
    pub(crate) items: Vec<Value>,
    pub(crate) has_next_page: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiQlDatabaseQuery {
    pub(crate) sql: String,
    pub(crate) count_sql: Option<String>,
    pub(crate) params: BTreeMap<String, Value>,
    pub(crate) page: u32,
    pub(crate) limit: u32,
    pub(crate) count_mode: ApiQlCountMode,
    pub(crate) order_by: Option<String>,
}

impl Default for ApiQlDatabaseQuery {
    fn default() -> Self {
        Self {
            sql: String::new(),
            count_sql: None,
            params: BTreeMap::new(),
            page: 1,
            limit: 25,
            count_mode: ApiQlCountMode::Disabled,
            order_by: None,
        }
    }
}

pub(crate) async fn execute_read_only_query(
    connection: ApiQlConnection,
    query: ApiQlDatabaseQuery,
) -> Result<ApiQlQueryResult, String> {
    if !connection.enabled {
        return Err("ApiQL connection is disabled".to_string());
    }

    let page_number = query.page.max(1);
    let limit = query.limit.clamp(1, 10_000);
    let base_sql = render_sql_template(&query.sql, &query.params);
    ensure_read_only_sql(&base_sql)?;
    let order_by = sanitize_order_by(query.order_by.as_deref())?;
    let offset = u64::from(page_number.saturating_sub(1)) * u64::from(limit);
    let paged_sql = match order_by.as_deref() {
        Some(order) => format!(
            "SELECT * FROM ({base_sql}) AS vw_apiql_base ORDER BY {order} LIMIT {limit} OFFSET {offset}"
        ),
        None => {
            format!("SELECT * FROM ({base_sql}) AS vw_apiql_base LIMIT {limit} OFFSET {offset}")
        }
    };
    let rendered_count_sql = match query.count_mode {
        ApiQlCountMode::Disabled => None,
        ApiQlCountMode::Enabled | ApiQlCountMode::Only => Some(
            query
                .count_sql
                .as_ref()
                .map(|value| render_sql_template(value, &query.params))
                .unwrap_or_else(|| {
                    format!("SELECT COUNT(1) AS count FROM ({base_sql}) AS vw_apiql_count")
                }),
        ),
    };

    let (total_record, items) = match query.count_mode {
        ApiQlCountMode::Disabled => (0, execute_sql_rows(&connection, paged_sql).await?),
        ApiQlCountMode::Only => {
            let count_sql = rendered_count_sql.expect("count SQL exists for count-only mode");
            (execute_sql_count(&connection, count_sql).await?, Vec::new())
        }
        ApiQlCountMode::Enabled => {
            let count_sql = rendered_count_sql.expect("count SQL exists for enabled count mode");
            let (total, items) = tokio::try_join!(
                execute_sql_count(&connection, count_sql),
                execute_sql_rows(&connection, paged_sql),
            )?;
            (total, items)
        }
    };

    let effective_total = match query.count_mode {
        ApiQlCountMode::Disabled => items.len() as u64,
        ApiQlCountMode::Enabled | ApiQlCountMode::Only => total_record,
    };
    let page = build_page(effective_total, page_number, limit, items.len());
    let has_next_page = if matches!(query.count_mode, ApiQlCountMode::Disabled) {
        items.len() as u32 >= limit
    } else {
        page.current_page < page.total_page
    };
    Ok(ApiQlQueryResult { page, items, has_next_page })
}

async fn execute_sql_rows(connection: &ApiQlConnection, sql: String) -> Result<Vec<Value>, String> {
    match connection.kind {
        ApiQlConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            tokio::task::spawn_blocking(move || sqlite_query_rows(&path, &sql))
                .await
                .map_err(|error| error.to_string())?
        }
        ApiQlConnectionKind::Mysql => execute_mysql_rows(connection, sql).await,
        ApiQlConnectionKind::Postgres => execute_postgres_rows(connection, sql).await,
    }
}

async fn execute_sql_count(connection: &ApiQlConnection, sql: String) -> Result<u64, String> {
    match connection.kind {
        ApiQlConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            tokio::task::spawn_blocking(move || sqlite_query_count(&path, &sql))
                .await
                .map_err(|error| error.to_string())?
        }
        ApiQlConnectionKind::Mysql => execute_mysql_count(connection, sql).await,
        ApiQlConnectionKind::Postgres => execute_postgres_count(connection, sql).await,
    }
}

fn sqlite_path(connection: &ApiQlConnection) -> Result<String, String> {
    connection
        .sqlite_path
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "SQLite connection requires sqlite_path".to_string())
}

fn database_connection_url(connection: &ApiQlConnection, label: &str) -> Result<String, String> {
    connection
        .connection_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{label} connection requires connection_url"))
}

#[cfg(not(feature = "memory-mariadb"))]
async fn execute_mysql_rows(
    connection: &ApiQlConnection,
    _sql: String,
) -> Result<Vec<Value>, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err("MySQL executor is disabled; enable memory-mariadb".to_string())
}

#[cfg(feature = "memory-mariadb")]
async fn execute_mysql_rows(
    connection: &ApiQlConnection,
    sql: String,
) -> Result<Vec<Value>, String> {
    let url = database_connection_url(connection, "MySQL")?;
    tokio::task::spawn_blocking(move || mysql_query_rows(&url, &sql))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(not(feature = "memory-postgres"))]
async fn execute_postgres_rows(
    connection: &ApiQlConnection,
    _sql: String,
) -> Result<Vec<Value>, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err("PostgreSQL executor is disabled; enable memory-postgres".to_string())
}

#[cfg(feature = "memory-postgres")]
async fn execute_postgres_rows(
    connection: &ApiQlConnection,
    sql: String,
) -> Result<Vec<Value>, String> {
    let url = database_connection_url(connection, "PostgreSQL")?;
    tokio::task::spawn_blocking(move || postgres_query_rows(&url, &sql))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(not(feature = "memory-mariadb"))]
async fn execute_mysql_count(connection: &ApiQlConnection, _sql: String) -> Result<u64, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err("MySQL executor is disabled; enable memory-mariadb".to_string())
}

#[cfg(feature = "memory-mariadb")]
async fn execute_mysql_count(connection: &ApiQlConnection, sql: String) -> Result<u64, String> {
    let url = database_connection_url(connection, "MySQL")?;
    tokio::task::spawn_blocking(move || mysql_query_rows(&url, &sql).and_then(count_from_items))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(not(feature = "memory-postgres"))]
async fn execute_postgres_count(connection: &ApiQlConnection, _sql: String) -> Result<u64, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err("PostgreSQL executor is disabled; enable memory-postgres".to_string())
}

#[cfg(feature = "memory-postgres")]
async fn execute_postgres_count(connection: &ApiQlConnection, sql: String) -> Result<u64, String> {
    let url = database_connection_url(connection, "PostgreSQL")?;
    tokio::task::spawn_blocking(move || postgres_query_rows(&url, &sql).and_then(count_from_items))
        .await
        .map_err(|error| error.to_string())?
}

fn sqlite_query_rows(path: &str, sql: &str) -> Result<Vec<Value>, String> {
    let connection = rusqlite::Connection::open(path).map_err(|error| error.to_string())?;
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let column_names =
        statement.column_names().iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let mut object = Map::new();
        for (index, name) in column_names.iter().enumerate() {
            let value = row.get_ref(index).map_err(|error| error.to_string())?;
            object.insert(name.clone(), sqlite_value_to_json(value));
        }
        items.push(Value::Object(object));
    }
    Ok(items)
}

fn sqlite_query_count(path: &str, sql: &str) -> Result<u64, String> {
    let connection = rusqlite::Connection::open(path).map_err(|error| error.to_string())?;
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let Some(row) = rows.next().map_err(|error| error.to_string())? else {
        return Ok(0);
    };
    if let Ok(value) = row.get::<usize, i64>(0) {
        return Ok(value.max(0) as u64);
    }
    if let Ok(value) = row.get::<usize, String>(0) {
        return value.parse::<u64>().map_err(|error| error.to_string());
    }
    Ok(0)
}

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => json!(value),
        ValueRef::Real(value) => json!(value),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).to_string()),
        ValueRef::Blob(value) => Value::String(format!("0x{}", hex::encode(value))),
    }
}

#[cfg(feature = "memory-mariadb")]
fn mysql_query_rows(url: &str, sql: &str) -> Result<Vec<Value>, String> {
    use mysql::prelude::Queryable;

    let pool = mysql_pool(url)?;
    let mut connection = pool.get_conn().map_err(|error| error.to_string())?;
    let result = connection.query_iter(sql).map_err(|error| error.to_string())?;
    let columns = result
        .columns()
        .as_ref()
        .iter()
        .map(|column| (column.name_str().to_string(), column.column_type(), column.flags()))
        .collect::<Vec<_>>();
    let mut items = Vec::new();
    for row in result {
        let row = row.map_err(|error| error.to_string())?;
        let mut object = Map::new();
        for ((name, column_type, flags), value) in columns.iter().zip(row.unwrap()) {
            object.insert(name.clone(), mysql_value_to_json(value, *column_type, *flags));
        }
        items.push(Value::Object(object));
    }
    Ok(items)
}

#[cfg(feature = "memory-mariadb")]
fn mysql_pool(url: &str) -> Result<Arc<mysql::Pool>, String> {
    if let Some(pool) = MYSQL_POOLS.lock().get(url).cloned() {
        return Ok(pool);
    }
    let options =
        mysql::Opts::from_url(url).map_err(|error| format!("Invalid MySQL URL: {error}"))?;
    let pool = Arc::new(mysql::Pool::new(options).map_err(|error| error.to_string())?);
    MYSQL_POOLS.lock().insert(url.to_string(), pool.clone());
    Ok(pool)
}

#[cfg(feature = "memory-mariadb")]
fn mysql_value_to_json(
    value: mysql::Value,
    column_type: mysql::consts::ColumnType,
    flags: mysql::consts::ColumnFlags,
) -> Value {
    match value {
        mysql::Value::NULL => Value::Null,
        mysql::Value::Bytes(bytes) => mysql_bytes_to_json(bytes, column_type, flags),
        mysql::Value::Int(value) => json!(value),
        mysql::Value::UInt(value) => json!(value),
        mysql::Value::Float(value) => json!(value),
        mysql::Value::Double(value) => json!(value),
        mysql::Value::Date(year, month, day, hour, minute, second, micros) => json!(format!(
            "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micros:06}"
        )),
        mysql::Value::Time(negative, days, hours, minutes, seconds, micros) => {
            let sign = if negative { "-" } else { "" };
            json!(format!("{sign}{days} {hours:02}:{minutes:02}:{seconds:02}.{micros:06}"))
        }
    }
}

#[cfg(feature = "memory-mariadb")]
fn mysql_bytes_to_json(
    bytes: Vec<u8>,
    column_type: mysql::consts::ColumnType,
    flags: mysql::consts::ColumnFlags,
) -> Value {
    use mysql::consts::ColumnType;

    match column_type {
        ColumnType::MYSQL_TYPE_TINY
        | ColumnType::MYSQL_TYPE_SHORT
        | ColumnType::MYSQL_TYPE_INT24
        | ColumnType::MYSQL_TYPE_LONG
        | ColumnType::MYSQL_TYPE_LONGLONG
        | ColumnType::MYSQL_TYPE_YEAR => {
            let text = String::from_utf8_lossy(&bytes);
            if flags.contains(mysql::consts::ColumnFlags::UNSIGNED_FLAG) {
                text.trim()
                    .parse::<u64>()
                    .map_or_else(|_| Value::String(text.to_string()), |value| json!(value))
            } else {
                text.trim()
                    .parse::<i64>()
                    .map_or_else(|_| Value::String(text.to_string()), |value| json!(value))
            }
        }
        ColumnType::MYSQL_TYPE_DECIMAL
        | ColumnType::MYSQL_TYPE_NEWDECIMAL
        | ColumnType::MYSQL_TYPE_FLOAT
        | ColumnType::MYSQL_TYPE_DOUBLE => {
            let text = String::from_utf8_lossy(&bytes);
            let Ok(value) = text.trim().parse::<f64>() else {
                return Value::String(text.to_string());
            };
            serde_json::Number::from_f64(value)
                .map_or_else(|| Value::String(text.to_string()), Value::Number)
        }
        ColumnType::MYSQL_TYPE_BIT => {
            let value = bytes.iter().fold(0_u64, |acc, byte| (acc << 8) | u64::from(*byte));
            json!(value)
        }
        ColumnType::MYSQL_TYPE_JSON => match String::from_utf8(bytes) {
            Ok(text) => serde_json::from_str(&text).unwrap_or(Value::String(text)),
            Err(error) => Value::String(format!("0x{}", hex::encode(error.into_bytes()))),
        },
        _ => match String::from_utf8(bytes) {
            Ok(text) => Value::String(text),
            Err(error) => Value::String(format!("0x{}", hex::encode(error.into_bytes()))),
        },
    }
}

#[cfg(feature = "memory-postgres")]
fn postgres_query_rows(url: &str, sql: &str) -> Result<Vec<Value>, String> {
    let mut client =
        postgres::Client::connect(url, postgres::NoTls).map_err(|error| error.to_string())?;
    let rows = client.query(sql, &[]).map_err(|error| error.to_string())?;
    rows.into_iter()
        .map(|row| {
            let mut object = Map::new();
            for (index, column) in row.columns().iter().enumerate() {
                object.insert(column.name().to_string(), postgres_value_to_json(&row, index)?);
            }
            Ok(Value::Object(object))
        })
        .collect()
}

#[cfg(feature = "memory-postgres")]
fn postgres_value_to_json(row: &postgres::Row, index: usize) -> Result<Value, String> {
    use postgres::types::Type;
    let kind = row.columns()[index].type_();
    if *kind == Type::BOOL {
        return postgres_optional_value::<bool, _>(row, index, |value| json!(value));
    }
    if *kind == Type::INT2 {
        return postgres_optional_value::<i16, _>(row, index, |value| json!(value));
    }
    if *kind == Type::INT4 {
        return postgres_optional_value::<i32, _>(row, index, |value| json!(value));
    }
    if *kind == Type::INT8 {
        return postgres_optional_value::<i64, _>(row, index, |value| json!(value));
    }
    if *kind == Type::FLOAT4 {
        return postgres_optional_value::<f32, _>(row, index, |value| json!(value));
    }
    if *kind == Type::FLOAT8 {
        return postgres_optional_value::<f64, _>(row, index, |value| json!(value));
    }
    if *kind == Type::DATE {
        return postgres_optional_value::<chrono::NaiveDate, _>(row, index, |value| {
            json!(value.to_string())
        });
    }
    if *kind == Type::TIMESTAMP {
        return postgres_optional_value::<chrono::NaiveDateTime, _>(row, index, |value| {
            json!(value.to_string())
        });
    }
    if *kind == Type::TIMESTAMPTZ {
        return postgres_optional_value::<chrono::DateTime<chrono::Utc>, _>(row, index, |value| {
            json!(value.to_rfc3339())
        });
    }
    row.try_get::<_, Option<String>>(index)
        .map(|value| value.map_or(Value::Null, Value::String))
        .map_err(|_| format!("unsupported PostgreSQL column type: {}", kind.name()))
}

#[cfg(feature = "memory-postgres")]
fn postgres_optional_value<T, F>(
    row: &postgres::Row,
    index: usize,
    to_json: F,
) -> Result<Value, String>
where
    T: for<'a> postgres::types::FromSql<'a>,
    F: FnOnce(T) -> Value,
{
    row.try_get::<_, Option<T>>(index)
        .map(|value| value.map_or(Value::Null, to_json))
        .map_err(|error| error.to_string())
}

#[cfg(any(feature = "memory-mariadb", feature = "memory-postgres"))]
fn count_from_items(items: Vec<Value>) -> Result<u64, String> {
    let Some(value) =
        items.first().and_then(Value::as_object).and_then(|item| item.values().next())
    else {
        return Ok(0);
    };
    match value {
        Value::Number(number) => number
            .as_u64()
            .or_else(|| number.as_i64().map(|value| value.max(0) as u64))
            .ok_or_else(|| "SQL count value is not an integer".to_string()),
        Value::String(text) => text.parse::<u64>().map_err(|error| error.to_string()),
        Value::Null => Ok(0),
        _ => Err("SQL count value is not a scalar".to_string()),
    }
}

fn ensure_read_only_sql(sql: &str) -> Result<(), String> {
    let normalized = sql.trim().to_ascii_lowercase();
    if !(normalized.starts_with("select") || normalized.starts_with("with")) {
        return Err("ApiQL only permits read-only SELECT/CTE queries".to_string());
    }
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
    if forbidden.iter().any(|token| normalized.contains(token)) {
        return Err("ApiQL detected write or dangerous SQL".to_string());
    }
    Ok(())
}

fn render_sql_template(template: &str, params: &BTreeMap<String, Value>) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |captures: &regex::Captures<'_>| {
            let key = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
            lookup_param(params, key).map(sql_literal).unwrap_or_else(|| "NULL".to_string())
        })
        .into_owned()
}

fn lookup_param<'a>(params: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a Value> {
    if let Some(value) = params.get(key) {
        return Some(value);
    }
    let mut segments = key.split('.');
    let mut current = params.get(segments.next()?)?;
    for segment in segments {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn sql_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(value) => i32::from(*value).to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => format!("'{}'", value.replace('\'', "''")),
        Value::Array(values) => values.iter().map(sql_literal).collect::<Vec<_>>().join(", "),
        Value::Object(_) => {
            format!("'{}'", serde_json::to_string(value).unwrap_or_default().replace('\'', "''"))
        }
    }
}

fn sanitize_order_by(order_by: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = order_by.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|segment| {
            let captures = ORDER_BY_SEGMENT_RE
                .captures(segment)
                .ok_or_else(|| format!("invalid order field: {segment}"))?;
            let field = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
            let direction = captures
                .get(3)
                .map(|value| value.as_str().to_ascii_uppercase())
                .unwrap_or_else(|| "ASC".to_string());
            Ok(format!("{field} {direction}"))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|parts| (!parts.is_empty()).then(|| parts.join(", ")))
}

fn build_page(
    total_record: u64,
    current_page: u32,
    per_page: u32,
    item_len: usize,
) -> ApiQlQueryPage {
    let total_page = if total_record == 0 {
        0
    } else {
        total_record.div_ceil(u64::from(per_page)).min(u64::from(u32::MAX)) as u32
    };
    let from = if item_len == 0 {
        0
    } else {
        u64::from(current_page.saturating_sub(1)) * u64::from(per_page) + 1
    };
    let to = if item_len == 0 { 0 } else { from + item_len as u64 - 1 };
    ApiQlQueryPage { per_page, current_page, total_page, total_record, from, to }
}
