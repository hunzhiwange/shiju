//! ApiQL PHP 兼容 HTTP handler。

#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use std::time::{Duration, Instant};

use axum::{
    Json,
    extract::{Extension, Path, RawQuery},
};
#[cfg(not(test))]
use once_cell::sync::Lazy;
#[cfg(not(test))]
use parking_lot::Mutex;
use serde_json::Value;
use vw_api_types::apiql::{ApiQlListMode, ApiQlResponse};
use vw_api_types::id::TenantScope;

use super::connections::{ApiQlConnection, ApiQlConnectionStore, local_connection_store};
use super::functions::api_ql_with_mode;
use crate::app::agent::gateway::ApiError;

#[cfg(not(test))]
const APIQL_CONNECTION_CACHE_TTL: Duration = Duration::from_secs(2);

#[cfg(not(test))]
static APIQL_CONNECTION_CACHE: Lazy<Mutex<Option<ApiQlConnectionCache>>> =
    Lazy::new(|| Mutex::new(None));

#[cfg(not(test))]
#[derive(Debug, Clone)]
struct ApiQlConnectionCache {
    tenant: TenantScope,
    loaded_at: Instant,
    connections: Arc<Vec<ApiQlConnection>>,
}

pub(super) async fn apiql_index(
    Extension(tenant): Extension<TenantScope>,
    Path(entity_class): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<ApiQlResponse<Value>>, ApiError> {
    dispatch_apiql_query(tenant, entity_class, raw_query, ApiQlListMode::Index).await
}

pub(super) async fn apiql_index_only(
    Extension(tenant): Extension<TenantScope>,
    Path(entity_class): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<ApiQlResponse<Value>>, ApiError> {
    dispatch_apiql_query(tenant, entity_class, raw_query, ApiQlListMode::IndexOnly).await
}

pub(super) async fn apiql_index_count(
    Extension(tenant): Extension<TenantScope>,
    Path(entity_class): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<ApiQlResponse<Value>>, ApiError> {
    dispatch_apiql_query(tenant, entity_class, raw_query, ApiQlListMode::IndexCount).await
}

pub(super) async fn apiql_index_page(
    Extension(tenant): Extension<TenantScope>,
    Path(entity_class): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<ApiQlResponse<Value>>, ApiError> {
    dispatch_apiql_query(tenant, entity_class, raw_query, ApiQlListMode::IndexPage).await
}

async fn dispatch_apiql_query(
    tenant: TenantScope,
    entity_class: String,
    raw_query: Option<String>,
    mode: ApiQlListMode,
) -> Result<Json<ApiQlResponse<Value>>, ApiError> {
    let result =
        api_ql_with_mode(tenant, &entity_class, raw_query.as_deref().unwrap_or_default(), mode)
            .await?;
    Ok(Json(result.response))
}

pub(super) async fn load_connection(
    tenant: TenantScope,
    connection_id: &str,
) -> Result<ApiQlConnection, ApiError> {
    let connections = load_connections(tenant).await;
    connections
        .iter()
        .find(|connection| connection.id == connection_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found(format!("ApiQL connection not found: {connection_id}")))
}

pub(super) fn load_connections_from_store(
    store: &ApiQlConnectionStore,
) -> anyhow::Result<Vec<ApiQlConnection>> {
    store.load_connections()
}

#[cfg(not(test))]
async fn load_connections(tenant: TenantScope) -> Arc<Vec<ApiQlConnection>> {
    if let Some(cached) = APIQL_CONNECTION_CACHE.lock().clone()
        && cached.tenant == tenant
        && cached.loaded_at.elapsed() <= APIQL_CONNECTION_CACHE_TTL
    {
        return cached.connections;
    }

    let connections: Arc<Vec<ApiQlConnection>> =
        Arc::new(load_connections_uncached(tenant.clone()));
    *APIQL_CONNECTION_CACHE.lock() = Some(ApiQlConnectionCache {
        tenant,
        loaded_at: Instant::now(),
        connections: connections.clone(),
    });
    connections
}

#[cfg(test)]
async fn load_connections(tenant: TenantScope) -> Vec<ApiQlConnection> {
    load_connections_uncached(tenant)
}

fn load_connections_uncached(tenant: TenantScope) -> Vec<ApiQlConnection> {
    match local_connection_store(tenant)
        .map_err(|error| error.to_string())
        .and_then(|store| load_connections_from_store(&store).map_err(|error| error.to_string()))
    {
        Ok(connections) => connections,
        Err(error) => {
            tracing::warn!(target: "vw_gateway", error = %error, "failed to load ApiQL connections");
            Vec::new()
        }
    }
}
