//! SHIJU 应用网关接口，代理目录、安装项、密钥绑定与启动准备请求。
//!
//! SHIJU application gateway API.

use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use vw_gateway_client::{
    SHIJU_CREDENTIAL_HEADER, ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest,
    ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto, ShijuCredential,
    ShijuIamApiKeyListDto,
};

use crate::gateway::ApiError;
use crate::shiju::apps::{ShijuAppClient, ShijuAppError, ShijuAppErrorKind};

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let client = ShijuAppClient::production()
        .unwrap_or_else(|error| panic!("failed to initialize SHIJU application client: {error}"));
    router_with_client(client)
}

pub(crate) fn router_with_client<S>(client: ShijuAppClient) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/shiju/apps/catalog", get(catalog_get))
        .route("/shiju/apps", get(installed_get))
        .route("/shiju/apps/api-keys", get(api_keys_get))
        .route(
            "/shiju/apps/{installation_id}/api-key-binding",
            get(api_key_binding_get).put(api_key_binding_put),
        )
        .route("/shiju/apps/{installation_id}/launch", post(prepare_launch_post))
        .layer(Extension(Arc::new(client)))
}

async fn catalog_get(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
) -> Result<Json<ShijuAppCatalogDto>, ApiError> {
    let credential = credential(&headers)?;
    client.catalog(credential.expose()).await.map(Json).map_err(map_error)
}

async fn installed_get(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
) -> Result<Json<ShijuAppListDto>, ApiError> {
    let credential = credential(&headers)?;
    client.installed(credential.expose()).await.map(Json).map_err(map_error)
}

async fn api_keys_get(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
) -> Result<Json<ShijuIamApiKeyListDto>, ApiError> {
    let credential = credential(&headers)?;
    client.api_keys(credential.expose()).await.map(Json).map_err(map_error)
}

async fn api_key_binding_get(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
) -> Result<Json<ShijuAppApiKeyBindingDto>, ApiError> {
    let credential = credential(&headers)?;
    client
        .api_key_binding(credential.expose(), &installation_id)
        .await
        .map(Json)
        .map_err(map_error)
}

async fn api_key_binding_put(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
    Json(request): Json<ShijuAppApiKeyBindingSaveRequest>,
) -> Result<StatusCode, ApiError> {
    let credential = credential(&headers)?;
    client
        .save_api_key_binding(credential.expose(), &installation_id, request.api_key_id)
        .await
        .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn prepare_launch_post(
    Extension(client): Extension<Arc<ShijuAppClient>>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
) -> Result<Json<ShijuAppLaunchDto>, ApiError> {
    let credential = credential(&headers)?;
    client.prepare_launch(credential.expose(), &installation_id).await.map(Json).map_err(map_error)
}

fn credential(headers: &HeaderMap) -> Result<ShijuCredential, ApiError> {
    let raw = headers
        .get(SHIJU_CREDENTIAL_HEADER)
        .ok_or_else(|| ApiError::bad_request("SHIJU credential header is required"))?
        .to_str()
        .map_err(|_| ApiError::bad_request("SHIJU credential header is invalid"))?;
    ShijuCredential::new(raw).map_err(|_| ApiError::bad_request("SHIJU credential cannot be empty"))
}

fn map_error(error: ShijuAppError) -> ApiError {
    match error.kind() {
        ShijuAppErrorKind::InvalidInput => ApiError::bad_request(error.to_string()),
        ShijuAppErrorKind::Upstream => {
            ApiError::internal("SHIJU application upstream request failed")
        }
    }
}

#[cfg(test)]
#[path = "shiju_apps_tests.rs"]
mod shiju_apps_tests;
