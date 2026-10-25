//! 当前用户连接器接口，复用账户路由的凭证提取和客户端扩展。
use super::shiju_account::{credential_from_headers, upstream_error};
use crate::{gateway::ApiError, shiju::account::ShijuAccountClient};
use axum::{
    Json, Router,
    extract::Extension,
    http::HeaderMap,
    routing::{get, post, put},
};
use std::sync::Arc;
use vw_gateway_client::{
    ShijuConnector, ShijuConnectorAuthorizeRequest, ShijuConnectorBinding, ShijuConnectorChallenge,
    ShijuConnectorPoll, ShijuConnectorPollRequest, ShijuConnectorSaveRequest,
};

pub(super) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/shiju/connectors", get(list))
        .route("/shiju/connectors/binding", put(save))
        .route("/shiju/connectors/authorizations", post(authorize))
        .route("/shiju/connectors/authorizations/poll", post(poll))
}
async fn list(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ShijuConnector>>, ApiError> {
    client
        .connectors(credential_from_headers(&headers)?.expose())
        .await
        .map(Json)
        .map_err(upstream_error)
}
async fn save(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    Json(request): Json<ShijuConnectorSaveRequest>,
) -> Result<Json<ShijuConnectorBinding>, ApiError> {
    client
        .save_connector_binding(credential_from_headers(&headers)?.expose(), &request)
        .await
        .map(Json)
        .map_err(upstream_error)
}
async fn authorize(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    Json(request): Json<ShijuConnectorAuthorizeRequest>,
) -> Result<Json<ShijuConnectorChallenge>, ApiError> {
    client
        .authorize_connector(credential_from_headers(&headers)?.expose(), &request)
        .await
        .map(Json)
        .map_err(upstream_error)
}
async fn poll(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    Json(request): Json<ShijuConnectorPollRequest>,
) -> Result<Json<ShijuConnectorPoll>, ApiError> {
    client
        .poll_connector(credential_from_headers(&headers)?.expose(), &request.flow_id)
        .await
        .map(Json)
        .map_err(upstream_error)
}
