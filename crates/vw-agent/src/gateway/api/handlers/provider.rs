//! Provider 路由模块

use std::collections::HashMap;

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::{InstanceQuery, resolve_directory, with_instance};
use crate::app::agent::provider;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/provider", get(provider_list))
        .route("/provider/auth", get(provider_auth_methods))
        .route("/provider/{provider_id}/oauth/authorize", post(provider_oauth_authorize))
        .route("/provider/{provider_id}/oauth/callback", post(provider_oauth_callback))
}

#[derive(Debug, Serialize)]
struct ProviderListResponse {
    all: Vec<provider::provider::Info>,
    default: HashMap<String, String>,
    connected: Vec<String>,
}

async fn provider_list(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ProviderListResponse>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let providers = provider::provider::list_for_settings()
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            let mut out = Vec::new();
            let mut defaults = HashMap::new();
            let mut connected = Vec::new();
            for p in providers.values() {
                out.push(p.clone());
                if p.key.as_deref().is_some_and(|k| !k.trim().is_empty()) {
                    connected.push(p.id.clone());
                }
                let models =
                    provider::provider::sort(p.models.values().cloned().collect::<Vec<_>>());
                if let Some(m) = models.first() {
                    defaults.insert(p.id.clone(), m.id.clone());
                }
            }
            connected.sort();
            Ok(ProviderListResponse { all: out, default: defaults, connected })
        })
    })
    .await?;
    Ok(Json(result))
}

async fn provider_auth_methods() -> Json<HashMap<String, Vec<provider::auth::Method>>> {
    Json(provider::auth::methods().await)
}

#[derive(Debug, Deserialize)]
struct OAuthAuthorizeRequest {
    method: usize,
}

async fn provider_oauth_authorize(
    Path(provider_id): Path<String>,
    Json(body): Json<OAuthAuthorizeRequest>,
) -> Result<Json<Option<provider::auth::Authorization>>, ApiError> {
    let result = provider::auth::authorize(&provider_id, body.method)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct OAuthCallbackRequest {
    method: usize,
    code: Option<String>,
}

async fn provider_oauth_callback(
    Path(provider_id): Path<String>,
    Json(body): Json<OAuthCallbackRequest>,
) -> Result<Json<bool>, ApiError> {
    provider::auth::callback(&provider_id, body.method, body.code.as_deref())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(true))
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
