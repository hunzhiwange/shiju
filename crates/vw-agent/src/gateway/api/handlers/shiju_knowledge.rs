//! 只读云端知识库路由，复用识矩账户凭据和上游客户端。
use super::shiju_account::{credential_from_headers, upstream_error};
use crate::{gateway::ApiError, shiju::account::ShijuAccountClient};
use axum::{
    Json, Router,
    extract::{
        Extension, Query,
        rejection::{JsonRejection, QueryRejection},
    },
    http::HeaderMap,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
use vw_gateway_client::{
    ShijuKnowledgeBaseDto, ShijuKnowledgeDocumentDto, ShijuKnowledgePageDto,
    ShijuKnowledgePageQuery, ShijuKnowledgeReadDto, ShijuKnowledgeReadRequest,
    ShijuKnowledgeSearchDto, ShijuKnowledgeSearchRequest,
};

pub(super) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/shiju/knowledge/bases", get(bases))
        .route("/shiju/knowledge/documents", get(documents))
        .route("/shiju/knowledge/search", post(search))
        .route("/shiju/knowledge/read", post(read))
}

// 显式列出文档查询字段，避免 flatten 与 deny_unknown_fields 的冲突。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentsQuery {
    kb_zid: String,
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size", rename = "pageSize")]
    page_size: u32,
    #[serde(default)]
    q: String,
}
fn default_page() -> u32 {
    ShijuKnowledgePageQuery::default().page
}
fn default_page_size() -> u32 {
    ShijuKnowledgePageQuery::default().page_size
}

async fn bases(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    query: Result<Query<ShijuKnowledgePageQuery>, QueryRejection>,
) -> Result<Json<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let Query(query) =
        query.map_err(|_| ApiError::bad_request("invalid SHIJU knowledge page query"))?;
    client.knowledge_bases(credential.expose(), &query).await.map(Json).map_err(upstream_error)
}
async fn documents(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    query: Result<Query<DocumentsQuery>, QueryRejection>,
) -> Result<Json<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let Query(query) =
        query.map_err(|_| ApiError::bad_request("invalid SHIJU knowledge documents query"))?;
    let page = ShijuKnowledgePageQuery { page: query.page, page_size: query.page_size, q: query.q };
    client
        .knowledge_documents(credential.expose(), &query.kb_zid, &page)
        .await
        .map(Json)
        .map_err(upstream_error)
}
async fn search(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    body: Result<Json<ShijuKnowledgeSearchRequest>, JsonRejection>,
) -> Result<Json<ShijuKnowledgeSearchDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let Json(body) =
        body.map_err(|_| ApiError::bad_request("invalid SHIJU knowledge search request"))?;
    client.knowledge_search(credential.expose(), &body).await.map(Json).map_err(upstream_error)
}
async fn read(
    Extension(client): Extension<Arc<ShijuAccountClient>>,
    headers: HeaderMap,
    body: Result<Json<ShijuKnowledgeReadRequest>, JsonRejection>,
) -> Result<Json<ShijuKnowledgeReadDto>, ApiError> {
    let credential = credential_from_headers(&headers)?;
    let Json(body) =
        body.map_err(|_| ApiError::bad_request("invalid SHIJU knowledge read request"))?;
    client.knowledge_read(credential.expose(), &body).await.map(Json).map_err(upstream_error)
}
