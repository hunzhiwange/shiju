//! GET query: project_id, target_host, time_zone (default UTC), and `query`,
//! a URL-encoded JSON UsageQuery. Identity selectors are authorized, never trusted.
use std::path::PathBuf;

use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::get};
use serde::Deserialize;
use serde_json::Value;
use vw_agent_apps::usage::{groups, logs, ranges, series, summary};
use vw_api_types::{ProjectScope, TenantScope, id::ProjectId, usage::*};

#[derive(Clone)]
struct Policy(Option<PathBuf>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadQuery {
    project_id: ProjectId,
    target_host: String,
    /// JSON keeps the shared tagged range and provider DTO intact in GET URLs.
    query: Option<String>,
    time_zone: Option<String>,
    interval: Option<series::UsageSeriesInterval>,
}

pub(crate) fn router() -> Router {
    Router::new()
        .route("/usage/logs", get(logs))
        .route("/usage/logs/{id}", get(detail))
        .route("/usage/sources", get(sources))
        .route("/usage/summary", get(summary))
        .route("/usage/summary/apps", get(apps))
        .route("/usage/summary/sessions", get(sessions))
        .route("/usage/first-date", get(first_date))
        .route("/usage/stats/providers", get(providers))
        .route("/usage/stats/models", get(models))
        .route("/usage/trends", get(trends))
        .route("/usage/heatmap", get(heatmap))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}

#[derive(Clone, Copy)]
enum Read { Logs, Sources, Summary, Apps, Sessions, FirstDate, Providers, Models, Trends, Heatmap }

type Response = Result<Json<Value>, StatusCode>;

macro_rules! handler {
    ($name:ident, $kind:ident) => {
        async fn $name(
            Extension(policy): Extension<Policy>,
            Extension(tenant): Extension<TenantScope>,
            headers: HeaderMap,
            request: Result<Query<ReadQuery>, axum::extract::rejection::QueryRejection>,
        ) -> Response {
            let Query(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
            tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request, Read::$kind, None))
                .await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        }
    };
}
handler!(logs, Logs);
handler!(sources, Sources);
handler!(summary, Summary);
handler!(apps, Apps);
handler!(sessions, Sessions);
handler!(first_date, FirstDate);
handler!(providers, Providers);
handler!(models, Models);
handler!(trends, Trends);
handler!(heatmap, Heatmap);

async fn detail(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(id): Path<String>,
    request: Result<Query<ReadQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let Query(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || execute(policy, tenant, headers, request, Read::Logs, Some(id)))
        .await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

fn json(value: impl serde::Serialize) -> Response {
    serde_json::to_value(value).map(Json).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
fn log_error(error: logs::UsageLogsError) -> StatusCode {
    match error {
        logs::UsageLogsError::InvalidInput => StatusCode::BAD_REQUEST,
        logs::UsageLogsError::RevisionConflict => StatusCode::CONFLICT,
        logs::UsageLogsError::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
fn summary_error(error: summary::UsageSummaryError) -> StatusCode {
    match error {
        summary::UsageSummaryError::InvalidInput => StatusCode::BAD_REQUEST,
        summary::UsageSummaryError::StorageUnavailable
        | summary::UsageSummaryError::ArchivePrecisionUnavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn execute(policy: Policy, tenant: TenantScope, headers: HeaderMap, request: ReadQuery,
    kind: Read, id: Option<String>) -> Response {
    let scope = ProjectScope::new(tenant, request.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    // Authorization precedes query decoding, resource lookup and database opening.
    super::super::agent_apps::authorize_usage_ledger_read(
        policy.0.as_deref(), &headers, &scope, &request.target_host)?;
    let local_host = crate::usage_bridge::context::local_host().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if request.target_host != local_host { return Err(StatusCode::FORBIDDEN); }
    let query: UsageQuery = match request.query {
        Some(value) => serde_json::from_str(&value).map_err(|_| StatusCode::BAD_REQUEST)?,
        None => UsageQuery { range: UsageRange::All, filter: UsageFilter::default(), cursor: None, limit: None },
    };
    let zone = request.time_zone.as_deref().unwrap_or("UTC");
    let now = UsageUnixSeconds(chrono::Utc::now().timestamp());
    let range = ranges::resolve_usage_range(&query.range, zone, now).map_err(|_| StatusCode::BAD_REQUEST)?;
    if query.limit.is_some_and(|v| v == 0 || v > logs::MAX_USAGE_LOG_PAGE_SIZE)
        || (!matches!(kind, Read::Logs) && query.cursor.is_some())
        || (request.interval.is_some() && !matches!(kind, Read::Trends)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let database = crate::usage_bridge::context::database().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // No migrations, scanning or backfill in a GET handler.
    let connection = rusqlite::Connection::open_with_flags(
        database.sqlite_path().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let host = request.target_host.as_str();
    let filter = &query.filter;
    if let Some(id) = id {
        if query.cursor.is_some() { return Err(StatusCode::BAD_REQUEST); }
        let detail = logs::get_usage_log_detail(&connection, &scope, host, &id).map_err(log_error)?
            .filter(|v| logs::effective_usage_log_filter(&v.record, filter, &range))
            .ok_or(StatusCode::NOT_FOUND)?;
        return json(detail);
    }
    match kind {
        Read::Logs => json(logs::query_usage_logs(&connection, &scope, host, &query, zone, now).map_err(log_error)?),
        Read::Sources => json(logs::query_usage_logs(&connection, &scope, host, &query, zone, now).map_err(log_error)?.sources),
        Read::Summary => json(summary::get_usage_summary(&connection, &scope, host, &range, filter).map_err(summary_error)?),
        Read::Apps => json(summary::get_usage_summary_by_app(&connection, &scope, host, &range, filter).map_err(summary_error)?),
        Read::Sessions => {
            // Tuple keys cannot be JSON object keys; preserve both identity dimensions in rows.
            let rows = summary::get_session_usage_summary(&connection, &scope, host, &range, filter).map_err(summary_error)?;
            json(rows.into_iter().map(|((app, session), summary)| serde_json::json!({
                "app": app, "native_session_id": session, "summary": summary,
            })).collect::<Vec<_>>())
        }
        Read::FirstDate => json(summary::get_first_usage_date(&connection, &scope, host, &range, filter).map_err(summary_error)?),
        Read::Providers => json(groups::get_provider_stats(&connection, &scope, host, &range, filter).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?),
        Read::Models => json(groups::get_model_stats(&connection, &scope, host, &range, filter).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?),
        Read::Trends => json(series::get_daily_trends(&connection, &scope, host, &range, filter, request.interval).map_err(summary_error)?),
        Read::Heatmap => json(series::get_usage_heatmap(&connection, &scope, host, &range, filter).map_err(summary_error)?),
    }
}
