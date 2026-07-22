//! AI-DATA 查询执行运行时。
//!
//! 该模块负责把网关请求解析成统一的查询上下文，然后按连接类型执行 SQL、
//! Cube 或 HTTP 查询，并对结果做分页、字段裁剪、模板格式化和字段转换。
//! 安全边界集中在只读 SQL 校验、排序字段白名单化和 HTTP 请求范围构造。

#[cfg(feature = "memory-mariadb")]
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "memory-mariadb")]
use std::sync::Arc;
use std::time::Duration;

use once_cell::sync::Lazy;
#[cfg(feature = "memory-mariadb")]
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};
use vw_api_types::data::{
    AiDataConnectionDto, AiDataConnectionKind, AiDataCountMode, AiDataPageDto, AiDataQueryKind,
    AiDataQueryRequest, AiDataQueryResponse, AiDataReportDto, AiDataReportSourceDto,
    AiDataSettings, AiDataTemplateFieldDto, AiDataTransformerFieldDto,
};

static PLACEHOLDER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}").expect("placeholder regex"));
static SINGLE_PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}\s*$").expect("single placeholder regex")
});
static ORDER_BY_SEGMENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^([A-Za-z0-9_.]+)(\s+(ASC|DESC))?$").expect("order by regex"));
#[cfg(feature = "memory-mariadb")]
static MYSQL_POOLS: Lazy<Mutex<HashMap<String, Arc<mysql::Pool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const FORMAT_SYS_NUMBER: i64 = 1;
const FORMAT_SYS_PRICE: i64 = 2;
const FORMAT_SYS_ORDER_PRICE: i64 = 3;
const FORMAT_INT: i64 = 4;
const FORMAT_FLOAT: i64 = 5;
const FORMAT_STRING: i64 = 6;

#[derive(Clone)]
struct ResolvedQuery {
    connection: AiDataConnectionDto,
    report_config: Option<Value>,
    query_kind: AiDataQueryKind,
    sql: Option<String>,
    count_sql: Option<String>,
    cube_query: Option<Value>,
    http_method: String,
    http_path: Option<String>,
    http_body: Option<Value>,
    params: BTreeMap<String, Value>,
    page: u32,
    limit: u32,
    count_mode: AiDataCountMode,
    order_by: Option<String>,
    search_fields: Option<Vec<String>>,
    template_fields: Option<Vec<AiDataTemplateFieldDto>>,
    transformer_fields: Option<Vec<AiDataTransformerFieldDto>>,
    template_code: Option<String>,
    append_pagination: bool,
    debug: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ReadOnlySqlQuery {
    pub(crate) sql: String,
    pub(crate) count_sql: Option<String>,
    pub(crate) params: BTreeMap<String, Value>,
    pub(crate) page: u32,
    pub(crate) limit: u32,
    pub(crate) count_mode: AiDataCountMode,
    pub(crate) order_by: Option<String>,
    pub(crate) debug: bool,
}

/// 测试 AI-DATA 连接是否可用。
///
/// # 参数
///
/// - `connection`: 需要测试的数据连接。
/// - `timeout_secs`: HTTP/Cube 请求超时时间。
///
/// # 返回值
///
/// 成功时返回可展示的连接状态文本。
///
/// # 错误
///
/// 当连接配置不完整、底层数据源访问失败，或当前构建不支持该连接类型时返回错误文本。
pub(super) async fn test_connection(
    connection: &AiDataConnectionDto,
    timeout_secs: u32,
) -> Result<String, String> {
    match connection.kind {
        AiDataConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || {
                let conn = rusqlite::Connection::open(&path_clone).map_err(|e| e.to_string())?;
                conn.query_row("SELECT 1", [], |_row| Ok::<(), rusqlite::Error>(()))
                    .map_err(|e| e.to_string())?;
                Ok::<String, String>(format!("SQLite OK: {path}"))
            })
            .await
            .map_err(|e| e.to_string())?
        }
        AiDataConnectionKind::Cube => {
            let response = send_json_request(
                connection,
                reqwest::Method::GET,
                Some(&cube_endpoint(connection, "meta")?),
                None,
                timeout_secs,
            )
            .await?;
            Ok(format!("Cube OK: HTTP {}", response.0))
        }
        AiDataConnectionKind::Http => {
            let url = http_endpoint(connection, connection.default_path.as_deref())?;
            let response =
                send_json_request(connection, reqwest::Method::GET, Some(&url), None, timeout_secs)
                    .await?;
            Ok(format!("HTTP OK: {}", response.0))
        }
        AiDataConnectionKind::Mysql => test_mysql_connection(connection).await,
        AiDataConnectionKind::Postgres => test_postgres_connection(connection).await,
    }
}

/// 读取连接目录信息。
///
/// # 参数
///
/// - `connection`: 目标连接。
/// - `timeout_secs`: HTTP/Cube 请求超时时间。
///
/// # 返回值
///
/// 返回可供模型或前端消费的 JSON 目录信息。
///
/// # 错误
///
/// 当目录读取失败、配置缺失，或当前构建不支持该连接类型时返回错误文本。
pub(super) async fn connection_catalog(
    connection: &AiDataConnectionDto,
    timeout_secs: u32,
) -> Result<Value, String> {
    match connection.kind {
        AiDataConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || sqlite_catalog(&path_clone))
                .await
                .map_err(|e| e.to_string())?
        }
        AiDataConnectionKind::Cube => {
            let (_, value) = send_json_request(
                connection,
                reqwest::Method::GET,
                Some(&cube_endpoint(connection, "meta")?),
                None,
                timeout_secs,
            )
            .await?;
            Ok(value)
        }
        AiDataConnectionKind::Http => Ok(json!({
            "schema_hint": connection.schema_hint,
            "default_path": connection.default_path,
            "headers": connection.headers.keys().collect::<Vec<_>>()
        })),
        AiDataConnectionKind::Mysql => mysql_catalog(connection),
        AiDataConnectionKind::Postgres => postgres_catalog(connection),
    }
}

pub(crate) async fn execute_read_only_sql_query(
    connection: AiDataConnectionDto,
    query: ReadOnlySqlQuery,
) -> Result<AiDataQueryResponse, String> {
    if !connection.enabled {
        return Err("AI-DATA 连接已被禁用".to_string());
    }

    execute_sql_query(&ResolvedQuery {
        connection,
        report_config: None,
        query_kind: AiDataQueryKind::Sql,
        sql: Some(query.sql),
        count_sql: query.count_sql,
        cube_query: None,
        http_method: "GET".to_string(),
        http_path: None,
        http_body: None,
        params: query.params,
        page: query.page.max(1),
        limit: query.limit.clamp(1, 10_000),
        count_mode: query.count_mode,
        order_by: query.order_by,
        search_fields: None,
        template_fields: None,
        transformer_fields: None,
        template_code: None,
        append_pagination: true,
        debug: query.debug,
    })
    .await
}

/// 执行 AI-DATA 查询请求。
///
/// # 参数
///
/// - `settings`: 查询默认限制、超时等设置。
/// - `connections`: 当前可用连接集合。
/// - `reports`: 当前可用报表集合。
/// - `request`: 查询请求体。
///
/// # 返回值
///
/// 返回统一的分页查询响应。
///
/// # 错误
///
/// 当请求无法解析到有效连接/报表，或底层 SQL、Cube、HTTP 执行失败时返回错误文本。
pub(super) async fn execute_query(
    settings: &AiDataSettings,
    connections: &[AiDataConnectionDto],
    reports: &[AiDataReportDto],
    request: AiDataQueryRequest,
) -> Result<AiDataQueryResponse, String> {
    let resolved = resolve_query(settings, connections, reports, request)?;

    match resolved.query_kind {
        AiDataQueryKind::Sql => execute_sql_query(&resolved).await,
        AiDataQueryKind::Cube => execute_cube_query(&resolved, settings.default_timeout_secs).await,
        AiDataQueryKind::Http => execute_http_query(&resolved, settings.default_timeout_secs).await,
    }
}

fn resolve_query(
    settings: &AiDataSettings,
    connections: &[AiDataConnectionDto],
    reports: &[AiDataReportDto],
    request: AiDataQueryRequest,
) -> Result<ResolvedQuery, String> {
    let params = merge_search_condition(request.params, request.search_condition)?;
    let page = request.page.unwrap_or(1).max(1);
    let limit = request.limit.unwrap_or(settings.default_limit).clamp(1, 10_000);
    let count_mode = request.count.unwrap_or(AiDataCountMode::Disabled);

    // 报表查询优先复用已保存的数据源配置，临时请求字段只覆盖显式传入的部分。
    let (report_config, source_from_report) = if let Some(report_id) = request.report_id.as_ref() {
        let report = reports
            .iter()
            .find(|item| item.id == *report_id || item.slug == *report_id)
            .ok_or_else(|| "AI-DATA 报表不存在".to_string())?;
        let source_key = request
            .source_key
            .clone()
            .or_else(|| report.default_source_key.clone())
            .or_else(|| report.sources.first().map(|item| item.source_key.clone()))
            .ok_or_else(|| "AI-DATA 报表未配置可用数据源".to_string())?;
        let source = report
            .sources
            .iter()
            .find(|item| item.source_key == source_key)
            .ok_or_else(|| "AI-DATA 报表数据源不存在".to_string())?
            .clone();
        (Some(prepare_report_config(&report.report_config)), Some(source))
    } else {
        (None, None)
    };

    let connection_id = request
        .connection_id
        .clone()
        .or_else(|| source_from_report.as_ref().map(|item| item.connection_id.clone()))
        .ok_or_else(|| "AI-DATA 查询必须提供 connection_id 或 report_id".to_string())?;
    let connection = connections
        .iter()
        .find(|item| item.id == connection_id)
        .cloned()
        .ok_or_else(|| "AI-DATA 连接不存在".to_string())?;
    if !connection.enabled {
        return Err("AI-DATA 连接已被禁用".to_string());
    }

    let query_kind = request
        .query_kind
        .clone()
        .or_else(|| source_from_report.as_ref().map(|item| item.query_kind.clone()))
        .unwrap_or_else(|| connection_default_query_kind(&connection.kind));
    let resolved_order_by =
        request.order_by.or_else(|| report_default_order_by(report_config.as_ref()));
    let resolved_template_code = request.template_code.or_else(|| {
        report_default_template_code(
            report_config.as_ref(),
            source_from_report.as_ref().map(|item| item.source_key.as_str()),
        )
    });

    Ok(ResolvedQuery {
        connection,
        report_config,
        query_kind,
        sql: request.sql.or_else(|| source_from_report.as_ref().and_then(|item| item.sql.clone())),
        count_sql: request
            .count_sql
            .or_else(|| source_from_report.as_ref().and_then(|item| item.count_sql.clone())),
        cube_query: request
            .cube_query
            .or_else(|| source_from_report.as_ref().and_then(|item| item.cube_query.clone())),
        http_method: request
            .http_method
            .or_else(|| source_from_report.as_ref().map(|item| item.http_method.clone()))
            .unwrap_or_else(|| "GET".to_string()),
        http_path: request
            .http_path
            .or_else(|| source_from_report.as_ref().and_then(|item| item.http_path.clone())),
        http_body: request
            .http_body
            .or_else(|| source_from_report.as_ref().and_then(|item| item.http_body.clone())),
        params,
        page,
        limit,
        count_mode,
        order_by: resolved_order_by,
        search_fields: request.search_fields,
        template_fields: request.template_fields,
        transformer_fields: request.transformer_fields,
        template_code: resolved_template_code,
        append_pagination: source_from_report.as_ref().is_none_or(|item| item.append_pagination),
        debug: request.debug.unwrap_or(false),
    })
}

fn merge_search_condition(
    mut params: BTreeMap<String, Value>,
    search_condition: Option<Value>,
) -> Result<BTreeMap<String, Value>, String> {
    let Some(search_condition) = search_condition else {
        return Ok(params);
    };

    let object = match search_condition {
        Value::Object(object) => object,
        Value::String(text) => {
            let parsed = serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())?;
            parsed
                .as_object()
                .cloned()
                .ok_or_else(|| "searchCondition 必须是 JSON 对象".to_string())?
        }
        _ => return Err("searchCondition 必须是 JSON 对象".to_string()),
    };

    for (key, value) in object {
        // searchCondition 作为兼容旧前端的附加筛选，后写入以覆盖同名 params。
        params.insert(key, value);
    }
    Ok(params)
}

/// 返回过滤隐藏模块后的报表副本。
///
/// # 参数
///
/// - `report`: 原始持久化报表。
///
/// # 返回值
///
/// 返回适合 API 输出和查询使用的报表副本。
pub(super) fn prepared_report(report: &AiDataReportDto) -> AiDataReportDto {
    let mut report = report.clone();
    report.report_config = prepare_report_config(&report.report_config);
    report
}

/// 清理报表配置中不应参与展示或执行的隐藏模块。
///
/// # 参数
///
/// - `report_config`: 原始报表配置 JSON。
///
/// # 返回值
///
/// 返回处理后的配置；非对象配置保持原样。
pub(super) fn prepare_report_config(report_config: &Value) -> Value {
    let Value::Object(mut object) = report_config.clone() else {
        return report_config.clone();
    };

    if let Some(modules) = object.get_mut("modules").and_then(Value::as_array_mut) {
        modules.retain(module_is_visible);
    }

    if let Some(alert_menu) = object.get_mut("alertMenu").and_then(Value::as_object_mut)
        && is_visible_flag(alert_menu.get("show"))
    {
        alert_menu.entry("menu".to_string()).or_insert_with(|| Value::Array(Vec::new()));
        alert_menu.entry("variable".to_string()).or_insert_with(|| Value::Object(Map::new()));
    }

    Value::Object(object)
}

fn module_is_visible(module: &Value) -> bool {
    let Value::Object(object) = module else {
        return true;
    };
    is_visible_flag(object.get("show"))
}

fn is_visible_flag(flag: Option<&Value>) -> bool {
    match flag {
        None => true,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_i64().unwrap_or(1) != 0,
        Some(Value::String(value)) => {
            let normalized = value.trim().to_ascii_uppercase();
            !(normalized.is_empty()
                || normalized == "F"
                || normalized == "FALSE"
                || normalized == "0")
        }
        Some(_) => true,
    }
}

fn report_default_order_by(report_config: Option<&Value>) -> Option<String> {
    let modules = report_config?.get("modules")?.as_array()?;
    let table = modules
        .iter()
        .find(|item| item.get("type").and_then(|value| value.as_str()) == Some("table"))?;
    let default_sort_fields = table.get("defaultSortFields")?.as_array()?;
    let mut parts = Vec::new();
    for field in default_sort_fields {
        let sort_field_id = field.get("sortFieldId").and_then(|value| value.as_str())?;
        let sort_method = field
            .get("sortMethod")
            .and_then(|value| value.as_str())
            .unwrap_or("asc")
            .to_ascii_uppercase();
        parts.push(format!("{sort_field_id} {sort_method}"));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn report_default_template_code(
    report_config: Option<&Value>,
    source_key: Option<&str>,
) -> Option<String> {
    let modules = report_config?.get("modules")?.as_array()?;
    modules
        .iter()
        .find(|item| {
            item.get("template_code").and_then(Value::as_str).is_some()
                && source_key
                    .is_none_or(|key| item.get("source").and_then(Value::as_str) == Some(key))
                && item.get("type").and_then(Value::as_str) == Some("table")
        })
        .or_else(|| {
            modules.iter().find(|item| {
                item.get("template_code").and_then(Value::as_str).is_some()
                    && source_key
                        .is_none_or(|key| item.get("source").and_then(Value::as_str) == Some(key))
            })
        })
        .or_else(|| {
            modules.iter().find(|item| {
                item.get("template_code").and_then(Value::as_str).is_some()
                    && item.get("type").and_then(Value::as_str) == Some("table")
            })
        })
        .or_else(|| {
            modules.iter().find(|item| item.get("template_code").and_then(Value::as_str).is_some())
        })
        .and_then(|item| item.get("template_code").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn connection_default_query_kind(kind: &AiDataConnectionKind) -> AiDataQueryKind {
    match kind {
        AiDataConnectionKind::Cube => AiDataQueryKind::Cube,
        AiDataConnectionKind::Http => AiDataQueryKind::Http,
        AiDataConnectionKind::Sqlite
        | AiDataConnectionKind::Mysql
        | AiDataConnectionKind::Postgres => AiDataQueryKind::Sql,
    }
}

async fn execute_sql_query(resolved: &ResolvedQuery) -> Result<AiDataQueryResponse, String> {
    let sql_template = resolved.sql.as_ref().ok_or_else(|| "缺少 SQL 查询模板".to_string())?;
    let base_sql = render_sql_template(sql_template, &resolved.params)?;
    ensure_read_only_sql(&base_sql)?;

    // ORDER BY 不能直接拼接用户输入，必须先限制成字段名 + ASC/DESC 片段。
    let order_by = sanitize_order_by(resolved.order_by.as_deref())?;
    let offset = u64::from(resolved.page.saturating_sub(1)) * u64::from(resolved.limit);
    let paged_sql = if resolved.append_pagination {
        match order_by.as_deref() {
            Some(order) => format!(
                "SELECT * FROM ({base_sql}) AS vw_ai_data_base ORDER BY {order} LIMIT {} OFFSET {offset}",
                resolved.limit
            ),
            None => format!(
                "SELECT * FROM ({base_sql}) AS vw_ai_data_base LIMIT {} OFFSET {offset}",
                resolved.limit
            ),
        }
    } else {
        base_sql.clone()
    };

    let rendered_count_sql = match resolved.count_mode {
        AiDataCountMode::Disabled => None,
        AiDataCountMode::Enabled | AiDataCountMode::Only => Some(
            resolved
                .count_sql
                .as_ref()
                .map(|value| render_sql_template(value, &resolved.params))
                .transpose()?
                .unwrap_or_else(|| {
                    format!("SELECT COUNT(1) AS count FROM ({base_sql}) AS vw_ai_data_count")
                }),
        ),
    };

    let (total_record, mut items) = match resolved.count_mode {
        AiDataCountMode::Disabled => {
            (0, execute_sql_rows(&resolved.connection, paged_sql.clone()).await?)
        }
        AiDataCountMode::Only => {
            let count_sql = rendered_count_sql
                .as_ref()
                .expect("count SQL is rendered for count-only mode")
                .clone();
            (execute_sql_count(&resolved.connection, count_sql).await?, Vec::new())
        }
        AiDataCountMode::Enabled => {
            let count_sql = rendered_count_sql
                .as_ref()
                .expect("count SQL is rendered for enabled count mode")
                .clone();
            let (total_record, items) = tokio::try_join!(
                execute_sql_count(&resolved.connection, count_sql),
                execute_sql_rows(&resolved.connection, paged_sql.clone()),
            )?;
            (total_record, items)
        }
    };
    post_process_items(&mut items, resolved)?;

    let effective_total = match resolved.count_mode {
        AiDataCountMode::Disabled => items.len() as u64,
        AiDataCountMode::Enabled | AiDataCountMode::Only => total_record,
    };
    let page = build_page(effective_total, resolved.page, resolved.limit, items.len());
    let has_next_page = if matches!(resolved.count_mode, AiDataCountMode::Disabled) {
        items.len() as u32 >= resolved.limit
    } else {
        page.current_page < page.total_page
    };

    Ok(AiDataQueryResponse {
        page,
        items,
        report_config: resolved.report_config.clone(),
        next_cursor: None,
        has_next_page,
        debug: resolved.debug.then(|| {
            json!({
                "query_kind": "sql",
                "rendered_sql": paged_sql,
                "count_sql": rendered_count_sql,
                "base_sql": base_sql,
                "connection_id": resolved.connection.id,
                "connection_kind": resolved.connection.kind,
            })
        }),
    })
}

async fn execute_cube_query(
    resolved: &ResolvedQuery,
    timeout_secs: u32,
) -> Result<AiDataQueryResponse, String> {
    if matches!(resolved.count_mode, AiDataCountMode::Only) {
        return Err("Cube 查询暂不支持 count_only".to_string());
    }
    let cube_query = resolved.cube_query.as_ref().ok_or_else(|| "缺少 Cube query".to_string())?;
    let rendered_query = render_json_template(cube_query, &resolved.params);
    let body = json!({ "query": rendered_query });
    let url = cube_endpoint(&resolved.connection, "load")?;
    let (status, value) = send_json_request(
        &resolved.connection,
        reqwest::Method::POST,
        Some(&url),
        Some(&body),
        timeout_secs,
    )
    .await?;
    let (mut items, total_record) = extract_items_and_total(value.clone());
    post_process_items(&mut items, resolved)?;
    let page = build_page(
        total_record.unwrap_or(items.len() as u64),
        resolved.page,
        resolved.limit,
        items.len(),
    );
    Ok(AiDataQueryResponse {
        page,
        items,
        report_config: resolved.report_config.clone(),
        next_cursor: None,
        has_next_page: false,
        debug: resolved.debug.then(|| {
            json!({
                "query_kind": "cube",
                "url": url,
                "status": status,
                "request_body": body,
                "response": value,
            })
        }),
    })
}

async fn execute_http_query(
    resolved: &ResolvedQuery,
    timeout_secs: u32,
) -> Result<AiDataQueryResponse, String> {
    if matches!(resolved.count_mode, AiDataCountMode::Only) {
        return Err("HTTP 查询暂不支持 count_only".to_string());
    }
    let method = parse_http_method(&resolved.http_method)?;
    let path = render_string_template(
        resolved.http_path.as_deref().or(resolved.connection.default_path.as_deref()).unwrap_or(""),
        &resolved.params,
    );
    let url = http_endpoint(&resolved.connection, Some(&path))?;
    let body =
        resolved.http_body.as_ref().map(|value| render_json_template(value, &resolved.params));
    let (status, value) = if method == reqwest::Method::GET || method == reqwest::Method::HEAD {
        send_query_request(&resolved.connection, &url, &resolved.params, timeout_secs).await?
    } else {
        let final_body = merge_json_object(body, &resolved.params);
        send_json_request(
            &resolved.connection,
            method,
            Some(&url),
            final_body.as_ref(),
            timeout_secs,
        )
        .await?
    };
    let (mut items, total_record) = extract_items_and_total(value.clone());
    post_process_items(&mut items, resolved)?;
    let page = build_page(
        total_record.unwrap_or(items.len() as u64),
        resolved.page,
        resolved.limit,
        items.len(),
    );
    Ok(AiDataQueryResponse {
        page,
        items,
        report_config: resolved.report_config.clone(),
        next_cursor: None,
        has_next_page: false,
        debug: resolved.debug.then(|| {
            json!({
                "query_kind": "http",
                "url": url,
                "status": status,
                "response": value,
            })
        }),
    })
}

async fn execute_sql_rows(
    connection: &AiDataConnectionDto,
    sql: String,
) -> Result<Vec<Value>, String> {
    match connection.kind {
        AiDataConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            tokio::task::spawn_blocking(move || sqlite_query_rows(&path, &sql))
                .await
                .map_err(|e| e.to_string())?
        }
        AiDataConnectionKind::Mysql => execute_mysql_rows(connection, sql).await,
        AiDataConnectionKind::Postgres => execute_postgres_rows(connection, sql).await,
        AiDataConnectionKind::Cube | AiDataConnectionKind::Http => {
            Err("当前连接类型不支持 SQL 查询".to_string())
        }
    }
}

async fn execute_sql_count(connection: &AiDataConnectionDto, sql: String) -> Result<u64, String> {
    match connection.kind {
        AiDataConnectionKind::Sqlite => {
            let path = sqlite_path(connection)?;
            tokio::task::spawn_blocking(move || sqlite_query_count(&path, &sql))
                .await
                .map_err(|e| e.to_string())?
        }
        AiDataConnectionKind::Mysql => execute_mysql_count(connection, sql).await,
        AiDataConnectionKind::Postgres => execute_postgres_count(connection, sql).await,
        AiDataConnectionKind::Cube | AiDataConnectionKind::Http => {
            Err("当前连接类型不支持 SQL 计数查询".to_string())
        }
    }
}

fn sqlite_path(connection: &AiDataConnectionDto) -> Result<String, String> {
    connection
        .sqlite_path
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "SQLite 连接缺少 sqlite_path".to_string())
}

fn database_connection_url(
    connection: &AiDataConnectionDto,
    label: &str,
) -> Result<String, String> {
    connection
        .connection_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{label} 连接缺少 connection_url"))
}

fn mysql_executor_disabled() -> String {
    "当前构建未启用 MySQL 执行器；如需接入请启用 memory-mariadb feature".to_string()
}

fn postgres_executor_disabled() -> String {
    "当前构建未启用 PostgreSQL 执行器；如需接入请启用 memory-postgres feature".to_string()
}

#[cfg(not(feature = "memory-mariadb"))]
fn mysql_catalog(connection: &AiDataConnectionDto) -> Result<Value, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err("当前构建未启用 MySQL 目录读取；如需接入请启用 memory-mariadb feature".to_string())
}

#[cfg(feature = "memory-mariadb")]
fn mysql_catalog(connection: &AiDataConnectionDto) -> Result<Value, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err("MySQL 目录读取暂未接入 AI-DATA catalog".to_string())
}

#[cfg(not(feature = "memory-postgres"))]
fn postgres_catalog(connection: &AiDataConnectionDto) -> Result<Value, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err("当前构建未启用 PostgreSQL 目录读取；如需接入请启用 memory-postgres feature".to_string())
}

#[cfg(feature = "memory-postgres")]
fn postgres_catalog(connection: &AiDataConnectionDto) -> Result<Value, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err("PostgreSQL 目录读取暂未接入 AI-DATA catalog".to_string())
}

#[cfg(not(feature = "memory-mariadb"))]
async fn test_mysql_connection(connection: &AiDataConnectionDto) -> Result<String, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err(mysql_executor_disabled())
}

#[cfg(feature = "memory-mariadb")]
async fn test_mysql_connection(connection: &AiDataConnectionDto) -> Result<String, String> {
    let url = database_connection_url(connection, "MySQL")?;
    tokio::task::spawn_blocking(move || {
        use mysql::prelude::Queryable;

        let pool = mysql_pool(&url).map_err(|e| format!("MySQL 连接失败: {e}"))?;
        let mut conn = pool.get_conn().map_err(|e| format!("MySQL 连接失败: {e}"))?;
        conn.query_drop("SELECT 1").map_err(|e| format!("MySQL 连接失败: {e}"))?;
        Ok::<String, String>("MySQL OK".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(not(feature = "memory-postgres"))]
async fn test_postgres_connection(connection: &AiDataConnectionDto) -> Result<String, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err(postgres_executor_disabled())
}

#[cfg(feature = "memory-postgres")]
async fn test_postgres_connection(connection: &AiDataConnectionDto) -> Result<String, String> {
    let url = database_connection_url(connection, "PostgreSQL")?;
    tokio::task::spawn_blocking(move || {
        let mut client =
            postgres::Client::connect(&url, postgres::NoTls).map_err(|e| e.to_string())?;
        client.simple_query("SELECT 1").map_err(|e| e.to_string())?;
        Ok::<String, String>("PostgreSQL OK".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(not(feature = "memory-mariadb"))]
async fn execute_mysql_rows(
    connection: &AiDataConnectionDto,
    _sql: String,
) -> Result<Vec<Value>, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err(mysql_executor_disabled())
}

#[cfg(feature = "memory-mariadb")]
async fn execute_mysql_rows(
    connection: &AiDataConnectionDto,
    sql: String,
) -> Result<Vec<Value>, String> {
    let url = database_connection_url(connection, "MySQL")?;
    tokio::task::spawn_blocking(move || mysql_query_rows(&url, &sql))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(feature = "memory-postgres"))]
async fn execute_postgres_rows(
    connection: &AiDataConnectionDto,
    _sql: String,
) -> Result<Vec<Value>, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err(postgres_executor_disabled())
}

#[cfg(feature = "memory-postgres")]
async fn execute_postgres_rows(
    connection: &AiDataConnectionDto,
    sql: String,
) -> Result<Vec<Value>, String> {
    let url = database_connection_url(connection, "PostgreSQL")?;
    tokio::task::spawn_blocking(move || postgres_query_rows(&url, &sql))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(feature = "memory-mariadb"))]
async fn execute_mysql_count(
    connection: &AiDataConnectionDto,
    _sql: String,
) -> Result<u64, String> {
    let _ = database_connection_url(connection, "MySQL")?;
    Err(mysql_executor_disabled())
}

#[cfg(feature = "memory-mariadb")]
async fn execute_mysql_count(connection: &AiDataConnectionDto, sql: String) -> Result<u64, String> {
    let url = database_connection_url(connection, "MySQL")?;
    tokio::task::spawn_blocking(move || {
        mysql_query_rows(&url, &sql).and_then(count_from_query_items)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(not(feature = "memory-postgres"))]
async fn execute_postgres_count(
    connection: &AiDataConnectionDto,
    _sql: String,
) -> Result<u64, String> {
    let _ = database_connection_url(connection, "PostgreSQL")?;
    Err(postgres_executor_disabled())
}

#[cfg(feature = "memory-postgres")]
async fn execute_postgres_count(
    connection: &AiDataConnectionDto,
    sql: String,
) -> Result<u64, String> {
    let url = database_connection_url(connection, "PostgreSQL")?;
    tokio::task::spawn_blocking(move || {
        postgres_query_rows(&url, &sql).and_then(count_from_query_items)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn sqlite_catalog(path: &str) -> Result<Value, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|e| e.to_string())?;
    let table_names = stmt
        .query_map([], |row| row.get::<usize, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut tables = Vec::new();
    for table_name in table_names {
        // 表名来自 sqlite_master，但仍转义单引号，避免构造 PRAGMA 时破坏语句边界。
        let pragma_sql = format!("PRAGMA table_info('{}')", table_name.replace('\'', "''"));
        let mut pragma = conn.prepare(&pragma_sql).map_err(|e| e.to_string())?;
        let columns = pragma
            .query_map([], |row| {
                Ok(json!({
                    "cid": row.get::<usize, i64>(0)?,
                    "name": row.get::<usize, String>(1)?,
                    "type": row.get::<usize, String>(2)?,
                    "not_null": row.get::<usize, i64>(3)? != 0,
                    "default_value": row.get::<usize, Option<String>>(4)?,
                    "primary_key": row.get::<usize, i64>(5)? != 0,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        tables.push(json!({ "name": table_name, "columns": columns }));
    }
    Ok(json!({ "tables": tables }))
}

fn sqlite_query_rows(path: &str, sql: &str) -> Result<Vec<Value>, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let column_names = stmt.column_names().iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut object = Map::new();
        for (index, name) in column_names.iter().enumerate() {
            let value_ref = row.get_ref(index).map_err(|e| e.to_string())?;
            object.insert(name.clone(), sqlite_value_ref_to_json(value_ref));
        }
        items.push(Value::Object(object));
    }
    Ok(items)
}

fn sqlite_query_count(path: &str, sql: &str) -> Result<u64, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let Some(row) = rows.next().map_err(|e| e.to_string())? else {
        return Ok(0);
    };
    if let Ok(value) = row.get::<usize, i64>(0) {
        return Ok(value.max(0) as u64);
    }
    if let Ok(value) = row.get::<usize, String>(0) {
        return value.parse::<u64>().map_err(|e| e.to_string());
    }
    Ok(0)
}

fn sqlite_value_ref_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => json!(v),
        ValueRef::Real(v) => json!(v),
        ValueRef::Text(v) => Value::String(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => Value::String(format!("0x{}", hex::encode(v))),
    }
}

#[cfg(feature = "memory-mariadb")]
fn mysql_query_rows(url: &str, sql: &str) -> Result<Vec<Value>, String> {
    use mysql::prelude::Queryable;

    let pool = mysql_pool(url)?;
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let result = conn.query_iter(sql).map_err(|e| e.to_string())?;
    let columns = result
        .columns()
        .as_ref()
        .iter()
        .map(|column| (column.name_str().to_string(), column.column_type(), column.flags()))
        .collect::<Vec<_>>();
    let mut items = Vec::new();
    for row_result in result {
        let row = row_result.map_err(|e| e.to_string())?;
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
    let opts = mysql::Opts::from_url(url).map_err(|e| format!("Invalid MySQL URL: {e}"))?;
    let pool = Arc::new(mysql::Pool::new(opts).map_err(|e| e.to_string())?);
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
        mysql::Value::Date(year, month, day, 0, 0, 0, 0) => {
            json!(format!("{year:04}-{month:02}-{day:02}"))
        }
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
        | ColumnType::MYSQL_TYPE_YEAR => mysql_integer_bytes_to_json(&bytes, flags),
        ColumnType::MYSQL_TYPE_DECIMAL
        | ColumnType::MYSQL_TYPE_NEWDECIMAL
        | ColumnType::MYSQL_TYPE_FLOAT
        | ColumnType::MYSQL_TYPE_DOUBLE => mysql_float_bytes_to_json(&bytes),
        ColumnType::MYSQL_TYPE_BIT => mysql_bit_bytes_to_json(&bytes),
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

#[cfg(feature = "memory-mariadb")]
fn mysql_integer_bytes_to_json(bytes: &[u8], flags: mysql::consts::ColumnFlags) -> Value {
    let text = String::from_utf8_lossy(bytes);
    if flags.contains(mysql::consts::ColumnFlags::UNSIGNED_FLAG) {
        return text
            .trim()
            .parse::<u64>()
            .map_or_else(|_| Value::String(text.to_string()), |value| json!(value));
    }
    text.trim()
        .parse::<i64>()
        .map_or_else(|_| Value::String(text.to_string()), |value| json!(value))
}

#[cfg(feature = "memory-mariadb")]
fn mysql_float_bytes_to_json(bytes: &[u8]) -> Value {
    let text = String::from_utf8_lossy(bytes);
    let Ok(value) = text.trim().parse::<f64>() else {
        return Value::String(text.to_string());
    };
    serde_json::Number::from_f64(value)
        .map_or_else(|| Value::String(text.to_string()), Value::Number)
}

#[cfg(feature = "memory-mariadb")]
fn mysql_bit_bytes_to_json(bytes: &[u8]) -> Value {
    if bytes.is_empty() {
        return json!(0_u64);
    }
    let value = bytes.iter().fold(0_u64, |acc, byte| (acc << 8) | u64::from(*byte));
    json!(value)
}

#[cfg(feature = "memory-postgres")]
fn postgres_query_rows(url: &str, sql: &str) -> Result<Vec<Value>, String> {
    let mut client = postgres::Client::connect(url, postgres::NoTls).map_err(|e| e.to_string())?;
    let rows = client.query(sql, &[]).map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        let mut object = Map::new();
        for (index, column) in row.columns().iter().enumerate() {
            object.insert(column.name().to_string(), postgres_cell_to_json(&row, index)?);
        }
        items.push(Value::Object(object));
    }
    Ok(items)
}

#[cfg(feature = "memory-postgres")]
fn postgres_cell_to_json(row: &postgres::Row, index: usize) -> Result<Value, String> {
    let column_type = row.columns()[index].type_();
    use postgres::types::Type;

    if *column_type == Type::BOOL {
        return postgres_optional_cell::<bool, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::INT2 {
        return postgres_optional_cell::<i16, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::INT4 {
        return postgres_optional_cell::<i32, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::INT8 {
        return postgres_optional_cell::<i64, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::FLOAT4 {
        return postgres_optional_cell::<f32, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::FLOAT8 {
        return postgres_optional_cell::<f64, _>(row, index, |value| json!(value));
    }
    if *column_type == Type::DATE {
        return postgres_optional_cell::<chrono::NaiveDate, _>(row, index, |value| {
            json!(value.to_string())
        });
    }
    if *column_type == Type::TIMESTAMP {
        return postgres_optional_cell::<chrono::NaiveDateTime, _>(row, index, |value| {
            json!(value.to_string())
        });
    }
    if *column_type == Type::TIMESTAMPTZ {
        return postgres_optional_cell::<chrono::DateTime<chrono::Utc>, _>(row, index, |value| {
            json!(value.to_rfc3339())
        });
    }

    row.try_get::<_, Option<String>>(index)
        .map(|value| value.map_or(Value::Null, Value::String))
        .map_err(|_| format!("PostgreSQL column type is not supported: {}", column_type.name()))
}

#[cfg(feature = "memory-postgres")]
fn postgres_optional_cell<T, F>(
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
        .map_err(|e| e.to_string())
}

#[cfg(any(feature = "memory-mariadb", feature = "memory-postgres"))]
fn count_from_query_items(items: Vec<Value>) -> Result<u64, String> {
    let Some(first) = items.first().and_then(Value::as_object) else {
        return Ok(0);
    };
    let Some(value) = first.values().next() else {
        return Ok(0);
    };
    match value {
        Value::Number(number) => number
            .as_u64()
            .or_else(|| number.as_i64().map(|value| value.max(0) as u64))
            .ok_or_else(|| "SQL count value is not an integer".to_string()),
        Value::String(text) => text.parse::<u64>().map_err(|e| e.to_string()),
        Value::Null => Ok(0),
        _ => Err("SQL count value is not a scalar".to_string()),
    }
}

fn ensure_read_only_sql(sql: &str) -> Result<(), String> {
    let normalized = sql.trim().to_ascii_lowercase();
    if !(normalized.starts_with("select") || normalized.starts_with("with")) {
        return Err("AI-DATA 仅允许只读 SELECT/CTE 查询".to_string());
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
    // 这里是轻量防线，不替代数据库权限；连接本身仍应使用只读账号或只读文件权限。
    if forbidden.iter().any(|token| normalized.contains(token)) {
        return Err("AI-DATA 检测到潜在写入或危险 SQL".to_string());
    }
    Ok(())
}

fn render_sql_template(template: &str, params: &BTreeMap<String, Value>) -> Result<String, String> {
    let rendered = PLACEHOLDER_RE.replace_all(template, |captures: &regex::Captures<'_>| {
        let key = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
        lookup_param(params, key).map(sql_literal).unwrap_or_else(|| "NULL".to_string())
    });
    Ok(rendered.into_owned())
}

fn render_string_template(template: &str, params: &BTreeMap<String, Value>) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |captures: &regex::Captures<'_>| {
            let key = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
            lookup_param(params, key).map(string_literal).unwrap_or_default()
        })
        .into_owned()
}

fn render_json_template(value: &Value, params: &BTreeMap<String, Value>) -> Value {
    match value {
        Value::String(text) => {
            if let Some(captures) = SINGLE_PLACEHOLDER_RE.captures(text) {
                let key = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
                // 整个字符串就是占位符时保留原 JSON 类型，避免数字/布尔被转成字符串。
                lookup_param(params, key).cloned().unwrap_or(Value::Null)
            } else {
                Value::String(render_string_template(text, params))
            }
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| render_json_template(item, params)).collect())
        }
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), render_json_template(value, params)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn lookup_param<'a>(params: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a Value> {
    if let Some(value) = params.get(key) {
        return Some(value);
    }
    let mut segments = key.split('.');
    let first = segments.next()?;
    let mut current = params.get(first)?;
    for segment in segments {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn sql_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(v) => {
            if *v {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("'{}'", v.replace('\'', "''")),
        Value::Array(values) => values.iter().map(sql_literal).collect::<Vec<_>>().join(", "),
        Value::Object(_) => {
            format!("'{}'", serde_json::to_string(value).unwrap_or_default().replace('\'', "''"))
        }
    }
}

fn string_literal(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Array(values) => values.iter().map(string_literal).collect::<Vec<_>>().join(","),
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn sanitize_order_by(order_by: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = order_by.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let mut parts = Vec::new();
    for segment in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let captures = ORDER_BY_SEGMENT_RE
            .captures(segment)
            .ok_or_else(|| format!("非法排序字段: {segment}"))?;
        let field = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
        let direction = captures
            .get(3)
            .map(|value| value.as_str().to_ascii_uppercase())
            .unwrap_or_else(|| "ASC".to_string());
        parts.push(format!("{field} {direction}"));
    }
    Ok((!parts.is_empty()).then(|| parts.join(", ")))
}

fn build_page(
    total_record: u64,
    current_page: u32,
    per_page: u32,
    item_len: usize,
) -> AiDataPageDto {
    let total_page = if total_record == 0 {
        0
    } else {
        ((total_record + u64::from(per_page) - 1) / u64::from(per_page)) as u32
    };
    let from = if item_len == 0 {
        0
    } else {
        u64::from(current_page.saturating_sub(1)) * u64::from(per_page) + 1
    };
    let to = if item_len == 0 { 0 } else { from + item_len as u64 - 1 };
    AiDataPageDto { per_page, current_page, total_page, total_record, from, to }
}

fn apply_search_fields(items: &mut [Value], search_fields: Option<&[String]>) {
    let Some(fields) = search_fields else {
        return;
    };
    for item in items {
        if let Value::Object(object) = item {
            object.retain(|key, _| fields.iter().any(|field| field == key));
        }
    }
}

fn post_process_items(items: &mut [Value], resolved: &ResolvedQuery) -> Result<(), String> {
    let template_formats = resolve_template_formats(
        resolved.report_config.as_ref(),
        resolved.template_fields.as_deref(),
        resolved.template_code.as_deref(),
    );
    apply_template_fields(items, &template_formats);
    apply_search_fields(items, resolved.search_fields.as_deref());
    apply_transformer_fields(
        items,
        resolved.transformer_fields.as_deref(),
        resolved.report_config.as_ref(),
        &resolved.params,
    )
}

fn resolve_template_formats(
    report_config: Option<&Value>,
    explicit_fields: Option<&[AiDataTemplateFieldDto]>,
    template_code: Option<&str>,
) -> BTreeMap<String, i64> {
    if let Some(fields) = explicit_fields {
        return template_field_map_from_slice(fields);
    }

    let Some(template_code) = template_code.filter(|value| !value.trim().is_empty()) else {
        return BTreeMap::new();
    };

    let external_fields = query_mock_template_source(report_config, template_code);
    if !external_fields.is_empty() {
        // mock 模板源更接近设计器预览数据，优先于静态 templateFields 配置。
        return external_fields;
    }

    lookup_template_fields(report_config, template_code)
}

fn query_mock_template_source(
    report_config: Option<&Value>,
    template_code: &str,
) -> BTreeMap<String, i64> {
    let Some(report_config) = report_config else {
        return BTreeMap::new();
    };

    for key in [
        "mock_template_source",
        "mockTemplateSource",
        "template_source",
        "templateSource",
        "template_service_mock",
        "templateServiceMock",
    ] {
        if let Some(value) = report_config.get(key) {
            let map = template_field_map_from_mock_source(value, template_code);
            if !map.is_empty() {
                return map;
            }
        }
    }

    if let Some(Value::Object(mock)) = report_config.get("mock") {
        for key in [
            "template_source",
            "templateSource",
            "template_catalog",
            "templateCatalog",
            "templates",
        ] {
            if let Some(value) = mock.get(key) {
                let map = template_field_map_from_mock_source(value, template_code);
                if !map.is_empty() {
                    return map;
                }
            }
        }
    }

    BTreeMap::new()
}

fn template_field_map_from_mock_source(
    value: &Value,
    template_code: &str,
) -> BTreeMap<String, i64> {
    match value {
        Value::Array(items) => {
            for item in items {
                let map = template_field_map_from_mock_source(item, template_code);
                if !map.is_empty() {
                    return map;
                }
            }
            BTreeMap::new()
        }
        Value::Object(object) => {
            if let Some(template) = object.get(template_code) {
                let map = template_field_map_from_value(template);
                if !map.is_empty() {
                    return map;
                }
            }

            for key in ["templates", "items", "list", "data", "records"] {
                if let Some(nested) = object.get(key) {
                    let map = template_field_map_from_mock_source(nested, template_code);
                    if !map.is_empty() {
                        return map;
                    }
                }
            }

            if object.get("template_code").and_then(Value::as_str) == Some(template_code)
                || object.get("templateCode").and_then(Value::as_str) == Some(template_code)
                || object.get("code").and_then(Value::as_str) == Some(template_code)
            {
                let map = template_field_map_from_value(value);
                if !map.is_empty() {
                    return map;
                }
            }

            BTreeMap::new()
        }
        _ => BTreeMap::new(),
    }
}

fn template_field_map_from_slice(fields: &[AiDataTemplateFieldDto]) -> BTreeMap<String, i64> {
    let mut map = BTreeMap::new();
    for field in fields {
        let normalized = normalize_field_code(&field.code);
        if !normalized.is_empty() {
            map.insert(normalized, field.value_type);
        }
    }
    map
}

fn lookup_template_fields(
    report_config: Option<&Value>,
    template_code: &str,
) -> BTreeMap<String, i64> {
    let Some(report_config) = report_config else {
        return BTreeMap::new();
    };

    for key in ["templates", "templateFields", "template_fields"] {
        if let Some(Value::Object(object)) = report_config.get(key)
            && let Some(value) = object.get(template_code)
        {
            let map = template_field_map_from_value(value);
            if !map.is_empty() {
                return map;
            }
        }
    }

    if let Some(modules) = report_config.get("modules").and_then(Value::as_array) {
        for module in modules {
            if module.get("template_code").and_then(Value::as_str) != Some(template_code) {
                continue;
            }
            for key in ["fields", "templateFields", "template_fields"] {
                if let Some(value) = module.get(key) {
                    let map = template_field_map_from_value(value);
                    if !map.is_empty() {
                        return map;
                    }
                }
            }
        }
    }

    BTreeMap::new()
}

fn template_field_map_from_value(value: &Value) -> BTreeMap<String, i64> {
    match value {
        Value::Array(items) => {
            let mut map = BTreeMap::new();
            for item in items {
                let Some(object) = item.as_object() else {
                    continue;
                };
                let code = object
                    .get("code")
                    .or_else(|| object.get("field"))
                    .or_else(|| object.get("key"))
                    .and_then(Value::as_str);
                let value_type = object
                    .get("valueType")
                    .or_else(|| object.get("value_type"))
                    .or_else(|| object.get("sysFormat"))
                    .and_then(parse_value_format);
                let (Some(code), Some(value_type)) = (code, value_type) else {
                    continue;
                };
                let normalized = normalize_field_code(code);
                if !normalized.is_empty() {
                    map.insert(normalized, value_type);
                }
            }
            map
        }
        Value::Object(object) => {
            object.get("fields").map(template_field_map_from_value).unwrap_or_default()
        }
        _ => BTreeMap::new(),
    }
}

fn parse_value_format(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "1" | "sys_number" => Some(FORMAT_SYS_NUMBER),
            "2" | "sys_price" => Some(FORMAT_SYS_PRICE),
            "3" | "sys_order_price" => Some(FORMAT_SYS_ORDER_PRICE),
            "4" | "int" => Some(FORMAT_INT),
            "5" | "float" => Some(FORMAT_FLOAT),
            "6" | "string" => Some(FORMAT_STRING),
            _ => None,
        },
        _ => None,
    }
}

fn apply_template_fields(items: &mut [Value], formats: &BTreeMap<String, i64>) {
    if formats.is_empty() {
        return;
    }
    for item in items {
        let Value::Object(object) = item else {
            continue;
        };
        for (key, value) in object.iter_mut() {
            let normalized = normalize_field_code(key);
            let Some(format_code) = formats.get(&normalized) else {
                continue;
            };
            *value = format_value_by_type(value.clone(), *format_code);
        }
    }
}

fn format_value_by_type(value: Value, format_code: i64) -> Value {
    match format_code {
        FORMAT_SYS_NUMBER => normalize_numeric_value(value, 2),
        FORMAT_SYS_PRICE | FORMAT_SYS_ORDER_PRICE => normalize_numeric_value(value, 2),
        FORMAT_INT => normalize_int_value(value),
        FORMAT_FLOAT => normalize_float_value(value),
        FORMAT_STRING => Value::String(string_literal(&value)),
        _ => value,
    }
}

fn normalize_numeric_value(value: Value, decimals: u32) -> Value {
    let Some(number) = value_as_f64(&value) else {
        return value;
    };
    let rounded = round_to(number, decimals);
    if rounded.fract().abs() < f64::EPSILON { json!(rounded as i64) } else { json!(rounded) }
}

fn normalize_int_value(value: Value) -> Value {
    if let Some(number) = value_as_f64(&value) {
        return json!(number as i64);
    }
    value
}

fn normalize_float_value(value: Value) -> Value {
    if let Some(number) = value_as_f64(&value) {
        return json!(number);
    }
    value
}

fn value_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn round_to(value: f64, decimals: u32) -> f64 {
    let factor = 10_f64.powi(decimals as i32);
    (value * factor).round() / factor
}

fn apply_transformer_fields(
    items: &mut [Value],
    transformers: Option<&[AiDataTransformerFieldDto]>,
    report_config: Option<&Value>,
    params: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let Some(transformers) = transformers else {
        return Ok(());
    };

    for transformer in transformers {
        let field_code = normalize_field_code(&transformer.code);
        if field_code.is_empty() {
            continue;
        }
        let transformer_type = normalize_field_code(&transformer.transformer_type);
        match transformer_type.as_str() {
            "percentage" => {
                apply_percentage_transformer(items, &field_code, &transformer.transformer_args)
            }
            "goods_options_name" | "goodsoptionsname" => apply_goods_options_name_transformer(
                items,
                &field_code,
                &transformer.transformer_args,
                report_config,
                params,
            ),
            _ => {
                // 未知转换器直接报错，避免悄悄返回未转换但看似成功的数据。
                return Err(format!(
                    "AI-DATA 暂不支持转换器类型: {}",
                    transformer.transformer_type
                ));
            }
        }
    }

    Ok(())
}

fn apply_percentage_transformer(
    items: &mut [Value],
    field_code: &str,
    transformer_args: &BTreeMap<String, Value>,
) {
    let decimals = transformer_args
        .get("decimals")
        .or_else(|| transformer_args.get("precision"))
        .and_then(Value::as_u64)
        .unwrap_or(2) as u32;
    for item in items {
        let Value::Object(object) = item else {
            continue;
        };
        let Some(actual_key) = find_actual_field_key(object, field_code) else {
            continue;
        };
        let Some(value) = object.get_mut(&actual_key) else {
            continue;
        };
        let Some(number) = value_as_f64(value) else {
            continue;
        };
        let rendered = round_to(number * 100.0, decimals);
        if rendered.fract().abs() < f64::EPSILON {
            *value = Value::String(format!("{}%", rendered as i64));
        } else {
            *value = Value::String(format!("{}%", rendered));
        }
    }
}

#[derive(Debug, Clone)]
struct MockGoodsOptionsRecord {
    company_id: Option<String>,
    goods_id: Option<String>,
    options_id: String,
    options_name: String,
}

fn apply_goods_options_name_transformer(
    items: &mut [Value],
    field_code: &str,
    transformer_args: &BTreeMap<String, Value>,
    report_config: Option<&Value>,
    params: &BTreeMap<String, Value>,
) {
    let mock_records = collect_mock_goods_options_records(transformer_args, report_config);
    for item in items {
        let Value::Object(object) = item else {
            continue;
        };

        let target_key = find_actual_field_key(object, field_code)
            .or_else(|| object.contains_key("options_name").then(|| "options_name".to_string()));
        let Some(target_key) = target_key else {
            continue;
        };

        let lookup_key =
            find_actual_field_key(object, "options_id").unwrap_or_else(|| target_key.clone());
        let lookup_value = object.get(&lookup_key).cloned().unwrap_or(Value::Null);
        let fallback_value =
            object.get("options_name").cloned().unwrap_or_else(|| lookup_value.clone());
        let replacement = resolve_mock_goods_options_name(
            object,
            &lookup_value,
            &mock_records,
            transformer_args,
            params,
        )
        .map(Value::String)
        .unwrap_or_else(|| format_goods_options_name_value(fallback_value));
        if let Some(value) = object.get_mut(&target_key) {
            *value = replacement;
        }
    }
}

fn collect_mock_goods_options_records(
    transformer_args: &BTreeMap<String, Value>,
    report_config: Option<&Value>,
) -> Vec<MockGoodsOptionsRecord> {
    let mut records = Vec::new();
    let mut consumed_arg_keys = BTreeSet::new();

    for key in [
        "mock_data",
        "mockData",
        "mock_options",
        "mockOptions",
        "mock_goods_options",
        "mockGoodsOptions",
    ] {
        if let Some((actual_key, value)) = btree_entry_by_normalized_key(transformer_args, key) {
            if !consumed_arg_keys.insert(actual_key.to_string()) {
                continue;
            }
            extend_mock_goods_options_records(value, &mut records);
        }
    }

    let Some(report_config) = report_config else {
        return records;
    };

    for key in ["mock_goods_options", "mockGoodsOptions", "mock_options", "mockOptions"] {
        if let Some(value) = report_config.get(key) {
            extend_mock_goods_options_records(value, &mut records);
        }
    }

    if let Some(Value::Object(mock)) = report_config.get("mock") {
        for key in ["goods_options", "goodsOptions", "options", "records", "data"] {
            if let Some(value) = mock.get(key) {
                extend_mock_goods_options_records(value, &mut records);
            }
        }
    }

    records
}

fn extend_mock_goods_options_records(value: &Value, records: &mut Vec<MockGoodsOptionsRecord>) {
    match value {
        Value::Array(items) => {
            for item in items {
                extend_mock_goods_options_records(item, records);
            }
        }
        Value::Object(object) => {
            let company_id = object
                .get("company_id")
                .or_else(|| object.get("companyId"))
                .and_then(value_lookup_string);
            let goods_id = object
                .get("goods_id")
                .or_else(|| object.get("goodsId"))
                .or_else(|| object.get("product_id"))
                .or_else(|| object.get("productId"))
                .and_then(value_lookup_string);
            let options_id = object
                .get("options_id")
                .or_else(|| object.get("optionsId"))
                .and_then(value_lookup_string)
                .map(|value| normalize_options_lookup_key(&value));
            let options_name = object
                .get("options_name")
                .or_else(|| object.get("optionsName"))
                .and_then(value_lookup_string);

            if let (Some(options_id), Some(options_name)) = (options_id, options_name) {
                records.push(MockGoodsOptionsRecord {
                    company_id: company_id.clone(),
                    goods_id: goods_id.clone(),
                    options_id,
                    options_name,
                });
            }

            for key in ["records", "items", "list", "data"] {
                if let Some(nested) = object.get(key) {
                    extend_mock_goods_options_records(nested, records);
                }
            }

            if object.get("options_id").is_none()
                && object.get("optionsId").is_none()
                && object.get("options_name").is_none()
                && object.get("optionsName").is_none()
            {
                for (key, nested) in object {
                    if matches!(
                        key.as_str(),
                        "company_id"
                            | "companyId"
                            | "goods_id"
                            | "goodsId"
                            | "product_id"
                            | "productId"
                            | "records"
                            | "items"
                            | "list"
                            | "data"
                    ) {
                        continue;
                    }
                    let Some(options_name) = value_lookup_string(nested) else {
                        continue;
                    };
                    records.push(MockGoodsOptionsRecord {
                        company_id: company_id.clone(),
                        goods_id: goods_id.clone(),
                        options_id: normalize_options_lookup_key(key),
                        options_name,
                    });
                }
            }
        }
        _ => {}
    }
}

fn resolve_mock_goods_options_name(
    object: &Map<String, Value>,
    lookup_value: &Value,
    records: &[MockGoodsOptionsRecord],
    transformer_args: &BTreeMap<String, Value>,
    params: &BTreeMap<String, Value>,
) -> Option<String> {
    if records.is_empty() {
        return None;
    }

    let options_id = value_lookup_string(lookup_value)?;
    let options_id = normalize_options_lookup_key(&options_id);
    if options_id.is_empty() || options_id == "0" {
        return None;
    }

    let company_id = resolve_goods_lookup_company_id(object, transformer_args, params);
    let goods_id = resolve_goods_lookup_goods_id(object, transformer_args, params);

    let mut best_match: Option<(&MockGoodsOptionsRecord, usize)> = None;
    for record in records {
        if record.options_id != options_id {
            continue;
        }
        if let Some(record_company_id) = record.company_id.as_deref()
            && company_id.as_deref() != Some(record_company_id)
        {
            continue;
        }
        if let Some(record_goods_id) = record.goods_id.as_deref()
            && goods_id.as_deref() != Some(record_goods_id)
        {
            continue;
        }

        // company_id/goods_id 命中的记录更具体，应优先于仅按 options_id 命中的兜底记录。
        let score =
            usize::from(record.company_id.is_some()) + usize::from(record.goods_id.is_some());
        if best_match.is_none_or(|(_, best_score)| score > best_score) {
            best_match = Some((record, score));
        }
    }

    best_match.map(|(record, _)| record.options_name.clone())
}

fn resolve_goods_lookup_company_id(
    object: &Map<String, Value>,
    transformer_args: &BTreeMap<String, Value>,
    params: &BTreeMap<String, Value>,
) -> Option<String> {
    btree_value_by_normalized_key(transformer_args, "company_id")
        .and_then(value_lookup_string)
        .or_else(|| {
            btree_value_by_normalized_key(params, "company_id").and_then(value_lookup_string)
        })
        .or_else(|| {
            json_value_by_normalized_key(object, "company_id").and_then(value_lookup_string)
        })
}

fn resolve_goods_lookup_goods_id(
    object: &Map<String, Value>,
    transformer_args: &BTreeMap<String, Value>,
    params: &BTreeMap<String, Value>,
) -> Option<String> {
    if let Some(field_name) = btree_value_by_normalized_key(transformer_args, "goods_field")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && let Some(value) = json_value_by_normalized_key(object, field_name)
    {
        return value_lookup_string(value);
    }

    for key in ["goods_id", "product_id"] {
        if let Some(value) = json_value_by_normalized_key(object, key).and_then(value_lookup_string)
        {
            return Some(value);
        }
    }
    for key in ["goods_id", "product_id"] {
        if let Some(value) =
            btree_value_by_normalized_key(params, key).and_then(value_lookup_string)
        {
            return Some(value);
        }
    }
    for key in ["goods_id", "product_id"] {
        if let Some(value) =
            btree_value_by_normalized_key(transformer_args, key).and_then(value_lookup_string)
        {
            return Some(value);
        }
    }

    None
}

fn normalize_options_lookup_key(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('[')
        && let Ok(Value::Array(items)) = serde_json::from_str::<Value>(trimmed)
    {
        // 兼容历史接口把多规格 options_id 序列化为 JSON 数组字符串的形态。
        let joined = items.iter().filter_map(value_lookup_string).collect::<Vec<_>>().join(",");
        if !joined.is_empty() {
            return joined;
        }
    }
    trimmed.split(',').map(str::trim).filter(|item| !item.is_empty()).collect::<Vec<_>>().join(",")
}

fn value_lookup_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Array(items) => {
            let joined = items.iter().filter_map(value_lookup_string).collect::<Vec<_>>().join(",");
            (!joined.is_empty()).then_some(joined)
        }
        _ => None,
    }
}

fn btree_value_by_normalized_key<'a>(
    map: &'a BTreeMap<String, Value>,
    key: &str,
) -> Option<&'a Value> {
    btree_entry_by_normalized_key(map, key).map(|(_, value)| value)
}

fn btree_entry_by_normalized_key<'a>(
    map: &'a BTreeMap<String, Value>,
    key: &str,
) -> Option<(&'a str, &'a Value)> {
    let normalized_key = normalize_field_code(key);
    map.iter()
        .find(|(candidate, _)| normalize_field_code(candidate) == normalized_key)
        .map(|(candidate, value)| (candidate.as_str(), value))
}

fn json_value_by_normalized_key<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    let normalized_key = normalize_field_code(key);
    map.iter()
        .find(|(candidate, _)| normalize_field_code(candidate) == normalized_key)
        .map(|(_, value)| value)
}

fn format_goods_options_name_value(value: Value) -> Value {
    match value {
        Value::Null => Value::String(String::new()),
        Value::String(text) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                return format_goods_options_name_value(parsed);
            }
            Value::String(text)
        }
        Value::Array(items) => Value::String(
            items
                .into_iter()
                .map(format_goods_options_name_value)
                .map(|value| string_literal(&value))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" / "),
        ),
        Value::Object(object) => Value::String(
            object
                .into_iter()
                .map(|(key, value)| {
                    let rendered = string_literal(&format_goods_options_name_value(value));
                    if rendered.is_empty() { key } else { format!("{key}:{rendered}") }
                })
                .collect::<Vec<_>>()
                .join(" / "),
        ),
        other => Value::String(string_literal(&other)),
    }
}

fn find_actual_field_key(object: &Map<String, Value>, normalized_target: &str) -> Option<String> {
    object.keys().find(|key| normalize_field_code(key) == normalized_target).cloned()
}

fn normalize_field_code(value: &str) -> String {
    let mut normalized = String::new();
    let mut prev_is_lower_or_digit = false;
    for ch in value.trim().chars() {
        if ch == '-' || ch == ' ' {
            if !normalized.ends_with('_') && !normalized.is_empty() {
                normalized.push('_');
            }
            prev_is_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() {
            if prev_is_lower_or_digit && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = false;
            continue;
        }
        normalized.push(ch.to_ascii_lowercase());
        prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }
    normalized
}

fn extract_items_and_total(value: Value) -> (Vec<Value>, Option<u64>) {
    match value {
        Value::Array(items) => {
            let total = items.len() as u64;
            (items, Some(total))
        }
        Value::Object(mut object) => {
            let total = object
                .get("total")
                .and_then(|value| value.as_u64())
                .or_else(|| object.get("count").and_then(|value| value.as_u64()))
                .or_else(|| {
                    object
                        .get("page")
                        .and_then(|value| value.get("total_record"))
                        .and_then(|value| value.as_u64())
                });
            if let Some(items) = object.remove("data").and_then(|value| value.as_array().cloned()) {
                return (items, total);
            }
            if let Some(items) = object.remove("items").and_then(|value| value.as_array().cloned())
            {
                return (items, total);
            }
            if let Some(items) = object.remove("list").and_then(|value| value.as_array().cloned()) {
                return (items, total);
            }
            (vec![Value::Object(object)], total.or(Some(1)))
        }
        other => (vec![other], Some(1)),
    }
}

fn parse_http_method(raw: &str) -> Result<reqwest::Method, String> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(reqwest::Method::GET),
        "POST" => Ok(reqwest::Method::POST),
        "HEAD" => Ok(reqwest::Method::HEAD),
        other => Err(format!("AI-DATA 仅支持 GET/POST/HEAD，收到 {other}")),
    }
}

fn http_endpoint(connection: &AiDataConnectionDto, path: Option<&str>) -> Result<String, String> {
    let base_url = connection
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "HTTP 连接缺少 base_url".to_string())?;
    let suffix = path.unwrap_or("");
    if suffix.is_empty() {
        return Ok(base_url.to_string());
    }
    Ok(format!("{}/{}", base_url.trim_end_matches('/'), suffix.trim_start_matches('/')))
}

fn cube_endpoint(connection: &AiDataConnectionDto, tail: &str) -> Result<String, String> {
    let prefix = connection.default_path.as_deref().unwrap_or("/cubejs-api/v1");
    http_endpoint(
        connection,
        Some(&format!("{}/{}", prefix.trim_end_matches('/'), tail.trim_start_matches('/'))),
    )
}

fn build_http_client(timeout_secs: u32) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(u64::from(timeout_secs.max(1))))
        .build()
        .map_err(|e| e.to_string())
}

fn apply_request_headers(
    mut builder: reqwest::RequestBuilder,
    connection: &AiDataConnectionDto,
) -> reqwest::RequestBuilder {
    if let Some(token) = connection.auth_token.as_deref().filter(|value| !value.trim().is_empty()) {
        builder = builder.header(reqwest::header::AUTHORIZATION, token);
    }
    for (key, value) in &connection.headers {
        builder = builder.header(key, value);
    }
    builder
}

async fn send_query_request(
    connection: &AiDataConnectionDto,
    url: &str,
    params: &BTreeMap<String, Value>,
    timeout_secs: u32,
) -> Result<(u16, Value), String> {
    let client = build_http_client(timeout_secs)?;
    let query =
        params.iter().map(|(key, value)| (key.clone(), string_literal(value))).collect::<Vec<_>>();
    let request = apply_request_headers(client.get(url).query(&query), connection);
    let response = request.send().await.map_err(|e| e.to_string())?;
    parse_http_response(response).await
}

async fn send_json_request(
    connection: &AiDataConnectionDto,
    method: reqwest::Method,
    url: Option<&str>,
    body: Option<&Value>,
    timeout_secs: u32,
) -> Result<(u16, Value), String> {
    let client = build_http_client(timeout_secs)?;
    let target = url.ok_or_else(|| "AI-DATA 缺少请求 URL".to_string())?;
    let mut request = client.request(method, target);
    request = apply_request_headers(request, connection);
    if let Some(value) = body {
        request = request.json(value);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    parse_http_response(response).await
}

async fn parse_http_response(response: reqwest::Response) -> Result<(u16, Value), String> {
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let excerpt = body.chars().take(300).collect::<String>();
        // 错误响应只截取短片段，避免把大响应体或潜在敏感载荷完整写入错误链路。
        return Err(format!("HTTP {}: {}", status.as_u16(), excerpt));
    }
    let value = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({ "raw": body }));
    Ok((status.as_u16(), value))
}

fn merge_json_object(body: Option<Value>, params: &BTreeMap<String, Value>) -> Option<Value> {
    match body {
        Some(Value::Object(mut object)) => {
            for (key, value) in params {
                object.insert(key.clone(), value.clone());
            }
            Some(Value::Object(object))
        }
        Some(other) => Some(other),
        None if !params.is_empty() => Some(Value::Object(
            params.iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        )),
        None => None,
    }
}

/// 截断文本到指定字符数。
///
/// # 参数
///
/// - `value`: 原始文本。
/// - `max_chars`: 最大字符数，而不是字节数。
///
/// # 返回值
///
/// 返回不超过 `max_chars` 个 Unicode scalar 的字符串。
pub(super) fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>()
}

/// 根据报表和数据源 key 解析 AI 查询上下文。
///
/// # 参数
///
/// - `reports`: 可用报表集合。
/// - `report_id`: 报表 id 或 slug。
/// - `source_key`: 可选数据源 key；未提供时使用报表默认数据源。
///
/// # 返回值
///
/// 成功时返回准备后的报表和选中的数据源；无法解析时返回 `None`。
pub(super) fn report_source_context(
    reports: &[AiDataReportDto],
    report_id: Option<&str>,
    source_key: Option<&str>,
) -> Option<(AiDataReportDto, AiDataReportSourceDto)> {
    let report = reports
        .iter()
        .find(|item| Some(item.id.as_str()) == report_id || Some(item.slug.as_str()) == report_id)?
        .clone();
    let effective_source_key = source_key
        .map(ToOwned::to_owned)
        .or_else(|| report.default_source_key.clone())
        .or_else(|| report.sources.first().map(|item| item.source_key.clone()))?;
    let source =
        report.sources.iter().find(|item| item.source_key == effective_source_key)?.clone();
    Some((prepared_report(&report), source))
}

#[cfg(test)]
#[path = "data_runtime_tests.rs"]
mod data_runtime_tests;
