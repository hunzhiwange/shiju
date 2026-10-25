//! SHIJU 账户网关接口，代理登录、资料读取与账户安全更新。
//!
//! 凭据只从专用请求头解析并交给上游客户端，错误统一映射为网关状态码。
//!
//! SHIJU account gateway API.

use std::sync::Arc;

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use vw_gateway_client::{
    SHIJU_CREDENTIAL_HEADER, ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest,
    ShijuAccountProfileDto, ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto,
    ShijuCredential, ShijuLoginChallengeDto, ShijuLoginPollDto,
};

use crate::gateway::ApiError;
use crate::shiju::account::{
    ShijuAccountClient, ShijuAccountError, ShijuAccountPoll, validate_password_update_request,
};

/// 使用生产 SHIJU 客户端构建账户路由。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let client = ShijuAccountClient::production()
        .unwrap_or_else(|error| panic!("failed to initialize SHIJU account client: {error}"));
    router_with_client(client)
}

/// 使用显式客户端构建账户路由，供确定性测试和替代上游复用。
pub(crate) fn router_with_client<S>(client: ShijuAccountClient) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(super::shiju_connectors::router())
        .route("/shiju/account/login", post(login_post))
        .route("/shiju/models/token", axum::routing::put(model_token_put))
        .route("/shiju/account/login/poll", post(login_poll_post))
        .route("/shiju/account/profile", get(profile_get))
        .route("/shiju/account/me", get(account_me_get).put(account_me_put))
        .route("/shiju/account/me/password", axum::routing::put(account_password_put))
        .layer(Extension(Arc::new(client)))
}

async fn model_token_put(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
) -> Result<Json<vw_gateway_client::ShijuModelTokenDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let token = client.model_token(credential.expose()).await.map_err(upstream_error)?;
    Ok(Json(token))
}

async fn login_post(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
) -> Result<Json<ShijuLoginChallengeDto>, ApiError> {
    let challenge = client.start_login().await.map_err(upstream_error)?;
    let dto = serde_json::from_value(serde_json::json!({
        "state": challenge.state(),
        "authorization_url": challenge.authorization_url(),
        "poll_interval_seconds": challenge.poll_interval_seconds(),
    }))
    .map_err(|_| ApiError::internal("SHIJU login response mapping failed"))?;
    Ok(Json(dto))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginPollRequest {
    state: String,
}

async fn login_poll_post(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    Json(body): Json<LoginPollRequest>,
) -> Result<Json<ShijuLoginPollDto>, ApiError> {
    if body.state.trim().is_empty() {
        return Err(ApiError::bad_request("SHIJU login state cannot be empty"));
    }
    let poll = client.poll_login(&body.state).await.map_err(upstream_error)?;
    let dto = match poll {
        ShijuAccountPoll::Pending => ShijuLoginPollDto::Pending,
        ShijuAccountPoll::Approved(value) => ShijuLoginPollDto::Approved {
            credential: ShijuCredential::new(value)
                .map_err(|_| ApiError::internal("SHIJU login response credential is invalid"))?,
        },
    };
    Ok(Json(dto))
}

async fn profile_get(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
) -> Result<Json<ShijuAccountProfileDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let profile = client.profile(credential.expose()).await.map_err(upstream_error)?;
    Ok(Json(profile))
}

async fn account_me_get(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
) -> Result<Json<ShijuAccountMeDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let profile = client.current_user(credential.expose()).await.map_err(upstream_error)?;
    Ok(Json(profile))
}

async fn account_me_put(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    Json(body): Json<ShijuAccountUpdateRequest>,
) -> Result<Json<ShijuAccountUpdateResponseDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("SHIJU account name cannot be empty"));
    }
    let username = body.username.trim();
    if username.is_empty() {
        return Err(ApiError::bad_request("SHIJU account username cannot be empty"));
    }
    let normalized =
        ShijuAccountUpdateRequest { name: name.to_string(), username: username.to_string() };
    let profile = client
        .update_current_user(credential.expose(), &normalized)
        .await
        .map_err(upstream_error)?;
    Ok(Json(profile))
}

async fn account_password_put(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    Json(body): Json<ShijuAccountPasswordUpdateRequest>,
) -> Result<StatusCode, ApiError> {
    let credential = credential_from_headers(&headers)?;
    validate_password_update_request(&body)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    client.update_password(credential.expose(), &body).await.map_err(upstream_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) fn credential_from_headers(headers: &HeaderMap) -> Result<ShijuCredential, ApiError> {
    let raw = headers
        .get(SHIJU_CREDENTIAL_HEADER)
        .ok_or_else(|| ApiError::bad_request("SHIJU credential header is required"))?
        .to_str()
        .map_err(|_| ApiError::bad_request("SHIJU credential header is invalid"))?;
    ShijuCredential::new(raw).map_err(|_| ApiError::bad_request("SHIJU credential cannot be empty"))
}

pub(super) fn upstream_error(error: ShijuAccountError) -> ApiError {
    if let Some(status) = error.status() {
        return ApiError { status, message: error.to_string() };
    }
    ApiError::internal("SHIJU account upstream request failed")
}

#[cfg(test)]
#[path = "shiju_account_tests.rs"]
mod shiju_account_tests;
