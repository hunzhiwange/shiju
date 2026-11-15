//! Provider 路由模块
#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use vw_api_transport::id::TenantScope;

use gateway_types::ApiError;
use agent_provider as provider;
use std::{future::Future, pin::Pin};

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Debug, Deserialize)]
pub struct InstanceQuery {
    pub directory: Option<String>,
}

/// Binds the existing tenant and project instance contexts without spawning work.
pub trait ProviderHost: Send + Sync + 'static {
    fn settings_provider_list() -> HostFuture<Result<
        (HashMap<String, provider::provider::Info>, HashMap<String, String>),
        provider::provider::CatalogError,
    >>;
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_context<T: Send + 'static>(
        tenant: TenantScope,
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
}

pub fn router<S, H: ProviderHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/provider", get(provider_list::<H>))
        .route("/provider/auth", get(provider_auth_methods))
        .route("/provider/{provider_id}/oauth/authorize", post(provider_oauth_authorize))
        .route("/provider/{provider_id}/oauth/callback", post(provider_oauth_callback))
}

#[derive(Debug, Serialize)]
pub struct ProviderListResponse {
    pub all: Vec<provider::provider::Info>,
    pub default: HashMap<String, String>,
    pub connected: Vec<String>,
}

pub async fn provider_list<H: ProviderHost>(
    Extension(tenant): Extension<TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ProviderListResponse>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_context(
        tenant,
        dir, move || {
            Box::pin(async move {
                let (providers, mut defaults) = H::settings_provider_list()
                    .await
                    .map_err(|error| ApiError::internal(error.to_string()))?;
                let mut out = Vec::new();
                let mut connected = Vec::new();
                for p in providers.into_values() {
                    if p.key.as_deref().is_some_and(|k| !k.trim().is_empty()) {
                        connected.push(p.id.clone());
                    }
                    if !defaults.contains_key(&p.id) {
                        let models = provider::provider::sort(
                            p.models.values().cloned().collect::<Vec<_>>(),
                        );
                        if let Some(m) = models.first() {
                            defaults.insert(p.id.clone(), m.id.clone());
                        }
                    }
                    out.push(p);
                }
                connected.sort();
                Ok(ProviderListResponse { all: out, default: defaults, connected })
            })
        },
    )
    .await?;
    Ok(Json(result))
}

pub async fn provider_auth_methods() -> Json<HashMap<String, Vec<provider::auth::Method>>> {
    Json(provider::auth::methods().await)
}

#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub method: usize,
}

pub async fn provider_oauth_authorize(
    Path(provider_id): Path<String>,
    Json(body): Json<OAuthAuthorizeRequest>,
) -> Result<Json<Option<provider::auth::Authorization>>, ApiError> {
    let result = provider::auth::authorize(&provider_id, body.method)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackRequest {
    pub method: usize,
    pub code: Option<String>,
}

pub async fn provider_oauth_callback(
    Path(provider_id): Path<String>,
    Json(body): Json<OAuthCallbackRequest>,
) -> Result<Json<bool>, ApiError> {
    provider::auth::callback(&provider_id, body.method, body.code.as_deref())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(true))
}
