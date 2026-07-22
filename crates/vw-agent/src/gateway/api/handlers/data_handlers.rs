//! AI-DATA HTTP API 处理函数。
//!
//! 该模块负责连接、报表、设置、查询和自然语言查询的网关入口。处理函数只做
//! 参数校验、持久化编排和错误映射，实际执行与存储细节分别委托给 runtime
//! 和 storage_support。

use axum::{Json, extract::Extension, extract::Path, extract::State};
use uuid::Uuid;
use vw_api_types::data::{
    AiDataAiQueryRequest, AiDataAiQueryResponse, AiDataConnectionCatalogResponse,
    AiDataConnectionDto, AiDataConnectionTestResponse, AiDataConnectionUpsertBody,
    AiDataQueryRequest, AiDataQueryResponse, AiDataReportDto, AiDataReportUpsertBody,
    AiDataSettings, AiDataSettingsUpdateBody,
};
use vw_api_types::id::TenantScope;

use super::{ai_support, runtime, storage_support};
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::state::AppState;

/// 读取 AI-DATA 设置。
pub(super) async fn data_settings_get(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<AiDataSettings>, ApiError> {
    Ok(Json(storage_support::load_settings(tenant).await))
}

/// 更新 AI-DATA 设置。
///
/// # 错误
///
/// 当设置持久化失败时返回 `ApiError`。
pub(super) async fn data_settings_put(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<AiDataSettingsUpdateBody>,
) -> Result<Json<AiDataSettings>, ApiError> {
    let mut settings = storage_support::load_settings(tenant.clone()).await;
    settings.default_limit = body.default_limit.clamp(1, 10_000);
    settings.default_timeout_secs = body.default_timeout_secs.clamp(1, 600);
    storage_support::save_settings(tenant, &settings).await?;
    Ok(Json(settings))
}

/// 列出全部 AI-DATA 连接。
pub(super) async fn data_connections_list(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Vec<AiDataConnectionDto>>, ApiError> {
    let mut connections = storage_support::load_connections(tenant).await;
    storage_support::sort_connections(&mut connections);
    Ok(Json(connections))
}

/// 按连接 id 读取单个 AI-DATA 连接。
///
/// # 错误
///
/// 当连接不存在时返回 404 风格的 `ApiError`。
pub(super) async fn data_connection_get(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
) -> Result<Json<AiDataConnectionDto>, ApiError> {
    let connection = storage_support::load_connections(tenant)
        .await
        .into_iter()
        .find(|item| item.id == connection_id)
        .ok_or_else(|| ApiError::not_found("AI-DATA 连接不存在"))?;
    Ok(Json(connection))
}

/// 创建 AI-DATA 连接并设为当前选中连接。
///
/// # 错误
///
/// 当请求体校验失败或持久化失败时返回 `ApiError`。
pub(super) async fn data_connection_create(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<AiDataConnectionUpsertBody>,
) -> Result<Json<AiDataConnectionDto>, ApiError> {
    validate_connection_upsert(&body)?;

    let now_ms = storage_support::now_ms();
    let mut connections = storage_support::load_connections(tenant.clone()).await;
    let connection = AiDataConnectionDto {
        id: Uuid::new_v4().to_string(),
        name: body.name.trim().to_string(),
        kind: body.kind,
        description: body.description.filter(|value| !value.trim().is_empty()),
        enabled: body.enabled,
        read_only: body.read_only,
        base_url: body.base_url.filter(|value| !value.trim().is_empty()),
        connection_url: body.connection_url.filter(|value| !value.trim().is_empty()),
        sqlite_path: body.sqlite_path.filter(|value| !value.trim().is_empty()),
        default_path: body.default_path.filter(|value| !value.trim().is_empty()),
        auth_token: body.auth_token.filter(|value| !value.trim().is_empty()),
        headers: body.headers,
        schema_hint: body.schema_hint.filter(|value| !value.trim().is_empty()),
        updated_at_ms: now_ms,
        last_used_ms: None,
    };
    connections.push(connection.clone());
    storage_support::sort_connections(&mut connections);
    storage_support::save_connections(tenant.clone(), &connections).await?;

    let mut settings = storage_support::load_settings(tenant.clone()).await;
    settings.selected_connection_id = Some(connection.id.clone());
    storage_support::save_settings(tenant, &settings).await?;

    Ok(Json(connection))
}

/// 更新指定 AI-DATA 连接。
///
/// # 错误
///
/// 当连接不存在、请求体校验失败或持久化失败时返回 `ApiError`。
pub(super) async fn data_connection_update(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
    Json(body): Json<AiDataConnectionUpsertBody>,
) -> Result<Json<AiDataConnectionDto>, ApiError> {
    validate_connection_upsert(&body)?;

    let now_ms = storage_support::now_ms();
    let mut connections = storage_support::load_connections(tenant.clone()).await;
    let Some(index) = connections.iter().position(|item| item.id == connection_id) else {
        return Err(ApiError::not_found("AI-DATA 连接不存在"));
    };

    let last_used_ms = connections[index].last_used_ms;
    let updated = AiDataConnectionDto {
        id: connection_id,
        name: body.name.trim().to_string(),
        kind: body.kind,
        description: body.description.filter(|value| !value.trim().is_empty()),
        enabled: body.enabled,
        read_only: body.read_only,
        base_url: body.base_url.filter(|value| !value.trim().is_empty()),
        connection_url: body.connection_url.filter(|value| !value.trim().is_empty()),
        sqlite_path: body.sqlite_path.filter(|value| !value.trim().is_empty()),
        default_path: body.default_path.filter(|value| !value.trim().is_empty()),
        auth_token: body.auth_token.filter(|value| !value.trim().is_empty()),
        headers: body.headers,
        schema_hint: body.schema_hint.filter(|value| !value.trim().is_empty()),
        updated_at_ms: now_ms,
        last_used_ms,
    };
    connections[index] = updated.clone();
    storage_support::sort_connections(&mut connections);
    storage_support::save_connections(tenant.clone(), &connections).await?;

    let mut settings = storage_support::load_settings(tenant.clone()).await;
    settings.selected_connection_id = Some(updated.id.clone());
    storage_support::save_settings(tenant, &settings).await?;

    Ok(Json(updated))
}

/// 删除指定 AI-DATA 连接。
///
/// # 错误
///
/// 当连接不存在或持久化失败时返回 `ApiError`。
pub(super) async fn data_connection_delete(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut connections = storage_support::load_connections(tenant.clone()).await;
    let Some(index) = connections.iter().position(|item| item.id == connection_id) else {
        return Err(ApiError::not_found("AI-DATA 连接不存在"));
    };

    let removed = connections.remove(index);
    storage_support::sort_connections(&mut connections);
    storage_support::save_connections(tenant.clone(), &connections).await?;

    let mut settings = storage_support::load_settings(tenant.clone()).await;
    if settings.selected_connection_id.as_deref() == Some(removed.id.as_str()) {
        settings.selected_connection_id = connections.first().map(|item| item.id.clone());
        storage_support::save_settings(tenant, &settings).await?;
    }

    Ok(Json(serde_json::json!({ "deleted_id": removed.id })))
}

/// 激活指定连接并记录最近使用时间。
///
/// # 错误
///
/// 当连接不存在或持久化失败时返回 `ApiError`。
pub(super) async fn data_connection_activate(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
) -> Result<Json<AiDataSettings>, ApiError> {
    let now_ms = storage_support::now_ms();
    let mut connections = storage_support::load_connections(tenant.clone()).await;
    let Some(index) = connections.iter().position(|item| item.id == connection_id) else {
        return Err(ApiError::not_found("AI-DATA 连接不存在"));
    };

    connections[index].last_used_ms = Some(now_ms);
    let selected_id = connections[index].id.clone();
    storage_support::sort_connections(&mut connections);
    storage_support::save_connections(tenant.clone(), &connections).await?;

    let mut settings = storage_support::load_settings(tenant.clone()).await;
    settings.selected_connection_id = Some(selected_id);
    storage_support::save_settings(tenant, &settings).await?;
    Ok(Json(settings))
}

/// 测试指定连接是否可用。
///
/// # 返回值
///
/// 连接层失败会被包装为 `ok: false` 的响应，连接记录不存在才返回 `ApiError`。
pub(super) async fn data_connection_test(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
) -> Result<Json<AiDataConnectionTestResponse>, ApiError> {
    let connection = storage_support::load_connections(tenant.clone())
        .await
        .into_iter()
        .find(|item| item.id == connection_id)
        .ok_or_else(|| ApiError::not_found("AI-DATA 连接不存在"))?;
    let timeout_secs = storage_support::load_settings(tenant).await.default_timeout_secs;
    let started = std::time::Instant::now();
    let result = runtime::test_connection(&connection, timeout_secs).await;
    let latency_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    match result {
        Ok(message) => Ok(Json(AiDataConnectionTestResponse { ok: true, message, latency_ms })),
        Err(message) => Ok(Json(AiDataConnectionTestResponse { ok: false, message, latency_ms })),
    }
}

/// 读取指定连接的目录信息。
///
/// # 错误
///
/// 当连接不存在或目录读取失败时返回 `ApiError`。
pub(super) async fn data_connection_catalog(
    Extension(tenant): Extension<TenantScope>,
    Path(connection_id): Path<String>,
) -> Result<Json<AiDataConnectionCatalogResponse>, ApiError> {
    let connection = storage_support::load_connections(tenant.clone())
        .await
        .into_iter()
        .find(|item| item.id == connection_id)
        .ok_or_else(|| ApiError::not_found("AI-DATA 连接不存在"))?;

    let timeout_secs = storage_support::load_settings(tenant).await.default_timeout_secs;
    let catalog = runtime::connection_catalog(&connection, timeout_secs)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(AiDataConnectionCatalogResponse {
        connection_id: connection.id,
        kind: connection.kind,
        catalog,
    }))
}

/// 列出全部 AI-DATA 报表。
pub(super) async fn data_reports_list() -> Result<Json<Vec<AiDataReportDto>>, ApiError> {
    let mut reports = storage_support::load_reports().await;
    storage_support::sort_reports(&mut reports);
    Ok(Json(reports.iter().map(runtime::prepared_report).collect()))
}

/// 按报表 id 读取单个报表。
///
/// # 错误
///
/// 当报表不存在时返回 `ApiError`。
pub(super) async fn data_report_get(
    Path(report_id): Path<String>,
) -> Result<Json<AiDataReportDto>, ApiError> {
    let report = storage_support::load_reports()
        .await
        .into_iter()
        .find(|item| item.id == report_id)
        .ok_or_else(|| ApiError::not_found("AI-DATA 报表不存在"))?;
    Ok(Json(runtime::prepared_report(&report)))
}

/// 创建 AI-DATA 报表。
///
/// # 错误
///
/// 当请求体校验失败、slug 冲突或持久化失败时返回 `ApiError`。
pub(super) async fn data_report_create(
    Json(body): Json<AiDataReportUpsertBody>,
) -> Result<Json<AiDataReportDto>, ApiError> {
    validate_report_upsert(&body)?;

    let mut reports = storage_support::load_reports().await;
    if reports.iter().any(|item| item.slug == body.slug.trim()) {
        return Err(ApiError::bad_request("AI-DATA 报表 slug 已存在"));
    }

    let report = AiDataReportDto {
        id: Uuid::new_v4().to_string(),
        name: body.name.trim().to_string(),
        slug: body.slug.trim().to_string(),
        data_source: body.data_source,
        default_source_key: body.default_source_key.filter(|value| !value.trim().is_empty()),
        report_config: body.report_config,
        sources: body.sources,
        updated_at_ms: storage_support::now_ms(),
    };
    reports.push(report.clone());
    storage_support::sort_reports(&mut reports);
    storage_support::save_reports(&reports).await?;
    Ok(Json(runtime::prepared_report(&report)))
}

/// 更新指定 AI-DATA 报表。
///
/// # 错误
///
/// 当报表不存在、slug 冲突、请求体校验失败或持久化失败时返回 `ApiError`。
pub(super) async fn data_report_update(
    Path(report_id): Path<String>,
    Json(body): Json<AiDataReportUpsertBody>,
) -> Result<Json<AiDataReportDto>, ApiError> {
    validate_report_upsert(&body)?;

    let mut reports = storage_support::load_reports().await;
    if reports.iter().any(|item| item.id != report_id && item.slug == body.slug.trim()) {
        return Err(ApiError::bad_request("AI-DATA 报表 slug 已存在"));
    }
    let Some(index) = reports.iter().position(|item| item.id == report_id) else {
        return Err(ApiError::not_found("AI-DATA 报表不存在"));
    };

    let updated = AiDataReportDto {
        id: report_id,
        name: body.name.trim().to_string(),
        slug: body.slug.trim().to_string(),
        data_source: body.data_source,
        default_source_key: body.default_source_key.filter(|value| !value.trim().is_empty()),
        report_config: body.report_config,
        sources: body.sources,
        updated_at_ms: storage_support::now_ms(),
    };
    reports[index] = updated.clone();
    storage_support::sort_reports(&mut reports);
    storage_support::save_reports(&reports).await?;
    Ok(Json(runtime::prepared_report(&updated)))
}

/// 删除指定 AI-DATA 报表。
///
/// # 错误
///
/// 当报表不存在或持久化失败时返回 `ApiError`。
pub(super) async fn data_report_delete(
    Path(report_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut reports = storage_support::load_reports().await;
    let Some(index) = reports.iter().position(|item| item.id == report_id) else {
        return Err(ApiError::not_found("AI-DATA 报表不存在"));
    };
    let removed = reports.remove(index);
    storage_support::sort_reports(&mut reports);
    storage_support::save_reports(&reports).await?;
    Ok(Json(serde_json::json!({ "deleted_id": removed.id })))
}

/// 执行结构化 AI-DATA 查询。
///
/// # 错误
///
/// 当连接、报表、模板或底层数据源执行失败时返回 `ApiError`。
pub(super) async fn data_query(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<AiDataQueryRequest>,
) -> Result<Json<AiDataQueryResponse>, ApiError> {
    let settings = storage_support::load_settings(tenant.clone()).await;
    let connections = storage_support::load_connections(tenant).await;
    let reports = storage_support::load_reports().await;
    let response = runtime::execute_query(&settings, &connections, &reports, body)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(response))
}

/// 执行 AI-DATA 自然语言查询。
///
/// # 错误
///
/// 当模型规划或底层查询失败时返回 `ApiError`。
pub(super) async fn data_ai_query(
    Extension(tenant): Extension<TenantScope>,
    State(state): State<AppState>,
    Json(body): Json<AiDataAiQueryRequest>,
) -> Result<Json<AiDataAiQueryResponse>, ApiError> {
    let settings = storage_support::load_settings(tenant.clone()).await;
    let connections = storage_support::load_connections(tenant).await;
    let reports = storage_support::load_reports().await;
    ai_support::handle_ai_query(State(state), settings, connections, reports, body).await
}

fn validate_connection_upsert(body: &AiDataConnectionUpsertBody) -> Result<(), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("请输入 AI-DATA 连接名称"));
    }

    match body.kind {
        vw_api_types::data::AiDataConnectionKind::Sqlite => {
            if body.sqlite_path.as_deref().is_none_or(|value| value.trim().is_empty()) {
                return Err(ApiError::bad_request("SQLite 连接必须提供 sqlite_path"));
            }
        }
        vw_api_types::data::AiDataConnectionKind::Mysql
        | vw_api_types::data::AiDataConnectionKind::Postgres => {
            if body.connection_url.as_deref().is_none_or(|value| value.trim().is_empty()) {
                return Err(ApiError::bad_request("数据库连接必须提供 connection_url"));
            }
        }
        vw_api_types::data::AiDataConnectionKind::Cube
        | vw_api_types::data::AiDataConnectionKind::Http => {
            if body.base_url.as_deref().is_none_or(|value| value.trim().is_empty()) {
                return Err(ApiError::bad_request("HTTP/Cube 连接必须提供 base_url"));
            }
        }
    }

    Ok(())
}

fn validate_report_upsert(body: &AiDataReportUpsertBody) -> Result<(), ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("请输入 AI-DATA 报表名称"));
    }
    if body.slug.trim().is_empty() {
        return Err(ApiError::bad_request("请输入 AI-DATA 报表 slug"));
    }
    if body.sources.is_empty() {
        return Err(ApiError::bad_request("AI-DATA 报表至少需要一个数据源"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "data_handlers_tests.rs"]
mod data_handlers_tests;
